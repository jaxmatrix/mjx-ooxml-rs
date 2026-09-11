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
use mjx_sml::NumberFormatLanguage;
use mjx_sml::{
    CellReference, GridBounds, InlineString, SharedStringTable, SheetFormatProperties,
    WorksheetPart,
};
use mjx_xlsx::{DateSystem, SheetDrawing, SheetFormatting, Workbook};

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
    dates: DateSystem,
    language: Option<NumberFormatLanguage>,
    rows: RowGeometry,
    merges: MergeIndex,
    split: PaneSplit,
    used: Option<GridBounds>,
    today: f64,
    drawing: Option<SheetDrawing>,
    print_area: Option<String>,
    print_titles: Option<String>,
    /// The charts anchored on the sheet, by the **anchor index** their drawing gives them, read
    /// through `mjx-layout-chart` (MJXOFF-178).
    ///
    /// Read here, eagerly, for the same reason the drawing part is: it is `O(charts)`, it answers a
    /// question every band asks, and a chart part is not reachable from a `SheetFormatting`.
    charts: Vec<(usize, mjx_layout_chart::ChartModel)>,
    /// The workbook theme's six accents, or the Office defaults when it states none.
    palette: mjx_layout_chart::ChartPalette,
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
        // ⚠ Read from the workbook rather than assumed. The two epochs are 1,462 days apart, so a
        // snapshot that defaulted to 1900 would shift every date in a Macintosh-authored workbook by
        // just over four years — silently, and in the one place a reader would notice at a glance.
        let dates = workbook.date_system()?;
        // The drawing part and the two print names are read here rather than lazily, for the reason
        // every other eager read in this type has: each is `O(stated elements)`, each answers a
        // question a later frame asks per cell, and neither is reachable from a `SheetFormatting`.
        let drawing = workbook.sheet_drawing(index)?;
        let (print_area, print_titles) = print_names(workbook, index);
        Ok(Self::from_parts(index, name, formatting, shared_strings)
            .with_date_system(dates)
            .with_drawing(drawing)
            .with_print_names(print_area, print_titles)
            .with_charts(read_charts(workbook, index)?)
            .with_palette(workbook.theme_accent_colors()?.map_or(
                mjx_layout_chart::ChartPalette::OFFICE,
                mjx_layout_chart::ChartPalette::from_accents,
            )))
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
            dates: DateSystem::Windows1900,
            language: None,
            rows,
            merges,
            split,
            used,
            today: today_serial(DateSystem::Windows1900),
            drawing: None,
            print_area: None,
            print_titles: None,
            charts: Vec::new(),
            palette: mjx_layout_chart::ChartPalette::OFFICE,
        }
    }

    /// The same snapshot counting its date serials from `dates`.
    ///
    /// [`SheetGrid::read`] sets this from `workbookPr@date1904`. A suite that authored a worksheet
    /// in memory gets [`DateSystem::Windows1900`], which is the schema default and what an absent
    /// `workbookPr` means.
    #[must_use]
    pub fn with_date_system(mut self, dates: DateSystem) -> Self {
        self.dates = dates;
        self.today = today_serial(dates);
        self
    }

    /// The same snapshot holding the sheet's drawing part.
    ///
    /// [`SheetGrid::read`] fills this in; a suite that authored a worksheet in memory has no
    /// package to reach a drawing part through and gets `None`.
    #[must_use]
    pub fn with_drawing(mut self, drawing: Option<SheetDrawing>) -> Self {
        self.drawing = drawing;
        self
    }

    /// The same snapshot holding the charts anchored on the sheet, by anchor index.
    ///
    /// [`SheetGrid::read`] fills this in; a suite that authored a worksheet in memory has no package
    /// to reach a chart part through and gets none, which lays a chart frame out as an empty box
    /// exactly as it did before MJXOFF-178.
    #[must_use]
    pub fn with_charts(mut self, charts: Vec<(usize, mjx_layout_chart::ChartModel)>) -> Self {
        self.charts = charts;
        self
    }

    /// The same snapshot handing charts the six accents `palette` names.
    ///
    /// **The workbook's own theme.** A chart whose series state no `c:spPr` — which is what Excel
    /// writes — takes `accent1 … accent6` from here, so a palette invented by this crate would paint
    /// a customer's chart off their own brand.
    #[must_use]
    pub fn with_palette(mut self, palette: mjx_layout_chart::ChartPalette) -> Self {
        self.palette = palette;
        self
    }

    /// The chart anchored at `anchor_index`, or `None` when that anchor frames none.
    #[must_use]
    pub fn chart(&self, anchor_index: usize) -> Option<&mjx_layout_chart::ChartModel> {
        self.charts
            .iter()
            .find(|(index, _)| *index == anchor_index)
            .map(|(_, chart)| chart)
    }

    /// The palette a chart on this sheet draws its unstated series colours from.
    #[must_use]
    pub fn palette(&self) -> &mjx_layout_chart::ChartPalette {
        &self.palette
    }

    /// The same snapshot holding the two print names, as the workbook's `definedNames` spell them.
    ///
    /// They are the workbook part's and not the worksheet's, which is why they arrive as strings
    /// rather than as ranges: `mjx_xlsx::SheetFormatting` holds a worksheet and a stylesheet, and a
    /// defined name is in neither.
    #[must_use]
    pub fn with_print_names(mut self, area: Option<String>, titles: Option<String>) -> Self {
        self.print_area = area;
        self.print_titles = titles;
        self
    }

    /// The same snapshot answering `TODAY()` as `serial`.
    ///
    /// **A `timePeriod` rule is the one conditional format whose answer changes overnight**, and a
    /// suite that asserted one against the system clock would go red on a particular morning. So
    /// the serial is a value the snapshot carries: it defaults to the machine's own clock in this
    /// workbook's epoch, and a caller — a test, or a shell that wants the sheet as it looked
    /// yesterday — says otherwise here.
    #[must_use]
    pub fn with_today(mut self, serial: f64) -> Self {
        self.today = serial;
        self
    }

    /// The serial `TODAY()` answers for this sheet.
    #[must_use]
    pub fn today(&self) -> f64 {
        self.today
    }

    /// The sheet's drawing part, and everything anchored in it.
    #[must_use]
    pub fn drawing(&self) -> Option<&SheetDrawing> {
        self.drawing.as_ref()
    }

    /// `_xlnm.Print_Area`'s definition, as the workbook wrote it.
    #[must_use]
    pub fn print_area(&self) -> Option<&str> {
        self.print_area.as_deref()
    }

    /// `_xlnm.Print_Titles`'s definition.
    #[must_use]
    pub fn print_titles(&self) -> Option<&str> {
        self.print_titles.as_deref()
    }

    /// Which epoch this sheet's date serials count from.
    #[must_use]
    pub fn date_system(&self) -> DateSystem {
        self.dates
    }

    /// The same snapshot resolving §18.8.30's **locale-dependent** built-in ids in `language`.
    ///
    /// Ids 27–36, 50–58 and the Thai block 59–62 / 67–81 have a *different* format code per UI
    /// language — id 30 is `m/d/yy` in `zh-tw`, `m-d-yy` in `zh-cn` and `mm-dd-yy` in `ko-kr` — so a
    /// consumer that does not know its language cannot answer for them at all, and
    /// [`mjx_sml::builtin_format_code`] correctly answers `None` rather than picking one.
    ///
    /// **This is the host's answer and not the document's.** §18.8.30 says the code depends on *"the
    /// consumer's UI language"*; nothing in a `.xlsx` states it, and reading one out of the file
    /// would be inventing it. So the default is `None` — those ids resolve to `General`, which is
    /// visibly wrong and honestly wrong — and a shell that knows what language it is running in says
    /// so here.
    #[must_use]
    pub fn with_number_format_language(mut self, language: Option<NumberFormatLanguage>) -> Self {
        self.language = language;
        self
    }

    /// Which UI language the locale-dependent built-in ids resolve in, or `None`.
    #[must_use]
    pub fn number_format_language(&self) -> Option<NumberFormatLanguage> {
        self.language
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

    /// The text a cell **stores**, before its number format is applied.
    ///
    /// This is the raw value: `45719` for a date, `1234.5` for a currency. The string a reader sees
    /// is [`crate::numfmt`]'s answer for this text and the cell's format code, and the box model
    /// asks for it per cell — see `SheetBoxModel::formats`.
    ///
    /// It stays public and stays raw because *occupancy* is a question about the stored value and
    /// not about the formatted one: [`cell_is_occupied`](Self::cell_is_occupied) reads this, and a
    /// format that renders a value as nothing (`;;;`) must not make a populated cell look empty to
    /// its neighbour's overflow.
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

/// The two built-in names that decide what a sheet prints, scoped to `index`.
///
/// **Consumed, not re-derived.** `mjx_xlsx::Workbook` already resolves a defined name's scope
/// against the tab list and reports which of §18.2.6's eight reserved names it is —
/// `Workbook::print_area` was already there and `print_titles` is its twin, added rather than
/// re-implemented here. A workbook whose names will not read at all prints its used range, which is
/// what a sheet with no names does.
fn print_names(workbook: &Workbook, index: usize) -> (Option<String>, Option<String>) {
    (
        workbook.print_area(index).ok().flatten(),
        workbook.print_titles(index).ok().flatten(),
    )
}

/// The serial number of today in `system`'s epoch.
///
/// DocumentedBehaviour: the Unix epoch is serial 25,569 in the 1900 system and 24,107 in the 1904
/// one, which is the same 1,462-day offset between the two that
/// [`crate::numfmt::datetime`] already carries. A clock that will not read answers serial 0, which
/// renders as `1/0/1900` and is visibly rather than silently wrong.
fn today_serial(system: DateSystem) -> f64 {
    let Ok(since) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return 0.0;
    };
    #[allow(clippy::cast_precision_loss)]
    let days = (since.as_secs() / 86_400) as f64;
    days + match system {
        DateSystem::Windows1900 => 25_569.0,
        DateSystem::Macintosh1904 => 24_107.0,
    }
}

/// Reads every chart anchored on the tab at `index`, by anchor index.
///
/// **Bytes in, model out.** `Workbook::chart_part_bytes` is already public, and `mjx-layout-chart`
/// owns everything from the XML inwards — which is what lets PowerPoint's and Word's box models read
/// the same chart through the same code rather than through three closures.
///
/// A chart part that will not parse is *skipped* rather than failing the sheet: a malformed chart is
/// one anchor drawn empty, and refusing the whole worksheet over it would lose every cell on it.
fn read_charts(
    workbook: &Workbook,
    index: usize,
) -> Result<Vec<(usize, mjx_layout_chart::ChartModel)>, SheetLayoutError> {
    let anchors = workbook.chart_anchor_indices(index)?;
    let mut charts = Vec::with_capacity(anchors.len());
    for anchor in anchors {
        let Some(bytes) = workbook.chart_part_bytes(index, anchor)? else {
            continue;
        };
        if let Ok(model) = mjx_layout_chart::ChartModel::read(&bytes) {
            charts.push((anchor, model));
        }
    }
    Ok(charts)
}
