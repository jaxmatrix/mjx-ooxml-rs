//! The [`Presentation`] entry point: open, read shape text, edit a run, save.

use mjx_dml::{Fill, FillSpec, PresetGeometry, ShapeGeometry, TextBody, Theme, ThemeInfo};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces::{DML_MAIN, PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_opc::{Package, PartName, Relationship, TargetMode};

use crate::error::PptxError;
use crate::geometry::ShapeBounds;
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

    /// The number of `p:sp` shapes on slide `slide_idx`.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range or the slide is malformed.
    pub fn shape_count(&mut self, slide_idx: usize) -> Result<usize, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        Ok(slide::shapes(sp_tree, &doc.interner).count())
    }

    /// The full text of shape `shape_idx` on slide `slide_idx` (paragraphs joined by `\n`).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
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
        let shape = nav::nth_child_mut(sp_tree, interner, PML, "sp", shape_idx).ok_or(
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
        let shape = nav::nth_child_mut(sp_tree, interner, PML, "sp", shape_idx).ok_or(
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
    /// relationship must already exist in the package.
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
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = nav::nth_child_mut(sp_tree, interner, PML, "sp", shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                slide: slide_idx,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let element = fill.to_fill(interner).to_xml(interner);
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

    /// The theme that governs slide `slide_idx`, as an interner-free [`ThemeInfo`] (its color scheme +
    /// fill-style matrix) — resolved by following the relationship chain slide → slideLayout →
    /// slideMaster → theme. Returns `Ok(None)` if any hop in the chain is absent (a deck without a
    /// theme). Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range, a relationship points outside the package
    /// ([`ExternalTarget`](PptxError::ExternalTarget)), or the theme part is not well-formed.
    pub fn slide_theme(&mut self, slide_idx: usize) -> Result<Option<ThemeInfo>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let Some(layout) = self.follow_rel(&slide_part, constants::REL_SLIDE_LAYOUT)? else {
            return Ok(None);
        };
        let Some(master) = self.follow_rel(&layout, constants::REL_SLIDE_MASTER)? else {
            return Ok(None);
        };
        let Some(theme_part) = self.follow_rel(&master, constants::REL_THEME)? else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(theme.to_info(&doc.interner)))
    }

    /// Appends a new rectangular text-box shape (`p:sp`) to slide `slide_idx`, laid out at `bounds`
    /// and containing `text` (one paragraph per line, split on `\n`; an empty line becomes an empty
    /// paragraph). Returns the index of the new shape among the slide's `p:sp` shapes. Only that
    /// slide part is marked dirty.
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

        // The new shape is the last `p:sp` child.
        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Appends a new autoshape (`p:sp`) with the given `preset` geometry to slide `slide_idx`, laid
    /// out at `bounds`, with an empty text body. Returns the index of the new shape among the slide's
    /// `p:sp` shapes. Only that slide part is marked dirty.
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
        let slide_target = self.slide_rel_target(&new_part);

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
        let rels = self
            .package
            .relationships_for(Some(&self.presentation_part))
            .ok_or(PptxError::MalformedPresentation(
                "presentation has no relationships",
            ))?;
        let mut max_n = 0u32;
        for rel in rels.iter() {
            if let Some(n) = rel
                .id
                .strip_prefix("rId")
                .and_then(|s| s.parse::<u32>().ok())
            {
                max_n = max_n.max(n);
            }
        }
        Ok(format!("rId{}", max_n + 1))
    }

    /// The relationship target for `new_part` relative to the presentation part's directory (falling
    /// back to the absolute part name if it is not under that directory).
    fn slide_rel_target(&self, new_part: &PartName) -> String {
        let pres_dir = dir_of(self.presentation_part.as_str());
        new_part
            .as_str()
            .strip_prefix(pres_dir)
            .map_or_else(|| new_part.as_str().to_owned(), str::to_owned)
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
