//! Table cells: their text, their own properties (fill, borders, insets, anchoring), formatting
//! a whole selection at once, and merging.

use mjx_dml::{
    Cell3D, CellBorder, CharacterPropertiesSpec, FillSpec, LineSpec, ParagraphPropertiesSpec,
    Table, TableCell, TableCellProperties, TextAnchoring, TextBody, TextDirection,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::namespaces::DML_MAIN;

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::geometry::CellMargins;
use crate::surface::Surface;
use crate::table::{CellFormat, Cells};
use crate::{build, nav, slide};

use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::text::{
    end_run_properties_of, paragraph_count_of, paragraph_properties_of, paragraph_text_of,
    run_count_of, run_properties_of, run_text_of, set_all_run_properties_in,
    set_end_run_properties_in, set_paragraph_properties_in, set_paragraph_run_properties_in,
    set_range_properties_in, set_run_properties_in, set_run_text, table_cell, table_dimensions_of,
    TextSite,
};
use super::Presentation;

impl Presentation {
    // -----------------------------------------------------------------------------------------
    // Text in a table cell
    //
    // A cell's `a:txBody` is the same `CT_TextBody` as a shape's `p:txBody`, so every one of these
    // is the corresponding shape method addressed at a cell instead — same operation, same errors,
    // same guarantees. The pair `(row, column)` addresses the cell; everything after it means what
    // it means on a shape.
    //
    // A cell covered by a merge still holds its own text body, and these reach it. Ask
    // `merged_cell_anchor` which cell actually renders at a position before reading text from one.
    // -----------------------------------------------------------------------------------------

    /// The text of the cell at `(row, column)` — its paragraphs joined by newlines.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotATable`] if the shape frames no table,
    /// [`PptxError::TableCellOutOfRange`] if there is no such cell, or another [`PptxError`] if an
    /// index is out of range, the part is malformed, or the cell has no text body.
    pub fn cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            Ok(body.text())
        })
    }

    /// The text that actually **renders** at `(row, column)` — the text of the cell if it stands
    /// alone, or of the merge **anchor** covering it if it is merged away.
    ///
    /// [`cell_text`](Self::cell_text) returns a covered cell's own (hidden) text, which is what an
    /// unmerge restores; this follows the merge to what a reader sees. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text), plus [`PptxError::TableCellOutOfRange`].
    pub fn visible_cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<String, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let (anchor_row, anchor_column) = self.merged_cell_anchor(surface, &path, row, column)?;
        self.cell_text(surface, &path, anchor_row, anchor_column)
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the cell's paragraphs) of the cell
    /// at `(row, column)`. Marks only that part dirty.
    ///
    /// A cell created by [`add_table`](Self::add_table) has one empty run, so `run_idx` is `0` for
    /// the common case of filling in a fresh table.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text), plus [`PptxError::RunHasNoText`] if the selected run has
    /// no `a:t`.
    pub fn set_cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        run_idx: usize,
        text: &str,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            set_run_text(body, run_idx, text)
        })
    }

    /// The number of paragraphs in the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_paragraph_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            Ok(paragraph_count_of(body))
        })
    }

    /// The number of runs in one paragraph of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_run_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            run_count_of(body, para_idx)
        })
    }

    /// The text of one paragraph of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_paragraph_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            paragraph_text_of(body, para_idx)
        })
    }

    /// The text of one run of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_run_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            run_text_of(body, para_idx, run_idx)
        })
    }

    /// The layout properties a paragraph of the cell at `(row, column)` declares of its own.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
        self.with_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| paragraph_properties_of(body, interner, para_idx),
        )
    }

    /// The character properties a run of the cell at `(row, column)` declares of its own.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| run_properties_of(body, interner, para_idx, run_idx),
        )
    }

    /// The paragraph-mark properties (`a:endParaRPr`) of a paragraph of the cell at `(row, column)`
    /// — the format an empty cell holds, and what text typed into it would take on.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| end_run_properties_of(body, interner, para_idx),
        )
    }

    /// Applies `spec` to one run of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    // Eight parameters, and every one a distinct coordinate: a surface, the frame, the cell's row
    // and column, a paragraph, a run, and the spec. `Cells` and `CellFormat` do not shorten this —
    // `CellFormat` carries how a cell *draws*, not its character properties, and `Cells` names a
    // *selection*, so using it here would let `Cells::All` be written where exactly one cell is
    // required, trading a compile-time guarantee for a runtime error. The bulk intention has its own
    // method already: `format_cell_text` applies one spec to every run of a whole selection.
    // `expect` rather than `allow`, so the day the list does fit, this attribute fails the build.
    #[expect(
        clippy::too_many_arguments,
        reason = "six independent cell coordinates plus the spec"
    )]
    pub fn set_cell_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_run_properties_in(body, interner, para_idx, run_idx, spec),
        )
    }

    /// Applies `spec` to **every run** of one paragraph of the cell at `(row, column)`, and to its
    /// paragraph mark.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_paragraph_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_paragraph_run_properties_in(body, interner, para_idx, spec),
        )
    }

    /// Applies `spec` to **every run of every paragraph** of the cell at `(row, column)` — what
    /// selecting a whole cell and restyling it means, and the usual way to make a header bold.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_run_properties_all(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_all_run_properties_in(body, interner, spec),
        )
    }

    /// Applies `spec` to a paragraph mark (`a:endParaRPr`) of the cell at `(row, column)`, creating
    /// the element if the paragraph has none — how an **empty** cell is formatted.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_end_run_properties_in(body, interner, para_idx, spec),
        )
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`) in the cell at `(row, column)`,
    /// creating the element if it has none. The properties **merge**, as run properties do.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_paragraph_properties_in(body, interner, para_idx, spec),
        )
    }

    /// Applies `spec` to part of a paragraph of the cell at `(row, column)` — the characters in
    /// `range`, counted in **Unicode scalars**. Splits runs at the range's edges, exactly as the
    /// shape-addressed form does.
    ///
    /// # Errors
    /// As [`set_text_range_properties`](Self::set_text_range_properties), plus the table errors of
    /// [`cell_text`](Self::cell_text).
    // Eight parameters, and every one a distinct coordinate: a surface, the frame, the cell's row
    // and column, a paragraph, a run, and the spec. `Cells` and `CellFormat` do not shorten this —
    // `CellFormat` carries how a cell *draws*, not its character properties, and `Cells` names a
    // *selection*, so using it here would let `Cells::All` be written where exactly one cell is
    // required, trading a compile-time guarantee for a runtime error. The bulk intention has its own
    // method already: `format_cell_text` applies one spec to every run of a whole selection.
    // `expect` rather than `allow`, so the day the list does fit, this attribute fails the build.
    #[expect(
        clippy::too_many_arguments,
        reason = "six independent cell coordinates plus the spec"
    )]
    pub fn set_cell_text_range_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        para_idx: usize,
        range: core::ops::Range<usize>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_range_properties_in(body, interner, para_idx, range, spec),
        )
    }

    // -----------------------------------------------------------------------------------------
    // Cell formatting — what actually draws
    //
    // A cell's fill and its six borders are the same `EG_FillProperties` and `CT_LineProperties`
    // a shape uses, so `FillSpec` and `LineSpec` carry them unchanged; only the element's tag
    // differs, which is why one `LineSpec` serves all six edges.
    //
    // Everything here writes into `a:tcPr`, creating it when the cell has none. An unstated value
    // reads as `None` rather than as the schema default, because the two are different facts: the
    // margins default to 0.1"/0.05", not to zero.
    // -----------------------------------------------------------------------------------------

    /// The fill the cell at `(row, column)` declares, or `None` when it declares none — in which
    /// case the table style decides. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties
                    .and_then(|properties| properties.fill(interner))
                    .map(|fill| fill.spec(interner)))
            },
        )
    }

    /// Fills the cell at `(row, column)`. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_fill(interner, Some(fill));
                Ok(())
            },
        )
    }

    /// Removes the cell's own fill, so the table style decides how it is filled again.
    ///
    /// This is **not** the same as filling it with [`FillSpec::None`], which states *no fill at all*
    /// and blocks the style.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn clear_cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_fill(interner, None);
                Ok(())
            },
        )
    }

    /// The border the cell at `(row, column)` declares on `edge`, or `None` if it declares none
    /// there. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties
                    .and_then(|properties| properties.border(interner, edge))
                    .map(|line| line.spec(interner)))
            },
        )
    }

    /// Draws a border on one edge of the cell at `(row, column)`. Marks only that part dirty.
    ///
    /// The five other edges are untouched: each is its own element, and this writes one of them.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        edge: CellBorder,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_border(interner, edge, Some(line));
                Ok(())
            },
        )
    }

    /// The ids of the header cells that describe the cell at `(row, column)` (`a:tcPr > a:headers`),
    /// in order — the accessibility association a screen reader announces. Empty when the cell names
    /// none. Reading does not dirty the part.
    ///
    /// Each id is another cell's `@id`; a table that uses headers gives its header cells ids and
    /// points each data cell at the ones above and beside it.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_headers(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<Vec<String>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties.map_or_else(Vec::new, |properties| properties.headers(interner)))
            },
        )
    }

    /// Sets the header-cell ids that describe the cell at `(row, column)`, replacing whatever it had;
    /// an empty slice removes the association. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_headers(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        header_ids: &[&str],
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_headers(interner, header_ids);
                Ok(())
            },
        )
    }

    /// Removes the border on one edge of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn clear_cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        edge: CellBorder,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_border(interner, edge, None);
                Ok(())
            },
        )
    }

    /// The four insets between the cell's edges and its text, each `None` when the cell does not
    /// state it. Reading does not dirty the part.
    ///
    /// An unstated margin is **not** a zero one — the schema defaults are `0.1"` horizontally and
    /// `0.05"` vertically, exposed as
    /// [`TableCellProperties::DEFAULT_MARGIN_HORIZONTAL`](mjx_dml::TableCellProperties::DEFAULT_MARGIN_HORIZONTAL)
    /// and its vertical counterpart.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_margins(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<CellMargins, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                let Some(properties) = properties else {
                    return Ok(CellMargins::default());
                };
                // A margin the cell states unreadably is a margin the cell does not state: the
                // renderer substitutes the schema default either way.
                Ok(CellMargins {
                    left: properties.left_margin(interner).ok().flatten(),
                    right: properties.right_margin(interner).ok().flatten(),
                    top: properties.top_margin(interner).ok().flatten(),
                    bottom: properties.bottom_margin(interner).ok().flatten(),
                })
            },
        )
    }

    /// Sets the cell's insets. Each field left `None` is **not written**, so a caller can set one
    /// margin without stating the other three.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_margins(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        margins: CellMargins,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_margins(
                    interner,
                    margins.left,
                    margins.right,
                    margins.top,
                    margins.bottom,
                );
                Ok(())
            },
        )
    }

    /// Where the text sits vertically in the cell at `(row, column)`, or `None` if unstated (the
    /// wire default is [`TextAnchoring::Top`]). Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_anchor(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<Option<TextAnchoring>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties.and_then(|properties| properties.anchor(interner).ok().flatten()))
            },
        )
    }

    /// Sets where the text sits vertically in the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_anchor(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        anchor: TextAnchoring,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_anchor(interner, Some(anchor));
                Ok(())
            },
        )
    }

    /// Which way the text flows in the cell at `(row, column)`, or `None` if unstated (the wire
    /// default is [`TextDirection::Horizontal`]). Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_text_direction(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<Option<TextDirection>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties
                    .and_then(|properties| properties.text_direction(interner).ok().flatten()))
            },
        )
    }

    /// Sets which way the text flows in the cell at `(row, column)` — how a rotated header row is
    /// made.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_text_direction(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        direction: TextDirection,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_text_direction(interner, Some(direction));
                Ok(())
            },
        )
    }

    /// Reads the `a:tcPr` of the cell at `(row, column)` — `None` when the cell declares none — and
    /// hands it, with the part's interner, to `read`. Does **not** dirty the part.
    fn with_cell_properties<R>(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        read: impl FnOnce(Option<&TableCellProperties>, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        let table = slide::shape_table(shape, &doc.interner).ok_or(PptxError::ShapeIsNotATable)?;
        let cell = table_cell(table, &doc.interner, row, column)?;
        let properties = match nav::child(cell, &doc.interner, DML_MAIN, "tcPr") {
            Some(element) => Some(TableCellProperties::from_xml(element, &doc.interner)?),
            None => None,
        };
        read(properties.as_ref(), &doc.interner)
    }

    /// Hands the `a:tcPr` of the cell at `(row, column)` to `edit` and writes it back, **creating
    /// the element when the cell has none** — inserted after the cell's `a:txBody`, per
    /// `CT_TableCell`'s sequence.
    ///
    /// Only the `a:tcPr` is parsed and rebuilt; the table around it is untouched.
    fn edit_cell_properties(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
        edit: impl FnOnce(&mut TableCellProperties, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;

        // Bounds first, against an immutable view, so the error can name the table's real shape.
        let (rows, columns) = {
            let table = slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
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

        let table = slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
        let row_element = slide::nth_row_mut(table, interner, row)
            .ok_or(PptxError::MalformedSlide("table row vanished"))?;
        let cell = slide::nth_cell_mut(row_element, interner, column)
            .ok_or(PptxError::MalformedSlide("table cell vanished"))?;

        let slot = cell_properties_slot(cell, interner)?;
        let mut properties = TableCellProperties::from_xml(slot, interner)?;
        edit(&mut properties, interner)?;
        properties.write_back(slot, interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Formatting many cells at once
    //
    // The per-property setters above each say one thing, which is right when a caller means one
    // thing. A navy header row with a rule under it is *one* intention, and saying it nine times in
    // a loop reads like nine. These take a `Cells` selection and a spec, in the shape the crate
    // already uses everywhere else.
    // -----------------------------------------------------------------------------------------

    /// Applies `format` to every cell in `cells`. Marks only that part dirty.
    ///
    /// **Only the properties `format` names are written**, so a fill can be applied across a region
    /// whose cells carry different borders without flattening them. A format that names nothing
    /// changes nothing, and creates no `a:tcPr` for a cell that had none.
    ///
    /// The table is located once and the selection walked within it, so formatting a whole table
    /// costs one traversal rather than one per cell.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotATable`] if the shape frames no table,
    /// [`PptxError::TableCellOutOfRange`] if the selection reaches outside it, or another
    /// [`PptxError`] if an index is out of range or the part is malformed.
    pub fn format_cells(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        cells: Cells,
        format: &CellFormat,
    ) -> Result<(), PptxError> {
        if format.is_empty() {
            return Ok(());
        }
        self.edit_selected_cells(
            surface.into(),
            shape_idx,
            &cells,
            true,
            |cell, interner, _, _| {
                let slot = cell_properties_slot(cell, interner)?;
                let mut properties = TableCellProperties::from_xml(slot, interner)?;
                apply_cell_format(&mut properties, interner, format);
                properties.write_back(slot, interner);
                Ok(())
            },
        )
    }

    /// Applies `spec` to **every run of every paragraph** in each cell of `cells`, and to each
    /// paragraph's mark — bolding a header row in one call.
    ///
    /// This is the cell-selection form of
    /// [`set_cell_run_properties_all`](Self::set_cell_run_properties_all).
    ///
    /// # Errors
    /// As [`format_cells`](Self::format_cells), plus a malformed text body.
    pub fn format_cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        cells: Cells,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_selected_cells(
            surface.into(),
            shape_idx,
            &cells,
            true,
            |cell, interner, _, _| {
                let Some(slot) = nav::child_mut(cell, interner, DML_MAIN, "txBody") else {
                    return Ok(()); // A cell with no text body has no runs to format.
                };
                let mut body = TextBody::from_xml(slot, interner)?;
                set_all_run_properties_in(&mut body, interner, spec)?;
                body.write_back(slot, interner);
                Ok(())
            },
        )
    }

    /// Applies `spec` to the layout properties of **every paragraph** in each cell of `cells` —
    /// right-aligning a column of numbers in one call.
    ///
    /// # Errors
    /// As [`format_cell_text`](Self::format_cell_text).
    pub fn format_cell_paragraphs(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        cells: Cells,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_selected_cells(
            surface.into(),
            shape_idx,
            &cells,
            true,
            |cell, interner, _, _| {
                let Some(slot) = nav::child_mut(cell, interner, DML_MAIN, "txBody") else {
                    return Ok(());
                };
                let mut body = TextBody::from_xml(slot, interner)?;
                let count = body.paragraphs().count();
                for index in 0..count {
                    set_paragraph_properties_in(&mut body, interner, index, spec)?;
                }
                body.write_back(slot, interner);
                Ok(())
            },
        )
    }

    /// Locates the table once, resolves `cells` against its real dimensions, and hands each selected
    /// `a:tc` to `edit` in row-major order.
    ///
    /// When `visible_only`, a cell covered by a merge (which renders nothing) is skipped — so
    /// formatting a selection touches only the anchors that actually show, and unmerging restores a
    /// covered cell's own formatting. Merging and unmerging pass `false`: they must reach covered
    /// cells to set and clear the merge flags.
    fn edit_selected_cells(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        cells: &Cells,
        visible_only: bool,
        edit: impl Fn(&mut RawElement, &mut Interner, usize, usize) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;

        let (rows, columns) = {
            let table = slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            table_dimensions_of(table, interner)
        };
        let positions = cells.resolve(rows, columns).map_err(|(row, column)| {
            PptxError::TableCellOutOfRange {
                row,
                column,
                rows,
                columns,
            }
        })?;

        let table = slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
        for (row, column) in positions {
            let row_element = slide::nth_row_mut(table, interner, row)
                .ok_or(PptxError::MalformedSlide("table row vanished"))?;
            let cell = slide::nth_cell_mut(row_element, interner, column)
                .ok_or(PptxError::MalformedSlide("table cell vanished"))?;
            if visible_only && raw_cell_is_covered(cell, interner) {
                continue;
            }
            edit(cell, interner, row, column)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Merging
    //
    // A merged region is anchored at its top-left cell, which states how far it reaches; the cells
    // it covers stay in the table, each stating that something to its left or above owns it. So the
    // grid never loses a cell, `(row, column)` addressing keeps working, and unmerging is simply
    // taking four attributes back off.
    // -----------------------------------------------------------------------------------------

    /// Merges `cells` into one region. Marks only that part dirty.
    ///
    /// The top-left cell becomes the anchor and is what renders; every other cell in the region is
    /// marked as covered. **No cell is removed and no text is touched** — a covered cell keeps its
    /// own text body, invisible until the region is unmerged again, so merging loses nothing.
    ///
    /// A merged region already **inside** the selection is absorbed into the new one. A selection of
    /// a single cell, or an empty one, changes nothing.
    ///
    /// # Errors
    /// Returns [`PptxError::TableMergeCrossesSelection`] if a cell in the selection belongs to a
    /// merged region reaching outside it — unmerge that region first — plus the errors of
    /// [`format_cells`](Self::format_cells).
    pub fn merge_cells(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        cells: Cells,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();

        // Read first: the region to merge, and whether any existing merge would be cut in half.
        let region = self.with_table(surface, &path, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let (row_range, column_range) =
                cells.bounds(rows, columns).map_err(|(row, column)| {
                    PptxError::TableCellOutOfRange {
                        row,
                        column,
                        rows,
                        columns,
                    }
                })?;
            if row_range.is_empty() || column_range.is_empty() {
                return Ok(None);
            }
            check_merges_fit(table, interner, &row_range, &column_range)?;
            Ok(Some((row_range, column_range)))
        })?;

        let Some((row_range, column_range)) = region else {
            return Ok(());
        };
        let (first_row, first_column) = (row_range.start, column_range.start);
        let (height, width) = (row_range.len(), column_range.len());
        let selection = Cells::rectangle(row_range, column_range);

        self.edit_selected_cells(
            surface,
            &path,
            &selection,
            false, // merging must reach the cells it covers, to mark them merged
            |cell, interner, row, column| {
                let mut typed = TableCell::from_xml(cell, interner)?;
                if row == first_row && column == first_column {
                    typed.set_spans(interner, width, height);
                    typed.set_merged(interner, false, false);
                } else {
                    // Covered: it says what owns it, not which cell that is — left, above, or both.
                    typed.set_spans(interner, 1, 1);
                    typed.set_merged(interner, column > first_column, row > first_row);
                }
                typed.write_back(cell, interner);
                Ok(())
            },
        )
    }

    /// Undoes the merge covering the cell at `(row, column)`, whichever cell of the region is named.
    /// Marks only that part dirty.
    ///
    /// Every cell in the region becomes an ordinary cell again, and each gets back the text it was
    /// holding all along. A cell that is not merged is left alone.
    ///
    /// # Errors
    /// As [`format_cells`](Self::format_cells).
    pub fn unmerge_cells(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();

        // The region is defined by its anchor, which the addressed cell may only point towards.
        let region = self.with_table(surface, &path, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let out_of_range = || PptxError::TableCellOutOfRange {
                row,
                column,
                rows,
                columns,
            };
            let (anchor_row, anchor_column) = table
                .merge_anchor(interner, row, column)
                .ok_or_else(out_of_range)?;
            let anchor = table
                .cell(anchor_row, anchor_column)
                .ok_or_else(out_of_range)?;
            Ok(Cells::rectangle(
                anchor_row..anchor_row + anchor.row_span(interner),
                anchor_column..anchor_column + anchor.column_span(interner),
            ))
        })?;

        self.edit_selected_cells(surface, &path, &region, false, |cell, interner, _, _| {
            let mut typed = TableCell::from_xml(cell, interner)?;
            typed.clear_merge(interner);
            typed.write_back(cell, interner);
            Ok(())
        })
    }
}

/// A `TextSite` naming one cell of the table a shape frames.
pub(super) fn cell(shape: impl Into<ShapePath>, row: usize, column: usize) -> TextSite {
    TextSite::Cell {
        shape: shape.into(),
        row,
        column,
    }
}

/// Checks that no merged region touching the rectangle reaches outside it.
///
/// A region wholly inside is fine — it is absorbed. One that crosses the boundary is not, because
/// truncating it would leave the table claiming a span that no longer fits, and growing the
/// selection to swallow it would merge cells the caller never named.
fn check_merges_fit(
    table: &Table,
    interner: &Interner,
    rows: &core::ops::Range<usize>,
    columns: &core::ops::Range<usize>,
) -> Result<(), PptxError> {
    for row in rows.clone() {
        for column in columns.clone() {
            let Some((anchor_row, anchor_column)) = table.merge_anchor(interner, row, column)
            else {
                continue;
            };
            let Some(anchor) = table.cell(anchor_row, anchor_column) else {
                continue;
            };
            let reaches_row = anchor_row + anchor.row_span(interner);
            let reaches_column = anchor_column + anchor.column_span(interner);
            let contained = anchor_row >= rows.start
                && reaches_row <= rows.end
                && anchor_column >= columns.start
                && reaches_column <= columns.end;
            if !contained {
                return Err(PptxError::TableMergeCrossesSelection { row, column });
            }
        }
    }
    Ok(())
}

/// The `a:tcPr` of a raw `a:tc`, creating it when the cell has none — placed after the cell's
/// `a:txBody`, since `CT_TableCell` is a sequence.
fn cell_properties_slot<'a>(
    cell: &'a mut RawElement,
    interner: &mut Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let index = match cell.children.iter().position(|node| match node {
        RawNode::Element(element) => nav::name_is(&element.name, interner, DML_MAIN, "tcPr"),
        _ => false,
    }) {
        Some(index) => index,
        None => {
            let at = cell
                .children
                .iter()
                .position(|node| match node {
                    RawNode::Element(element) => {
                        !nav::name_is(&element.name, interner, DML_MAIN, "txBody")
                    }
                    _ => false,
                })
                .unwrap_or(cell.children.len());
            let element = build::leaf(interner, "a", DML_MAIN, "tcPr", Vec::new());
            cell.children.insert(at, RawNode::Element(element));
            cell.empty = false;
            at
        }
    };
    match &mut cell.children[index] {
        RawNode::Element(element) => Ok(element),
        _ => Err(PptxError::MalformedSlide(
            "cell properties are not an element",
        )),
    }
}

/// Writes the properties a [`CellFormat`] names onto one cell's `a:tcPr`, leaving the rest alone.
fn apply_cell_format(
    properties: &mut TableCellProperties,
    interner: &mut Interner,
    format: &CellFormat,
) {
    if let Some(fill) = format.fill() {
        properties.set_fill(interner, fill);
    }
    for (edge, line) in format.borders() {
        properties.set_border(interner, *edge, line.as_ref());
    }
    let margins = format.margins();
    properties.set_margins(
        interner,
        margins.left,
        margins.right,
        margins.top,
        margins.bottom,
    );
    let (anchor, direction, overflow) = format.framing();
    if anchor.is_some() {
        properties.set_anchor(interner, anchor);
    }
    if direction.is_some() {
        properties.set_text_direction(interner, direction);
    }
    if overflow.is_some() {
        properties.set_horizontal_overflow(interner, overflow);
    }
    let (material, bevel, light_rig) = format.cell_3d();
    if material.is_some() || bevel.is_some() || light_rig.is_some() {
        let mut cell_3d = Cell3D::new(interner);
        if let Some(material) = material {
            cell_3d.set_material(interner, material);
        }
        if let Some(bevel) = bevel {
            cell_3d.set_bevel(interner, bevel);
        }
        if let Some(light_rig) = light_rig {
            cell_3d.set_light_rig(interner, light_rig);
        }
        properties.set_cell_3d(interner, &cell_3d);
    }
}

// --- Effective cell formatting helpers -----------------------------------------------------------

/// Whether a raw `a:tc` is covered by a merge (states a truthy `hMerge` or `vMerge`), so it renders
/// nothing. A checked cheaply off the attributes, without parsing the whole cell — formatting a
/// selection skips such cells, though merging and unmerging must still reach them.
fn raw_cell_is_covered(cell: &RawElement, interner: &Interner) -> bool {
    cell.attributes.iter().any(|attribute| {
        attribute.name.prefix.is_none()
            && matches!(interner.resolve(attribute.name.local), "hMerge" | "vMerge")
            && matches!(
                std::str::from_utf8(&attribute.value).map(str::trim),
                Ok("1" | "true" | "on")
            )
    })
}

/// The cell at `(row, column)`, or a typed out-of-range error naming the table's shape.
pub(super) fn cell_at(table: &Table, row: usize, column: usize) -> Result<&TableCell, PptxError> {
    let (rows, columns) = (table.row_count(), table.column_count());
    table
        .cell(row, column)
        .ok_or(PptxError::TableCellOutOfRange {
            row,
            column,
            rows,
            columns,
        })
}
