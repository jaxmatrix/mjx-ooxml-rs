//! Resolving a chart's `c:f` against the cells of this workbook (MJXOFF-111, E4).
//!
//! # The case that exists nowhere else in this library
//!
//! A chart in a presentation or a document carries a copy of its data: `c:externalData` names an
//! embedded `.xlsx`, and the caches beside each `c:f` are what draws. A chart on a **worksheet**
//! normally has neither. Its `c:numRef`/`c:strRef` carry a `c:f` naming a range in the sheets the
//! chart already lives among, and the cells *are* the source — so a reader that can only answer
//! from the cache can only ever answer what the file last cached, which may be nothing like what
//! the sheet now says.
//!
//! This module is the resolver that answers from the cells. It reads; it changes nothing.
//!
//! # It resolves a reference. It does not evaluate a formula.
//!
//! `c:f` is a *reference*, not an expression, and the two are not the same job. Working out that
//! `Data!$B$2:$B$4` names three cells is address arithmetic; working out what `=SUM(B2:B4)` comes
//! to is a calculation engine, which this project states as a permanent non-goal (MJXOFF-115). A
//! cell holding a formula answers here with its **cached value**, exactly as
//! [`Workbook::cell_text`](crate::Workbook::cell_text) does, and the guide says so.
//!
//! # Bounded by the range, not by the sheet
//!
//! MJXOFF-135 measured this crate's read path on a 300,000-cell worksheet and MJXOFF-137 acted on
//! it: no per-cell accessor ships on the facade or either binding, because reaching one cell costs
//! a whole-worksheet parse. A resolver that then materialised the whole sheet to answer a
//! three-point series would give that measurement straight back.
//!
//! So two things are true of every resolution here, and
//! `crates/mjx-xlsx/examples/chart_range_cost.rs` asserts both with the counting allocator rather
//! than describing them:
//!
//! * **each sheet is parsed at most once per call**, however many series or areas name it — the
//!   resolver caches the [`WorksheetPart`] and the shared-string table for the length of one call;
//! * **the walk is `min(rows the range spans, rows the sheet has)`**. A three-row range on a
//!   300,000-cell sheet looks three rows up by number; a whole-column reference walks the sheet's
//!   populated rows instead of a million addressable ones. Neither shape can walk more than the
//!   other, and the same rule applies within a row for the columns.
//!
//! # A blank cell is absent, not zero
//!
//! [`ResolvedRange::cells`] carries only the cells that hold something, each with the `offset` it
//! sits at within the reference. That is the same shape the caches themselves have — a `c:numCache`
//! writes `<c:pt idx="…">` for the points it has and omits the rest — so a cache and a resolution
//! line up index for index without either being padded, and a range naming a million addressable
//! cells costs what its populated cells cost.
//!
//! # Every unresolvable case is reported, never guessed
//!
//! A `c:f` comes out of a file somebody else wrote. It may name a sheet that has been deleted, a
//! defined name that does not exist, another workbook this library never opens, or markup that is
//! not a reference at all. Each of those is a [`RangeProblem`] recorded against the area it came
//! from — the resolution still answers, with the areas it *could* resolve — because a chart whose
//! second series points at a deleted sheet still has a first series worth reading.

use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_sml::{
    AddressError, Cell, CellRange, CellReference, GridBounds, ReferenceAreas, Row,
    SharedStringTable, SheetData, SheetQualifiedReference, WorksheetPart,
};

use crate::error::XlsxError;
use crate::workbook::{DefinedNameEntry, DefinedNameScope, Workbook};

/// How deep a defined name may be defined in terms of another before this stops following it.
///
/// A name whose definition is another name is legal and Excel follows it. A name that reaches
/// itself is not, and a file can carry one. The depth is a cheap second guard beside the
/// already-visited check: neither alone would refuse both `A → B → A → B …` and a chain a thousand
/// names long, and both come out of files this library did not write.
const MAX_DEFINED_NAME_DEPTH: usize = 16;

/// What one cell a reference reaches holds, decoded.
///
/// The variants are the things a populated cell can say once its `c@t` has been read. A blank cell
/// is not one of them: it is absent from [`ResolvedRange::cells`] entirely, for the reason this
/// module's own documentation gives.
#[derive(Debug, Clone, PartialEq)]
pub enum RangeCellValue {
    /// A number — `c@t="n"`, or a cell with no `t` at all, which is the same thing.
    Number(f64),
    /// Text — a shared string resolved through `xl/sharedStrings.xml`, an `inlineStr`'s own `<is>`,
    /// or the string result of a formula (`t="str"`).
    Text(String),
    /// A boolean (`t="b"`).
    Boolean(bool),
    /// An error code the file carried — `#DIV/0!`, `#N/A` (`t="e"`).
    ///
    /// Reported rather than turned into absence: a chart drawing a series with `#N/A` in it draws a
    /// gap, and a caller cannot tell that from an empty cell unless it is told.
    Error(String),
    /// A `t="s"` whose index names no entry in the shared-string table, or a cell whose value will
    /// not decode as its own type says it should.
    ///
    /// A defect in the file, reported where it is rather than repaired or dropped — the same
    /// judgement [`Workbook::cell_text`](crate::Workbook::cell_text) makes when it answers `None`.
    Undecodable,
}

impl RangeCellValue {
    /// This value as a number, or `None` for one that is not one.
    ///
    /// A boolean is **not** a number here. Excel plots `TRUE` as `1`, and this library could too —
    /// but that is a coercion rule in a calculation model this project does not have, and inventing
    /// one would put a number in a series the file never wrote. A caller that wants Excel's rule can
    /// apply it to the [`Boolean`](Self::Boolean) this reports.
    #[must_use]
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// The text of a string cell, or of an error code. `None` for anything else.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) | Self::Error(text) => Some(text),
            _ => None,
        }
    }

    /// What a category axis would show for this cell — the text of a string, the shortest
    /// round-tripping spelling of a number, `TRUE`/`FALSE` for a boolean, the code of an error.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Text(text) | Self::Error(text) => text.clone(),
            Self::Number(number) => number.to_string(),
            Self::Boolean(value) => if *value { "TRUE" } else { "FALSE" }.to_owned(),
            Self::Undecodable => String::new(),
        }
    }
}

/// One cell a reference reached, and where it sits in that reference.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRangeCell {
    /// The cell's zero-based position **within the whole reference** — area by area in the order the
    /// reference wrote them, and inside an area row by row and then column by column.
    ///
    /// This is the index a cache's `c:pt@idx` uses, which is what lets a cache and a resolution be
    /// compared without padding either with blanks.
    pub offset: u64,
    /// The tab the cell is on.
    pub sheet_index: usize,
    /// Where on that tab.
    pub reference: CellReference,
    /// What the cell holds.
    pub value: RangeCellValue,
}

/// Why one area of a reference could not be resolved.
///
/// Every variant is a fact about the file rather than a failure of this crate, which is why an area
/// carrying one is *reported* alongside the areas that did resolve rather than failing the whole
/// call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeProblem {
    /// The text is not a reference this grammar admits.
    Malformed(AddressError),
    /// The reference names a sheet this workbook does not have — a tab deleted or renamed after the
    /// chart was written.
    UnknownSheet {
        /// The name the file wrote, with its quoting undone.
        name: String,
    },
    /// The far end of a 3-D span (`Sheet1:Sheet3!A1`) names no tab of this workbook.
    UnknownSpanEnd {
        /// The end that names no tab.
        name: String,
    },
    /// The reference names another workbook (`[1]Sheet1!A1`).
    ///
    /// This library performs no external I/O by design, so those cells are not here to be read. The
    /// chart's cache is the only answer, and it is reported beside this one.
    ExternalBook {
        /// The index into the workbook's external-link list the file wrote.
        index: u32,
    },
    /// The reference names a tab whose part is a chartsheet or a dialogsheet — which has no cells —
    /// or one whose relationship reaches no part at all.
    SheetHasNoCells {
        /// The tab the reference named.
        sheet_index: usize,
    },
    /// The area is a defined name this workbook does not define, in either scope.
    UnknownDefinedName {
        /// The name the file wrote.
        name: String,
    },
    /// A defined name is defined in terms of itself, directly or through a chain.
    DefinedNameCycle {
        /// The name the chain came back to.
        name: String,
    },
    /// A chain of defined names ran deeper than this resolver follows (sixteen).
    DefinedNameTooDeep {
        /// The name the chain was following when it stopped.
        name: String,
    },
}

/// One area of a reference, resolved as far as it could be.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedArea {
    /// The area exactly as the reference wrote it.
    pub reference: String,
    /// The tab it names, once every defined name has been followed.
    pub sheet_index: Option<usize>,
    /// The rectangle it names on that tab, ordered — `C3:A1` and `A1:C3` name the same one.
    pub bounds: Option<GridBounds>,
    /// Why it could not be resolved, or `None` when it was.
    pub problem: Option<RangeProblem>,
}

/// A whole `c:f`, resolved against this workbook's cells.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRange {
    /// The reference exactly as the chart wrote it.
    pub reference: String,
    /// Its areas, in the order it wrote them. A single-area reference has one; a defined name that
    /// stands for three has three.
    pub areas: Vec<ResolvedArea>,
    /// How many cells the reference **addresses**, blanks included.
    ///
    /// Saturating: a reference naming whole columns addresses more cells than a `u64` would care to
    /// multiply, and no answer here is worth an overflow.
    pub addressed_cells: u64,
    /// The cells that hold something, in reference order. See this module's own documentation for
    /// why a blank cell is absent rather than present-and-empty.
    pub cells: Vec<ResolvedRangeCell>,
}

impl ResolvedRange {
    /// Whether every area resolved.
    #[must_use]
    pub fn is_fully_resolved(&self) -> bool {
        !self.areas.is_empty() && self.areas.iter().all(|area| area.problem.is_none())
    }

    /// The first problem any area reported, or `None`.
    #[must_use]
    pub fn problem(&self) -> Option<&RangeProblem> {
        self.areas.iter().find_map(|area| area.problem.as_ref())
    }

    /// The numeric values the reference reaches, laid out **positionally** against it — `None` where
    /// the cell is blank or holds something that is not a number.
    ///
    /// Capped at `limit` entries, which a caller sets from the thing it is lining the values up
    /// against: a series' cache point count, almost always. Without a cap this would be the one
    /// method here that a whole-column reference could make enormous, which is precisely the shape
    /// MJXOFF-137 declined to ship.
    #[must_use]
    pub fn numbers(&self, limit: usize) -> Vec<Option<f64>> {
        let mut out = vec![None; self.positional_length(limit)];
        for cell in &self.cells {
            if let Some(slot) = usize::try_from(cell.offset)
                .ok()
                .and_then(|at| out.get_mut(at))
            {
                *slot = cell.value.number();
            }
        }
        out
    }

    /// The labels the reference reaches, laid out positionally — an empty string where the cell is
    /// blank, which is what a category axis shows for one.
    ///
    /// Capped like [`numbers`](Self::numbers), and for the same reason.
    #[must_use]
    pub fn labels(&self, limit: usize) -> Vec<String> {
        let mut out = vec![String::new(); self.positional_length(limit)];
        for cell in &self.cells {
            if let Some(slot) = usize::try_from(cell.offset)
                .ok()
                .and_then(|at| out.get_mut(at))
            {
                *slot = cell.value.label();
            }
        }
        out
    }

    /// How long a positional view of this range is: what it addresses, capped at `limit`.
    fn positional_length(&self, limit: usize) -> usize {
        usize::try_from(self.addressed_cells)
            .unwrap_or(usize::MAX)
            .min(limit)
    }
}

impl Workbook {
    /// Resolves `reference` — a chart's `c:f`, a defined name's definition, anything of that shape —
    /// against this workbook's cells, with `sheet_index` as the tab an area that names none means.
    ///
    /// Reading only: nothing is dirtied, and a workbook this was called on saves byte for byte what
    /// it would have saved before. `&mut self` because resolving a defined name reads
    /// `xl/workbook.xml`, which parses a tree onto the package the way every other model read here
    /// does.
    ///
    /// See this module's own documentation for what is resolved, what is deliberately not (a
    /// formula's *value*), what a blank cell does, and why the walk is bounded by the range rather
    /// than by the sheet.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `sheet_index` names no tab, or [`XlsxError`] if the workbook
    /// part or a worksheet part this had to open is not well-formed. An area that cannot be
    /// *resolved* is not an error: it comes back as a [`RangeProblem`] on that area.
    pub fn resolve_range_reference(
        &mut self,
        sheet_index: usize,
        reference: &str,
    ) -> Result<ResolvedRange, XlsxError> {
        let sheets = self.sheets().len();
        if sheet_index >= sheets {
            return Err(XlsxError::NoSuchSheet {
                index: sheet_index,
                sheets,
            });
        }
        let names = self.defined_names()?;
        RangeResolver::new(self, names).resolve(sheet_index, reference)
    }
}

/// Resolves one or more references against a workbook, parsing each sheet it needs **once**.
///
/// The caching is the whole reason this is a value rather than a free function: a chart with four
/// series naming the same sheet would otherwise pay four whole-worksheet parses for what one
/// answers, which is the cost MJXOFF-135 measured and MJXOFF-153 is filed to fix at its root.
pub(crate) struct RangeResolver<'a> {
    workbook: &'a Workbook,
    /// The workbook's defined names, read once before the resolver was built.
    names: Vec<DefinedNameEntry>,
    /// The worksheet parts opened so far, by tab index. `None` for a tab that has no cells to read
    /// — a chartsheet, a dialogsheet, a tab whose relationship reaches nothing — cached so that the
    /// answer is not re-derived per area.
    sheets: Vec<(usize, Option<WorksheetPart>)>,
    /// The shared-string table. The outer `Option` is *not read yet*; the inner one is *this
    /// workbook has none*.
    strings: Option<Option<SharedStringTable>>,
}

impl<'a> RangeResolver<'a> {
    /// A resolver over `workbook`, with `names` already read from it, and nothing opened yet.
    pub(crate) fn new(workbook: &'a Workbook, names: Vec<DefinedNameEntry>) -> Self {
        Self {
            workbook,
            names,
            sheets: Vec::new(),
            strings: None,
        }
    }

    /// Resolves `reference`, with `default_sheet` as the tab an unqualified area means.
    ///
    /// # Errors
    /// [`XlsxError`] if a worksheet part this had to open is not well-formed XML or does not match
    /// `CT_Worksheet`.
    pub(crate) fn resolve(
        &mut self,
        default_sheet: usize,
        reference: &str,
    ) -> Result<ResolvedRange, XlsxError> {
        let mut resolved = ResolvedRange {
            reference: reference.to_owned(),
            areas: Vec::new(),
            addressed_cells: 0,
            cells: Vec::new(),
        };
        match ReferenceAreas::parse(reference) {
            Ok(areas) => {
                for area in areas {
                    let mut followed = Vec::new();
                    self.resolve_area(default_sheet, area, &mut followed, &mut resolved)?;
                }
            }
            Err(problem) => resolved
                .areas
                .push(unresolved(reference, RangeProblem::Malformed(problem))),
        }
        // The gather walks a row at a time and, inside a row, whichever axis is cheaper — both of
        // which happen to produce ascending offsets for every file this project has read. `sorted`
        // makes that a guarantee rather than an observation, because `ResolvedRange::numbers` and
        // the series comparison beside it both read `cells` as *the reference's own order*.
        resolved.cells.sort_by_key(|cell| cell.offset);
        Ok(resolved)
    }

    /// Resolves one area, following a defined name if that is what it is, and appends what it found
    /// to `out`.
    ///
    /// `followed` is the chain of defined names already entered, which is what refuses a name
    /// defined in terms of itself without following it forever.
    fn resolve_area(
        &mut self,
        default_sheet: usize,
        area: &str,
        followed: &mut Vec<String>,
        out: &mut ResolvedRange,
    ) -> Result<(), XlsxError> {
        // 1. A sheet-qualified reference — what a chart's `c:f` is in every file this project has
        //    read.
        match SheetQualifiedReference::parse(area) {
            Ok(reference) => return self.resolve_qualified(area, reference, out),
            // Not sheet-qualified at all: it may still be a bare range or a defined name.
            Err(AddressError::MissingSheetSeparator) => {}
            Err(problem) => {
                out.areas
                    .push(unresolved(area, RangeProblem::Malformed(problem)));
                return Ok(());
            }
        }

        // 2. A bare range, on the tab the caller said an unqualified area means.
        if let Ok(range) = CellRange::parse(area) {
            return self.collect(default_sheet, area, range, out);
        }

        // 3. A defined name. `@name` is `ST_Xstring` and matching it is exact — a case-insensitive
        //    match is a rule this library would be inventing, which is the judgement
        //    `Workbook::defined_name` already made.
        self.follow_defined_name(default_sheet, area, followed, out)
    }

    /// Resolves an area that named a sheet.
    fn resolve_qualified(
        &mut self,
        area: &str,
        reference: SheetQualifiedReference<'_>,
        out: &mut ResolvedRange,
    ) -> Result<(), XlsxError> {
        if let Some(index) = reference.external_book() {
            out.areas
                .push(unresolved(area, RangeProblem::ExternalBook { index }));
            return Ok(());
        }
        let first = reference.first_sheet().name();
        let Some(first_index) = self.workbook.sheet_index_by_name(&first) else {
            out.areas.push(unresolved(
                area,
                RangeProblem::UnknownSheet {
                    name: first.into_owned(),
                },
            ));
            return Ok(());
        };
        // A 3-D span (`Sheet1:Sheet3!A1`) names the same rectangle on every tab from one end to the
        // other in tab order. Walking the tab list is mechanical — it evaluates nothing — so it is
        // done rather than refused, and an end naming no tab is reported rather than assumed to be
        // the first or the last.
        let last_index = match reference.last_sheet() {
            None => first_index,
            Some(last) => {
                let name = last.name();
                match self.workbook.sheet_index_by_name(&name) {
                    Some(index) => index,
                    None => {
                        out.areas.push(unresolved(
                            area,
                            RangeProblem::UnknownSpanEnd {
                                name: name.into_owned(),
                            },
                        ));
                        return Ok(());
                    }
                }
            }
        };
        let (low, high) = (first_index.min(last_index), first_index.max(last_index));
        for index in low..=high {
            self.collect(index, area, reference.target(), out)?;
        }
        Ok(())
    }

    /// Resolves `area` as a defined name, following its definition once it is found.
    fn follow_defined_name(
        &mut self,
        default_sheet: usize,
        area: &str,
        followed: &mut Vec<String>,
        out: &mut ResolvedRange,
    ) -> Result<(), XlsxError> {
        if followed.iter().any(|name| name == area) {
            out.areas.push(unresolved(
                area,
                RangeProblem::DefinedNameCycle {
                    name: area.to_owned(),
                },
            ));
            return Ok(());
        }
        if followed.len() >= MAX_DEFINED_NAME_DEPTH {
            out.areas.push(unresolved(
                area,
                RangeProblem::DefinedNameTooDeep {
                    name: area.to_owned(),
                },
            ));
            return Ok(());
        }
        let Some(definition) = self.defined_name_definition(default_sheet, area) else {
            out.areas.push(unresolved(
                area,
                RangeProblem::UnknownDefinedName {
                    name: area.to_owned(),
                },
            ));
            return Ok(());
        };
        followed.push(area.to_owned());
        // A name's definition may itself name several areas, so it goes back through the splitter
        // rather than straight to the area parser.
        match ReferenceAreas::parse(&definition) {
            Ok(areas) => {
                for inner in areas {
                    self.resolve_area(default_sheet, inner, followed, out)?;
                }
            }
            Err(problem) => out
                .areas
                .push(unresolved(area, RangeProblem::Malformed(problem))),
        }
        followed.pop();
        Ok(())
    }

    /// The definition of the name `wanted`, scoped to `sheet_index` if there is one there and to the
    /// workbook otherwise — the resolution order §18.2.6 states.
    fn defined_name_definition(&self, sheet_index: usize, wanted: &str) -> Option<String> {
        let mut workbook_scoped = None;
        for entry in &self.names {
            if entry.name != wanted {
                continue;
            }
            match entry.scope {
                DefinedNameScope::Sheet { index, .. } if index == sheet_index => {
                    return Some(entry.definition.clone());
                }
                DefinedNameScope::Workbook => workbook_scoped = Some(entry.definition.clone()),
                _ => {}
            }
        }
        workbook_scoped
    }

    /// Walks `range` on the tab at `sheet_index`, appending one entry per populated cell.
    fn collect(
        &mut self,
        sheet_index: usize,
        area: &str,
        range: CellRange,
        out: &mut ResolvedRange,
    ) -> Result<(), XlsxError> {
        let bounds = range.normalized_bounds();
        let base = out.addressed_cells;
        out.addressed_cells = out.addressed_cells.saturating_add(bounds.cell_count());

        // The values are gathered first and finished second: gathering borrows the worksheet part
        // out of `self`, and turning a shared-string index into text needs `self` again for the
        // string table.
        let mut gathered: Vec<(u64, CellReference, PendingValue)> = Vec::new();
        {
            let Some(store) = self
                .sheet_part(sheet_index)?
                .and_then(WorksheetPart::sheet_data)
            else {
                out.areas.push(ResolvedArea {
                    reference: area.to_owned(),
                    sheet_index: Some(sheet_index),
                    bounds: Some(bounds),
                    problem: Some(RangeProblem::SheetHasNoCells { sheet_index }),
                });
                return Ok(());
            };
            gather(store, bounds, base, &mut gathered);
        }

        for (offset, reference, pending) in gathered {
            let value = match pending {
                PendingValue::Ready(value) => value,
                PendingValue::SharedString(index) => match self.shared_string(index)? {
                    Some(text) => RangeCellValue::Text(text),
                    None => RangeCellValue::Undecodable,
                },
            };
            out.cells.push(ResolvedRangeCell {
                offset,
                sheet_index,
                reference,
                value,
            });
        }
        out.areas.push(ResolvedArea {
            reference: area.to_owned(),
            sheet_index: Some(sheet_index),
            bounds: Some(bounds),
            problem: None,
        });
        Ok(())
    }

    /// The shared string at `index`, reading `xl/sharedStrings.xml` at most once per resolver.
    fn shared_string(&mut self, index: u32) -> Result<Option<String>, XlsxError> {
        if self.strings.is_none() {
            self.strings = Some(self.workbook.shared_strings()?);
        }
        let Some(Some(table)) = self.strings.as_ref() else {
            return Ok(None);
        };
        let Some(item) = table.item(index) else {
            return Ok(None);
        };
        Ok(Some(
            item.text().map_err(mjx_sml::SmlError::from)?.into_owned(),
        ))
    }

    /// The worksheet part behind the tab at `sheet_index`, parsed at most once per resolver.
    ///
    /// `None` for a tab with no cells — a chartsheet, a dialogsheet, or one whose relationship
    /// reaches no part.
    fn sheet_part(&mut self, sheet_index: usize) -> Result<Option<&WorksheetPart>, XlsxError> {
        if !self.sheets.iter().any(|(index, _)| *index == sheet_index) {
            let part = self.workbook.worksheet_markup(sheet_index)?;
            self.sheets.push((sheet_index, part));
        }
        Ok(self
            .sheets
            .iter()
            .find(|(index, _)| *index == sheet_index)
            .and_then(|(_, part)| part.as_ref()))
    }
}

/// A value that is decoded, or a shared-string index still waiting on the string table.
///
/// A shared-string index is never an *answer* — it means nothing without a second part to look it up
/// in — so it stays internal rather than becoming a [`RangeCellValue`] variant a caller would have
/// to resolve itself.
enum PendingValue {
    Ready(RangeCellValue),
    SharedString(u32),
}

/// An area that could not be resolved, with the reason.
fn unresolved(area: &str, problem: RangeProblem) -> ResolvedArea {
    ResolvedArea {
        reference: area.to_owned(),
        sheet_index: None,
        bounds: None,
        problem: Some(problem),
    }
}

/// Gathers every populated cell of `store` inside `bounds`, at its offset from `base`.
///
/// **This is where the bulk-data discipline lives.** The row walk is whichever of the two is
/// smaller: addressing the range's own rows by number when there are fewer of them than the sheet
/// has populated, and filtering the sheet's populated rows when the range spans more of the grid
/// than the sheet fills. A three-row range on a 300,000-cell sheet therefore costs three row
/// lookups, and a whole-column reference on a small sheet costs that sheet's rows — neither can cost
/// more than the other would.
fn gather(
    store: &SheetData,
    bounds: GridBounds,
    base: u64,
    out: &mut Vec<(u64, CellReference, PendingValue)>,
) {
    let columns = u64::from(bounds.last_column() - bounds.first_column()) + 1;
    let range_rows = u64::from(bounds.last_row() - bounds.first_row()) + 1;
    let populated_rows = store.row_count() as u64;
    if range_rows <= populated_rows {
        for row_index in bounds.first_row()..=bounds.last_row() {
            let Some(row) = store.row(row_index.saturating_add(1)) else {
                continue;
            };
            gather_row(&row, bounds, columns, base, out);
        }
    } else {
        for row in store.rows() {
            gather_row(&row, bounds, columns, base, out);
        }
    }
}

/// Appends every cell of `row` that falls inside `bounds`, at its offset within the reference.
fn gather_row(
    row: &Row<'_>,
    bounds: GridBounds,
    columns: u64,
    base: u64,
    out: &mut Vec<(u64, CellReference, PendingValue)>,
) {
    // A row that wrote no `r` is at no address, so no range reaches it — MJXOFF-95 holds it with a
    // number of `0` and reports it as an anomaly rather than inferring one from its position.
    let Some(number) = row.number() else {
        return;
    };
    let Some(row_index) = number.checked_sub(1) else {
        return; // `r="0"` is outside the grid
    };
    if row_index < bounds.first_row() || row_index > bounds.last_row() {
        return;
    }
    let row_base = base + u64::from(row_index - bounds.first_row()) * columns;
    // The same choice the row walk makes, on the column axis.
    if columns <= row.cell_count() as u64 {
        for column in bounds.first_column()..=bounds.last_column() {
            let Some(cell) = row.cell(column) else {
                continue;
            };
            push_cell(&cell, bounds, row_base, out);
        }
    } else {
        for cell in row.cells() {
            let column = cell.reference().column();
            if column < bounds.first_column() || column > bounds.last_column() {
                continue;
            }
            push_cell(&cell, bounds, row_base, out);
        }
    }
}

/// Appends one cell at its offset, unless it is blank.
fn push_cell(
    cell: &Cell<'_>,
    bounds: GridBounds,
    row_base: u64,
    out: &mut Vec<(u64, CellReference, PendingValue)>,
) {
    let reference = cell.reference();
    let Some(value) = decode(cell) else {
        return;
    };
    let offset = row_base + u64::from(reference.column() - bounds.first_column());
    out.push((offset, reference, value));
}

/// What one cell holds, or `None` for a blank one.
fn decode(cell: &Cell<'_>) -> Option<PendingValue> {
    let undecodable_if_written = |cell: &Cell<'_>| {
        cell.raw_value()
            .map(|_| PendingValue::Ready(RangeCellValue::Undecodable))
    };
    match cell.cell_type() {
        CellType::SharedString => cell.shared_string_index().map(PendingValue::SharedString),
        CellType::InlineString => {
            let markup = cell.inline_string_markup()?;
            let Ok(string) = mjx_sml::InlineString::parse(markup) else {
                return Some(PendingValue::Ready(RangeCellValue::Undecodable));
            };
            Some(PendingValue::Ready(match string.item().text() {
                Ok(text) => RangeCellValue::Text(text.into_owned()),
                Err(_) => RangeCellValue::Undecodable,
            }))
        }
        CellType::Boolean => match cell.boolean() {
            Some(value) => Some(PendingValue::Ready(RangeCellValue::Boolean(value))),
            None => undecodable_if_written(cell),
        },
        CellType::Number => match cell.number() {
            Some(value) => Some(PendingValue::Ready(RangeCellValue::Number(value))),
            None => undecodable_if_written(cell),
        },
        CellType::Error => match cell.value() {
            Ok(Some(text)) => Some(PendingValue::Ready(RangeCellValue::Error(
                text.into_owned(),
            ))),
            Ok(None) => None,
            Err(_) => Some(PendingValue::Ready(RangeCellValue::Undecodable)),
        },
        CellType::FormulaString => match cell.value() {
            Ok(Some(text)) => Some(PendingValue::Ready(RangeCellValue::Text(text.into_owned()))),
            Ok(None) => None,
            Err(_) => Some(PendingValue::Ready(RangeCellValue::Undecodable)),
        },
    }
}
