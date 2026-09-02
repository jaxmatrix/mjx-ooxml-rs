//! Shape text: reading and writing runs, paragraphs, fields and their properties, plus the list
//! styles a shape declares for its own outline levels.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{
    CharacterPropertiesSpec, Deck, Error, IndentLevel, ParagraphPropertiesSpec, ShapePath, Surface,
};

impl Deck {
    /// The full text of shape `shape_idx` on `surface` (paragraphs joined by `\n`).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_text`](mjx_pptx::Presentation::shape_text).
    pub fn shape_text(&mut self, surface: Surface, shape_idx: ShapePath) -> Result<String, Error> {
        Ok(self
            .presentation
            .shape_text(surface.to_model(), shape_idx.to_model())?)
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in document
    /// order) of shape `shape_idx` on `surface`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_text`](mjx_pptx::Presentation::set_shape_text).
    pub fn set_shape_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        run_idx: u32,
        text: &str,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(run_idx),
            text,
        )?)
    }

    /// Replaces the **whole text** of shape `shape_idx` on `surface` with `text` — one paragraph per
    /// line, each holding exactly one run, so `shape_text` reads back exactly what was written. Marks
    /// only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_text_content`](mjx_pptx::Presentation::set_shape_text_content).
    pub fn set_shape_text_content(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        text: &str,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_text_content(
            surface.to_model(),
            shape_idx.to_model(),
            text,
        )?)
    }

    /// The number of paragraphs in shape `shape_idx`'s text body. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::paragraph_count`](mjx_pptx::Presentation::paragraph_count).
    pub fn paragraph_count(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.paragraph_count(
            surface.to_model(),
            shape_idx.to_model(),
        )?))
    }

    /// The number of runs in paragraph `para_idx` of shape `shape_idx`. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::run_count`](mjx_pptx::Presentation::run_count).
    pub fn run_count(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.run_count(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?))
    }

    /// The text of paragraph `para_idx` — its runs concatenated. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::paragraph_text`](mjx_pptx::Presentation::paragraph_text).
    pub fn paragraph_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.paragraph_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?)
    }

    /// The text of one run. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::run_text`](mjx_pptx::Presentation::run_text).
    pub fn run_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.run_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// The number of text fields (`a:fld`) in paragraph `para_idx` — generated values such as a slide
    /// number or a date. Fields are a **separate index space** from the runs, so a field never shifts a
    /// run index. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::paragraph_field_count`](mjx_pptx::Presentation::paragraph_field_count).
    pub fn paragraph_field_count(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.paragraph_field_count(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?))
    }

    /// The cached text of field `field_idx` in paragraph `para_idx` — the value the producer last
    /// computed for it (a slide number, a formatted date), not a live value. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::paragraph_field_text`](mjx_pptx::Presentation::paragraph_field_text).
    pub fn paragraph_field_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        field_idx: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.paragraph_field_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(field_idx),
        )?)
    }

    /// What field `field_idx` in paragraph `para_idx` generates (`a:fld@type`, e.g. `slidenum` or
    /// `datetime`), or `None` if it names no type. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::paragraph_field_type`](mjx_pptx::Presentation::paragraph_field_type).
    pub fn paragraph_field_type(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        field_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self.presentation.paragraph_field_type(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(field_idx),
        )?)
    }

    /// The layout properties a paragraph declares of its own (`a:pPr`), or `None` if it declares none —
    /// in which case every property is inherited. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::paragraph_properties`](mjx_pptx::Presentation::paragraph_properties).
    pub fn paragraph_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<Option<ParagraphPropertiesSpec>, Error> {
        Ok(self.presentation.paragraph_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?)
    }

    /// The character properties a run declares of its own (`a:rPr`), or `None` if it declares none.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::run_properties`](mjx_pptx::Presentation::run_properties).
    pub fn run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, Error> {
        Ok(self.presentation.run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::end_run_properties`](mjx_pptx::Presentation::end_run_properties).
    pub fn end_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, Error> {
        Ok(self.presentation.end_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?)
    }

    /// Applies `spec` to one run's character properties, creating its `a:rPr` if it has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_run_properties`](mjx_pptx::Presentation::set_run_properties).
    pub fn set_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
            spec,
        )?)
    }

    /// Applies `spec` to **every run** in paragraph `para_idx`, and to its `a:endParaRPr` if it has one
    /// — so text typed at the end of the paragraph takes the same formatting, which is what selecting a
    /// paragraph and restyling it means.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_paragraph_run_properties`](mjx_pptx::Presentation::set_paragraph_run_properties).
    pub fn set_paragraph_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_paragraph_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            spec,
        )?)
    }

    /// Applies `spec` to **every run of every paragraph** in the shape, and to each paragraph's
    /// `a:endParaRPr` where present — selecting a whole text box and restyling it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_run_properties`](mjx_pptx::Presentation::set_shape_run_properties).
    pub fn set_shape_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            spec,
        )?)
    }

    /// Merges adjacent runs in paragraph `para_idx` that would render identically, returning the number
    /// of runs merged away. This undoes the run splitting that `set_text_range_properties` does:
    /// formatting a sub-range splits a run, and repeatedly formatting overlapping ranges leaves a
    /// paragraph with more runs than it needs.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::coalesce_paragraph_runs`](mjx_pptx::Presentation::coalesce_paragraph_runs).
    pub fn coalesce_paragraph_runs(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.coalesce_paragraph_runs(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
        )?))
    }

    /// Merges adjacent identical runs across **every** paragraph of a shape's text body, returning the
    /// total number of runs merged away. The per-paragraph rule is `coalesce_paragraph_runs`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::coalesce_shape_runs`](mjx_pptx::Presentation::coalesce_shape_runs).
    pub fn coalesce_shape_runs(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.coalesce_shape_runs(
            surface.to_model(),
            shape_idx.to_model(),
        )?))
    }

    /// Applies `spec` to the paragraph-mark properties (`a:endParaRPr`), creating the element if the
    /// paragraph has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_end_run_properties`](mjx_pptx::Presentation::set_end_run_properties).
    pub fn set_end_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_end_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            spec,
        )?)
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if it has
    /// none. The properties **merge**, as run properties do.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_paragraph_properties`](mjx_pptx::Presentation::set_paragraph_properties).
    pub fn set_paragraph_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_paragraph_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            spec,
        )?)
    }

    /// The layout properties the shape's own list style offers at `level` (`a:lstStyle > a:lvlNpPr`),
    /// or `None` if it offers none there — or declares no list style at all. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_list_style_level`](mjx_pptx::Presentation::shape_list_style_level).
    pub fn shape_list_style_level(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        level: IndentLevel,
    ) -> Result<Option<ParagraphPropertiesSpec>, Error> {
        Ok(self.presentation.shape_list_style_level(
            surface.to_model(),
            shape_idx.to_model(),
            level,
        )?)
    }

    /// The properties the shape's own list style offers where no level applies (`a:lstStyle >
    /// a:defPPr`), or `None` if it declares none. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_list_style_default`](mjx_pptx::Presentation::shape_list_style_default).
    pub fn shape_list_style_default(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<ParagraphPropertiesSpec>, Error> {
        Ok(self
            .presentation
            .shape_list_style_default(surface.to_model(), shape_idx.to_model())?)
    }

    /// Applies `spec` to what the shape's own list style offers at `level`, creating the `a:lstStyle` —
    /// and the `a:lvlNpPr` within it — if the shape has none. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_list_style_level`](mjx_pptx::Presentation::set_shape_list_style_level).
    pub fn set_shape_list_style_level(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        level: IndentLevel,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_list_style_level(
            surface.to_model(),
            shape_idx.to_model(),
            level,
            spec,
        )?)
    }

    /// Applies `spec` to what the shape's own list style offers where no level applies (`a:lstStyle >
    /// a:defPPr`), creating the elements if the shape has none. Marks only that part dirty. Merges as
    /// `set_shape_list_style_level` does.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_list_style_default`](mjx_pptx::Presentation::set_shape_list_style_default).
    pub fn set_shape_list_style_default(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_list_style_default(
            surface.to_model(),
            shape_idx.to_model(),
            spec,
        )?)
    }

    /// Removes what the shape's own list style offers at `level`, so the level falls through to the
    /// tier below again. Returns whether it offered anything there; a `false` changes nothing and does
    /// **not** dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_shape_list_style_level`](mjx_pptx::Presentation::clear_shape_list_style_level).
    pub fn clear_shape_list_style_level(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        level: IndentLevel,
    ) -> Result<bool, Error> {
        Ok(self.presentation.clear_shape_list_style_level(
            surface.to_model(),
            shape_idx.to_model(),
            level,
        )?)
    }

    /// Removes the default properties of the shape's own list style (`a:lstStyle > a:defPPr`). Returns
    /// whether it had any; a `false` changes nothing and does **not** dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_shape_list_style_default`](mjx_pptx::Presentation::clear_shape_list_style_default).
    pub fn clear_shape_list_style_default(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<bool, Error> {
        Ok(self
            .presentation
            .clear_shape_list_style_default(surface.to_model(), shape_idx.to_model())?)
    }

    /// Removes the shape's own list style entirely (`a:lstStyle`), so every level falls through to the
    /// tier below. Returns whether the shape had one; a `false` changes nothing and does **not** dirty
    /// the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_shape_list_style`](mjx_pptx::Presentation::clear_shape_list_style).
    pub fn clear_shape_list_style(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<bool, Error> {
        Ok(self
            .presentation
            .clear_shape_list_style(surface.to_model(), shape_idx.to_model())?)
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **Unicode
    /// scalars** across the paragraph's whole text.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_text_range_properties`](mjx_pptx::Presentation::set_text_range_properties).
    pub fn set_text_range_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        range: core::ops::Range<u32>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_text_range_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(range.start)..index(range.end),
            spec,
        )?)
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **grapheme
    /// clusters**: what a reader would call characters, and what a text selection actually spans.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_text_range_properties_by_grapheme`](mjx_pptx::Presentation::set_text_range_properties_by_grapheme).
    pub fn set_text_range_properties_by_grapheme(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        range: core::ops::Range<u32>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_text_range_properties_by_grapheme(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(range.start)..index(range.end),
            spec,
        )?)
    }
}
