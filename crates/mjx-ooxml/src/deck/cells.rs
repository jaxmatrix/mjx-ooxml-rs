//! Table cells: their text, their own formatting, the selection-shaped bulk formatters, and
//! merging.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{
    CellBorder, CellFormat, CellMargins, Cells, CharacterPropertiesSpec, Deck, Error, FillSpec,
    LineSpec, ParagraphPropertiesSpec, ShapePath, Surface, TextAnchoring, TextDirection,
};

impl Deck {
    /// The text of the cell at `(row, column)` — its paragraphs joined by newlines.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_text`](mjx_pptx::Presentation::cell_text).
    pub fn cell_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.cell_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// The text that actually **renders** at `(row, column)` — the text of the cell if it stands alone,
    /// or of the merge **anchor** covering it if it is merged away.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::visible_cell_text`](mjx_pptx::Presentation::visible_cell_text).
    pub fn visible_cell_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.visible_cell_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the cell's paragraphs) of the cell at
    /// `(row, column)`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_text`](mjx_pptx::Presentation::set_cell_text).
    pub fn set_cell_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        run_idx: u32,
        text: &str,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(run_idx),
            text,
        )?)
    }

    /// The number of paragraphs in the cell at `(row, column)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_paragraph_count`](mjx_pptx::Presentation::cell_paragraph_count).
    pub fn cell_paragraph_count(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.cell_paragraph_count(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?))
    }

    /// The number of runs in one paragraph of the cell at `(row, column)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_run_count`](mjx_pptx::Presentation::cell_run_count).
    pub fn cell_run_count(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.cell_run_count(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
        )?))
    }

    /// The text of one paragraph of the cell at `(row, column)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_paragraph_text`](mjx_pptx::Presentation::cell_paragraph_text).
    pub fn cell_paragraph_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.cell_paragraph_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
        )?)
    }

    /// The text of one run of the cell at `(row, column)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_run_text`](mjx_pptx::Presentation::cell_run_text).
    pub fn cell_run_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<String, Error> {
        Ok(self.presentation.cell_run_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// The layout properties a paragraph of the cell at `(row, column)` declares of its own.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_paragraph_properties`](mjx_pptx::Presentation::cell_paragraph_properties).
    pub fn cell_paragraph_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<Option<ParagraphPropertiesSpec>, Error> {
        Ok(self.presentation.cell_paragraph_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
        )?)
    }

    /// The character properties a run of the cell at `(row, column)` declares of its own.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_run_properties`](mjx_pptx::Presentation::cell_run_properties).
    pub fn cell_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, Error> {
        Ok(self.presentation.cell_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// The paragraph-mark properties (`a:endParaRPr`) of a paragraph of the cell at `(row, column)` —
    /// the format an empty cell holds, and what text typed into it would take on.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_end_run_properties`](mjx_pptx::Presentation::cell_end_run_properties).
    pub fn cell_end_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, Error> {
        Ok(self.presentation.cell_end_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
        )?)
    }

    /// Delegates to `Presentation::set_cell_run_properties`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_run_properties`](mjx_pptx::Presentation::set_cell_run_properties).
    #[expect(
        clippy::too_many_arguments,
        reason = "the coordinates the delegated method takes, restated one for one"
    )]
    pub fn set_cell_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            index(run_idx),
            spec,
        )?)
    }

    /// Applies `spec` to **every run** of one paragraph of the cell at `(row, column)`, and to its
    /// paragraph mark.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_paragraph_run_properties`](mjx_pptx::Presentation::set_cell_paragraph_run_properties).
    pub fn set_cell_paragraph_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_paragraph_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            spec,
        )?)
    }

    /// Applies `spec` to **every run of every paragraph** of the cell at `(row, column)` — what
    /// selecting a whole cell and restyling it means, and the usual way to make a header bold.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_run_properties_all`](mjx_pptx::Presentation::set_cell_run_properties_all).
    pub fn set_cell_run_properties_all(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_run_properties_all(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            spec,
        )?)
    }

    /// Applies `spec` to a paragraph mark (`a:endParaRPr`) of the cell at `(row, column)`, creating the
    /// element if the paragraph has none — how an **empty** cell is formatted.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_end_run_properties`](mjx_pptx::Presentation::set_cell_end_run_properties).
    pub fn set_cell_end_run_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_end_run_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            spec,
        )?)
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`) in the cell at `(row, column)`,
    /// creating the element if it has none. The properties **merge**, as run properties do.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_paragraph_properties`](mjx_pptx::Presentation::set_cell_paragraph_properties).
    pub fn set_cell_paragraph_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_paragraph_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            spec,
        )?)
    }

    /// Delegates to `Presentation::set_cell_text_range_properties`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_text_range_properties`](mjx_pptx::Presentation::set_cell_text_range_properties).
    #[expect(
        clippy::too_many_arguments,
        reason = "the coordinates the delegated method takes, restated one for one"
    )]
    pub fn set_cell_text_range_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        para_idx: u32,
        range: core::ops::Range<u32>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_text_range_properties(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            index(para_idx),
            index(range.start)..index(range.end),
            spec,
        )?)
    }

    /// The fill the cell at `(row, column)` declares, or `None` when it declares none — in which case
    /// the table style decides. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_fill`](mjx_pptx::Presentation::cell_fill).
    pub fn cell_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self.presentation.cell_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// Fills the cell at `(row, column)`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_fill`](mjx_pptx::Presentation::set_cell_fill).
    pub fn set_cell_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            fill,
        )?)
    }

    /// Removes the cell's own fill, so the table style decides how it is filled again.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_cell_fill`](mjx_pptx::Presentation::clear_cell_fill).
    pub fn clear_cell_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<(), Error> {
        Ok(self.presentation.clear_cell_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// The border the cell at `(row, column)` declares on `edge`, or `None` if it declares none there.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_border`](mjx_pptx::Presentation::cell_border).
    pub fn cell_border(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, Error> {
        Ok(self.presentation.cell_border(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            edge,
        )?)
    }

    /// Draws a border on one edge of the cell at `(row, column)`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_border`](mjx_pptx::Presentation::set_cell_border).
    pub fn set_cell_border(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        edge: CellBorder,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_border(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            edge,
            line,
        )?)
    }

    /// The ids of the header cells that describe the cell at `(row, column)` (`a:tcPr > a:headers`), in
    /// order — the accessibility association a screen reader announces. Empty when the cell names none.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_headers`](mjx_pptx::Presentation::cell_headers).
    pub fn cell_headers(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<Vec<String>, Error> {
        Ok(self.presentation.cell_headers(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// Sets the header-cell ids that describe the cell at `(row, column)`, replacing whatever it had;
    /// an empty slice removes the association. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_headers`](mjx_pptx::Presentation::set_cell_headers).
    pub fn set_cell_headers(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        header_ids: &[&str],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_headers(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            header_ids,
        )?)
    }

    /// Removes the border on one edge of the cell at `(row, column)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_cell_border`](mjx_pptx::Presentation::clear_cell_border).
    pub fn clear_cell_border(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> Result<(), Error> {
        Ok(self.presentation.clear_cell_border(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            edge,
        )?)
    }

    /// The four insets between the cell's edges and its text, each `None` when the cell does not state
    /// it. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_margins`](mjx_pptx::Presentation::cell_margins).
    pub fn cell_margins(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<CellMargins, Error> {
        Ok(self.presentation.cell_margins(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// Sets the cell's insets. Each field left `None` is **not written**, so a caller can set one
    /// margin without stating the other three.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_margins`](mjx_pptx::Presentation::set_cell_margins).
    pub fn set_cell_margins(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        margins: CellMargins,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_margins(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            margins,
        )?)
    }

    /// Where the text sits vertically in the cell at `(row, column)`, or `None` if unstated (the wire
    /// default is `TextAnchoring::Top`). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_anchor`](mjx_pptx::Presentation::cell_anchor).
    pub fn cell_anchor(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<Option<TextAnchoring>, Error> {
        Ok(self.presentation.cell_anchor(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// Sets where the text sits vertically in the cell at `(row, column)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_anchor`](mjx_pptx::Presentation::set_cell_anchor).
    pub fn set_cell_anchor(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        anchor: TextAnchoring,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_anchor(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            anchor,
        )?)
    }

    /// Which way the text flows in the cell at `(row, column)`, or `None` if unstated (the wire default
    /// is `TextDirection::Horizontal`). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_text_direction`](mjx_pptx::Presentation::cell_text_direction).
    pub fn cell_text_direction(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<Option<TextDirection>, Error> {
        Ok(self.presentation.cell_text_direction(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }

    /// Sets which way the text flows in the cell at `(row, column)` — how a rotated header row is made.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_cell_text_direction`](mjx_pptx::Presentation::set_cell_text_direction).
    pub fn set_cell_text_direction(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
        direction: TextDirection,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_cell_text_direction(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
            direction,
        )?)
    }

    /// Applies `format` to every cell in `cells`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::format_cells`](mjx_pptx::Presentation::format_cells).
    pub fn format_cells(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        cells: Cells,
        format: &CellFormat,
    ) -> Result<(), Error> {
        Ok(self.presentation.format_cells(
            surface.to_model(),
            shape_idx.to_model(),
            cells,
            format,
        )?)
    }

    /// Applies `spec` to **every run of every paragraph** in each cell of `cells`, and to each
    /// paragraph's mark — bolding a header row in one call.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::format_cell_text`](mjx_pptx::Presentation::format_cell_text).
    pub fn format_cell_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        cells: Cells,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.format_cell_text(
            surface.to_model(),
            shape_idx.to_model(),
            cells,
            spec,
        )?)
    }

    /// Applies `spec` to the layout properties of **every paragraph** in each cell of `cells` — right-
    /// aligning a column of numbers in one call.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::format_cell_paragraphs`](mjx_pptx::Presentation::format_cell_paragraphs).
    pub fn format_cell_paragraphs(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        cells: Cells,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.format_cell_paragraphs(
            surface.to_model(),
            shape_idx.to_model(),
            cells,
            spec,
        )?)
    }

    /// Merges `cells` into one region. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::merge_cells`](mjx_pptx::Presentation::merge_cells).
    pub fn merge_cells(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        cells: Cells,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .merge_cells(surface.to_model(), shape_idx.to_model(), cells)?)
    }

    /// Undoes the merge covering the cell at `(row, column)`, whichever cell of the region is named.
    /// Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::unmerge_cells`](mjx_pptx::Presentation::unmerge_cells).
    pub fn unmerge_cells(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<(), Error> {
        Ok(self.presentation.unmerge_cells(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            index(column),
        )?)
    }
}
