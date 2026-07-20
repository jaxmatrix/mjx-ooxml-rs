//! The [`Presentation`] entry point: open, read shape text, edit a run, save.

use mjx_dml::{
    resolve_color, resolve_effects, resolve_fill, resolve_line, ColorMap, EffectList,
    EffectListSpec, Fill, FillSpec, LineProperties, LineSpec, PresetGeometry, ResolvedColor,
    SchemeColors, ShapeGeometry, TextBody, Theme, ThemeInfo,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces::{DML_MAIN, PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_opc::{ImageFormat, Package, PartName, Relationship, TargetMode};

use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::slide::ShapeKind;
use crate::{build, constants, nav, slide};

/// An open PresentationML document: an OPC [`Package`] plus its resolved presentation part and the
/// ordered list of slide parts.
///
/// Reads and edits are addressed by index (`slide_idx`, `shape_idx`, `run_idx`). Reading a part never
/// dirties it; editing marks only the one slide part dirty, so [`save`](Self::save) re-emits every
/// other part byte-identically.
#[derive(Debug)]
pub struct Presentation {
    package: Package,
    presentation_part: PartName,
    slides: Vec<PartName>,
}

impl Presentation {
    /// Opens a presentation from its container bytes, resolving the presentation part and the ordered
    /// slide parts.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the package is unreadable, has no `officeDocument` relationship, or its
    /// `presentation.xml` / relationships are malformed.
    pub fn open(bytes: &[u8]) -> Result<Self, PptxError> {
        let mut package = Package::open(bytes)?;

        // Package root -> officeDocument relationship -> the presentation part.
        let presentation_part = {
            let root_rels = package
                .relationships_for(None)
                .ok_or(PptxError::MissingOfficeDocument)?;
            let rel = root_rels
                .by_type(constants::REL_OFFICE_DOCUMENT)
                .next()
                .ok_or(PptxError::MissingOfficeDocument)?;
            if rel.mode == TargetMode::External {
                return Err(PptxError::ExternalTarget {
                    target: rel.target.clone(),
                });
            }
            nav::resolve_from_root(&rel.target)?
        };
        if package.part_bytes(&presentation_part).is_none() {
            return Err(PptxError::MissingPresentationPart(
                presentation_part.as_str().to_owned(),
            ));
        }

        // presentation.xml -> p:sldIdLst -> each p:sldId's r:id (owned, so the tree borrow ends here).
        let slide_rids: Vec<String> = {
            let doc = package.part_tree(&presentation_part)?;
            let interner = &doc.interner;
            let root = &doc.root;
            let rels_prefix = nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)
                .ok_or(PptxError::MalformedPresentation(
                    "no relationships namespace declared",
                ))?;
            let sld_id_lst = nav::child(root, interner, PML, "sldIdLst")
                .ok_or(PptxError::MalformedPresentation("missing p:sldIdLst"))?;
            let mut rids = Vec::new();
            for sld_id in nav::children(sld_id_lst, interner, PML, "sldId") {
                let rid = nav::prefixed_attr_value(sld_id, interner, rels_prefix, "id")
                    .ok_or(PptxError::MalformedPresentation("p:sldId has no r:id"))??;
                rids.push(rid);
            }
            rids
        };

        // Resolve each r:id against presentation.xml.rels into a slide PartName.
        let slides = {
            let pres_rels = package.relationships_for(Some(&presentation_part)).ok_or(
                PptxError::MalformedPresentation("presentation has no relationships"),
            )?;
            let mut slides = Vec::with_capacity(slide_rids.len());
            for rid in &slide_rids {
                let rel = pres_rels
                    .by_id(rid)
                    .ok_or_else(|| PptxError::SlideRelNotFound { id: rid.clone() })?;
                if rel.mode == TargetMode::External {
                    return Err(PptxError::ExternalTarget {
                        target: rel.target.clone(),
                    });
                }
                slides.push(nav::resolve_target(&presentation_part, &rel.target)?);
            }
            slides
        };

        Ok(Self {
            package,
            presentation_part,
            slides,
        })
    }

    /// Serializes the presentation back to container bytes (only edited parts re-serialize).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, PptxError> {
        Ok(self.package.save()?)
    }

    /// The part name of the main presentation part (`/ppt/presentation.xml`).
    #[must_use]
    pub fn presentation_part(&self) -> &PartName {
        &self.presentation_part
    }

    /// The number of slides, in presentation order.
    #[must_use]
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    /// The part name of slide `idx` (does not touch the package).
    #[must_use]
    pub fn slide_part(&self, idx: usize) -> Option<&PartName> {
        self.slides.get(idx)
    }

    /// The number of shapes on slide `slide_idx` — of **every** [`ShapeKind`] (autoshapes, pictures,
    /// groups, graphic frames, connectors), in document order. A group counts as one shape; its
    /// members are not separately addressable.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range or the slide is malformed.
    pub fn shape_count(&mut self, slide_idx: usize) -> Result<usize, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        Ok(slide::shapes(sp_tree, &doc.interner).count())
    }

    /// What kind of shape `shape_idx` on slide `slide_idx` is — which of the index-addressed APIs
    /// apply to it (a [`Picture`](ShapeKind::Picture) takes the `p:spPr` surface but has no text body;
    /// a [`GroupShape`](ShapeKind::GroupShape) has no `p:spPr` at all).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn shape_kind(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<ShapeKind, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        slide::shape_kind(shape, &doc.interner)
            .ok_or(PptxError::MalformedSlide("shape tree child is not a shape"))
    }

    /// The full text of shape `shape_idx` on slide `slide_idx` (paragraphs joined by `\n`).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body ([`ShapeHasNoTextBody`](PptxError::ShapeHasNoTextBody) — a picture or group never
    /// has one).
    pub fn shape_text(&mut self, slide_idx: usize, shape_idx: usize) -> Result<String, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let txbody =
            slide::shape_txbody(shape, &doc.interner).ok_or(PptxError::ShapeHasNoTextBody)?;
        let body = TextBody::from_xml(txbody, &doc.interner)?;
        Ok(body.text())
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in document
    /// order) of shape `shape_idx` on slide `slide_idx`. Marks only that slide part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, the shape has no
    /// text body, or the selected run has no `a:t`.
    pub fn set_shape_text(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
        run_idx: usize,
        text: &str,
    ) -> Result<(), PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` for name resolution / rebuild, `root` for locate + replace.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let slot =
            nav::child_mut(shape, interner, PML, "txBody").ok_or(PptxError::ShapeHasNoTextBody)?;

        let mut body = TextBody::from_xml(slot, interner)?;
        let run_count = body
            .paragraphs()
            .flat_map(|paragraph| paragraph.runs())
            .count();
        let run = body
            .paragraphs_mut()
            .flat_map(|paragraph| paragraph.runs_mut())
            .nth(run_idx)
            .ok_or(PptxError::RunIndexOutOfRange {
                index: run_idx,
                count: run_count,
            })?;
        if !run.set_text(text) {
            return Err(PptxError::RunHasNoText);
        }
        // The edit lands here: rebuild the txBody in place, reusing the part's own interner.
        *slot = body.to_xml(interner);
        Ok(())
    }

    /// The preset geometry of shape `shape_idx` on slide `slide_idx`, as a typed [`ShapeGeometry`]
    /// (named adjustments in friendly units). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, the shape has no
    /// `a:prstGeom` ([`ShapeHasNoGeometry`](PptxError::ShapeHasNoGeometry)), or its `prst` names a
    /// shape type this build does not recognize ([`UnknownShapeType`](PptxError::UnknownShapeType)).
    pub fn shape_geometry(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<ShapeGeometry, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let prst_geom =
            slide::shape_prstgeom(shape, &doc.interner).ok_or(PptxError::ShapeHasNoGeometry)?;
        let geometry = PresetGeometry::from_xml(prst_geom, &doc.interner)?;
        geometry
            .shape(&doc.interner)
            .ok_or(PptxError::UnknownShapeType)
    }

    /// Sets the preset geometry of shape `shape_idx` on slide `slide_idx` from a typed
    /// [`ShapeGeometry`] — rewriting the shape's `a:prstGeom@prst` and its adjustment `a:gd`s. Marks
    /// only that slide part dirty; everything else re-emits verbatim.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `a:prstGeom` to edit.
    pub fn set_shape_geometry(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
        geometry: ShapeGeometry,
    ) -> Result<(), PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` for name resolution / rebuild, `root` for locate + replace.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoGeometry)?;
        let slot = nav::child_mut(sp_pr, interner, DML_MAIN, "prstGeom")
            .ok_or(PptxError::ShapeHasNoGeometry)?;

        let mut geom = PresetGeometry::from_xml(slot, interner)?;
        geom.set_shape(interner, geometry);
        // The edit lands here: rebuild the prstGeom in place, reusing the part's own interner.
        *slot = geom.to_xml(interner);
        Ok(())
    }

    /// The explicit fill of shape `shape_idx` on slide `slide_idx`, as an interner-free [`FillSpec`],
    /// or `None` if the shape declares no fill in its `p:spPr` (its fill is then inherited from the
    /// placeholder / style / theme — resolving that is a separate, future task). Reading does not
    /// dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the fill element
    /// is not well-formed.
    pub fn shape_fill(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        match slide::shape_fill(shape, &doc.interner) {
            Some(fill) => {
                let fill = Fill::from_xml(fill, &doc.interner)?;
                Ok(Some(fill.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the fill of shape `shape_idx` on slide `slide_idx` from an interner-free [`FillSpec`],
    /// rebuilding the `p:spPr` fill element (replacing an existing one in place, or inserting a new
    /// one after any geometry and before `a:ln`). Marks only that slide part dirty.
    ///
    /// A [`FillSpec::Blip`] writes only the `a:blip@r:embed` reference; the image part and its
    /// relationship must already exist in the package — create both with
    /// [`add_image`](Self::add_image), which returns the id to use.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_fill(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the fill element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        // A picture fill carries an `r:embed`, so the built element must be able to resolve the `r`
        // prefix — computed from the part root before the borrow descends into the shape tree.
        let rel_declaration = match fill {
            FillSpec::Blip { .. } => build::relationship_prefix_declaration(root, interner),
            _ => None,
        };
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let mut element = fill.to_fill(interner).to_xml(interner);
        if let Some(declaration) = rel_declaration {
            element.attributes.push(declaration);
        }
        let node = RawNode::Element(element);
        match slide::fill_child_index(sp_pr, interner) {
            Some(index) => sp_pr.children[index] = node,
            None => {
                let at = slide::fill_insert_index(sp_pr, interner);
                sp_pr.children.insert(at, node);
                sp_pr.empty = false;
            }
        }
        Ok(())
    }

    /// Sets shape `shape_idx` on slide `slide_idx` to an explicit "no fill" (`a:noFill`). A shorthand
    /// for [`set_shape_fill`](Self::set_shape_fill) with [`FillSpec::None`].
    ///
    /// # Errors
    /// As [`set_shape_fill`](Self::set_shape_fill).
    pub fn set_shape_no_fill(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<(), PptxError> {
        self.set_shape_fill(slide_idx, shape_idx, &FillSpec::None)
    }

    /// The **explicit** outline of shape `shape_idx` on slide `slide_idx` — its `p:spPr > a:ln` as an
    /// interner-free [`LineSpec`] — or `None` when the shape declares no `a:ln` (its outline is then
    /// inherited; effective outline resolution is a later step). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the outline element
    /// is not well-formed.
    pub fn shape_outline(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<Option<LineSpec>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        match slide::shape_line(shape, &doc.interner) {
            Some(line) => {
                let line = LineProperties::from_xml(line, &doc.interner)?;
                Ok(Some(line.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the outline of shape `shape_idx` on slide `slide_idx` from an interner-free [`LineSpec`],
    /// rebuilding the `p:spPr` `a:ln` element (replacing an existing one in place, or inserting a new
    /// one after any geometry and fill, before effects). Marks only that slide part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_outline(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the outline element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let element = line.to_line(interner).to_xml(interner);
        let node = RawNode::Element(element);
        match slide::line_child_index(sp_pr, interner) {
            Some(index) => sp_pr.children[index] = node,
            None => {
                let at = slide::line_insert_index(sp_pr, interner);
                sp_pr.children.insert(at, node);
                sp_pr.empty = false;
            }
        }
        Ok(())
    }

    /// Sets shape `shape_idx` on slide `slide_idx` to an explicit "no outline" (`<a:ln><a:noFill/></a:ln>`).
    /// A shorthand for [`set_shape_outline`](Self::set_shape_outline) with a [`LineSpec`] whose fill is
    /// [`FillSpec::None`] — PowerPoint's "no line", distinct from an absent `a:ln`.
    ///
    /// # Errors
    /// As [`set_shape_outline`](Self::set_shape_outline).
    pub fn set_shape_no_outline(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<(), PptxError> {
        let line = LineSpec {
            fill: Some(FillSpec::None),
            ..LineSpec::new()
        };
        self.set_shape_outline(slide_idx, shape_idx, &line)
    }

    /// The **explicit** effects of shape `shape_idx` on slide `slide_idx` — its `p:spPr > a:effectLst`
    /// as an interner-free [`EffectListSpec`] — or `None` when the shape declares no `a:effectLst` (its
    /// effects are then inherited; effective effect resolution is a later step). A shape whose effects
    /// use the rarer `a:effectDag` alternative also reads as `None` (that opaque graph is not modeled).
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the effect element
    /// is not well-formed.
    pub fn shape_effects(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<Option<EffectListSpec>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        match slide::shape_effects(shape, &doc.interner) {
            Some(effects) => {
                let effects = EffectList::from_xml(effects, &doc.interner)?;
                Ok(Some(effects.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the effects of shape `shape_idx` on slide `slide_idx` from an interner-free
    /// [`EffectListSpec`], rebuilding the `p:spPr` `a:effectLst` element (replacing an existing effect
    /// container in place — either an `a:effectLst` or the mutually-exclusive `a:effectDag`, which is
    /// overwritten — or inserting a new one after any geometry, fill, and outline, before the 3-D and
    /// extension children). Marks only that slide part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_effects(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
        effects: &EffectListSpec,
    ) -> Result<(), PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the effect element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let element = effects.to_effect_list(interner).to_xml(interner);
        let node = RawNode::Element(element);
        match slide::effect_child_index(sp_pr, interner) {
            Some(index) => sp_pr.children[index] = node,
            None => {
                let at = slide::effect_insert_index(sp_pr, interner);
                sp_pr.children.insert(at, node);
                sp_pr.empty = false;
            }
        }
        Ok(())
    }

    /// Sets shape `shape_idx` on slide `slide_idx` to explicit "no effects" (an empty `<a:effectLst/>`).
    /// A shorthand for [`set_shape_effects`](Self::set_shape_effects) with an empty [`EffectListSpec`] —
    /// the explicitly-cleared effect state that overrides inheritance, distinct from an absent
    /// `a:effectLst`. Reads back as `Some(EffectListSpec::default())`.
    ///
    /// # Errors
    /// As [`set_shape_effects`](Self::set_shape_effects).
    pub fn set_shape_no_effects(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<(), PptxError> {
        self.set_shape_effects(slide_idx, shape_idx, &EffectListSpec::new())
    }

    /// The theme that governs slide `slide_idx`, as an interner-free [`ThemeInfo`] (its color scheme +
    /// fill-style matrix) — resolved by following the relationship chain slide → slideLayout →
    /// slideMaster → theme. Returns `Ok(None)` if any hop in the chain is absent (a deck without a
    /// theme). Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range, a relationship points outside the package
    /// ([`ExternalTarget`](PptxError::ExternalTarget)), or the theme part is not well-formed.
    pub fn slide_theme(&mut self, slide_idx: usize) -> Result<Option<ThemeInfo>, PptxError> {
        let Some(theme_part) = self.slide_theme_part(slide_idx)? else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(theme.to_info(&doc.interner)))
    }

    /// The theme [`PartName`] governing slide `slide_idx`, via slide → slideLayout → slideMaster →
    /// theme; `None` if any hop is absent.
    fn slide_theme_part(&self, slide_idx: usize) -> Result<Option<PartName>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let Some(layout) = self.follow_rel(&slide_part, constants::REL_SLIDE_LAYOUT)? else {
            return Ok(None);
        };
        let Some(master) = self.follow_rel(&layout, constants::REL_SLIDE_MASTER)? else {
            return Ok(None);
        };
        self.follow_rel(&master, constants::REL_THEME)
    }

    /// The **effective** fill of shape `shape_idx` on slide `slide_idx`, as an interner-free
    /// [`FillSpec`] whose colors are resolved to concrete `RRGGBB` values — the fill the shape actually
    /// renders. Three sources are tried, in order: an explicit `p:spPr` fill; a `p:style > a:fillRef`
    /// (the theme fill-style at that index, `phClr` substituted by the reference's color); and, for a
    /// placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the slide layout
    /// then the master. Scheme colors and color transforms are baked against the slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields a fill. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_fill(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        let map = self
            .slide_color_map(slide_idx)?
            .unwrap_or_else(ColorMap::identity);
        let theme_part = self.slide_theme_part(slide_idx)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        // The candidate shapes, in inheritance order: the shape itself, then (if it is a placeholder)
        // the matching placeholder on the layout, then the master.
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let placeholder = {
            let doc = self.package.part_tree(&slide_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let count = slide::shapes(sp_tree, &doc.interner).count();
            let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
                PptxError::ShapeIndexOutOfRange {
                    slide: slide_idx,
                    index: shape_idx,
                    count,
                },
            )?;
            slide::shape_placeholder(shape, &doc.interner)
        };

        let mut candidates = vec![(slide_part.clone(), Candidate::Index(shape_idx))];
        if let Some(ph) = placeholder {
            if let Some(layout) = self.follow_rel(&slide_part, constants::REL_SLIDE_LAYOUT)? {
                if let Some(master) = self.follow_rel(&layout, constants::REL_SLIDE_MASTER)? {
                    candidates.push((master, Candidate::Placeholder(ph)));
                }
                candidates.insert(1, (layout, Candidate::Placeholder(ph)));
            }
        }

        for (part, candidate) in candidates {
            // Extract the candidate's own fill while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
                let shape = match candidate {
                    Candidate::Index(idx) => slide::shapes(sp_tree, &doc.interner).nth(idx),
                    Candidate::Placeholder(ph) => {
                        slide::find_placeholder(sp_tree, ph, &doc.interner)
                    }
                };
                match shape {
                    Some(shape) => shape_own_fill(shape, &doc.interner, &scheme, &map)?,
                    None => OwnFill::Absent,
                }
            };

            match own {
                OwnFill::Resolved(spec) => return Ok(Some(spec)),
                OwnFill::StyleRef(idx, color) => {
                    // Resolve the referenced theme fill-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.fill_style(idx) {
                            return Ok(Some(resolve_fill(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnFill::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** outline of shape `shape_idx` on slide `slide_idx`, as an interner-free
    /// [`LineSpec`] whose stroke color is resolved to a concrete `RRGGBB` value — the outline the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`; a
    /// `p:style > a:lnRef` (the theme line-style at that index, `phClr` substituted by the reference's
    /// color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on
    /// the slide layout then the master. Scheme colors and color transforms are baked against the
    /// slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields an outline. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_outline(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<Option<LineSpec>, PptxError> {
        let map = self
            .slide_color_map(slide_idx)?
            .unwrap_or_else(ColorMap::identity);
        let theme_part = self.slide_theme_part(slide_idx)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        // The candidate shapes, in inheritance order: the shape itself, then (if it is a placeholder)
        // the matching placeholder on the layout, then the master.
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let placeholder = {
            let doc = self.package.part_tree(&slide_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let count = slide::shapes(sp_tree, &doc.interner).count();
            let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
                PptxError::ShapeIndexOutOfRange {
                    slide: slide_idx,
                    index: shape_idx,
                    count,
                },
            )?;
            slide::shape_placeholder(shape, &doc.interner)
        };

        let mut candidates = vec![(slide_part.clone(), Candidate::Index(shape_idx))];
        if let Some(ph) = placeholder {
            if let Some(layout) = self.follow_rel(&slide_part, constants::REL_SLIDE_LAYOUT)? {
                if let Some(master) = self.follow_rel(&layout, constants::REL_SLIDE_MASTER)? {
                    candidates.push((master, Candidate::Placeholder(ph)));
                }
                candidates.insert(1, (layout, Candidate::Placeholder(ph)));
            }
        }

        for (part, candidate) in candidates {
            // Extract the candidate's own outline while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
                let shape = match candidate {
                    Candidate::Index(idx) => slide::shapes(sp_tree, &doc.interner).nth(idx),
                    Candidate::Placeholder(ph) => {
                        slide::find_placeholder(sp_tree, ph, &doc.interner)
                    }
                };
                match shape {
                    Some(shape) => shape_own_line(shape, &doc.interner, &scheme, &map)?,
                    None => OwnLine::Absent,
                }
            };

            match own {
                OwnLine::Resolved(spec) => return Ok(Some(spec)),
                OwnLine::StyleRef(idx, color) => {
                    // Resolve the referenced theme line-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.line_style(idx) {
                            return Ok(Some(resolve_line(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnLine::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** effects of shape `shape_idx` on slide `slide_idx`, as an interner-free
    /// [`EffectListSpec`] whose colors are resolved to concrete `RRGGBB` values — the effects the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:effectLst`; a
    /// `p:style > a:effectRef` (the theme effect-style at that index, `phClr` substituted by the
    /// reference's color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot
    /// placeholder on the slide layout then the master. Scheme colors and color transforms are baked
    /// against the slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields effects. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_effects(
        &mut self,
        slide_idx: usize,
        shape_idx: usize,
    ) -> Result<Option<EffectListSpec>, PptxError> {
        let map = self
            .slide_color_map(slide_idx)?
            .unwrap_or_else(ColorMap::identity);
        let theme_part = self.slide_theme_part(slide_idx)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        // The candidate shapes, in inheritance order: the shape itself, then (if it is a placeholder)
        // the matching placeholder on the layout, then the master.
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let placeholder = {
            let doc = self.package.part_tree(&slide_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let count = slide::shapes(sp_tree, &doc.interner).count();
            let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
                PptxError::ShapeIndexOutOfRange {
                    slide: slide_idx,
                    index: shape_idx,
                    count,
                },
            )?;
            slide::shape_placeholder(shape, &doc.interner)
        };

        let mut candidates = vec![(slide_part.clone(), Candidate::Index(shape_idx))];
        if let Some(ph) = placeholder {
            if let Some(layout) = self.follow_rel(&slide_part, constants::REL_SLIDE_LAYOUT)? {
                if let Some(master) = self.follow_rel(&layout, constants::REL_SLIDE_MASTER)? {
                    candidates.push((master, Candidate::Placeholder(ph)));
                }
                candidates.insert(1, (layout, Candidate::Placeholder(ph)));
            }
        }

        for (part, candidate) in candidates {
            // Extract the candidate's own effects while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
                let shape = match candidate {
                    Candidate::Index(idx) => slide::shapes(sp_tree, &doc.interner).nth(idx),
                    Candidate::Placeholder(ph) => {
                        slide::find_placeholder(sp_tree, ph, &doc.interner)
                    }
                };
                match shape {
                    Some(shape) => shape_own_effects(shape, &doc.interner, &scheme, &map)?,
                    None => OwnEffects::Absent,
                }
            };

            match own {
                OwnEffects::Resolved(spec) => return Ok(Some(*spec)),
                OwnEffects::StyleRef(idx, color) => {
                    // Resolve the referenced theme effect-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.effect_style(idx) {
                            return Ok(Some(resolve_effects(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnEffects::Absent => {}
            }
        }

        Ok(None)
    }

    /// Appends a new rectangular text-box shape (`p:sp`) to slide `slide_idx`, laid out at `bounds`
    /// and containing `text` (one paragraph per line, split on `\n`; an empty line becomes an empty
    /// paragraph). Returns the index of the new shape in the slide's one shape index space (see
    /// [`shape_count`](Self::shape_count)). Only that slide part is marked dirty.
    ///
    /// The shape is a plain text box (`p:cNvSpPr@txBox="1"`, `a:prstGeom@prst="rect"`) with no
    /// placeholder, so it renders as free-standing text. Its non-visual id (`p:cNvPr@id`) is one past
    /// the largest id already present on the slide, keeping ids unique.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range or the slide is malformed.
    pub fn add_text_box(
        &mut self,
        slide_idx: usize,
        text: &str,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the new names, `root` receives the new subtree.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let shape = build_text_box(interner, next_id, text, bounds);
        sp_tree.children.push(RawNode::Element(shape));
        sp_tree.empty = false;

        // The new shape is the last child of the shape tree.
        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Appends a new autoshape (`p:sp`) with the given `preset` geometry to slide `slide_idx`, laid
    /// out at `bounds`, with an empty text body. Returns the index of the new shape in the slide's one
    /// shape index space (see [`shape_count`](Self::shape_count)). Only that slide part is marked dirty.
    ///
    /// The shape is created with the preset's default adjustments; customize them afterward with
    /// [`set_shape_geometry`](Self::set_shape_geometry). Its non-visual id (`p:cNvPr@id`) is one past
    /// the largest id already present on the slide, keeping ids unique.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range or the slide is malformed.
    pub fn add_shape(
        &mut self,
        slide_idx: usize,
        preset: PresetShapeType,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let shape = build_shape(interner, next_id, preset.to_wire(), bounds);
        sp_tree.children.push(RawNode::Element(shape));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0, and
    /// returns its index. The new slide is a blank shape tree; add content with
    /// [`add_text_box`](Self::add_text_box) or use [`add_slide_with_text`](Self::add_slide_with_text).
    ///
    /// This performs the package edits an added slide requires: it inserts the new slide part (with
    /// its content type), synthesizes the slide's relationships (to the layout), adds the
    /// presentation → slide relationship, and appends a `p:sldId` to `p:sldIdLst`. Every pre-existing
    /// part other than `presentation.xml` stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError::NoSlideLayout`] if the deck has no slide to inherit a layout from, or
    /// another [`PptxError`] if `presentation.xml` is malformed or a package edit fails.
    pub fn add_slide(&mut self) -> Result<usize, PptxError> {
        // Inherit slide 0's layout: reuse its relationship target verbatim (the new slide shares the
        // same directory, so the relative target resolves identically).
        let first_slide = self.slides.first().ok_or(PptxError::NoSlideLayout)?.clone();
        let layout_target = {
            let rels = self
                .package
                .relationships_for(Some(&first_slide))
                .ok_or(PptxError::NoSlideLayout)?;
            rels.by_type(constants::REL_SLIDE_LAYOUT)
                .next()
                .ok_or(PptxError::NoSlideLayout)?
                .target
                .clone()
        };

        let new_part = self.next_slide_part()?;
        let new_rid = self.next_presentation_rid()?;
        let slide_target = nav::relative_target(&self.presentation_part, &new_part);

        // 1. Insert the new slide part (registers its content-type Override).
        self.package.insert_part(
            &new_part,
            constants::CONTENT_TYPE_SLIDE,
            build::empty_slide_bytes(),
        )?;
        // 2. Synthesize the new slide's .rels with the slideLayout relationship.
        self.package.add_relationship(
            Some(&new_part),
            Relationship {
                id: "rId1".to_owned(),
                rel_type: constants::REL_SLIDE_LAYOUT.to_owned(),
                target: layout_target,
                mode: TargetMode::Internal,
            },
        )?;
        // 3. Add the presentation → slide relationship.
        self.package.add_relationship(
            Some(&self.presentation_part),
            Relationship {
                id: new_rid.clone(),
                rel_type: constants::REL_SLIDE.to_owned(),
                target: slide_target,
                mode: TargetMode::Internal,
            },
        )?;
        // 4. Append the p:sldId (with its r:id) to p:sldIdLst.
        self.append_sld_id(&new_rid)?;

        self.slides.push(new_part);
        Ok(self.slides.len() - 1)
    }

    /// Stores `bytes` as an image part of the package and relates it to slide `slide_idx`, returning
    /// the **slide-scoped relationship id** that names the image — the `rel_id` to hand to
    /// [`FillSpec::Blip`] via [`set_shape_fill`](Self::set_shape_fill).
    ///
    /// The format is identified from the bytes ([`ImageFormat::sniff`]), which decides the media part's
    /// extension and its content type; the bytes themselves are stored verbatim and never re-encoded.
    /// The part is named `media/image{N}.{ext}` beside the presentation part, with `N` one past the
    /// largest existing image number.
    ///
    /// **Identical images are stored once**: if a media part already holds exactly these bytes it is
    /// reused, and if this slide already relates to it, the existing relationship id is returned and
    /// the package is not touched at all. Otherwise only `[Content_Types].xml`, the new media part, and
    /// this slide's `.rels` change — every other pre-existing part stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError::SlideIndexOutOfRange`] if `slide_idx` is out of range,
    /// [`PptxError::UnrecognizedImageFormat`] if the bytes match no known image format, or another
    /// [`PptxError`] if a package edit fails.
    pub fn add_image(&mut self, slide_idx: usize, bytes: &[u8]) -> Result<String, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let format = ImageFormat::sniff(bytes).ok_or(PptxError::UnrecognizedImageFormat)?;

        let media_part = match self.media_part_with_bytes(bytes) {
            Some(existing) => {
                // Already stored: reuse the slide's relationship to it when there is one.
                if let Some(id) = self.image_rel_id_for(&slide_part, &existing)? {
                    return Ok(id);
                }
                existing
            }
            None => {
                let part = self.next_media_part(format.file_extension())?;
                // Registering the Default first means `insert_part` adds no per-part Override.
                self.package
                    .set_content_type_default(format.file_extension(), format.content_type())?;
                self.package
                    .insert_part(&part, format.content_type(), bytes.to_vec())?;
                part
            }
        };

        let rel_id = self.next_rid_for(&slide_part);
        self.package.add_relationship(
            Some(&slide_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_IMAGE.to_owned(),
                target: nav::relative_target(&slide_part, &media_part),
                mode: TargetMode::Internal,
            },
        )?;
        Ok(rel_id)
    }

    /// The media part whose stored bytes equal `bytes`, if the package already holds one. Comparing
    /// slices short-circuits on length, so this is a cheap scan even for large images.
    fn media_part_with_bytes(&self, bytes: &[u8]) -> Option<PartName> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        self.package
            .part_names()
            .filter(|part| part.as_str().starts_with(&media_dir))
            .find(|part| self.package.part_bytes(part) == Some(bytes))
    }

    /// The id of `source`'s existing [`REL_IMAGE`](constants::REL_IMAGE) relationship pointing at
    /// `target`, or `None` if it has none.
    fn image_rel_id_for(
        &self,
        source: &PartName,
        target: &PartName,
    ) -> Result<Option<String>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(source)) else {
            return Ok(None);
        };
        for rel in rels.by_type(constants::REL_IMAGE) {
            if rel.mode == TargetMode::External {
                continue; // a linked image never names a part in this package
            }
            if &nav::resolve_target(source, &rel.target)? == target {
                return Ok(Some(rel.id.clone()));
            }
        }
        Ok(None)
    }

    /// A fresh image part name in the presentation's `media/` directory: `image{N}.{extension}` with
    /// `N` one past the largest existing image number, whatever its extension.
    fn next_media_part(&self, extension: &str) -> Result<PartName, PptxError> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = image_number(part.as_str(), &media_dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{media_dir}image{}.{extension}", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// Adds a new slide (via [`add_slide`](Self::add_slide)) carrying a single text box with `text`
    /// laid out at `bounds`, and returns the new slide's index.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the slide cannot be added (see [`add_slide`](Self::add_slide)).
    pub fn add_slide_with_text(
        &mut self,
        text: &str,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let idx = self.add_slide()?;
        self.add_text_box(idx, text, bounds)?;
        Ok(idx)
    }

    /// A fresh slide part name in slide 0's directory: `slide{N}.xml` with `N` one past the largest
    /// existing slide number.
    fn next_slide_part(&self) -> Result<PartName, PptxError> {
        let first = self.slides.first().ok_or(PptxError::NoSlideLayout)?;
        let dir = dir_of(first.as_str());
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = slide_number(part.as_str(), dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{dir}slide{}.xml", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// The next free presentation-scoped relationship id (`rId{N}`), one past the current maximum.
    fn next_presentation_rid(&self) -> Result<String, PptxError> {
        if self
            .package
            .relationships_for(Some(&self.presentation_part))
            .is_none()
        {
            return Err(PptxError::MalformedPresentation(
                "presentation has no relationships",
            ));
        }
        Ok(self.next_rid_for(&self.presentation_part))
    }

    /// The next free relationship id (`rId{N}`) in `part`'s `.rels`, one past the current maximum —
    /// `rId1` when the part has no relationships yet (a slide need not have any).
    fn next_rid_for(&self, part: &PartName) -> String {
        let mut max_n = 0u32;
        if let Some(rels) = self.package.relationships_for(Some(part)) {
            for rel in rels.iter() {
                if let Some(n) = rel
                    .id
                    .strip_prefix("rId")
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    max_n = max_n.max(n);
                }
            }
        }
        format!("rId{}", max_n + 1)
    }

    /// Appends `<p:sldId id=".." r:id="new_rid"/>` to `p:sldIdLst`, choosing the next slide id (≥256,
    /// one past the largest existing `p:sldId@id` — masters in `p:sldMasterIdLst` are not considered).
    fn append_sld_id(&mut self, new_rid: &str) -> Result<(), PptxError> {
        let part = self.presentation_part.clone();
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;

        // The `r:id` prefix: attribute namespaces are not resolved by the reader, so find the prefix
        // bound to the relationships namespace.
        let rels_prefix = nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .ok_or(PptxError::MalformedPresentation(
                "no relationships namespace declared",
            ))?;
        let sld_id_lst = nav::child_mut(root, interner, PML, "sldIdLst")
            .ok_or(PptxError::MalformedPresentation("missing p:sldIdLst"))?;

        let mut max_id = 255u32;
        for child in &sld_id_lst.children {
            if let RawNode::Element(element) = child {
                if nav::name_is(&element.name, interner, PML, "sldId") {
                    if let Some(id) = element
                        .attributes
                        .iter()
                        .find(|attr| {
                            attr.name.prefix.is_none() && interner.resolve(attr.name.local) == "id"
                        })
                        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
                        .and_then(|value| value.parse::<u32>().ok())
                    {
                        max_id = max_id.max(id);
                    }
                }
            }
        }
        let new_id = max_id + 1;

        let attrs = vec![
            build::attr(interner, "id", &new_id.to_string()),
            build::attr_prefixed(interner, rels_prefix, "id", new_rid),
        ];
        let sld_id = build::leaf(interner, "p", PML, "sldId", attrs);
        sld_id_lst.children.push(RawNode::Element(sld_id));
        sld_id_lst.empty = false;
        Ok(())
    }

    fn slide_part_checked(&self, slide_idx: usize) -> Result<&PartName, PptxError> {
        self.slides
            .get(slide_idx)
            .ok_or(PptxError::SlideIndexOutOfRange {
                index: slide_idx,
                count: self.slides.len(),
            })
    }

    /// Follows the single relationship of type `rel_type` from `part` to a target [`PartName`], or
    /// `None` if `part` has no such relationship. Errors if the relationship points outside the
    /// package. This is the shared hop used to walk slide → layout → master → theme.
    fn follow_rel(&self, part: &PartName, rel_type: &str) -> Result<Option<PartName>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(part)) else {
            return Ok(None);
        };
        let Some(rel) = rels.by_type(rel_type).next() else {
            return Ok(None);
        };
        if rel.mode == TargetMode::External {
            return Err(PptxError::ExternalTarget {
                target: rel.target.clone(),
            });
        }
        Ok(Some(nav::resolve_target(part, &rel.target)?))
    }

    /// The effective theme [`ColorMap`] for slide `slide_idx`: the slide master's `p:clrMap` (reached
    /// via slide → slideLayout → slideMaster), replaced by the slide's own `p:clrMapOvr >
    /// a:overrideClrMapping` when it supplies a full mapping (a `masterClrMapping`, an absent override,
    /// or a schema-loose attribute-less override all inherit the master's map). It maps the logical
    /// color names a shape may reference (`bg1`/`tx1`/…) to the theme's concrete scheme slots.
    /// `Ok(None)` when there is no reachable master or no `p:clrMap`. Reading does not dirty a part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range, a relationship points outside the package
    /// ([`ExternalTarget`](PptxError::ExternalTarget)), or a part is not well-formed.
    pub fn slide_color_map(&mut self, slide_idx: usize) -> Result<Option<ColorMap>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let Some(layout) = self.follow_rel(&slide_part, constants::REL_SLIDE_LAYOUT)? else {
            return Ok(None);
        };
        let Some(master) = self.follow_rel(&layout, constants::REL_SLIDE_MASTER)? else {
            return Ok(None);
        };

        let base = {
            let doc = self.package.part_tree(&master)?;
            nav::child(&doc.root, &doc.interner, PML, "clrMap")
                .and_then(|clr_map| slide::parse_color_map(clr_map, &doc.interner))
        };
        let Some(base) = base else {
            return Ok(None);
        };

        let doc = self.package.part_tree(&slide_part)?;
        let effective = nav::child(&doc.root, &doc.interner, PML, "clrMapOvr")
            .and_then(|ovr| nav::child(ovr, &doc.interner, DML_MAIN, "overrideClrMapping"))
            .and_then(|mapping| slide::parse_color_map(mapping, &doc.interner))
            .unwrap_or(base);
        Ok(Some(effective))
    }
}

/// How to locate a candidate shape within a part's shape tree during effective-fill resolution.
enum Candidate {
    /// The originally-requested shape, by index (the slide itself).
    Index(usize),
    /// The matching placeholder on an ancestor part (layout / master).
    Placeholder(slide::Placeholder),
}

/// A candidate shape's own fill, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved fill, a theme style reference to resolve against the theme, or
/// no fill.
enum OwnFill {
    /// An explicit `p:spPr` fill, already resolved to concrete colors.
    Resolved(FillSpec),
    /// A `p:style > a:fillRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no fill of its own.
    Absent,
}

/// The fill a `shape` declares itself (explicit `p:spPr` fill, or a `p:style > a:fillRef`), resolved
/// against `scheme` / `map`. The style-reference case returns its index + resolved color for the
/// caller to resolve against the theme (which lives in a different part interner).
fn shape_own_fill(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnFill, PptxError> {
    if let Some(fill_element) = slide::shape_fill(shape, interner) {
        let fill = Fill::from_xml(fill_element, interner)?;
        return Ok(OwnFill::Resolved(resolve_fill(
            &fill, scheme, map, None, interner,
        )));
    }
    if let Some(reference) = slide::shape_fill_ref(shape, interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnFill::StyleRef(idx, color));
        }
    }
    Ok(OwnFill::Absent)
}

/// A candidate shape's own outline, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved outline, a theme style reference to resolve against the theme,
/// or no outline.
enum OwnLine {
    /// An explicit `p:spPr > a:ln`, already resolved to a concrete stroke color.
    Resolved(LineSpec),
    /// A `p:style > a:lnRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no outline of its own.
    Absent,
}

/// The outline a `shape` declares itself (explicit `p:spPr > a:ln`, or a `p:style > a:lnRef`), resolved
/// against `scheme` / `map`. The style-reference case returns its index + resolved color for the caller
/// to resolve against the theme (which lives in a different part interner).
fn shape_own_line(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnLine, PptxError> {
    if let Some(line_element) = slide::shape_line(shape, interner) {
        let line = LineProperties::from_xml(line_element, interner)?;
        return Ok(OwnLine::Resolved(resolve_line(
            &line, scheme, map, None, interner,
        )));
    }
    if let Some(reference) = slide::shape_line_ref(shape, interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnLine::StyleRef(idx, color));
        }
    }
    Ok(OwnLine::Absent)
}

/// A candidate shape's own effects, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved effect list, a theme style reference to resolve against the
/// theme, or no effects.
enum OwnEffects {
    /// An explicit `p:spPr > a:effectLst`, already resolved to concrete colors. Boxed — an
    /// [`EffectListSpec`] is far larger than the other variants.
    Resolved(Box<EffectListSpec>),
    /// A `p:style > a:effectRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no effects of its own.
    Absent,
}

/// The effects a `shape` declares itself (explicit `p:spPr > a:effectLst`, or a `p:style > a:effectRef`),
/// resolved against `scheme` / `map`. The style-reference case returns its index + resolved color for the
/// caller to resolve against the theme (which lives in a different part interner).
fn shape_own_effects(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnEffects, PptxError> {
    if let Some(effect_element) = slide::shape_effects(shape, interner) {
        let effects = EffectList::from_xml(effect_element, interner)?;
        return Ok(OwnEffects::Resolved(Box::new(resolve_effects(
            &effects, scheme, map, None, interner,
        ))));
    }
    if let Some(reference) = slide::shape_effect_ref(shape, interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnEffects::StyleRef(idx, color));
        }
    }
    Ok(OwnEffects::Absent)
}

/// The directory portion of an absolute part name, including the trailing `/` (e.g.
/// `/ppt/slides/slide1.xml` → `/ppt/slides/`).
fn dir_of(part: &str) -> &str {
    let end = part.rfind('/').map_or(0, |idx| idx + 1);
    &part[..end]
}

/// Extracts `N` from a `slide{N}.xml` part directly inside `dir` (e.g. `/ppt/slides/slide3.xml` with
/// `dir = /ppt/slides/` → `3`). Returns `None` for anything else (e.g. the `_rels` subfolder).
fn slide_number(part: &str, dir: &str) -> Option<u32> {
    part.strip_prefix(dir)?
        .strip_prefix("slide")?
        .strip_suffix(".xml")?
        .parse::<u32>()
        .ok()
}

/// Extracts `N` from an `image{N}.{ext}` part directly inside `dir` (e.g. `/ppt/media/image3.png`
/// with `dir = /ppt/media/` → `3`), whatever the extension. Returns `None` for anything else.
fn image_number(part: &str, dir: &str) -> Option<u32> {
    let name = part.strip_prefix(dir)?.strip_prefix("image")?;
    let digits = &name[..name.find('.').unwrap_or(name.len())];
    digits.parse::<u32>().ok()
}

/// The largest `p:cNvPr@id` anywhere under `sp_tree` (0 if none). Non-visual ids are unique per
/// slide, so the next free id is one past this maximum.
fn max_cnvpr_id(sp_tree: &RawElement, interner: &Interner) -> u32 {
    fn walk(element: &RawElement, interner: &Interner, max: &mut u32) {
        if nav::name_is(&element.name, interner, PML, "cNvPr") {
            if let Some(id) = element
                .attributes
                .iter()
                .find(|attr| {
                    attr.name.prefix.is_none() && interner.resolve(attr.name.local) == "id"
                })
                .and_then(|attr| std::str::from_utf8(&attr.value).ok())
                .and_then(|value| value.parse::<u32>().ok())
            {
                *max = (*max).max(id);
            }
        }
        for child in &element.children {
            if let RawNode::Element(child) = child {
                walk(child, interner, max);
            }
        }
    }
    let mut max = 0;
    walk(sp_tree, interner, &mut max);
    max
}

/// Builds a plain text-box `p:sp` with non-visual id `id`, laid out at `bounds`, whose text body
/// holds one paragraph per line of `text`.
/// `p:nvSpPr` — non-visual shape properties: `p:cNvPr@id,name`, `p:cNvSpPr` (with `txBox="1"` iff
/// `tx_box`), and an empty `p:nvPr`.
fn build_nv_sp_pr(interner: &mut Interner, id: u32, name: &str, tx_box: bool) -> RawElement {
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", name),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let cnvsppr_attrs = if tx_box {
        vec![build::attr(interner, "txBox", "1")]
    } else {
        Vec::new()
    };
    let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", cnvsppr_attrs);
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    build::node(
        interner,
        "p",
        PML,
        "nvSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_sp_pr),
            RawNode::Element(nv_pr),
        ],
    )
}

/// `p:spPr` — visual shape properties: an `a:xfrm` transform at `bounds` plus `a:prstGeom@prst` with
/// an empty `a:avLst` (the preset's default adjustments).
fn build_sp_pr(interner: &mut Interner, prst: &str, bounds: ShapeBounds) -> RawElement {
    let off_attrs = vec![
        build::attr(interner, "x", &bounds.offset_x_emu.to_string()),
        build::attr(interner, "y", &bounds.offset_y_emu.to_string()),
    ];
    let off = build::leaf(interner, "a", DML_MAIN, "off", off_attrs);
    let ext_attrs = vec![
        build::attr(interner, "cx", &bounds.width_emu.to_string()),
        build::attr(interner, "cy", &bounds.height_emu.to_string()),
    ];
    let ext = build::leaf(interner, "a", DML_MAIN, "ext", ext_attrs);
    let xfrm = build::node(
        interner,
        "a",
        DML_MAIN,
        "xfrm",
        Vec::new(),
        vec![RawNode::Element(off), RawNode::Element(ext)],
    );
    let av_lst = build::leaf(interner, "a", DML_MAIN, "avLst", Vec::new());
    let prstgeom_attrs = vec![build::attr(interner, "prst", prst)];
    let prst_geom = build::node(
        interner,
        "a",
        DML_MAIN,
        "prstGeom",
        prstgeom_attrs,
        vec![RawNode::Element(av_lst)],
    );
    build::node(
        interner,
        "p",
        PML,
        "spPr",
        Vec::new(),
        vec![RawNode::Element(xfrm), RawNode::Element(prst_geom)],
    )
}

/// A whole `p:sp` text box: `nvSpPr` (`txBox="1"`) + `spPr` (`prst="rect"`) + a `txBody` with one
/// `a:p` per line of `text`.
fn build_text_box(interner: &mut Interner, id: u32, text: &str, bounds: ShapeBounds) -> RawElement {
    let nv_sp_pr = build_nv_sp_pr(interner, id, &format!("TextBox {id}"), true);
    let sp_pr = build_sp_pr(interner, "rect", bounds);

    // p:txBody — required a:bodyPr + a:lstStyle, then one a:p per line.
    let body_pr = build::leaf(interner, "a", DML_MAIN, "bodyPr", Vec::new());
    let lst_style = build::leaf(interner, "a", DML_MAIN, "lstStyle", Vec::new());
    let mut tx_children = vec![RawNode::Element(body_pr), RawNode::Element(lst_style)];
    for line in text.split('\n') {
        tx_children.push(RawNode::Element(build_paragraph(interner, line)));
    }
    let tx_body = build::node(interner, "p", PML, "txBody", Vec::new(), tx_children);

    build::node(
        interner,
        "p",
        PML,
        "sp",
        Vec::new(),
        vec![
            RawNode::Element(nv_sp_pr),
            RawNode::Element(sp_pr),
            RawNode::Element(tx_body),
        ],
    )
}

/// A whole `p:sp` autoshape: `nvSpPr` (no `txBox`) + `spPr` with the `prst` preset geometry + an
/// empty `txBody` (`a:bodyPr`, `a:lstStyle`, one empty `a:p`).
fn build_shape(interner: &mut Interner, id: u32, prst: &str, bounds: ShapeBounds) -> RawElement {
    let nv_sp_pr = build_nv_sp_pr(interner, id, &format!("Shape {id}"), false);
    let sp_pr = build_sp_pr(interner, prst, bounds);

    let body_pr = build::leaf(interner, "a", DML_MAIN, "bodyPr", Vec::new());
    let lst_style = build::leaf(interner, "a", DML_MAIN, "lstStyle", Vec::new());
    let empty_p = build_paragraph(interner, "");
    let tx_body = build::node(
        interner,
        "p",
        PML,
        "txBody",
        Vec::new(),
        vec![
            RawNode::Element(body_pr),
            RawNode::Element(lst_style),
            RawNode::Element(empty_p),
        ],
    );

    build::node(
        interner,
        "p",
        PML,
        "sp",
        Vec::new(),
        vec![
            RawNode::Element(nv_sp_pr),
            RawNode::Element(sp_pr),
            RawNode::Element(tx_body),
        ],
    )
}

/// Builds one `a:p`. An empty line yields an empty paragraph (`<a:p/>`); otherwise a single run
/// (`a:r > a:t`) carrying the line's text.
fn build_paragraph(interner: &mut Interner, line: &str) -> RawElement {
    if line.is_empty() {
        return build::leaf(interner, "a", DML_MAIN, "p", Vec::new());
    }
    let t = build::text_leaf(interner, "a", DML_MAIN, "t", Vec::new(), line);
    let run = build::node(
        interner,
        "a",
        DML_MAIN,
        "r",
        Vec::new(),
        vec![RawNode::Element(t)],
    );
    build::node(
        interner,
        "a",
        DML_MAIN,
        "p",
        Vec::new(),
        vec![RawNode::Element(run)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_dml::{ColorSchemeSlot, SchemeColor};

    fn fixture() -> Vec<u8> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sample.pptx");
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
    }

    #[test]
    fn slide_color_map_resolves_master_mapping() {
        // The fixture master's p:clrMap is the standard mapping (bg1=lt1, tx1=dk1, …), and slide 0
        // has no p:clrMapOvr — so the effective map is the master's.
        let mut pres = Presentation::open(&fixture()).expect("open");
        let map = pres
            .slide_color_map(0)
            .expect("slide_color_map")
            .expect("fixture has a color map");
        assert_eq!(
            map.resolve(SchemeColor::Background1),
            Some(ColorSchemeSlot::Light1)
        );
        assert_eq!(
            map.resolve(SchemeColor::Text1),
            Some(ColorSchemeSlot::Dark1)
        );
        assert_eq!(
            map.resolve(SchemeColor::Accent1),
            Some(ColorSchemeSlot::Accent1)
        );
    }

    /// Injects a `p:sp` placeholder of `ph_type` with an explicit `solidFill schemeClr {scheme}` into
    /// `part`'s shape tree (the layout/master have empty trees in the fixture).
    fn inject_placeholder_fill(
        pres: &mut Presentation,
        part: &PartName,
        ph_type: &str,
        scheme: &str,
    ) {
        let doc = pres.package.part_tree_mut(part).expect("part tree");
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");

        let ph_attrs = vec![build::attr(interner, "type", ph_type)];
        let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);
        let nv_pr = build::node(
            interner,
            "p",
            PML,
            "nvPr",
            Vec::new(),
            vec![RawNode::Element(ph)],
        );
        let cnvpr_attrs = vec![
            build::attr(interner, "id", "10"),
            build::attr(interner, "name", "Injected"),
        ];
        let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
        let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", Vec::new());
        let nv_sp_pr = build::node(
            interner,
            "p",
            PML,
            "nvSpPr",
            Vec::new(),
            vec![
                RawNode::Element(c_nv_pr),
                RawNode::Element(c_nv_sp_pr),
                RawNode::Element(nv_pr),
            ],
        );

        let clr_attrs = vec![build::attr(interner, "val", scheme)];
        let scheme_clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
        let solid = build::node(
            interner,
            "a",
            DML_MAIN,
            "solidFill",
            Vec::new(),
            vec![RawNode::Element(scheme_clr)],
        );
        let sp_pr = build::node(
            interner,
            "p",
            PML,
            "spPr",
            Vec::new(),
            vec![RawNode::Element(solid)],
        );
        let sp = build::node(
            interner,
            "p",
            PML,
            "sp",
            Vec::new(),
            vec![RawNode::Element(nv_sp_pr), RawNode::Element(sp_pr)],
        );
        sp_tree.children.push(RawNode::Element(sp));
        sp_tree.empty = false;
    }

    #[test]
    fn effective_fill_inherits_from_layout_placeholder() {
        let mut pres = Presentation::open(&fixture()).expect("open");
        let slide0 = pres.slide_part_checked(0).expect("slide").clone();
        let layout = pres
            .follow_rel(&slide0, constants::REL_SLIDE_LAYOUT)
            .expect("rel")
            .expect("layout");

        // The layout's ctrTitle placeholder carries an explicit accent2 fill.
        inject_placeholder_fill(&mut pres, &layout, "ctrTitle", "accent2");

        // Slide 0's ctrTitle placeholder declares no fill of its own, so it inherits the layout's —
        // resolved against the real theme (accent2 = ED7D31).
        assert_eq!(
            pres.effective_shape_fill(0, 0).expect("effective fill"),
            Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("ED7D31".into())))
        );
    }

    /// Injects a `p:sp` placeholder of `ph_type` whose `spPr` holds an `a:ln` with a
    /// `solidFill schemeClr {scheme}` stroke into `part`'s shape tree.
    fn inject_placeholder_outline(
        pres: &mut Presentation,
        part: &PartName,
        ph_type: &str,
        scheme: &str,
    ) {
        let doc = pres.package.part_tree_mut(part).expect("part tree");
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");

        let ph_attrs = vec![build::attr(interner, "type", ph_type)];
        let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);
        let nv_pr = build::node(
            interner,
            "p",
            PML,
            "nvPr",
            Vec::new(),
            vec![RawNode::Element(ph)],
        );
        let cnvpr_attrs = vec![
            build::attr(interner, "id", "11"),
            build::attr(interner, "name", "InjectedLine"),
        ];
        let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
        let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", Vec::new());
        let nv_sp_pr = build::node(
            interner,
            "p",
            PML,
            "nvSpPr",
            Vec::new(),
            vec![
                RawNode::Element(c_nv_pr),
                RawNode::Element(c_nv_sp_pr),
                RawNode::Element(nv_pr),
            ],
        );

        let clr_attrs = vec![build::attr(interner, "val", scheme)];
        let scheme_clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
        let solid = build::node(
            interner,
            "a",
            DML_MAIN,
            "solidFill",
            Vec::new(),
            vec![RawNode::Element(scheme_clr)],
        );
        let ln = build::node(
            interner,
            "a",
            DML_MAIN,
            "ln",
            Vec::new(),
            vec![RawNode::Element(solid)],
        );
        let sp_pr = build::node(
            interner,
            "p",
            PML,
            "spPr",
            Vec::new(),
            vec![RawNode::Element(ln)],
        );
        let sp = build::node(
            interner,
            "p",
            PML,
            "sp",
            Vec::new(),
            vec![RawNode::Element(nv_sp_pr), RawNode::Element(sp_pr)],
        );
        sp_tree.children.push(RawNode::Element(sp));
        sp_tree.empty = false;
    }

    #[test]
    fn effective_outline_inherits_from_layout_placeholder() {
        let mut pres = Presentation::open(&fixture()).expect("open");
        let slide0 = pres.slide_part_checked(0).expect("slide").clone();
        let layout = pres
            .follow_rel(&slide0, constants::REL_SLIDE_LAYOUT)
            .expect("rel")
            .expect("layout");

        // The layout's ctrTitle placeholder carries an explicit accent2 outline.
        inject_placeholder_outline(&mut pres, &layout, "ctrTitle", "accent2");

        // Slide 0's ctrTitle declares no outline of its own, so it inherits the layout's — resolved
        // against the real theme (accent2 = ED7D31).
        let effective = pres
            .effective_shape_outline(0, 0)
            .expect("effective outline")
            .expect("inherited outline");
        assert_eq!(
            effective.fill,
            Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("ED7D31".into())))
        );
    }

    #[test]
    fn effective_outline_resolves_a_line_ref_against_the_theme() {
        let mut pres = Presentation::open(&fixture()).expect("open");
        let idx = pres
            .add_shape(
                0,
                PresetShapeType::Rectangle,
                ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
            )
            .expect("add shape");

        // Give the shape a p:style > a:lnRef into the theme's line-style 2 (w=12700), with accent1 as
        // the phClr substitute.
        {
            let part = pres.slide_part_checked(0).expect("slide").clone();
            let doc = pres.package.part_tree_mut(&part).expect("part tree");
            let RawDocument { interner, root, .. } = doc;
            let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");
            let sp = slide::nth_shape_mut(sp_tree, interner, idx).expect("sp");
            let clr_attrs = vec![build::attr(interner, "val", "accent1")];
            let clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
            let ln_ref_attrs = vec![build::attr(interner, "idx", "2")];
            let ln_ref = build::node(
                interner,
                "a",
                DML_MAIN,
                "lnRef",
                ln_ref_attrs,
                vec![RawNode::Element(clr)],
            );
            let style = build::node(
                interner,
                "p",
                PML,
                "style",
                Vec::new(),
                vec![RawNode::Element(ln_ref)],
            );
            sp.children.push(RawNode::Element(style));
            sp.empty = false;
        }

        // The effective outline is theme line-style 2 (w=12700) with phClr baked to accent1 (4472C4).
        let effective = pres
            .effective_shape_outline(0, idx)
            .expect("effective outline")
            .expect("line-ref outline");
        assert_eq!(effective.width, Some(mjx_dml::LineWidth::from_emu(12700)));
        assert_eq!(
            effective.fill,
            Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("4472C4".into())))
        );
    }
}
