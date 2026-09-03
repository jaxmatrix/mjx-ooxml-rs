//! Shape text: paragraphs, runs, fields, their properties, and the shape's own list style.
//!
//! This module also holds the addressing core the table-cell text surface reuses: a [`TextSite`]
//! names either a shape's `p:txBody` or one table cell's `a:txBody`, and every read and edit below
//! goes through it.

use mjx_dml::{
    resolve_character_properties, CharacterProperties, CharacterPropertiesSpec, ColorMap, FillSpec,
    IndentLevel, ParagraphContent, ParagraphProperties, ParagraphPropertiesSpec, SchemeColors,
    Table, TextBody, TextListStyle,
};
use mjx_ooxml_core::{
    FromXml, Interner, RawAttribute, RawDocument, RawElement, RawNode, Symbol, ToXml,
};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};
use mjx_ooxml_types::presentationml::PlaceholderType;

use crate::address::ShapePath;
use crate::cursor::ShapeEdit;
use crate::error::PptxError;
use crate::slide::ShapeKind;
use crate::surface::Surface;
use crate::{nav, slide};

use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::element_builders::{build_paragraph, build_text_body};
use super::Presentation;

impl Presentation {
    /// The full text of shape `shape_idx` on `surface` (paragraphs joined by `\n`).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body ([`ShapeHasNoTextBody`](PptxError::ShapeHasNoTextBody) — a picture or group never
    /// has one).
    pub fn shape_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<String, PptxError> {
        let surface = surface.into();
        self.with_text_body(surface, shape_idx, |body, _| Ok(body.text()))
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in document
    /// order) of shape `shape_idx` on `surface`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, the shape has no
    /// text body, or the selected run has no `a:t`.
    pub fn set_shape_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        run_idx: usize,
        text: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        self.edit_text_body(surface, shape_idx, |body, _| {
            set_run_text(body, run_idx, text)
        })
    }

    /// Replaces the **whole text** of shape `shape_idx` on `surface` with `text` — one paragraph per
    /// line, each holding exactly one run, so [`shape_text`](Self::shape_text) reads back exactly what
    /// was written. Marks only that part dirty.
    ///
    /// This is the wholesale counterpart of [`set_shape_text`](Self::set_shape_text), which rewrites
    /// one existing run: use this when the new text is the point and whatever was there is not. Any
    /// per-run formatting the old text carried is discarded with it; the body's own layout (`a:bodyPr`
    /// — autofit, insets, anchoring — and `a:lstStyle`) is **kept**, so restating a placeholder's text
    /// does not disturb how that placeholder is laid out. Restyle the new text afterwards with
    /// [`set_shape_run_properties`](Self::set_shape_run_properties).
    ///
    /// A shape with no `p:txBody` is given one, which only a `p:sp` ([`ShapeKind::Shape`]) may have.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape is of a
    /// kind that cannot hold text ([`ShapeHasNoTextBody`](PptxError::ShapeHasNoTextBody) — a picture,
    /// a group, a graphic frame, a connector).
    pub fn set_shape_text_content(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        text: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the paragraphs, `root` holds the body they land in.
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        set_text_content_in(shape, interner, text)
    }

    /// Reads a shape's text body and hands it, with the part's interner, to `read`. Does **not**
    /// dirty the part.
    pub(super) fn with_text_body<R>(
        &mut self,
        surface: Surface,
        shape: impl Into<ShapePath>,
        read: impl FnOnce(&TextBody, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        self.with_text_body_at(surface, TextSite::Shape(shape.into()), read)
    }

    /// Locates a shape's text body, hands it to `edit`, and writes the result back.
    pub(super) fn edit_text_body(
        &mut self,
        surface: Surface,
        shape: impl Into<ShapePath>,
        edit: impl FnOnce(&mut TextBody, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(surface, TextSite::Shape(shape.into()), edit)
    }

    /// Reads the text body at `site` and hands it, with the part's interner, to `read` — for the
    /// accessors that need the interner to resolve what they return. Does **not** dirty the part.
    ///
    /// The interner is borrowed rather than cloned: a part's interner holds every string in it, and
    /// copying that per property read would be absurd.
    pub(super) fn with_text_body_at<R>(
        &mut self,
        surface: Surface,
        site: TextSite,
        read: impl FnOnce(&TextBody, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        let shape = resolve_shape_ref(doc, surface, site.shape_path())?;
        let txbody = locate_text_body(shape, &doc.interner, site)?;
        let body = TextBody::from_xml(txbody, &doc.interner)?;
        read(&body, &doc.interner)
    }

    /// Locates the text body at `site`, hands it to `edit`, and writes the result back — the one
    /// place every text-editing call shares, so the split borrow and the rebuild happen once.
    ///
    /// Marks only that part dirty, and only when `edit` succeeds is the body written back. Only the
    /// addressed `a:txBody` is parsed and rebuilt: reaching a table cell walks the raw tree rather
    /// than parsing the whole table, so editing one cell costs the same as editing a shape.
    pub(super) fn edit_text_body_at(
        &mut self,
        surface: Surface,
        site: TextSite,
        edit: impl FnOnce(&mut TextBody, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        // Split the borrow: `interner` for names and rebuilding, `root` for locate + replace.
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, site.shape_path())?;
        let slot = locate_text_body_mut(shape, interner, site)?;

        let mut body = TextBody::from_xml(slot, interner)?;
        edit(&mut body, interner)?;
        body.write_back(slot, interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Text formatting — the paragraph axis
    //
    // `set_shape_text` above addresses runs *flat* across the whole body, which is the shorthand for
    // the common one-paragraph case. Everything below addresses a paragraph first and a run within
    // it, matching the document tree — and matching what a user selects.
    // -----------------------------------------------------------------------------------------

    /// The number of paragraphs in shape `shape_idx`'s text body. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<usize, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            Ok(paragraph_count_of(body))
        })
    }

    /// The number of runs in paragraph `para_idx` of shape `shape_idx`. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn run_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            run_count_of(body, para_idx)
        })
    }

    /// The text of paragraph `para_idx` — its runs concatenated. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            paragraph_text_of(body, para_idx)
        })
    }

    /// The text of one run. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn run_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            run_text_of(body, para_idx, run_idx)
        })
    }

    /// The number of text fields (`a:fld`) in paragraph `para_idx` — generated values such as a slide
    /// number or a date. Fields are a **separate index space** from the runs, so a field never shifts
    /// a run index. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_field_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            field_count_of(body, para_idx)
        })
    }

    /// The cached text of field `field_idx` in paragraph `para_idx` — the value the producer last
    /// computed for it (a slide number, a formatted date), not a live value. Reading does not dirty
    /// the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_field_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        field_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            field_text_of(body, para_idx, field_idx)
        })
    }

    /// What field `field_idx` in paragraph `para_idx` generates (`a:fld@type`, e.g. `slidenum` or
    /// `datetime`), or `None` if it names no type. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_field_type(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        field_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            field_type_of(body, interner, para_idx, field_idx)
        })
    }

    /// The layout properties a paragraph declares of its own (`a:pPr`), or `None` if it declares
    /// none — in which case every property is inherited. Reading does not dirty the part.
    ///
    /// This is what the paragraph *says*, not what it renders as: a property left unset here is
    /// inherited from the shape's list style, the placeholder, the layout, the master and the theme,
    /// in that order.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            paragraph_properties_of(body, interner, para_idx)
        })
    }

    /// The character properties a run declares of its own (`a:rPr`), or `None` if it declares none.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            run_properties_of(body, interner, para_idx, run_idx)
        })
    }

    /// The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares none.
    ///
    /// This is the format an **empty** paragraph holds — what keeps a blank line its size, and what
    /// text typed into it would take on. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            end_run_properties_of(body, interner, para_idx)
        })
    }

    /// Applies `spec` to one run's character properties, creating its `a:rPr` if it has none.
    ///
    /// The properties **merge**: what the spec names is set, and everything else the run carried —
    /// including the state this model does not describe, like `lang` or `dirty` — is left alone.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        run_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_run_properties_in(body, interner, para_idx, run_idx, spec)
        })
    }

    /// Applies `spec` to **every run** in paragraph `para_idx`, and to its `a:endParaRPr` if it has
    /// one — so text typed at the end of the paragraph takes the same formatting, which is what
    /// selecting a paragraph and restyling it means.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_paragraph_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_paragraph_run_properties_in(body, interner, para_idx, spec)
        })
    }

    /// Applies `spec` to **every run of every paragraph** in the shape, and to each paragraph's
    /// `a:endParaRPr` where present — selecting a whole text box and restyling it.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_shape_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_all_run_properties_in(body, interner, spec)
        })
    }

    /// Merges adjacent runs in paragraph `para_idx` that would render identically, returning the
    /// number of runs merged away. This undoes the run splitting that
    /// [`set_text_range_properties`](Self::set_text_range_properties) does: formatting a sub-range
    /// splits a run, and repeatedly formatting overlapping ranges leaves a paragraph with more runs
    /// than it needs.
    ///
    /// Two adjacent runs merge only when **both** hold, so the paragraph reads exactly the same
    /// afterwards:
    /// - their **effective** formatting is identical — resolved through the full inheritance ladder,
    ///   so a run that sets a property explicitly merges with a neighbour that inherits the same value
    ///   (this compares meaning, not raw XML); and
    /// - neither carries distinguishing state this model does not describe — a hyperlink, an `rtl`, an
    ///   `a:extLst`, a foreign attribute — so nothing is dropped by the merge.
    ///
    /// A line break or field between two runs keeps them apart. When nothing merges, the call changes
    /// nothing and does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn coalesce_paragraph_runs(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();

        let run_count = self.run_count(surface, path.clone(), para_idx)?;
        if run_count < 2 {
            return Ok(0);
        }

        // The effective formatting of each run, resolved through the full ladder.
        let mut effective = Vec::with_capacity(run_count);
        for run_idx in 0..run_count {
            effective.push(self.effective_run_properties(
                surface,
                path.clone(),
                para_idx,
                run_idx,
            )?);
        }

        // Decide, per run, whether it may merge left — content-adjacent, effective-equal, and with
        // matching unmodeled state. This is a read; it does not dirty the part.
        let mergeable = self.with_text_body(surface, &path, |body, interner| {
            let paragraph = nth_paragraph(body, para_idx)?;
            let mut adjacency = vec![false; run_count];
            let mut properties: Vec<Option<&CharacterProperties>> = Vec::with_capacity(run_count);
            let mut run_index = 0;
            let mut previous_was_run = false;
            for item in paragraph.content() {
                if let ParagraphContent::Run(run) = item {
                    adjacency[run_index] = previous_was_run;
                    properties.push(run.properties());
                    run_index += 1;
                    previous_was_run = true;
                } else {
                    previous_was_run = false;
                }
            }
            let mut mergeable = vec![false; run_count];
            for index in 1..run_count {
                mergeable[index] = adjacency[index]
                    && effective[index] == effective[index - 1]
                    && unmodeled_state_eq(properties[index - 1], properties[index], interner);
            }
            Ok(mergeable)
        })?;

        if !mergeable.iter().any(|&flag| flag) {
            return Ok(0); // Nothing to merge — leave the part untouched.
        }

        let mut merged = 0;
        self.edit_text_body(surface, &path, |body, _| {
            merged = nth_paragraph_mut(body, para_idx)?.coalesce_adjacent_runs(&mergeable);
            Ok(())
        })?;
        Ok(merged)
    }

    /// Merges adjacent identical runs across **every** paragraph of a shape's text body, returning the
    /// total number of runs merged away. The per-paragraph rule is
    /// [`coalesce_paragraph_runs`](Self::coalesce_paragraph_runs).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn coalesce_shape_runs(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let paragraph_count = self.paragraph_count(surface, path.clone())?;
        let mut total = 0;
        for para_idx in 0..paragraph_count {
            total += self.coalesce_paragraph_runs(surface, path.clone(), para_idx)?;
        }
        Ok(total)
    }

    /// Applies `spec` to the paragraph-mark properties (`a:endParaRPr`), creating the element if the
    /// paragraph has none.
    ///
    /// This is how an **empty** paragraph is formatted — a placeholder that has been added but not
    /// yet typed into, for instance.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_end_run_properties_in(body, interner, para_idx, spec)
        })
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if it has
    /// none. The properties **merge**, as run properties do.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_paragraph_properties_in(body, interner, para_idx, spec)
        })
    }

    // -----------------------------------------------------------------------------------------
    // The shape's own list style (`a:lstStyle`)
    //
    // A paragraph takes its layout from itself first, and from the shape's list style next — the
    // third tier of the text ladder, and the only one that says "every paragraph at this indent
    // level, in this shape". The setters below are the authoring half of that tier: one statement
    // per level, rather than the same `set_paragraph_properties` call repeated over every paragraph
    // and re-applied to each one added later.
    //
    // These write the shape's *declared* list style. What a paragraph then renders as is still the
    // whole ladder's answer — read it with `effective_paragraph_properties`.
    // -----------------------------------------------------------------------------------------

    /// The layout properties the shape's own list style offers at `level` (`a:lstStyle > a:lvlNpPr`),
    /// or `None` if it offers none there — or declares no list style at all. Reading does not dirty
    /// the part.
    ///
    /// This is what the shape *states*, not what a paragraph at that level renders as: the tiers
    /// below it (the placeholder's list style, the master's text styles, `p:defaultTextStyle`) are
    /// not consulted. For the resolved answer use
    /// [`effective_paragraph_properties`](Self::effective_paragraph_properties).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn shape_list_style_level(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        level: IndentLevel,
    ) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            Ok(body
                .list_style()
                .and_then(|style| style.level(interner, level))
                .map(|properties| properties.spec(interner)))
        })
    }

    /// The properties the shape's own list style offers where no level applies (`a:lstStyle >
    /// a:defPPr`), or `None` if it declares none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`shape_list_style_level`](Self::shape_list_style_level).
    pub fn shape_list_style_default(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            Ok(body
                .list_style()
                .and_then(|style| style.default_properties(interner))
                .map(|properties| properties.spec(interner)))
        })
    }

    /// Applies `spec` to what the shape's own list style offers at `level`, creating the
    /// `a:lstStyle` — and the `a:lvlNpPr` within it — if the shape has none. Marks only that part
    /// dirty.
    ///
    /// This is list formatting **for the whole shape**: every paragraph at `level`, including ones
    /// added later, picks it up without stating anything itself. The properties **merge**, as a
    /// paragraph's own do — a property `spec` leaves unset is left where it was, not cleared — so
    /// naming an indent here cannot flatten the bullet a previous call set.
    ///
    /// `spec`'s [`with_default_run_properties`](ParagraphPropertiesSpec::with_default_run_properties)
    /// carries the level's `a:defRPr`, which is how a level's *character* formatting (its size,
    /// weight, colour) is stated.
    ///
    /// A paragraph that states the same property itself still wins: this is the tier beneath the
    /// paragraph, not above it.
    ///
    /// # Errors
    /// As [`shape_list_style_level`](Self::shape_list_style_level).
    pub fn set_shape_list_style_level(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        level: IndentLevel,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            let mut style = body
                .list_style()
                .cloned()
                .unwrap_or_else(|| TextListStyle::new(interner));
            style.set_level(interner, level, spec);
            body.set_list_style(style);
            Ok(())
        })
    }

    /// Applies `spec` to what the shape's own list style offers where no level applies
    /// (`a:lstStyle > a:defPPr`), creating the elements if the shape has none. Marks only that part
    /// dirty. Merges as [`set_shape_list_style_level`](Self::set_shape_list_style_level) does.
    ///
    /// # Errors
    /// As [`shape_list_style_level`](Self::shape_list_style_level).
    pub fn set_shape_list_style_default(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            let mut style = body
                .list_style()
                .cloned()
                .unwrap_or_else(|| TextListStyle::new(interner));
            style.set_default_properties(interner, spec);
            body.set_list_style(style);
            Ok(())
        })
    }

    /// Removes what the shape's own list style offers at `level`, so the level falls through to the
    /// tier below again. Returns whether it offered anything there; a `false` changes nothing and
    /// does **not** dirty the part.
    ///
    /// The `a:lstStyle` itself is left in place — it may still state other levels. Use
    /// [`clear_shape_list_style`](Self::clear_shape_list_style) to drop the whole element.
    ///
    /// # Errors
    /// As [`shape_list_style_level`](Self::shape_list_style_level).
    pub fn clear_shape_list_style_level(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        level: IndentLevel,
    ) -> Result<bool, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        if self
            .shape_list_style_level(surface, path.clone(), level)?
            .is_none()
        {
            return Ok(false);
        }
        self.edit_text_body(surface, path, |body, interner| {
            if let Some(style) = body.list_style_mut() {
                style.remove_level(interner, level);
            }
            Ok(())
        })?;
        Ok(true)
    }

    /// Removes the default properties of the shape's own list style (`a:lstStyle > a:defPPr`).
    /// Returns whether it had any; a `false` changes nothing and does **not** dirty the part.
    ///
    /// # Errors
    /// As [`shape_list_style_level`](Self::shape_list_style_level).
    pub fn clear_shape_list_style_default(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<bool, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        if self
            .shape_list_style_default(surface, path.clone())?
            .is_none()
        {
            return Ok(false);
        }
        self.edit_text_body(surface, path, |body, interner| {
            if let Some(style) = body.list_style_mut() {
                style.remove_default_properties(interner);
            }
            Ok(())
        })?;
        Ok(true)
    }

    /// Removes the shape's own list style entirely (`a:lstStyle`), so every level falls through to
    /// the tier below. Returns whether the shape had one; a `false` changes nothing and does **not**
    /// dirty the part.
    ///
    /// # Errors
    /// As [`shape_list_style_level`](Self::shape_list_style_level).
    pub fn clear_shape_list_style(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<bool, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let present = self.with_text_body(surface, path.clone(), |body, _| {
            Ok(body.list_style().is_some())
        })?;
        if !present {
            return Ok(false);
        }
        self.edit_text_body(surface, path, |body, _| {
            body.remove_list_style();
            Ok(())
        })?;
        Ok(true)
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **Unicode
    /// scalars** across the paragraph's whole text.
    ///
    /// A run boundary is where formatting changes, so formatting part of a run **splits** it: after
    /// this call the paragraph holds up to two more runs than before, and only those inside `range`
    /// carry `spec`. A range that already lines up with run boundaries splits nothing. Runs are never
    /// merged back together, so the file changes only where it had to.
    ///
    /// For a range taken from a real text selection, prefer
    /// [`set_text_range_properties_by_grapheme`](Self::set_text_range_properties_by_grapheme):
    /// scalar offsets can fall inside a grapheme cluster, splitting an emoji from its modifier.
    ///
    /// # Errors
    /// Returns [`PptxError::TextRangeOutOfBounds`] if the range ends before it starts or runs past
    /// the paragraph's text, or another [`PptxError`] if an index is out of range, the slide is
    /// malformed, or the shape has no text body.
    pub fn set_text_range_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        range: core::ops::Range<usize>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_range_properties_in(body, interner, para_idx, range, spec)
        })
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **grapheme
    /// clusters**: what a reader would call characters, and what a text selection actually spans.
    ///
    /// `👍🏽` is one grapheme (two scalars), so a range that covers it cannot split it in half. The
    /// offsets are converted to scalars and the work is done by
    /// [`set_text_range_properties`](Self::set_text_range_properties).
    ///
    /// # Errors
    /// As [`set_text_range_properties`](Self::set_text_range_properties), with the bounds reported in
    /// graphemes.
    pub fn set_text_range_properties_by_grapheme(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        para_idx: usize,
        range: core::ops::Range<usize>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let text = self.paragraph_text(surface, &path, para_idx)?;
        let scalars = grapheme_range_to_scalars(&text, &range)?;
        self.set_text_range_properties(surface, &path, para_idx, scalars, spec)
    }
}

/// The level a paragraph is read at (`a:pPr@lvl`), or [`IndentLevel::TOP`] when it states none.
///
/// Read **once**, before the walk: it selects which `a:lvlNpPr` every list-style tier contributes, so
/// every tier must be asked about the same level. A paragraph index past the end reads as the top
/// level rather than failing — the caller's own index error surfaces from the tier that needs it.
pub(super) fn paragraph_level(
    body: &TextBody,
    para_idx: usize,
    interner: &Interner,
) -> IndentLevel {
    nth_paragraph(body, para_idx)
        .ok()
        .and_then(|paragraph| paragraph.properties())
        .and_then(|properties| properties.level(interner).ok().flatten())
        .unwrap_or(IndentLevel::TOP)
}

/// One list-style tier, as up to **two** rungs: the properties `list_style` defines at `level`, and
/// beneath them the style's own `a:defPPr`. Both come back as interner-free specs with their colors
/// baked, in priority order.
///
/// The `a:defPPr` rung is what a level the style does not define falls to. ECMA-376 Part 1
/// §21.1.2.2.2 calls it "the paragraph properties that are to be applied when no other paragraph
/// properties have been specified", and §21.1.2.2.6 says of a paragraph that "if no properties are
/// listed then properties specified in the `defPPr` element are used". There is no fallback to
/// `a:lvl1pPr`: §21.1.2.4.13 keys the nine level elements strictly to `a:pPr@lvl`.
///
/// A rung the style does not state yields nothing rather than an empty spec, so the fold above stays
/// honest about which tiers actually spoke.
pub(super) fn list_style_tier(
    list_style: Option<&TextListStyle>,
    level: IndentLevel,
    scheme: &SchemeColors,
    map: &ColorMap,
    interner: &Interner,
) -> [Option<ParagraphPropertiesSpec>; 2] {
    let Some(list_style) = list_style else {
        return [None, None];
    };
    let resolve = |properties: ParagraphProperties| {
        resolved_paragraph_spec(&properties, scheme, map, interner)
    };
    [
        list_style.level(interner, level).map(resolve),
        list_style.default_properties(interner).map(resolve),
    ]
}

/// A tier's paragraph properties as an interner-free spec, with the colors of its `a:defRPr` resolved
/// to concrete RGB (`ParagraphProperties::spec` leaves a scheme color a scheme color).
pub(super) fn resolved_paragraph_spec(
    properties: &ParagraphProperties,
    scheme: &SchemeColors,
    map: &ColorMap,
    interner: &Interner,
) -> ParagraphPropertiesSpec {
    let spec = properties.spec(interner);
    match properties.default_run_properties(interner) {
        Some(default) => spec.with_default_run_properties(resolve_character_properties(
            &default, scheme, map, None, interner,
        )),
        None => spec,
    }
}

/// Which of a master's three text styles governs a placeholder slot: titles are styled by
/// `p:titleStyle`, the date / footer / slide-number chrome by `p:otherStyle`, and everything else —
/// body, subtitle, object, chart, table — by `p:bodyStyle`.
pub(super) fn master_style_local(slot: slide::Placeholder) -> &'static str {
    if slot.is_title_family() {
        return "titleStyle";
    }
    match slot.kind {
        PlaceholderType::DateAndTime
        | PlaceholderType::Footer
        | PlaceholderType::SlideNumber
        | PlaceholderType::Header => "otherStyle",
        _ => "bodyStyle",
    }
}

/// Which of a master's text styles governs a shape that is **not** a placeholder, and so has no slot
/// to be matched on.
///
/// ECMA-376 Part 1 §19.3.1.35 draws the line at the text box: `p:otherStyle` is "used on all text not
/// covered by the `titleStyle` or `bodyStyle` elements", and is "to be used for specifying the text
/// formatting of text within a slide shape but **not** within a text box. Text box styling is handled
/// from within the `bodyStyle` element."
pub(super) fn non_placeholder_style_local(is_text_box: bool) -> &'static str {
    if is_text_box {
        "bodyStyle"
    } else {
        "otherStyle"
    }
}

// ---------------------------------------------------------------------------------------------
// Text-body operations
//
// Each of these is one text operation, named once. A shape's `p:txBody` and a table cell's
// `a:txBody` are the same `CT_TextBody`, so the public surface spells the two apart while every
// operation below has exactly one definition — adding a cell method is delegation, not a second
// implementation, and a new text feature stays a single change.
// ---------------------------------------------------------------------------------------------

/// The number of typed paragraphs in a body.
pub(super) fn paragraph_count_of(body: &TextBody) -> usize {
    body.paragraphs().count()
}

/// The number of typed runs in one paragraph.
pub(super) fn run_count_of(body: &TextBody, para_idx: usize) -> Result<usize, PptxError> {
    Ok(nth_paragraph(body, para_idx)?.runs().count())
}

/// One paragraph's text — its runs concatenated.
pub(super) fn paragraph_text_of(body: &TextBody, para_idx: usize) -> Result<String, PptxError> {
    Ok(nth_paragraph(body, para_idx)?.text())
}

/// One run's text.
pub(super) fn run_text_of(
    body: &TextBody,
    para_idx: usize,
    run_idx: usize,
) -> Result<String, PptxError> {
    let paragraph = nth_paragraph(body, para_idx)?;
    Ok(nth_run(paragraph, run_idx)?.text().to_owned())
}

/// The number of fields (`a:fld`) in one paragraph.
fn field_count_of(body: &TextBody, para_idx: usize) -> Result<usize, PptxError> {
    Ok(nth_paragraph(body, para_idx)?.fields().count())
}

/// One field's cached text (the content of its `a:t`).
fn field_text_of(body: &TextBody, para_idx: usize, field_idx: usize) -> Result<String, PptxError> {
    let paragraph = nth_paragraph(body, para_idx)?;
    Ok(nth_field(paragraph, field_idx)?.text().to_owned())
}

/// What one field generates (`a:fld@type`), or `None` if it names none.
fn field_type_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
    field_idx: usize,
) -> Result<Option<String>, PptxError> {
    let paragraph = nth_paragraph(body, para_idx)?;
    Ok(nth_field(paragraph, field_idx)?
        .field_type(interner)
        .ok()
        .flatten()
        .map(std::borrow::Cow::into_owned))
}

/// The layout properties a paragraph declares of its own.
pub(super) fn paragraph_properties_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
    Ok(nth_paragraph(body, para_idx)?
        .properties()
        .map(|properties| properties.spec(interner)))
}

/// The character properties a run declares of its own.
pub(super) fn run_properties_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
    run_idx: usize,
) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
    let paragraph = nth_paragraph(body, para_idx)?;
    Ok(nth_run(paragraph, run_idx)?
        .properties()
        .map(|properties| properties.spec(interner)))
}

/// The paragraph-mark properties (`a:endParaRPr`) a paragraph declares.
pub(super) fn end_run_properties_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
    Ok(nth_paragraph(body, para_idx)?
        .end_properties()
        .map(|properties| properties.spec(interner)))
}

/// Applies `spec` to one run.
pub(super) fn set_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    run_idx: usize,
    spec: &CharacterPropertiesSpec,
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
    run.set_properties(spec, interner);
    Ok(())
}

/// Applies `spec` to every run of one paragraph, and to its paragraph mark.
pub(super) fn set_paragraph_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    apply_to_paragraph(paragraph, spec, interner);
    Ok(())
}

/// Applies `spec` to every run of every paragraph, and to each paragraph mark.
pub(super) fn set_all_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    for paragraph in body.paragraphs_mut() {
        apply_to_paragraph(paragraph, spec, interner);
    }
    Ok(())
}

/// Applies `spec` to a paragraph's mark (`a:endParaRPr`), creating the element if absent.
pub(super) fn set_end_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    nth_paragraph_mut(body, para_idx)?.set_end_properties(spec, interner);
    Ok(())
}

/// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if absent.
pub(super) fn set_paragraph_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    spec: &ParagraphPropertiesSpec,
) -> Result<(), PptxError> {
    nth_paragraph_mut(body, para_idx)?.set_properties(spec, interner);
    Ok(())
}

/// Applies `spec` to a scalar-offset range within one paragraph, splitting runs at its edges.
pub(super) fn set_range_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    range: core::ops::Range<usize>,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    apply_to_scalar_range(paragraph, range, spec, interner)
}

/// Replaces the text of the `run_idx`-th run of `body`, flattened over its paragraphs in document
/// order — what `set_shape_text` and `set_cell_text` both mean by "set the text".
pub(super) fn set_run_text(
    body: &mut TextBody,
    run_idx: usize,
    text: &str,
) -> Result<(), PptxError> {
    let count = body
        .paragraphs()
        .flat_map(|paragraph| paragraph.runs())
        .count();
    let run = body
        .paragraphs_mut()
        .flat_map(|paragraph| paragraph.runs_mut())
        .nth(run_idx)
        .ok_or(PptxError::RunIndexOutOfRange {
            index: run_idx,
            count,
        })?;
    if !run.set_text(text) {
        return Err(PptxError::RunHasNoText);
    }
    Ok(())
}

/// Which text body an index-addressed text call is about.
///
/// A shape's `p:txBody` and a table cell's `a:txBody` are the *same* `CT_TextBody`, so every text
/// operation applies to either; this is how the private locators say which one. The public surface
/// spells the two apart (`shape_text` / `cell_text`), but the logic below them exists once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TextSite {
    /// The shape's own text body.
    Shape(ShapePath),
    /// A cell of the table the shape frames.
    Cell {
        /// The graphic frame's address in the shape tree.
        shape: ShapePath,
        /// The cell's row.
        row: usize,
        /// The cell's column.
        column: usize,
    },
}

impl TextSite {
    /// The shape this site is inside, whichever kind it is.
    fn shape_path(&self) -> &ShapePath {
        match self {
            Self::Shape(path) | Self::Cell { shape: path, .. } => path,
        }
    }
}

/// The text body `site` names within `shape`.
fn locate_text_body<'a>(
    shape: &'a RawElement,
    interner: &Interner,
    site: TextSite,
) -> Result<&'a RawElement, PptxError> {
    match site {
        TextSite::Shape(_) => {
            slide::shape_txbody(shape, interner).ok_or(PptxError::ShapeHasNoTextBody)
        }
        TextSite::Cell { row, column, .. } => {
            let table = slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            let cell = table_cell(table, interner, row, column)?;
            nav::child(cell, interner, DML_MAIN, "txBody").ok_or(PptxError::ShapeHasNoTextBody)
        }
    }
}

/// The text body `site` names within `shape`, mutably.
fn locate_text_body_mut<'a>(
    shape: &'a mut RawElement,
    interner: &Interner,
    site: TextSite,
) -> Result<&'a mut RawElement, PptxError> {
    match site {
        TextSite::Shape(_) => {
            nav::child_mut(shape, interner, PML, "txBody").ok_or(PptxError::ShapeHasNoTextBody)
        }
        TextSite::Cell { row, column, .. } => {
            // The bounds are checked against an immutable view first, so the error can report the
            // table's real shape before the tree is borrowed mutably.
            let (rows, columns) = {
                let table =
                    slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
                table_dimensions_of(table, interner)
            };
            if row >= rows || column >= columns {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column,
                    rows,
                    columns,
                });
            }
            let table =
                slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            let row_element = slide::nth_row_mut(table, interner, row)
                .ok_or(PptxError::MalformedSlide("table row vanished"))?;
            let cell = slide::nth_cell_mut(row_element, interner, column)
                .ok_or(PptxError::MalformedSlide("table cell vanished"))?;
            nav::child_mut(cell, interner, DML_MAIN, "txBody").ok_or(PptxError::ShapeHasNoTextBody)
        }
    }
}

/// The cell at `(row, column)` of a raw `a:tbl`, or a typed out-of-range error naming the table's
/// real shape.
pub(super) fn table_cell<'a>(
    table: &'a RawElement,
    interner: &Interner,
    row: usize,
    column: usize,
) -> Result<&'a RawElement, PptxError> {
    let (rows, columns) = table_dimensions_of(table, interner);
    let out_of_range = || PptxError::TableCellOutOfRange {
        row,
        column,
        rows,
        columns,
    };
    if row >= rows || column >= columns {
        return Err(out_of_range());
    }
    let row_element = slide::nth_dml_child(table, interner, "tr", row).ok_or_else(out_of_range)?;
    slide::nth_dml_child(row_element, interner, "tc", column).ok_or_else(out_of_range)
}

/// A raw `a:tbl`'s dimensions: its row count, and its column count **as the grid declares it**
/// (`a:tblGrid` is where a table states its width, not any row's cell count).
///
/// A table this model cannot parse reports `(0, 0)` rather than failing — the callers all turn that
/// into an out-of-range error naming the shape, which is the more useful thing to say.
pub(super) fn table_dimensions_of(table: &RawElement, interner: &Interner) -> (usize, usize) {
    Table::from_xml(table, interner)
        .map(|table| (table.row_count(), table.column_count()))
        .unwrap_or_default()
}

/// Resolves `path` to a shape in `doc`, wrapping a miss as the typed error naming `surface`. The one
/// read-side entry point every `shape_*` accessor shares, so the descent and the error wording live
/// in one place.
/// A recorded [`ShapeEdit`] with whatever package work it needed already done — the form the write
/// pass of [`Presentation::apply_shape_edits`] consumes.
///
/// Only the two rel-bearing intents change shape: a hyperlink becomes the relationship id (and
/// action) to stamp, an image becomes the id of the media relationship it was stored as. Everything
/// else is carried through untouched, because everything else is pure element work.
pub(super) enum PreparedEdit {
    /// An edit that needs nothing from the package.
    Element(ShapeEdit),
    /// A shape's own click hyperlink: the relationship to name, or `None` to clear it.
    Hyperlink {
        rel_id: Option<String>,
        action: Option<&'static str>,
    },
    /// A picture's embedded image, as the relationship id that now names it.
    Image(String),
}

impl PreparedEdit {
    /// Whether this edit is applied against the parsed text model — see
    /// [`ShapeEdit::edits_text_model`].
    fn edits_text_model(&self) -> bool {
        matches!(self, Self::Element(edit) if edit.edits_text_model())
    }
}

/// Applies a run of edits that all address `shape`, in order, against one resolution of it.
///
/// A maximal stretch of text-model edits is applied against a single `TextBody` parse and rebuild;
/// everything else is one raw-tree edit each.
pub(super) fn apply_edits_to_shape(
    shape: &mut RawElement,
    interner: &mut Interner,
    edits: &[(ShapePath, PreparedEdit)],
    rel_prefix: Symbol,
    blip_declaration: Option<&RawAttribute>,
) -> Result<(), PptxError> {
    let mut at = 0;
    while at < edits.len() {
        if edits[at].1.edits_text_model() {
            let end = edits[at..]
                .iter()
                .position(|(_, edit)| !edit.edits_text_model())
                .map_or(edits.len(), |offset| at + offset);
            apply_text_model_edits(shape, interner, &edits[at..end])?;
            at = end;
        } else {
            apply_edit_to_element(shape, interner, &edits[at].1, rel_prefix, blip_declaration)?;
            at += 1;
        }
    }
    Ok(())
}

/// Applies one raw-tree edit to an already-resolved shape, by handing it to the same `slide`
/// primitive the corresponding flat setter calls.
fn apply_edit_to_element(
    shape: &mut RawElement,
    interner: &mut Interner,
    edit: &PreparedEdit,
    rel_prefix: Symbol,
    blip_declaration: Option<&RawAttribute>,
) -> Result<(), PptxError> {
    match edit {
        PreparedEdit::Element(ShapeEdit::Fill(fill)) => {
            // Only a picture fill carries an `r:embed`, so only it needs the prefix declaration.
            let declaration = match fill {
                FillSpec::Picture { .. } => blip_declaration.cloned(),
                _ => None,
            };
            slide::set_fill(shape, interner, fill, declaration)
        }
        PreparedEdit::Element(ShapeEdit::Outline(line)) => slide::set_line(shape, interner, line),
        PreparedEdit::Element(ShapeEdit::Effects(effects)) => {
            slide::set_effects(shape, interner, effects)
        }
        PreparedEdit::Element(ShapeEdit::Scene3D(scene)) => {
            slide::set_scene_3d(shape, interner, scene)
        }
        PreparedEdit::Element(ShapeEdit::ClearScene3D) => {
            slide::remove_scene_3d(shape, interner);
            Ok(())
        }
        PreparedEdit::Element(ShapeEdit::Shape3DProperties(properties)) => {
            slide::set_sp3d(shape, interner, properties)
        }
        PreparedEdit::Element(ShapeEdit::ClearShape3DProperties) => {
            slide::remove_sp3d(shape, interner);
            Ok(())
        }
        PreparedEdit::Element(ShapeEdit::Geometry(geometry)) => {
            slide::set_geometry(shape, interner, geometry)
        }
        PreparedEdit::Element(ShapeEdit::Transform(transform)) => {
            let slot = slide::shape_transform_slot_mut(shape, interner)?;
            transform.apply(slot, interner);
            Ok(())
        }
        PreparedEdit::Element(ShapeEdit::Text(text)) => set_text_content_in(shape, interner, text),
        PreparedEdit::Hyperlink { rel_id, action } => {
            slide::set_shape_hyperlink(shape, interner, rel_prefix, rel_id.as_deref(), *action)
        }
        PreparedEdit::Image(rel_id) => slide::set_blip_embed(shape, interner, rel_prefix, rel_id),
        // The remaining variants are applied elsewhere, because they need more than a resolved
        // shape: the text-model edits go through `apply_text_model_edits`, against the parsed body
        // rather than the raw tree, and a bounds edit is converted from slide coordinates against
        // the part root before `write_shape_edits` ever resolves the shape.
        PreparedEdit::Element(_) => Ok(()),
    }
}

/// Applies a run of text-model edits against **one** parse and rebuild of the shape's text body —
/// so formatting a paragraph and then a range within it costs a single round trip.
fn apply_text_model_edits(
    shape: &mut RawElement,
    interner: &mut Interner,
    edits: &[(ShapePath, PreparedEdit)],
) -> Result<(), PptxError> {
    let slot =
        nav::child_mut(shape, interner, PML, "txBody").ok_or(PptxError::ShapeHasNoTextBody)?;
    let mut body = TextBody::from_xml(slot, interner)?;
    for (_, edit) in edits {
        let PreparedEdit::Element(edit) = edit else {
            continue;
        };
        match edit {
            ShapeEdit::RunProperties {
                paragraph,
                run,
                spec,
            } => set_run_properties_in(&mut body, interner, *paragraph, *run, spec)?,
            ShapeEdit::ParagraphRunProperties { paragraph, spec } => {
                set_paragraph_run_properties_in(&mut body, interner, *paragraph, spec)?;
            }
            ShapeEdit::AllRunProperties(spec) => {
                set_all_run_properties_in(&mut body, interner, spec)?;
            }
            ShapeEdit::EndRunProperties { paragraph, spec } => {
                set_end_run_properties_in(&mut body, interner, *paragraph, spec)?;
            }
            ShapeEdit::ParagraphProperties { paragraph, spec } => {
                set_paragraph_properties_in(&mut body, interner, *paragraph, spec)?;
            }
            ShapeEdit::TextRangeProperties {
                paragraph,
                range,
                spec,
                graphemes,
            } => {
                // A grapheme range is converted here, against the body as the earlier edits in this
                // run have left it — the same text the flat method reads before converting.
                let range = if *graphemes {
                    let text = paragraph_text_of(&body, *paragraph)?;
                    grapheme_range_to_scalars(&text, range)?
                } else {
                    range.clone()
                };
                set_range_properties_in(&mut body, interner, *paragraph, range, spec)?;
            }
            // Not a text-model edit; `apply_edits_to_shape` never routes one here.
            _ => {}
        }
    }
    body.write_back(slot, interner);
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Text-body helpers — one place for "find the body, do the thing, put it back"
// ---------------------------------------------------------------------------------------------

/// The `para_idx`-th paragraph of a body, or a typed out-of-range error.
pub(super) fn nth_paragraph(
    body: &TextBody,
    para_idx: usize,
) -> Result<&mjx_dml::Paragraph, PptxError> {
    let count = body.paragraphs().count();
    body.paragraphs()
        .nth(para_idx)
        .ok_or(PptxError::ParagraphIndexOutOfRange {
            index: para_idx,
            count,
        })
}

/// The `run_idx`-th run of a paragraph, or a typed out-of-range error.
pub(super) fn nth_run(
    paragraph: &mjx_dml::Paragraph,
    run_idx: usize,
) -> Result<&mjx_dml::TextRun, PptxError> {
    let count = paragraph.runs().count();
    paragraph
        .runs()
        .nth(run_idx)
        .ok_or(PptxError::RunIndexOutOfRange {
            index: run_idx,
            count,
        })
}

/// The `field_idx`-th field (`a:fld`) of a paragraph, or a typed out-of-range error.
fn nth_field(
    paragraph: &mjx_dml::Paragraph,
    field_idx: usize,
) -> Result<&mjx_dml::TextField, PptxError> {
    let count = paragraph.fields().count();
    paragraph
        .fields()
        .nth(field_idx)
        .ok_or(PptxError::FieldIndexOutOfRange {
            index: field_idx,
            count,
        })
}

/// Whether two runs' character properties carry the same state this model does not describe, treating
/// a run with no `a:rPr` as carrying no such state. Used by run coalescing so a merge never drops a
/// hyperlink, an `rtl`, or any other unmodeled attribute one run had and the other did not.
fn unmodeled_state_eq(
    a: Option<&CharacterProperties>,
    b: Option<&CharacterProperties>,
    interner: &Interner,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(properties), None) | (None, Some(properties)) => {
            properties.has_only_modeled_state(interner)
        }
        (Some(a), Some(b)) => a.unmodeled_state_eq(b, interner),
    }
}

/// The `para_idx`-th paragraph of a body, mutably.
pub(super) fn nth_paragraph_mut(
    body: &mut TextBody,
    para_idx: usize,
) -> Result<&mut mjx_dml::Paragraph, PptxError> {
    let count = body.paragraphs().count();
    body.paragraphs_mut()
        .nth(para_idx)
        .ok_or(PptxError::ParagraphIndexOutOfRange {
            index: para_idx,
            count,
        })
}

/// Applies `spec` to every run of `paragraph` and to its paragraph mark, so text typed at the end
/// takes the same formatting.
fn apply_to_paragraph(
    paragraph: &mut mjx_dml::Paragraph,
    spec: &CharacterPropertiesSpec,
    interner: &mut Interner,
) {
    for run in paragraph.runs_mut() {
        run.set_properties(spec, interner);
    }
    if paragraph.end_properties().is_some() {
        paragraph.set_end_properties(spec, interner);
    }
}

/// Splits `paragraph`'s runs at the range's boundaries, then applies `spec` to every run that now
/// falls wholly inside it.
pub(super) fn apply_to_scalar_range(
    paragraph: &mut mjx_dml::Paragraph,
    range: core::ops::Range<usize>,
    spec: &CharacterPropertiesSpec,
    interner: &mut Interner,
) -> Result<(), PptxError> {
    let length = paragraph.text().chars().count();
    if range.start > range.end || range.end > length {
        return Err(PptxError::TextRangeOutOfBounds {
            start: range.start,
            end: range.end,
            length,
        });
    }
    if range.start == range.end {
        return Ok(()); // An empty selection formats nothing.
    }

    // Split at the far boundary first: splitting at the near one would shift everything after it,
    // while the far offset is expressed in the *original* coordinates.
    split_at_offset(paragraph, range.end);
    split_at_offset(paragraph, range.start);

    // After the splits every run lies wholly inside or wholly outside the range, so a running count
    // of scalars is enough to tell which.
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
            run.set_properties(spec, interner);
        }
    }
    Ok(())
}

/// Splits whichever run contains the paragraph-level scalar `offset`, unless it already falls on a
/// run boundary — where there is nothing to split.
pub(super) fn split_at_offset(paragraph: &mut mjx_dml::Paragraph, offset: usize) {
    let mut consumed = 0;
    let mut target = None;
    for (index, run) in paragraph.runs().enumerate() {
        let len = run.text().chars().count();
        if offset > consumed && offset < consumed + len {
            target = Some((index, offset - consumed));
            break;
        }
        consumed += len;
    }
    if let Some((index, within)) = target {
        paragraph.split_run_at(index, within);
    }
}

/// Converts a grapheme-cluster range into the scalar range covering the same text.
fn grapheme_range_to_scalars(
    text: &str,
    range: &core::ops::Range<usize>,
) -> Result<core::ops::Range<usize>, PptxError> {
    use unicode_segmentation::UnicodeSegmentation;

    let clusters: Vec<&str> = text.graphemes(true).collect();
    if range.start > range.end || range.end > clusters.len() {
        return Err(PptxError::TextRangeOutOfBounds {
            start: range.start,
            end: range.end,
            length: clusters.len(),
        });
    }
    let scalars_before = |count: usize| -> usize {
        clusters[..count]
            .iter()
            .map(|cluster| cluster.chars().count())
            .sum()
    };
    Ok(scalars_before(range.start)..scalars_before(range.end))
}

/// Replaces the text of `shape` with one `a:p` per line of `text`, each holding exactly one run.
///
/// The body's own `a:bodyPr` and `a:lstStyle` survive — only the paragraphs are swapped — so
/// restating a shape's text leaves how that text is laid out alone. A shape with no `p:txBody` is
/// given a whole new one, which only a `p:sp` may have.
///
/// This is the one implementation of the edit: [`Presentation::set_shape_text_content`] and the shape
/// cursor's `text` both land here.
fn set_text_content_in(
    shape: &mut RawElement,
    interner: &mut Interner,
    text: &str,
) -> Result<(), PptxError> {
    // Built before the body is located, so the `&mut Interner` is free of the tree borrow.
    let paragraphs: Vec<RawElement> = text
        .split('\n')
        .map(|line| build_paragraph(interner, line))
        .collect();

    if nav::child(shape, interner, PML, "txBody").is_some() {
        let body =
            nav::child_mut(shape, interner, PML, "txBody").ok_or(PptxError::ShapeHasNoTextBody)?;
        replace_paragraphs(body, interner, paragraphs);
        return Ok(());
    }
    if slide::shape_kind(shape, interner) != Some(ShapeKind::Shape) {
        return Err(PptxError::ShapeHasNoTextBody);
    }
    let body = build_text_body(interner, paragraphs);
    replace_txbody(shape, interner, body);
    Ok(())
}

/// Swaps every `a:p` of a text body for `paragraphs`, keeping every other child — `a:bodyPr`,
/// `a:lstStyle` — exactly where and as it was. Paragraphs are last in `CT_TextBody`, so appending
/// them after the survivors restores the schema's content order.
fn replace_paragraphs(body: &mut RawElement, interner: &Interner, paragraphs: Vec<RawElement>) {
    body.children.retain(|node| {
        !matches!(node, RawNode::Element(element)
            if nav::name_is(&element.name, interner, DML_MAIN, "p"))
    });
    body.children
        .extend(paragraphs.into_iter().map(RawNode::Element));
    body.empty = body.children.is_empty();
}

/// Replaces `shape`'s `p:txBody` with `new_body`, preserving its position among the shape's children;
/// appends it if the shape had none. Used to overwrite a notes body placeholder's text wholesale.
pub(super) fn replace_txbody(shape: &mut RawElement, interner: &Interner, new_body: RawElement) {
    let existing = shape.children.iter().position(|child| {
        matches!(child, RawNode::Element(element) if nav::name_is(&element.name, interner, PML, "txBody"))
    });
    match existing {
        Some(index) => shape.children[index] = RawNode::Element(new_body),
        None => {
            shape.children.push(RawNode::Element(new_body));
            shape.empty = false;
        }
    }
}
