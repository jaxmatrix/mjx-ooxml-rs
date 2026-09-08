//! [`SheetGrid`] — one worksheet, read once and held, which is what
//! [`BoxModel::Content`](mjx_layout::BoxModel::Content) has to be.
//!
//! # Why a snapshot, and why it is also the residency
//!
//! [`BoxModel::layout_page`](mjx_layout::BoxModel::layout_page) takes `&Self::Content`, so the
//! content must be something a shared reference can answer from. For PowerPoint that forced a
//! snapshot outright, because every `mjx_pptx::Presentation` reader is `&mut self` for lazy part
//! parsing. **Excel's readers are `&self`**, so `Content = Workbook` would have compiled — and it
//! would have been catastrophically slow, which is the more important half.
//!
//! `crates/mjx-xlsx/docs/guide/large_workbooks.md` says why in its own words: *"A worksheet is cheap
//! to hold and expensive to open, and this library holds nothing between calls… every call that
//! reaches into a sheet's cells parses that sheet's part again."* `docs/BENCHMARKS.md` measures the
//! first `worksheet_markup` on a 300,000-cell sheet at roughly half a second. A box model that
//! called a `Workbook` accessor per visible cell would re-parse the whole worksheet two thousand
//! times a frame.
//!
//! So the snapshot holds [`mjx_xlsx::SheetFormatting`] — one worksheet **and** the styles part it
//! resolves against, both parsed, owning neither the package nor a borrow of it. That type is
//! MJXOFF-167's residency seen from the layout side: parse once, resolve many. Everything a viewport
//! asks for afterwards is a binary search in a packed store.
//!
//! # What is read eagerly, and what is not
//!
//! Eagerly, because each is `O(stated elements)` and answers every later question:
//! the row index, the merge index, the pane, the used range, and the shared-string table.
//!
//! **Not** the column geometry, and that is a deliberate split: a column's width is quoted in
//! characters of the workbook's Normal font, so building it needs a face resolved and a glyph
//! measured. That belongs to the box model, which owns the font engine, and it is why
//! [`SheetGrid::rows`] is here while `columns` is not.

use mjx_layout::PartId;
use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_sml::{
    CellReference, GridBounds, InlineString, SharedStringTable, SheetFormatProperties,
    WorksheetPart,
};
use mjx_xlsx::{SheetFormatting, Workbook};

use crate::address;
use crate::error::SheetLayoutError;
use crate::geometry::{RowGeometry, COLUMN_COUNT, ROW_COUNT};
use crate::merge::MergeIndex;
use crate::panes::PaneSplit;

/// One worksheet, read once: the cells, the styles, the geometry overrides, the merges and the pane.
#[derive(Debug)]
pub struct SheetGrid {
    index: usize,
    name: String,
    formatting: SheetFormatting,
    shared_strings: Option<SharedStringTable>,
    rows: RowGeometry,
    merges: MergeIndex,
    split: PaneSplit,
    used: Option<GridBounds>,
}

impl SheetGrid {
    /// Reads the tab at `index` of `workbook`.
    ///
    /// Takes `&Workbook` rather than `&mut Workbook`: nothing here writes, and reading a part does
    /// not dirty the package — the same courtesy [`Workbook::sheet_formatting`] extends.
    ///
    /// # Errors
    /// [`SheetLayoutError::NoSuchSheet`] when the workbook has no such tab;
    /// [`SheetLayoutError::NotAWorksheet`] when the tab reaches no worksheet part or the workbook
    /// relates to no styles part; [`SheetLayoutError::Workbook`] when a part will not parse.
    pub fn read(workbook: &Workbook, index: usize) -> Result<Self, SheetLayoutError> {
        let sheets = workbook.sheets();
        let Some(entry) = sheets.get(index) else {
            return Err(SheetLayoutError::NoSuchSheet {
                requested: index,
                count: sheets.len(),
            });
        };
        let name = entry.name.clone();
        let Some(formatting) = workbook.sheet_formatting(index)? else {
            return Err(SheetLayoutError::NotAWorksheet { index });
        };
        let shared_strings = workbook.shared_strings()?;
        Ok(Self::from_parts(index, name, formatting, shared_strings))
    }

    /// The snapshot over parts a caller already holds — the path a suite that authored a worksheet
    /// in memory takes, and the one [`SheetGrid::read`] itself ends at.
    #[must_use]
    pub fn from_parts(
        index: usize,
        name: String,
        formatting: SheetFormatting,
        shared_strings: Option<SharedStringTable>,
    ) -> Self {
        let worksheet = formatting.worksheet();
        let format = worksheet.format_properties();
        let rows = RowGeometry::read(worksheet, format);
        let merges = MergeIndex::read(worksheet).unwrap_or_default();
        let split = PaneSplit::read(worksheet);
        let used = worksheet
            .dimension()
            .and_then(|dimension| dimension.range(worksheet.interner()).ok())
            .map(mjx_sml::CellRange::normalized_bounds);
        Self {
            index,
            name,
            formatting,
            shared_strings,
            rows,
            merges,
            split,
            used,
        }
    }

    /// Which tab this is.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The part number this sheet's fragments are addressed under.
    #[must_use]
    pub fn part(&self) -> PartId {
        address::part_of(self.index)
    }

    /// The tab's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The worksheet and the styles part, both parsed.
    #[must_use]
    pub fn formatting(&self) -> &SheetFormatting {
        &self.formatting
    }

    /// The worksheet's model.
    #[must_use]
    pub fn worksheet(&self) -> &WorksheetPart {
        self.formatting.worksheet()
    }

    /// `sheetFormatPr`, or `None` for a sheet that writes none.
    #[must_use]
    pub fn format_properties(&self) -> Option<&SheetFormatProperties> {
        self.worksheet().format_properties()
    }

    /// The vertical axis — every row that states a height, a hidden flag or an outline level.
    #[must_use]
    pub fn rows(&self) -> &RowGeometry {
        &self.rows
    }

    /// Every merged region, indexed.
    #[must_use]
    pub fn merges(&self) -> &MergeIndex {
        &self.merges
    }

    /// How the window is divided.
    #[must_use]
    pub fn split(&self) -> &PaneSplit {
        &self.split
    }

    /// The **cached** bounding box `x:dimension` states, or `None` when the sheet writes none.
    ///
    /// Cached, in exactly the sense a formula's `<v>` is: the producer computed it when it saved and
    /// nothing here recomputes it. [`SheetGrid::last_populated_row`] is the answer taken from the
    /// cells themselves, and the two may disagree on a file Excel wrote.
    #[must_use]
    pub fn declared_bounds(&self) -> Option<GridBounds> {
        self.used
    }

    /// The last row the sheet actually writes a `<row>` for, zero-based, or `None` for an empty
    /// sheet.
    ///
    /// Taken from the store rather than from `x:dimension`, and it is the number the scrollable
    /// extent is drawn from: a stale dimension would give a reader a scrollbar that stops above
    /// their data.
    #[must_use]
    pub fn last_populated_row(&self) -> Option<u32> {
        let cells = self.worksheet().sheet_data()?;
        let from_cells = cells
            .rows()
            .filter_map(|row| row.number())
            .filter_map(|number| number.checked_sub(1))
            .max();
        let from_geometry = self.rows.spans().last().map(|span| span.row);
        match (from_cells, from_geometry) {
            (Some(cells), Some(geometry)) => Some(cells.max(geometry)),
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        }
    }

    /// How many rows the sheet is worth scrolling through — one past the last populated one, bounded
    /// by the grid.
    #[must_use]
    pub fn used_row_count(&self) -> u32 {
        self.last_populated_row()
            .map_or(0, |row| row.saturating_add(1))
            .min(ROW_COUNT)
    }

    /// The last column any populated row writes a cell in, zero-based, or `None`.
    #[must_use]
    pub fn last_populated_column(&self) -> Option<u16> {
        let cells = self.worksheet().sheet_data()?;
        cells
            .rows()
            .filter_map(|row| row.cells().last().map(|cell| cell.reference().column()))
            .max()
    }

    /// How many columns the sheet is worth scrolling through.
    #[must_use]
    pub fn used_column_count(&self) -> u32 {
        self.last_populated_column()
            .map_or(0, |column| u32::from(column).saturating_add(1))
            .min(COLUMN_COUNT)
    }

    /// The cell at `row`/`column`, or `None` when the sheet writes none there.
    ///
    /// **A binary search in the packed store**, never a walk: `O(log rows + log cells in the row)`.
    #[must_use]
    pub fn cell(&self, row: u32, column: u16) -> Option<mjx_sml::Cell<'_>> {
        let reference = CellReference::relative(column, row).ok()?;
        self.worksheet().cell(reference)
    }

    /// The row at `row`, or `None` when the sheet writes none.
    #[must_use]
    pub fn row(&self, row: u32) -> Option<mjx_sml::Row<'_>> {
        self.worksheet().sheet_data()?.row(row.checked_add(1)?)
    }

    /// The text a cell displays.
    ///
    /// **No number formatting**: MJXOFF-171 renders a cell's raw stored value, and the `numFmt`
    /// evaluator is MJXOFF-172's whole subject. A numeric cell therefore shows the digits the file
    /// wrote, which is right for a shared string and visibly wrong for a date — deliberately, and
    /// only until R17.
    ///
    /// Follows [`Workbook::cell_text`] exactly rather than re-deriving it: a shared-string index is
    /// resolved through the table, an `<is>` is parsed, and everything else is the `<v>` the file
    /// wrote.
    #[must_use]
    pub fn cell_text(&self, cell: &mjx_sml::Cell<'_>) -> Option<String> {
        if let Some(shared) = cell.shared_string_index() {
            let table = self.shared_strings.as_ref()?;
            let item = table.item(shared)?;
            return item.text().ok().map(std::borrow::Cow::into_owned);
        }
        if let Some(inline) = cell.inline_string_markup() {
            let string = InlineString::parse(inline).ok()?;
            return string.item().text().ok().map(std::borrow::Cow::into_owned);
        }
        cell.value()
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
            .filter(|text| !text.is_empty())
    }

    /// Whether a cell holds anything a reader would see — which is what *"overflow stops at the
    /// first non-empty cell"* actually means.
    ///
    /// A `<c>` with a style and no value is **empty** for this purpose: it is how Excel records a
    /// formatted-but-blank cell, and treating it as a wall would stop every overflow in a sheet
    /// whose whole used range is formatted.
    ///
    /// GUESS: that a formatted-but-valueless cell does not stop an overflow. It is the reading that
    /// makes a real workbook look right; Excel's exact rule for a cell holding only a space, or a
    /// formula whose cached value is the empty string, has not been observed on Windows.
    #[must_use]
    pub fn cell_is_occupied(&self, cell: &mjx_sml::Cell<'_>) -> bool {
        if cell.cell_type() == CellType::InlineString {
            return cell.inline_string_markup().is_some();
        }
        self.cell_text(cell).is_some_and(|text| !text.is_empty())
    }

    /// The shared-string table, or `None` when the workbook has none.
    #[must_use]
    pub fn shared_strings(&self) -> Option<&SharedStringTable> {
        self.shared_strings.as_ref()
    }
}
