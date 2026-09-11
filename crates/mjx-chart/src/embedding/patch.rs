//! Writing a chart's data into the workbook it **already** embeds — the cells its own `c:f`
//! formulas name, and not one byte more (MJXOFF-208).
//!
//! # The rule this module exists to keep
//!
//! A chart's embedded workbook is a part **the producer wrote**. Until MJXOFF-208 a data edit threw
//! it away: [`embedded_workbook_for_chart_space`](super::embedded_workbook_for_chart_space) built a
//! fresh one-sheet package and the host crate wrote that over the part, so every extra sheet, cell
//! format, defined name, macro and document property the workbook carried was gone — silently, from
//! a call that only said *set this series to these numbers*. The behaviour was documented, which
//! made it honest; it was also the **default**, which made it wrong. `CLAUDE.md`'s standing design
//! rule is *supply a default only in the absence of the user's own, never in place of it*, and a
//! file opened from disk keeps what it came with.
//!
//! So this module does the other thing. It reads the `c:f` beside each cache, resolves it to cells,
//! and writes the chart's numbers into **those cells**. Everything else in the package — every other
//! sheet, every style, the shared-string table, `docProps`, whatever the producer put there — comes
//! back byte for byte, because it is never touched: [`mjx_opc::Package`] re-emits an unedited part
//! from the buffer it was read from, and [`mjx_sml::WorksheetPart`] rewrites only the row a cell
//! lands in.
//!
//! # Patch, or refuse. Never guess, and never regenerate behind the caller's back
//!
//! The tempting third option — *look at the workbook, decide whether it seems producer-written, and
//! regenerate when it does not* — is a heuristic, and a heuristic is wrong on somebody's file
//! silently and in the destructive direction. There is no such branch here. Either the reference
//! resolves to cells this library can write, and they are written, or the patch **refuses** with
//! [`ChartAccessError::EmbeddedWorkbookNotWritable`] naming the `c:f` and saying what about it could
//! not be resolved. [`ReferenceProblem`] is the whole list, and every entry of it is a *shape of
//! reference*, decided from the text the producer wrote — never from a guess about provenance.
//!
//! A caller who would genuinely rather have a fresh workbook than the producer's one says so, with
//! the `regenerate_chart_workbook` method each host crate exposes. That is the escape hatch, and it
//! is now the thing a caller has to ask for rather than the thing that happens to them.
//!
//! # A cell that already says the right thing is not written
//!
//! [`apply_workbook_patch`] compares before it writes, and answers `Ok(None)` — *leave the part
//! alone* — when every planned cell already holds the value it was going to be given. That is not
//! the same answer as *there is no workbook*, and the host crates keep the two apart: a refresh over
//! a workbook that already agrees still reports that it found one. Two things turn on it, and
//! neither is cosmetic:
//!
//! * **A refresh over a file that already agrees writes nothing at all.** Re-saving a package
//!   rewrites its ZIP container even when every part inside is identical, so the host's own
//!   byte-identity guarantee for that part survives only if the write is skipped entirely.
//! * **A shared string stays shared.** New text is written as an inline string, because interning it
//!   would mean editing `xl/sharedStrings.xml` as well — see [`PlannedValue::Text`]. Comparing first
//!   means only the labels that actually changed convert, instead of every label a refresh touches.
//!
//! # Two functions, because the interner and the package cannot be borrowed at once
//!
//! Planning reads the chart, which lives behind the host package's part tree; applying reads the
//! embedded workbook, which lives in the same package. A host cannot hold both borrows, so the split
//! is not a stylistic one: [`plan_workbook_patch`] finishes with the part tree, and
//! [`apply_workbook_patch`] starts with the bytes. It also gives a data edit its all-or-nothing
//! shape — the half that can refuse runs before the chart part is touched, and the half that writes
//! runs after.

use std::borrow::Cow;

use mjx_ooxml_core::Interner;
use mjx_opc::{OpcError, Package, PartName, TargetMode};
use mjx_sml::write::constants::{REL_OFFICE_DOCUMENT, REL_SHARED_STRINGS, REL_WORKSHEET};
use mjx_sml::{
    AddressError, Anchoring, Cell, CellRange, CellReference, CellValue, InlineString,
    ReferenceAreas, SharedStringTable, SheetQualifiedReference, SmlError, WorkbookPart,
    WorksheetPart,
};

use crate::data::{
    CategoryData, DataPoint, Formula, NumberReference, NumericData, SeriesText, StringReference,
};
use crate::ops::{series_at, ChartAccessError};
use crate::plot::Series;
use crate::space::ChartSpace;

// =================================================================================================
// The public surface
// =================================================================================================

/// Which of a chart's references [`plan_workbook_patch`] writes, and what it writes into them.
///
/// The three shapes are the three callers. A data edit knows exactly one series and exactly one kind
/// of data, so it names them and nothing else is looked at — a second series whose `c:f` this
/// library cannot resolve must not make an unrelated edit fail. `refresh_chart_workbook` is the
/// caller that deliberately asks about everything.
#[derive(Debug, Clone, Copy)]
pub enum WorkbookPatch<'a> {
    /// Every reference the chart names — each series' name, its categories and its values (and a
    /// bubble series' sizes) — set to the caches the chart currently holds.
    ///
    /// This is what *refresh* means: make the workbook say what the chart draws.
    EveryReference,

    /// The values of the series at `series_idx` (0-based across the chart's plots), set to `values`.
    SeriesValues {
        /// The series to write, numbered as [`crate::chart_ops::series`] numbers them.
        series_idx: usize,
        /// The numbers to put in the cells the series' `c:val`/`c:yVal` names.
        values: &'a [f64],
    },

    /// The category labels of the series at `series_idx`, set to `labels`.
    SeriesCategories {
        /// The series to write, numbered as [`crate::chart_ops::series`] numbers them.
        series_idx: usize,
        /// The text to put in the cells the series' `c:cat`/`c:xVal` names.
        labels: &'a [&'a str],
    },
}

/// What went wrong while writing a chart's data into the workbook it embeds.
///
/// Two halves that stay apart on purpose. [`Access`](Self::Access) is a verdict about the **chart** —
/// a `c:f` that names no cells this library can write — and each host crate already lifts
/// [`ChartAccessError`] into its own error type through an exhaustive `match`. [`Sml`](Self::Sml) is
/// the embedded package refusing to be read or written, which is the same class of failure as any
/// other malformed part.
#[derive(Debug, thiserror::Error)]
pub enum ChartWorkbookError {
    /// The chart itself is why the workbook cannot be written.
    #[error(transparent)]
    Access(#[from] ChartAccessError),

    /// The embedded workbook package could not be read, edited or written.
    #[error(transparent)]
    Sml(#[from] SmlError),
}

impl From<OpcError> for ChartWorkbookError {
    fn from(error: OpcError) -> Self {
        Self::Sml(SmlError::Opc(error))
    }
}

/// Why a `c:f` could not be resolved to cells this library is willing to write.
///
/// Every entry is a property of the **reference text** or of the workbook it names, decided from
/// what the producer wrote. None of them is a guess about where the workbook came from, and none of
/// them has a fallback that writes something else instead — that is the whole point of the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceProblem {
    /// The text is not a cell reference at all — a defined name, a function call, or markup this
    /// library does not parse. Resolving it would need a formula evaluator, which this project
    /// states as a permanent non-goal.
    NotACellReference,

    /// The reference carries no sheet qualifier (`$B$2:$B$5` rather than `Sheet1!$B$2:$B$5`), so it
    /// does not say which of the workbook's sheets it means.
    NoSheetNamed,

    /// The reference names another workbook (`[1]Sheet1!$A$1`). That book is not this package's to
    /// edit, and this library never opens one.
    AnotherWorkbook,

    /// The reference spans several sheets (`Sheet1:Sheet3!$A$1`), or its areas do not all name the
    /// same one, so a point in the cache does not name one cell.
    SeveralSheets,

    /// The reference names whole columns or whole rows (`Sheet1!$B:$B`). Which cells of a whole
    /// column a series' points occupy is Excel's decision about populated data, not an address, and
    /// writing at a guessed offset would put numbers in cells the chart never named.
    WholeColumnsOrRows,

    /// The reference names a rectangle more than one column wide *and* more than one row tall, so
    /// the order its points map onto its cells is not stated. A multi-level category axis
    /// (`c:multiLvlStrRef`) reaches here, and deliberately: its level-to-column order is a
    /// convention rather than something the reference says.
    Rectangular,

    /// The workbook has no worksheet by the name the reference gives — the sheet was deleted, or the
    /// name reaches a chart sheet or a dialog sheet, which have no cells to write into.
    NoSuchSheet,

    /// The reference names fewer cells than the data has points, so writing them all would put
    /// values outside the range the chart itself says its data lives in.
    ///
    /// Growing the range would mean rewriting the producer's own `c:f`, which is the same class of
    /// act this whole module exists to stop.
    FewerCellsThanPoints,
}

impl std::fmt::Display for ReferenceProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotACellReference => "it is not a cell reference",
            Self::NoSheetNamed => "it names no sheet",
            Self::AnotherWorkbook => "it names another workbook",
            Self::SeveralSheets => "it does not name one single sheet",
            Self::WholeColumnsOrRows => "it names whole columns or rows rather than cells",
            Self::Rectangular => "it names a rectangle rather than one row or one column of cells",
            Self::NoSuchSheet => "the embedded workbook has no worksheet by that name",
            Self::FewerCellsThanPoints => "it names fewer cells than the data has points",
        })
    }
}

/// The cells a patch will write, worked out from the chart alone.
///
/// Built by [`plan_workbook_patch`] and consumed by [`apply_workbook_patch`]. It holds no borrow of
/// the chart or of any package, which is what lets a host finish with the chart's part tree before
/// it opens the workbook — see this module's own documentation for why that matters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkbookPatchPlan {
    ranges: Vec<PlannedRange>,
}

impl WorkbookPatchPlan {
    /// Whether the plan names no cell at all — a chart whose data is entirely literal, or a scope
    /// that found nothing to write.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.iter().all(|range| range.cells.is_empty())
    }
}

/// Works out which cells of the chart's embedded workbook `patch` writes, and what goes in them.
///
/// Reads the chart and nothing else, so it can run while the host still holds the chart part's tree —
/// and so a refusal reaches the caller before anything has been edited.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] for a series index past the end, or
/// [`ChartAccessError::EmbeddedWorkbookNotWritable`] naming the `c:f` and the [`ReferenceProblem`]
/// that stopped it.
pub fn plan_workbook_patch(
    space: &ChartSpace,
    interner: &Interner,
    patch: WorkbookPatch<'_>,
) -> Result<WorkbookPatchPlan, ChartAccessError> {
    let mut ranges = Vec::new();
    match patch {
        WorkbookPatch::EveryReference => {
            if let Some(area) = space.plot_area() {
                for series in area.all_series() {
                    plan_every_reference_of(series, interner, &mut ranges)?;
                }
            }
        }
        WorkbookPatch::SeriesValues { series_idx, values } => {
            let series = series_at(space, series_idx)?;
            if let Some(formula) = value_formula(series) {
                let points = values
                    .iter()
                    .enumerate()
                    .map(|(offset, &value)| (offset, PlannedValue::Number(value)))
                    .collect();
                plan_reference(formula.text(), points, &mut ranges)?;
            }
        }
        WorkbookPatch::SeriesCategories { series_idx, labels } => {
            let series = series_at(space, series_idx)?;
            if let Some(formula) = category_label_formula(series) {
                let points = labels
                    .iter()
                    .enumerate()
                    .map(|(offset, label)| (offset, PlannedValue::Text((*label).to_owned())))
                    .collect();
                plan_reference(formula.text(), points, &mut ranges)?;
            }
        }
    }
    Ok(WorkbookPatchPlan { ranges })
}

/// Writes `plan` into `workbook` and serializes the package back, or answers `Ok(None)` when there
/// is nothing to write.
///
/// `Ok(None)` means **leave the part exactly as it is**: the plan named no cell, or every cell it
/// named already holds the value it would have been given. The host must not write the part in that
/// case, because re-saving a package changes its container bytes even when every part inside is
/// identical.
///
/// # Errors
/// [`ChartWorkbookError::Access`] carrying [`ChartAccessError::EmbeddedWorkbookNotWritable`] when a
/// planned reference names a worksheet the workbook does not have, or
/// [`ChartWorkbookError::Sml`] if the embedded package is not a readable OPC package, if a part it
/// names is not the markup it claims, or if the cell store refuses a value.
pub fn apply_workbook_patch(
    plan: &WorkbookPatchPlan,
    workbook: &[u8],
) -> Result<Option<Vec<u8>>, ChartWorkbookError> {
    if plan.is_empty() {
        return Ok(None);
    }
    apply(workbook, &plan.ranges)
}

// =================================================================================================
// Finding the part, once, for all three host crates
// =================================================================================================

/// The part of `package` holding the workbook a chart embeds, given the relationship id its
/// `c:externalData` names — or `None` when there is no embedded workbook to reach.
///
/// `None` has three causes and none of them is an error, which is why they are one answer: the
/// relationship id names no relationship (a reference the file left dangling), the relationship is
/// **external** and so points at a workbook somebody else hosts, or its target does not resolve to a
/// part name at all. A chart in any of those states simply has no workbook here to read or write.
///
/// This lives in `mjx-chart` rather than three times over in `mjx-pptx`, `mjx-docx` and `mjx-xlsx`
/// because all three ask exactly this question and all three answered it with the same twenty lines.
/// `mjx-chart` is rank 2.2 and `mjx-opc` is rank 1.0, so the edge it needs points strictly down and
/// already exists; the three format crates are rank 3.0 and reach this the way they reach every
/// other `mjx_chart` function.
///
/// Whether the part actually holds bytes is the caller's next question, because the caller is the
/// one holding the package it would read them from.
#[must_use]
pub fn embedded_workbook_part(
    package: &Package,
    chart_part: &PartName,
    relationship_id: &str,
) -> Option<PartName> {
    let relationship = package
        .relationships_for(Some(chart_part))?
        .by_id(relationship_id)?;
    if relationship.mode == TargetMode::External {
        return None;
    }
    chart_part.resolve(&relationship.target).ok()
}

// =================================================================================================
// Planning — from the chart alone, before the package is even opened
// =================================================================================================

/// One resolved `c:f`: the sheet it names, the cells it names there, and what belongs in each.
#[derive(Debug, Clone, PartialEq)]
struct PlannedRange {
    /// The `c:f` exactly as the chart wrote it, kept for the refusal a missing sheet earns.
    reference: String,
    /// The sheet the reference qualified itself with, unescaped.
    sheet: String,
    /// The cells to write, in the order the reference walks them.
    cells: Vec<(CellReference, PlannedValue)>,
}

/// A value bound for a cell.
#[derive(Debug, Clone, PartialEq)]
enum PlannedValue {
    /// A number, written as [`CellValue::Number`] does.
    Number(f64),
    /// Text.
    ///
    /// Written as an **inline string** (`t="inlineStr"`), not as a shared-string index. A shared
    /// string would mean editing `xl/sharedStrings.xml` too — appending an item, fixing two counts,
    /// and creating the part, its content-type override and its relationship when the workbook has
    /// none — so a call that was asked to change one label would rewrite a second part of somebody
    /// else's file. An inline string keeps the edit inside the one worksheet part, and Excel reads
    /// the two spellings identically. A label that has not changed is not written at all, so a
    /// workbook's existing shared strings stay shared.
    Text(String),
}

impl PlannedValue {
    /// Whether this is a value SpreadsheetML can express at all.
    ///
    /// `NaN` and the infinities have no numeric spelling, and a cache can hold a `c:v` that parses
    /// to one. The point is dropped rather than written, which leaves that cell as the file had it —
    /// the same thing `chart_ops::set_series_values` does to the cache itself.
    fn is_writable(&self) -> bool {
        match self {
            Self::Number(number) => number.is_finite(),
            Self::Text(_) => true,
        }
    }
}

/// Plans every reference one series names: its `c:tx`, its categories, its values and — for a bubble
/// series — its sizes. Each is written from the cache the chart currently holds.
fn plan_every_reference_of(
    series: &Series,
    interner: &Interner,
    ranges: &mut Vec<PlannedRange>,
) -> Result<(), ChartAccessError> {
    if let Some(reference) = series.name_source().and_then(SeriesText::reference) {
        plan_string_reference(reference, interner, ranges)?;
    }

    if let Some(categories) = series.categories().or_else(|| series.x_data()) {
        if let Some(reference) = categories.string_reference() {
            plan_string_reference(reference, interner, ranges)?;
        } else if let Some(reference) = categories.number_reference() {
            plan_number_reference(reference, interner, ranges)?;
        } else if let Some(reference) = categories.multi_level_reference() {
            // A multi-level axis names a rectangle, and `ReferenceProblem::Rectangular` is the
            // refusal it earns — stated here rather than left to fall through, so the reason given
            // is the reference's shape and not "no branch matched".
            if let Some(formula) = reference.formula() {
                return Err(not_writable(&formula.text(), ReferenceProblem::Rectangular));
            }
        }
    }

    for numeric in [series.values(), series.y_data(), series.bubble_sizes()]
        .into_iter()
        .flatten()
    {
        if let Some(reference) = numeric.reference() {
            plan_number_reference(reference, interner, ranges)?;
        }
    }
    Ok(())
}

/// Plans a `c:strRef` from the labels it currently caches, each at the `c:pt@idx` it declares.
fn plan_string_reference(
    reference: &StringReference,
    interner: &Interner,
    ranges: &mut Vec<PlannedRange>,
) -> Result<(), ChartAccessError> {
    let (Some(formula), Some(cache)) = (reference.formula(), reference.cache()) else {
        return Ok(());
    };
    let points = cache
        .points()
        .enumerate()
        .map(|(position, point)| {
            (
                point_offset(point, interner, position),
                PlannedValue::Text(point.value_str().unwrap_or_default()),
            )
        })
        .collect();
    plan_reference(formula.text(), points, ranges)
}

/// Plans a `c:numRef` from the numbers it currently caches, each at the `c:pt@idx` it declares.
fn plan_number_reference(
    reference: &NumberReference,
    interner: &Interner,
    ranges: &mut Vec<PlannedRange>,
) -> Result<(), ChartAccessError> {
    let (Some(formula), Some(cache)) = (reference.formula(), reference.cache()) else {
        return Ok(());
    };
    let points = cache
        .points()
        .enumerate()
        .filter_map(|(position, point)| {
            let value = point.value_f64()?;
            Some((
                point_offset(point, interner, position),
                PlannedValue::Number(value),
            ))
        })
        .collect();
    plan_reference(formula.text(), points, ranges)
}

/// Where in its range a cached point sits.
///
/// **`c:pt@idx`, not the point's position in the file.** A cache is allowed to be sparse — a series
/// with a blank third value writes points `0`, `1`, `3` — and counting positions instead would slide
/// every later value one cell up the producer's column. `@idx` is `use="required"`, so the fallback
/// is only reached by a malformed cache, where document order is the best statement left.
fn point_offset(point: &DataPoint, interner: &Interner, position: usize) -> usize {
    point
        .index(interner)
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or(position)
}

/// The `c:f` of the series' numeric values, or `None` when it has none to write into.
fn value_formula(series: &Series) -> Option<&Formula> {
    series
        .values()
        .or_else(|| series.y_data())
        .and_then(NumericData::reference)
        .and_then(NumberReference::formula)
}

/// The `c:f` of the series' **string** category labels, or `None`.
///
/// Only the string shape: a numeric or multi-level category source has no labels to rewrite, and
/// `chart_ops::set_series_categories` refuses such a series before the workbook is ever reached.
fn category_label_formula(series: &Series) -> Option<&Formula> {
    series
        .categories()
        .or_else(|| series.x_data())
        .and_then(CategoryData::string_reference)
        .and_then(StringReference::formula)
}

/// Resolves one `c:f` to cells, pairs each point with the cell at its offset, and appends the
/// result to `ranges`.
///
/// An offset the data has no point for is not written: a range of four cells given points `0` and
/// `2` writes two cells and leaves the other two holding whatever the file put there. Writing a
/// blank instead would be deleting content in cells the caller said nothing about.
fn plan_reference(
    reference: String,
    points: Vec<(usize, PlannedValue)>,
    ranges: &mut Vec<PlannedRange>,
) -> Result<(), ChartAccessError> {
    let points: Vec<(usize, PlannedValue)> = points
        .into_iter()
        .filter(|(_, value)| value.is_writable())
        .collect();
    let Some(wanted) = points.iter().map(|(offset, _)| offset + 1).max() else {
        return Ok(());
    };
    let (sheet, cells) =
        cells_of(&reference, wanted).map_err(|problem| not_writable(&reference, problem))?;
    ranges.push(PlannedRange {
        reference,
        sheet,
        cells: points
            .into_iter()
            .filter_map(|(offset, value)| Some((*cells.get(offset)?, value)))
            .collect(),
    });
    Ok(())
}

/// The refusal a reference earns, carrying the `c:f` exactly as the chart wrote it.
fn not_writable(reference: &str, problem: ReferenceProblem) -> ChartAccessError {
    ChartAccessError::EmbeddedWorkbookNotWritable {
        reference: reference.to_owned(),
        problem,
    }
}

/// The sheet a `c:f` names and the first `wanted` cells it names there.
///
/// Areas are walked in the order the reference writes them and each contributes its cells in reading
/// order along its one populated axis, which is the order a cache's `c:pt@idx` counts in. Every area
/// of a multi-area reference must name the same sheet, because a plan writes one sheet per range;
/// one that does not is refused as [`ReferenceProblem::SeveralSheets`].
fn cells_of(
    formula: &str,
    wanted: usize,
) -> Result<(String, Vec<CellReference>), ReferenceProblem> {
    let areas = ReferenceAreas::parse(formula).map_err(|_| ReferenceProblem::NotACellReference)?;
    let mut sheet: Option<String> = None;
    let mut cells = Vec::with_capacity(wanted);
    for area in areas {
        let reference = SheetQualifiedReference::parse(area).map_err(|error| match error {
            AddressError::MissingSheetSeparator | AddressError::EmptySheetName => {
                ReferenceProblem::NoSheetNamed
            }
            _ => ReferenceProblem::NotACellReference,
        })?;
        if reference.external_book().is_some() {
            return Err(ReferenceProblem::AnotherWorkbook);
        }
        if reference.last_sheet().is_some() {
            return Err(ReferenceProblem::SeveralSheets);
        }
        if matches!(
            reference.target(),
            CellRange::Columns { .. } | CellRange::Rows { .. }
        ) {
            return Err(ReferenceProblem::WholeColumnsOrRows);
        }
        let named = reference.first_sheet().name().into_owned();
        match &sheet {
            None => sheet = Some(named),
            Some(first) if first.eq_ignore_ascii_case(&named) => {}
            Some(_) => return Err(ReferenceProblem::SeveralSheets),
        }

        let bounds = reference.target().normalized_bounds();
        let columns = u32::from(bounds.last_column() - bounds.first_column()) + 1;
        let rows = bounds.last_row() - bounds.first_row() + 1;
        if columns > 1 && rows > 1 {
            return Err(ReferenceProblem::Rectangular);
        }
        // The rectangular case is refused above, so exactly one of these two loops runs more than
        // once: a single column walked down, or a single row walked across.
        for row in bounds.first_row()..=bounds.last_row() {
            for column in bounds.first_column()..=bounds.last_column() {
                let cell =
                    CellReference::new(column, row, Anchoring::Absolute, Anchoring::Absolute)
                        .map_err(|_| ReferenceProblem::NotACellReference)?;
                cells.push(cell);
                if cells.len() == wanted {
                    return Ok((sheet.unwrap_or_default(), cells));
                }
            }
        }
    }
    Err(ReferenceProblem::FewerCellsThanPoints)
}

// =================================================================================================
// Applying — the only part that opens the package
// =================================================================================================

/// Writes the planned cells into `workbook` and serializes it back, or `None` when every one of them
/// already held its value.
fn apply(workbook: &[u8], ranges: &[PlannedRange]) -> Result<Option<Vec<u8>>, ChartWorkbookError> {
    let mut package = Package::open(workbook).map_err(SmlError::Opc)?;
    let sheets = worksheet_parts(&mut package)?;
    let strings = shared_strings(&mut package)?;

    // One pass per sheet, however many ranges land on it: a worksheet part is parsed once, edited in
    // place and written once. A pass per range would parse a 300,000-cell sheet once per series.
    let mut written = false;
    let mut seen: Vec<&str> = Vec::new();
    for range in ranges {
        if seen
            .iter()
            .any(|sheet| sheet.eq_ignore_ascii_case(&range.sheet))
        {
            continue;
        }
        seen.push(&range.sheet);

        let part = sheets
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(&range.sheet))
            .map(|(_, part)| part.clone())
            .ok_or_else(|| refusal(range, ReferenceProblem::NoSuchSheet))?;
        let bytes = package
            .part_payload(&part)
            .ok_or_else(|| SmlError::Opc(OpcError::UnknownPart(part.as_str().to_owned())))?;
        let mut markup = WorksheetPart::read_part(&bytes)?
            .ok_or_else(|| refusal(range, ReferenceProblem::NoSuchSheet))?;

        let mut changed = false;
        for same_sheet in ranges
            .iter()
            .filter(|other| other.sheet.eq_ignore_ascii_case(&range.sheet))
        {
            for (cell, value) in &same_sheet.cells {
                if already_holds(&markup, *cell, value, strings.as_ref()) {
                    continue;
                }
                markup.set_cell_value(*cell, cell_value(value))?;
                changed = true;
            }
        }
        if changed {
            package.replace_part_bytes(&part, markup.to_markup())?;
            written = true;
        }
    }

    if !written {
        return Ok(None);
    }
    // `save_unchecked`, not `save`: this changed no part graph, no relationship and no content type —
    // only the bytes inside one worksheet — so validation here can only reject a fault the file
    // arrived with, and refusing there would lose the caller's edit over a defect this library did
    // not make.
    Ok(Some(package.save_unchecked().map_err(SmlError::Opc)?))
}

/// Whether the cell already says exactly what the patch was going to make it say.
///
/// A missing cell holds nothing, so it never matches. A number is compared as a number, so a file
/// that spelled it `9.0` keeps that spelling; text is compared after resolving a shared-string
/// index, so a shared string that has not changed stays shared.
fn already_holds(
    markup: &WorksheetPart,
    reference: CellReference,
    value: &PlannedValue,
    strings: Option<&SharedStringTable>,
) -> bool {
    let Some(cell) = markup.cell(reference) else {
        return false;
    };
    match value {
        PlannedValue::Number(number) => cell.number() == Some(*number),
        PlannedValue::Text(text) => cell_text(&cell, strings).as_deref() == Some(text.as_str()),
    }
}

/// The text a cell holds, with a `t="s"` index resolved through the table, or `None` when it holds
/// no text or the text will not decode.
fn cell_text(cell: &Cell<'_>, strings: Option<&SharedStringTable>) -> Option<String> {
    if let Some(index) = cell.shared_string_index() {
        return strings?
            .item(index)?
            .text()
            .ok()
            .map(std::borrow::Cow::into_owned);
    }
    if let Some(inline) = cell.inline_string_markup() {
        return InlineString::parse(inline)
            .ok()?
            .item()
            .text()
            .ok()
            .map(std::borrow::Cow::into_owned);
    }
    None
}

/// The refusal a range earns when the workbook cannot give it cells to write.
fn refusal(range: &PlannedRange, problem: ReferenceProblem) -> ChartWorkbookError {
    ChartWorkbookError::Access(ChartAccessError::EmbeddedWorkbookNotWritable {
        reference: range.reference.clone(),
        problem,
    })
}

/// The value to write, borrowed from the plan.
fn cell_value(value: &PlannedValue) -> CellValue<'_> {
    match value {
        PlannedValue::Number(number) => CellValue::Number(*number),
        PlannedValue::Text(text) => CellValue::InlineString(text),
    }
}

/// Every worksheet in the embedded package, by the tab name the workbook part gives it.
///
/// A chart sheet or a dialog sheet is deliberately absent: it is reached through a relationship of a
/// different type, and there are no cells behind it to write. A reference naming one is refused as
/// [`ReferenceProblem::NoSuchSheet`], which is what it is from here — the name reaches no worksheet.
fn worksheet_parts(package: &mut Package) -> Result<Vec<(String, PartName)>, ChartWorkbookError> {
    let workbook_part = office_document_part(package)?;
    let entries = sheet_entries(package, &workbook_part)?;

    let Some(relationships) = package.relationships_for(Some(&workbook_part)) else {
        return Ok(Vec::new());
    };
    let mut sheets = Vec::with_capacity(entries.len());
    for (name, relationship_id) in entries {
        let Some(relationship) = relationships.by_id(&relationship_id) else {
            continue;
        };
        if relationship.rel_type != REL_WORKSHEET {
            continue;
        }
        let Ok(part) = workbook_part.resolve(&relationship.target) else {
            continue;
        };
        sheets.push((name, part));
    }
    Ok(sheets)
}

/// Each tab's name and the relationship id it names its part with, read through `mjx-sml`'s model.
///
/// Split out because reading them borrows the workbook part's tree and resolving them borrows the
/// package's relationships, and the two borrows cannot overlap.
fn sheet_entries(
    package: &mut Package,
    workbook_part: &PartName,
) -> Result<Vec<(String, String)>, ChartWorkbookError> {
    let document = package.part_tree(workbook_part).map_err(SmlError::Opc)?;
    let interner = &document.interner;
    let Some(part) = WorkbookPart::read_part(document)? else {
        return Err(malformed(
            "the embedded workbook's root is not an x:workbook",
        ));
    };
    let reference_prefix = part.relationship_prefix(interner);
    let Some(list) = part.sheets() else {
        return Ok(Vec::new());
    };
    Ok(list
        .entries()
        .map(|sheet| {
            let name = sheet
                .name(interner)
                .ok()
                .flatten()
                .map(Cow::into_owned)
                .unwrap_or_default();
            let relationship = sheet
                .relationship_id(interner, reference_prefix)
                .ok()
                .flatten()
                .unwrap_or_default();
            (name, relationship)
        })
        .collect())
}

/// The embedded workbook's shared-string table, or `None` when it has no `xl/sharedStrings.xml`.
///
/// Read so that a `t="s"` cell can be **compared** with the text a patch would write. Nothing here
/// ever writes into the table; see [`PlannedValue::Text`] for why.
fn shared_strings(package: &mut Package) -> Result<Option<SharedStringTable>, ChartWorkbookError> {
    let workbook_part = office_document_part(package)?;
    let Some(target) = package
        .relationships_for(Some(&workbook_part))
        .and_then(|rels| rels.by_type(REL_SHARED_STRINGS).next())
        .map(|relationship| relationship.target.clone())
    else {
        return Ok(None);
    };
    let Ok(part) = workbook_part.resolve(&target) else {
        return Ok(None);
    };
    if !package.contains_part(&part) {
        return Ok(None);
    }
    let document = package.part_tree(&part).map_err(SmlError::Opc)?;
    Ok(SharedStringTable::read_part(document)?)
}

/// The part the package's root relationships call the office document — `xl/workbook.xml` in every
/// workbook anybody writes, found by relationship type rather than by that name.
fn office_document_part(package: &Package) -> Result<PartName, ChartWorkbookError> {
    let relationship = package
        .relationships_for(None)
        .and_then(|rels| rels.by_type(REL_OFFICE_DOCUMENT).next())
        .ok_or_else(|| malformed("the embedded workbook names no office-document relationship"))?;
    PartName::resolve_from_root(&relationship.target).map_err(Into::into)
}

/// A malformed-package refusal with a written reason.
fn malformed(reason: &str) -> ChartWorkbookError {
    ChartWorkbookError::Sml(SmlError::Opc(OpcError::Malformed(reason.to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(reference: &str, wanted: usize) -> (String, Vec<(u16, u32)>) {
        let (sheet, cells) = cells_of(reference, wanted).expect("a resolvable reference");
        (
            sheet,
            cells
                .iter()
                .map(|cell| (cell.column(), cell.row()))
                .collect(),
        )
    }

    #[test]
    fn a_single_column_range_walks_down_it() {
        let (sheet, cells) = walk("Sheet0!$B$2:$B$5", 4);
        assert_eq!(sheet, "Sheet0");
        assert_eq!(cells, [(1, 1), (1, 2), (1, 3), (1, 4)]);
    }

    #[test]
    fn a_single_row_range_walks_across_it() {
        assert_eq!(walk("Sheet1!$B$1:$D$1", 3).1, [(1, 0), (2, 0), (3, 0)]);
    }

    #[test]
    fn a_single_cell_reference_names_one_cell() {
        let (sheet, cells) = walk("'My Sheet'!$B$1", 1);
        assert_eq!(sheet, "My Sheet");
        assert_eq!(cells, [(1, 0)]);
    }

    #[test]
    fn a_union_of_two_areas_on_one_sheet_is_walked_in_order() {
        assert_eq!(
            walk("(Data!$B$2:$B$3,Data!$D$2:$D$3)", 4).1,
            [(1, 1), (1, 2), (3, 1), (3, 2)]
        );
    }

    #[test]
    fn every_shape_this_library_will_not_write_is_refused_by_name() {
        for (reference, expected) in [
            ("SUM(Sheet1!$B$2:$B$3)", ReferenceProblem::NotACellReference),
            ("$B$2:$B$5", ReferenceProblem::NoSheetNamed),
            ("[1]Sheet1!$B$2:$B$5", ReferenceProblem::AnotherWorkbook),
            ("Sheet1:Sheet3!$B$2", ReferenceProblem::SeveralSheets),
            ("Sheet1!$B:$B", ReferenceProblem::WholeColumnsOrRows),
            ("Sheet1!$A$2:$B$5", ReferenceProblem::Rectangular),
            ("Sheet1!$B$2:$B$3", ReferenceProblem::FewerCellsThanPoints),
            (
                "(One!$B$2:$B$3,Two!$B$2:$B$3)",
                ReferenceProblem::SeveralSheets,
            ),
        ] {
            assert_eq!(
                cells_of(reference, 4).expect_err("refused"),
                expected,
                "{reference}"
            );
        }
    }

    #[test]
    fn a_sparse_cache_writes_each_point_at_the_index_it_declares() {
        // The trap this guards: counting positions instead of reading `c:pt@idx` slides every value
        // after a gap one cell up somebody else's column.
        let mut ranges = Vec::new();
        plan_reference(
            "Sheet1!$B$2:$B$5".to_owned(),
            vec![
                (0, PlannedValue::Number(1.0)),
                (3, PlannedValue::Number(4.0)),
            ],
            &mut ranges,
        )
        .expect("a resolvable reference");
        assert_eq!(
            ranges[0]
                .cells
                .iter()
                .map(|(cell, _)| (cell.column(), cell.row()))
                .collect::<Vec<_>>(),
            [(1, 1), (1, 4)],
            "the second point lands on B5, not B3"
        );
    }

    #[test]
    fn a_non_finite_number_is_dropped_rather_than_written() {
        let mut ranges = Vec::new();
        plan_reference(
            "Sheet1!$B$2:$B$3".to_owned(),
            vec![
                (0, PlannedValue::Number(f64::NAN)),
                (1, PlannedValue::Number(2.0)),
            ],
            &mut ranges,
        )
        .expect("a resolvable reference");
        assert_eq!(ranges[0].cells.len(), 1);
        assert_eq!(ranges[0].cells[0].0.row(), 2, "only B3 is written");
    }
}
