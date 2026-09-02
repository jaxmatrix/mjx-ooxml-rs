//! Table structure: dimensions, row and column sizing, insertion and removal, merge spans, and the
//! table styles a table binds.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{
    Deck, Emu, Error, ShapeBounds, ShapePath, Surface, TablePart, TableStyleDefinition,
    TableStyleFormat, TableStylePart,
};

impl Deck {
    /// Adds a `rows` x `columns` table to `surface`, laid out inside `bounds`, and returns its index in
    /// the shape tree.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_table`](mjx_pptx::Presentation::add_table).
    pub fn add_table(
        &mut self,
        surface: Surface,
        rows: u32,
        columns: u32,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_table(
            surface.to_model(),
            index(rows),
            index(columns),
            bounds,
        )?))
    }

    /// The shape of the table shape `shape_idx` on `surface` frames, as `(rows, columns)`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::table_dimensions`](mjx_pptx::Presentation::table_dimensions).
    pub fn table_dimensions(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(u32, u32), Error> {
        Ok({
            let (a, b) = self
                .presentation
                .table_dimensions(surface.to_model(), shape_idx.to_model())?;
            (count(a), count(b))
        })
    }

    /// The width of column `column` of the table shape `shape_idx` frames, or `None` if the column
    /// states none. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::column_width`](mjx_pptx::Presentation::column_width).
    pub fn column_width(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        column: u32,
    ) -> Result<Option<Emu>, Error> {
        Ok(self.presentation.column_width(
            surface.to_model(),
            shape_idx.to_model(),
            index(column),
        )?)
    }

    /// Sets the width of column `column`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_column_width`](mjx_pptx::Presentation::set_column_width).
    pub fn set_column_width(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        column: u32,
        width: Emu,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_column_width(
            surface.to_model(),
            shape_idx.to_model(),
            index(column),
            width,
        )?)
    }

    /// The height row `row` asks for, or `None` if it states none. PowerPoint grows a row whose content
    /// does not fit, so a rendered row is never shorter than this but may be taller.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::row_height`](mjx_pptx::Presentation::row_height).
    pub fn row_height(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
    ) -> Result<Option<Emu>, Error> {
        Ok(self
            .presentation
            .row_height(surface.to_model(), shape_idx.to_model(), index(row))?)
    }

    /// Sets the height row `row` asks for. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_row_height`](mjx_pptx::Presentation::set_row_height).
    pub fn set_row_height(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        height: Emu,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_row_height(
            surface.to_model(),
            shape_idx.to_model(),
            index(row),
            height,
        )?)
    }

    /// Inserts a row into the table shape `shape_idx` frames so it becomes row `row`; `row` equal to
    /// the current row count appends at the end. The new row copies the height of the row beside it and
    /// its cells are empty and ready for `set_cell_text`. A merge the new row falls inside grows to
    /// include it. Marks only that part dirty; the frame's own bounds are **not** enlarged (as
    /// PowerPoint does not either — resize with `set_shape_bounds`).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::insert_row`](mjx_pptx::Presentation::insert_row).
    pub fn insert_row(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .insert_row(surface.to_model(), shape_idx.to_model(), index(row))?)
    }

    /// Removes row `row` from the table shape `shape_idx` frames. A merge the row lies inside shrinks;
    /// a merge anchored in the row promotes the cell below it, which takes over the anchor's text and
    /// formatting so the table looks unchanged. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_row`](mjx_pptx::Presentation::remove_row).
    pub fn remove_row(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .remove_row(surface.to_model(), shape_idx.to_model(), index(row))?)
    }

    /// Inserts a column into the table shape `shape_idx` frames so it becomes column `column`; `column`
    /// equal to the current column count appends. The grid gains one `a:gridCol` (width copied from the
    /// column beside it) and every row gains one empty cell, so the grid and rows stay in step. A merge
    /// the new column falls inside grows to include it. Marks only that part dirty; the frame's own
    /// bounds are **not** enlarged.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::insert_column`](mjx_pptx::Presentation::insert_column).
    pub fn insert_column(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        column: u32,
    ) -> Result<(), Error> {
        Ok(self.presentation.insert_column(
            surface.to_model(),
            shape_idx.to_model(),
            index(column),
        )?)
    }

    /// Removes column `column` from the table shape `shape_idx` frames: its `a:gridCol` and one cell
    /// from every row, together. A merge the column lies inside shrinks; a merge anchored in the column
    /// promotes the cell to its right, which takes over the anchor's text and formatting. Marks only
    /// that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_column`](mjx_pptx::Presentation::remove_column).
    pub fn remove_column(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        column: u32,
    ) -> Result<(), Error> {
        Ok(self.presentation.remove_column(
            surface.to_model(),
            shape_idx.to_model(),
            index(column),
        )?)
    }

    /// How many rows and columns the cell at `(row, column)` spans, as `(rows, columns)` — the same
    /// order `table_dimensions` answers in, and the order every address on this surface is written in.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::cell_span`](mjx_pptx::Presentation::cell_span).
    pub fn cell_span(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<(u32, u32), Error> {
        Ok({
            let (a, b) = self.presentation.cell_span(
                surface.to_model(),
                shape_idx.to_model(),
                index(row),
                index(column),
            )?;
            (count(a), count(b))
        })
    }

    /// Which cell actually renders at `(row, column)` — itself when it is not merged away, or the
    /// anchor of the merged region covering it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::merged_cell_anchor`](mjx_pptx::Presentation::merged_cell_anchor).
    pub fn merged_cell_anchor(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        row: u32,
        column: u32,
    ) -> Result<(u32, u32), Error> {
        Ok({
            let (a, b) = self.presentation.merged_cell_anchor(
                surface.to_model(),
                shape_idx.to_model(),
                index(row),
                index(column),
            )?;
            (count(a), count(b))
        })
    }

    /// Whether the table shape `shape_idx` frames declares banding/emphasis `part` (a `a:tblPr` flag),
    /// or `None` if it does not state the flag. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::table_part`](mjx_pptx::Presentation::table_part).
    pub fn table_part(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        part: TablePart,
    ) -> Result<Option<bool>, Error> {
        Ok(self
            .presentation
            .table_part(surface.to_model(), shape_idx.to_model(), part)?)
    }

    /// Turns a table's banding/emphasis flag `part` on or off, creating its `a:tblPr` if it had none.
    /// `false` removes the flag rather than writing a `"0"`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_table_part`](mjx_pptx::Presentation::set_table_part).
    pub fn set_table_part(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        part: TablePart,
        on: bool,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_table_part(surface.to_model(), shape_idx.to_model(), part, on)?)
    }

    /// The GUID of the table style the table shape `shape_idx` frames names (`a:tableStyleId`), or
    /// `None` if it names none. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::table_style_id`](mjx_pptx::Presentation::table_style_id).
    pub fn table_style_id(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .presentation
            .table_style_id(surface.to_model(), shape_idx.to_model())?)
    }

    /// Points the table shape `shape_idx` frames at the table style `style_id`, creating its `a:tblPr`
    /// if it had none. Does not check that the style exists — pair it with `create_table_style`. Marks
    /// only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_table_style`](mjx_pptx::Presentation::set_table_style).
    pub fn set_table_style(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        style_id: &str,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_table_style(surface.to_model(), shape_idx.to_model(), style_id)?)
    }

    /// Creates the presentation's `tableStyles.xml` part if it has none, and adds a style with GUID
    /// `style_id` and gallery name `style_name` — replacing one already carrying that GUID. The style
    /// is born empty; give its parts formatting with `format_table_style_part`, and point a table at it
    /// with `set_table_style`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::create_table_style`](mjx_pptx::Presentation::create_table_style).
    pub fn create_table_style(&mut self, style_id: &str, style_name: &str) -> Result<(), Error> {
        Ok(self.presentation.create_table_style(style_id, style_name)?)
    }

    /// Sets the formatting the style `style_id` gives table `part` (`wholeTbl`, `firstRow`, a banded
    /// row, a corner cell). Only the facets `format` sets are written; the part keeps whatever else it
    /// held. Marks only the `tableStyles.xml` part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::format_table_style_part`](mjx_pptx::Presentation::format_table_style_part).
    pub fn format_table_style_part(
        &mut self,
        style_id: &str,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .format_table_style_part(style_id, part, format)?)
    }

    /// Gives the table shape `shape_idx` frames its own **inline** style (`a:tableStyle`), replacing
    /// any inline or referenced style it had — the lean alternative to a shared `tableStyles.xml`
    /// style: the whole look is spelled out in `definition` and travels with the table, so no shared
    /// part, relationship or referenced GUID is involved. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_inline_table_style`](mjx_pptx::Presentation::set_inline_table_style).
    pub fn set_inline_table_style(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        definition: &TableStyleDefinition,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_inline_table_style(
            surface.to_model(),
            shape_idx.to_model(),
            definition,
        )?)
    }

    /// Sets the formatting the table's **inline** style gives one `part`, creating the inline style if
    /// the table had none — the incremental sibling of `set_inline_table_style`, mirroring
    /// `format_table_style_part` for a self-contained style. Only the facets `format` sets are written.
    /// Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::format_inline_table_style_part`](mjx_pptx::Presentation::format_inline_table_style_part).
    pub fn format_inline_table_style_part(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), Error> {
        Ok(self.presentation.format_inline_table_style_part(
            surface.to_model(),
            shape_idx.to_model(),
            part,
            format,
        )?)
    }
}
