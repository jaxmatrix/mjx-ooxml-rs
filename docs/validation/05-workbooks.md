# Workbooks — the checks

Six areas, `V-XLSX-01` … `V-XLSX-06`. `docs/validation/00-method.md` states the id scheme, the risk
levels and the result convention; `docs/validation/01-index.md` binds each area to its artefact;
`docs/validation/02-risk-order.md` says which of these to do first and why.

**Nothing on this page is marked** — including the two columns of the table `V-XLSX-02` carries in.

## How these six areas were derived

Not from the twenty Excel tickets. From two things that shipped:

* **The facade's own module structure.** `crates/mjx-ooxml/src/workbook/` is fifteen files. Six of
  them can be *authored* and therefore validated: `cells.rs` + `sheets.rs` (`V-XLSX-01`),
  `styles.rs` (`V-XLSX-02`), `grid.rs` (`V-XLSX-03`), `charts.rs` (`V-XLSX-04`), `drawings.rs`
  (`V-XLSX-05`), `comments.rs` + `hyperlinks.rs` (`V-XLSX-06`). The other five —
  `features.rs`, `names.rs`, `preserved.rs`, `print.rs` and `tables.rs` — carry **readers and
  removers and no authoring call at all**, which is why the index page records them as deliberately
  not covered: an entry describing an action the API cannot perform is worse than no entry.
* **`crates/mjx-xlsx/docs/guide/deliberate_limitations.md`**, whose *Gaps rather than decisions*
  table names exactly the same absences from the other direction — *writing a formula into a cell*,
  *removing a sheet*, *authoring a theme part*.

Where the two disagree the surface wins, because a page can be stale and a `pub fn` cannot.

## Coverage, against the limitations page

Excel's page is shaped differently from the other two: it has no *Built, not yet verified* list, so
the mapping is against its **decisions** and its **gaps**.

| Area | Risk | The limitations page says | What that means here |
|---|---|---|---|
| `V-XLSX-01` | low | *Gap* — **writing a formula into a cell** has no owner | `V-XLSX-01.4` reads formulas from a fixture and authors none |
| `V-XLSX-01` | low | *Gap* — **removing a sheet** has no owner; *gap* — **authoring a theme part** has none either | Recorded, not checked. `Workbook::blank` writes no `xl/theme/theme1.xml`, and `V-XLSX-01.3` is where Excel is asked whether that is acceptable |
| `V-XLSX-02` | high | *Decision* — nothing is evaluated | The two style layers are pure markup, and `V-XLSX-02.1` is the pass's whole Excel deliverable |
| `V-XLSX-03` | medium | *Decision* — **a filter, a sort and a validation rule are recorded, never applied**; **nothing is repaired on read** | Non-goals. `V-XLSX-03.3` checks that a hidden row is the file's own `row@hidden` and not something a filter did |
| `V-XLSX-04` | medium | *Decision* — a chart's workbook is not this crate's; the writer is `mjx-sml`'s | R4's third host. `V-XLSX-04` reads a **live range** rather than an embedded workbook, which is the case neither other format has |
| `V-XLSX-05` | medium | — | Plain modelled markup, with three anchor modes |
| `V-XLSX-06` | low | *Decision* — **no I/O, ever**, so an external hyperlink is never fetched | Non-goal; the check is that the target survives, not that it resolves |
| every area | — | **184 of `sml.xsd`'s 367 complex types are preserved rather than modelled** | Nine clusters with per-cluster reasons. **A documented gap is not a validation failure**; nothing on that table is a defect to file |
| `V-XLSX-01`, `V-XLSX-02` | — | *Decision* — **a conditional-formatting rule is reported, never resolved** | Non-goal. `V-XLSX-02.5` reads the rules' priority order and never asks whether a condition is true |

## `V-XLSX-01` · `cell-values` — cell values of every kind, shared strings and sheet tabs

Risk **low**. Shipped by `MJXOFF-95` (the cell store), `MJXOFF-97` (shared strings), `MJXOFF-100`
(the workbook part) and `MJXOFF-112` (`Workbook::blank`).

#### V-XLSX-01.1 — a workbook authored from nothing, opened in Excel

- **Risk** high — the fifth of Excel's five risk areas, and *the format is the least forgiving of the three about structural detail*.
- **Shipped by** `MJXOFF-112`.
- **Artefact** `v-xlsx-01-authored.xlsx`
- **Object** the whole package: `xl/workbook.xml`, one worksheet per sheet, `xl/sharedStrings.xml`, `xl/styles.xml` and both `docProps` parts — and **no** `xl/theme/theme1.xml`, because nothing here authors one.
- **Action** open it in Excel.
- **Expect** **no repair prompt, and no "we found a problem with some content"**. This is the single question the Excel half of the pass exists for: everything else assumes Excel is content with the container.
  Calls: `Workbook::blank` · `Workbook.blank` · `Workbook.blank`
  Calls: `Workbook::validate` · `Workbook.validate` · `Workbook.validate`
  Result: — · — · — · —

#### V-XLSX-01.2 — every value kind, in one sheet

- **Risk** low.
- **Shipped by** `MJXOFF-95` and `MJXOFF-97`.
- **Artefact** `v-xlsx-01-authored.xlsx`
- **Object** on the *Summary* sheet: shared strings in A1:C1 and A2:A4; numbers in B2:C4; `B6` a **boolean** (`t="b"`, false); `B7` an **error** (`t="e"`, `#N/A`); `A8` an **inline** string (`t="inlineStr"`, *not* shared); `B8` a **blank** cell with no value element at all.
- **Action** click each and read the formula bar and the cell's type.
- **Expect** `B6` shows **FALSE**; `B7` shows **#N/A** and Excel treats it as an error value, not as text; `A8` shows its text and does **not** appear in the shared-string table; `B8` is genuinely empty — `ISBLANK(B8)` is true. Numbers show as `1250000`, `0.125` and so on, unformatted.
  Calls: `Workbook::write_cells` · `Workbook.write_cells` · `Workbook.writeCells`
  Calls: `Workbook::read_range` · `Workbook.read_range` · `Workbook.readRange`
  Result: — · — · — · —

#### V-XLSX-01.3 — two sheet tabs, named and ordered

- **Risk** low.
- **Shipped by** `MJXOFF-100`.
- **Artefact** `v-xlsx-01-authored.xlsx`
- **Object** the two `<sheet>` entries: *Summary* (renamed from the blank workbook's default) and *Notes* (added).
- **Action** look at the tab bar.
- **Expect** two tabs, **Summary** then **Notes**, in that order, with *Summary* active. A sheet whose `r:id` and `sheetId` disagree is a repair, and the order in `xl/workbook.xml` is the tab order.
  Calls: `Workbook::rename_sheet` · `Workbook.rename_sheet` · `Workbook.renameSheet`
  Calls: `Workbook::add_sheet` · `Workbook.add_sheet` · `Workbook.addSheet`
  Calls: `Workbook::sheets` · `Workbook.sheets` · `Workbook.sheets`
  Result: — · — · — · —

#### V-XLSX-01.4 — shared and array formulas survive an edit to a cell they depend on

- **Risk** high — the third of Excel's five risk areas.
- **Shipped by** `MJXOFF-115`.
- **Artefact** `tests/fixtures/formulas.xlsx`
- **Object** the shared-formula group (a master `<f t="shared" ref="…" si="…">` and its followers), an array formula, and their cached `<v>` values. `B2` holds `=A2*2`; `A2` holds 1; the cached result is 2.
- **Action** set `A2` to `50` through this library, save, and open in Excel. Then look at `B2` **before** and **after** Excel recalculates.
- **Expect** on open, the shared group is intact — the master still carries the `ref` and the followers still carry only their `si` — and Excel recalculates without complaining. Note that this library leaves the cached `<v>` **exactly as it was** (`2`): blanking it destroys data in cells the caller never named, and setting `fullCalcOnLoad` writes into a part the caller did not ask to edit. **A stale cached value is a decision, not a defect** — what this check is really asking is whether Excel is happy to recalculate one.
  Calls: `Workbook::write_cells` · `Workbook.write_cells` · `Workbook.writeCells`
  Calls: `Workbook::read_sheet` · `Workbook.read_sheet` · `Workbook.readSheet`
  Result: — · — · — · —

#### V-XLSX-01.5 — the grid anomalies a file arrives with are held, not corrected

- **Risk** medium.
- **Shipped by** `MJXOFF-117`, with the reporting from `MJXOFF-102`.
- **Artefact** `tests/fixtures/worksheet_spine.xlsx`
- **Object** whatever `grid_anomalies` reports — a `dimension` that disagrees with the cells, a duplicate `row@r`, an overlapping merge.
- **Action** open the file in Excel and see what Excel makes of each.
- **Expect** Excel opens it. Each anomaly is **reported and none is corrected**, because correcting a file to match what this library expects is how a fidelity library loses the argument it exists to win. What Excel does with each is the answer this check records; a disagreement is information, not a defect.
  Calls: `Workbook::grid_anomalies` · `Workbook.grid_anomalies` · `Workbook.gridAnomalies`
  Result: — · — · — · —

## `V-XLSX-02` · `cell-formats` — the two style layers: direct `cellXfs` over a named `cellStyleXfs`

Risk **high**. Shipped by `MJXOFF-105` (the resource tables) and `MJXOFF-108` (the `xf` indirection,
number formats and effective cell formatting). **This is the highest-risk area in the Excel half.**

#### V-XLSX-02.1 — the inherited comparison table, filled in

- **Risk** high — the first and second of Excel's five risk areas, in one exercise.
- **Shipped by** `MJXOFF-108`.
- **Artefact** `tests/fixtures/effective_cell_format.xlsx`
- **Object** `docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md` — **28 rows**, written by `MJXOFF-108`, each an answer this workspace gives today, each with its basis in ECMA-376 Part 1, and each with an **Excel says** and a **Verdict** column that are **empty and unmarked**.
- **Action** open the fixture in real Microsoft Excel and fill those two columns in, row by row. That table is the deliverable; do **not** re-word, re-number or re-mark it here.
- **Expect** the fixture is built so that reading the wrong layer gives a **visibly wrong** answer: `cellXfs[1]` and `cellStyleXfs[1]` state a different value for **every one of the six aspects**; records 1–4 name the same `xfId` and the same four resource indices and differ **only** in their `applyX` attributes (false, absent, true, mixed), which is the only arrangement that can tell the three states apart; record 5 sits on a `cellStyleXfs` record that suppresses `applyFont` itself; columns B–D carry `col@style="7"`; row 2 writes `customFormat="1" s="6"` and row 3 writes `s="6"` with **no** `customFormat`. **An empty cell is the honest record of work that has not happened; a column of "unverified" is a completed-looking table that says nothing.**
  Calls: `Workbook::effective_cell_format` · `Workbook.effective_cell_format` · `Workbook.effectiveCellFormat`
  Result: — · — · — · —

#### V-XLSX-02.2 — the two layers, in a file this library authored

- **Risk** high.
- **Shipped by** `MJXOFF-108` over `MJXOFF-105`.
- **Artefact** `v-xlsx-02-authored.xlsx`
- **Object** A1, B1 and C1 on the *Formats* sheet. The direct `cellXfs` record names `xfId="1"` beneath it and overrides all three of font, fill and border: the **lower** layer is Times New Roman 13 pt italic on fill `445566` with a thick right border; the **upper** layer is Arial 12 pt bold on fill `112233` with a thin left border.
- **Action** click A1 and read Home → Font, Fill Colour and Borders; then Cell Styles to see the named style beneath it.
- **Expect** **Arial, 12 pt, bold**, fill **`112233`**, a **thin left** border — the upper layer wins every aspect it claims with `applyX="true"`. The lower layer's Times New Roman, `445566` and thick right border must **not** show.
  Calls: `Workbook::append_cell_format` · `Workbook.append_cell_format` · `Workbook.appendCellFormat`
  Calls: `Workbook::set_cell_style` · `Workbook.set_cell_style` · `Workbook.setCellStyle`
  Calls: `Workbook::append_font` · `Workbook.append_font` · `Workbook.appendFont`
  Result: — · — · — · —

#### V-XLSX-02.3 — a number format code, rendered

- **Risk** high — the first of Excel's five risk areas.
- **Shipped by** `MJXOFF-108`.
- **Artefact** `tests/fixtures/effective_cell_format.xlsx`
- **Object** the cells whose `numFmtId` resolves through a built-in code and the ones whose code is spelled out in `numFmts`.
- **Action** compare the code `format_code` reports against Home → Number → Format Cells → Custom for the same cell, and against what the cell **renders**.
- **Expect** the code matches. There is **no number-format engine here** — this library reports the code in force and never applies it — so a difference between the reported code and the rendered text is a finding about resolution, not about formatting.
  Calls: `Workbook::effective_cell_format` · `Workbook.effective_cell_format` · `Workbook.effectiveCellFormat`
  Result: — · — · — · —

#### V-XLSX-02.4 — a merged range's format comes from its anchor

- **Risk** medium.
- **Shipped by** `MJXOFF-108` with `MJXOFF-117`'s merge grid.
- **Artefact** `v-xlsx-03-authored.xlsx`
- **Object** the merge `A6:C6`, and what `effective_merged_cell_format` reports for a cell inside it.
- **Action** click anywhere in the merged region and read Home → Font and Fill.
- **Expect** Excel shows the **anchor's** format across the whole region, and `effective_merged_cell_format` reports the same one. Asking a covered cell for its own format answers what that cell states, which is a different question and must not be confused with this one.
  Calls: `Workbook::effective_merged_cell_format` · `Workbook.effective_merged_cell_format` · `Workbook.effectiveMergedCellFormat`
  Calls: `Workbook::merged_range_containing` · `Workbook.merged_range_containing` · `Workbook.mergedRangeContaining`
  Result: — · — · — · —

#### V-XLSX-02.5 — conditional-formatting rules, in priority order

- **Risk** high — the fourth of Excel's five risk areas.
- **Shipped by** `MJXOFF-120`.
- **Artefact** `tests/fixtures/conditional_formatting.xlsx`
- **Object** the `cfRule` blocks, their `@priority` values and their `@stopIfTrue` flags.
- **Action** compare the order `conditional_formatting_ranges` reports against Home → Conditional Formatting → Manage Rules, which lists them in the order Excel applies them.
- **Expect** the same order. Note the boundary: this library reports **which rules apply to a cell**, merged across blocks and in priority order, and reports the `dxf` each would impose **beside** the base format, never folded into it. It never answers **whether a condition is true**, and `stopIfTrue` is reported as a *position in the chain* rather than applied as a truncation, because applying it means knowing which earlier rule fired. **A rule that does not fire is not a defect.**
  Calls: `Workbook::conditional_formatting_ranges` · `Workbook.conditional_formatting_ranges` · `Workbook.conditionalFormattingRanges`
  Calls: `Workbook::conditional_formatting_rule_count` · `Workbook.conditional_formatting_rule_count` · `Workbook.conditionalFormattingRuleCount`
  Result: — · — · — · —

#### V-XLSX-02.6 — a pattern fill Excel accepts

- **Risk** high — a **defect found by `MJXOFF-122` and not fixed**, recorded here so the pass sees it deliberately.
- **Shipped by** `MJXOFF-105`.
- **Artefact** `v-xlsx-02-authored.xlsx`
- **Object** the two pattern fills, `445566` and `112233`, and the `@rgb` attribute each writes.
- **Action** open the file in Excel and look at whether the two fills draw at all.
- **Expect** `PatternFillSpec::solid` writes a **ten-character** `@rgb` where `sml.xsd` types it as an eight-character `ST_UnsignedIntHex`. **Both validators passed it**, which is why it reached this page rather than a test. If Excel refuses the fill, or draws it in the wrong colour, that is this — not a resolution problem — and the fix belongs in `mjx-sml`'s writer, not in the pass.
  Calls: `Workbook::append_pattern_fill` · `Workbook.append_pattern_fill` · `Workbook.appendPatternFill`
  Result: — · — · — · —

## `V-XLSX-03` · `grid` — merged ranges, row heights, column widths, hiding and outline levels

Risk **medium**. Shipped by `MJXOFF-117`.

#### V-XLSX-03.1 — a merged range, and two column-width runs

- **Risk** low.
- **Shipped by** `MJXOFF-117`.
- **Artefact** `v-xlsx-03-authored.xlsx`
- **Object** `<mergeCell ref="A6:C6"/>`; `<col min="1" max="1" width="24" customWidth="true"/>` and `<col min="2" max="3" width="14" customWidth="true"/>`.
- **Action** select A6, then drag-select the column headers A, B and C and read each width.
- **Expect** A6:C6 is one merged cell. Column **A is 24** characters wide; columns **B and C are 14**. A `<col>` run written per column rather than as a range would give the same picture and a different file, so compare the widths, not the count.
  Calls: `Workbook::merge_cells` · `Workbook.merge_cells` · `Workbook.mergeCells`
  Calls: `Workbook::set_column_width` · `Workbook.set_column_width` · `Workbook.setColumnWidth`
  Result: — · — · — · —

#### V-XLSX-03.2 — a custom row height, and an outline group

- **Risk** medium.
- **Shipped by** `MJXOFF-117`.
- **Artefact** `v-xlsx-03-authored.xlsx`
- **Object** row 1 at `ht="30"` with `customHeight`; rows 3 and 4 at `outlineLevel="1"`.
- **Action** read row 1's height; then look for the outline bracket beside rows 3 and 4 and collapse it.
- **Expect** row 1 is **30 points** tall. Rows 3 and 4 form **one outline group at level 1**, and Excel draws the collapse control for it. An outline level written without the surrounding `sheetFormatPr@outlineLevelRow` still groups, which is what this check confirms.
  Calls: `Workbook::set_row_height` · `Workbook.set_row_height` · `Workbook.setRowHeight`
  Calls: `Workbook::set_row_outline_level` · `Workbook.set_row_outline_level` · `Workbook.setRowOutlineLevel`
  Result: — · — · — · —

#### V-XLSX-03.3 — a hidden row and a hidden column, hidden by the file and not by a filter

- **Risk** medium.
- **Shipped by** `MJXOFF-117`.
- **Artefact** `v-xlsx-03-authored.xlsx`
- **Object** row 5 with `hidden="true"`, and `<col min="6" max="6" hidden="true"/>`.
- **Action** look for the gap in the row headers and in the column headers; then Home → Format → Unhide.
- **Expect** row **5** and column **F** are hidden, and unhiding shows them. Row visibility is `row@hidden` — **the file's own statement** — and this library never writes it because a filter was added: *setting an autofilter hides no row, and removing one unhides none*. A hidden row here is hidden because somebody asked for it.
  Calls: `Workbook::set_row_hidden` · `Workbook.set_row_hidden` · `Workbook.setRowHidden`
  Calls: `Workbook::set_column_hidden` · `Workbook.set_column_hidden` · `Workbook.setColumnHidden`
  Result: — · — · — · —

## `V-XLSX-04` · `charts` — range charts anchored to a worksheet

Risk **medium**. Shipped by `MJXOFF-111`, on `MJXOFF-107`'s anchor index and `MJXOFF-99`'s writer.

#### V-XLSX-04.1 — a chart that reads a live range, not an embedded workbook

- **Risk** high — **R4** in the host where it works differently.
- **Shipped by** `MJXOFF-111`.
- **Artefact** `v-xlsx-04-authored.xlsx`
- **Object** the chart on the *Charts* sheet. Its category reference is `Charts!$A$2:$A$4`, its one series reads `Charts!$B$2:$B$4` and takes its name from `Charts!$B$1`.
- **Action** click the chart and read the formula bar's `SERIES(…)`; then change `B3` and watch the chart.
- **Expect** the `SERIES` formula names those three ranges. Changing `B3` **redraws the chart**, because it reads the sheet rather than an embedded workbook — the only one of the three hosts where that is true.
  Calls: `Workbook::add_range_chart` · `Workbook.add_range_chart` · `Workbook.addRangeChart`
  Calls: `Workbook::chart_series_references` · `Workbook.chart_series_references` · `Workbook.chartSeriesReferences`
  Result: — · — · — · —

#### V-XLSX-04.2 — a cached series that has gone stale against its cells

- **Risk** medium.
- **Shipped by** `MJXOFF-111`.
- **Artefact** `tests/fixtures/chart_stale_cache.xlsx`
- **Object** a chart whose `c:numCache` disagrees with the cells its `c:f` names.
- **Action** open it in Excel and see which set of numbers is drawn.
- **Expect** Excel redraws from the **cells**, and the cache is only what a consumer that cannot read the sheet would use. `chart_series_freshness` reports the disagreement and **nothing corrects it**; `refresh_chart_cache_from_cells` is the deliberate call that does.
  Calls: `Workbook::chart_series_freshness` · `Workbook.chart_series_freshness` · `Workbook.chartSeriesFreshness`
  Calls: `Workbook::refresh_chart_cache_from_cells` · `Workbook.refresh_chart_cache_from_cells` · `Workbook.refreshChartCacheFromCells`
  Result: — · — · — · —

#### V-XLSX-04.3 — title, axis title and legend on a worksheet chart

- **Risk** low.
- **Shipped by** `MJXOFF-111`.
- **Artefact** `v-xlsx-04-authored.xlsx`
- **Object** the chart's title *Revenue by region*, its category-axis title *Region*, and its legend.
- **Action** read each off the rendered chart.
- **Expect** all three, with the legend **below** the plot. These go through `mjx_chart::chart_ops` — the same body of code all three hosts call — so a difference between this and `V-PPTX-04.2` means a host has grown a local chart path.
  Calls: `Workbook::set_chart_title` · `Workbook.set_chart_title` · `Workbook.setChartTitle`
  Calls: `Workbook::set_chart_legend` · `Workbook.set_chart_legend` · `Workbook.setChartLegend`
  Calls: `Workbook::set_chart_axis_title` · `Workbook.set_chart_axis_title` · `Workbook.setChartAxisTitle`
  Result: — · — · — · —

## `V-XLSX-05` · `drawings` — anchored pictures, and how they behave when the grid moves

Risk **medium**. Shipped by `MJXOFF-107`.

#### V-XLSX-05.1 — a two-cell anchor and a one-cell anchor

- **Risk** medium.
- **Shipped by** `MJXOFF-107`.
- **Artefact** `v-xlsx-05-authored.xlsx`
- **Object** two pictures on the *Drawings* sheet: a `xdr:twoCellAnchor` from column 1, row 4 to column 8, row 7 named *Two-cell anchored*; and a `xdr:oneCellAnchor` at column 10, row 4, sized **2 × 1 in**, named *One-cell anchored*.
- **Action** click each and read Picture Format → Size and → Properties.
- **Expect** the two-cell one reports **Move and size with cells**; the one-cell one reports **Move but don't size with cells** and measures 2" by 1". Its extent is stated in EMU on the drawing rather than derived from the grid, so a wrong unit conversion shows as a wrong size here and nowhere else.
  Calls: `Workbook::add_two_cell_anchored_picture` · `Workbook.add_two_cell_anchored_picture` · `Workbook.addTwoCellAnchoredPicture`
  Calls: `Workbook::add_one_cell_anchored_picture` · `Workbook.add_one_cell_anchored_picture` · `Workbook.addOneCellAnchoredPicture`
  Result: — · — · — · —

#### V-XLSX-05.2 — inserting a row above an anchored picture moves it

- **Risk** medium.
- **Shipped by** `MJXOFF-107`.
- **Artefact** `v-xlsx-05-authored.xlsx`
- **Object** both anchors, after `insert_rows_into_drawing` has shifted them.
- **Action** insert two rows above row 4 through this library, save, and open in Excel. Then do the same insertion **in Excel** on the original and compare where the pictures land.
- **Expect** the two agree. This library rewrites the anchors; Excel rewrites them its own way; a disagreement about which pictures move is exactly what the resizing behaviour is *for*, and no test in this repository can see it.
  Calls: `Workbook::insert_rows_into_drawing` · `Workbook.insert_rows_into_drawing` · `Workbook.insertRowsIntoDrawing`
  Calls: `Workbook::sheet_anchor_bounds` · `Workbook.sheet_anchor_bounds` · `Workbook.sheetAnchorBounds`
  Result: — · — · — · —

#### V-XLSX-05.3 — a worksheet drawing a producer wrote

- **Risk** medium.
- **Shipped by** `MJXOFF-107`.
- **Artefact** `tests/fixtures/worksheet_drawings.xlsx`
- **Object** the three anchor modes as they appear in a file this project did not author from `blank`.
- **Action** open it, read every anchor back, save, and reopen in Excel.
- **Expect** every drawing lands where it did before. `sheet_drawing` addresses objects by their position in the drawing part; an absolute anchor in particular has no cell to fall back on, so a mis-read there moves the object to the origin.
  Calls: `Workbook::sheet_drawing` · `Workbook.sheet_drawing` · `Workbook.sheetDrawing`
  Calls: `Workbook::add_absolute_anchored_picture` · `Workbook.add_absolute_anchored_picture` · `Workbook.addAbsoluteAnchoredPicture`
  Result: — · — · — · —

## `V-XLSX-06` · `comments-and-links` — cell comments and cell hyperlinks

Risk **low**. Shipped by `MJXOFF-114` (comments and their VML backing) and `MJXOFF-127`
(hyperlinks and the object-anchor vocabulary).

#### V-XLSX-06.1 — two cell comments, with their VML backing

- **Risk** medium.
- **Shipped by** `MJXOFF-114`.
- **Artefact** `v-xlsx-06-authored.xlsx`
- **Object** the comments on **B2** and **C4**, `xl/comments1.xml`, and the legacy VML drawing that gives each its box.
- **Action** hover B2, then C4; then Review → Show All Comments.
- **Expect** B2 reads *"Confirm the North America figure before publishing."* and C4 reads *"Growth restated in March."*, both attributed to **Reviewer**, each drawn in its own box. A comment without its VML shape is a comment Excel does not draw — the two parts have to agree, and only a renderer can say whether they do.
  Calls: `Workbook::add_cell_comment` · `Workbook.add_cell_comment` · `Workbook.addCellComment`
  Calls: `Workbook::sheet_comments` · `Workbook.sheet_comments` · `Workbook.sheetComments`
  Calls: `Workbook::sheet_vml_part_bytes` · `Workbook.sheet_vml_part_bytes` · `Workbook.sheetVmlPartBytes`
  Result: — · — · — · —

#### V-XLSX-06.2 — a cell hyperlink whose target is never fetched

- **Risk** low.
- **Shipped by** `MJXOFF-127`.
- **Artefact** `v-xlsx-06-authored.xlsx`
- **Object** `<hyperlink ref="A1" r:id="…"/>` and the external relationship behind it.
- **Action** hover A1 and click it.
- **Expect** the target is `https://example.com/investors`. **No I/O, ever** — the URL is stored and never fetched — so the only claim is that the relationship survives and Excel can follow it.
  Calls: `Workbook::set_cell_hyperlink_url` · `Workbook.set_cell_hyperlink_url` · `Workbook.setCellHyperlinkUrl`
  Calls: `Workbook::cell_hyperlink` · `Workbook.cell_hyperlink` · `Workbook.cellHyperlink`
  Result: — · — · — · —

#### V-XLSX-06.3 — a third-party comment, and a legacy form control

- **Risk** medium.
- **Shipped by** `MJXOFF-114`.
- **Artefact** `tests/fixtures/comments_third_party.xlsx`, `tests/fixtures/legacy_form_control.xlsx`
- **Object** a comment written by another producer, and a form control whose `vml_shape_id_for_form_control` hop names a `v:shape@id`.
- **Action** open both in Excel; round-trip each through this library and reopen.
- **Expect** both open unchanged, and the identifier hop resolves. This is the Excel half of the same question `V-PPTX-02.13` asks for PowerPoint, and like that one it has so far been asserted only against markup this project can read rather than markup Excel wrote.
  Calls: `Workbook::vml_shape_id_for_form_control` · `Workbook.vml_shape_id_for_form_control` · `Workbook.vmlShapeIdForFormControl`
  Calls: `Workbook::vml_shape_id_for_ole_object` · `Workbook.vml_shape_id_for_ole_object` · `Workbook.vmlShapeIdForOleObject`
  Result: — · — · — · —

#### V-XLSX-06.4 — the preserved half of `sml.xsd`, carried through an unrelated edit

- **Risk** medium — a **documented gap**, checked as fidelity rather than as a feature.
- **Shipped by** `MJXOFF-133`.
- **Artefact** `tests/fixtures/preserved_parts.xlsx`
- **Object** the pivot table and its cache, the external link, the connection, the query table and the XML map — **184 of `sml.xsd`'s 367 complex types**, in nine clusters, each preserved rather than modelled with a reason on the guide's own page.
- **Action** edit one cell through this library, save, and open in Excel. Then use the pivot table.
- **Expect** every preserved part comes back **byte for byte**, and the pivot table still refreshes in Excel. **Nothing on that table is a defect to file** — the check is that the file survives, not that the features are modelled.
  Calls: `Workbook::preserved_parts` · `Workbook.preserved_parts` · `Workbook.preservedParts`
  Calls: `Workbook::pivot_tables` · `Workbook.pivot_tables` · `Workbook.pivotTables`
  Result: — · — · — · —

#### V-XLSX-06.5 — the walkthrough workbook, in three languages

- **Risk** medium.
- **Shipped by** `MJXOFF-137`, projected by the two bindings.
- **Artefact** `crates/mjx-ooxml/examples/build_a_workbook.rs` — `cargo run -p mjx-ooxml --example build_a_workbook`, and the Python and Node copies beside it.
- **Object** the workbook each of the three writes.
- **Action** open one in real Excel and read it against what the walkthrough describes.
- **Expect** it renders as the walkthrough says, with no repair prompt. **Note for the reviewer:** `CellFormatSpec` has three different shapes across the three languages — a known inconsistency `MJXOFF-122` recorded and did not fix — so a *style* written through Python or TypeScript is the one place the three walkthroughs are least likely to agree.
  Calls: `Workbook::save` · `Workbook.save` · `Workbook.save`
  Result: — · — · — · —
