//! Table structure and style: dimensions, column widths and row heights, whole-row and
//! whole-column edits, and the table style a table draws itself with.

use mjx_dml::{
    Emu, Table, TableColumn, TablePart, TablePartStyle, TableProperties, TableRow, TableStyle,
    TableStyleFlags, TableStyleList, TableStylePart,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};
use mjx_opc::{PartName, Relationship, TargetMode};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::surface::Surface;
use crate::table::{TableStyleDefinition, TableStyleFormat};
use crate::{build, constants, nav, slide};

use super::deck::dir_of;
use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::element_builders::build_table_cell;
use super::text::table_dimensions_of;
use super::Presentation;

impl Presentation {
    // -----------------------------------------------------------------------------------------
    // Tables
    //
    // A table is what a `p:graphicFrame` frames, so it is a shape like any other on the index
    // space: it is positioned with `set_shape_bounds`, counted by `shape_count`, and removed by
    // `remove_shape`. What is addressed *inside* it is a cell, by `(row, column)`.
    //
    // Merging never removes a cell, so the grid is rectangular and every position within the table
    // is addressable — a cell covered by a merge is a real cell that simply renders nothing.
    // -----------------------------------------------------------------------------------------

    /// Adds a `rows` x `columns` table to `surface`, laid out inside `bounds`, and returns its
    /// index in the shape tree.
    ///
    /// Columns share the width evenly and rows the height; resize either afterwards with
    /// [`set_column_width`](Self::set_column_width) / [`set_row_height`](Self::set_row_height).
    /// Every cell starts with one empty paragraph, ready for
    /// [`set_cell_text`](Self::set_cell_text).
    ///
    /// The table is a shape: move it with [`set_shape_bounds`](Self::set_shape_bounds), and drop it
    /// with [`remove_shape`](Self::remove_shape).
    ///
    /// # Errors
    /// Returns [`PptxError::InvalidTableSize`] if either dimension is zero — a table with no cells
    /// is not something PowerPoint will open — or another [`PptxError`] if the surface index is out
    /// of range or the part is malformed.
    pub fn add_table(
        &mut self,
        surface: impl Into<Surface>,
        rows: usize,
        columns: usize,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        if rows == 0 || columns == 0 {
            return Err(PptxError::InvalidTableSize { rows, columns });
        }
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let frame = build_table_frame(interner, next_id, rows, columns, bounds);
        sp_tree.children.push(RawNode::Element(frame));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// The shape of the table shape `shape_idx` on `surface` frames, as `(rows, columns)`.
    ///
    /// The column count comes from the table's `a:tblGrid`, which is where a table declares its
    /// width — not from counting some row's cells. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotATable`] if the shape frames no table, or another
    /// [`PptxError`] if an index is out of range or the part is malformed.
    pub fn table_dimensions(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(usize, usize), PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let _ = interner;
            Ok((table.row_count(), table.column_count()))
        })
    }

    /// The width of column `column` of the table shape `shape_idx` frames, or `None` if the column
    /// states none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`] if there is no such column.
    pub fn column_width(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        column: usize,
    ) -> Result<Option<Emu>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let columns = table.column_count();
            let grid_column = table.grid().and_then(|grid| grid.column(column)).ok_or(
                PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows: table.row_count(),
                    columns,
                },
            )?;
            Ok(grid_column.width(interner))
        })
    }

    /// Sets the width of column `column`. Marks only that part dirty.
    ///
    /// The frame's own bounds are **not** adjusted: a table whose columns no longer sum to its
    /// frame width is what PowerPoint itself produces when a column is dragged, and the frame is
    /// resized separately with [`set_shape_bounds`](Self::set_shape_bounds).
    ///
    /// # Errors
    /// As [`column_width`](Self::column_width).
    pub fn set_column_width(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        column: usize,
        width: Emu,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let (rows, columns) = table_dimensions_of(table, interner);
            if column >= columns {
                return Err(PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows,
                    columns,
                });
            }
            let grid = nav::child_mut(table, interner, DML_MAIN, "tblGrid")
                .ok_or(PptxError::MalformedSlide("table has no a:tblGrid"))?;
            let slot = nav::nth_child_matching_mut(grid, interner, column, |element, interner| {
                nav::name_is(&element.name, interner, DML_MAIN, "gridCol")
            })
            .ok_or(PptxError::MalformedSlide("table column vanished"))?;
            // Through the model's own setter, so a width has one spelling in the codebase.
            let mut typed = TableColumn::from_xml(slot, interner)?;
            typed.set_width(interner, width);
            *slot = typed.to_xml(interner);
            Ok(())
        })
    }

    /// The height row `row` asks for, or `None` if it states none. PowerPoint grows a row whose
    /// content does not fit, so a rendered row is never shorter than this but may be taller.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`] if there is no such row.
    pub fn row_height(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
    ) -> Result<Option<Emu>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let rows = table.row_count();
            let table_row = table.row(row).ok_or(PptxError::TableCellOutOfRange {
                row,
                column: 0,
                rows,
                columns: table.column_count(),
            })?;
            Ok(table_row.height(interner))
        })
    }

    /// Sets the height row `row` asks for. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`row_height`](Self::row_height).
    pub fn set_row_height(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        height: Emu,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let (rows, columns) = table_dimensions_of(table, interner);
            if row >= rows {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column: 0,
                    rows,
                    columns,
                });
            }
            let slot = slide::nth_row_mut(table, interner, row)
                .ok_or(PptxError::MalformedSlide("table row vanished"))?;
            let mut typed = TableRow::from_xml(slot, interner)?;
            typed.set_height(interner, height);
            *slot = typed.to_xml(interner);
            Ok(())
        })
    }

    // ---------------------------------------------------------------------------------------------
    // Structural edits — grow and shrink a table by whole rows and columns.
    //
    // Unlike a cell text edit (which reaches one `a:tc` in the raw tree), a row or column edit
    // touches every row, so the whole `a:tbl` is parsed to the typed `Table`, mutated there — where
    // merge adjustment and anchor promotion are expressed in terms of the model — and written back.
    // Round-tripping the fidelity wrappers preserves everything this workstream does not model, and
    // the span-adjustment logic itself lives in `mjx-dml`. These wrappers own only the range checks,
    // which need the dimensions the model already reports.
    // ---------------------------------------------------------------------------------------------

    /// Inserts a row into the table shape `shape_idx` frames so it becomes row `row`; `row` equal to
    /// the current row count appends at the end. The new row copies the height of the row beside it
    /// and its cells are empty and ready for [`set_cell_text`](Self::set_cell_text). A merge the new
    /// row falls inside grows to include it. Marks only that part dirty; the frame's own bounds are
    /// **not** enlarged (as PowerPoint does not either — resize with
    /// [`set_shape_bounds`](Self::set_shape_bounds)).
    ///
    /// # Errors
    /// [`PptxError::TableCellOutOfRange`] if `row` is past the end, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn insert_row(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let rows = typed.row_count();
            if row > rows {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column: 0,
                    rows,
                    columns: typed.column_count(),
                });
            }
            typed.insert_row(interner, row, build_table_cell)?;
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// Removes row `row` from the table shape `shape_idx` frames. A merge the row lies inside shrinks;
    /// a merge anchored in the row promotes the cell below it, which takes over the anchor's text and
    /// formatting so the table looks unchanged. Marks only that part dirty.
    ///
    /// # Errors
    /// [`PptxError::InvalidTableSize`] if `row` is the table's only row (a table cannot have none),
    /// [`PptxError::TableCellOutOfRange`] if `row` is out of range, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn remove_row(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let (rows, columns) = (typed.row_count(), typed.column_count());
            if row >= rows {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column: 0,
                    rows,
                    columns,
                });
            }
            if rows == 1 {
                return Err(PptxError::InvalidTableSize { rows: 0, columns });
            }
            typed.remove_row(interner, row);
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// Inserts a column into the table shape `shape_idx` frames so it becomes column `column`;
    /// `column` equal to the current column count appends. The grid gains one `a:gridCol` (width
    /// copied from the column beside it) and every row gains one empty cell, so the grid and rows
    /// stay in step. A merge the new column falls inside grows to include it. Marks only that part
    /// dirty; the frame's own bounds are **not** enlarged.
    ///
    /// # Errors
    /// [`PptxError::TableCellOutOfRange`] if `column` is past the end, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn insert_column(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        column: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let columns = typed.column_count();
            if column > columns {
                return Err(PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows: typed.row_count(),
                    columns,
                });
            }
            typed.insert_column(interner, column, build_table_cell)?;
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// Removes column `column` from the table shape `shape_idx` frames: its `a:gridCol` and one cell
    /// from every row, together. A merge the column lies inside shrinks; a merge anchored in the
    /// column promotes the cell to its right, which takes over the anchor's text and formatting.
    /// Marks only that part dirty.
    ///
    /// # Errors
    /// [`PptxError::InvalidTableSize`] if `column` is the table's only column,
    /// [`PptxError::TableCellOutOfRange`] if `column` is out of range, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn remove_column(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        column: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let (rows, columns) = (typed.row_count(), typed.column_count());
            if column >= columns {
                return Err(PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows,
                    columns,
                });
            }
            if columns == 1 {
                return Err(PptxError::InvalidTableSize { rows, columns: 0 });
            }
            typed.remove_column(interner, column);
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// How many columns and rows the cell at `(row, column)` spans, as `(columns, rows)`.
    ///
    /// `(1, 1)` for an ordinary cell. A cell **covered** by a merge also reports `(1, 1)` — ask
    /// [`merged_cell_anchor`](Self::merged_cell_anchor) which cell actually renders there.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`].
    pub fn cell_span(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<(usize, usize), PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let cell = table
                .cell(row, column)
                .ok_or(PptxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                })?;
            Ok((cell.column_span(interner), cell.row_span(interner)))
        })
    }

    /// Which cell actually renders at `(row, column)` — itself when it is not merged away, or the
    /// anchor of the merged region covering it.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`].
    pub fn merged_cell_anchor(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        row: usize,
        column: usize,
    ) -> Result<(usize, usize), PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            table
                .merge_anchor(interner, row, column)
                .ok_or(PptxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                })
        })
    }

    /// Reads the table shape `shape_idx` frames as a typed [`Table`] and hands it, with the part's
    /// interner, to `read`. Does **not** dirty the part.
    pub(super) fn with_table<R>(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        read: impl FnOnce(&Table, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        let element =
            slide::shape_table(shape, &doc.interner).ok_or(PptxError::ShapeIsNotATable)?;
        let table = Table::from_xml(element, &doc.interner)?;
        read(&table, &doc.interner)
    }

    /// Hands the raw `a:tbl` of the table shape `shape_idx` frames to `edit`, which reaches the one
    /// child it means to change.
    ///
    /// The table element itself is not reparsed or rebuilt — only whatever `edit` replaces — so
    /// resizing a column costs one small element, not the whole table.
    fn edit_table_child(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        edit: impl FnOnce(&mut RawElement, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        let table = slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
        edit(table, interner)
    }

    // ---------------------------------------------------------------------------------------------
    // Table styles and the seven a:tblPr flags.
    //
    // The flags (`firstRow`, `bandRow`, …) live on the table's own `a:tblPr`; they emphasize nothing
    // by themselves, they tell the table **style** which parts to treat specially. The style lives in
    // the presentation's `tableStyles.xml` part, named by GUID from `a:tblPr > a:tableStyleId`. This
    // block reads and writes both: the flags on the table, the style in the shared part.
    // ---------------------------------------------------------------------------------------------

    /// Whether the table shape `shape_idx` frames declares banding/emphasis `part` (a `a:tblPr` flag),
    /// or `None` if it does not state the flag. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn table_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        part: TablePart,
    ) -> Result<Option<bool>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            Ok(table
                .properties()
                .and_then(|props| props.part(interner, part)))
        })
    }

    /// Turns a table's banding/emphasis flag `part` on or off, creating its `a:tblPr` if it had none.
    /// `false` removes the flag rather than writing a `"0"`. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn set_table_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        part: TablePart,
        on: bool,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |props, interner| {
            props.set_part(interner, part, on);
            Ok(())
        })
    }

    /// The GUID of the table style the table shape `shape_idx` frames names (`a:tableStyleId`), or
    /// `None` if it names none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn table_style_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            Ok(table
                .properties()
                .and_then(|props| props.table_style_id(interner))
                .map(str::to_owned))
        })
    }

    /// Points the table shape `shape_idx` frames at the table style `style_id`, creating its
    /// `a:tblPr` if it had none. Does not check that the style exists — pair it with
    /// [`create_table_style`](Self::create_table_style). Marks only that part dirty.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn set_table_style(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        style_id: &str,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |props, interner| {
            props.set_table_style_id(interner, style_id);
            Ok(())
        })
    }

    /// Creates the presentation's `tableStyles.xml` part if it has none, and adds a style with GUID
    /// `style_id` and gallery name `style_name` — replacing one already carrying that GUID. The style
    /// is born empty; give its parts formatting with
    /// [`format_table_style_part`](Self::format_table_style_part), and point a table at it with
    /// [`set_table_style`](Self::set_table_style).
    ///
    /// # Errors
    /// Returns a [`PptxError`] if the package is malformed or the part cannot be created.
    pub fn create_table_style(
        &mut self,
        style_id: &str,
        style_name: &str,
    ) -> Result<(), PptxError> {
        let part = self.ensure_table_styles_part(style_id)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut list = TableStyleList::from_xml(root, interner)?;
        let style = TableStyle::new(interner, style_id, style_name);
        list.upsert_style(interner, &style);
        *root = list.to_xml(interner);
        Ok(())
    }

    /// Sets the formatting the style `style_id` gives table `part` (`wholeTbl`, `firstRow`, a banded
    /// row, a corner cell). Only the facets `format` sets are written; the part keeps whatever else
    /// it held. Marks only the `tableStyles.xml` part dirty.
    ///
    /// # Errors
    /// [`PptxError::TableStyleNotFound`] if no `tableStyles.xml` defines `style_id`.
    pub fn format_table_style_part(
        &mut self,
        style_id: &str,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), PptxError> {
        let not_found = || PptxError::TableStyleNotFound {
            style_id: style_id.to_owned(),
        };
        let part_name = self.table_styles_part()?.ok_or_else(not_found)?;
        let doc = self.package.part_tree_mut(&part_name)?;
        let RawDocument { interner, root, .. } = doc;
        let mut list = TableStyleList::from_xml(root, interner)?;
        let mut style = list.style(interner, style_id).ok_or_else(not_found)?;
        let mut part_style = style
            .part(interner, part)
            .unwrap_or_else(|| TablePartStyle::new(interner));
        format.apply(&mut part_style, interner);
        style.set_part(interner, part, &part_style);
        list.upsert_style(interner, &style);
        *root = list.to_xml(interner);
        Ok(())
    }

    /// Gives the table shape `shape_idx` frames its own **inline** style (`a:tableStyle`), replacing
    /// any inline or referenced style it had — the lean alternative to a shared `tableStyles.xml`
    /// style: the whole look is spelled out in `definition` and travels with the table, so no shared
    /// part, relationship or referenced GUID is involved. Marks only that part dirty.
    ///
    /// A styled part renders only when the table declares it: pair this with
    /// [`set_table_part`](Self::set_table_part) to turn on the `firstRow` / `bandRow` / … flags a part
    /// needs (a table from [`add_table`](Self::add_table) already has `firstRow` and `bandRow` on).
    /// The style resolves through [`with_table_style`](Self::with_table_style) and the
    /// `effective_cell_*` readers exactly as a shared one does.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn set_inline_table_style(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        definition: &TableStyleDefinition,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |properties, interner| {
            let mut style =
                TableStyle::new(interner, definition.style_id(), definition.style_name());
            for (part, format) in definition.parts() {
                let mut part_style = TablePartStyle::new(interner);
                format.apply(&mut part_style, interner);
                style.set_part(interner, *part, &part_style);
            }
            properties.set_inline_style(interner, &style);
            Ok(())
        })
    }

    /// Sets the formatting the table's **inline** style gives one `part`, creating the inline style if
    /// the table had none — the incremental sibling of [`set_inline_table_style`](Self::set_inline_table_style),
    /// mirroring [`format_table_style_part`](Self::format_table_style_part) for a self-contained style.
    /// Only the facets `format` sets are written. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn format_inline_table_style_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |properties, interner| {
            let mut style = properties.inline_style(interner).unwrap_or_else(|| {
                TableStyle::new(
                    interner,
                    crate::table::DEFAULT_INLINE_STYLE_ID,
                    crate::table::DEFAULT_INLINE_STYLE_NAME,
                )
            });
            let mut part_style = style
                .part(interner, part)
                .unwrap_or_else(|| TablePartStyle::new(interner));
            format.apply(&mut part_style, interner);
            style.set_part(interner, part, &part_style);
            properties.set_inline_style(interner, &style);
            Ok(())
        })
    }

    /// Reads the table style the table shape `shape_idx` frames resolves to and hands it, with the
    /// `tableStyles.xml` interner, to `read`. `None` when the table names no style or the named style
    /// is not defined. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), or if the package is malformed.
    pub fn with_table_style<R>(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        read: impl FnOnce(&TableStyle, &Interner) -> Result<R, PptxError>,
    ) -> Result<Option<R>, PptxError> {
        self.with_resolved_style(surface.into(), shape_idx, read)
    }

    /// The style a table resolves to, handed to `read` — an **inline** `a:tableStyle` if the table
    /// carries one, else the shared style its `a:tableStyleId` names. `None` when it resolves to
    /// neither. An inline style is read against the slide part's interner (where it lives), a shared
    /// one against the `tableStyles.xml` interner; either way the [`TableStyle`] model is the same.
    pub(super) fn with_resolved_style<R>(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        read: impl FnOnce(&TableStyle, &Interner) -> Result<R, PptxError>,
    ) -> Result<Option<R>, PptxError> {
        let path = shape_idx.into();
        // An inline style wins, and lives in the slide part — the `TableStyle` is owned, but its
        // symbols resolve against that part's interner, which re-opening the part hands back.
        let inline = self.with_table(surface, &path, |table, interner| {
            Ok(table
                .properties()
                .and_then(|properties| properties.inline_style(interner)))
        })?;
        if let Some(style) = inline {
            let part = self.surface_part(surface)?;
            let doc = self.package.part_tree(&part)?;
            return read(&style, &doc.interner).map(Some);
        }

        let Some(style_id) = self.table_style_id(surface, &path)? else {
            return Ok(None);
        };
        let Some(part) = self.table_styles_part()? else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&part)?;
        let list = TableStyleList::from_xml(&doc.root, &doc.interner)?;
        match list.style(&doc.interner, &style_id) {
            Some(style) => read(&style, &doc.interner).map(Some),
            None => Ok(None),
        }
    }

    /// Reads the table's `a:tblPr` (creating it if absent) as a typed [`TableProperties`], hands it to
    /// `edit`, and writes it back. Only the `a:tblPr` is reparsed — the rest of the table is untouched.
    fn edit_table_properties(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        edit: impl FnOnce(&mut TableProperties, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface, shape_idx, |table, interner| {
            let slot = table_properties_slot(table, interner)?;
            let mut typed = TableProperties::from_xml(slot, interner)?;
            edit(&mut typed, interner)?;
            *slot = typed.to_xml(interner);
            Ok(())
        })
    }

    /// The presentation's `tableStyles.xml` part, or `None` if it has none.
    fn table_styles_part(&self) -> Result<Option<PartName>, PptxError> {
        self.follow_rel(&self.presentation_part, constants::REL_TABLE_STYLES)
    }

    /// The `tableStyles.xml` part, creating it (with an empty list whose default is `default_style_id`)
    /// and wiring its relationship and content type if the presentation had none.
    pub(super) fn ensure_table_styles_part(
        &mut self,
        default_style_id: &str,
    ) -> Result<PartName, PptxError> {
        if let Some(part) = self.table_styles_part()? {
            return Ok(part);
        }
        let part = PartName::new(&format!(
            "{}tableStyles.xml",
            dir_of(self.presentation_part.as_str())
        ))?;
        self.package.insert_part(
            &part,
            constants::CONTENT_TYPE_TABLE_STYLES,
            build::table_styles_bytes(default_style_id),
        )?;
        let rel_id = self.next_presentation_rid()?;
        let target = nav::relative_target(&self.presentation_part, &part);
        self.package.add_relationship(
            Some(&self.presentation_part),
            Relationship {
                id: rel_id,
                rel_type: constants::REL_TABLE_STYLES.to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;
        Ok(part)
    }
}

/// The `a:tblPr` of a raw `a:tbl`, creating it when the table has none — placed **first**, since
/// `CT_Table` is a sequence of `tblPr?`, `tblGrid`, `tr*`.
fn table_properties_slot<'a>(
    table: &'a mut RawElement,
    interner: &mut Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let index = match table.children.iter().position(|node| match node {
        RawNode::Element(element) => nav::name_is(&element.name, interner, DML_MAIN, "tblPr"),
        _ => false,
    }) {
        Some(index) => index,
        None => {
            let element = build::leaf(interner, "a", DML_MAIN, "tblPr", Vec::new());
            table.children.insert(0, RawNode::Element(element));
            table.empty = false;
            0
        }
    };
    match &mut table.children[index] {
        RawNode::Element(element) => Ok(element),
        _ => Err(PptxError::MalformedSlide(
            "table properties are not an element",
        )),
    }
}

/// The table's banding/emphasis flags, or all-false when it declares no `a:tblPr`.
pub(super) fn table_flags(table: &Table, interner: &Interner) -> TableStyleFlags {
    table
        .properties()
        .map(|properties| TableStyleFlags::from_properties(properties, interner))
        .unwrap_or_default()
}

/// A whole `p:graphicFrame` holding a `rows` x `columns` table, laid out inside `bounds`.
///
/// Columns share the width evenly and rows the height — a caller resizes either afterwards. Each
/// cell gets an `a:txBody` with one empty paragraph, because PowerPoint expects a cell to have one
/// and a caller's first act is to put text in it. `firstRow` and `bandRow` are what PowerPoint
/// itself writes for a new table: they claim nothing about appearance on their own, they tell a
/// table style which parts to emphasize.
pub(super) fn build_table_frame(
    interner: &mut Interner,
    id: u32,
    rows: usize,
    columns: usize,
    bounds: ShapeBounds,
) -> RawElement {
    // p:nvGraphicFramePr — cNvPr, cNvGraphicFramePr (locked against grouping, as Office writes it),
    // and an empty nvPr.
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &format!("Table {id}")),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let lock_attrs = vec![build::attr(interner, "noGrp", "1")];
    let frame_locks = build::leaf(interner, "a", DML_MAIN, "graphicFrameLocks", lock_attrs);
    let c_nv_frame_pr = build::node(
        interner,
        "p",
        PML,
        "cNvGraphicFramePr",
        Vec::new(),
        vec![RawNode::Element(frame_locks)],
    );
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    let nv_frame_pr = build::node(
        interner,
        "p",
        PML,
        "nvGraphicFramePr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_frame_pr),
            RawNode::Element(nv_pr),
        ],
    );

    // p:xfrm — a graphic frame's transform is PresentationML's, not DrawingML's, and is required.
    let mut xfrm = build::node(interner, "p", PML, "xfrm", Vec::new(), Vec::new());
    bounds.to_transform().apply(&mut xfrm, interner);

    // a:tblGrid — the grid is where a table declares its width.
    let column_width = bounds.width_emu / columns.max(1) as i64;
    let grid_columns: Vec<RawNode> = (0..columns)
        .map(|index| {
            // The last column absorbs the rounding, so the columns sum to the frame's width.
            let width = if index + 1 == columns {
                bounds.width_emu - column_width * (columns as i64 - 1)
            } else {
                column_width
            };
            let attrs = vec![build::attr(interner, "w", &width.to_string())];
            RawNode::Element(build::leaf(interner, "a", DML_MAIN, "gridCol", attrs))
        })
        .collect();
    let grid = build::node(interner, "a", DML_MAIN, "tblGrid", Vec::new(), grid_columns);

    let row_height = bounds.height_emu / rows.max(1) as i64;
    let table_rows: Vec<RawNode> = (0..rows)
        .map(|_| {
            let cells: Vec<RawNode> = (0..columns)
                .map(|_| RawNode::Element(build_table_cell(interner)))
                .collect();
            let attrs = vec![build::attr(interner, "h", &row_height.to_string())];
            RawNode::Element(build::node(interner, "a", DML_MAIN, "tr", attrs, cells))
        })
        .collect();

    let tbl_pr_attrs = vec![
        build::attr(interner, "firstRow", "1"),
        build::attr(interner, "bandRow", "1"),
    ];
    let tbl_pr = build::leaf(interner, "a", DML_MAIN, "tblPr", tbl_pr_attrs);
    let mut table_children = vec![RawNode::Element(tbl_pr), RawNode::Element(grid)];
    table_children.extend(table_rows);
    let table = build::node(interner, "a", DML_MAIN, "tbl", Vec::new(), table_children);

    let data_attrs = vec![build::attr(interner, "uri", slide::TABLE_GRAPHIC_URI)];
    let graphic_data = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphicData",
        data_attrs,
        vec![RawNode::Element(table)],
    );
    let graphic = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphic",
        Vec::new(),
        vec![RawNode::Element(graphic_data)],
    );

    build::node(
        interner,
        "p",
        PML,
        "graphicFrame",
        Vec::new(),
        vec![
            RawNode::Element(nv_frame_pr),
            RawNode::Element(xfrm),
            RawNode::Element(graphic),
        ],
    )
}
