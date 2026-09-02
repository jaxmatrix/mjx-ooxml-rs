//! The legacy surfaces: OLE objects, ActiveX controls, ink, SmartArt diagrams, and the VML
//! drawings that PowerPoint pairs with the first two.

#[cfg(feature = "vml")]
use mjx_ooxml_core::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawDocument, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{SchemaNamespace, DML_DIAGRAM, DML_MAIN, PML};
use mjx_opc::{PartName, Relationship, TargetMode};
#[cfg(feature = "vml")]
use mjx_vml::is_vml_content_type;

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::external::{default_placeholder_ole, OleObject};
use crate::geometry::ShapeBounds;
use crate::legacy::{
    self, ActiveXControlSpec, ActiveXPersistence, DiagramContent, DiagramPartKind, DiagramParts,
    DiagramRelationshipIds, InkReference, OleObjectData, OleObjectSpec,
};
use crate::surface::Surface;
use crate::{build, constants, nav, slide};

use super::deck::{dir_of, stem_number};
use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::element_builders::build_nv_graphic_frame_pr;
use super::pictures::build_picture;
use super::Presentation;

impl Presentation {
    /// The relationship id the OLE frame `shape_idx` on `surface` names for its embedded object
    /// (`p:oleObj@r:id`), or `None` when the shape is not a graphic frame holding an OLE object.
    /// Reading does not dirty the part.
    ///
    /// The embedded object lives in a separate part (`/ppt/embeddings/oleObjectN.bin`, or an embedded
    /// `.xlsx`/`.docx`); this returns the id of the slide relationship that names it, which
    /// [`ole_object_part_bytes`](Self::ole_object_part_bytes) resolves to bytes.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn ole_object_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::ole_object_rel_id(shape, &doc.interner).map(str::to_owned))
    }

    /// The raw bytes of the embedded object the OLE frame `shape_idx` on `surface` references
    /// (`/ppt/embeddings/oleObjectN.bin` or an embedded package), exactly as the package holds them, or
    /// `None` when the shape frames no OLE object. Borrowed from the package, so the part is not copied.
    ///
    /// The embedded object is **not modeled** — it is an opaque OLE stream or embedded document, carried
    /// through a round-trip verbatim. Reading does not dirty anything.
    ///
    /// # Errors
    /// As [`ole_object_rel_id`](Self::ole_object_rel_id), plus [`PptxError::ExternalTarget`] if the
    /// relationship points outside the package.
    pub fn ole_object_part_bytes(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.ole_object_rel_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(part) = self.part_for_rel(&slide_part, &rel_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// The relationship id of the **fallback snapshot** image the OLE frame `shape_idx` on `surface`
    /// carries (`p:oleObj > p:pic > p:blipFill > a:blip@r:embed`), or `None` when the frame is not an
    /// OLE object or has no snapshot. Reading does not dirty the part.
    ///
    /// This is the image a renderer draws in place of the (never-executed) embedded object.
    /// [`ole_snapshot_image_bytes`](Self::ole_snapshot_image_bytes) resolves it to bytes.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn ole_snapshot_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::ole_snapshot_rel_id(shape, &doc.interner).map(str::to_owned))
    }

    /// The stored bytes of the OLE fallback snapshot image the frame `shape_idx` on `surface` embeds,
    /// exactly as the package holds them (never decoded or re-encoded), or `None` when the frame is not
    /// an OLE object or carries no snapshot. Borrowed from the package.
    ///
    /// # Errors
    /// As [`ole_snapshot_rel_id`](Self::ole_snapshot_rel_id), plus [`PptxError::ExternalTarget`] if the
    /// relationship points outside the package.
    pub fn ole_snapshot_image_bytes(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.ole_snapshot_rel_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(part) = self.part_for_rel(&slide_part, &rel_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// The `progId` the OLE frame `shape_idx` on `surface` declares (e.g. `"Excel.Sheet.12"`) — the
    /// application that owns the embedded object — or `None` when the shape frames no OLE object or the
    /// attribute is absent. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn ole_prog_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::ole_prog_id(shape, &doc.interner).map(str::to_owned))
    }

    /// Every OLE object frame on `surface`, with where its object data is referenced from and whether
    /// that reference is external.
    ///
    /// An external object is the case that can be unreachable on another platform; an OLE object
    /// displays via its snapshot image regardless, so
    /// [`replace_ole_object_with_placeholder`](Self::replace_ole_object_with_placeholder) can neutralize
    /// it safely. This saves the caller from walking the shapes. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `surface` cannot be resolved or a slide is malformed.
    pub fn ole_objects(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<Vec<OleObject>, PptxError> {
        let surface = surface.into();
        let count = self.shape_count(surface)?;
        let mut objects = Vec::new();
        for shape_index in 0..count {
            let path: ShapePath = shape_index.into();
            let Some(rel_id) = self.ole_object_data_rel_id(surface, &path)? else {
                continue; // not an OLE frame, or one with no resolvable object data
            };
            let Some(rel) = self
                .package
                .relationships_for(Some(&self.surface_part(surface)?))
                .and_then(|rels| rels.by_id(&rel_id))
            else {
                continue;
            };
            let target = rel.target.clone();
            let external = rel.mode == TargetMode::External;
            let prog_id = self.ole_prog_id(surface, shape_index)?;
            objects.push(OleObject {
                shape_index,
                target,
                external,
                prog_id,
            });
        }
        Ok(objects)
    }

    /// Replaces the object data of the OLE frame `shape_idx` on `surface` with an in-package
    /// placeholder, so an object that points at unreachable external data resolves inside the package
    /// instead. The placeholder is `placeholder` if given, else [`default_placeholder_ole`] (a minimal
    /// valid compound file). The `p:oleObj` markup is unchanged — its relationship is simply retargeted
    /// at the placeholder — and the object keeps displaying via its snapshot image.
    ///
    /// The caller decides an object is inaccessible (the library does no external I/O); use
    /// [`ole_objects`](Self::ole_objects) to find the candidates. If the old reference was to an
    /// *embedded* part, that part is left unreferenced; sweep it with
    /// [`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts) if wanted.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAnOleObject`] if the shape frames no OLE object, or another [`PptxError`]
    /// if an index is out of range or the slide is malformed.
    pub fn replace_ole_object_with_placeholder(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        placeholder: Option<&[u8]>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let rel_id = self
            .ole_object_data_rel_id(surface, &path)?
            .ok_or(PptxError::ShapeIsNotAnOleObject)?;

        let placeholder_part = self.next_embedding_part("bin")?;
        let bytes = match placeholder {
            Some(bytes) => bytes.to_vec(),
            None => default_placeholder_ole(),
        };
        self.package
            .insert_part(&placeholder_part, constants::CONTENT_TYPE_OLE_OBJECT, bytes)?;

        let slide_part = self.surface_part(surface)?;
        let target = slide_part.relative_target(&placeholder_part);
        self.package.retarget_relationship(
            Some(&slide_part),
            &rel_id,
            &target,
            TargetMode::Internal,
        )?;
        Ok(())
    }

    /// The relationship id an OLE frame names for its object data — `p:oleObj@r:id` (embedded) or
    /// `@r:link` (linked), whichever is present — or `None` when the shape frames no OLE object.
    fn ole_object_data_rel_id(
        &mut self,
        surface: Surface,
        shape_idx: &ShapePath,
    ) -> Result<Option<String>, PptxError> {
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, shape_idx)?;
        Ok(slide::ole_object_data_rel_id(shape, &doc.interner).map(str::to_owned))
    }

    /// The number of legacy **ActiveX** form controls on `surface` (`p:cSld > p:controls > p:control`).
    ///
    /// A control is not a shape — it lives beside the shape tree, so it is addressed by its own index in
    /// `0..count`, not the shape index space. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the slide is malformed.
    pub fn activex_control_count(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        Ok(slide::controls(&doc.root, &doc.interner).count())
    }

    /// The relationship id the ActiveX control `control_idx` on `surface` names for its control part
    /// (`p:control@r:id`), or `None` when there is no such control. Reading does not dirty the part.
    ///
    /// The control part (`/ppt/activeX/activeXN.xml`, `ax:ocx` markup) lives separately;
    /// [`activex_part_bytes`](Self::activex_part_bytes) resolves this id to its bytes.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the slide is malformed.
    pub fn activex_control_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let Some(control) = slide::nth_control(&doc.root, &doc.interner, control_idx) else {
            return Ok(None);
        };
        Ok(slide::control_rel_id(control, &doc.interner).map(str::to_owned))
    }

    /// The `name` the ActiveX control `control_idx` on `surface` declares (e.g. `"CommandButton1"`), or
    /// `None` when there is no such control or it is unnamed. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the slide is malformed.
    pub fn activex_control_name(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let Some(control) = slide::nth_control(&doc.root, &doc.interner, control_idx) else {
            return Ok(None);
        };
        Ok(slide::control_name(control, &doc.interner).map(str::to_owned))
    }

    /// The control part (`/ppt/activeX/activeXN.xml`) the ActiveX control `control_idx` on `surface`
    /// references, or `None` when there is no such control.
    fn activex_part(
        &mut self,
        surface: Surface,
        control_idx: usize,
    ) -> Result<Option<PartName>, PptxError> {
        let Some(rel_id) = self.activex_control_rel_id(surface, control_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        self.part_for_rel(&slide_part, &rel_id)
    }

    /// The raw bytes of the ActiveX control part (`ax:ocx` markup) the control `control_idx` on
    /// `surface` references, exactly as the package holds them, or `None` when there is no such control.
    /// Borrowed from the package; reading does not dirty anything.
    ///
    /// The control markup is **not modeled** — it is carried through a round-trip verbatim.
    ///
    /// # Errors
    /// As [`activex_control_rel_id`](Self::activex_control_rel_id), plus [`PptxError::ExternalTarget`]
    /// if the relationship points outside the package.
    pub fn activex_part_bytes(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(part) = self.activex_part(surface, control_idx)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// The ActiveX control's **persisted state** — the bytes of `/ppt/activeX/activeXN.bin` — for the
    /// control `control_idx` on `surface`, or `None` when there is no such control or it persists no
    /// state. Borrowed from the package; reading does not dirty anything.
    ///
    /// This is what [`set_activex_state`](Self::set_activex_state) replaces.
    ///
    /// This resolves the two-hop chain: `p:control@r:id` → the control part, then that part's
    /// `activeXControlBinary` relationship → the `.bin`. Not modeled — carried through verbatim.
    ///
    /// # Errors
    /// As [`activex_control_rel_id`](Self::activex_control_rel_id), plus [`PptxError::ExternalTarget`]
    /// if either relationship points outside the package.
    pub fn activex_state_bytes(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(activex_part) = self.activex_part(surface, control_idx)? else {
            return Ok(None);
        };
        let Some(binary_part) =
            self.follow_rel(&activex_part, constants::REL_ACTIVEX_CONTROL_BINARY)?
        else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&binary_part))
    }

    /// The relationship id of the **fallback snapshot** image the ActiveX control `control_idx` on
    /// `surface` carries (`p:control > p:pic > p:blipFill > a:blip@r:embed`), or `None` when there is no
    /// such control or snapshot. Reading does not dirty the part.
    ///
    /// This is the image a renderer draws in place of the (never-executed) control.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the slide is malformed.
    pub fn activex_snapshot_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let Some(control) = slide::nth_control(&doc.root, &doc.interner, control_idx) else {
            return Ok(None);
        };
        Ok(slide::pic_snapshot_rel_id(control, &doc.interner).map(str::to_owned))
    }

    /// The stored bytes of the ActiveX control's fallback snapshot image for the control `control_idx`
    /// on `surface`, exactly as the package holds them (never decoded or re-encoded), or `None` when
    /// there is no such control or snapshot. Borrowed from the package.
    ///
    /// # Errors
    /// As [`activex_snapshot_rel_id`](Self::activex_snapshot_rel_id), plus [`PptxError::ExternalTarget`]
    /// if the relationship points outside the package.
    pub fn activex_snapshot_image_bytes(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.activex_snapshot_rel_id(surface, control_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(part) = self.part_for_rel(&slide_part, &rel_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// The names of every legacy **VML** drawing part in the package (`ppt/drawings/vmlDrawingN.vml`
    /// and the like), in package order.
    ///
    /// VML is Transitional-only legacy markup that producers still emit for OLE-object fallbacks,
    /// comment authoring shapes, ink, and legacy form controls. It is recognized by content type
    /// ([`is_vml_content_type`]) rather than navigated from a shape, so this finds VML referenced from
    /// any part — slides, notes, masters, handout — uniformly.
    ///
    /// Preserve-first: the parts are **not modeled**. Read their raw bytes with
    /// [`vml_part_bytes`](Self::vml_part_bytes); untouched, they round-trip verbatim. Reading does not
    /// dirty anything.
    ///
    /// Requires the `vml` crate feature.
    #[cfg(feature = "vml")]
    #[must_use]
    pub fn vml_part_names(&self) -> Vec<PartName> {
        self.package
            .part_names()
            .filter(|part| {
                self.package
                    .content_type_of(part)
                    .is_some_and(is_vml_content_type)
            })
            .collect()
    }

    /// The raw bytes of the VML drawing `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed from the package, so the
    /// part is not copied and nothing is dirtied.
    ///
    /// Pair with [`vml_part_names`](Self::vml_part_names). Preserve-first: the bytes are the legacy VML
    /// XML verbatim, not a model.
    ///
    /// Requires the `vml` crate feature.
    #[cfg(feature = "vml")]
    #[must_use]
    pub fn vml_part_bytes(&self, part: &PartName) -> Option<&[u8]> {
        self.package.part_bytes(part)
    }

    /// The names of every **ink** (InkML) part in the package (`ppt/ink/inkN.xml`), in package order.
    ///
    /// Ink is legacy handwriting carried as an InkML part referenced from the shape tree by a
    /// `p14:contentPart` — which producers wrap in `mc:AlternateContent`, out of reach of the shape
    /// index space. So ink is recognized by its content type ([`CONTENT_TYPE_INKML`]) rather than
    /// navigated from a shape, finding every InkML part uniformly.
    ///
    /// Preserve-first: the parts are **not modeled**. Read their raw bytes with
    /// [`ink_part_bytes`](Self::ink_part_bytes); untouched, they round-trip verbatim. Reading does not
    /// dirty anything.
    ///
    /// [`CONTENT_TYPE_INKML`]: constants::CONTENT_TYPE_INKML
    #[must_use]
    pub fn ink_part_names(&self) -> Vec<PartName> {
        self.package
            .part_names()
            .filter(|part| {
                self.package.content_type_of(part) == Some(constants::CONTENT_TYPE_INKML)
            })
            .collect()
    }

    /// The raw bytes of the ink (InkML) `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed from the package, so the
    /// part is not copied and nothing is dirtied.
    ///
    /// Pair with [`ink_part_names`](Self::ink_part_names). Preserve-first: the bytes are the InkML XML
    /// verbatim, not a model.
    #[must_use]
    pub fn ink_part_bytes(&self, part: &PartName) -> Option<&[u8]> {
        self.package.part_bytes(part)
    }

    // -----------------------------------------------------------------------------------------
    // Ink — tying an InkML part back to the shape that references it
    // -----------------------------------------------------------------------------------------

    /// Every ink (InkML) part `surface` references, with where it is referenced from.
    ///
    /// [`ink_part_names`](Self::ink_part_names) finds the InkML parts a *package* carries;
    /// this answers the other question — which shape a given stroke set belongs to. A reference is
    /// reported only when the part it names really is InkML, so an unrelated `p:contentPart` (the
    /// same element also carries custom XML) is not mistaken for ink.
    ///
    /// A `p:contentPart` is a shape, so its `shape_index` is `Some`; a `p14:contentPart` is wrapped in
    /// `mc:AlternateContent`, which is not in the shape index space, so its index is `None`. Reading
    /// does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `surface` cannot be resolved or the part is malformed.
    pub fn ink_references(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<Vec<InkReference>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let candidates = slide::content_part_references(sp_tree, &doc.interner);

        let mut references = Vec::with_capacity(candidates.len());
        for (shape_index, rel_id) in candidates {
            let part = self.part_for_rel(&slide_part, &rel_id).unwrap_or(None);
            let is_ink = part.as_ref().is_some_and(|part| {
                self.package.content_type_of(part) == Some(constants::CONTENT_TYPE_INKML)
            });
            if !is_ink {
                continue;
            }
            references.push(InkReference {
                shape_index,
                rel_id,
                part,
            });
        }
        Ok(references)
    }

    /// The ink part the shape `shape_idx` on `surface` references, or `None` when that shape is not a
    /// content part or does not reference ink.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the surface is malformed.
    pub fn ink_part_for_shape(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<PartName>, PptxError> {
        let surface = surface.into();
        Ok(self
            .ink_references(surface)?
            .into_iter()
            .find(|reference| reference.shape_index == Some(shape_idx))
            .and_then(|reference| reference.part))
    }

    /// The shape index of the content part on `surface` that references the ink `part`, or `None`
    /// when no shape on that surface does (or the reference lives inside an `mc:AlternateContent`,
    /// which is out of the shape index space).
    ///
    /// This is the question a caller asks holding a stroke set: *which shape is this?*
    ///
    /// # Errors
    /// Returns [`PptxError`] if `surface` cannot be resolved or the part is malformed.
    pub fn shape_for_ink_part(
        &mut self,
        surface: impl Into<Surface>,
        part: &PartName,
    ) -> Result<Option<usize>, PptxError> {
        let surface = surface.into();
        Ok(self
            .ink_references(surface)?
            .into_iter()
            .find(|reference| reference.part.as_ref() == Some(part))
            .and_then(|reference| reference.shape_index))
    }

    /// Adds an ink (InkML) part holding `inkml` to the package and a `p:contentPart` referencing it to
    /// `surface`, and returns the new shape's index in the one shape index space.
    ///
    /// The content part is written as PresentationML's own `p:contentPart` (`CT_Rel`) rather than the
    /// Office 2010 `p14:contentPart` producers wrap in `mc:AlternateContent`: the plain element is in
    /// the shape index space, so the ink can be found, moved in the tree and removed like any other
    /// shape. Ink positions itself from its own InkML coordinates, which is why there are no bounds to
    /// give — `p:contentPart` has no transform.
    ///
    /// # Errors
    /// [`PptxError::InvalidInkContent`] if `inkml` is not an InkML document, or another [`PptxError`]
    /// if the surface index is out of range or a package edit fails.
    pub fn add_ink(
        &mut self,
        surface: impl Into<Surface>,
        inkml: &[u8],
    ) -> Result<usize, PptxError> {
        check_inkml(inkml)?;
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;

        let ink_part = self.next_part_in("ink", "ink", "xml")?;
        self.package
            .insert_part(&ink_part, constants::CONTENT_TYPE_INKML, inkml.to_vec())?;
        let rel_id = self.next_rid_for(&slide_part);
        self.package.add_relationship(
            Some(&slide_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_INK.to_owned(),
                target: nav::relative_target(&slide_part, &ink_part),
                mode: TargetMode::Internal,
            },
        )?;

        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let content_part = build_content_part(interner, &rel_id, rel_declaration);
        sp_tree.children.push(RawNode::Element(content_part));
        sp_tree.empty = false;
        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Replaces the strokes of the ink the shape `shape_idx` on `surface` references, in place.
    ///
    /// Only the ink part changes: the slide is not touched at all, so every other part — the slide
    /// included — stays byte-identical.
    ///
    /// # Errors
    /// [`PptxError::InvalidInkContent`] if `inkml` is not an InkML document,
    /// [`PptxError::ShapeIsNotAContentPart`] if the shape references no ink, or another [`PptxError`]
    /// if an index is out of range.
    pub fn set_ink_content(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        inkml: &[u8],
    ) -> Result<(), PptxError> {
        check_inkml(inkml)?;
        let surface = surface.into();
        let part = self
            .ink_part_for_shape(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotAContentPart)?;
        self.package.replace_part_bytes(&part, inkml.to_vec())?;
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // SmartArt diagrams
    // -----------------------------------------------------------------------------------------

    /// The four relationship ids the SmartArt frame `shape_idx` on `surface` names in its
    /// `dgm:relIds`, or `None` when the shape frames no diagram. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the surface is malformed.
    pub fn diagram_relationship_ids(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<DiagramRelationshipIds>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        let Some(rel_ids) = slide::diagram_rel_ids(shape, &doc.interner) else {
            return Ok(None);
        };
        let read =
            |local: &str| slide::diagram_rel_id(rel_ids, &doc.interner, local).map(str::to_owned);
        Ok(Some(DiagramRelationshipIds {
            data: read("dm"),
            layout: read("lo"),
            style: read("qs"),
            colors: read("cs"),
        }))
    }

    /// The parts of the SmartArt diagram the frame `shape_idx` on `surface` references, resolved to
    /// part names — the relationship graph behind the diagram, `None` when the shape frames none.
    ///
    /// Four come from the frame's own `dgm:relIds`. The fifth, the cached drawing, hangs off the
    /// **data** part rather than the frame, so it is found by following that part's own relationships.
    /// Reading does not dirty anything.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the surface is malformed, or a relationship
    /// points outside the package.
    pub fn diagram_parts(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<DiagramParts>, PptxError> {
        let surface = surface.into();
        let Some(ids) = self.diagram_relationship_ids(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let resolve = |id: &Option<String>| -> Result<Option<PartName>, PptxError> {
            match id {
                Some(id) => self.part_for_rel(&slide_part, id),
                None => Ok(None),
            }
        };
        let data = resolve(&ids.data)?;
        let layout = resolve(&ids.layout)?;
        let style = resolve(&ids.style)?;
        let colors = resolve(&ids.colors)?;
        let drawing = match &data {
            Some(data) => self.follow_rel(data, constants::REL_DIAGRAM_DRAWING)?,
            None => None,
        };
        Ok(Some(DiagramParts {
            data,
            layout,
            style,
            colors,
            drawing,
        }))
    }

    /// The raw bytes of a diagram `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed; nothing is dirtied.
    ///
    /// Pair with [`diagram_parts`](Self::diagram_parts).
    #[must_use]
    pub fn diagram_part_bytes(&self, part: &PartName) -> Option<&[u8]> {
        self.package.part_bytes(part)
    }

    /// Adds a SmartArt diagram to `surface`, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    ///
    /// Four parts are written under `ppt/diagrams/` — the data model, the layout definition, the quick
    /// style and the colour transform — each related from the surface with its own relationship type,
    /// and a `p:graphicFrame` whose `dgm:relIds` names all four. That is the whole graph a consumer
    /// walks. Build [`DiagramContent`] from a list of labels with
    /// [`vertical_list`](DiagramContent::vertical_list), or hand over four documents of your own with
    /// [`from_parts`](DiagramContent::from_parts).
    ///
    /// The cached drawing (`dsp:drawing`) is **not** written: it is a cache of a layout this library
    /// does not run, and PowerPoint regenerates it. Its absence is valid — the four parts are what the
    /// schema requires.
    ///
    /// The diagram is a shape: move it with [`set_shape_bounds`](Self::set_shape_bounds) and drop it
    /// with [`remove_shape`](Self::remove_shape).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or a package edit fails.
    pub fn add_diagram(
        &mut self,
        surface: impl Into<Surface>,
        content: &DiagramContent,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;

        // Every part name is chosen before anything is written, so the four land in one numbered set.
        let number = self.next_diagram_number();
        let parts = [
            ("data", constants::CONTENT_TYPE_DIAGRAM_DATA, &content.data),
            (
                "layout",
                constants::CONTENT_TYPE_DIAGRAM_LAYOUT,
                &content.layout,
            ),
            (
                "quickStyle",
                constants::CONTENT_TYPE_DIAGRAM_STYLE,
                &content.style,
            ),
            (
                "colors",
                constants::CONTENT_TYPE_DIAGRAM_COLORS,
                &content.colors,
            ),
        ];
        let rel_types = [
            constants::REL_DIAGRAM_DATA,
            constants::REL_DIAGRAM_LAYOUT,
            constants::REL_DIAGRAM_QUICK_STYLE,
            constants::REL_DIAGRAM_COLORS,
        ];

        let mut rel_ids = Vec::with_capacity(4);
        for ((stem, content_type, bytes), rel_type) in parts.into_iter().zip(rel_types) {
            let part = self.diagram_part_name(stem, number)?;
            self.package
                .insert_part(&part, content_type, bytes.clone())?;
            let rel_id = self.next_rid_for(&slide_part);
            self.package.add_relationship(
                Some(&slide_part),
                Relationship {
                    id: rel_id.clone(),
                    rel_type: rel_type.to_owned(),
                    target: nav::relative_target(&slide_part, &part),
                    mode: TargetMode::Internal,
                },
            )?;
            rel_ids.push(rel_id);
        }

        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let frame = build_diagram_frame(interner, next_id, &rel_ids, bounds, rel_declaration);
        sp_tree.children.push(RawNode::Element(frame));
        sp_tree.empty = false;
        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Replaces one part of the SmartArt diagram the frame `shape_idx` on `surface` references, in
    /// place.
    ///
    /// Only that part changes: the frame keeps naming it by the same relationship, and every other
    /// part of the deck — the other three diagram parts included — stays byte-identical. This is how a
    /// caller re-labels a diagram (replace [`Data`](DiagramPartKind::Data)) or restyles it (replace
    /// [`Colors`](DiagramPartKind::Colors)) without disturbing the rest.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotADiagram`] if the shape frames no diagram,
    /// [`PptxError::DiagramPartMissing`] if the diagram has no part of that kind, or another
    /// [`PptxError`] if an index is out of range.
    pub fn set_diagram_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        kind: DiagramPartKind,
        bytes: Vec<u8>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let parts = self
            .diagram_parts(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotADiagram)?;
        let part = match kind {
            DiagramPartKind::Data => parts.data,
            DiagramPartKind::Layout => parts.layout,
            DiagramPartKind::Style => parts.style,
            DiagramPartKind::Colors => parts.colors,
            DiagramPartKind::Drawing => parts.drawing,
        }
        .ok_or(PptxError::DiagramPartMissing { kind })?;
        self.package.replace_part_bytes(&part, bytes)?;
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // OLE objects — authoring and editing
    // -----------------------------------------------------------------------------------------

    /// Adds an OLE object to `surface`, laid out inside `bounds`, and returns its index in the shape
    /// tree.
    ///
    /// Three things are written: the object's data (an `oleObjectN.bin` stream, an embedded package,
    /// or nothing at all for a link), the snapshot image a consumer draws in its place, and a
    /// `p:graphicFrame` whose `p:oleObj` names both. The frame is written **without** the
    /// `mc:AlternateContent` wrapper PowerPoint uses for its VML fallback: a bare `p:oleObj` is what
    /// `CT_OleObject` describes, and the snapshot inside it is what renders.
    ///
    /// The object is a shape: move it with [`set_shape_bounds`](Self::set_shape_bounds), read it back
    /// with [`ole_objects`](Self::ole_objects), and drop it with [`remove_shape`](Self::remove_shape).
    ///
    /// # Errors
    /// [`PptxError::UnrecognizedImageFormat`] if the snapshot bytes match no known image format, or
    /// another [`PptxError`] if the surface index is out of range or a package edit fails.
    pub fn add_ole_object(
        &mut self,
        surface: impl Into<Surface>,
        spec: &OleObjectSpec<'_>,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;

        // The snapshot first: if the bytes are not an image, nothing else has been written.
        let snapshot_rel_id = self.add_image(surface, spec.snapshot_image)?;

        let (data_rel_id, linked) = match spec.data {
            OleObjectData::EmbeddedStream(bytes) => {
                let part = self.next_embedding_part("bin")?;
                self.package.insert_part(
                    &part,
                    constants::CONTENT_TYPE_OLE_OBJECT,
                    bytes.to_vec(),
                )?;
                (
                    self.relate(&slide_part, &part, constants::REL_OLE_OBJECT)?,
                    false,
                )
            }
            OleObjectData::EmbeddedPackage {
                bytes,
                extension,
                content_type,
            } => {
                let part = self.next_embedding_part(extension)?;
                self.package
                    .insert_part(&part, content_type, bytes.to_vec())?;
                (
                    self.relate(&slide_part, &part, constants::REL_PACKAGE)?,
                    false,
                )
            }
            OleObjectData::Linked(target) => {
                let rel_id = self.next_rid_for(&slide_part);
                self.package.add_relationship(
                    Some(&slide_part),
                    Relationship {
                        id: rel_id.clone(),
                        rel_type: constants::REL_OLE_OBJECT.to_owned(),
                        target: target.to_owned(),
                        mode: TargetMode::External,
                    },
                )?;
                (rel_id, true)
            }
        };

        let name = spec.name.map(str::to_owned);
        let prog_id = spec.prog_id.to_owned();
        let show_as_icon = spec.show_as_icon;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let frame = build_ole_frame(
            interner,
            next_id,
            &OleFrameParts {
                data_rel_id: &data_rel_id,
                snapshot_rel_id: &snapshot_rel_id,
                prog_id: &prog_id,
                name: name.as_deref(),
                show_as_icon,
                linked,
            },
            bounds,
            rel_declaration,
        );
        sp_tree.children.push(RawNode::Element(frame));
        sp_tree.empty = false;
        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Sets the `progId` of the OLE frame `shape_idx` on `surface` — which application owns the
    /// embedded object. Only the surface's part is dirtied.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAnOleObject`] if the shape frames no OLE object, or another
    /// [`PptxError`] if an index is out of range.
    pub fn set_ole_prog_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        prog_id: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        // Prove the shape really is an OLE frame before dirtying the part.
        if self.ole_object_data_rel_id(surface, &path)?.is_none() {
            return Err(PptxError::ShapeIsNotAnOleObject);
        }
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &path)?;
        let object =
            slide::ole_object_mut(shape, interner).ok_or(PptxError::ShapeIsNotAnOleObject)?;
        let attribute = build::attr(interner, "progId", prog_id);
        set_or_replace_attr(&mut object.attributes, interner, "progId", attribute);
        Ok(())
    }

    /// Replaces the data of the OLE object the frame `shape_idx` on `surface` embeds, in place.
    ///
    /// The relationship and the `p:oleObj` markup are untouched, so the slide is not dirtied at all —
    /// only the embedding part changes. An object whose data is *linked* has no part to replace;
    /// retarget it with
    /// [`replace_ole_object_with_placeholder`](Self::replace_ole_object_with_placeholder) instead.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAnOleObject`] if the shape frames no OLE object or its data is linked
    /// rather than embedded, or another [`PptxError`] if an index is out of range.
    pub fn set_ole_object_data(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        bytes: &[u8],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let rel_id = self
            .ole_object_data_rel_id(surface, &path)?
            .ok_or(PptxError::ShapeIsNotAnOleObject)?;
        let slide_part = self.surface_part(surface)?;
        let part = self
            .part_for_rel(&slide_part, &rel_id)?
            .ok_or(PptxError::ShapeIsNotAnOleObject)?;
        self.package.replace_part_bytes(&part, bytes.to_vec())?;
        Ok(())
    }

    /// Replaces the fallback snapshot image of the OLE frame `shape_idx` on `surface` — the picture a
    /// consumer draws in place of the object it will never run.
    ///
    /// The `p:oleObj` markup is untouched: a new image part is stored and the frame's existing image
    /// relationship is retargeted at it, so every other part stays byte-identical. Identical bytes are
    /// stored once, as [`add_image`](Self::add_image) does.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAnOleObject`] if the shape frames no OLE object or carries no snapshot,
    /// [`PptxError::UnrecognizedImageFormat`] if the bytes match no known image format, or another
    /// [`PptxError`] if an index is out of range.
    pub fn set_ole_snapshot_image(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        bytes: &[u8],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let rel_id = self
            .ole_snapshot_rel_id(surface, &path)?
            .ok_or(PptxError::ShapeIsNotAnOleObject)?;
        self.retarget_image_relationship(surface, &rel_id, bytes)
    }

    // -----------------------------------------------------------------------------------------
    // ActiveX controls — authoring and editing
    // -----------------------------------------------------------------------------------------

    /// Adds an ActiveX form control to `surface`, laid out inside `bounds`, and returns its index in
    /// the surface's **control** index space (not the shape index space — a `p:control` is a sibling
    /// of the shape tree, not a member of it).
    ///
    /// Three things are written: the control part (`ppt/activeX/activeXN.xml`, `ax:ocx` markup naming
    /// the COM class id), its persisted state (`activeXN.bin`, when the spec carries one), and the
    /// snapshot image a consumer draws in the control's place. The `p:controls` container is created
    /// when the surface has none.
    ///
    /// # Errors
    /// [`PptxError::UnrecognizedImageFormat`] if the snapshot bytes match no known image format, or
    /// another [`PptxError`] if the surface index is out of range or a package edit fails.
    pub fn add_activex_control(
        &mut self,
        surface: impl Into<Surface>,
        spec: &ActiveXControlSpec<'_>,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;

        // The snapshot first: if the bytes are not an image, nothing else has been written.
        let snapshot_rel_id = self.add_image(surface, spec.snapshot_image)?;

        let number = self.next_activex_number();
        let control_part = self.activex_part_name(number, "xml")?;
        let binary_rel_id = match spec.state {
            Some(state) => {
                let binary_part = self.activex_part_name(number, "bin")?;
                self.package.insert_part(
                    &binary_part,
                    constants::CONTENT_TYPE_ACTIVEX_BINARY,
                    state.to_vec(),
                )?;
                // The control part is brand new, so its relationship space is empty and rId1 is free.
                Some(("rId1".to_owned(), binary_part))
            }
            None => None,
        };
        self.package.insert_part(
            &control_part,
            constants::CONTENT_TYPE_ACTIVEX,
            legacy::activex_part_bytes(
                spec.class_id,
                spec.persistence,
                binary_rel_id.as_ref().map(|(id, _)| id.as_str()),
            ),
        )?;
        if let Some((rel_id, binary_part)) = &binary_rel_id {
            self.package.add_relationship(
                Some(&control_part),
                Relationship {
                    id: rel_id.clone(),
                    rel_type: constants::REL_ACTIVEX_CONTROL_BINARY.to_owned(),
                    target: nav::relative_target(&control_part, binary_part),
                    mode: TargetMode::Internal,
                },
            )?;
        }
        let control_rel_id = self.relate(&slide_part, &control_part, constants::REL_CONTROL)?;

        let name = spec.name.to_owned();
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let next_id = slide::sp_tree(root, interner)
            .map(|tree| slide::max_cnvpr_id(tree, interner).max(1) + 1)
            .unwrap_or(2);
        let controls = slide::controls_mut(root, interner)?;
        let control = build_control(
            interner,
            next_id,
            &name,
            &control_rel_id,
            &snapshot_rel_id,
            bounds,
            rel_declaration,
        );
        controls.children.push(RawNode::Element(control));
        controls.empty = false;
        Ok(slide::controls(root, interner).count() - 1)
    }

    /// Points the OLE frame `shape_idx` on `surface` at the VML shape with `identifier`
    /// (`p:oleObj@spid`) — how an authored object is bound to the legacy fallback that draws it.
    ///
    /// The identifier is not checked against any drawing: the fallback is often written after the
    /// frame, and a `spid` naming a shape that does not exist yet is exactly what a producer emits
    /// mid-write. Read it back with [`ole_legacy_shape_id`](Self::ole_legacy_shape_id) and resolve it
    /// with `with_vml_shape_for_ole_object`, which the `vml` crate feature adds.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAnOleObject`] if the shape frames no OLE object, or another
    /// [`PptxError`] if an index is out of range.
    pub fn set_ole_legacy_shape_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        identifier: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        if self.ole_object_data_rel_id(surface, &path)?.is_none() {
            return Err(PptxError::ShapeIsNotAnOleObject);
        }
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &path)?;
        let object =
            slide::ole_object_mut(shape, interner).ok_or(PptxError::ShapeIsNotAnOleObject)?;
        let attribute = build::attr(interner, "spid", identifier);
        set_or_replace_attr(&mut object.attributes, interner, "spid", attribute);
        Ok(())
    }

    /// Points the ActiveX control `control_idx` on `surface` at the VML shape with `identifier`
    /// (`p:control@spid`). As [`set_ole_legacy_shape_id`](Self::set_ole_legacy_shape_id).
    ///
    /// # Errors
    /// [`PptxError::ActiveXControlOutOfRange`] if the surface has no such control, or another
    /// [`PptxError`] if the surface index is out of range.
    pub fn set_activex_control_shape_id(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
        identifier: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let count = self.activex_control_count(surface)?;
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let control = slide::nth_control_mut(root, interner, control_idx).ok_or(
            PptxError::ActiveXControlOutOfRange {
                index: control_idx,
                count,
            },
        )?;
        let attribute = build::attr(interner, "spid", identifier);
        set_or_replace_attr(&mut control.attributes, interner, "spid", attribute);
        Ok(())
    }

    /// The `spid` the ActiveX control `control_idx` on `surface` names — the `id` of the VML shape
    /// that draws it in a legacy consumer — or `None` when there is no such control or it names none.
    ///
    /// Resolve it against the surface's VML drawing with `with_vml_shape_for_activex_control`,
    /// which the `vml` crate feature adds.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the surface is malformed.
    pub fn activex_control_shape_id(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let Some(control) = slide::nth_control(&doc.root, &doc.interner, control_idx) else {
            return Ok(None);
        };
        Ok(slide::legacy_shape_id(control, &doc.interner).map(str::to_owned))
    }

    /// The COM class id the ActiveX control `control_idx` on `surface` names (`ax:ocx@ax:classid`),
    /// or `None` when there is no such control or its part states none.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or a relationship points outside the
    /// package.
    pub fn activex_class_id(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        self.activex_ocx_attribute(surface.into(), control_idx, "classid")
    }

    /// How the ActiveX control `control_idx` on `surface` persists its state
    /// (`ax:ocx@ax:persistence`), or `None` when there is no such control, its part states none, or
    /// it names a value the ActiveX part does not define.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or a relationship points outside the
    /// package.
    pub fn activex_persistence(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<Option<ActiveXPersistence>, PptxError> {
        Ok(self
            .activex_ocx_attribute(surface.into(), control_idx, "persistence")?
            .as_deref()
            .and_then(ActiveXPersistence::from_wire))
    }

    /// Renames the ActiveX control `control_idx` on `surface` (`p:control@name`). Only the surface's
    /// part is dirtied.
    ///
    /// # Errors
    /// [`PptxError::ActiveXControlOutOfRange`] if the surface has no such control, or another
    /// [`PptxError`] if the surface index is out of range.
    pub fn set_activex_control_name(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
        name: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let count = self.activex_control_count(surface)?;
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let control = slide::nth_control_mut(root, interner, control_idx).ok_or(
            PptxError::ActiveXControlOutOfRange {
                index: control_idx,
                count,
            },
        )?;
        let attribute = build::attr(interner, "name", name);
        set_or_replace_attr(&mut control.attributes, interner, "name", attribute);
        Ok(())
    }

    /// Replaces the persisted state of the ActiveX control `control_idx` on `surface`, in place.
    ///
    /// The slide and the control part are untouched — only the `.bin` changes.
    ///
    /// # Errors
    /// [`PptxError::ActiveXControlOutOfRange`] if the surface has no such control, or if that control
    /// persists no state (there is then no part to replace).
    pub fn set_activex_state(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
        state: &[u8],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let count = self.activex_control_count(surface)?;
        let out_of_range = || PptxError::ActiveXControlOutOfRange {
            index: control_idx,
            count,
        };
        let activex_part = self
            .activex_part(surface, control_idx)?
            .ok_or_else(out_of_range)?;
        let binary_part = self
            .follow_rel(&activex_part, constants::REL_ACTIVEX_CONTROL_BINARY)?
            .ok_or_else(out_of_range)?;
        self.package
            .replace_part_bytes(&binary_part, state.to_vec())?;
        Ok(())
    }

    /// Replaces the fallback snapshot image of the ActiveX control `control_idx` on `surface` — the
    /// picture a consumer draws in place of the control it will never run.
    ///
    /// As [`set_ole_snapshot_image`](Self::set_ole_snapshot_image): a new image part is stored and the
    /// control's existing image relationship is retargeted at it, so the slide is not dirtied.
    ///
    /// # Errors
    /// [`PptxError::ActiveXControlOutOfRange`] if the surface has no such control or it carries no
    /// snapshot, [`PptxError::UnrecognizedImageFormat`] if the bytes match no known image format, or
    /// another [`PptxError`] if the surface index is out of range.
    pub fn set_activex_snapshot_image(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
        bytes: &[u8],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let count = self.activex_control_count(surface)?;
        let rel_id = self.activex_snapshot_rel_id(surface, control_idx)?.ok_or(
            PptxError::ActiveXControlOutOfRange {
                index: control_idx,
                count,
            },
        )?;
        self.retarget_image_relationship(surface, &rel_id, bytes)
    }

    /// Removes the ActiveX control `control_idx` from `surface`, closing the gap in the control index
    /// space. Only the surface's part is dirtied.
    ///
    /// As with [`remove_shape`](Self::remove_shape), the parts the control used — its `ax:ocx`, its
    /// `.bin`, its snapshot image — are left in place; sweep them with
    /// [`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts) if wanted.
    ///
    /// # Errors
    /// [`PptxError::ActiveXControlOutOfRange`] if the surface has no such control, or another
    /// [`PptxError`] if the surface index is out of range.
    pub fn remove_activex_control(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let count = self.activex_control_count(surface)?;
        let out_of_range = PptxError::ActiveXControlOutOfRange {
            index: control_idx,
            count,
        };
        if control_idx >= count {
            return Err(out_of_range);
        }
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let c_sld = nav::child_mut(root, interner, PML, "cSld")
            .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
        let controls = nav::child_mut(c_sld, interner, PML, "controls")
            .ok_or(PptxError::MalformedSlide("missing p:controls"))?;
        let position = slide::nth_control_position(controls, interner, control_idx)
            .ok_or(PptxError::MalformedSlide("missing p:control"))?;
        controls.children.remove(position);
        // Take the control's own indentation with it, so removals do not pile up blank lines.
        if position > 0 && nav::is_whitespace_text(&controls.children[position - 1]) {
            controls.children.remove(position - 1);
        }
        Ok(())
    }

    /// The `ax:ocx` attribute `local` of the control `control_idx` on `surface`, or `None`.
    fn activex_ocx_attribute(
        &mut self,
        surface: Surface,
        control_idx: usize,
        local: &str,
    ) -> Result<Option<String>, PptxError> {
        let Some(part) = self.activex_part(surface, control_idx)? else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&part)?;
        let interner = &doc.interner;
        // `ax:classid` and `ax:persistence` are namespace-qualified, and the reader leaves an
        // attribute's prefix unresolved — so the prefix the part binds to the ActiveX namespace is
        // looked up rather than assumed. A part that binds it to nothing states no such attribute.
        let Some(prefix) = nav::namespace_prefix(&doc.root, interner, ACTIVEX) else {
            return Ok(None);
        };
        nav::prefixed_attr_value(&doc.root, interner, prefix, local).transpose()
    }

    // -----------------------------------------------------------------------------------------
    // Legacy VML — resolving the shape an OLE object or a control points at
    // -----------------------------------------------------------------------------------------

    /// The `spid` the OLE frame `shape_idx` on `surface` names — the `id` of the VML shape that draws
    /// it in a legacy consumer — or `None` when the shape frames no OLE object or names no `spid`.
    ///
    /// Resolve it against the surface's VML drawing with `with_vml_shape_for_ole_object`, which the
    /// `vml` crate feature adds.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the surface is malformed.
    pub fn ole_legacy_shape_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        let Some(object) = slide::ole_object(shape, &doc.interner) else {
            return Ok(None);
        };
        Ok(slide::legacy_shape_id(object, &doc.interner).map(str::to_owned))
    }

    /// The legacy VML drawing part `surface` relates to, or `None` when it has none.
    ///
    /// A surface relates to at most one `vmlDrawing` part, which holds the fallback shapes for every
    /// OLE object and legacy control on it. [`vml_part_names`](Self::vml_part_names) finds every VML
    /// part in the *package*; this answers which one belongs to a given slide.
    ///
    /// Requires the `vml` crate feature.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the relationship points outside
    /// the package.
    #[cfg(feature = "vml")]
    pub fn vml_drawing_part(
        &self,
        surface: impl Into<Surface>,
    ) -> Result<Option<PartName>, PptxError> {
        let slide_part = self.surface_part(surface.into())?;
        self.follow_rel(&slide_part, mjx_vml::REL_VML_DRAWING)
    }

    /// Reads the VML drawing `part` as a typed [`Drawing`](mjx_vml::Drawing) and hands it, with the
    /// part's interner, to `read`. Does **not** dirty the part.
    ///
    /// The interner comes with the drawing because a VML name is an interned symbol: it means nothing
    /// outside the part it was read from, so the drawing cannot outlive it. That is why this is a
    /// closure rather than a getter.
    ///
    /// Requires the `vml` crate feature.
    ///
    /// # Errors
    /// [`PptxError::PartIsNotVmlDrawing`] if `part` is not a VML drawing, or another [`PptxError`] if
    /// the package has no such part or it is not well-formed XML.
    #[cfg(feature = "vml")]
    pub fn with_vml_drawing<R>(
        &mut self,
        part: &PartName,
        read: impl FnOnce(&mjx_vml::Drawing, &Interner) -> R,
    ) -> Result<R, PptxError> {
        self.check_is_vml(part)?;
        let doc = self.package.part_tree(part)?;
        let drawing = mjx_vml::Drawing::from_xml(&doc.root, &doc.interner)?;
        Ok(read(&drawing, &doc.interner))
    }

    /// Parses the VML drawing `part`, hands the whole [`Drawing`](mjx_vml::Drawing) to `edit`, and
    /// writes the mutated tree back — dirtying **only** that part.
    ///
    /// Everything the model does not name rides through unchanged, so an edit to one shape leaves its
    /// siblings and every unmodelled child exactly as they were.
    ///
    /// Requires the `vml` crate feature.
    ///
    /// # Errors
    /// [`PptxError::PartIsNotVmlDrawing`] if `part` is not a VML drawing, or another [`PptxError`] if
    /// the package has no such part or it is not well-formed XML.
    #[cfg(feature = "vml")]
    pub fn edit_vml_drawing<R>(
        &mut self,
        part: &PartName,
        edit: impl FnOnce(&mut mjx_vml::Drawing, &mut Interner) -> R,
    ) -> Result<R, PptxError> {
        self.check_is_vml(part)?;
        let doc = self.package.part_tree_mut(part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut drawing = mjx_vml::Drawing::from_xml(root, interner)?;
        let outcome = edit(&mut drawing, interner);
        *root = drawing.to_xml(interner);
        Ok(outcome)
    }

    /// Stores `drawing` as a new legacy VML drawing part and relates it to `surface`, returning the
    /// part's name.
    ///
    /// The `vml` content-type Default is registered if the package has none, and the part is named
    /// `ppt/drawings/vmlDrawingN.vml` with `N` one past the largest already present. Build the bytes
    /// with [`mjx_vml::DrawingPart`].
    ///
    /// Requires the `vml` crate feature.
    ///
    /// # Errors
    /// [`PptxError::Xml`] if `drawing` is not well-formed XML — a VML part is XML, and storing bytes
    /// that are not would make the package unopenable — or another [`PptxError`] if the surface index
    /// is out of range or a package edit fails.
    #[cfg(feature = "vml")]
    pub fn add_vml_drawing(
        &mut self,
        surface: impl Into<Surface>,
        drawing: &[u8],
    ) -> Result<PartName, PptxError> {
        // Parsed and thrown away: this only proves the bytes are XML before they enter the package.
        mjx_xml::fidelity::parse(drawing)?;
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let part = self.next_part_in("drawings", "vmlDrawing", mjx_vml::VML_DEFAULT_EXTENSION)?;
        // Registering the Default first means `insert_part` adds no per-part Override, which is how
        // Office writes it — a `.vml` extension is not shared with any other content type.
        self.package
            .set_content_type_default(mjx_vml::VML_DEFAULT_EXTENSION, mjx_vml::CONTENT_TYPE_VML)?;
        self.package
            .insert_part(&part, mjx_vml::CONTENT_TYPE_VML, drawing.to_vec())?;
        self.relate(&slide_part, &part, mjx_vml::REL_VML_DRAWING)?;
        Ok(part)
    }

    /// Resolves the OLE frame `shape_idx` on `surface` to the VML shape that draws it, and hands that
    /// shape to `read`.
    ///
    /// This is the hop the legacy fallback needs: `p:oleObj@spid` names an `id`, that `id` belongs to
    /// a `v:shape` in the VML drawing the same surface relates to, and this walks it. `None` when the
    /// shape frames no OLE object, names no `spid`, the surface has no VML drawing, or the drawing
    /// holds no shape with that id. Reading does not dirty anything.
    ///
    /// Requires the `vml` crate feature.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, or the VML part is malformed.
    #[cfg(feature = "vml")]
    pub fn with_vml_shape_for_ole_object<R>(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        read: impl FnOnce(&mjx_vml::Shape, &Interner) -> R,
    ) -> Result<Option<R>, PptxError> {
        let surface = surface.into();
        let Some(identifier) = self.ole_legacy_shape_id(surface, shape_idx)? else {
            return Ok(None);
        };
        self.with_vml_shape(surface, &identifier, read)
    }

    /// Resolves the ActiveX control `control_idx` on `surface` to the VML shape that draws it, and
    /// hands that shape to `read`. As
    /// [`with_vml_shape_for_ole_object`](Self::with_vml_shape_for_ole_object), from `p:control@spid`.
    ///
    /// Requires the `vml` crate feature.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range, or the VML part is malformed.
    #[cfg(feature = "vml")]
    pub fn with_vml_shape_for_activex_control<R>(
        &mut self,
        surface: impl Into<Surface>,
        control_idx: usize,
        read: impl FnOnce(&mjx_vml::Shape, &Interner) -> R,
    ) -> Result<Option<R>, PptxError> {
        let surface = surface.into();
        let Some(identifier) = self.activex_control_shape_id(surface, control_idx)? else {
            return Ok(None);
        };
        self.with_vml_shape(surface, &identifier, read)
    }

    /// Hands the VML shape with `identifier` in `surface`'s VML drawing to `read`, or answers `None`
    /// when the surface has no VML drawing or the drawing holds no such shape.
    #[cfg(feature = "vml")]
    fn with_vml_shape<R>(
        &mut self,
        surface: Surface,
        identifier: &str,
        read: impl FnOnce(&mjx_vml::Shape, &Interner) -> R,
    ) -> Result<Option<R>, PptxError> {
        let Some(part) = self.vml_drawing_part(surface)? else {
            return Ok(None);
        };
        self.with_vml_drawing(&part, |drawing, interner| {
            drawing
                .shape_by_identifier(interner, identifier)
                .map(|shape| read(shape, interner))
        })
    }

    /// Rejects a part that is not a legacy VML drawing, so a caller cannot hand a slide to the VML
    /// model and get nonsense back.
    #[cfg(feature = "vml")]
    fn check_is_vml(&self, part: &PartName) -> Result<(), PptxError> {
        if self
            .package
            .content_type_of(part)
            .is_some_and(is_vml_content_type)
        {
            return Ok(());
        }
        Err(PptxError::PartIsNotVmlDrawing {
            part: part.as_str().to_owned(),
        })
    }

    /// The number the next SmartArt diagram's four parts share — one past the largest `dataN.xml`
    /// already under `ppt/diagrams/`. The four are numbered together so a diagram's parts stay a
    /// recognizable set, exactly as Office numbers them.
    fn next_diagram_number(&self) -> u32 {
        let directory = format!("{}diagrams/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            for stem in ["data", "layout", "quickStyle", "colors"] {
                if let Some(n) = stem_number(part.as_str(), &directory, stem) {
                    max_n = max_n.max(n);
                }
            }
        }
        max_n + 1
    }

    /// The part name of one document of diagram `number`.
    fn diagram_part_name(&self, stem: &str, number: u32) -> Result<PartName, PptxError> {
        let directory = format!("{}diagrams/", dir_of(self.presentation_part.as_str()));
        PartName::new(&format!("{directory}{stem}{number}.xml")).map_err(PptxError::from)
    }

    /// The number the next ActiveX control's part and `.bin` share — one past the largest already
    /// under `ppt/activeX/`, counting both extensions so the pair never straddles two numbers.
    fn next_activex_number(&self) -> u32 {
        let directory = format!("{}activeX/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = stem_number(part.as_str(), &directory, "activeX") {
                max_n = max_n.max(n);
            }
        }
        max_n + 1
    }

    /// The part name of ActiveX control `number` with the given extension (`xml` or `bin`).
    fn activex_part_name(&self, number: u32, extension: &str) -> Result<PartName, PptxError> {
        let directory = format!("{}activeX/", dir_of(self.presentation_part.as_str()));
        PartName::new(&format!("{directory}activeX{number}.{extension}")).map_err(PptxError::from)
    }
}

/// The ActiveX control part's namespace as a [`SchemaNamespace`], so `nav`'s prefix lookup can find
/// whichever prefix a part binds it to. A Microsoft extension with no Strict variant.
const ACTIVEX: SchemaNamespace = SchemaNamespace {
    transitional: constants::ACTIVEX_NAMESPACE,
    strict: None,
};

/// Rejects bytes that are not an InkML document.
///
/// An ink part is registered under the `application/inkml+xml` content type, so a package that
/// declares InkML and carries something else is malformed the moment it is saved. Checked by parsing
/// and looking at the root element's namespace — the cheapest check that is actually a check.
fn check_inkml(bytes: &[u8]) -> Result<(), PptxError> {
    let document = mjx_xml::fidelity::parse(bytes).map_err(|_| PptxError::InvalidInkContent)?;
    let namespace = document
        .root
        .name
        .namespace
        .map(|symbol| document.interner.resolve(symbol));
    if namespace == Some(constants::INKML_NAMESPACE) {
        return Ok(());
    }
    Err(PptxError::InvalidInkContent)
}

/// Sets an unprefixed attribute, rewriting the existing one in place — so attribute order, and every
/// other attribute, is preserved — or appending `built` when the element has none.
fn set_or_replace_attr(
    attributes: &mut Vec<RawAttribute>,
    interner: &Interner,
    local: &str,
    built: RawAttribute,
) {
    if let Some(existing) = attributes
        .iter_mut()
        .find(|attr| attr.name.prefix.is_none() && interner.resolve(attr.name.local) == local)
    {
        existing.value = built.value;
        return;
    }
    attributes.push(built);
}

/// A `p:contentPart` (`CT_Rel`) referencing `rel_id` — how a shape tree points at an ink part.
fn build_content_part(
    interner: &mut Interner,
    rel_id: &str,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    let mut attributes = Vec::with_capacity(2);
    if let Some(declaration) = rel_declaration {
        attributes.push(declaration);
    }
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    attributes.push(build::attr_prefixed(interner, rel_prefix, "id", rel_id));
    build::leaf(interner, "p", PML, "contentPart", attributes)
}

/// A whole `p:graphicFrame` framing a SmartArt diagram: the frame furniture plus a `dgm:relIds`
/// naming the diagram's four parts (data, layout, quick style, colours — in that order).
fn build_diagram_frame(
    interner: &mut Interner,
    id: u32,
    rel_ids: &[String],
    bounds: ShapeBounds,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    let nv_frame_pr = build_nv_graphic_frame_pr(interner, id, &format!("Diagram {id}"));
    let mut xfrm = build::node(interner, "p", PML, "xfrm", Vec::new(), Vec::new());
    bounds.to_transform().apply(&mut xfrm, interner);

    let mut attributes = vec![build::namespace_declaration(
        interner,
        "dgm",
        DML_DIAGRAM.transitional,
    )];
    if let Some(declaration) = rel_declaration {
        attributes.push(declaration);
    }
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    // The schema requires all four; they are written in the order ECMA-376 Part 1 §21.4.2.22 lists.
    for (local, rel_id) in ["dm", "lo", "qs", "cs"].into_iter().zip(rel_ids) {
        attributes.push(build::attr_prefixed(interner, rel_prefix, local, rel_id));
    }
    let rel_ids_element = build::leaf(interner, "dgm", DML_DIAGRAM, "relIds", attributes);

    let data_attrs = vec![build::attr(interner, "uri", slide::DIAGRAM_GRAPHIC_URI)];
    let graphic_data = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphicData",
        data_attrs,
        vec![RawNode::Element(rel_ids_element)],
    );
    let graphic = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphic",
        Vec::new(),
        vec![RawNode::Element(graphic_data)],
    );
    build::node(
        interner,
        "p",
        PML,
        "graphicFrame",
        Vec::new(),
        vec![
            RawNode::Element(nv_frame_pr),
            RawNode::Element(xfrm),
            RawNode::Element(graphic),
        ],
    )
}

/// What [`build_ole_frame`] needs to name: the two relationships, the owning application, and how the
/// object presents itself. Grouped so the builder takes one argument rather than six.
struct OleFrameParts<'a> {
    /// The relationship naming the object's data.
    data_rel_id: &'a str,
    /// The relationship naming the snapshot image drawn in the object's place.
    snapshot_rel_id: &'a str,
    /// The `progId` of the application that owns the object.
    prog_id: &'a str,
    /// The object's display name, or `None`.
    name: Option<&'a str>,
    /// Whether the object is drawn as an icon rather than its content.
    show_as_icon: bool,
    /// Whether the data is linked rather than embedded — which of `p:link` / `p:embed` is written.
    linked: bool,
}

/// A whole `p:graphicFrame` framing an OLE object: the frame furniture, a `p:oleObj` naming the
/// object's data, and the `p:pic` snapshot a consumer draws in its place.
fn build_ole_frame(
    interner: &mut Interner,
    id: u32,
    parts: &OleFrameParts<'_>,
    bounds: ShapeBounds,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    let nv_frame_pr = build_nv_graphic_frame_pr(interner, id, &format!("Object {id}"));
    let mut xfrm = build::node(interner, "p", PML, "xfrm", Vec::new(), Vec::new());
    bounds.to_transform().apply(&mut xfrm, interner);

    let mut ole_attrs = Vec::with_capacity(6);
    if let Some(name) = parts.name {
        ole_attrs.push(build::attr(interner, "name", name));
    }
    if parts.show_as_icon {
        ole_attrs.push(build::attr(interner, "showAsIcon", "1"));
    }
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    ole_attrs.push(build::attr_prefixed(
        interner,
        rel_prefix,
        "id",
        parts.data_rel_id,
    ));
    // `a:ST_PositiveCoordinate32` is unsigned, so a frame with no extent simply states none.
    if let Ok(width) = u32::try_from(bounds.width_emu) {
        ole_attrs.push(build::attr(interner, "imgW", &width.to_string()));
    }
    if let Ok(height) = u32::try_from(bounds.height_emu) {
        ole_attrs.push(build::attr(interner, "imgH", &height.to_string()));
    }
    ole_attrs.push(build::attr(interner, "progId", parts.prog_id));

    let binding_local = if parts.linked { "link" } else { "embed" };
    let binding = build::leaf(interner, "p", PML, binding_local, Vec::new());
    // The snapshot picture is a shape of the slide like any other, so it takes the id after the
    // frame's rather than a constant: two OLE objects on one slide would otherwise both write a
    // picture with the same non-visual id, which is a duplicate PowerPoint repairs. The caller
    // allocates from `max_cnvpr_id`, which sees this one, so the next frame starts past it.
    let picture = build_picture(interner, id + 1, parts.snapshot_rel_id, bounds, None);
    let ole_object = build::node(
        interner,
        "p",
        PML,
        "oleObj",
        ole_attrs,
        vec![RawNode::Element(binding), RawNode::Element(picture)],
    );

    let mut data_attrs = vec![build::attr(interner, "uri", slide::OLE_GRAPHIC_URI)];
    if let Some(declaration) = rel_declaration {
        data_attrs.push(declaration);
    }
    let graphic_data = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphicData",
        data_attrs,
        vec![RawNode::Element(ole_object)],
    );
    let graphic = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphic",
        Vec::new(),
        vec![RawNode::Element(graphic_data)],
    );
    build::node(
        interner,
        "p",
        PML,
        "graphicFrame",
        Vec::new(),
        vec![
            RawNode::Element(nv_frame_pr),
            RawNode::Element(xfrm),
            RawNode::Element(graphic),
        ],
    )
}

/// A `p:control` (`CT_Control`) naming an ActiveX control part and carrying the `p:pic` snapshot a
/// consumer draws in the control's place.
fn build_control(
    interner: &mut Interner,
    id: u32,
    name: &str,
    control_rel_id: &str,
    snapshot_rel_id: &str,
    bounds: ShapeBounds,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    let mut attributes = Vec::with_capacity(6);
    if let Some(declaration) = rel_declaration {
        attributes.push(declaration);
    }
    attributes.push(build::attr(interner, "name", name));
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    attributes.push(build::attr_prefixed(
        interner,
        rel_prefix,
        "id",
        control_rel_id,
    ));
    if let Ok(width) = u32::try_from(bounds.width_emu) {
        attributes.push(build::attr(interner, "imgW", &width.to_string()));
    }
    if let Ok(height) = u32::try_from(bounds.height_emu) {
        attributes.push(build::attr(interner, "imgH", &height.to_string()));
    }
    let picture = build_picture(interner, id, snapshot_rel_id, bounds, None);
    build::node(
        interner,
        "p",
        PML,
        "control",
        attributes,
        vec![RawNode::Element(picture)],
    )
}
