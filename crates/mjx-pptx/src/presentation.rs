//! The [`Presentation`] entry point: open, read shape text, edit a run, save.

use mjx_dml::TextBody;
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_opc::{Package, PartName, TargetMode};

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

    fn slide_part_checked(&self, slide_idx: usize) -> Result<&PartName, PptxError> {
        self.slides
            .get(slide_idx)
            .ok_or(PptxError::SlideIndexOutOfRange {
                index: slide_idx,
                count: self.slides.len(),
            })
    }
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
fn build_text_box(interner: &mut Interner, id: u32, text: &str, bounds: ShapeBounds) -> RawElement {
    let id_str = id.to_string();
    let name = format!("TextBox {id}");

    // p:nvSpPr — non-visual shape properties.
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id_str),
        build::attr(interner, "name", &name),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let cnvsppr_attrs = vec![build::attr(interner, "txBox", "1")];
    let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", cnvsppr_attrs);
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
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

    // p:spPr — visual shape properties (transform + preset rectangle geometry).
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
    let prstgeom_attrs = vec![build::attr(interner, "prst", "rect")];
    let prst_geom = build::node(
        interner,
        "a",
        DML_MAIN,
        "prstGeom",
        prstgeom_attrs,
        vec![RawNode::Element(av_lst)],
    );
    let sp_pr = build::node(
        interner,
        "p",
        PML,
        "spPr",
        Vec::new(),
        vec![RawNode::Element(xfrm), RawNode::Element(prst_geom)],
    );

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
