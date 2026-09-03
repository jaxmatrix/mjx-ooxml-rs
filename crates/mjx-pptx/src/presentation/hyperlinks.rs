//! Hyperlinks: on a run, on a text range, and on a whole shape — each one a relationship the
//! deck owns, named by `r:id` from an `a:hlinkClick`.

use mjx_dml::TextBody;
use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_opc::{PartName, Relationship, TargetMode};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::hyperlink::Hyperlink;
use crate::surface::Surface;
use crate::{constants, nav, slide};

use super::deck::relationship_prefix;
use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::text::{nth_paragraph, nth_paragraph_mut, nth_run, split_at_offset};
use super::Presentation;

impl Presentation {
    // -----------------------------------------------------------------------------------------
    // Hyperlinks — on a run, on a text range, and on a whole shape
    //
    // A hyperlink names a relationship by `r:id`: a URL is an external relationship, a slide jump an
    // internal one to the target slide. The relationship indirection lives here; the caller works in
    // resolved `Hyperlink`s. Setting a link adds its relationship; clearing (or replacing) removes the
    // one the old link named once nothing else in the part still references it.
    // -----------------------------------------------------------------------------------------

    /// The click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` on `surface`,
    /// resolved to a [`Hyperlink`] (a URL or a slide index), or `None` if the run has no hyperlink — or
    /// one this build does not model (a mouse-over action, a show jump). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a relationship
    /// points outside the package.
    pub fn run_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<Option<Hyperlink>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.run_hyperlink_rel_id(surface, shape_idx, para_idx, run_idx)? else {
            return Ok(None);
        };
        let part = self.surface_part(surface)?;
        self.resolve_hyperlink(&part, &rel_id)
    }

    /// Sets the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` to
    /// `link`, adding its relationship. If the run already linked somewhere, that relationship is
    /// removed once nothing else in the part still names it.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a package edit
    /// fails.
    pub fn set_run_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
        link: &Hyperlink,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let previous = self.run_hyperlink_rel_id(surface, &path, para_idx, run_idx)?;
        let (rel_id, action) = self.add_hyperlink_rel(&part, link)?;
        self.edit_text_body(surface, &path, |body, interner| {
            set_run_hyperlink_in(body, interner, para_idx, run_idx, Some(&rel_id), action)
        })?;
        if let Some(previous) = previous {
            self.remove_hyperlink_rel_if_unreferenced(&part, &previous)?;
        }
        Ok(())
    }

    /// Removes the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx`, and
    /// the relationship it named once nothing else in the part still references it. A no-op if the run
    /// has no hyperlink.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a package edit
    /// fails.
    pub fn clear_run_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let Some(previous) = self.run_hyperlink_rel_id(surface, &path, para_idx, run_idx)? else {
            return Ok(());
        };
        self.edit_text_body(surface, &path, |body, interner| {
            set_run_hyperlink_in(body, interner, para_idx, run_idx, None, None)
        })?;
        self.remove_hyperlink_rel_if_unreferenced(&part, &previous)
    }

    /// Sets the click hyperlink over a **scalar range** of paragraph `para_idx` in shape `shape_idx`,
    /// splitting runs at the boundaries so exactly the selected text is linked (as
    /// [`set_text_range_properties`](Self::set_text_range_properties) does). One relationship is added
    /// and shared by every run in the range. An empty range links nothing.
    ///
    /// Offsets count Unicode scalars; see [`set_text_range_properties`](Self::set_text_range_properties)
    /// on grapheme boundaries.
    ///
    /// # Errors
    /// Returns [`PptxError::TextRangeOutOfBounds`] if the range exceeds the paragraph, or another
    /// [`PptxError`] if an index is out of range or a package edit fails.
    pub fn set_text_range_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        range: core::ops::Range<usize>,
        link: &Hyperlink,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        // Validate the range before adding a relationship, so a bad call leaves the package untouched.
        let length = self
            .paragraph_text(surface, &path, para_idx)?
            .chars()
            .count();
        if range.start > range.end || range.end > length {
            return Err(PptxError::TextRangeOutOfBounds {
                start: range.start,
                end: range.end,
                length,
            });
        }
        if range.start == range.end {
            return Ok(());
        }
        let part = self.surface_part(surface)?;
        let (rel_id, action) = self.add_hyperlink_rel(&part, link)?;
        self.edit_text_body(surface, &path, |body, interner| {
            set_range_hyperlink_in(body, interner, para_idx, range, &rel_id, action)
        })
    }

    /// The click hyperlink on shape `shape_idx` itself (`p:cNvPr > a:hlinkClick`), resolved to a
    /// [`Hyperlink`], or `None` if the shape has no hyperlink (or one this build does not model).
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a relationship
    /// points outside the package.
    pub fn shape_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<Hyperlink>, PptxError> {
        let surface = surface.into();
        let part = self.surface_part(surface)?;
        let rel_id = {
            let doc = self.package.part_tree(&part)?;
            let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
            slide::shape_hyperlink_rel_id(shape, &doc.interner).map(str::to_owned)
        };
        match rel_id {
            Some(rel_id) => self.resolve_hyperlink(&part, &rel_id),
            None => Ok(None),
        }
    }

    /// Sets the click hyperlink on shape `shape_idx` itself to `link`, adding its relationship and
    /// removing the one any previous link named once unreferenced.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a package edit
    /// fails.
    pub fn set_shape_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        link: &Hyperlink,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let previous = self.shape_hyperlink_rel_id(&part, &path)?;
        let (rel_id, action) = self.add_hyperlink_rel(&part, link)?;
        self.edit_shape_hyperlink(surface, &part, &path, Some(&rel_id), action)?;
        if let Some(previous) = previous {
            self.remove_hyperlink_rel_if_unreferenced(&part, &previous)?;
        }
        Ok(())
    }

    /// Removes the click hyperlink on shape `shape_idx` itself, and the relationship it named once
    /// unreferenced. A no-op if the shape has no hyperlink.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a package edit
    /// fails.
    pub fn clear_shape_hyperlink(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let Some(previous) = self.shape_hyperlink_rel_id(&part, &path)? else {
            return Ok(());
        };
        self.edit_shape_hyperlink(surface, &part, &path, None, None)?;
        self.remove_hyperlink_rel_if_unreferenced(&part, &previous)
    }

    /// The relationship id of run `(para_idx, run_idx)`'s click hyperlink, or `None` if it has none.
    fn run_hyperlink_rel_id(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        self.with_text_body(surface, shape_idx, |body, interner| {
            let paragraph = nth_paragraph(body, para_idx)?;
            let run = nth_run(paragraph, run_idx)?;
            Ok(run.hyperlink_rel_id(interner))
        })
    }

    /// The relationship id of shape `shape_idx`'s own click hyperlink, or `None` if it has none.
    pub(super) fn shape_hyperlink_rel_id(
        &mut self,
        part: &PartName,
        path: &ShapePath,
    ) -> Result<Option<String>, PptxError> {
        let doc = self.package.part_tree(part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let shape = slide::resolve_shape(sp_tree, &doc.interner, path).ok();
        Ok(shape.and_then(|shape| {
            slide::shape_hyperlink_rel_id(shape, &doc.interner).map(str::to_owned)
        }))
    }

    /// Writes (or clears) shape `shape_idx`'s own `a:hlinkClick`, resolving the relationships prefix
    /// the way [`set_picture_image`](Self::set_picture_image) does.
    fn edit_shape_hyperlink(
        &mut self,
        surface: Surface,
        part: &PartName,
        path: &ShapePath,
        rel_id: Option<&str>,
        action: Option<&str>,
    ) -> Result<(), PptxError> {
        let doc = self.package.part_tree_mut(part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_prefix = relationship_prefix(root, interner);
        let shape = resolve_shape_in(root, interner, surface, path)?;
        slide::set_shape_hyperlink(shape, interner, rel_prefix, rel_id, action)
    }

    /// Adds the relationship a [`Hyperlink`] needs to `part`, returning its id and the `action`
    /// attribute (if any) to stamp on the `a:hlinkClick`. A URL is an external relationship; a slide
    /// jump an internal one to the target slide part.
    pub(super) fn add_hyperlink_rel(
        &mut self,
        part: &PartName,
        link: &Hyperlink,
    ) -> Result<(String, Option<&'static str>), PptxError> {
        let rel_id = self.next_rid_for(part);
        match link {
            Hyperlink::Url(url) => {
                self.package.add_relationship(
                    Some(part),
                    Relationship {
                        id: rel_id.clone(),
                        rel_type: constants::REL_HYPERLINK.to_owned(),
                        target: url.clone(),
                        mode: TargetMode::External,
                    },
                )?;
                Ok((rel_id, None))
            }
            Hyperlink::Slide(slide_idx) => {
                let target = self.slide_part_checked(*slide_idx)?.clone();
                self.package.add_relationship(
                    Some(part),
                    Relationship {
                        id: rel_id.clone(),
                        rel_type: constants::REL_SLIDE.to_owned(),
                        target: nav::relative_target(part, &target),
                        mode: TargetMode::Internal,
                    },
                )?;
                Ok((rel_id, Some(PPACTION_SLIDE_JUMP)))
            }
        }
    }

    /// Resolves a hyperlink relationship of `part` back to a [`Hyperlink`]: an external `hyperlink`
    /// relationship is its URL, an internal `slide` relationship is that slide's index. `None` for a
    /// relationship of any other kind, or one whose target is not a slide in this deck.
    fn resolve_hyperlink(
        &self,
        part: &PartName,
        rel_id: &str,
    ) -> Result<Option<Hyperlink>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(part)) else {
            return Ok(None);
        };
        let Some(rel) = rels.by_id(rel_id) else {
            return Ok(None);
        };
        if rel.rel_type == constants::REL_HYPERLINK {
            return Ok(Some(Hyperlink::Url(rel.target.clone())));
        }
        if rel.rel_type == constants::REL_SLIDE && rel.mode == TargetMode::Internal {
            let target = nav::resolve_target(part, &rel.target)?;
            return Ok(self
                .slides
                .iter()
                .position(|slide| *slide == target)
                .map(Hyperlink::Slide));
        }
        Ok(None)
    }

    /// Removes hyperlink relationship `rel_id` from `part` unless some `hlink*` element in the part
    /// still names it — a link shared by more than one run survives until its last user is gone.
    pub(super) fn remove_hyperlink_rel_if_unreferenced(
        &mut self,
        part: &PartName,
        rel_id: &str,
    ) -> Result<(), PptxError> {
        let still_used = {
            let doc = self.package.part_tree(part)?;
            let mut ids = Vec::new();
            collect_hyperlink_rel_ids(&doc.root, &doc.interner, &mut ids);
            ids.contains(&rel_id)
        };
        if !still_used {
            self.package.remove_relationship(Some(part), rel_id)?;
        }
        Ok(())
    }
}

/// The `action` a slide-jump `a:hlinkClick` carries (MS-OI29500 — the ECMA schema leaves `action` an
/// opaque string); PowerPoint pairs it with an internal `slide` relationship naming the target slide.
const PPACTION_SLIDE_JUMP: &str = "ppaction://hlinksldjump";

/// Sets (or clears) the click hyperlink on run `(para_idx, run_idx)`.
fn set_run_hyperlink_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    run_idx: usize,
    rel_id: Option<&str>,
    action: Option<&str>,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    let count = paragraph.runs().count();
    let run = paragraph
        .runs_mut()
        .nth(run_idx)
        .ok_or(PptxError::RunIndexOutOfRange {
            index: run_idx,
            count,
        })?;
    run.set_hyperlink(interner, rel_id, action);
    Ok(())
}

/// Splits `para_idx`'s runs at the range's boundaries, then sets the click hyperlink on every run
/// that now falls wholly inside it — the hyperlink counterpart of [`apply_to_scalar_range`].
fn set_range_hyperlink_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    range: core::ops::Range<usize>,
    rel_id: &str,
    action: Option<&str>,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    let length = paragraph.text().chars().count();
    if range.start > range.end || range.end > length {
        return Err(PptxError::TextRangeOutOfBounds {
            start: range.start,
            end: range.end,
            length,
        });
    }
    if range.start == range.end {
        return Ok(());
    }
    split_at_offset(paragraph, range.end);
    split_at_offset(paragraph, range.start);

    let mut consumed = 0;
    let mut targets = Vec::new();
    for (index, run) in paragraph.runs().enumerate() {
        let len = run.text().chars().count();
        if consumed >= range.start && consumed + len <= range.end {
            targets.push(index);
        }
        consumed += len;
    }
    for index in targets {
        if let Some(run) = paragraph.runs_mut().nth(index) {
            run.set_hyperlink(interner, Some(rel_id), action);
        }
    }
    Ok(())
}

/// Collects the relationship ids named by every `a:hlink*` element (click, hover, mouse-over) in the
/// tree — used to decide whether a hyperlink relationship is still in use before removing it.
fn collect_hyperlink_rel_ids<'a>(
    element: &'a RawElement,
    interner: &'a Interner,
    out: &mut Vec<&'a str>,
) {
    let local = interner.resolve(element.name.local);
    if local == "hlinkClick" || local == "hlinkHover" || local == "hlinkMouseOver" {
        if let Some(id) = element
            .attributes
            .iter()
            .find(|attr| interner.resolve(attr.name.local) == "id")
            .and_then(|attr| std::str::from_utf8(&attr.value).ok())
        {
            out.push(id);
        }
    }
    for child in &element.children {
        if let RawNode::Element(child) = child {
            collect_hyperlink_rel_ids(child, interner, out);
        }
    }
}
