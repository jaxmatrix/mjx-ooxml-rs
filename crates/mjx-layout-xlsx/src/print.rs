//! Print layout: the **second** pagination discipline over the same sheet.
//!
//! # Two paginations, and they do not agree
//!
//! R16 gave this crate one already: a *band* is `Constraints::content.height()` tall, bands tile
//! uniformly in sheet space, and page **n** never needs page **n−1**'s checkpoint — which is what
//! makes `BoxModel::layout_page` resumable by construction. That is the **viewport**: what a reader
//! scrolls through.
//!
//! This is the other one. A printed page is bounded on **both** axes, its size comes from
//! `pageSetup` rather than from the caller, a manual `brk` forces a boundary wherever a person put
//! one, `fitToWidth` changes the scale until the columns fit, and rows and columns repeat at the top
//! and left of every page. The two share the grid and nothing else, and this module is deliberately
//! **not** an implementation of `BoxModel`: a `Checkpoint` that meant *page 4 of the printout* and a
//! `Checkpoint` that meant *the fourth band of the window* would be two meanings of one type, and
//! the contract has one signature for both.
//!
//! # ⚠ The provenance of every number here
//!
//! Nobody has run Excel. Following MJXOFF-172's example, each value this module chooses is labelled
//! where it is chosen:
//!
//! * **SpecCode** — the number is ECMA-376's. The paper-size table is §18.3.1.63's own enumeration;
//!   the schema defaults (`@scale` 100, `@fitToWidth`/`@fitToHeight` 1, `@paperSize` 1,
//!   `@firstPageNumber` 1, `@orientation` `default`) are the XSD's.
//! * **DocumentedBehaviour** — the behaviour has an external, checkable definition. `@fitToWidth`
//!   and `@fitToHeight` are *"the number of horizontal/vertical pages to fit on"*; `pageMargins` is
//!   in inches; `brk@id` is the row or column the break falls **before**; `pageOrder` names the
//!   traversal.
//! * **EngineDerived** — a change detector and **not evidence about Excel**. Every rounding
//!   decision, the treatment of the header and footer margins, and what a scale below the fit
//!   does to a single oversized column are all this.
//!
//! There is nothing here in the fourth category — a number transcribed from a Windows sitting —
//! because no such sitting has happened.

use std::ops::Range;

use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::spreadsheetml::{PageOrder, PrintOrientation};
use mjx_sml::{GridBounds, WorksheetPart};

use crate::geometry::{GridGeometry, COLUMN_COUNT, ROW_COUNT};
use crate::sheet::SheetGrid;

/// One page's four margins, in EMU.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PrintMargins {
    /// `pageMargins@left`.
    pub left: Emu,
    /// `pageMargins@right`.
    pub right: Emu,
    /// `pageMargins@top`.
    pub top: Emu,
    /// `pageMargins@bottom`.
    pub bottom: Emu,
    /// `pageMargins@header` — the distance from the top of the paper to the header.
    pub header: Emu,
    /// `pageMargins@footer`.
    pub footer: Emu,
}

impl Default for PrintMargins {
    /// Excel's own *Normal* margins, which are what a sheet writing no `pageMargins` prints with.
    ///
    /// EngineDerived: 0.7 inch at the sides, 0.75 top and bottom, 0.3 for the header and footer.
    /// Those are the numbers Excel's *Page Layout* ribbon calls *Normal*; the schema states no
    /// default at all, because all six attributes are `use="required"`.
    fn default() -> Self {
        Self {
            left: Emu::from_inches(0.7),
            right: Emu::from_inches(0.7),
            top: Emu::from_inches(0.75),
            bottom: Emu::from_inches(0.75),
            header: Emu::from_inches(0.3),
            footer: Emu::from_inches(0.3),
        }
    }
}

/// One printable page: which rows and columns fall on it, and what repeats at its edges.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PrintPage {
    /// The page number a header's `&P` would print, counting from `pageSetup@firstPageNumber`.
    pub number: u32,
    /// The rows of the sheet's own data on this page, zero-based, hidden rows excluded.
    pub rows: Range<u32>,
    /// Its columns.
    pub columns: Range<u16>,
    /// The rows `_xlnm.Print_Titles` repeats at the top of **every** page, if any.
    pub repeated_rows: Option<Range<u32>>,
    /// The columns it repeats at the left.
    pub repeated_columns: Option<Range<u16>>,
}

impl PrintPage {
    /// Whether a cell falls on this page — in its data, or in a repeated title.
    #[must_use]
    pub fn contains(&self, row: u32, column: u16) -> bool {
        let in_rows = self.rows.contains(&row)
            || self
                .repeated_rows
                .as_ref()
                .is_some_and(|rows| rows.contains(&row));
        let in_columns = self.columns.contains(&column)
            || self
                .repeated_columns
                .as_ref()
                .is_some_and(|columns| columns.contains(&column));
        in_rows && in_columns
    }
}

/// Everything a sheet states about printing, read once.
#[derive(Clone, PartialEq, Debug)]
pub struct PrintSetup {
    /// The paper, **after** `@orientation` has been applied: width first.
    pub paper: (Emu, Emu),
    /// The margins.
    pub margins: PrintMargins,
    /// `pageSetup@scale`, as a multiplier. SpecCode: the schema default is 100.
    pub scale: f64,
    /// `pageSetup@fitToWidth` — how many pages wide the sheet is squeezed into, or `None` when it
    /// is not squeezed.
    pub pages_wide: Option<u32>,
    /// `pageSetup@fitToHeight`.
    pub pages_tall: Option<u32>,
    /// `pageSetup@pageOrder`. SpecCode: the schema default is `downThenOver`.
    pub order: PageOrder,
    /// `pageSetup@firstPageNumber`, honoured only when `@useFirstPageNumber` is set.
    pub first_page_number: u32,
    /// `printOptions@gridLines`.
    pub prints_grid_lines: bool,
    /// `printOptions@headings` — the `1 2 3` / `A B C` rulers.
    pub prints_headings: bool,
    /// `printOptions@horizontalCentered`.
    pub centred_horizontally: bool,
    /// `printOptions@verticalCentered`.
    pub centred_vertically: bool,
    /// `_xlnm.Print_Area`, or the whole used range when the sheet defines none.
    pub area: Vec<GridBounds>,
    /// The rows `_xlnm.Print_Titles` repeats.
    pub repeated_rows: Option<Range<u32>>,
    /// The columns it repeats.
    pub repeated_columns: Option<Range<u16>>,
    /// Every **manual** `rowBreaks/brk@id`, zero-based: the row each break falls before.
    pub row_breaks: Vec<u32>,
    /// Every manual `colBreaks/brk@id`, zero-based.
    pub column_breaks: Vec<u16>,
}

impl PrintSetup {
    /// Reads `grid`'s print settings.
    #[must_use]
    pub fn read(grid: &SheetGrid) -> Self {
        let sheet = grid.worksheet();
        let interner = sheet.interner();
        let margins = read_margins(sheet);
        let (paper, scale, pages_wide, pages_tall, order, first_page_number) =
            read_page_setup(sheet);
        let options = sheet.print_options();
        let area = grid
            .print_area()
            .map(parse_ranges)
            .filter(|ranges: &Vec<GridBounds>| !ranges.is_empty())
            .unwrap_or_else(|| grid.declared_bounds().into_iter().collect());
        let (repeated_rows, repeated_columns) =
            grid.print_titles().map_or((None, None), parse_titles);
        Self {
            paper,
            margins,
            scale,
            pages_wide,
            pages_tall,
            order,
            first_page_number,
            prints_grid_lines: options
                .is_some_and(|options| options.prints_grid_lines(interner).unwrap_or(false)),
            prints_headings: options.is_some_and(|options| {
                options
                    .prints_row_and_column_headings(interner)
                    .unwrap_or(false)
            }),
            centred_horizontally: options
                .is_some_and(|options| options.centred_horizontally(interner).unwrap_or(false)),
            centred_vertically: options
                .is_some_and(|options| options.centred_vertically(interner).unwrap_or(false)),
            area,
            repeated_rows,
            repeated_columns,
            row_breaks: manual_breaks(sheet.row_breaks(), interner),
            column_breaks: manual_breaks(sheet.column_breaks(), interner)
                .into_iter()
                .map(|at| u16::try_from(at).unwrap_or(u16::MAX))
                .collect(),
        }
    }

    /// The area inside the margins, in EMU.
    ///
    /// EngineDerived: the header and footer margins are **not** subtracted. `@header` and `@footer`
    /// are distances from the edge of the paper to those bands, and Excel prints a header inside the
    /// top margin rather than above the body — so subtracting both would shrink every page by an
    /// inch and a half for a sheet with neither.
    #[must_use]
    pub fn printable(&self) -> (Emu, Emu) {
        (
            (self.paper.0 - self.margins.left - self.margins.right).max(Emu::from_emu(1)),
            (self.paper.1 - self.margins.top - self.margins.bottom).max(Emu::from_emu(1)),
        )
    }
}

/// A sheet, paginated for printing.
#[derive(Clone, PartialEq, Debug)]
pub struct PrintPagination {
    /// Every page, in the order `pageOrder` prints them.
    pub pages: Vec<PrintPage>,
    /// The scale actually applied — `pageSetup@scale`, or the one `fitToWidth`/`fitToHeight` forced.
    pub scale: f64,
    /// Whether that scale came from a fit rather than from `@scale`.
    pub scaled_to_fit: bool,
    /// The area inside the margins, in EMU.
    pub printable: (Emu, Emu),
}

impl PrintPagination {
    /// How many pages the sheet prints on.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pages.len()
    }

    /// Whether it prints on none — a sheet with no used range and no print area.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    /// The page a cell falls on, or `None` when it is outside the print area.
    #[must_use]
    pub fn page_of(&self, row: u32, column: u16) -> Option<&PrintPage> {
        self.pages
            .iter()
            .find(|page| page.rows.contains(&row) && page.columns.contains(&column))
    }
}

/// Paginates `grid` for printing.
///
/// **Two passes and no search.** The columns are walked once to find the vertical boundaries and the
/// rows once to find the horizontal ones, and the pages are their product in `pageOrder`'s
/// traversal — which is what makes the cost the number of *printed* rows and columns rather than the
/// number of pages. A fit-to-page scale is solved in closed form from the same two walks, because a
/// scale is a multiplier on every width at once.
#[must_use]
pub fn paginate(grid: &SheetGrid, geometry: &GridGeometry, setup: &PrintSetup) -> PrintPagination {
    let printable = setup.printable();
    let Some(area) = envelope(&setup.area) else {
        return PrintPagination {
            pages: Vec::new(),
            scale: setup.scale,
            scaled_to_fit: false,
            printable,
        };
    };
    let _ = grid;

    let columns = visible_columns(geometry, area.2, area.3);
    let rows = visible_rows(geometry, area.0, area.1);
    let title_width: Emu = setup
        .repeated_columns
        .clone()
        .map(|range| {
            range
                .filter(|column| !geometry.columns().is_hidden(*column))
                .map(|column| geometry.columns().width(column))
                .fold(Emu::ZERO, |total, width| total + width)
        })
        .unwrap_or(Emu::ZERO);
    let title_height: Emu = setup
        .repeated_rows
        .clone()
        .map(|range| {
            range
                .filter(|row| !geometry.rows().is_hidden(*row))
                .map(|row| geometry.rows().height(row))
                .fold(Emu::ZERO, |total, height| total + height)
        })
        .unwrap_or(Emu::ZERO);

    let total_width = columns
        .iter()
        .map(|column| geometry.columns().width(*column))
        .fold(Emu::ZERO, |total, width| total + width);
    let total_height = rows
        .iter()
        .map(|row| geometry.rows().height(*row))
        .fold(Emu::ZERO, |total, height| total + height);

    let (scale, scaled_to_fit) = resolve_scale(
        setup,
        printable,
        (total_width + title_width, total_height + title_height),
    );

    let column_bands: Vec<Range<u16>> = split(
        &columns,
        |column| geometry.columns().width(*column),
        scale,
        printable.0 - scaled(title_width, scale),
        |column| setup.column_breaks.contains(column),
    )
    .into_iter()
    .map(|(first, last)| first..last.saturating_add(1))
    .collect();
    let row_bands: Vec<Range<u32>> = split(
        &rows,
        |row| geometry.rows().height(*row),
        scale,
        printable.1 - scaled(title_height, scale),
        |row| setup.row_breaks.contains(row),
    )
    .into_iter()
    .map(|(first, last)| first..last.saturating_add(1))
    .collect();

    let mut pages = Vec::with_capacity(column_bands.len().saturating_mul(row_bands.len()));
    let mut number = setup.first_page_number.max(1);
    // DocumentedBehaviour: `downThenOver` walks a column of pages to the bottom before moving right,
    // and `overThenDown` walks a row of pages to the right before moving down. §18.3.1.63 names both.
    let ordered: Vec<(&Range<u16>, &Range<u32>)> = match setup.order {
        PageOrder::DownThenOver => column_bands
            .iter()
            .flat_map(|columns| row_bands.iter().map(move |rows| (columns, rows)))
            .collect(),
        PageOrder::OverThenDown => row_bands
            .iter()
            .flat_map(|rows| column_bands.iter().map(move |columns| (columns, rows)))
            .collect(),
    };
    for (columns, rows) in ordered {
        pages.push(PrintPage {
            number,
            rows: rows.clone(),
            columns: columns.clone(),
            repeated_rows: setup
                .repeated_rows
                .clone()
                .filter(|titles| !titles.contains(&rows.start)),
            repeated_columns: setup
                .repeated_columns
                .clone()
                .filter(|titles| !titles.contains(&columns.start)),
        });
        number = number.saturating_add(1);
    }
    PrintPagination {
        pages,
        scale,
        scaled_to_fit,
        printable,
    }
}

/// The scale to print at, and whether a fit chose it.
///
/// DocumentedBehaviour: `@fitToWidth` and `@fitToHeight` are *"the number of horizontal/vertical
/// pages to fit on"*, so the scale is whichever of the two ratios is smaller — a sheet squeezed to
/// one page wide and one page tall shrinks until **both** hold.
///
/// EngineDerived: the result is clamped to `0.10..=4.00` — Excel's own dialogue accepts 10% to 400%
/// — and a fit that would enlarge the sheet does not, because `Fit to 1 page wide` never blows a
/// narrow sheet up in Excel either.
fn resolve_scale(setup: &PrintSetup, printable: (Emu, Emu), total: (Emu, Emu)) -> (f64, bool) {
    let Some(fit) = setup.pages_wide.or(setup.pages_tall) else {
        return (setup.scale, false);
    };
    let _ = fit;
    let ratio = |available: Emu, needed: Emu, pages: Option<u32>| -> Option<f64> {
        let pages = pages?;
        if pages == 0 || needed.emu() <= 0 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        let across = available.emu() as f64 * f64::from(pages);
        #[allow(clippy::cast_precision_loss)]
        Some(across / needed.emu() as f64)
    };
    let horizontal = ratio(printable.0, total.0, setup.pages_wide);
    let vertical = ratio(printable.1, total.1, setup.pages_tall);
    let chosen = match (horizontal, vertical) {
        (Some(across), Some(down)) => across.min(down),
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => return (setup.scale, false),
    };
    // ⚠ **Floored to a whole percentage**, and this is not cosmetic. The exact ratio makes the
    // content *precisely* as wide as the page, so the last column tips over the accumulator by a
    // rounding EMU and a fit-to-one-page sheet paginates onto two — which is what this engine did
    // before the floor, and what a reader would report as a bug in the fit.
    //
    // It is also the more faithful answer: Excel's own *Page Setup* dialogue states the scale as a
    // whole number, and the file writes `@scale` as an `xsd:unsignedInt`, so a fit that produced
    // 63.7% could not be saved as itself. GUESS: that Excel floors rather than rounds.
    let whole_percent = (chosen.min(1.0) * 100.0).floor() / 100.0;
    (whole_percent.clamp(0.10, 4.00), true)
}

/// `length` at `scale`.
fn scaled(length: Emu, scale: f64) -> Emu {
    #[allow(clippy::cast_precision_loss)]
    Emu::from_emu_rounded(length.emu() as f64 * scale)
}

/// Cuts a list of rows or columns into pages.
///
/// A manual break is honoured **before** the accumulator, so a break two columns in makes a page of
/// two columns even where twenty would have fitted; and a single item wider than the whole page gets
/// a page to itself rather than an empty one and then an overflowing one.
fn split<T: Copy + Ord>(
    items: &[T],
    size: impl Fn(&T) -> Emu,
    scale: f64,
    available: Emu,
    is_break: impl Fn(&T) -> bool,
) -> Vec<(T, T)> {
    let mut bands = Vec::new();
    let mut start: Option<T> = None;
    let mut last: Option<T> = None;
    let mut used = Emu::ZERO;
    for item in items {
        let extent = scaled(size(item), scale);
        let forced = is_break(item) && start.is_some();
        let overflows = start.is_some() && used + extent > available;
        if forced || overflows {
            if let (Some(first), Some(previous)) = (start, last) {
                bands.push((first, previous));
            }
            start = None;
            used = Emu::ZERO;
        }
        if start.is_none() {
            start = Some(*item);
        }
        used += extent;
        last = Some(*item);
    }
    if let (Some(first), Some(previous)) = (start, last) {
        bands.push((first, previous));
    }
    bands
}

/// The smallest span containing every range of the print area: `(first row, last row, first column,
/// last column)`, all inclusive.
///
/// Four numbers rather than a [`GridBounds`], because that type is built by normalising a
/// [`CellRange`](mjx_sml::CellRange) and has no constructor of its own — the right shape for an
/// address and the wrong one for an accumulator.
///
/// A **multi-range print area is printed as its bounding box.** GUESS: Excel prints each area as
/// its own set of pages, so a sheet whose print area is `A1:B2,D4:E5` prints two pages there and one
/// here. Reported rather than hidden; the bounding box is the answer that never omits a cell the
/// person asked for.
fn envelope(area: &[GridBounds]) -> Option<(u32, u32, u16, u16)> {
    let first = area.first()?;
    Some(area.iter().skip(1).fold(
        (
            first.first_row(),
            first.last_row(),
            first.first_column(),
            first.last_column(),
        ),
        |wide, next| {
            (
                wide.0.min(next.first_row()),
                wide.1.max(next.last_row()),
                wide.2.min(next.first_column()),
                wide.3.max(next.last_column()),
            )
        },
    ))
}

/// The visible columns of an inclusive span.
fn visible_columns(geometry: &GridGeometry, first: u16, last: u16) -> Vec<u16> {
    let mut out = Vec::new();
    let mut column = first;
    while let Some(visible) = geometry.columns().next_visible_column(column) {
        if visible > last {
            break;
        }
        out.push(visible);
        let Some(next) = visible.checked_add(1) else {
            break;
        };
        column = next;
    }
    out
}

/// The visible rows of an inclusive span.
fn visible_rows(geometry: &GridGeometry, first: u32, last: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut row = first;
    while let Some(visible) = geometry.rows().next_visible_row(row) {
        if visible > last {
            break;
        }
        out.push(visible);
        let Some(next) = visible.checked_add(1) else {
            break;
        };
        row = next;
    }
    out
}

/// `pageMargins`, or Excel's *Normal* margins for a sheet that writes none.
fn read_margins(sheet: &WorksheetPart) -> PrintMargins {
    let Some(margins) = sheet.page_margins() else {
        return PrintMargins::default();
    };
    let interner = sheet.interner();
    let default = PrintMargins::default();
    let inches = |value: Result<f64, mjx_ooxml_core::AttributeError>, fallback: Emu| {
        value.map_or(fallback, Emu::from_inches)
    };
    PrintMargins {
        left: inches(margins.left_inches(interner), default.left),
        right: inches(margins.right_inches(interner), default.right),
        top: inches(margins.top_inches(interner), default.top),
        bottom: inches(margins.bottom_inches(interner), default.bottom),
        header: inches(margins.header_inches(interner), default.header),
        footer: inches(margins.footer_inches(interner), default.footer),
    }
}

/// `pageSetup`, with the schema's own defaults for everything it does not state.
fn read_page_setup(
    sheet: &WorksheetPart,
) -> ((Emu, Emu), f64, Option<u32>, Option<u32>, PageOrder, u32) {
    let interner = sheet.interner();
    let Some(setup) = sheet.page_setup() else {
        return (
            paper_of(1, PrintOrientation::Portrait),
            1.0,
            None,
            None,
            PageOrder::DownThenOver,
            1,
        );
    };
    let orientation = setup
        .orientation(interner)
        .unwrap_or(PrintOrientation::Default);
    let index = setup.paper_size_index(interner).unwrap_or(1);
    let stated = setup
        .paper_width(interner)
        .ok()
        .flatten()
        .and_then(|width| universal_measure(&width))
        .zip(
            setup
                .paper_height(interner)
                .ok()
                .flatten()
                .and_then(|height| universal_measure(&height)),
        );
    // DocumentedBehaviour: §18.3.1.63 says `@paperWidth`/`@paperHeight` *"override @paperSize"*.
    let paper = match stated {
        Some((width, height)) => orient((width, height), orientation),
        None => paper_of(index, orientation),
    };
    let pages_wide = setup.pages_wide(interner).unwrap_or(1);
    let pages_tall = setup.pages_tall(interner).unwrap_or(1);
    // GUESS: that a `fitToWidth`/`fitToHeight` other than the schema default of 1 means the sheet is
    // fitted, and that both at 1 means it is not. The real switch is
    // `sheetPr/pageSetUpPr@fitToPage`, which is a *different element* and which Excel does write —
    // but a file that states `fitToWidth="2"` and no `pageSetUpPr` is asking for two pages wide by
    // any reading, and honouring the attribute the file wrote is the answer that cannot lose a
    // person's setting.
    let fitted = pages_wide != 1 || pages_tall != 1;
    #[allow(clippy::cast_lossless)]
    let scale = f64::from(setup.scale_percentage(interner).unwrap_or(100)) / 100.0;
    (
        paper,
        scale,
        fitted.then_some(pages_wide).filter(|pages| *pages > 0),
        fitted.then_some(pages_tall).filter(|pages| *pages > 0),
        setup
            .page_order(interner)
            .unwrap_or(PageOrder::DownThenOver),
        if setup.uses_first_page_number(interner).unwrap_or(false) {
            setup.first_page_number(interner).unwrap_or(1)
        } else {
            1
        },
    )
}

/// Turns a portrait pair into the orientation the sheet asked for.
///
/// SpecCode: `ST_Orientation`'s third member is `default`, which means *the printer's*. There is no
/// printer here, so it is read as portrait — the same answer as an absent `pageSetup`.
fn orient(portrait: (Emu, Emu), orientation: PrintOrientation) -> (Emu, Emu) {
    match orientation {
        PrintOrientation::Landscape => (portrait.1, portrait.0),
        PrintOrientation::Portrait | PrintOrientation::Default => portrait,
    }
}

/// The paper `index` names, in portrait, then oriented.
///
/// SpecCode: the codes and their names are §18.3.1.63's own enumeration. The **dimensions** are the
/// standard ones for those names (ISO 216 for the A series, ANSI for Letter and its relatives), and
/// are DocumentedBehaviour rather than SpecCode — the specification lists the names and not the
/// measurements.
///
/// An unlisted code is Letter, which is the schema's own default for `@paperSize`.
fn paper_of(index: u32, orientation: PrintOrientation) -> (Emu, Emu) {
    let millimetres = |width: f64, height: f64| {
        (
            Emu::from_inches(width / 25.4),
            Emu::from_inches(height / 25.4),
        )
    };
    let inches = |width: f64, height: f64| (Emu::from_inches(width), Emu::from_inches(height));
    let portrait = match index {
        1 | 2 => inches(8.5, 11.0),          // Letter, Letter Small
        3 => inches(11.0, 17.0),             // Tabloid
        4 => inches(17.0, 11.0),             // Ledger
        5 => inches(8.5, 14.0),              // Legal
        6 => inches(5.5, 8.5),               // Statement
        7 => inches(7.25, 10.5),             // Executive
        8 => millimetres(297.0, 420.0),      // A3
        9 | 10 => millimetres(210.0, 297.0), // A4, A4 Small
        11 => millimetres(148.0, 210.0),     // A5
        12 => millimetres(250.0, 353.0),     // B4 (JIS)
        13 => millimetres(176.0, 250.0),     // B5 (JIS)
        14 => inches(8.5, 13.0),             // Folio
        15 => millimetres(215.0, 275.0),     // Quarto
        16 => inches(10.0, 14.0),
        17 => inches(11.0, 17.0),
        18 => inches(8.5, 11.0),         // Note
        20 => inches(4.125, 9.5),        // Envelope #10
        27 => millimetres(110.0, 220.0), // Envelope DL
        28 => millimetres(162.0, 229.0), // Envelope C5
        43 => millimetres(100.0, 148.0), // Japanese postcard
        70 => millimetres(210.0, 297.0), // A4 Extra
        _ => inches(8.5, 11.0),
    };
    orient(portrait, orientation)
}

/// Reads an `ST_PositiveUniversalMeasure` — a number and one of six unit suffixes.
///
/// SpecCode: the six units are the simple type's own (`mm`, `cm`, `in`, `pt`, `pc`, `pi`).
fn universal_measure(text: &str) -> Option<Emu> {
    let text = text.trim();
    let (number, unit) = text.split_at(text.len().checked_sub(2)?);
    let value: f64 = number.trim().parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(match unit {
        "mm" => Emu::from_inches(value / 25.4),
        "cm" => Emu::from_inches(value / 2.54),
        "in" => Emu::from_inches(value),
        "pt" => Emu::from_points(value),
        // A pica and a pitch are both twelve points.
        "pc" | "pi" => Emu::from_points(value * 12.0),
        _ => return None,
    })
}

/// The zero-based rows or columns a `brk` list marks, **manual breaks only**.
///
/// DocumentedBehaviour: `brk@id` is *"the row or column the break falls before"*, one-based on the
/// wire like every other row and column number in `sml.xsd`. An automatic break is not read: it is a
/// record of where the *producer's* pagination fell, and this module computes its own — carrying
/// both would be two answers to one question, and the file's would go stale the moment a column
/// width changed.
fn manual_breaks(
    breaks: Option<&mjx_sml::PageBreaks>,
    interner: &mjx_ooxml_core::Interner,
) -> Vec<u32> {
    let Some(breaks) = breaks else {
        return Vec::new();
    };
    let mut out: Vec<u32> = breaks
        .breaks()
        .filter(|entry| entry.is_manual(interner).unwrap_or(false))
        .filter_map(|entry| entry.at(interner).ok())
        .filter_map(|at| at.checked_sub(1))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Parses a `_xlnm.Print_Area` definition into ranges.
///
/// The definition is a formula — `Sheet1!$A$1:$D$20,Sheet1!$F$1:$H$9` — and this reads exactly the
/// shape a print area takes and nothing else. Anything it cannot read is dropped rather than
/// guessed at, which turns a defined name it does not understand into *print the used range* rather
/// than into a wrong page count.
#[must_use]
pub fn parse_ranges(definition: &str) -> Vec<GridBounds> {
    definition
        .split(',')
        .filter_map(|token| {
            let token = strip_sheet(token);
            mjx_sml::CellRange::parse(token)
                .ok()
                .map(|range| range.normalized_bounds())
        })
        .collect()
}

/// Parses a `_xlnm.Print_Titles` definition into the rows and the columns it repeats.
///
/// `Sheet1!$1:$3` is three rows; `Sheet1!$A:$B` is two columns; `Sheet1!$1:$3,Sheet1!$A:$B` is both.
/// The two are told apart by shape, which is what makes them unambiguous: a row range has digits on
/// both sides of the colon and a column range has letters.
#[must_use]
pub fn parse_titles(definition: &str) -> (Option<Range<u32>>, Option<Range<u16>>) {
    let mut rows = None;
    let mut columns = None;
    for token in definition.split(',') {
        let token = strip_sheet(token);
        let Some((first, last)) = token.split_once(':') else {
            continue;
        };
        let (first, last) = (first.replace('$', ""), last.replace('$', ""));
        if let (Ok(first), Ok(last)) = (first.parse::<u32>(), last.parse::<u32>()) {
            let (first, last) = (first.min(last), first.max(last));
            let start = first.saturating_sub(1);
            rows = Some(start..last.min(ROW_COUNT));
            continue;
        }
        if let (Some(first), Some(last)) = (column_index(&first), column_index(&last)) {
            let (first, last) = (first.min(last), first.max(last));
            let end = u32::from(last).saturating_add(1).min(COLUMN_COUNT);
            columns = Some(first..u16::try_from(end).unwrap_or(u16::MAX));
        }
    }
    (rows, columns)
}

/// Drops a `Sheet1!` prefix, quoted or not.
fn strip_sheet(token: &str) -> &str {
    let token = token.trim();
    match token.rsplit_once('!') {
        Some((_, range)) => range,
        None => token,
    }
}

/// `A` is 0, `AA` is 26.
fn column_index(letters: &str) -> Option<u16> {
    let letters = letters.trim();
    if letters.is_empty() || !letters.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    let mut index: u32 = 0;
    for byte in letters.bytes() {
        let digit = u32::from(byte.to_ascii_uppercase() - b'A') + 1;
        index = index.checked_mul(26)?.checked_add(digit)?;
    }
    u16::try_from(index.checked_sub(1)?).ok()
}
