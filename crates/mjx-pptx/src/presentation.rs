//! The [`Presentation`] entry point: open, read shape text, edit a run, save.

use mjx_dml::TextBody;
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_ooxml_types::namespaces::{PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_opc::{Package, PartName, TargetMode};

use crate::error::PptxError;
use crate::{constants, nav, slide};

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

    fn slide_part_checked(&self, slide_idx: usize) -> Result<&PartName, PptxError> {
        self.slides
            .get(slide_idx)
            .ok_or(PptxError::SlideIndexOutOfRange {
                index: slide_idx,
                count: self.slides.len(),
            })
    }
}
