# Changelog

All notable changes to **mjx-ooxml-rs** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning

The project is pre-release and uses `v0.0.x`: the patch number is incremented each development
iteration until the first milestone. Milestones then advance the minor version:

- **`v0.1`** — PowerPoint (`.pptx`) complete
- **`v0.2`** — Word (`.docx`) complete
- **`v0.3`** — Excel (`.xlsx`) complete

Further milestones (rendering, bindings, …) are defined as that work is scheduled. The public API is
**not** stable until `v0.1`.

## [Unreleased — 0.1.0]

`v0.1` is where the public API stops being free to change. The milestone ships when the PowerPoint
slice is complete; until then the working versions stay `0.0.x` and this section accumulates every
break made on the way, so the migration note for `0.1.0` is written as the breaks happen rather than
reconstructed afterwards.

### Breaking changes

| Was | Is | Why |
|-----|----|-----|
| `Presentation::cell_span` → `(columns, rows)` | → `(rows, columns)` | `table_dimensions` answers `(rows, columns)`, `merged_cell_anchor` answers `(row, column)`, and every cell method takes `(row, column)`. Two same-typed `usize`s are read as a habit, not as a signature. |
| `mjx_dml::BlipFill`, `BlipFillMode` | `PictureFill`, `PictureFillMode` | `blip` is ECMA's abbreviation for "binary large image or picture" and nothing else's. This crate already expanded `a:buBlip` to `BulletPicture`. |
| `mjx_dml::Fill::Blip`, `FillSpec::Blip` | `Fill::Picture`, `FillSpec::Picture` | Same token, same expansion. Office's own name for it is "Picture fill". |
| `mjx_dml::StyleMatrixReference::idx` | `index` | An abbreviation named after the `@idx` attribute; the docs already called it an index. |
| `mjx_chart::DataLabelSpec::show_*` (7 fields) | `shows_*` | The struct a caller reads (`DataLabelSettings`) already said `shows_*`; the two differed by one letter. `TrendlineSpec` / `ChartTrendlineData` agree on all nine of theirs. |
| `mjx_chart::ErrorBarSpec::plus`, `minus` | `plus_values`, `minus_values` | Matches `ChartErrorBarData` and `ErrorBars::plus_values()`; `plus` alone did not say plus *what*. |
| `mjx_pptx::ChartErrorBarData::has_no_end_cap` | `no_end_cap` | Matches `ErrorBarSpec`, the struct that writes the same `c:noEndCap`. |
| `mjx_pptx::PptxError::PictureHasNoBlipFill` | `PictureHasNoImage` | Drops the token, and says what the caller can act on. |
| `mjx_pptx::Presentation::activex_binary_bytes` | `activex_state_bytes` | Reads exactly what `set_activex_state` writes; the pair named one artefact two ways. |
| `mjx_pptx::PptxError` was `#[non_exhaustive]` | it is not | A `#[non_exhaustive]` enum forces a wildcard arm on every downstream `match`, which is exactly what would let a new failure mode be silently filed under a catch-all. `mjx_ooxml::Error`'s classification is deliberately exhaustive: adding a variant now fails the build until someone decides which of the eleven `ErrorCode`s it belongs to. |
| `delete_chart_data_labels`, `Axis::is_deleted`, `DataLabels::delete_all`, `auto_title_deleted` (12 public identifiers) | `suppress_chart_data_labels`, `is_suppressed`, `suppress_all`, `auto_title_suppressed` | `delete_*` wrote a `c:delete` (*draw nothing here*) and sat beside `remove_*`, which removes the element (*say nothing here*). Two operations, two near-synonyms, no way to tell them apart from the method list. `delete` was the spec element's own name; a public identifier that needs the spec open to be read is the thing the convention forbids. The wire token is unchanged and still named in every item's docs. |
| `mjx_sml::SmlError::SheetDataTooLarge` | `PackedStoreTooLarge` | There are two packed stores in `mjx-sml` now — the cell store and the shared-string table — over one shared byte arena, and the variant either of them raises said "the cell store's byte space" in its message. A name and a message that are true of one of two callers is the kind of small lie that survives into a user's terminal. |
| `mjx_docx::PageOrientation` (hand-written, MJXOFF-98) | `mjx_docx::PageOrientation` (re-export of `mjx_ooxml_types::wordprocessingml::PageOrientation`) | A duplicate of the generated enum, caught in MJXOFF-109's own pre-dispatch review — "consume, do not re-create" is the generator's whole reason to exist. `PageOrientation::to_wire(self) -> Option<&'static str>` (`None` for `Portrait`, the schema default) is **removed**: the generated type's own `to_wire(self) -> &'static str` always returns a token, and the "omit the attribute for `Portrait`" convenience now lives in `SectionProperties`'s writer (`crate::page::orientation_wire_value`, crate-private), not as a method on the value type. |
| `mjx_docx::TableStyleOverrideContent::TableProperties`/`TableRowProperties`/`TableCellProperties`, and the same three `StyleDefinitionContent` variants | inner type `Unmodeled` → `TableProperties`/`RowProperties`/`CellProperties` | These variants had no public accessor before MJXOFF-119 (a value of either enum was unreachable from outside the crate), so this is breaking only in the formal sense of a public enum's variant shape changing, never in practice. |
| `mjx_sml::ConditionalFormattingFormula` | `mjx_sml::FormulaElement` (module `mjx_sml::formula::element`) | MJXOFF-123. `sml.xsd` hangs three elements off `ST_Formula` — `cfRule/formula`, `dataValidation/formula1` and `dataValidation/formula2` — whose content model, escaping rules and no-evaluation contract are identical, so the type carries its own local name and there is one implementation rather than three. `new` gains a `local: &str` parameter for the same reason. The answer to a second consumer is one helper both can reach, not a copy with a different doc comment. |
| `mjx_docx::{RunPropertyContent, ParagraphMarkRunPropertyContent, ParagraphPropertyContent, StyleParagraphPropertyContent, SectionPropertyContent, NumberingPropertyContent}::Change`/`Inserted`/`Deleted`/`MovedFrom`/`MovedTo`, `FieldCharacterContent::NumberingChange` | inner type `Unmodeled` → the real revision type (`RunPropertiesChange`, `ParagraphMarkPropertiesChange`, `ParagraphPropertiesChange`, `TrackChangeMarker`, `SectionPropertiesChange`, `TrackChangeNumbering`) | MJXOFF-126. `ParagraphProperties::change()` already had a public accessor returning `Option<&Unmodeled>` — this one is a real, consumer-visible signature change, not only a formal one; every other listed variant had no accessor before this child, matching the row above. |
| `mjx_chart::EmbeddedWorkbook` (`new`, `Default`, `push_row`, `sheet_name`, `rows`, `for_chart_data`, `for_chart_space`, `to_package_bytes`), `mjx_chart::WorkbookCell` (`Blank`, `Number`, `Text`, `text`), `mjx_chart::CONTENT_TYPE_WORKBOOK_PACKAGE`, `mjx_chart::DEFAULT_SHEET_NAME` | **removed.** The two layout entry points become the free functions `mjx_chart::embedded_workbook_for_chart_data(&ChartData) -> Result<Vec<u8>, mjx_sml::SmlError>` and `mjx_chart::embedded_workbook_for_chart_space(&ChartSpace) -> Result<Vec<u8>, SmlError>`; the two constants become `mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE` and `mjx_sml::write::DEFAULT_SHEET_NAME`; the grid type has no replacement, because `mjx-chart` no longer holds a spreadsheet model | MJXOFF-99. `mjx-chart` carried a minimal SpreadsheetML writer because a chart embeds a real `.xlsx` and no SpreadsheetML crate existed — the workspace's one sanctioned duplicate, with a note in its own header naming this child as its executioner. `mjx-sml` (rank 2.1) now writes it and `mjx-chart` (2.2) reaches down to it; `mjx-chart → mjx-xlsx`, which the old note proposed, would have been an upward edge the layering forbids. What a chart's workbook *contains* did not change by a byte. |
| `mjx_pptx::PptxError` gains `Sml(mjx_sml::SmlError)` | — | The same removal: a chart's embedded workbook is now written by `mjx-sml`, so its failures reach a PresentationML caller as themselves rather than being flattened into `Opc`. `PptxError` is deliberately not `#[non_exhaustive]`, so this is a breaking addition; `mjx_ooxml::Error` classifies it through the same `sml_code` that `mjx-xlsx`'s errors go through, and no `ErrorCode` was added — nothing changes for either binding. |
| `mjx_chart::ChartLabelScope::Plot { plot_idx: usize }`, `Series { series_idx: usize }`, `Point { series_idx: usize, point_idx: u32 }` | `Plot { plot_index: u32 }`, `Series { series_index: u32 }`, `Point { series_index: u32, point_index: u32 }` | MJXOFF-118. These were the **last three public fields in the workspace spelled `*_idx`** — an abbreviation named after `c:idx`, which is exactly the case 0.0.69 already settled for `mjx_dml::StyleMatrixReference::idx`. The width goes with the name: this type crosses the facade to both bindings, and **both already published these three as `u32` and cast on the way in and out**, so the rename and the narrowing change nothing in Python or TypeScript and delete five casts (two of them `usize as u32`, which truncate rather than fail). |
| `mjx_pptx::ShapeInfo::index`, `mjx_pptx::LayoutInfo::index`, `mjx_pptx::LayoutInfo::master_index` — `usize` | `u32` | MJXOFF-118, finishing A9's own recorded loose end (*"better normalised once at v0.1"*). All three structs are re-exported **verbatim** by `mjx-ooxml` and by both bindings, which means they bypass `crates/mjx-ooxml/src/index.rs` — the one place the facade's `u32`/model `usize` width difference is meant to be crossed — and carried a host-dependent width into a foreign-function-facing type. Both bindings already read all three as `u32`; those casts are gone. A `mjx-pptx` caller feeding one of these back into a `Presentation` method converts once (`usize::try_from`), which `crates/mjx-pptx/src/index.rs` documents; a `mjx-ooxml` caller can now pass `ShapeInfo::index` straight to a `Deck` method, which was not possible before. |

Nothing else in the public surface changed name or shape. The sweep read all 1,561 public
identifiers of the eleven merged PowerPoint children; everything else either already followed the
convention or is a spec-sourced proper noun (`Srgb`, `ScRgb`, `OleObject`, the preset-shape names
whose digits are part of their identity).

The one candidate the sweep declined to settle on its own — whether `delete_chart_data_labels`
should become `suppress_*`, given that `delete` is the spec element's own name and runs through a
dozen coherent `mjx-chart` identifiers — was decided in favour of the rename and taken in 0.0.69,
whole rather than in part: renaming only the `mjx-pptx` method would have traded one inconsistency
for another. It is the row above. A grep in CI now keeps the spelling from drifting back.

## [0.0.152] - 2026-09-09

**Tables that split across pages, floating objects, and the text that flows around them
(MJXOFF-176, R21).**

The two features the plan names as consistently under-estimated, in one child because they interact:
a float anchored in a table cell wraps against the cell, and a table that splits across a page has to
re-run every wrap on the continuation. Both are also the easiest features in a Word renderer to ship
*unimplemented* — a table that fits on one page is laid out identically by an engine that cannot
split a table at all, and a float with `wp:wrapNone` changes no line — so every gate here is written
against the value that would be identical either way.

### Added

- **`mjx-docx` reads the body as a block tree.** `DocumentFormatting::blocks` interleaves paragraphs
  and tables in document order, which a paragraph list structurally cannot: a document whose tables
  were dropped paginates differently from the one its author wrote. Tables, rows and cells are
  resolved to plain numbers (`TableFormatting`, `RowFormatting`, `CellFormatting`), a cell's content
  is a block tree of its own — which is the whole of *nested tables to arbitrary depth* — and
  `ParagraphFormatting::drawings` resolves every `w:drawing`'s extent, anchoring, wrap mode and wrap
  polygon **without a `mjx-dml` type in the surface**, because the box model above deliberately does
  not depend on DrawingML.
- **Every paragraph in the document still resolves through one ladder, in one pass.** A cell's
  paragraphs are appended to the same flat list the body's live in, so `blocks()` indexes it and
  nothing is copied; the first `top_level_paragraph_count()` entries keep the order `w:sectPr` spans
  are numbered against, which a cell paragraph interleaved into them would have moved.
- **The table style's own tier now reaches a cell**, folded across the six conditional regions
  `table_regions::applicable_regions` already resolved. Its **run** properties change text
  *measurement*, so a bold heading row breaks its lines where Word breaks them rather than where an
  unstyled one would.
- **Four modules in `mjx-layout-docx`, and the split between them is the design.** `wrap` is geometry
  with no document in it — polygons, bands, the largest-side rule; `float` turns a `wp:anchor` into a
  rectangle in a column; `table` is the grid, the two layout algorithms and the slices a page break
  falls between; `block` is the one abstraction that lets a paragraph and a table go through the same
  paginator, which is why `w:keepNext` still works *across* a table.
- **Auto-fit is a constraint solve over content widths**, not a heuristic: every cell is measured at
  an unbounded measure and at one EMU, and the columns take the unique assignment that puts each the
  same fraction of the way from its minimum to its maximum. Fixed layout reads `w:tblGrid` and stops.
  `tests/a_table_grid_is_solved.rs` asserts the two produce **different numbers for the same
  content**, which an engine that returned the declared grid for both would fail.
- **`w:cantSplit` is an ordering constraint and `w:tblHeader` a repeating prefix**, and neither is
  special-cased inside the paginator: a row that may not split contributes **one** slice, so it moves
  whole through the same arithmetic `w:keepLines` already used, and a repeated heading adds a
  constant to a continuation's height.
- **Real wrap polygons.** `wp:wrapTight` and `wp:wrapThrough` take the polygon rather than the
  bounding box, exactly rather than by sampling, and the two elements genuinely differ: `tight` takes
  the outline's outermost crossings and `through` keeps every covered interval, so text enters a
  concavity in one and not the other.

### Fixed

- **`combine_run_tiers` never merged the table-style tier it was given.** Every caller passed the
  all-`None` identity until this child, so the missing `merge_under` was invisible;
  `Document::effective_cell_run_properties` has always merged it in that position, and two
  orchestrations of one ladder is precisely the shape `crates/mjx-docx/tests/residency.rs` exists to
  keep honest.

### Notes

- **A float's frame is resolved against the page's body height and never against the assembly's own
  reduced height.** That is not a detail: the footnote fixed point's two-assembly proof rests on *the
  body content placed is non-increasing in the reservation*, which would be false if a float anchored
  to the bottom of its column moved between the two assemblies. `crates/mjx-layout-docx/src/notes.rs`
  carries the amended argument in full.
- **An inline drawing's advance is not measured**, and it is declared rather than approximated:
  reserving one means a fixed advance on a composer run and `mjx_layout::TextRun` has no such field.
  A line carrying an inline drawing is measured as if the drawing were not on it. It is the same
  shape as R20's footnote-mark gap and belongs to the same later child.
- **Nothing here is parity with Word.** The provenance ledger prints on every run: **40 `SpecCode`,
  23 `DocumentedBehaviour`, 61 `EngineDerived`** over 124 rows. R21's own share is the weakest in the
  crate for a nameable reason — *there is no external standard for text wrapping at all*. The
  sharpest guess is what unit a `wp:wrapPolygon`'s coordinates are in: the schema says EMU and Word
  writes 21600ths of the extent, and both readings are implemented with the choice made per object.

## [0.0.151] - 2026-09-09

**Sections, columns, headers, footers and footnotes — and the fixed point between a note and the
body it takes space from (MJXOFF-175, R20).**

R19 flowed text down one column of one page shape. Real documents change page shape half way
through, run several columns, repeat furniture at the margins, and carry a **second flow that
competes with the first for vertical space**. The last of those is the whole difficulty: a footnote's
height decides how much body text fits on its page, and the body text decides which footnotes are on
it. `crates/mjx-layout-docx/src/notes.rs` resolves that circularity with a monotone reservation and
**proves it converges in at most two body assemblies**, with the argument written out in the module
rather than left to be rediscovered — an undocumented fixed-point loop is where a hang lives.

### Added

- **Four modules in `mjx-layout-docx`, one per subsystem.** `section` — the sheet, the margins, the
  columns, the break kinds and the blank page an `evenPage` break demands; `stream` — a header, a
  footer or a note laid out through the *same* flow engine the body uses, with its lines flattened so
  that a note can split across pages; `notes` — the body/footnote fixed point and the note area it
  produces; `numbering` — page, line and note counters, and the numeral systems they are written in.
- **Multi-column flow with balancing.** A page is a stack of *column groups*, one per section that
  shares the sheet, which is what a `continuous` break means. Balancing at such a break is a
  **bisection** for the shortest column height at which the remaining content still fits, not a
  division of the total by the column count: content is placed in whole lines, so the division's
  answer is routinely a hair short and a column a hair short spills a whole line — unbalancing the
  thing being balanced.
- **Headers and footers, with `w:titlePg` and `w:evenAndOddHeaders` resolved once in `mjx-docx`.**
  A page selects a stream rather than resolving one, and a header's height comes off the body's.
- **Footnotes with their own reflow**, Word's `separator` and `continuationSeparator` rules, and
  continuation across pages for a note taller than the page it is referenced from.
- **Endnotes as *flow*.** §17.11.3's `sectEnd`/`docEnd` are positions in the flow, not a second area,
  so an endnote's paragraphs are spliced into the body's own stream at the end of their scope —
  which is the whole difference between an endnote and a footnote, in one sentence.
- **Line numbers, computed *and drawn*.** A page number is displayed by a `PAGE` field and a
  footnote's mark by `w:footnoteRef`, both of which are R22's to render from the run stream; a line
  number is in no run stream at all, so a child that computed it and drew nothing would leave a
  feature no later renderer could complete.
- **`DocumentBoxModel::last_page`** — the page number, the section, the column heights, the notes and
  their numbers, the printed line numbers, and how many body assemblies the fixed point needed. None
  of it belongs in a tree of positioned boxes, and all of it is what a gate has to assert on.
- **`mjx-docx`'s residency grew the rest of a section** — the break kind, `w:cols` with §17.6.4's
  precedence applied, `w:titlePg`, `w:pgNumType`, `w:lnNumType`, `w:vAlign` and the note rules — plus
  the **header, footer, footnote and endnote content streams**, read once each and resolved through
  the same ladder the body's paragraphs go through, and `ParagraphFormatting::note_references`.

### Changed

- **The document's own page geometry now outranks the caller's `Constraints`**, falling back to it
  wherever a section states nothing. Not a preference: which section page 200 is in is not knowable
  without laying out the 199 before it, so the caller cannot choose. `mjx_docx::SectionFormatting`
  is no longer `Copy` (it carries a column list).
- **The continuation state is version 2, forty-five bytes**, and old thirteen-byte checkpoints are
  refused rather than misread. The four new fields are exactly the four things that cannot be
  recomputed from the position — the displayed page number, the continuous line-number counter, and
  the carried note with its number and resume line. A footnote's *number* is deliberately **not**
  among them: the *n*th reference in document order is note *n*, which is a prefix sum taken once.
- **Fragment addresses carry a part.** A header's third paragraph and the body's third paragraph are
  not the same place; `mjx_layout_docx::address` numbers the five streams, and the baseline snapshots
  print the part so that a header drawn twice cannot look like a header drawn once.

### Notes

- **Nothing here is parity with Word, and the evidence got weaker.** R19's strongest rows quoted
  UAX #14, an external definition of exactly what a line breaker consumes. There is no equivalent for
  *where a footnote area's gap goes* or *which number decides that a page is even*: ECMA-376 defines
  the attributes and is nearly silent on the rendering. The provenance ledger now prints **31
  `SpecCode` / 20 `DocumentedBehaviour` / 46 `EngineDerived`** on every run, and the `EngineDerived`
  rows are change detectors rather than evidence.
- **Word still has no scene companion** (MJXOFF-255), so a Word `FragmentTree` cannot reach pixels.
- Tables and floating objects are R21; fields, numbering, revision marks and OMML are R22.

## [0.0.150] - 2026-09-09

**Word's flow engine: lines, justification, and a pagination that is emergent (MJXOFF-174, R19).**

PowerPoint's box model places absolutely and Excel's addresses a grid: in both, page *N* is reachable
without ever looking at page *N−1*. **Word's pagination is emergent** — where page 200 begins depends
on everything on the 199 pages before it, and a single font substitution moves every boundary in the
document. `mjx-layout`'s `Checkpoint` exists for exactly that, and this is its first real consumer.

### Added

- **`mjx-layout-docx`, rank 3.6** — the third box model, beside PowerPoint's and Excel's so that an
  edge between any two of them is *sideways* and the layering gate refuses it by name. Thirteen
  modules: line layout against a per-line measure, the five alignments and two kinds of expansion,
  the three line rules, the five tab kinds with their leaders and their implicit grid, hyphenation,
  and a paginator that honours `w:pageBreakBefore`, `w:keepLines`, `w:widowControl` and `w:keepNext`.
- **`mjx_docx::Document::formatting` — the whole document read once.** The per-paragraph reader
  re-parses `word/document.xml`, `word/styles.xml` and the theme on **every call**, which is right
  for a caller asking one question and quadratic for a layout engine asking per paragraph. The
  read-once surface is `mjx-xlsx`'s `SheetFormatting` answer applied to a second format; the ladder's
  order is now stated in exactly one place and both orchestrations call it, with
  `crates/mjx-docx/tests/residency.rs` asserting they agree paragraph by paragraph and run by run.
- **`mjx_docx::Document::edit_paragraph_properties`** — the one primitive behind every `CT_PPrBase`
  member, exactly as `edit_section_properties` is the one primitive behind every `w:sectPr` member.
  Without it a caller could author a paragraph's *text* and not its *layout*.
- **`mjx_ooxml_types::support::universal_measure` and `half_point_measure`.** `ST_TwipsMeasure` and
  `ST_HpsMeasure` are `xsd:union`s of a number and a universal measure, so `w:defaultTabStop` may
  legally read `"0.5in"`. Two crates read them; one parser answers both.
- **Hyphenation in `mjx-text` and `mjx-layout`.** `BreakKind::Hyphenation`,
  `break_opportunities_with_hyphenation`, `LineBreaker::with_hyphenation` / `next_line_with`, and
  `LineComposer::hyphenating` — which adds the hyphen's own advance to every candidate it measures,
  because a line fitted without it overruns its measure on every hyphenated line.

### Changed

- **`LineComposer` reports `hyphenated`** on a composed line, and `ComposedLine` gained the field.
  A soft hyphen already broke a line through UAX #14's class `BA`; whether a *hyphen is drawn* is a
  rendering rule and now has somewhere to live.
- **`mjx_docx::Hyperlink::content` is reachable inside the crate** (it already existed as
  `pub(crate)`), which is what lets a reader walk a paragraph without living in `body.rs`.

### Verified

- **The checkpoint is proved by work, not by output.** `DocumentBoxModel::paragraphs_visited`
  reports what a call had to look at, and `tests/a_checkpoint_is_work_not_output.rs` lays page 41 out
  both ways: the fragments are identical EMU for EMU, the work is not, and — the assertion that makes
  the other two mean something — **withholding the checkpoint makes the cost scale with the page
  number while supplying it does not.**
- **Each of the four pagination constraints is asserted by page assignment**, in a pair that differs
  only in the one attribute, so a constraint that changed no page assignment fails rather than
  passes.
- **Justification is asserted on positions**, against each line's own natural positions rather than
  against a left-aligned layout — the two are cut differently and are not comparable. The identity
  case (a line with no gap) is asserted as an identity.
- **Fragment-tier baselines over the Word corpus**, eight committed specimens reached page by page
  through each checkpoint, every one stating `approver = generator` — which records that it is a
  change detector and that no person has looked at it.
- **Termination** for an unsatisfiable thousand-link `w:keepNext` chain, a paragraph taller than its
  page, a column narrower than one glyph and an indent wider than the column.
- **Provenance is declared and printed**: 13 `SpecCode`, 15 `DocumentedBehaviour`, 19
  `EngineDerived`, of 47 rows. **Nobody ran Word.** The `EngineDerived` rows are change detectors and are not
  evidence about Word; every one carries the reason it is a guess.

### Known limitations

- **Word has no scene companion.** `mjx-scene-docx` is the ticket that has to follow this one; until
  it exists a Word `FragmentTree` cannot reach pixels.
- **No pattern hyphenator ships.** `mjx_text::PatternHyphenator` exists and works; the Liang patterns
  it needs are language data and none is committed here, so `w:autoHyphenation` hyphenates only at
  the soft hyphens an author wrote unless a caller supplies one.
- **No CJK face is committed**, so the East Asian justification difference is asserted at the cut and
  the expansion point rather than end to end.
- Sections, columns, headers, footers and footnotes are R20; tables and floating objects R21; fields,
  numbering, revision marks and OMML R22.

## [0.0.149] - 2026-09-09

**Conditional formatting evaluated, cell drawings placed, and a sheet paginated for print
(MJXOFF-173, R18).**

`crates/mjx-xlsx/docs/guide/deliberate_limitations.md` named rule evaluation as a standing refusal,
because the library was a reader and a writer. A renderer has no such option: a sheet whose data
bars, colour scales and highlights are missing is not a rendering of that sheet. This child turns the
documented non-goal into implemented behaviour **one tier up** — in the box model, where a decided
rule is a rendering fact rather than a document one — and the documentation moves with it.

### Added

- **`mjx_layout_xlsx::condfmt`, six modules.** All eighteen members of `ST_CfType` are decided:
  the twelve `cellIs` operators over literal operands, `top10` (by rank and by percentage, from
  either end), `aboveAverage` with `@equalAverage` and `@stdDev`, `duplicateValues`/`uniqueValues`,
  the four text kinds, blanks and errors, the ten `timePeriod` windows, and the three graded kinds.
- **The `dxf` layer composes in Excel's order, not the file's.** Rules apply from the lowest
  `@priority` number down; the **first** rule to state a member keeps it, because §18.8.15 makes a
  `dxf` a delta and every one of its children is `minOccurs="0"`. `@stopIfTrue` ends the walk after
  the rule carrying it has fired, and does nothing on one that has not.
- **⚠ A `dxf`'s fill is read differently from a cell's, and this decides whether the whole feature
  is visible.** Excel writes a highlight as `<patternFill><bgColor rgb="FFFFC7CE"/></patternFill>` —
  no `@patternType`, and the colour in `bgColor`. Read as a cell's fill that is a pattern of `none`
  with no foreground, which paints nothing: the rule fires, the report says so, and the sheet looks
  identical.
- **Colour scales, data bars and icon sets as numbers.** A bar's width is a fraction of the cell
  (`@minLength` and `@maxLength` included, so the schema defaults give 0.10 and 0.90 rather than 0
  and 1); a scale answers the two stops a value fell between and how far along it is; an icon
  answers its index, after `@reverse`.
- **`Decoration::scale_fill`** — a colour scale travels **unresolved**, as two `CT_Color` stops and
  a position, because blending them needs the theme part and the workbook's `indexedColors`.
  `mjx-scene-xlsx` does the blend and the midpoint is asserted at the encoded display list.
- **`mjx_layout_xlsx::drawings`** — the three anchor modes placed against this crate's own row
  heights and column widths, which is the grid the cells underneath are on.
- **`mjx_layout_xlsx::print`** — the sheet's **second** pagination: paper and orientation, margins,
  manual row and column breaks, `_xlnm.Print_Area`, `_xlnm.Print_Titles` repeated at the top and
  left of every later page, `@scale`, fit-to-page, `@pageOrder` and `@firstPageNumber`.
- **`mjx_xlsx::AnchorPlacement` and `AnchorCell`** on `SheetDrawingObject`, plus
  `mjx_xlsx::drawing_geometry` — the three `mjx-dml` types the drawing surface takes as arguments,
  re-exported so a caller that declares no `mjx-dml` edge can still call it.
- **`Workbook::print_titles`**, the twin of the existing `print_area`.

### Changed

- **`Workbook::defined_names`, `defined_name` and `print_area` take `&self`.** Reading a part is not
  a mutation, and the box model's snapshot holds a `&Workbook`. New `Workbook::read_workbook_markup`
  is the `&self` counterpart of `workbook_markup`, and it reads an **edited** part from its tree
  rather than answering `MissingWorkbookPart` — a part in `PartBody::Edited` has no stored bytes.
- **The decoration sharing key gained a conditional signature.** Two cells with one `xf` that fired
  different rules no longer share a handle; two that fired the same rules still do, so a screen of
  highlighted cells stays one entry rather than two hundred.
- **`deliberate_limitations.md` §2 and `conditional_formatting.md` are scoped rather than absolute.**
  The refusal is now stated as a property of the read/write path, with the reason it stops there.

### Deliberately not done, and reported rather than hidden

- **Sparklines.** `x14:sparklineGroups` lives in a worksheet's `extLst`, which `mjx-sml` preserves
  verbatim and deliberately does not model. There is nothing to evaluate until a `mjx-sml`
  workstream models the extension.
- **A data bar's axis, negative fill, border and gradient**, all of which are `x14:dataBar`. What is
  drawn is Excel 2007's bar: a solid rectangle growing rightward, negatives clamped to
  `@minLength`.
- **An icon's artwork.** The index is computed and asserted; the eighteen sets of glyphs are Excel's
  and are not in this repository, so a stand-in would be an invented picture presented as the file's.
- **The inside of a drawing.** A drawing's content is DrawingML and `mjx-layout-pptx` lays DrawingML
  out — at rank 3.6, the same rank as `mjx-layout-xlsx`, so the edge is *sideways* and the layering
  gate refuses it by name. The placement is this child's; the content needs a crate below both box
  models that neither of them owns.
- **A `dxf`'s `numFmt`**, which is reported and not applied.

### Fixed

- A fit-to-page scale is **floored to a whole percentage**. The exact ratio makes the content
  precisely as wide as the page, so the last column tipped the accumulator over by a rounding EMU
  and a fit-to-one-page sheet paginated onto two.

## [0.0.148] - 2026-09-09

**The number-format engine — a date stops being a serial (MJXOFF-172, R17).**

R16 built Excel's box model and deliberately left number formatting out: a cell rendered its **raw
stored value**, so a date read `45719` and a currency read `1234.5`. MJXOFF-244 then built
`mjx-scene-xlsx`, which made that output visible. This child is the evaluator — `numFmt@formatCode`
applied to a cell's value — and it is a sub-project rather than a feature: a small language with its
own grammar, four conditional sections, bracketed conditions, colour codes, two date epochs and a set
of behaviours that are quirks rather than rules.

### Added

- **`mjx_layout_xlsx::numfmt`** — the evaluator, six modules: the two-phase parser (a format code
  cannot be classified while it is being read — `,` groups or divides depending on what *follows* it,
  `/` is a fraction bar only between digit runs, and `m` is a month or a minute depending on its
  neighbours), the numeric renderer, the date arithmetic, `General` and the fifteen-digit display
  clamp, the two caches, and the module that joins them.
- **Everything the language has, except five things named as absent**: the three digit placeholders
  and their padding, decimal points, thousands grouping, trailing-comma scaling, percentages,
  scientific *and engineering* notation (`##0.0E+0` on `12345` is `12.3E+3`), fractions with variable
  and fixed denominators, quoted and escaped literals, `_` skips, `*` fills, `@` substitution,
  `General`, the eight colour names and `[ColorN]`, bracketed conditions, `[$…]` currency and locale
  prefixes, every date and time token, `AM/PM` in four spellings, sub-seconds, elapsed `[h]`/`[mm]`/
  `[ss]`, and both date systems.
- **`Decoration::text_colour`** and a decoration table keyed on `(effective format, colour)`. A
  number format states its colour per **section**, and which section runs depends on the *value* — so
  the negative cells of a `#,##0;[Red]#,##0` column are red and the positive ones are not, with one
  `xf` between them. Every mechanism the two crates share for *not* duplicating a decoration worked
  against that, and this is what resolves it.
- **`SheetPalette::resolve_indexed`** in `mjx-scene-xlsx`, and a `text_decoration` that prefers a
  format's colour over the font's. `[Red]` travels as a bare **row of `indexedColors`**, unresolved,
  for the same reason a `mjx_sml::Color` does: resolving a palette row is the companion's job.
- **`CellReport::text`** — what a cell displayed, after its format ran. A `GlyphRunFragment` carries a
  shaped run and a byte range and *not* the string those bytes index, so a hit test, an accessibility
  tree or an exporter had nowhere to ask. It moves the string the box model already built.
- **`SheetGrid::with_number_format_language`**, and with it the *other* half of §18.8.30. Ids 27–36,
  50–58 and the Thai block have a **different** code per UI language — id 30 is `m/d/yy` in `zh-tw`,
  `m-d-yy` in `zh-cn` and `mm-dd-yy` in `ko-kr` — and nothing in a `.xlsx` states which language a
  consumer runs in, so `builtin_format_code` correctly answers `None` for all of them. The default is
  still `None` (those ids fall back to `General`, which is visibly and honestly wrong); a shell that
  knows its language now has somewhere to say so, and `format_code_in` is used when it does.
- **`SheetGrid::date_system` / `with_date_system`**, read from `workbookPr@date1904` by
  `SheetGrid::read`. The two epochs are 1,462 days apart, so a snapshot that defaulted to 1900 would
  shift every date in a Macintosh-authored workbook by four years, silently.
- **Four new suites** — 29 cases across `mjx-layout-xlsx` and `mjx-scene-xlsx`, plus five more in the two crates’ existing surface and ladder gates. A conformance table of **193 rows** with a declared provenance on each;
  a malformed-code suite over 57 broken codes × 21 values × two epochs × four value kinds; an
  end-to-end suite that reaches the engine through a real `numFmtId`; and a reachability gate that
  fails if any construct of the language — any `Element`, any `DateToken`, any `SectionKind`, any of
  the four `AM/PM` spellings — is not reached **by a row of the table**.

### Changed

- **`Workbook::date_system` takes `&self`** rather than `&mut self`. Reading a part does not dirty
  the package, and a box model holding a `&Workbook` has to be able to ask which epoch it is laying
  out. Relaxing a receiver is source-compatible.
- **`mjx-layout-xlsx`'s seven fragment baselines regenerated.** Four moved, all in the same
  direction: a one-character cell became an eight- or ten-character one. `100.000%` where the file
  says `1`; `2.00  USD;` where it says `2`. Still `approver = generator` — a change detector, not a
  human review.
- **`tests/the_ladder_is_consumed.rs` no longer forbids `evaluate`**, because R17 *is* the evaluator
  the refusal was holding the place for. What replaced it is stronger and aimed at this child's own
  named trap: the built-in format codes of §18.8.30 are a table nobody wrote down, a short copy falls
  back to `General` and **looks plausible**, so the gate now greps this crate's source for the codes
  themselves — asking `mjx-sml` for the list rather than restating it.

### ⚠ Two Excel quirks, reproduced rather than corrected

- **Serial 60 is 1900-02-29**, a day that never existed. Lotus 1-2-3 had the defect, Excel copied it
  for file compatibility, and a renderer that prints the arithmetically right answer prints something
  no Excel user has ever seen. The 1900 epoch is therefore *three* branches and not one constant.
- **Fifteen significant digits.** Excel stores a `f64` and displays it as a fifteen-digit decimal,
  which is why `=0.1+0.2` shows `0.3` in one cell while `=(0.1+0.2)=0.3` is `FALSE` in the next. That
  clamp also makes the rounding **decimal**: `2.675` is stored as `2.67499999999999982…`, and rounding
  the binary value to two places gives `2.67` where Excel gives `2.68`.

Both are asserted, and a renderer that "fixes" either fails deliberately.

### ⚠ Nothing here is parity with Excel, and the conformance table says so per row

The ticket asked for a table *transcribed from real Excel output*, and correctly warned that a table
whose expectations came from running the engine is green for any behaviour whatsoever. **No such
transcription was possible**: nobody ran Excel. So every row declares where its expectation came from
— 31 spec-transcribed codes, 110 documented behaviours, 52 engine-derived rows that are change
detectors and **not** evidence — and a test prints the split so nobody mistakes a green run for one.
`MJX_NUMFMT_CONFORMANCE_SHEET=<path>` writes the table as a sheet a person takes to Excel;
`docs/validation/07-the-reference-pack.md` §7 says how, and it is ten minutes.

Every behaviour chosen rather than read is marked `GUESS:` at its site. The sharpest: where a `-`
lands relative to a currency sign; whether a *conditioned* second section still renders unsigned;
what a conditional code no section matches shows (Excel fills the cell with `#`, which needs a
width); `General`'s digit count and its scientific thresholds, both of which are width-dependent in
Excel and are not here; how wide `_` is; and that a whole number under `# ?/?` blanks its fraction.

### Deliberately not implemented

Localised month and weekday names (English only — no locale database may enter the cross-build
matrix, and ECMA-376 publishes no name table), the Japanese era and Thai Buddhist calendars, `*`
expansion and `#######` overflow (both need a cell width the evaluator does not have, and a width in
the evaluator would put a column's geometry into the cache key of every value on the sheet).

## [0.0.147] - 2026-09-09

**`mjx-scene-xlsx` — Excel's scene companion, and the first worksheet to reach pixels (MJXOFF-244).**

R16 built Excel's box model and named its own weakest part: *"nobody has looked at a picture, and
here that is not a criticism of the gate — there is no path to one."* There was no path because a
`FragmentTree` carries `DecorationRef`s — bare numbers — and something has to resolve them into
paints. That something is `mjx_scene::ResourceResolver`, whose own documentation names the
implementor as *the box model's companion*; and the box model cannot be it, because
`crates/mjx-layout-xlsx/tests/the_seam_holds.rs` refuses `mjx-scene` there by name. So the resolver
got a crate, exactly as PowerPoint's did.

### Added

- **`crates/mjx-scene-xlsx` at rank 3.7**, *beside* `mjx-scene-pptx` rather than above it — the same
  arrangement, and for the same reason, as the two box models at 3.6. Two companions at equal rank
  makes an edge between them **sideways**, which the layering gate refuses by name: the two formats
  meet at `mjx-scene`, and a resolver that could read the other's would have made the display list a
  place where two formats negotiate. Both rank tables grown (`CLAUDE.md` and
  `xtask/tests/layering.rs`), each with what the rank does **not** buy written out.
- **A worksheet's colours, resolved.** `SheetPalette` holds the two tables a SpreadsheetML colour
  needs — the theme (addressed **by position**, not by a `SchemeColor` token) and the legacy indexed
  palette — plus the pair of system colours `auto="1"` and `indexed="64"`/`"65"` fall back to. The
  resolution algorithm is ECMA-376 §18.8.19's and already lived in `mjx-sml`; this calls it.
- **`Workbook::theme_colors`** in `mjx-xlsx`. `mjx_sml::styles::resolve_color` takes a
  `SchemeColors`, and that type's own documentation says *"getting the theme part out of the package
  is `mjx-xlsx`'s"* — but nothing there answered it, so every consumer wanting a theme colour would
  have become a second reader of the package. It is not a mutation: the part keeps its bytes.
- **Fills in all four of their shapes**: `patternType="solid"` (which paints its **`fgColor`** — the
  single most commonly got-wrong rule in SpreadsheetML), the seventeen hatches, a `dxf`'s
  colour-with-no-pattern third state, and gradients in both `linear` and `path`.
- **A worksheet travels the whole pipeline to pixels**, headlessly, in
  `crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs` — the only crate that may name
  a format crate (3.0) and `mjx-paint` (5.5) together. The gate asserts **content**: fragment counts
  by kind, band counts, one `DrawGlyphs` per glyph run, ink coverage with a floor *and* a ceiling,
  ink inside the tree's own rectangles, and `DrawReport::placeholders` against a count taken from the
  geometry provider **before** the render.

### Changed

- **A cell's borders are fragments now, not a value on its decoration.** R16 carried the four edges
  on `Decoration`, which a display list cannot consume: `mjx_scene::Decoration` has **one** stroke
  and a cell has four edges that differ in weight, colour and style, so a resolver handed those four
  can only pick one — and picking one draws it on all four sides. `mjx-layout-xlsx` now emits each
  edge as its own filled `BoxFragment`, which is what `mjx-layout-pptx` already does for a table
  cell, with the new `crates/mjx-layout-xlsx/src/border.rs` owning the weight table. Bands are
  emitted **after** every cell of a pane region, so that every fill is behind every border; a band
  drawn as a child of its own cell would be covered by the next cell's fill.
- **A gradient fill carries its stops.** R16 carried `is_gradient: bool` on the reasoning that the
  companion would resolve the gradient; the companion is handed a catalogue and nothing else, so a
  boolean names no stops and a gradient-filled cell had no path to a pixel at all.
- **`CellReport` carries its decoration handle**, which is what lets `text_decoration` turn a glyph
  run's address back into the cell's font colour. This is the method PowerPoint's companion still
  answers `None` from, and the difference is the document model rather than the effort: a cell's text
  is one string in one cell, and a slide's run lives inside a paragraph inside a shape.

### Fixed

- **The legacy indexed palette's alpha is no longer read as an opacity.** ECMA-376 §18.8.27 prints
  every row of the table with an ARGB alpha of `00` — black is `00000000` — and `mjx-sml` reports
  what Part 1 prints, which is right for a model and catastrophic for a painter: read as an opacity
  it makes every `indexed` colour in every workbook **fully transparent**, so
  `<left style="medium"><color indexed="8"/></left>` is a border that is simply not there. A colour
  reached through `@indexed` is now drawn opaque; one that states `@rgb` is left exactly as written.

### Reported, not fixed

- **The MJXOFF-243 opacity loss does not exist on this path**, and that was checked rather than
  assumed. A SpreadsheetML colour's `@rgb` is `AARRGGBB` with the alpha first, and it survives to
  `mjx_scene::Color::alpha` — `crates/mjx-scene-xlsx/tests/the_alpha_survives.rs` asserts it at the
  **encoded display list** rather than at the resolver. One narrower loss on the same subject does
  exist one crate below: `mjx_dml::SchemeColors::from_scheme` drops each theme slot's own alpha.
- **A border band is solid**, so the eight dashed `ST_BorderStyle` values draw as solid lines of the
  right weight. The style is still carried, so the fix is a change to two crates rather than a value
  recovered from the document again; `tests/the_dash_is_lost_at_the_band.rs` asserts the loss.
- **Gridlines are not in the fragment tree.** `showGridLines` defaults to on and a grey grid is the
  most recognisable thing on an Excel screen, but a gridline is a property of the *view* rather than
  of the document, and no hit test can ever land on one. Where it belongs is a decision for the child
  that draws a sheet on a screen.
- **`darkTrellis` and `lightTrellis` are drawn identically**, because DrawingML defines one trellis
  and SpreadsheetML two.

Nothing here is parity with Excel and nothing is described as such: every reading is marked `GUESS:`
at its site, and confirmation is a human sitting against real Microsoft Excel on Windows.

## [0.0.146] - 2026-09-09

**Excel's box model — a worksheet becomes a fragment tree (MJXOFF-171, R16).**

The second implementation of `mjx_layout::BoxModel`, and the first whose layout discipline is a
**grid**. PowerPoint places shapes absolutely, so a slide's layout costs what its shapes cost. A
worksheet addresses 16,384 columns by 1,048,576 rows — seventeen billion cells — so its layout has
to cost what is *on screen*, and that one fact decides the whole crate.

### Added

- **`crates/mjx-layout-xlsx` at rank 3.6**, *beside* `mjx-layout-pptx` rather than above it. Two box
  models at equal rank makes an edge between them **sideways**, which the layering gate refuses by
  name: a spreadsheet's box model must not know what a slide is. Both rank tables grown (`CLAUDE.md`
  and `xtask/tests/layering.rs`).
- **Two sparse indices and no dense array.** `RowGeometry` holds one record per row that states a
  height, a hidden flag, an outline level or a collapse flag; `ColumnGeometry` one per `col` **run**,
  which is what `CT_Col` already is. Both answer *where is row n* and *which row is at y* by binary
  search plus one multiplication, at a cost independent of how far down the sheet the question is
  asked.
- **Units converted in one place**: a row's `@ht` in points, a column's `@width` in characters of the
  Normal font's maximum digit width — *measured* through `mjx-text` rather than assumed — and EMU out
  the other side, through ECMA-376 §18.3.1.13's own truncating round trip.
- **Merged regions render once**, at the union rectangle, from the anchor; the covered positions
  produce no fragment at all. Each of the union's four borders resolves from the perimeter cells
  scanning outward, so a border Excel wrote onto the *last* column of a merge is not lost.
- **Text overflow into empty neighbours, in all four states** — spills, stops at the first non-empty
  cell in the direction of alignment, suppressed by `wrapText`, suppressed by `horizontal="fill"` —
  with the direction taken from the **resolved** alignment, so a number in a `general` cell spills
  leftward.
- **Wrap, shrink-to-fit, indent, rotation and stacked text**, and the eight horizontal alignments
  including `fill`, `centerContinuous` and `distributed`.
- **Frozen and split panes** as up to four regions over **one** grid geometry, with `@xSplit` read as
  a column count for a freeze and as twentieths of a point for a split.
- **Auto-fit column width**, measured over a column's populated cells and memoised. Row heights are
  deliberately *not* recomputed: Excel writes the fitted height into the file, and a recomputed one
  would make the top of row *n* depend on every row above it — a prefix sum over the addressable
  range rather than the stated one, which the sparse index cannot coexist with.
- **`Row::cell_after` / `Row::cell_before`** in `mjx-sml`: the nearest populated cell strictly to one
  side of a column, by binary search. The overflow rule needs it; without it a renderer would probe
  up to 16,383 columns per overflowing cell.
- **`CellFormatResolver::interner`** in `mjx-sml`, so a caller holding a resolver can read an
  attribute off the `Font`, `Fill`, `Border` or `CellAlignment` it answers with.

### The gate

`crates/mjx-layout-xlsx/tests/sparsity_is_measured.rs` lays out a sheet whose only populated cell is
`XFD1048576` under a counting global allocator. One band costs **58 KB** and visits 20 rows by 9
columns; the band containing the far corner, 54,613 bands down and scrolled to column 16,380, costs
**12 KB** and 359 µs. The same file then does what the crate refuses to do — walks a *range of
coordinates* over one 16,384th of the grid — and shows it already costs **8.4 MB**, sixty-four times
the whole windowed layout's bound. A gate that cannot fail is not a gate.

### Not in this child

Number formatting (MJXOFF-172 — a cell renders its raw stored value, and the format code in force is
carried on every decoration so that child is a change to one crate), conditional formatting,
drawings and print layout (MJXOFF-173), charts (R23), and formula evaluation, which does not exist in
this loop at all: a cached value is rendered as stored, which is correct for a viewer.

**Nothing here is parity with Excel.** Every behaviour chosen rather than read is marked `GUESS:` at
its site, and confirmation is a human sitting against real Microsoft Excel on Windows.

## [0.0.145] - 2026-09-08

**The first end-to-end deck — a `.pptx` becomes pixels (MJXOFF-170, R15).**

Every stage of the render pipeline has existed and been gated on its own since R06. **Nothing had
ever driven all of it at once with a real document.** This is the child that does:

```
Presentation → SlideDeck → SlideBoxModel → FragmentTree
             → SlideResources + SlideGeometry → build_scene → DisplayList
             → SoftwarePainter → pixels
```

`crates/mjx-reference-pack/tests/a_real_deck_reaches_pixels.rs` runs that chain over three committed
fixtures, headlessly, on the pure-Rust painter.

### The new crate

**`mjx-scene-pptx` at rank 3.7** — PowerPoint's companion to the box model: the `ResourceResolver`
that turns `mjx-layout-pptx`'s decoration, image and outline handles into `mjx-scene`'s paints,
strokes and effect DAG, and the `GeometryProvider` that resolves an outline handle through
`mjx-geometry`'s preset tables.

**It is a crate rather than a module because `mjx-layout-pptx`'s own seam gate refuses `mjx-scene` by
name**, on the ground that a box model which built a display list would have merged two stages the
architecture separates on purpose. That gate is right; the answer to it is a crate, not an exemption.
3.7 is the only rank from which one crate can name the box model (3.6), the display list (1.7) and
the geometry tables (2.5) at once while staying below the viewport (3.8).

### `mjx-layout-pptx` grew the rest of PowerPoint's visual vocabulary

- **Tables** — the grid, column widths, row heights that *grow* to fit their text, merged and spanned
  cells, cell insets and anchoring, effective cell fills and borders through the table style's six
  conditional bands. Cells become `BoxFragment`s carrying a `TableCell`; the frame becomes a
  `TableFragment`.
- **Effects** — a shape's effective `a:effectLst`, carried on its decoration and translated into
  `mjx-scene`'s effect chain in ECMA-376's own child order.
- **Pictures** — `p:pic` becomes an `ImageFragment` with a handle shared by relationship id, so a
  page that repeats a logo decodes it once.
- **Speaker notes** — `SlideBoxModel::layout_notes` lays a notes slide out through the identical
  walk, addressed under `mjx-session`'s notes part rather than its slides part.

### `mjx-pptx` grew one reader

`Presentation::shape_preset` answers a shape's `a:prstGeom@prst`. `shape_geometry` answers the
*typed adjustments* and carries no `ST_ShapeType` token, so before this there was no way for a
renderer to ask which preset a shape draws — which the first end-to-end render found the moment it
needed to feed a geometry provider.

### The gates that would catch a silent regression

- **Every effect proved by its absence failing.** Each of the seven is rendered with and without,
  and the pixels must differ; and no two kinds may rasterise identically, which refuses a
  translation that mapped them all onto one.
- **Merged cells that a naive walk gets wrong** — a 4×3 grid carrying a horizontal merge, a vertical
  merge and a second horizontal merge outside the header row, asserted on rectangles and counts.
- **Nested group transforms compose** — a shape two groups deep, each scaling, where the right
  answer (6×) and the single-application defect (3×) are different numbers.
- **Stand-ins are counted, not assumed** — the painter's `placeholders` is compared against the
  geometry provider's own count of unanswerable handles, taken before the render.

### ⚠ A defect this release asserts rather than fixes

`mjx-dml`'s `resolve_fill` / `resolve_line` / `resolve_effects` bake every colour to a
`ColorSpec::Srgb` hex **triplet**, which has no alpha channel — so an `a:alpha` transform is lost.
The standard Office theme puts `<a:alpha val="63000"/>` on the shadow of every shape, so a shadow
renders **solid** where the document asks for 63 %.
`crates/mjx-scene-pptx/tests/the_opacity_is_lost_at_the_spec_boundary.rs` proves the loss off a real
fixture and records what fixing it costs. It is a work item of its own: `ColorSpec` is constructed at
263 sites and matched in both bindings.

### ⚠ Nothing here is parity with PowerPoint

Every behaviour chosen rather than read is marked `GUESS:` at its site. Confirmation is a human
sitting against real Microsoft Office on Windows.

## [0.0.144] - 2026-09-08

**A `.pptx` becomes a `FragmentTree` — the first real box model (MJXOFF-169, R14).**

Thirteen children built machinery: fonts, shaping, the `BoxModel` contract, the display list, four
painters, a fidelity oracle, a session, a viewport. **Not one of them laid out a document.** This is
the one that does: `mjx-layout-pptx` (rank 3.6) walks a slide's shape tree in z-order, places every
shape at the bounds `mjx-pptx` resolves for it, and lays out every text body inside its shape — the
four insets, the columns, the nine indent levels with their bullets, the line spacing, the anchor,
and the autofit that ties them together.

### The new crate

- **`crates/mjx-layout-pptx`** — `SlideBoxModel`, the first implementation of
  `mjx_layout::BoxModel`. One slide is one page, so `estimate_extent` is `Exact` (the only box model
  in the programme whose "estimate" is the answer), `invalidate` names pages rather than a suffix,
  and a checkpoint carries four bytes.
- **It consumes the seven-tier effective-property ladder and re-derives none of it.**
  `effective_shape_bounds`, `effective_shape_transform`, `effective_body_properties`,
  `effective_paragraph_properties`, `effective_run_properties`, `effective_shape_fill` and
  `effective_shape_outline` answer every question about what a shape *says*.
  `tests/the_ladder_is_consumed.rs` holds that by grepping this crate's source for the wire
  vocabulary a second resolver would need, and by refusing every *declared*-property reader by name.
- **Every measurement comes from `mjx-text`.** Shaping, bidirectional resolution, script
  itemisation, face fallback and line breaking; nothing here measures a glyph.

### `a:bodyPr` is modelled for the first time

It was preserved verbatim and typed nowhere, because fidelity never needed it — and layout does: the
insets, the anchor, the wrap flag, the column count and the autofit choice are the whole of a text
body's geometry.

- **`mjx_dml::TextBodyProperties` / `TextBodyPropertiesSpec`** — the fifth typed piece of
  `a:txBody`, in the same two-type shape as the four before it. Its schema defaults are named
  constants applied at the point of use, so an authored `0` inset is still distinguishable from an
  unstated one. `TextWrapping` is hand-written beside the attribute that reads it, because
  `ST_TextWrappingType` is not in the generator's curated type list.
- **`Presentation::body_properties`, `effective_body_properties`, `set_body_properties`** — the
  stated value, the value after the placeholder chain has been walked, and a merging writer.

### ⚠ Nothing in the new crate is parity with PowerPoint

ECMA-376 says what the attributes are and is nearly silent on what a renderer does with them, so a
number of behaviours are readings rather than facts. Every one is marked `GUESS:` at the site that
makes the choice. The sharpest is autofit, and it is deliberately split in two:

- **honouring** a stored `a:normAutofit@fontScale` reproduces exactly what the author saw, and is
  exact;
- **computing** one is running PowerPoint's own search, which has never been specified. The ladder
  of scales is derived from the values PowerPoint is observed to write; the order the two factors are
  stepped in is a guess.

`AutofitOutcome::recomputed` is what keeps the two apart, and `AutofitPolicy::Disabled` exists so a
gate can prove the search is what makes the difference. Confirmation is the Windows sitting.

### Gates

Fragment-tier golden snapshots over nine committed slides, each carrying an approval record that
says `generator` — a real record, and not a human review. Autofit proved on **overflowing** fixtures
with the computed scale asserted as a number. A `SourceRef` round trip that goes out of the crate
and back through `mjx-pptx`. Nine indent levels asserted as nine distinct indents. The addressing
scheme checked against `mjx-session`'s, which wrote it down first.

## [0.0.143] - 2026-09-08

**Viewport windowing, byte-budgeted caches and frame scheduling — and the unbounded residency
MJXOFF-167 declared is now closed (MJXOFF-168, R13).**

A four-hundred-page document at 150 dpi is roughly four gigabytes of pixels. Nobody holds that
anywhere, client or server, so *"high-throughput page view memory management"* is a **windowing**
problem rather than a hosting one, and `mjx-view` (rank 3.8) is where that discipline lives.

### The new crate

- **`crates/mjx-view`** — generic over `mjx_layout::BoxModel` and over a new `SceneSource` seam, so
  it windows a `.pptx`, a `.docx` and an HTML paste with one body of code and names none of them.
  Four things in it:
  - **`PageWindow`** — the pages on screen plus a prefetch ring **biased in the direction of
    travel**, sized by the viewport rather than fixed at one page (`Viewport::with_zoom` is how a
    zoom becomes a window size).
  - **`CacheBudget`** — a declared byte ceiling for each of the seven stages `UI_PLATFORM_PLAN.md`
    §4 L6 names. Three are held here; the other four are `mjx-scene`'s and the painter's, and the
    table says which rather than pretending otherwise. **Checkpoints are kept for every page and
    fragments are not** — that asymmetry is the design, and it is what turns a jump to page 300
    into one page of layout.
  - **`ScrollModel`** — the scrollbar exists before anything is laid out, and estimates become
    measurements **without the scrollbar jumping under the reader's thumb**. The state is an
    *anchor* (a page and how far into it), never a document offset, which is the whole fix.
  - **`FrameBudget`** — 16.6 ms at 60 Hz, checked between tasks; visible pages run first, prefetch
    is deferred rather than dropped, and a fling gets a preview tier that lays pages out and builds
    no display lists.

### The weakness R12 declared, closed

- **`SpreadsheetSession`'s worksheet residency is bounded**, by **bytes** rather than by a sheet
  count — `mjx-sml`'s own gate measures its packed store at 36.8 bytes per cell, which is what makes
  a figure in bytes meaningful. A clean sheet is evictable, least recently used first, and re-parses
  from its part; a **dirty** sheet is pinned and never evicted, because evicting it would drop an
  edit the package has never seen. `resident_bytes`, `pinned_residency_bytes`,
  `residency_evictions` and `least_recently_used_sheet` make it a measurement, and
  `crates/mjx-session/tests/residency_budget.rs` walks forty sheets through a budget that fits four.

### One cache, three consumers

- **`mjx_ooxml_core::ByteBudgetCache`** — the workspace's one byte-budgeted least-recently-used
  cache, at rank 0.0 because its three consumers sit at 1.7, 3.5 and 3.8 and a cache written in the
  highest of those is unreachable from the other two. The same argument, and the same answer, as
  `Emu` moving down in MJXOFF-160. `mjx-scene`'s `MeshCache` was rebuilt on it and its ten-case
  budget gate passes unchanged.

### Two failures a viewport must not have, and does not

- **An over-long estimate is not an error.** A window is built from the scroll model's page count,
  which is a guess until a page reports no continuation, so a window can name a page that is not
  there — including one the same frame has just discovered is past the end. The frame **skips** it
  and counts it in `FrameReport::pages_past_the_end`. `ViewFailure` therefore has no viewport
  variant at all: *no such page* is a question only a box model can answer, and a viewport that
  answered it could disagree with the document it is showing.
- **A reflow makes the document's length a guess again.** A page that ended the content made the
  scrollbar's length a fact; an insertion undoes that, and a model that kept the old figure would
  clamp the scrollbar short of the content the edit added. So a reflow clears the end marker and the
  next frame asks `BoxModel::estimate_extent` how long the document is now — on the frame, because
  that is where the content is.

### Breaking

- **`mjx_session::Invalidation` carries a `mjx_layout::ChangeKind`.** An address says *where*, and a
  box model needs *what*: a reformat cannot move the content after it and an insertion moves every
  page that follows. `Invalidation::new` takes a third argument; `Invalidation::at` keeps its
  meaning and reports `Reformatted`, and `Invalidation::reflowing` is the wide one. `WordSession`
  reports the wide one for a run whose text changed length.

## [0.0.142] - 2026-09-08

**The resident document: an operation journal recorded the instant an edit happens, and a commit
that serialises dirty parts on a schedule rather than on every operation (MJXOFF-167, R12).**

The rest of this workspace is a batch library and holds nothing between calls —
`crates/mjx-xlsx/docs/guide/large_workbooks.md` says so in its own words, and it is right for a
program that opens a file, changes it and writes it out. It is fatal for an editor. `mjx-session`
holds a document open and separates the three layers `docs/client-platform/SESSION_AND_PERSISTENCE.md`
names: **record** into the journal immediately, **apply** to the model immediately, **commit** on a
schedule.

### Added

- **`mjx-session` at rank 3.5**, a new workspace member, in both rank tables and in `README.md`'s
  ladder. `Session` is generic over `ResidentDocument`; `PresentationSession`, `WordSession` and
  `SpreadsheetSession` implement it for the three formats behind a default `ooxml` feature.
- **An operation vocabulary written in `mjx-layout`'s address space.** An `Operation` is a
  `SourceRef` — a part number, a path of small integers, a character range — plus either a `Value`
  or a `LayoutRect`. No OOXML type appears in one, which is what makes the journal a journal a
  non-OOXML box model can also produce, and what keeps the collaboration seam open at no cost now.
  Every operation is an **absolute assignment**, which is what makes replaying a journal tail over
  work a commit already wrote idempotent.
- **`CommitScheduler` and seven triggers** — idle, max age, a dirty-byte threshold, the journal's
  memory bound, an explicit save, backgrounding and a consistency point — with a gesture that defers
  the four economic ones and cannot defer the other three. **Backgrounding is mandatory and
  immediate**: iOS terminates backgrounded applications without warning.
- **Undo units that are semantic and independent of the commit window.** A unit owns its own steps
  rather than pointing into the journal, precisely because the journal is truncated at every commit;
  tying the two together is the classic bug where undo jumps by however much happened to be batched.
- **A framed, checksummed journal encoding and `Recovery`.** A record carries its own length and an
  FNV-1a check, so a torn tail — the shape a crash leaves — costs exactly that record.
- **`Package::settle_edited_parts` and `Package::dirty_part_names`**, with `dirty_parts` /
  `settle_dirty_parts` on all three format types. Settling serialises each dirty part once and moves
  it to *clean but still resident*: the next save writes it verbatim, the next edit costs no
  re-parse, and the dirty set actually clears. Without it a part edited once re-serialises on every
  commit for the rest of the session, which is the cost batching exists to avoid.

### Changed

- **The documented copy-on-write rule now states two moments instead of one**, in `CLAUDE.md` and
  `PLAN.md`. *On first edit, drop the raw bytes and mark the part dirty — the model is now
  authoritative; serialise at commit, once, however many edits have accumulated.* The implementation
  always worked this way; the prose described one moment. **The round-trip guarantee is untouched**
  and `crates/mjx-session/tests/fidelity.rs` says so: opening and committing a `.pptx`, a `.docx` and
  an `.xlsx` changes no part, editing changes exactly one, and an edit followed by its undo changes
  none.

### Measured

- **Twenty keystrokes into one run cost one part serialisation; the same twenty under
  `CommitPolicy::per_operation` cost twenty.** Asserted on all three formats, because *"a commit
  produces a valid document"* is green for a session with no batching in it at all — the check
  passes precisely when the feature is absent. Five hundred keystrokes cost **one** commit and one
  serialisation, a ratio of 500 : 1.
- **Recording is allocation-free.** A thousand payload-free operations into a reserved journal hand
  the allocator **0 bytes** of work, measured with `mjx-allocation-counter` — its third consumer.
  The gate is `harness = false` for the reason `mjx-sml`'s is: the first version of it was a
  three-case harness and read 92,376 bytes, every one of them another case's, on another thread.
- **Recovery is proved against a real process kill.** The suite re-launches its own test binary, has
  the child record four operations, flush, record two more without flushing and then `abort()`, and
  reads the file back from the parent: the four are there and the two are not. A same-process replay
  would have proved only that the encoder agrees with the decoder.

## [0.0.141] - 2026-09-08

**Audit pass 10: the token editor stops accepting a colour that breaks the next build, and the two
crates above the graph get the gate their rank cannot give them.**

Ten findings from a read-only audit of MJXOFF-166, ordered by how badly each would mislead a reader
into believing something was proved that was not.

### Fixed

- **The live token editor validated a value's *type* and not its *contrast*, and the contrast rule
  is the one a design tweak trips.** `DESIGN_TOKENS.md` §2.2 — every colour tagged for text reaches
  4.5 : 1 against its declared background — lived in `xtask/src/codegen/tokens/model.rs`, and
  `mjx-canvas-harness` may not depend on `xtask`. So a person tweaked a text colour in the harness,
  got a green write-back, and `cargo run -p xtask -- tokens` went red afterwards: exactly the
  failure the editor's own comment says its validation exists to prevent. **The rule moved down into
  `mjx-tokens`**, where every writer of `tokens.json` can reach it — `check_usage`, `contrast_ratio`
  and `ColorUsage` are one implementation, and `xtask`'s generator now *calls* it rather than
  keeping a copy. This matters immediately: the user is about to audit sixty-one elements through
  that editor.
- **Entry 30's note indicator was badged `touch` and recorded no grab region** — a touch element the
  audit surface could not answer for, on the entry whose own description is *"four device pixels on
  a side at 1×, which is where a triangle stops being one"*. Found by the new assertion below rather
  than by reading.

### Added

- **`TokenIdentity` carries `usage` and `background` as data**, not as prose in a generated doc
  comment. A program could not read the old form, which is why the rule could not travel.
- **`crates/mjx-canvas-harness/tests/the_seam_holds.rs` and
  `crates/mjx-render-oracle/tests/the_seam_holds.rs`.** Both crates sit above the whole document
  graph with **no rank**, so `xtask/tests/layering.rs` — whose rule is a comparison of two ranks —
  holds nothing about what either depends on and would have accepted `mjx-canvas-harness → mjx-pptx`
  without a word, while `CLAUDE.md` described the property as though something held it. Each gate
  asserts its crate's dependency set **exactly** and scans its sources for a forbidden name. Both
  read the one scanner at `crates/mjx-paint/tests/support/manifest.rs`, shared by `#[path]` include
  rather than copied — what is worth sharing is the three parser defects MJXOFF-164 fixed in it, and
  a copy would not carry them — and that scanner's **own two instrument tests** compile into all
  three gates, because a manifest scanner that silently stops seeing a category passes forever.
- **`crates/mjx-canvas-harness/tests/the_router_answers.rs`.** `server.rs` shipped at 591 lines with
  no test calling any of it: an `abort()` in the `("GET", "/api/probe")` arm would have fired
  nowhere, and the existing assertion was that the HTML *string contains* `"/api/probe?"` — a claim
  about a hyperlink. Twelve cases now drive `route` and a new `read_request` directly: every route
  with its content type, a `POST /api/tokens` round trip against a copy of the source, eight
  refusals each checked to leave the file byte-identical, the body-length cap, the `Content-Length`
  parse and an unreadable request line.
- **`xtask/tests/unsafe_allowance.rs`**, holding the exact set of files that carry
  `#![allow(unsafe_code)]`. `README.md` said *"the two binding crates are the only ones"* — false
  since MJXOFF-163, there are four crates and five files — and ended *"CI greps them to keep that
  claim true"*, which made a stale claim look mechanically guarded. The grep it named guards what
  the allowance is *used for* and never who has one.
- **An assertion that every entry declaring `Axis::Input` records a grab region.** It was read in two
  places and both purely for display.
- **Disclosure where the reader actually is.** `check` now ends with *"61 of 61 carry NO HUMAN
  REVIEW"*, matching the oracle's; the harness page carries a banner saying the sixty-one designs
  are proposals awaiting the user's pass. The gallery, the checklist and the CI job already said so
  — the command CI runs and the page the review happens on did not.

### Documentation

- **`CLAUDE.md`'s claim that the harness names no format crate *"exactly like the oracle"* was false,
  and backwards.** The oracle declares `mjx-geometry`, `mjx-dml` and `mjx-ooxml-types` for the
  `preset-star` specimen; the harness declares none of them. The harness's property is the
  **stronger** of the two. Both sentences are corrected, and the paragraph now states plainly that
  neither property was enforced by anything until this pass.
- `README.md`'s test-only crate list named three of six and drew none of the distinction between the
  three *below* their consumers and the three *above the whole graph*; `CONTRIBUTING.md` still said
  *"pure-Rust dependencies only in shipped crates"*, superseded by MJXOFF-163. Both are held by
  `xtask/tests/unsafe_allowance.rs` now, because a claim that drifted once will drift again.
- **`docs/UI_PLATFORM_PLAN.md` §11.2**, the description of the canvas harness that did not exist —
  the plan's only account of R11 was a stale parenthetical saying in-canvas UI *"is audited as
  generated image plates instead"*, which R11 superseded. Written in the shape of §11.1.
- `Entry::responds` records that six of the sixty-one declarations were corrected *from* the first
  measurement and that only two are recoverable from the tree, because the crate landed in one
  commit. A declaration written from a measurement cannot detect that the measurement was wrong to
  begin with, and saying which four is not something an agent can reconstruct.

## [0.0.140] - 2026-09-08

**The canvas UI harness: sixty-one in-canvas elements, exercised by hand** (MJXOFF-166).

`docs/client-platform/CANVAS_UI_INVENTORY.md` §2 lists sixty-one things the Rust renderer draws that
are not document content — selection outlines, resize and rotation handles, carets, squiggles, range
borders, marching ants, page shadows, focus rings. They are the most design-sensitive surface in the
product and **none of them can be a web component**, so a Storybook-only audit would miss all
sixty-one. This is the surface they are audited on.

### Added

- **`crates/mjx-canvas-harness`** — a new test-only crate, `publish = false`, outside the ranked
  graph and at the top of it beside `mjx-reference-pack`. `cargo run -p mjx-canvas-harness -- serve`
  binds a local server and prints a URL; the page carries a searchable scene list, a state panel for
  every axis of the matrix (default/hover/active/focused/disabled × light/dark × 1×/2×/3× ×
  pointer/touch), an overlay ruler, a hit-test visualiser, a pixel inspector and a live token editor.
- **Sixty-one synthetic scenes**, each a `FragmentTree` built by hand. **The harness needs no
  document and no fixture**, which is the property that lets in-canvas design be settled while the
  format renderers are still being built.
- **A live token editor that writes back** to `docs/client-platform/data/tokens.json`, editing the
  narrowest possible span so a one-character tweak is a one-line diff rather than a reformat of the
  one hand-edited artefact in the pipeline.
- **Sixty-one plates**, through `mjx-render-oracle`'s plate generator, its PNG encoder, its manifest
  schema and its baseline store — no second emitter and no second schema. This is the **first**
  consumer of that crate's plate generator; `mjx-reference-pack` uses its authority vocabulary and
  never renders a plate, so the permitted half of the layering rule was asserted and unexercised
  until now, and `xtask/tests/layering.rs` now names both consumers.
- **A new CI job, `canvas-harness`**, which uploads the plates, the gallery and the checklist on
  failure as well as on success. It needs no GPU and no external reader.
- **`docs/client-platform/CANVAS_UI_AUDIT.md`** — the checklist, one line per element. **Nothing in
  it is ticked and no agent may tick it.**

### The gates, and the traps they are written against

- *"All 61 elements have a scene" is satisfied by 61 empty canvases.* Five counters per entry —
  placeholders, draw calls, covered pixels, **distinct colours**, declared command kinds — plus a
  command count strictly above the bare stage's. The distinct-colour counter is R10's own hand-off:
  an overlay drawn in the page's own ink passes every other check and is invisible.
- *A state toggle that does nothing is invisible to any reachability check.* Every entry declares
  which axes move its pixels; the suite measures the declaration **in both directions** and prints
  the per-axis counts. It found six disagreements on its first run. All sixty-one canonical renders
  are asserted to be distinct pictures, which is what makes "61 elements" a number rather than a
  claim.
- *A hit-test visualiser that computes its own regions proves nothing about the index.* A grab region
  records a **fragment**, and the rectangle is `SpatialIndex::bounds_of` inflated by the input
  device's padding. There is nowhere for a second computation to live.
- *A golden image generated by the code under test always matches the code under test.* Answered by
  not answering it here: the baselines, the approval events and the explicit-only regeneration path
  are `mjx_render_oracle::baseline`'s, unchanged. **Every plate is stamped `approver = generator`,
  which is a real approval record and is not a human review.**

### ⚠ What this does not include

**The mobile render surface.** MJXOFF-166 specifies a `tauri-plugin-mjx-surface` and it was not
written: a plugin that compiles and has never created a surface would satisfy the sentence and prove
nothing. The eleven touch entries are exercised in a **mobile browser** over the LAN
(`serve --host 0.0.0.0`) — a real touch device at a real density — and that is not the native
surface. `docs/client-platform/CANVAS_UI_INVENTORY.md` §4.2 says exactly what is missing, and
corrects the ticket's premise while it is there: R08 already generalised `SurfaceHost` over a
platform window handle, so what is missing is a **shell** on a phone, not a rendering seam.

## [0.0.139] - 2026-09-08

**The preset-shape geometry sweep runs on CI, and the class of hole it belonged to is now a test**
(MJXOFF-197).

`crates/mjx-dml/tests/guide_formula.rs`'s
`every_guide_of_every_preset_shape_definition_evaluates` walks the *entire* normative preset-shape
corpus — every guide of every shape block of `presetShapeDefinitions.xml`, evaluated at a
deliberately lopsided box so a guide that confuses two extents cannot pass by coincidence. **It had
never once executed on CI**, in the whole history of the repository, and reported `ok` every time.

Three individually-correct facts composed into the hole. The suite skips when `References/` is
absent, which is right — the tree is licensed material and git-ignored. Its escape
`MJX_REQUIRE_PRESET_GEOMETRY` was set by no workflow. And the only job that extracts `References/`
never named `-p mjx-dml`; `mjx-dml` appeared in no job in any workflow file. **An absent corpus reads
exactly like success**, which is why two independent programmes built the same instrument and neither
noticed.

### The archive comes first, and that ordering is the finding

The obvious fix — set the variable on `schema-validity` — **would have turned CI red**, because that
job had no ECMA-376 Part 1 to read. A step written that cannot execute is this defect one level up,
and the ticket committed it inside the ticket that names it.

So `.github/scripts/fetch-ecma-schemas.sh` carries **Part 1** now, pinned in
`.github/ecma-376-archives.sha256` by a SHA-256 computed from a fresh download off ECMA's own server
— never from the copy sitting in a developer's `References/`, which is precisely the artefact whose
provenance nobody can reconstruct later.

Part 1 needs **two** of its six members (`OfficeOpenXML-XMLSchema-Strict` and
`OfficeOpenXML-DrawingMLGeometries`), and the script's `outer|member|marker` entry format assumed
one. Two entries sharing one outer archive is the tempting shape and it is wrong twice:
`verify_archives` runs `sha256sum --check --strict` against a manifest that carries each file once,
so the two lists stop corresponding and the next person adding a part cannot tell which is
authoritative. The member field is a **list** instead, each member with its own marker, so one
archive stays one entry — and a new guard refuses an `ARCHIVES` entry the manifest does not cover,
which would otherwise be fetched and extracted unverified.

**What it costs is two numbers, not one:** the download and the CI cache grow by **42 MB** (the
outer archive is atomic and 35.3 MB of it is the part's PDF), while the extracted tree grows by
**~1.5 MB**. The `-j`-plus-explicit-member extraction is what keeps the difference; the PDF is never
written to disk.

### Two gates, both observed executing rather than merely configured

`schema-validity` now runs `cargo test -p mjx-dml --test guide_formula` under
`MJX_REQUIRE_PRESET_GEOMETRY=1`, and `cargo test -p xtask --bin xtask`, which is the selector that
reaches `the_committed_geometry_table_is_exactly_what_the_file_produces` — the byte-for-byte
re-derivation of `crates/mjx-geometry/src/generated.rs`. That one had the same hole for a subtler
reason: it lives in the xtask **binary's** unit tests, and `lint-test` runs `--workspace` without a
schema tree while this job selected integration targets by name.

Both steps run with `--nocapture` and both now **print the counts they evaluated**, because
`cargo test` swallows a passing test's output and a green tick cannot distinguish "the corpus was
swept" from "the corpus was absent". A log line with a number in it can.

The sibling drift check over `mjx-ooxml-types` — the other programme's `codegen_drift.rs`, which
needs this same Part 1 archive — is not in this tree. It is **MJXOFF-227**, whose stated precondition
is the archive line added here.

### And the class, closed by an instrument rather than by a sweep

`xtask/tests/escape_hatches.rs` enumerates every `MJX_REQUIRE_…` in the workspace and fails on one
that is neither bound by a workflow nor justified where it is defined. A one-time census was never
going to be enough: **the roster expired twice while this ticket was open**, once from a child in
this programme and once from a peer branch.

Three states, not two, and the middle one is why the workflow is *parsed* rather than grepped. A
string census cannot tell a binding from the comment a careful author writes explaining why there is
no binding — and this repository has exactly that case, `MJX_REQUIRE_OFFICE_CORPUS` appearing in
`ci.yml` only inside "deliberately NOT set". So the parser reads the file's indentation structure
with comments removed quote-aware, and counts a key only under an `env` ancestor. The scanner has its
own instrument tests over synthetic trees — including one proving a comment is not a binding and one
proving the gate can go red at all — because a scanner that silently stops discriminating passes
forever, which is this ticket's own defect class one level up.

`MJX_REQUIRE_OFFICE_CORPUS` and `MJX_REQUIRE_OFFICE_EXPORTS` are marked `MJX-ESCAPE-UNSET` at their
definition sites, with the reason they already carried in prose: both guard directories that ship
empty by design and that no agent may fill, so binding either would make the build red about
something no build can fix.

## [0.0.138] - 2026-09-08

**The fidelity oracle — layered assertions, perceptual diffing, and the document plate gallery**
(MJXOFF-165, Phase R position 10 of 24).

For an engine claiming parity this is the most important instrument in the project, and it is a
foundational child rather than a later one: building the renderer for a year and *then* asking how
close it is would be the defining mistake available here. Every layout child from R14 onward is gated
by this harness, so it exists before there is anything to regress.

### The trap it is written against

**A golden image generated by the code under test always matches the code under test.**
Regenerate-and-compare is green by construction and proves nothing whatever. Three refusals answer
it, and none is a matter of policy:

* **A baseline with no approval record fails**, rather than passing — proved by adding one.
* **Regeneration is explicit and cannot happen implicitly.** `Baselines::regenerate` takes a
  `RegenerationIntent`, whose only constructor reads `MJX_ORACLE_REGENERATE`. A check path cannot
  make one by accident because it cannot make one at all.
* **Regeneration removes the approval it overwrites**, before writing anything. A path that kept it
  would ratify exactly the regression the baseline exists to catch.

An approval binds **the bytes and not the name**: a SHA-256 per artefact, which `sha256sum`
reproduces, so the record is auditable by the person whose approval it claims.

**And the honest part.** Every baseline shipped here is stamped `approver = generator`, which is a
real approval record — the digest binding is live — and is **not a human review**.
`awaiting_human_review` lists all of them, the gallery says so at the top of its page, and the
manifest says `"reviewed": false`. An agent cannot manufacture a person's approval, and stamping one
with a human's name would be worse than having no approval machinery at all.
`docs/validation/08-the-fidelity-oracle.md` is the page addressed to the person who can end that,
and a suite holds the page to the code.

### The three tiers, and the localisation that is the actual gate

One specimen is asserted at three stages of one pipeline — `FragmentTree` snapshot, `DisplayList`
snapshot, rendered pixels — and the **first tier to differ** is the one that caused it, so a failure
reports `Layout`, `SceneBuilding` or `Painting` rather than reporting that the picture changed. Tiers
one and two need no painter at all.

Three tiers that always agree are one tier written three times, so what is asserted is the *pattern*:
moving a fragment reddens all three; **recolouring a paint leaves tier one green**, because a
fragment tree carries a `DecorationRef` — a bare number — and never a colour; corrupting the stored
image alone reddens only the third.

### The pixel tier, and a tolerance that was measured absorbing a real defect

Perceptual diffing with a structural metric, never a byte compare. It carries four numbers, and the
fourth is there because this crate's own PDF suite caught the first three failing: a word displaced
eight points is 0.85 % of a page and drags the mean SSIM only from 1.0 to 0.98, both of which a
cross-producer tolerance allows. **A fraction and a mean both divide by the whole page**, so neither
can see a local defect; the minimum over windows has no denominator, and
`Tolerance::worst_window_similarity` is what makes the tier local.

### The PDF pipeline, proved before the Windows sitting

`pdftotext -bbox-layout` over two of our own exports **names the word** that moved — not *"the pages
differ"* — and `pdftoppm` rasterises both sides with **one rasteriser** at one stated DPI, which is
the only construction in which "pixel perfect against PowerPoint" is a coherent phrase. Both readers
are external on purpose; `MJX_REQUIRE_TOOLS=1` turns an absence into a failure.

### The premultiplication decision, recorded before the first baseline

`mjx_paint::Pixels` is premultiplied and **stays** premultiplied — it is what the render targets
hold, and un-premultiplying on readback would be lossy at alpha zero. A PNG sample is not
premultiplied, so the conversion happens exactly once, at the file boundary, and ImageMagick reads
`#FF000080` back out of a half-alpha red to prove it.

### Also

* **`mjx-render-oracle`** — new, test-only, `publish = false`, outside the rank graph, one step below
  `mjx-reference-pack`, which is the only crate allowed to depend on it. The *authority vocabulary*
  moved down into it from the pack, because a workspace with two answers to "how much is this
  reference worth" has one too many and nothing may depend on the pack.
* **A PNG encoder and a matching decoder**, hand-written and dependency-free. MJXOFF-165's brief says
  to reuse `mjx_paint::export::png`; **there is no such module** — R09 never wrote one. Fixed-Huffman
  deflate over an LZ77 search, byte-reproducible, checked by ImageMagick rather than by itself.
* **SHA-256**, hand-written, checked against FIPS 180-4's vectors *and* against the system
  `sha256sum`.
* **The plate manifest** R11 and U01 load: PNG plus JSON, parsed back by the suite from the
  consumer's side rather than asserted as a string.
* **The gallery** — one self-contained HTML file, uploaded by a new `oracle` CI job on failure,
  because a failure a reviewer cannot see is a failure nobody fixes. It prints the *measured*
  `DrawReport::placeholders`, which is zero now that Phase G has landed, rather than a sentence about
  stand-ins that would have to be remembered and edited.
* **A regression arrives with its picture, and only then.** When a plate stops matching its approved
  baseline the gallery carries three images — the current render, what a person approved, and the
  difference amplified four times — and the manifest names all three. On a run that matches they are
  absent: a black rectangle under every green plate is one a reader learns to scroll past, and by the
  time a real diff appeared they would scroll past that too.
* **`docs/validation/08-the-fidelity-oracle.md`** — the page addressed to the person who can end the
  circularity, listed in the series index, and held to the code by a suite: it fails if it names a
  plate that does not exist, quotes a variable the code does not read, or goes on claiming nobody has
  looked once somebody has.

## [0.0.137] - 2026-09-08

**The reference pack — what the one Windows sitting needs, prepared in advance** (MJXOFF-207,
Phase G position 6 of 6, the epic's last child).

Seven questions have accumulated that **no agent can answer**, because each needs Microsoft Office on
Windows and a person to run it. Arranging that is expensive, so it should happen once and cover
everything at once. This release makes that morning four exports long.

### Added — `mjx-reference-pack`, test-only, outside the rank graph

A new crate that authors the artefacts and ingests what comes back. It has **no rank**, and for the
opposite reason the other three test-only crates have none: they sit below their consumers so they
can be reached from everywhere, and this one sits at the **top** — it names `mjx-pptx`, `mjx-docx`,
`mjx-geometry` and `mjx-paint` together, which no shipped crate could legally do. Nothing may depend
on it in either dependency section, and `xtask/tests/layering.rs` refuses the edge by name.

**Four artefacts**, generated reproducibly (`cargo run -p mjx-reference-pack -- generate`):

| File | Asks |
|---|---|
| `01-presets-at-their-defaults.pptx` | all 187 preset shapes at their own defaults |
| `02-presets-at-their-extremes.pptx` | the same 187 with **every handle at an end of its domain** |
| `03-type-specimens-and-hatches.pptx` | advance rulers and line pitch for five families, and all 54 preset hatches |
| `04-hanging-punctuation.docx` | whether Word hangs ASCII `,` and `.` past the measure |

The fourth is a `.docx` because **`w:overflowPunct` is a WordprocessingML setting**: there is no way
to ask a `.pptx` the hanging question at all.

**Four readers**, each of which refuses a number it could not see. `read_advances`,
`read_line_pitch`, `read_hanging` and `read_hatch_tiles` turn an exported PDF into answers through
`pdftotext -bbox-layout` and `pdftoppm`, and a probe whose words did not come back is `not evidence`
rather than an advance of zero.

**`docs/validation/07-the-reference-pack.md`** is the hand-off, a sibling of MJXOFF-130's own Office
pass, and `tests/office-exports/` is where the exports land. **It ships empty and no
agent may fill it**, for the reason the corpus above it ships empty: the value of an Office export is
entirely its provenance.

### Added — `Presentation::set_shape_adjustments`, the `a:avLst` writer by wire name

`set_shape_geometry` writes `mjx-dml`'s **typed** `ShapeGeometry`, and a deck of every preset *at an
extreme of its own handles* cannot be authored through it. The new call takes `&[(&str, i32)]` —
`adj`, `adj1`, `adj5` — in the file's own units, upserts each into the shape's `a:avLst`, and leaves
the `prst` token and every unnamed adjustment exactly as they were. All **285 adjustments across the
119 adjustable presets** are written and read back in `crates/mjx-pptx/tests/preset_adjustments.rs`.

### Fixed — the count in MJXOFF-206's hand-off, and one in `mjx-paint`'s documentation

* **"An extremes deck cannot be authored for 70 of the 187" is not what the numbers say.**
  `ShapeGeometry` has 118 variants, of which one is `Unmodeled`, so 117 presets are typed and 70 are
  not — that arithmetic is right. But **only 119 presets have an adjustment at all**, and of those
  **exactly two** are untyped: `sun` and `teardrop`. The other 68 untyped presets are `rect`,
  `ellipse`, `line` and their kin, which have no handle to move and therefore no extreme to author.
  Measured in `exactly_two_adjustable_presets_have_no_typed_variant`.
* **`mjx_paint::pattern` said "Nine are pictorial" and listed ten.** The list was right and the count
  was not; the length is now asserted.

### Found, and reported rather than changed

* **`Document::from_package` and `Presentation::from_package` refuse a package whose main part has
  been edited.** Both probe for it with `Package::part_bytes`, which answers `None` for a part in the
  `Edited` state — the state `part_tree_mut` leaves it in. The failure is `MissingDocumentPart`,
  naming a part that is present and correct, and both constructors document themselves as taking
  *"one authored part by part"*, which is exactly the case that does not work.
* **A probe that repeats a character measures the character in a context no ordinary text puts it
  in.** The advance ruler was built as `(run - probe) / (N - 1)` and read Arial's `f` at 260
  thousandths of an em against a published 278, and `1` at 482 against 556 — `ff` is a ligature, and
  a run of `1`s is shaped. The ruler now measures against an `HH` baseline box instead, which is
  exact and shaping-free; **89 of Arial's 92 probes then read back within 0.93 thousandths of an em
  of the published table** through LibreOffice's own export, and the run is kept as a second estimate
  whose disagreements name the shaping.

### The claim this release does *not* make

**Nothing here says any shape matches PowerPoint.** Every gate in the new crate passes with no
authoritative reference in existence, because the reference is the one thing an agent cannot produce.
A LibreOffice run is `Provisional`, `parity_count` over one is **zero by construction**, and the
gradient and hatch exclusions are attached to the *provider* so they lift by themselves when the
Office exports arrive. The suites say so in their own file names —
`the_plumbing_is_proved_and_not_the_fidelity.rs`, `an_excluded_result_is_not_evidence.rs`.

## [0.0.136] - 2026-09-08

**The provider wired in, and the placeholder proved gone** (MJXOFF-206, Phase G position 5).

Everything Phase G built could be true while the renderer still drew placeholders, **because the
provider is chosen by the caller.** This release makes the real one what a document is rendered
with, and — more to the point — makes "the placeholder is gone" a checked claim instead of a
sentence.

### Fixed — half of `DrawReport::placeholders` had never executed

The count is incremented in exactly two places in `crates/mjx-paint/src/plan.rs`: once under
`Command::FillPath` and once under `Command::StrokePath`. **Every stand-in scene in this workspace
was filled**, so the second line had never run. Replacing it with `std::process::abort()` left
`mjx-paint`, `mjx-scene` and `mjx-geometry` green — a mutation that should have aborted the process
and did not.

That is the exact shape of the defect the field exists to prevent, one level down: a counter that
never increments satisfies *"zero placeholders"* perfectly. An outlined shape is not exotic —
`straightConnector1` has no interior at all, and **63 of the 186 presets** end a contour without an
`a:close` — so a deck of connectors would have reported a clean fidelity render while drawing
framed, crossed rounded rectangles. The stroke arm is now asserted in five places: the software
painter, the GPU painter, both document exporters, and `mjx-geometry`'s own wiring suite. Mutating
the line now aborts.

### The wiring gate — `crates/mjx-geometry/tests/the_provider_is_wired_in.rs`

The existing placeholder-count cases build their scenes from the typed registry, draw them as fills
and lower them with `plan_frame`. Each of those three is a place a wiring defect can hide, and the
new suite closes all three:

- **the document's own `a:prstGeom` is the route.** All 186 presets are registered through
  `ShapeOutline::from_preset_geometry`, out of a `PresetGeometry` built the way a `.pptx` has it.
  That bridge was written, documented and exported in MJXOFF-202 and had **never been shown to reach
  a painter's report** — a bridge that resolves correctly and is wired to nothing renders exactly as
  many placeholders as no bridge at all;
- **both command kinds**, and each alone as well as together, so a page that silently skipped one
  cannot pass by reporting a lower number;
- **both orientations**, because `ss` is `min(w, h)` and a sheet laid out at one aspect ratio is one
  sample;
- **every pure-Rust painter** — `tiny-skia`, SVG and PDF — asserted in both directions over a deck
  of 186 shapes and over a scene that certainly contains one that cannot be drawn. R09 added three
  painters and asserted this field for none of them; the `wgpu` painter asserts the same pair in
  `crates/mjx-paint/tests/a_page_becomes_pixels.rs`, where a missing adapter is a named skip.

**All three producers of a stand-in are exercised, not one**: an unregistered handle
(`UnregisteredOutline`), `upArrow` — the single preset `ST_ShapeType` declares and
`presetShapeDefinitions.xml` defines nothing for (`UnseededShape`) — and `circularArrow` at `adj5`'s
own minimum, where `swAng` has no value (`SingularGeometry`). The three are asserted to land on
three *different* arms, so the list cannot quietly become one case written three times. And the
stand-in is asserted to be **`mjx-scene`'s own**, command for command: a provider that grew a second
framed rectangle of its own would pass every count and fail that comparison.

### The census — no shipped code renders with the stand-in, and every test that uses one says why

`crates/mjx-geometry/tests/the_stand_in_is_named_wherever_it_is_used.rs` reads the workspace off the
file system and asserts two things:

1. **exactly one shipped file constructs `PlaceholderGeometry`** — `crates/mjx-geometry/src/
   provider.rs`, the `UnknownShapePolicy::StandIn` fall-through. That is the whole of *"the
   placeholder is gone"*: no other `src/` path can put one on a page; and
2. **every other file that constructs one declares a reason**, on a line carrying `MJX-STAND-IN:`
   and at least sixty characters of prose. **Seventeen files carry one** — the tessellator's suites
   need a provider that answers every handle and do not care what it draws, and `mjx-scene`
   (rank 1.7) and `mjx-paint` (forbidden by name in `tests/the_seam_holds.rs`) *cannot* name the
   real one.

The idiom is `mjx-paint`'s `MJX-PAINT-SURFACE-UNSAFE`, and for its reason: a claim CI does not check
is a claim that quietly stops being true. The gate carries its own negative control, so it is shown
able to fail rather than merely observed to pass.

### The gallery, drawn through the renderer

`cargo run -p mjx-geometry --example painted_gallery -- <stem>` writes the same 186 plates as
`plate_gallery`, in the same grid, but draws none of them itself: every shape is a
`Geometry::Unresolved` in a `DisplayList`, resolved by a painter walking that list. It writes an SVG
through `SvgPainter` — whose root carries `data-mjx-placeholders="0"`, so the picture states its own
count — and a PNG through the pure-Rust `SoftwarePainter`, and it **refuses to write either** if the
report is not zero or the sheet is missing shapes. It is an aid for a person, not a gate: MJXOFF-201
§6 is unchanged, and the authoritative visual check is PowerPoint on Windows.

### Fixed — a count four comments quoted and nothing checked

Writing G06's hand-off meant restating *"sixty-four of the presets end a contour without an
`a:close`"*, which appears in four comments across `mjx-geometry` and `mjx-paint` and is sourced from
**no assertion at all**. The crate asserts a different quantity — `PATHS_THE_FILE_LEAVES_OPEN`, 70 of
the file's 319 *paths* — and a path count is not a shape count.

Measured: it is **63**. `every_preset_is_structurally_sound.rs` now carries
`PRESETS_WITH_AN_OPEN_PATH` beside the path-level constant and counts both in the same walk, so the
shape-level figure is checked where it was quoted. Nothing depended on the wrong number, which is
precisely why it survived four readings: an unchecked figure is not caught by being read.

### Documentation corrected

*"Every preset shape in this platform resolves to a stand-in today"* was true when it was written
and, in one wording or another, ran through **fourteen files** across `mjx-scene`, `mjx-paint` and
`docs/UI_PLATFORM_PLAN.md` — including the documentation on `DrawReport::placeholders` itself and
`mjx-paint`'s own end-to-end frame example, which built a stand-in and then asserted the page had no
placeholders. Every one now says what is true: a preset resolves to the document's own geometry,
which makes the flag a *signal* rather than a constant. `docs/UI_PLATFORM_PLAN.md`'s gap #1 is marked
closed with its evidence kept rather than deleted, and §1.11's prediction — *"swapping in the
generated table later is one implementation, not a rework"* — is recorded as having held.

## [0.0.135] - 2026-09-08

**Verification across all 186 presets — structural, differential and monotonic** (MJXOFF-205, Phase
G position 4).

It is mechanically easy to produce 186 shapes that resolve and geometrically hard to know any of
them is right. Nobody reads 3 923 guide formulas and the output is visual, so this release is three
gates that need no reference render, and the defect the first of them found.

### The monotonicity gate now covers 285 adjustments, not four

`an_adjustment_moves_the_shape.rs` states a direction in a shape's own words and asserts the
geometry moves that way; it covers `triangle`, `roundRect`, `rightArrow` and `pie`. The remaining
182 presets had never had an adjustment checked against a geometric expectation.
`every_adjustment_moves_its_shape.rs` sweeps **all 285 (shape, adjustment) pairs across 119
presets**, in both orientations, and asks four questions:

- **every one of the 285 moves its geometry** — no exceptions;
- **the axis it moves on is the axis its own `a:ahLst` handle declares** — a differential against a
  part of ECMA-376's file the path table does not read — with **eight** named exceptions, each a
  shape whose handle drags along one axis and whose geometry is arranged along the other;
- **it keeps its shape in its box throughout**, with **eleven** named exceptions carrying measured
  bounds in both orientations, and the worst shape that stays inside measuring 0.005 28 px against a
  0.01 px tolerance — so halving the tolerance would fail a correct shape; and
- **three adjustments reach a point their own formulas have no value at**, each at a stop a handle
  drag reaches, each answering `SingularGeometry` so a stand-in is counted rather than the page
  failing.

### Fixed — an arc on a degenerate ellipse landed a whole radius from the pen

Found by that gate, on its first sample. `parametric_angle` tested for a degenerate ellipse by
asking whether `wR·sin θ` and `hR·cos θ` were **both exactly zero**, and `sin π` is `1.22e-16` in
binary floating point. So an `a:arcTo` whose ellipse has one radius zero took the general branch,
`atan2(1.22e-16, -0.0)` answered `π/2` instead of `π`, and the quarter turn that invented put the
derived centre a whole `wR` from the pen. **`can` at `adj = 0` drew its top ellipse 80 device pixels
left of a box beginning at zero, and `leftBracket` at `adj = 0` drew a full 160 outside a 160-pixel
box** — both at an adjustment's own minimum, which a handle drag reaches. The guard is now on the
radii, which is what the function's own documentation always claimed. **No existing gate saw it**:
the box census resolves at default adjustments, the four monotonicity cases are four other shapes,
and the degenerate-size sweep asserts finiteness rather than position.

### A third route to the same geometry, through the parser

`the_third_route_is_the_parser.rs` writes each of the 186 out as `a:custGeom` XML, reads it back
through `mjx_xml::fidelity::parse`, and resolves it with `mjx-dml` alone — then compares **all three
surfaces**: 6 189 path commands, 362 text rectangles and 1 712 connection sites, in two
orientations. Every one agrees **exactly**, to 0.0 px and 0.0°. The route shares the arc
decomposition and the guide evaluator (there is one of each in the workspace, deliberately) and
shares nothing else: the table is read off the wire as `ST_AdjCoordinate` and `ST_AdjAngle` rather
than out of a `static`, and the map onto the page — `CT_Path2D`'s `@w`/`@h` coordinate box included
— is written a second time.

### The structural walk: correspondence, not counts

`every_preset_is_structurally_sound.rs` walks each of the 186 at **six sizes** and at **every
adjustment's own extremes**, and asserts the resolved command list *is* the table's step list with
exactly three transformations: an arc expanded into cubics, a `MoveTo` inserted where a step follows
an `a:close` (six presets, all accent callouts), and a second `a:close` suppressed (none). It also
asserts the map onto the box is affine, by resolving every preset in two boxes eight times apart and
comparing every point through the transform between them — 15 096 points, worst disagreement
6.1e-5 px.

### Each check proved able to fail

One mutation per check, each run over the whole table and each reddening **one shape and only that
shape**: a `pie` whose swing angle is written in degrees rather than the wire's sixty-thousandths
(the structural walk), a `triangle` whose apex guide reads `h` where the file writes `w` (the
differential, 20 px), and a `triangle` whose apex is pinned to a constant equal to its own default —
identical at its defaults and dead under a sweep (the monotonicity gate).

### Also

- `cargo run -p mjx-geometry --example plate_gallery -- out.svg` draws all 186 on one sheet with
  their boxes, text rectangles and connection sites. It is **for the user to compare against
  PowerPoint** and nothing gates on it: a picture that looks right is not evidence.
- `orientations()` moved into the suites' shared scaffolding; it had been written twice and was
  about to be written twice more.
- Corrected stale prose on `TextRectangle::Singular`, which still said four presets are singular
  there and named `parallelogram`. It is three, and `parallelogram`'s singular guide is one its
  *connection sites* read — measured in 0.0.134 and asserted since, while the sentence in
  `resolve.rs` went on saying otherwise.

## [0.0.134] - 2026-09-07

**The text rectangle and the connection sites — `a:rect` and `a:cxnLst`** (MJXOFF-204, Phase G
position 3).

A preset shape is more than its outline. `a:rect` says where text goes *inside* it and `a:cxnLst`
says where a connector attaches and which way it leaves. Both live in the same file MJXOFF-203 read
and are written in the same guide language, so both are extracted by the same parse and resolved
through the same guide environment and the same affine map the paths use — not a second
implementation of either.

**181 of the 186 declare a text rectangle** (`chartPlus`, `chartStar`, `chartX`, `line` and
`lineInv` do not), and **136 of those inset it from the shape's own box**. **173 declare connection
sites**, 856 in all; the thirteen that do not are the nine connectors — a connector has nothing to
connect to — plus the three `chart*` marks and `funnel`.

### Absent, unresolvable and broken are three different answers

The defect this had to avoid is invisible: a text rectangle that silently falls back to the bounding
box still renders text, just in the wrong place, and every test that asks *"did text appear"*
passes. So `mjx_geometry::preset_text_rectangle` answers with a four-armed `TextRectangle` —
`Declared`, `NotDeclared`, `Singular { guide }`, `Inverted { crossed }` — and the bounding-box
fallback is `or_bounding_box`, a **named call** a reviewer can grep for rather than a default
buried in the resolver.

All four arms occur in ECMA-376's own data. Three presets lose their rectangle to a singular `il` at
`adj2 = 0`, that adjustment's own minimum (`leftRightUpArrow`, `leftUpArrow`, `quadArrow`), and both
`ellipseRibbon`s cross their edges at `adj1`'s maximum, where the ribbon's body is squeezed to
nothing.

A connection site's list, by contrast, fails as a whole when one of its members has no value:
a connector names a site by `a:cxn@idx`, so dropping the fourth would renumber the fifth and attach
every connector after it to the wrong side.

### Two more defects in ECMA-376's own geometry file

MJXOFF-203 corrected eight malformed formulas. Two more turned up here, both found by a gate rather
than by reading, and both corrected in `xtask` with the file's own sibling rows as evidence and its
text guarded so a later edition cannot leave a silent rewrite behind:

- **`pie`'s `a:rect` is transposed** — `t="ir" r="it"`, a horizontal guide used as the top edge and
  a vertical one as the right. The rectangle it describes is inverted on both axes and reaches
  16.6 points below a 120-point shape. Every one of the other 180 maps `l t r b` to
  `il it ir ib`.
- **`squareTabs`'s sixth connection site reads `y="x1"`** — again a horizontal guide as a vertical
  coordinate. Its three siblings are the other three inner corners, the corrected point `(dx, y1)`
  is a vertex of the shape's own second path, and as written the site sits ten points below the
  shape's bottom edge.

### Every census is now taken in two orientations

`ss` is `min(w, h)`, so in a landscape box the shorter side is always the height — and every box and
every non-degenerate extent `mjx-geometry`'s suites had was landscape or square. An implementation
that read `h` where a formula says `ss` was therefore invisible to every gate in the crate. There is
now a portrait box with the same `ss`, and every census is taken in both.

It found two things a single aspect ratio had hidden, neither of them a defect: `chevron`'s text
rectangle collapses to the whole box in portrait, because its `il` is a `?:` whose condition is
`w - 2·x1` with `x1` a fraction of `ss` — **an else-arm no landscape box can reach** — and `chord`'s
inscribed rectangle clears its own chord in landscape and does not in portrait.

### Also

- The extractor now treats an empty XML element as a start immediately followed by an end. A
  self-closing `<gdLst/>` previously set a section flag that was never cleared, which would have made
  every later element of that shape unreadable. ECMA-376's file writes none, so the output is
  unchanged; the latent bug is closed.
- `crate::seed`'s hand-written reference gains `rect`'s and `ellipse`'s text rectangles and
  deliberately gains nothing else. A text rectangle is a design decision and a connection site's
  placement a convention, so transcribing the other four would have been a copy of the file wearing
  a different hat. `ellipse`'s is the exception worth having: the largest inscribed rectangle has
  half-axes `a/√2`, `b/√2` — a theorem, and a second measurement the file could have disagreed with
  while drawing the identical outline. It agrees to 0.0003 device pixels.

## [0.0.133] - 2026-09-07

**All 186 preset shapes ECMA-376 defines, extracted from `presetShapeDefinitions.xml`**
(MJXOFF-203, Phase G position 2).

MJXOFF-202 built the machine and seeded it with six shapes transcribed by hand. This fills it from
the normative file: every shape's whole `a:gdLst` and `a:avLst` in declaration order, every
`a:pathLst` with each path's coordinate box, its `@fill`/`@stroke`/`@extrusionOk` flags and its
ordered steps. `mjx-geometry`'s provider needed no structural change to consume it.

**186, and not 187.** `ST_ShapeType` declares 187 values and ECMA-376's own geometry file has no
`upArrow` element at all. That is a gap in the spec, not in the extraction, and it is named in
`mjx_geometry::PRESETS_WITHOUT_GEOMETRY` — derived from the difference between the enumeration and
the file rather than written down — so `upArrow` is the one preset that still reaches
`UnknownShapePolicy`.

### The differential, reported per shape

The strongest gate available was diffing this mechanical extraction against MJXOFF-202's hand
transcription from the spec's *prose*: two genuinely independent routes to one answer.
`crates/mjx-geometry/tests/the_two_routes_agree.rs` runs it on every build, over the defaults and
both ends of every adjustment's domain — 18 comparisons — and reports each with its derivation
class, because **they are not six confirmations**:

| Shape | Result | What the agreement is worth |
|---|---|---|
| `rect` | 0.00000 px | Fully independent, and proves the least: there is one way to draw a rectangle. |
| `ellipse` | 0.00000 px | **Structural coincidence, not a second measurement.** Four 90° `a:arcTo` quadrants clockwise from `(l, vc)` is the only structure DrawingML's arc semantics make natural, and MJXOFF-202 predicted the file would use it. |
| `triangle` | 0.00000 px | Paths independent; the `adj` domain came from the generated `adjustments_of`. |
| `roundRect` | 0.00000 px | Paths independent; the `adj` domain came from `adjustments_of`. |
| `rightArrow` | 0.00000 px | **Strong.** Seven points and eight guides, including `dy1 = */ h a1 200000` — the row MJXOFF-202 named as its own weakest point. The file writes the same eight formulas and the same seven points, in the same order. |
| `pie` | 0.00000 px | **Strong on the paths**, and it disagreed structurally: the file draws `moveTo(rim) → arcTo → lnTo(hc, vc) → close` and the seed draws `moveTo(hc, vc) → lnTo(rim) → arcTo → close`. Same wedge, rotated start point — exactly the difference MJXOFF-202 predicted, which is why the comparison is of resolved outlines and never of step lists. |

`triangle` also disagreed on *names*: the file's apex guide is `x2`, and its `x1` is a different
formula (`*/ w a 200000`, for the text rectangle). A diff on guide names would have reported a
contradiction where there is agreement to the EMU. The comparison is a symmetric point-to-segment
Hausdorff distance over flattened contours, and a one-digit slip in `rightArrow`'s divisor measures
30 device pixels against it.

### Added

- **`crates/mjx-geometry/src/generated.rs`** — 186 `PresetShapeDefinition` rows, 3 612 `gdLst`
  guides, 298 `avLst` values, 319 paths and 2 907 drawing steps, emitted by
  `cargo run -p xtask -- codegen`. Committed output, never a `build.rs`.
- **`PresetShapeDefinition::adjustment_values`** — the shape's whole `a:avLst`, not the subset
  `adjustments_of` exposes. An `avLst` entry no adjust handle references is not a user-facing
  adjustment and **is** a name the shape's `gdLst` reads: `pentagon`'s first guide is
  `*/ wd2 hf 100000`. Nine shapes could not evaluate a single guide without it.
- **`PresetPath::fill` / `stroke` / `extrusion_ok`**, and the consumer MJXOFF-202 required them to
  arrive with. `contours_of_definition` / `preset_contours` / `PresetGeometryProvider::contours`
  answer per `a:path`, each contour carrying its own treatment — the answer `ResolvedOutline`
  cannot hold, and the reason `arc` needs the flags at all: it strokes a `fill="none"` path and
  fills a `stroke="false"` sibling. `outline_of_definition` **acts** on the pair, leaving out the
  one contour in the whole file that is neither filled nor stroked
  (`flowChartMultidocument`'s third).
- **`GeometryError::SingularGeometry`**, and `has_no_geometry_to_draw` (was
  `is_a_gap_in_the_table`). ECMA-376's formulas divide and take square roots, and at the ends of an
  adjustment's domain the divisor can be zero — `circularArrow`'s `swAng` has no value at
  `adj5 = 0`, which is that adjustment's own *minimum*. Six presets have such a point; they answer
  with a counted stand-in under `StandIn` rather than failing the page, and never with a silent
  empty path.
- **`Derivation::ExtractedFromTheGeometryFile`**, the third value, carried by every generated row.
- **`mjx_geometry::seed::HAND_TRANSCRIBED_SHAPES`** — the six hand-written rows, now public and no
  longer the live table. They are the differential's reference; a reference nothing compares
  against is not a reference.

### Changed

- **`seeded_shapes()` returns the generated table**, 186 rows instead of six. Nothing else in the
  provider changed.
- **The `gdLst` is evaluated one guide at a time.** A guide with no finite value is left
  *undefined* rather than fatal, and so is every later guide naming it. In four of the ten shapes
  that have a singular point the guide is `il`/`it`/`ir`/`ib` — the **text rectangle**'s insets,
  which draw nothing — so the shape now draws where it previously could not. A *path* reading one
  is `SingularGeometry`; a malformed formula or a name nothing defines stays fatal, because those
  are table defects.
- **Eight formulas of `presetShapeDefinitions.xml` are corrected on the way out.** `+-` takes three
  arguments and these give it four, with a trailing `0` after an expression that is already
  complete; `circularArrow`, `leftCircularArrow` and `leftRightCircularArrow` cannot evaluate a
  single guide without the correction. Each has a sibling written a few guides earlier with three
  arguments and the same shape (`xG = "+- xH dxG 0"` beside `xB = "+- xH 0 dxB 0"`), so dropping
  the excess token is the file's own reading rather than a guess. Two gates hold the table honest:
  `apply_errata` fails if a corrected guide says something else, and `check_formula_arity` walks
  every formula afterwards and fails on any still malformed — which is what caught the two of the
  eight the first draft missed.
- **`xtask` gains a `mjx-dml` dependency**, so that arity gate checks against
  `GuideOperator::argument_count` — the same function the resolver checks against — rather than a
  second copy of §20.1.9.11's argument counts.

### Fixed

- `xtask/src/codegen/geometry.rs`'s header said the adjustment-bound closure was **335** guides,
  and MJXOFF-201 and MJXOFF-203 repeated the figure from it. It is **334** —
  `crates/mjx-dml/tests/guide_formula.rs` has asserted that number all along — and the geometry
  table's own suite now asserts it too, so the prose and the assertion cannot drift apart again.

## [0.0.132] - 2026-09-07

**`mjx-geometry`: the preset shape path tables, and the `GeometryProvider` that ends the
placeholder** (MJXOFF-202, Phase G position 1).

Until this release every preset shape in every deck resolved to the same framed, crossed rounded
rectangle — `mjx-scene`'s `PlaceholderGeometry`, built deliberately wrong so that a placeholder
render could never be mistaken for a fidelity one. This is the machine that replaces it, built and
gated against six shapes transcribed by hand so that it could exist before
`presetShapeDefinitions.xml` was available. MJXOFF-203 feeds the same machine 187 shapes instead of
six.

### Added

- **`mjx-geometry`, a new crate at rank 2.5**, between shared markup and the format tier. Every
  other rank in the workspace is justified by what its crates may not *reach*; this one is placed by
  what may not reach **it**. A preset path table is DrawingML, so it lives above `mjx-dml` (2.0) —
  which is what keeps `mjx-scene` (1.7) and `mjx-layout` (1.6) structurally unable to depend on it,
  and therefore keeps a display list from ever learning what a `.pptx` is. It is deliberately
  *below* the format tier, because a format crate is allowed to know what its own shapes look like.
  Grown in all three rank tables (`xtask/tests/layering.rs`, `CLAUDE.md`, `README.md`) and proved by
  mutation in both directions: `mjx-sml -> mjx-geometry` (2.1 → 2.5) and `mjx-geometry -> mjx-pptx`
  (2.5 → 3.0) each go red naming both crates and both ranks.
- **`PresetGeometryProvider`** — a registry of outline handles, each naming a `PresetShapeType`, the
  shape's extents and its `a:avLst` overrides. Answering a handle evaluates the shape's whole
  `gdLst` through `mjx-dml`'s own evaluator, resolves its path through `mjx-dml`'s own resolver, and
  maps the result onto the device-pixel box the seam supplied. Every answer carries
  `OutlineProvenance::Document`, which is the field R10 gates a fidelity render on.
  `ShapeOutline::from_preset_geometry` is the bridge from a document's own `a:prstGeom`.
- **`UnknownShapePolicy`** — what a provider does with a shape it cannot draw: `Refuse`, so the page
  fails rather than rendering a lie, or `StandIn`, which answers with `mjx-scene`'s placeholder
  *with its provenance intact* so the render is visibly wrong and countable. Neither answer is an
  empty path. The policy applies only to a gap in the table: a seeded shape whose guides will not
  evaluate is an error under both, because papering over it would hide the one failure the seed
  table exists to catch.
- **Six seed shapes** — `rect`, `ellipse`, `triangle`, `roundRect`, `rightArrow` and `pie` —
  transcribed from ECMA-376 Part 1 §20.1.10.56's descriptions and §20.1.9.11's formula language,
  each recording **how independently it was derived**, because MJXOFF-203's strongest gate is
  diffing its mechanical extraction against a transcription that did not come from the same file,
  and a comparison whose two sides share a source proves nothing.
- **Arc decomposition.** `mjx-dml` resolves an `a:arcTo` to two radii and two angles and stops,
  because how many cubics an arc becomes is a renderer's decision. `mjx_geometry::arc` is that
  decision, and it settles the question the spec answers only implicitly: **`stAng` is a true angle,
  not an ellipse parameter**, which the `arc` preset's own `cat2 wd2 ht1 wt1` start point proves —
  the derived centre lands on `(hc, vc)` under that reading and nowhere near it under the other.
- **`mjx-paint`'s seam gate now forbids naming `mjx-geometry`.** At rank 5.5 the edge would be a
  legal downward one, and a painter that built its own preset provider would be a painter that knows
  what an `a:prstGeom` is.

### Changed

- Nothing below the display list. `mjx-scene` still takes a `&dyn GeometryProvider` and still has
  never heard of OOXML; `mjx-paint` still does not name `mjx-dml`. That was MJXOFF-201 §3's rule and
  it held without amendment.

## [0.0.131] - 2026-09-07

### Merged `main` into the client-platform phase branch

**Two programmes ran concurrently and numbered independently, so `0.0.122` through `0.0.130` each
appear TWICE below** — once for the client-platform renderer (MJXOFF-155, `mjx-tokens` through
`mjx-paint`) and once for the validation and corpus work on `main` (MJXOFF-128, MJXOFF-130 and the
Phases B–F programme). Neither set is wrong and neither is renumbered: nothing here is published,
the two histories describe disjoint crates, and rewriting either would break the commit trail that
`MJXOFF-<n>` references depend on. **This entry is the seam.** From here the numbering is single
again, and the renderer side is the one that renumbered.

The client-platform entries come first, then `main`'s.

## [0.0.130] - 2026-09-07

**`mjx-paint` part 2: the `tiny-skia` software painter, the PDF and SVG exporters, and the
cross-painter gate they exist to make possible** (MJXOFF-164, Phase R position 9).

**Three more painters against the same contract.** `SoftwarePainter` executes a `FramePlan` on the
processor with no graphics stack, no window and no `unsafe`; `PdfPainter` and `SvgPainter` write
documents from the same lowering. All four are `Painter`, so a caller — and R10's oracle — drives
them through one loop.

**The software painter is not a fallback, it is what makes the rest of the programme testable.**
Every golden image from here on is taken headlessly through it, and it is the guarantee that a fully
pure-Rust path to pixels always exists — which is half of what made 0.0.129's amendment to the
pure-Rust rule a boundary rather than a concession. `tiny-skia` is used for scan conversion and pixel
storage only; the *shading* mirrors `backend/shaders.wgsl` function for function, because a second
painter that shaded differently would make the comparison compare two shaders rather than two
rasterisers.

**Cross-painter equivalence, and the way it degrades into nothing.** `compare_painters` renders one
display list through two painters and reports where they disagree. If the GPU painter is unavailable,
"the painters agree" silently becomes "`tiny-skia` agrees with itself", so the **library refuses** two
painters with the same name (`PaintError::PaintersNotDistinct`), before either is asked to draw. Over
a page using all nine commands the two agree on 99.6 % of pixels, and on all seven effect kinds to
within one level of 255.

**PDF text is text.** A run becomes a `/Type0` font with `/Identity-H` encoding, a `/CIDFontType2`
descendant with an `/Identity` `CIDToGIDMap`, an embedded subset in `/FontFile2`, and a `/ToUnicode`
CMap — which is the part that decides whether a reader can extract anything at all. `pdftotext`, a
reader this project did not write, reads the word back out; checking our own export with our own
reader would prove nothing. Gradients are PDF shadings built from the same 256-texel ramp both
rasterisers sample, hatches are tiling patterns from the same fifty-four masks, group opacity is a
transparency-group form XObject, and text filled with a gradient uses text render mode 7 so it stays
selectable. PDF has no blur operator, so every effect that needs one is rasterised through the
software painter and embedded — the documented fallback.

**SVG is vector output and a readable view of the display list.** Every element carries
`data-mjx-command`, `data-mjx-table`/`row`, `data-mjx-paint`, `data-mjx-role` and
`data-mjx-provenance`, so *"the third shape is the wrong colour"* becomes *"command 12 names paint
row 4"*. Validated with `xmllint`.

### Font subsetting, in the font engine

`mjx_text::subset_truetype` cuts a face down to the glyphs a page drew. It **truncates rather than
renumbers**: every glyph keeps its own id, so a composite glyph's component ids are correct because
they were never touched — which is where every subsetter bug lives. A `CFF` face comes back whole and
says so. `FaceReader` also grew `outline` (a glyph's path at a size) and `for_each_mapped_character`
(the `cmap`, read backwards, for a `/ToUnicode` map).

### Defects found and fixed

* **`plan::draws_behind` decided nothing.** 0.0.129 documented it as the single place all four
  painters learn an effect's ordering from, and the `wgpu` painter called it as
  `let _ = draws_behind(..)` — computed and discarded — while each arm hard-coded its own ordering.
  Flipping the function failed a test and changed no pixel; swapping two pushes in the shadow arm
  painted every shadow **on top of its shape** and left the suite green. Both painters now assemble
  an effect's composite steps from it and from the new `replaces_subtree`, and where the ink lands is
  asserted on pixels by region.
* **An inner shadow's offset moved its mask as well as its blur**, so it leaked outside the shape it
  is inside. The offset now shifts only the source lookup, in the shader and on the processor.
* **`Effect::new` leaves both reflection alphas at zero**, so every reflection this workspace had ever
  constructed was invisible — and two painters drawing nothing agree perfectly.
* **The seam gate at rank 5.5 had three holes** that compose into a one-commit escape to the facade:
  `mjx_ooxml` was absent from its forbidden list (only `mjx_ooxml_types` and `mjx_ooxml_core` were),
  the manifest scan read `[dependencies]` alone and missed this crate's own target-specific table, and
  it read only the `name.workspace = true` spelling. All three closed, and the scanner now has its own
  instrument test.
* **Sixteen public items were reachable from nothing**, including `Viewport::physical_x`/`physical_y`
  and `DesktopWindow::requesting_redraws_through`. A new reachability gate covers functions,
  constants, enum variants **and struct fields**.

### Supersession

`PLAN.md`'s Phase 7 line describes an IR → SVG → raster → PDF chain. **It is superseded**: both
exporters consume the display list directly through the same `plan_frame_with` every painter uses,
and neither is built out of the other.

## [0.0.129] - 2026-09-07

**`mjx-paint`: the `Painter` contract, the `wgpu` painter, and the two architecture rules that had
to change** (MJXOFF-163, Phase R position 8).

**Where pixels first appear in this programme.** A display list becomes a frame: tessellated path
fills and strokes, batched glyph quads out of the atlas, pictures, the fifty-four preset hatches,
gradients, exact path clipping through a stencil buffer, opacity groups, five blend modes, the seven
DrawingML effects as offscreen subtree renders, and four-sample multisampling — on Vulkan, Metal,
Direct3D 12, OpenGL ES / WebGL 2 and WebGPU, from one shader and one pipeline layout.

### Two `CLAUDE.md` rules amended, in writing rather than quietly

- **The pure-Rust rule now governs the *document graph*, not the workspace.** A pixel cannot reach a
  screen without the operating system's graphics stack, and `wgpu` links `ash`, `metal`/`objc2` and
  `windows-rs`. Ranks 0 through the facade stay pure Rust; **`mjx-paint` at rank 5.5 is the declared
  platform boundary** and the only crate that may link a graphics API. `tiny-skia` remains a
  *required* second painter (R09) precisely so a fully pure-Rust path to pixels always exists.
- **`mjx-paint` is the fourth crate with a local `#![allow(unsafe_code)]`**, with exactly one
  hand-written `unsafe` block: the surface created from a window handle the shell supplied. CI greps
  `crates/mjx-paint/src` for the keyword and fails on any line without the
  `MJX-PAINT-SURFACE-UNSAFE` marker, in the same job that guards `bindings/*/src`.

### The rank does not protect the painter's own edges, and something else had to

Rank 5.5 sits above the facade so that *nothing in the document graph can depend on `mjx-paint`* —
which is what keeps a GPU out of `bindings/mjx-python`. But the layering gate refuses only edges that
point up or sideways, so at 5.5 every crate in the workspace is a legal dependency of the painter.
`crates/mjx-paint/tests/the_seam_holds.rs` is what holds the architecture's second seam instead: it
refuses `mjx-layout`, `mjx-dml`, every format crate and the facade in the painter's source, asserts
the manifest exactly, and confines `mjx-text` to the **one** file that adapts R04's atlas delta —
asserting both that no other file names it and that that one still does.

### `mjx-scene`: a mesh now says where its outline came from

`ResolvedOutline::provenance` was written once and read zero times: `outline_of` dropped it at the
single point where the crate consumes a geometry provider, so **a painter could not tell a
placeholder rounded rectangle from the document's own shape**. Since every preset shape resolves to
a placeholder today, the guard R10's fidelity rule depends on did not exist. `SceneMesh` now carries
a `Provenance` — origin and the provider's label — populated by `tessellate_scene` through the new
`Tessellator::fill_resolved` / `stroke_resolved`; the painter counts them into
`DrawReport::placeholders` and paints them in a warning colour.

### Added

- `crates/mjx-paint` — `Painter`, `SurfaceHost`, `Viewport`, `Frame`, `Resources`, `AtlasSource`,
  `ImageSource`, the GPU-free `FramePlan` lowering, the budgeted `TexturePool`, the fifty-four
  preset hatch masks and the gradient-ramp resolver.
- `mjx_scene::Provenance`, `SceneMesh::provenance`, `Tessellator::fill_resolved` and
  `Tessellator::stroke_resolved`.
- A `render` CI job on a software Vulkan implementation with `MJX_REQUIRE_GPU=1`, so a missing
  device there is a failure rather than a silent skip.

## [0.0.128] - 2026-09-07

**`lyon` tessellation and the geometry-provider seam** (MJXOFF-162, Phase R position 7).

Paths become triangles, above the display list and below every painter. Tessellating here rather
than in a painter is what makes the result deterministic across platforms — R10's golden images rest
on it — testable without a GPU, and shared by four painters and two exporters. It is also why this
platform's vector rendering is a *tessellation* pipeline and not a compute-shader one: `wgpu`'s
WebGL2 backend has no compute stage, and the browser is a target.

Real preset geometry is **deliberately not built here**, by decision: it depends on context the
concurrent MJXOFF-88 programme supplies. What ships is the seam and a stand-in behind it, and every
gate above is written against paths that are *not* the stand-in.

### Added

- **`GeometryProvider`** — one method, no OOXML in its signature, taking the shape's box as a
  `SceneRect` in device pixels. Not `mjx_layout::Extent`, which is a *page count* carrying an
  `ExtentPrecision`: a signature that took one as a size would compile, read plausibly and mean
  something else.
- **`PlaceholderGeometry`** — the first implementation: a framed, crossed rounded rectangle at the
  shape's own box, deliberately not a shape DrawingML defines, carrying `OutlineProvenance` and a
  label naming the handle it stands in for, so a placeholder render can never be mistaken for a
  fidelity render.
- **`Tessellator`** — fills under both winding rules; strokes with every join, cap, miter limit,
  preset dash and compound band; beziers flattened to a tolerance derived from the scale bucket the
  record already carries. Degenerate paths — zero-length, self-intersecting, `NaN`, coordinates past
  every limit — produce empty or clamped meshes and never panic.
- **`MeshCache`** — triangles kept per `(path, style, scale bucket)` under a byte budget, keyed on
  the path's coordinate **bit patterns** rather than on a hash of them, so a collision cannot hand a
  painter somebody else's shape.
- **`tessellate_scene`** — every mesh a display list needs, in paint order.
- **`lyon_tessellation`** as a workspace dependency of `mjx-scene` alone, and **`mjx-dml` as a
  `dev-dependency` of `mjx-scene`** — never a dependency: rank 2.0 above rank 1.7 is the edge the
  layering gate exists to refuse, and the exemption buys a seam test satisfied by DrawingML's own
  resolved `custGeom` rather than by a second invention.

### Fixed

- **A latent panic on untrusted input in the display-list decoder.** `SECTION_SLOTS` was the literal
  `14` and the decoder wrote `sections[kind_value]` — a direct array index driven by input bytes, in
  bounds only because the section vocabulary happened to stop at thirteen. A fourteenth section kind
  would have made a display list *from a file a reader opened* index out of bounds. The slot count is
  derived from `SectionKind::ALL` now, with a compile-time assertion that every wire value has a
  slot; the write is a `get_mut`; and the header's section-count bound is expressed against the
  vocabulary rather than against the array.
- **The four-byte section alignment was held by arithmetic and asserted nowhere.** The writer pads
  nothing between sections, so a stride that is not a multiple of `SECTION_ALIGNMENT` makes a table
  of an odd number of records push the next section onto an offset this crate then rejects. Asserted
  at compile time and in `tests/the_encoding_is_pinned_to_literals.rs`.
- **The encoding gate sampled the section vocabulary rather than sweeping it**, so a kind could be
  added, or renumbered, with no byte literal disagreeing. Every kind's wire value and stride is now
  pinned to a hand-written table, and a real thirteen-section blob's rows are compared against it.

## [0.0.127] - 2026-09-07

**The display list, and its flat binary encoding** (MJXOFF-161, Phase R position 6).

The upper of the architecture's two seams. `mjx-layout`'s `FragmentTree` is the seam above which
nothing has heard of OOXML; `DisplayList` is the one **below which nothing has heard of a font, a
layout algorithm or a document either**. That is what lets four painters — GPU, software, PDF, SVG —
consume one output, and what will later let the same bytes cross a transport boundary without a
redesign, because they are already bytes.

### Added

- **`crates/mjx-scene`, rank 1.7** — a new crate depending on `mjx-ooxml-core`, `mjx-tokens`,
  `mjx-text` and `mjx-layout`, and on **no format crate and not on `mjx-dml`**. All four rank tables
  grew: `xtask/tests/layering.rs`, `CLAUDE.md`, `README.md` and `docs/UI_PLATFORM_PLAN.md` §7.
- **Nine commands** — `PushTransform`, `PushClip`, `PushOpacity`, `PushEffect`, `Pop`, `FillPath`,
  `StrokePath`, `DrawGlyphs`, `DrawImage` — and the balanced push/pop stack a malformed stream
  cannot violate.
- **The paint vocabulary**: solid; linear, radial and path gradients with DrawingML's full stop, tile
  and flip semantics; all 54 preset patterns; picture fills with crop, tiling and the image
  adjustments DrawingML defines (alpha, luminance, greyscale, duotone, colour change). Neutral
  names, none of `mjx-dml`'s types.
- **The effect vocabulary**: blur, glow, outer and inner shadow, soft edge, reflection and fill
  overlay, composed as a **DAG** in topological order. Declared here as data; executed in R08/R09.
- **The flat binary encoding** — a versioned, typed-record arena in one `Vec<u8>`: a 32-byte header,
  a section table with a row per non-empty table, and thirteen sections of which ten have a fixed
  stride so entry *n* is an offset multiply. Readable without deserialisation, cacheable to disk,
  and diffable frame to frame.
- **`build_scene`** — a `FragmentTree` in, a `DisplayList` out, driven only by fragments. `place_run`
  and the glyph atlas are called *here*, because this is the first layer that knows the device scale.
- **A reachability gate over the public surface** that sees **enum variants** as well as functions
  and constants. It found three orphans in `mjx-scene` on its first run, all three removed or given
  a test.

### Changed

- **`crates/mjx-layout/tests/public_surface_is_reachable.rs` now scans enum variants too.** It found
  three in `mjx-layout` — `ExtentPrecision::Exact`, `LayoutError::PageBeyondContent` and
  `ChangeKind::Reformatted` — each constructed nowhere in the workspace. They are recorded in a new
  `VARIANTS_ALLOWED_WITHOUT_A_CALLER` list with the reason, because giving one a producer is a
  contract decision rather than a gate's to make. `ExtentPrecision::Exact` in particular means the
  estimated-against-measured distinction R13 was told to depend on has one realisable value today.
- **`docs/UI_PLATFORM_PLAN.md` §7 corrected `mjx-scene` from rank 2.6 to 1.7.** At 2.6 the crate
  would sit *above* `mjx-dml`, which makes `mjx-scene → mjx-dml` a legal **downward** edge — so the
  layering gate, which only refuses an edge that points up or sideways, would have enforced nothing
  at all. `mjx-layout` was placed at 1.6 for exactly this reason.

## [0.0.126] - 2026-09-07

**The box model contract** (MJXOFF-160, Phase R position 5).

The layer the whole client-platform architecture is organised around. `mjx-layout` defines what a box
model *is* and what it produces, and nothing else: no document format is laid out here, and no OOXML
type may appear in its public API. Above a `FragmentTree`, scene building, painting, hit-testing,
selection, caret placement, comment anchoring, accessibility and every exporter are written against
six fragment kinds and a `SourceRef`, and none of them can tell a `.docx` from a Markdown file. That
is what makes the box model swappable, which was the requirement this architecture exists to satisfy.

### Added

- **`crates/mjx-layout`, rank 1.6** — a new crate depending on `mjx-ooxml-core` and `mjx-text` and on
  **no format crate, ever**. `CLAUDE.md`'s rank table, `README.md`'s ladder and
  `xtask/tests/layering.rs` all grew the row; adding `mjx-pptx` to its manifest turns the layering
  test red naming both crates and both ranks, which is how the seam is held rather than asserted.
- **`BoxModel`** — `layout_page(content, page, constraints, resume) -> PageFragments`,
  `estimate_extent(content, constraints) -> Extent`, `invalidate(change) -> DirtyPages`, and
  `signature() -> ModelSignature`, with associated `Content` and `Error` types. `estimate_extent`
  takes the constraints as well as the content, which the specification's sketch did not: a page
  count is a function of the page, and an extent computed without one would be an answer to no
  question.
- **`FragmentTree`** — `BoxFragment`, `LineFragment`, `GlyphRunFragment`, `ImageFragment`,
  `ShapeFragment` and `TableFragment` in a flat arena with parent/child/sibling indices: one
  allocation for a page, iteration in memory order for a painter, no recursion anywhere. Transforms
  and clips live in shared side tables, so a page with no rotation stores exactly one `Transform`
  however many fragments are on it. Children are in paint order; the children of a `LineFragment` are
  in **visual** order while their `SourceRef`s stay **logical**, which is the contract half of
  `mjx-text`'s "shape in logical order, place in visual order".
- **`SourceRef`** on every fragment — a part number, a path of child indices and a character range,
  and **no OOXML type**. Six path segments are held inline and deeper ones spill to a shared `Arc`,
  because there is one of these per fragment and hundreds of thousands of fragments in a document.
  Ordering is document order, which is what lets an invalidation binary-search and a caret walk.
- **`Checkpoint`** — the continuation token that makes page 300 reachable without laying out 299.
  Carries the page it ends, a shared inspectable position, a `ModelSignature` and the box model's own
  opaque bytes under a 1 KiB ceiling. `Checkpoint::state_for` checks **both** halves — the right model
  *and* the right page — because a checkpoint from the right model and the wrong page produces a page
  that looks entirely plausible and holds the wrong content.
- **`SpatialIndex`** — a uniform grid in CSR form, built in bulk at the end of layout, with an
  oversized list for fragments too large to write into cells. Point and rectangle queries, and a
  `topmost_at` that answers what a click is about.
- **`LineComposer`** — the join to the text engine. Fits a line against a measure, shapes each item,
  and emits the segments in visual order. It **calls** `mjx-text` and never re-implements measurement.
- **`mjx_ooxml_core::measure`** — `Emu` and `Angle` moved down from `mjx-dml`, which now re-exports
  them. `mjx-layout` positions every fragment in EMU and may not depend on `mjx-dml`; a second `Emu`
  is the defect the layering rule exists to prevent, so the type moved rather than being copied.
  `Emu` gained saturating arithmetic, `from_twips`/`from_inches` and `from_emu_rounded`; **there is
  still exactly one `Emu` in the workspace.**
- **`ShapedRun` is `PartialEq`** — needed to prove that resuming a page from a checkpoint produces the
  *same fragments*, which is a question about values rather than about the `Arc` identity
  `shares_glyphs_with` already answered.

### Fixed

- **`LineBreaker::next_line` exempted every paragraph's last line from its own measure** (MJXOFF-158,
  found here). UAX #14 reports the end of the text as a *mandatory* break, and the loop returned at
  the first mandatory opportunity without measuring it — so `"a bbbbbbbbbbbbbbbbbbbb"` against a
  measure of five came back as one twenty-two-character line, discarding the fitting break at byte 2
  that the loop had already found. The end sentinel is now an ordinary candidate; a real hard break —
  a line feed, `U+2028`, a paragraph separator — is still taken whatever the measure says.

### Changed

- **`LineBreakOptions::default()` is now plain UAX #14** rather than Japanese typesetting, and the
  Japanese answer has a name: `LineBreakOptions::japanese_typesetting()`. The old default enabled
  `east_asian_rules` and `hanging_punctuation`, and JIS X 4051's hangable set contains the **ASCII**
  comma and full stop — so a caller who said nothing got an English paragraph whose line-final full
  stop did not count against the measure. Both flags are named for document settings (`w:kinsoku`,
  `w:overflowPunct`), and a document that carries neither has not asked for either; a `Default` that
  silently enables two settings the document did not write is a hidden policy, not a default.
  **What is deliberately *not* decided here:** whether Word hangs an ASCII full stop in a *Japanese*
  paragraph, and whether it treats a Latin paragraph in the same document differently. That is a
  measurement against Word, and the character set is left exactly as it was until the reference pass
  makes it. `mjx-text`'s suite gained the Latin case that was missing.

### Tested

- **A second `BoxModel` implementation with no OOXML in it** — `tests/support/plain_text.rs` reflows
  a `Vec<String>` into a fixed-width column, produces a real `FragmentTree` with real `SourceRef`s,
  paginates, resumes and hit-tests. An abstraction with one implementation is a guess.
- **Checkpoint resumption proved equivalent, not merely present** — pages 1..=*N* laid out in order
  and page *N* laid out alone from *N−1*'s checkpoint compare equal as whole fragment trees, down to
  the glyphs. Perturbing one byte of the continuation makes them differ, so the equality can fail.
- **The spatial index proved against brute force** — every point and rectangle query over randomised
  fragment sets equals a linear scan's, across fragment counts and rectangle sizes that move the grid
  through three shapes and both the cell and oversized paths.
- **`ShapedGlyph::unsafe_to_break` has its first consumer and its first assertions.** It had zero
  readers anywhere in the workspace, so no assertion could depend on it. `slice_width` reads it, and
  two fixtures show why: Carlito's `ffi` ligature in `"office"` leaves bytes 2 and 3 with no glyph to
  slice at, and Liberation Sans's kerned `"AV"` has a glyph at byte 1 that only the flag marks unsafe
  — the case that distinguishes reading the flag from ignoring it.
- **A reachability gate over this crate's own public surface.** Every `pub fn` and `pub const` in
  `src/` must be named somewhere else, or the suite fails and prints it. It found four orphans on its
  first run; two were deleted and two given tests. MJXOFF-155 §9 item 10 asked for a gate rather than
  a list, and this is that gate, scoped to the crate where a dead export does the most harm.

## [0.0.125] - 2026-09-06

**Glyph rasterisation and the scale-bucketed atlas** (MJXOFF-159, Phase R position 4).

`mjx-text` could say what glyphs a run becomes and where they sit in the face's own units. It can now
say what one of those glyphs *looks like* at a zoom level, and hold the answer under a byte ceiling
that a test measures rather than a document asserts. This is the first place in the client-platform
programme where one of `docs/UI_PLATFORM_PLAN.md` §12's performance budgets becomes a gate.

### Added

- **`raster`** — `GlyphRasteriser`, `GlyphRasterKey`, `FaceId`, `GlyphRender`, `GlyphRoute`,
  `GlyphBitmap`, `GlyphOutline`, `OutlineCommand`, `OutlinePoint`, `BitmapFormat`, `Hinting`,
  `ScaleBucket`, `SubpixelPosition`, `RasterStatistics`. Rasterisation is `swash` — pure Rust, built
  over `FontFace::data()` plus `index()` exactly as the shaper is, because `swash` reads fonts with
  `skrifa` and this crate reads them with `ttf-parser`: each parser reads the file for itself and
  neither is ever handed the other's view.
- **Two routes, and the threshold between them is a named constant.** Below
  `OUTLINE_PIXELS_PER_EM_THRESHOLD` (96 pixels to the em) a glyph is a coverage bitmap; above it, a
  path for R07 to tessellate. The constant carries all three reasons it is where it is — a bitmap's
  area grows with the square of the size, hinting stops mattering once stems are six pixels wide, and
  a path is correct at every zoom — so moving it moves all three together.
- **Colour glyphs rasterise to RGBA**, and the face's `colour_formats()` is what decides it: the
  `COLR`/`CPAL`, `sbix` and `CBDT` tables were probed once when the face was parsed, and the
  rasteriser asks that answer rather than re-opening them. A face carrying colour glyphs stays on the
  bitmap route at any size, because a layer stack has no single outline to hand a tessellator, and is
  bounded instead by `MAXIMUM_RASTERISED_PIXELS_PER_EM`.
- **Scale buckets.** `SCALE_BUCKET_STEP_PIXELS_PER_EM` quantises the raster scale to a quarter of a
  pixel per em, and a run is *positioned as well as rasterised* at its bucket's size —
  `RunPlacement::residual_scale` is the one number a painter applies to the whole run to reach the
  size that was asked for. Positioning at the bucket rather than at the request is what makes the
  technique exact (positions and images scale together, so no letter moves relative to another) and
  is also what makes it work at all: two sizes in one bucket place byte-identically, so the second
  costs no rasterisation.
- **Quantised subpixel positioning.** `SUBPIXEL_POSITION_COUNT` horizontal phases per pixel, so text
  does not snap to the pixel grid as it scrolls. `SubpixelPosition::split` returns the whole pixel
  and the phase together, because rounding to the nearest phase can carry into the next pixel.
  Vertical positions are snapped deliberately: a fractional baseline undoes the hinter's work.
- **`placement`** — `place_run`, `RunPlacement`, `PlacedGlyph`, `DeviceScale`. Walks a `ShapedRun`
  forwards, applies `ShapedGlyph`'s `x_offset`/`y_offset` — which is what puts a combining mark over
  its base instead of on the baseline — and produces a raster key and a whole-pixel position per
  glyph.
- **`atlas`** — `GlyphAtlas`, `AtlasEntry`, `AtlasPageIndex`, `AtlasDelta`, `AtlasUpload`,
  `AtlasPageCreation`, `AtlasStatistics`, `PreparedRun`, `PreparedGlyph`, `PreparedImage`. Best-fit
  shelf packing into `ATLAS_PAGE_SIZE_PIXELS`-square pages, eviction by whole page under
  `DESKTOP_GLYPH_ATLAS_BYTE_CEILING` / `MOBILE_GLYPH_ATLAS_BYTE_CEILING` (a quarter of §12's texture
  budgets each), and a per-frame `AtlasDelta` so a painter uploads only what changed. **The atlas
  never evicts a page holding a glyph the current frame has drawn**; when that leaves nothing to
  evict it returns `GlyphAtlasExhausted` instead, because a cache that throws its working set away
  satisfies every byte bound and draws nothing.
- **The budget, asserted.** `crates/mjx-text/tests/glyph_atlas_allocation.rs` is a harness-free
  binary installing `mjx-allocation-counter` — the workspace's one counting allocator, now on its
  third consumer rather than its second implementation. It asserts the ceiling from both sides (the
  atlas's own accounting *and* the allocator's), that eviction really ran, that the atlas is not
  empty, and that the frame being drawn survived; and it re-runs the identical workload under half
  the ceiling to prove the ceiling is what bounds it.

### Fixed

- **A panic on a malformed font, in a dependency, on the untrusted-input path.** The new corruption
  sweep in `crates/mjx-text/tests/glyph_rasterisation.rs` found that `read-fonts 0.41.0` — which
  `swash 0.2.10` pins through `skrifa 0.44` — indexes a zero-length slice when a `glyf` entry's
  `endPtsOfContours` wraps its point count to zero. One flipped byte in an embedded font reaches it.
  `read-fonts 0.43.3` fixes it upstream and the fix is out of semver reach, so `raster.rs` closes the
  boundary: the panic is caught, every piece of `swash` state it could have left half-written is
  thrown away, and the caller gets a typed `FontError::UnreadableGlyphOutline`. The test asserts the
  boundary actually fires, so it cannot quietly become dead. **The durable fix is a dependency
  decision and is recorded on MJXOFF-159 for the repository's owner.**

### Notes

- The `synthetic_font` test builder grew `glyf`/`loca` outlines and a `COLR`/`CPAL` pair, so the
  colour route and the rasterisation route are exercised without committing an emoji binary — which
  MJXOFF-157 already escalated as a repository owner's decision. It also now writes each glyph's left
  side bearing to match its outline, because a TrueType rasteriser shifts an outline by `lsb - xMin`
  and a bearing of zero silently stacked two colour layers meant to sit side by side.
- Two records that the MJXOFF-155 ledger left for whichever child came next, both corrected here.
  `cargo run -p xtask -- tokens` grew an **`--out-dir <directory>`** flag in `0.0.123` and shipped
  with no entry: it moves where the three token artefacts are *written* (and, with `--check`, which
  copies are compared) without moving where the source is read from, and it exists so that
  `xtask/tests/tokens.rs` can exercise the write path without truncating a file another test binary
  is reading in a concurrent process. And `crates/mjx-text/assets/fonts/README.md` cited
  `docs/UI_PLATFORM_PLAN.md` for a statement that document does not make — §10 names Caladea as a
  bundled substitute and says nothing at all about its licence.

## [0.0.124] - 2026-09-06

**Shaping, bidirectional resolution, itemisation, line breaking and hyphenation** (MJXOFF-158,
Phase R position 3).

`mjx-text` could say which face a run is drawn in and what its numbers are. It can now say what
glyphs the run becomes, in what order, at what positions, and where a line may end. A renderer that
draws one glyph per code point looks approximately right in Latin and visibly broken in half the
world's scripts; this is the layer where that difference is decided.

### Added

- **`shaping`** — `Shaper`, `ShapingRequest`, `ShapedRun`, `ShapedGlyph`, `FontSize` and
  `shape_uncached`. Shaping is `rustybuzz`, a pure-Rust port of the engine Office itself shapes
  with, built over `FontFace::data()` so no `ttf_parser` type crosses the boundary. Output is in the
  face's own units — never a pixel size — so a shaped run is reusable at any zoom, and
  `ShapedRun::advance_in_points` is where a size is finally applied. **Parley is deliberately not
  adopted**, and the reason is recorded in the crate and module documentation so it is not
  re-opened: it is a layout library, and this project's line and page decisions have to come out
  where Office's do.
- **`direction`** — `BidiAnalysis`, `ParagraphDirection`, `TextDirection`, `BidiLevel`,
  `DirectionalRun`, `DeclaredRunDirection`. UAX #9 through `unicode-bidi`, with the document's own
  declaration ahead of the content: `w:bidi` and `a:pPr/@rtl` set the base direction outright rather
  than being inferred by rule P2/P3, which gets a right-to-left paragraph that opens with a Latin
  word wrong. A run's `w:rtl` is expressed as a UAX #9 **embedding**, not an override, so a number
  inside it still reads left to right.
- **`script`** — `TextScript`, `ScriptRun`, `itemise_by_script`, `itemise_range_by_script`. ISO
  15924 codes rather than an enumeration that would have to grow with Unicode; `Zyyy`/`Zinh`
  characters extend the run they touch rather than splitting it.
- **`itemisation`** — `itemise`, `TextItem`. The three cuts a shaping call needs — embedding level,
  then script, then face — in that order, and the first caller of `FontRequest::requiring`, which is
  what makes R02's third tier answerable.
- **`line_breaking`** — `LineBreaker`, `break_opportunities`, `KinsokuRules`, `LineBreakOptions`,
  `LineBreak`. UAX #14 through `unicode-linebreak`, plus the East Asian rules Office applies on top:
  JIS X 4051's 行頭禁則 and 行末禁則 sets (`w:kinsoku`), a document's own sets
  (`w:noLineBreaksBefore` / `w:noLineBreaksAfter`), and hanging punctuation (`w:overflowPunct`).
- **`segmentation`** — grapheme-cluster and UAX #29 word boundaries, the granularity R11's caret and
  every selection extension will move by.
- **`hyphenation`** — the `Hyphenator` trait with `NoHyphenation`, `SoftHyphenHyphenator` (complete,
  and needing no language data) and `PatternHyphenator`, a full implementation of Liang's algorithm
  with a TeX pattern reader and an exception dictionary. **No language's patterns are shipped**: a
  pattern set is licensed data, and which to commit is the same kind of repository-owner decision as
  the bundled font faces.
- **`feature`** — `TypographyOptions`, `FeatureSet`, `FeatureTag` and the vocabulary a document's own
  properties map onto. The set is canonical (sorted, one entry per tag) because the shaped-run cache
  keys on it.
- **`cache`** — `ShapedRunCache`, `CacheStatistics`. Keyed on face identity, size, direction,
  script, language, features and text — the plan's `(font, size, features, text)` plus the three
  additions without which a hit would draw the wrong glyphs. Faces are compared by `Arc` identity and
  retained, which is what keeps the pointer sound. Least-recently-used, evicted in batches.
- **`FontError::ShapedRunTooWide`** — a run whose advances sum past what an `AdvanceWidth` carries is
  refused rather than wrapping into a negative width.

### Fixed

- **`AdvanceWidth::equals` has a caller and coverage.** It shipped in 0.0.123 as public API with
  neither, and its documented cross-em property was unverified — found by MJXOFF-157's review, when
  replacing its body with `std::process::abort()` stopped no test. `ShapedRun::occupies_the_same_width_as`
  is the caller, and three tests cover the case a naive `font_units == font_units` gets wrong.
- **The `wasm-pack` CI job, red since 0.0.122** (origin MJXOFF-156). `bindings/mjx-wasm/npm/package.json`
  still said `0.0.121` while the workspace had moved twice, and `build-npm.sh` refuses to build on the
  mismatch by design. The number is now correct **and the hole is closed**: the rule that three files
  carry the version — the workspace manifest, this file, and the npm package — is now written in
  `CLAUDE.md` and beside the version itself, not only inside the build script the person doing the
  bump never opens.
- **The `naming (suppress, not delete)` CI job, red since 0.0.122** (origin MJXOFF-156).
  `crates/mjx-tokens/src/generated.rs` spells `tracked_change_delete` / `trackedChangeDelete`, and
  the gate forbids `delete` as an identifier. The rule is not wrong and the token is not wrong: the
  gate exists because a *chart* element is suppressed rather than deleted, and a tracked change that
  removed text genuinely is a deletion — the same judgement already recorded for `mjx-docx`'s
  `RevisionKind::Deleted`. The name is also not this repository's to choose: it is generated from
  allr.work's own `--color-tracked-change-delete`. Allow-listed by exact token and exact file, with
  its reasoning, and probed both ways — an unrelated `delete_token` planted in that very file still
  fails the gate.
- **`the_parse_path_contains_no_unwrap_expect_or_panic` walked only the top level of `src/`** and
  asserted a floor of nine files against a crate that had ten. A crate laid out as
  `src/shaping/mod.rs` would have been green over code the walk never opened. The walk is now
  recursive and the floor is the crate's real file count, so a module added or removed without
  updating it fails.

### Changed

- `LineBreaker::next_line` reports the end of the text as `LineBreakKind::EndOfText` rather than
  `Mandatory`. UAX #14 calls it a mandatory break because there is nothing after it to break before;
  a caller that treated it as a hard break would draw a paragraph mark that is not there.
- An empty paragraph declared right-to-left keeps its direction. `unicode-bidi` reports no paragraph
  for an empty string, and the base direction fell back to left-to-right — which is what an author
  sees after pressing Return in a Hebrew document.

### Dependencies

`rustybuzz` 0.20, `unicode-bidi` 0.3, `unicode-linebreak` 0.1 and `unicode-script` 0.5, all used only
by `mjx-text`, all pure Rust, none of them adding a C dependency or an `unsafe` block to the shipped
graph. `rustybuzz` reads faces through the same `ttf-parser` 0.25 the crate already declared, so the
two can never disagree about a face. `mjx-text` cross-compiles for `wasm32-unknown-unknown` and
`aarch64-linux-android` unchanged.

## [0.0.123] - 2026-09-06

**The font engine: three tiers, a metric-compatible substitution table, and a substitution manifest
a user can read** (MJXOFF-157, Phase R position 2).

Fonts are the largest fidelity variable in the renderer, and metric compatibility is a correctness
requirement rather than a nicety. If a substituted face's advance widths differ from the original's,
every line breaks in a different place and pagination diverges from Office on page one. Everything
R19–R22 does with Word's reflow rests on this.

### Added

- **`crates/mjx-text`** — a new crate at **rank 1.5**, depending on `mjx-ooxml-core` and
  `mjx-tokens` and on no format crate and not on `mjx-dml`. It answers two questions and no others:
  *which face should this run be drawn in*, and *what are that face's numbers*. Shaping is R03 and
  rasterisation is R04.
- **Face loading and metrics** through `ttf-parser`: units per em, the `hhea` and `OS/2` vertical
  metrics, cap height, x-height, italic angle, underline and strikeout, glyph advances and bounding
  boxes, variable-font axes, and the colour-glyph formats (`COLR`/`CPAL`, `sbix`, `CBDT`, `SVG `).
  `FontFace` holds the bytes and the once-per-face values; `FaceReader` is the borrowed view the
  once-per-glyph lookups go through, so a self-referential struct — and the `unsafe` it would need —
  never arises.
- **The system font database** through `fontdb`, taken without default features so neither `memmap2`
  nor a fontconfig C library is ever linked. **An empty system tier is a supported configuration,
  not a failure**: iOS exposes no system font files to a sandboxed process, and reaching them would
  need CoreText, which this crate may not link.
- **The three tiers, resolved in order** — embedded in the document, installed on the device,
  bundled with the application, and then the tier-3 *policy*: eleven fetchable Noto subsets with
  their coverage and their download size. There is no transport in this loop, so a resolution that
  reaches tier 3 is a `FetchPlan`.
- **The substitution table**, sourced from `fontconfig`'s `30-metric-aliases.conf` — Calibri →
  Carlito, Cambria → Caladea, Arial/Times New Roman/Courier New → the Liberation family, plus Arial
  Narrow, Georgia, Symbol and the PostScript base-35. It is consulted **before** any blind fallback,
  and the blind fallback runs once after a whole font stack rather than once per entry.
- **Published reference metrics** for the originals, in `mjx_text::reference`, every number
  transcribed from outside this repository and carrying its citation and an authority flag: the
  (URW)++ base-35 AFM widths for Arial, Times New Roman and Courier New, `@capsizecss/metrics`
  4.2.0's measurements of Microsoft's own faces for their vertical metrics, and ECMA-376 Part 1
  §18.3.1.13's Maximum Digit Width for Calibri.
- **Metric compatibility measured at resolution time**, not only in a test: every substitution the
  table makes is compared against those references and the verdict — verified, divergent with the
  worst character named, or unverified with the reason — is written into the manifest.
- **The substitution manifest**, per document, queryable: what was asked for, what was used, which
  tier answered, whether the line breaks survive, and how many runs it affects. U08's font picker
  renders it.
- **Embedded fonts**, including the ECMA-376 / XPS obfuscation the `.docx` form uses: the key is a
  GUID whose sixteen bytes, reversed, mask the face's first thirty-two.
- **`crates/mjx-text/assets/fonts/`** — the five regular metric-compatible faces (Carlito, Caladea,
  Liberation Sans, Liberation Serif, Liberation Mono), 1.8 MB, each under the SIL Open Font License
  1.1 with the licence text committed beside it and a `README.md` recording every file's source
  version and SHA-256.

### Changed

- **`CLAUDE.md`'s rank table and `xtask/tests/layering.rs` both gain rank 1.5**, as they must.
- **The layering gate now counts exercised tiers from both ends.** `mjx-ooxml-core`, `mjx-derive`
  and `mjx-tokens` declare no workspace dependency at all, so no edge ever *leaves* their tiers and
  a list of outgoing edges could never cover them. `mjx-text -> mjx-tokens` and
  `mjx-text -> mjx-ooxml-core` are the first edges to reach two of them, and the gate now fails if
  nothing reaches a tier that ought to be reachable.

### Known limitation

- **Cambria → Caladea is recorded as unverified, deliberately.** Microsoft publishes no width table
  for Cambria and it is absent from the metric collections that carry Arial, Times New Roman and
  Courier New, so there is no independent reference to measure Caladea against. The pair resolves,
  and the manifest says the substitution is unproven rather than claiming a verification nothing
  stands behind. Filling it in needs one measurement of the real face on a licensed Windows
  machine — advance widths are facts about a font rather than the font program, so what that
  produces is numbers.

## [0.0.122] - 2026-09-06

**One design-token source, three generated consumers — and the contrast rule enforced rather than
documented** (MJXOFF-156, Phase R position 1).

The chrome is HTML and the document canvas is Rust, and **a canvas cannot inherit a CSS custom
property**. A token system that stopped at a stylesheet would leave the in-canvas UI — selection
handles, alignment guides, rulers, marching ants — visually detached from the application drawn
around it. So one source reaches three consumers that share nothing.

### Added

- **`docs/client-platform/data/tokens.json`** — the token source, in the W3C Design Tokens
  Community Group format. It carries the allr.work values **as measured** in `DESIGN_TOKENS.md` §1
  (read from the site's own stylesheet, not approximated) plus the three additions an editor needs
  and a marketing site does not: the derived dark palette (§2.1), the document-surface palette that
  keeps the page true white in both schemes (§2.3), and the semantic aliases. 92 tokens.
- **`cargo run -p xtask -- tokens`** — the generator, a subcommand of the existing codegen rather
  than a second generator, sharing its `rustfmt` pass, its plain writer and its committed-output
  doctrine. `tokens --check` regenerates in memory and refuses instead of writing, naming the file,
  the line and both spellings of the first value that differs.
- **`ui/tokens/tokens.css`** — every token as a flat custom property under the name the source
  stylesheet itself declares (`--color-paper`, `--radius-card`), which is what makes *"drop the
  platform into a host that already defines these properties and it re-themes with no code
  change"* true rather than aspirational — plus a colour-scheme layer resolving `--theme-*` and
  `--document-*` in the only cascade order that behaves: light by default, the system's preference
  unless the host asked for light, an explicit `data-theme` over both.
- **`ui/tokens/tokens.ts`** — one interface per group, the `Tokens` type, the constant that
  satisfies it, `ColorScheme`, and `customProperties`, the path → custom-property map a runtime
  resolver reads host overrides through. Property names are **camelCase**, from the same rule
  `bindings/mjx-wasm` applies to every method it exports.
- **`mjx-tokens`, a new crate at rank 0.2** — `Tokens`, `Tokens::DEFAULTS`, the eight value types
  (`Color`, `Dimension`, `Duration`, `CubicBezier`, `FontStack`, `Shadow`, `TokenValue`,
  `LengthUnit`), the `TOKENS` identity table, and `resolve`, which layers *explicit configuration →
  host-supplied overrides → generated defaults*. The two sources are treated differently on
  purpose: a host element carries properties that are not ours, so an unknown name there is skipped;
  explicit configuration is the caller's own, so an unknown name there is a typo, and a typo
  silently ignored is the classic theming bug. It declares **no workspace dependency at all** —
  `thiserror` and nothing else. `CLAUDE.md`'s rank table and `xtask/tests/layering.rs` both carry
  the row.

### The contrast rule is a build failure

`DESIGN_TOKENS.md` §2.2 measures `--color-green` `#2e9e63` at **3.39 : 1** on white — legal for a
fill, illegal for body text — and `--color-green-deep` `#1e7a49` at **5.34 : 1**, and calls getting
that backwards *"the most likely accessibility defect in the chrome"*. Every colour token in the
source now declares `usage` (`on-light-text`, `on-dark-text` or `fill-only`) and, where it is
meaningful, the `background` it was measured against. **There is no default**, because a default
would make an omission invisible. The generator refuses a token tagged for text that does not reach
4.5 : 1, quoting the ratio it measured; it also refuses a text tag that names a background of the
wrong lightness, a translucent text colour, an alias cycle, a dangling alias, two colour schemes
that disagree about their members, and two tokens that would reach the same custom property.

The measured ratio is recorded in all three artefacts for every colour that declares a background,
so a `fill-only` decision is auditable at the point of use rather than only at the point it was
taken. One such decision is new: `--color-honey-deep` `#b77e1f` measures **3.49 : 1** on white — the
honey ramp has no text-legal deep step the way the green ramp does — so it, and the tracked-change
colours built on it, are `fill-only`.

### The two gates, both divergence gates

*"The three artefacts are generated and committed"* is satisfied by three files nothing reads.

- **Derived, not hand-written** — `xtask/tests/tokens.rs` runs the real binary with `--check`, and
  the emitters' own tests add a token to a source and watch it appear in all three.
- **Equal to each other** — `crates/mjx-tokens/tests/artefacts_agree.rs` parses the emitted CSS and
  TypeScript from disk, with no help from the generator that wrote them, and compares every value
  against the Rust table. The three names a token goes by come out of `TOKENS` rather than being
  re-derived, because a test that recomputed `--color-ink-soft` from `color.inkSoft` itself could
  agree perfectly with a wrong rule.

### Changed

- **`xtask`'s JSON reader moved to `xtask/src/json.rs`** and grew number and boolean payloads and an
  ordered-member accessor. It was a private module of `xtask/tests/layering.rs` while that gate was
  its only consumer; the token generator is a second one, and an integration test cannot reach a
  binary crate's private modules, so the gate now pulls the one file in by path rather than keeping
  a copy. A workspace with two JSON readers in it has one reader too many.

## [0.0.130] - 2026-09-07

### The Office-authored corpus — the ingestion path, and the weakness it retires (MJXOFF-130, F3)

**Phase F's third child, and the last of the sixty-two-child programme. It builds the road; it
cannot supply the traffic.** No test in this repository has ever read a file Microsoft Office wrote —
the deepest weakness the project has, recorded as `R2` — and the one an agent may not close, because
the value of an Office-authored file is entirely its provenance. **The corpus ships empty, nothing is
marked, and nothing is tagged.**

### Added

- **`xtask/tests/office_corpus.rs`** — the corpus suite. It walks `tests/office-authored/` (**the
  corpus is the directory**, the same rule `mjx-fixtures` makes for every byte-identity corpus) and
  holds every file it finds to: per-part decompressed-payload identity and container-structure
  identity across an edit-free save (`mjx-opc`'s `roundtrip` semantics); every XML part through the
  fidelity tree (`tree_roundtrip`'s); **the same round-trip through the facade**, so `Deck`,
  `Document` and `Workbook` are held to markup nobody here wrote; `Package::validate` before and
  after; and A7c's child-order audit over Office's own output, which is the strongest available check
  that the generated `ChildOrder` tables say what Office actually writes.
- **`cargo run -p xtask -- validation-artefacts --ingest <file>`** — the other direction of the
  artefact command. Hand it something saved out of Office and it reports which validation entry the
  file answers, every check above, and **where it would be committed**. It copies nothing: committing
  a file is a decision taken against the redistribution rule, and a command that filed it would be
  taking that decision for the person running it.
- **`docs/validation/06-the-office-pass.md`** — the hand-off. The order to work through (`R1` first,
  and stop there if PowerPoint disagrees), what a failure looks like against a documented gap, what
  to save out of which application and where to put it, the six checks that settle a **decision**
  rather than report a fact, the seven that are **blocked** and whether the corpus unblocks each, and
  the two escalations and three unfixed defects the programme is handing over.
- **`mjx_schema_gate::audit_order_report`** and `OrderAudit` — the child-order walk without the
  panic, so a *reporter* can print the round-trip and package verdicts too.
  `audit_deck_order` and `assert_deck_is_in_schema_order` are now written on top of it: one walk,
  three callers, and the second of those opens the package once instead of twice.

### Changed

- **`tests/office-authored/README.md`** now carries the **redistribution rule** — a committed file
  must be one whose *content* we authored, started from *Blank* rather than from one of Office's
  templates, carrying nothing from anywhere else and no personal data, with rights that need no
  argument. It is checked per file, before committing, and **recorded in a table in that file**; an
  unclear case is left out and *said* to have been left out.
- **Three verification blocks, rewritten conditioned on the corpus rather than ahead of it.** The
  PowerPoint and Word gaps pages say what now exists and that it is empty; the Excel guide grows the
  *Built, not yet verified against Excel* section it never had, with the six rows MJXOFF-79's risk
  list implies and the entry id that checks each.
- **`mjx-schema-gate` is a dependency of `xtask`** rather than a dev-dependency of it. The ingest
  command reports the same schema and child-order verdicts a suite asserts, and it is a command
  rather than a test; the alternative was a second child-order walk inside `xtask`. Nothing shipped
  depends on the gate, and `xtask` is host-only, `publish = false` and outside the ranked graph —
  which `xtask/tests/layering.rs` already distinguishes. The gate's own documentation said it was a
  dev-dependency "of nothing else", which had been untrue since MJXOFF-122; it now says what is true.

### Fixed

- **`two_runs_produce_byte_identical_artefacts` no longer assumes an empty corpus.** It asserted
  `names.len() == AREAS.len()`, which would have started failing the day the first Office-authored
  original landed — a gate that breaks on the work it is waiting for. The expected count is now
  derived from how many areas have an original.
- **`docs/validation/02-risk-order.md` said "five" design questions and listed six.** The 0.0.129
  entry below already said six.
- **Three gates in `xtask/tests/validation_harness.rs` would have gone red the day the first
  Office-authored original landed**, and none of them for a reason that is this library's. Measured,
  not predicted: with a stand-in file in the corpus slot, `every_generated_artefact_is_a_valid_package`
  failed on `21 != 20` (a second hard-coded `AREAS.len()` beside the determinism one) and
  `every_generated_artefact_is_schema_valid_and_in_child_order` failed on `v-xlsx-02-edited.xlsx` for
  a `workbookPr@dateCompatibility` **LibreOffice** wrote — a deviation `tolerances.rs` already
  records for that fixture, reaching the gate through a path that consults no tolerance list. An
  `edited` artefact is mostly somebody else's file, re-emitted verbatim, so it is now held to **no
  *new* defect**: what the original arrived with is subtracted, and anything left is ours. The
  authored artefacts are unchanged — nothing in a file we wrote is excused.
- **The validation harness's schema half was skipping in every CI run.** `xtask/tests/` is reached
  only by `lint-test`, which has no `References/`, so
  `every_generated_artefact_is_schema_valid_and_in_child_order` validated **nothing** on CI and the
  child-order half carried the job alone. The `schema-validity` job now runs
  `cargo test -p xtask --test validation_harness --test office_corpus` under `MJX_REQUIRE_SCHEMA=1`,
  where the schemas are. Both suites are green there; the point is that nobody knew.

### The `mc:Ignorable` / `CT_Extension` seam — diagnosed, reproduced, and deliberately not tolerated

**The first real Excel workbook, and any file carrying an Office chart, will report a schema
deviation, and it is a defect of this project rather than of the file.** The gate validates the
markup-compatibility-*resolved* view of a part, because `mc:Ignorable` names attributes the base
schema has no declaration for; resolution removes an ignorable element together with its content; and
`sml.xsd`'s and `dml-chart.xsd`'s `CT_Extension` declare their wildcard as a bare
`<xsd:any processContents="lax"/>`, whose `minOccurs` therefore defaults to **1**. The emptied
`<ext>` is then rejected with *Missing child element(s)*.

`xtask/tests/office_corpus.rs` reproduces **three views** of one worksheet, authored there for the
purpose and presented as nothing else: as a producer writes it (rejected — `mc:Ignorable` is not
allowed, which is why the gate resolves at all), with the compatibility attributes removed and the
ignorable content kept (**validates**), and fully resolved (rejected). So the schema does not object
to the extension; it objects to the **hole** resolution leaves. `pml.xsd`'s own `CT_Extension` and
`dml-main.xsd`'s `CT_OfficeArtExtension` both say `minOccurs="0"`, which is why the defect reaches
presentations and documents through their *charts* rather than through their main parts — it is not
Excel's alone.

**It is not recorded as a tolerance.** A tolerance is for one file and one message and never for a
defect of ours; recording this one would file a gate defect as a quirk of somebody's spreadsheet, and
it would then look for ever like a property of the corpus. It is filed as **MJXOFF-196** with the
reproduction, the schema sweep behind it and three candidate fixes, and it is the one thing that goes
red when the first original lands: the `-edited` artefact built from it reaches
`assert_authored_deck_is_schema_valid`, which tolerates nothing. The reproduction fails the day the
seam is fixed, which is the signal to delete it.

### Where the line is drawn on an ingested file

An ingested file is **not ours**, and that decides what may fail a build. Byte identity at the
container and through the facade, the fidelity tree, a package defect *we* introduced by saving, and
a part out of `xsd:sequence` all **fail**. A defect the file **arrived** with is *reported* — A7b's
scope rule is that such a file must still open and re-save unchanged — and so is a part its producer
wrote that the ECMA-376 XSDs reject, because MJXOFF-103 measured Apache POI 5.5.1 writing an empty
`<c:tx/>` that `dml-chart.xsd` refuses, and reddening a build over somebody else's markup teaches
nobody anything.

The suite proves itself able to fail rather than asserting that it can: the same engine is run over
four deliberately broken packages — bytes that are not a ZIP, a package cut in half, a worksheet
renamed at the root, and a relationship with no target — and each must be caught by the check that
owns it, with a sound package as the control. The first spelling of the last one pointed at
`xl/theme/theme1.xml`, which `sample.xlsx` *has*: it was not a corruption at all, and `Package::validate`
was right to hold. A mutation has to be reachable before its verdict means anything.

## [0.0.129] - 2026-09-07

### The validation checklist — every entry, all three formats, ordered by risk (MJXOFF-128, F2)

**Phase F's second child. It writes every checklist entry and marks nothing.** 113 checks across
`docs/validation/`, each a stable id, the MJXOFF id of the child that shipped the feature, an
artefact, an object, an action, an expected result, a risk level, three call chains and a **blank**
result line. Judging what real Office renders needs a person with Office in front of them; that pass
is the user's, and nothing here stands in for it.

### Added

- **`docs/validation/02-risk-order.md`** — the order the pass is worked through, which is
  deliberately not the order the pages are numbered in. **R1 first, and still the single
  highest-risk item in the repository**: the 0.0.58 tier-5 change, where a non-placeholder shape
  takes the master's `p:otherStyle` / `p:bodyStyle` per §19.3.1.35, real PowerPoint is believed to
  match the *previous* behaviour, and the change is isolated in one revertible commit. Then R2
  through R7 unchanged from MJXOFF-63, then Word's five risk areas (MJXOFF-74) and Excel's five
  (MJXOFF-79). It also collects, in one table, the **six checks that are design questions rather
  than checks** — each says *record which happens* and names the decision that follows, and none may
  be marked `differs`, because there is nothing to differ from.
- **`docs/validation/03-presentations.md`, `04-documents.md`, `05-workbooks.md`** — 62, 26 and 25
  checks. Every harvested number from the Phase A children's own completion reports survives in the
  terms its report used: 44 pt and 28 pt from `p:txStyles`, Widescreen 13.333 x 7.5 in, `accent1` =
  `4472C4`, Calibri Light and Calibri, 41.5 / 42.5 / 43.5 rather than 19.2 / 21.4 / 16.7, **100000**
  for a square chevron and **200000** for a 2:1 one, 45 degrees and ~3 pt out and ~4 pt blur, slice 1
  exploded 25 % and slice 0 `2E75B6`, a polynomial trendline of order 3, a merged total row of one
  row by two columns, and all sixteen plot types with `c:stockChart` drawing high-low-close from
  three series.
- **`V-PPTX-07` (`geometry`) and `V-PPTX-08` (`chart-decoration`)** — two new areas in
  `xtask validation-artefacts`, in all three languages. They exist because writing the checks found
  harvested expected results with no file to check them against, and MJXOFF-122's own rule is that
  an entry may not describe an artefact nobody produces. `V-PPTX-07` is **the one artefact authored
  at 4:3** (`9_144_000` x `6_858_000`) on a slide taken from the layout, so A3's rescaled-placeholder
  question has a file at last; it also carries the two chevrons whose `maxAdj` guide (`*/ 100000 w
  ss`) answers 100000 and 200000, the four arc-tangent presets `moon` / `arc` / `circularArrow` /
  `gear9`, and a `custGeom` with all five of `a:avLst`, `a:gdLst`, `a:cxnLst`, `a:rect` and
  `a:pathLst` whose apex is placed by the guide `apex = */ w 1 2`. `V-PPTX-08` carries the three
  label tiers merged, the exploded and recoloured pie slices, the order-3 trendline extended two
  categories forward with equation and R-squared, two sets of error bars on one scatter series (one
  per axis), a `c:dPt` left dangling at index 2, a value axis bounded 0-25 and reversed — the only
  artefact that writes a `CT_Scaling`, where `c:max` precedes `c:min` — a chart detached from its
  embedded workbook, and the sixteen plot types four to a slide.
- **`xtask/tests/validation_calls.rs`** — the gate that makes the two machine-checkable claims in a
  page of prose actually checked. Every `Calls:` line is resolved against three surfaces that have
  nothing to do with each other: every `pub fn` inside an `impl Deck` / `impl Document` /
  `impl Workbook` in the facade, every `def` inside the three classes of the committed `.pyi`, and
  every `#[wasm_bindgen(js_name = "…")]` in the same three `impl` blocks of the WebAssembly binding.
  The TypeScript half is checked *against the Rust half of the same chain* — the documented
  camelCase name must be the `js_name` the binding publishes for that exact `snake_case` method — so
  a chain that renamed one half and not the other fails even though both names exist. And every
  artefact a check names must be a file this repository produces: a name `validation-artefacts`
  writes, a path under `tests/fixtures/`, or an example's source.

### Fixed

- **A three-language call chain named a method a default build does not publish.**
  `Deck::vml_part_names` is `#[cfg(feature = "vml")]` in both bindings, so neither the Python stub
  nor the WebAssembly surface has it; the entry that named it now says so and uses the modern half
  of the same hop. Found by the new gate while it was being written, which is what it is for.

### Notes

- **Seven checks name no artefact and say **blocked**, each with the reason and what would unblock
  it.** They are not padding: `ColorSpec` carries a colour's kind and value and **no transform
  children**, so nothing can author the `comp` / `gray` / `gamma` / `invGamma` that R3 is about;
  `CharacterPropertiesSpec` has no font setter, so nothing can author a `+mj-sym` reference;
  `set_shape_transform` writes **only the fields its argument names** — an unset field means *leave
  it alone*, never *clear it* — so nothing can author the rotation-only transform R7 is about; and
  the facade has no `add_alt_chunk`. The rest wait on MJXOFF-130's Office-authored corpus.
- **MJXOFF-108's 28-row comparison table is carried in untouched.** Its **Excel says** and
  **Verdict** columns are still empty and unmarked. `V-XLSX-02.1` points at it and adds nothing.
- **`MJXOFF-143` had already closed the whole-part re-flow limitation**, and PowerPoint's gaps page
  had already moved that row to *What used to be here*. This child did not move it; it records the
  closure in `V-PPTX-08.11` so a reviewer does not report a re-flow as a defect.
- The Word and Excel area lists were derived from **the facade's own module structure** read against
  each format's gaps page, not from ticket text. Excel's `features`, `names`, `preserved`, `print`
  and `tables` modules carry readers and removers and no authoring call at all, which is why those
  areas are recorded as deliberately uncovered.

## [0.0.128] - 2026-09-07

### The consolidated validation harness — the artefacts a human Office pass reads (MJXOFF-122, F1)

**Phase F's first child, and the first thing in this repository that admits what it cannot check.**
Every fixture here was written by this project or by LibreOffice; no test has ever read a file
Microsoft Office wrote, and no gate in this workspace can answer *does real Office render what we
intended?* This child builds everything that question needs except the answer.

**It marks nothing.** There is no `pass` in any result line, no verdict inferred from a LibreOffice
conversion, and no place where an agent stands in for a person with Office in front of them. A test
asserts that, over every page of `docs/validation/`.

#### `xtask` grows a fourth command

`cargo run -p xtask -- validation-artefacts [--format pptx|docx|xlsx] [--area <id or number>]
[--out <dir>] [--list]` writes two artefacts for each of **eighteen** validation areas — six per
format. Every one is produced through `mjx-ooxml` and names no crate below it, so the human pass
validates the facade and its error mapping as a side effect.

* The **authored** variant is built from `Deck::blank`, `Document::blank` or `Workbook::blank`:
  nothing is read from disk, so every byte is one this library wrote.
* The **edited** variant is built by editing an Office-authored original from
  `tests/office-authored/`. That corpus is MJXOFF-130's to fill and is empty, so every edit variant
  **skips by name** — the area and the exact path it looked for — and `MJX_REQUIRE_OFFICE_CORPUS=1`
  turns any such skip into a failure, the arrangement `MJX_REQUIRE_SOFFICE=1` already makes for the
  `office_open` canary.

The command lives in a new **library target** for `xtask`, because `xtask/tests/validation_index.rs`
is written against the area catalogue and an integration test cannot see a binary's modules. The
alternative — parsing the binary's `--list` output — would have made a text format the contract
instead of a type. `codegen`, `fuzz` and `corpus` stay private to the binary; nothing depends on
`xtask`, and `xtask/tests/layering.rs` still says so.

#### The same eighteen artefacts, in three languages

`bindings/mjx-python/tests/test_validation_artefacts.py` and
`bindings/mjx-wasm/tests/node/validation_artefacts.mjs` are the same eighteen generators, call for
call, and each compares its output against the Rust one **part by part, byte for byte**. That is
A10's acceptance test generalised from one walkthrough to the whole catalogue: a binding method
wired to the wrong facade method changes one part payload, and a human reading the file in Office
would never know why it looked wrong.

Both comparisons state their artefact set over the *filesystem* rather than over a list in their own
file, so an area added in Rust and not in a binding fails rather than being silently skipped.

#### The index, and why it is not a tautology

`docs/validation/01-index.md` binds every entry id to the artefacts it is read against, and
`xtask/tests/validation_index.rs` compares two lists that cannot drift together: the **entry** side
is parsed out of hand-written markdown, the **artefact** side is a `read_dir` of the directory the
`xtask` binary has just written. Both are floored against the catalogue before either comparison
runs, so a parser that stopped matching table rows fails rather than passing an empty comparison.
The edited column is checked as an *if and only if* against the Office corpus, so an empty corpus is
still an assertion.

#### Every artefact through the gates a machine can answer

`xtask/tests/validation_harness.rs` runs the ECMA-376 schema gate, `Package::validate` and the
child-order audit over all eighteen, and quotes the audit's per-part `elements_visited` counts —
89 parts audited, none of them vacuous. Preserved-foreign skips are pinned by *label*, so a part
that starts skipping under a new one fails rather than quietly widening what the gate tolerates. Two
runs of the command are asserted **byte-identical**, without which the three-language comparison
means nothing. One artefact per format is converted by LibreOffice as a canary — one conversion,
never a sweep, and never a verdict.

#### Documentation

* `docs/validation/00-method.md` — the entry-id scheme, the three risk levels, the result
  convention, how to file an issue, and the rule that keeps the exercise honest: **a documented gap
  is never a validation failure**.
* `docs/validation/01-index.md` — the eighteen entries, and the areas the harness deliberately does
  not cover, each with its reason.
* `tests/office-authored/README.md` — the corpus slot, its naming convention, and why it is not
  under `tests/fixtures/`.

MJXOFF-108's 28-row effective-cell-format table is carried in **unchanged**: its *Excel says* and
*Verdict* columns are still empty and unmarked, and nothing here touched them.

## [0.0.127] - 2026-09-07

### The cross-format consistency pass — one reading of the whole public surface (MJXOFF-118, E6)

**Phase E's last child, and the last cheap moment to rename anything.** Three formats were built in
three phases, months apart, by different agents following the same rules; rules produce consistency
locally, and only a deliberate cross-cutting read produces it globally. This is that read. Nothing
here changes a byte any file receives.

#### The shared-markup reachability table, and the test that keeps it true

The gate MJXOFF-82 named: **nothing in `mjx-dml`, `mjx-sml`, `mjx-chart`, `mjx-vml` or `mjx-omml` is
reachable from one format's facade surface but not another's without a written reason.**
`crates/mjx-ooxml/docs/shared_markup_reachability.md` (rendered as
`mjx_ooxml::shared_markup_reachability`) is that table, and
`crates/mjx-ooxml/tests/shared_markup_reachability.rs` re-derives it from the workspace on every
`cargo test` — the crate-level grid out of three `Cargo.toml`s, the 102-capability grid out of the
facade's own `src/`. A method added to one surface and not the others, a note no row cites, a format
crate that starts modelling a shared markup: each fails naming the row it is about.

What the derivation found:

- **`mjx-chart` came out symmetric.** 25 of 28 chart capabilities are on all three surfaces — and
  read the other way, *every* capability on all three surfaces is a chart capability. The three that
  are not each have a reason in the file format, not in this library.
- **Sixty-seven DrawingML capabilities reach `Deck` alone**, in four groups with four different
  reasons. The shape-properties one names an open seam rather than hiding it: `mjx-docx` already
  models `wp:spPr` as `mjx_dml::ShapeProperties`, but that type is interner-bound and the facade's
  boundary is not, and only `mjx-pptx` built the interner-free spec layer that crosses it.
- **`theme`/`color_map` reaching `Deck` alone is the clearest gap.** `mjx-sml` resolves
  `<color theme="N"/>` to a slot number and says the theme part is `mjx-xlsx`'s to fetch;
  `mjx-xlsx` does not fetch it, so an Excel theme colour comes back as a position where the same
  colour in a `.pptx` comes back as `RRGGBB`. No ticket owned this before the table did.
- **`mjx-vml` and `mjx-omml` reach no surface as types** — both interner-bound trees with no
  binding-friendly projection — and the bytes-and-identifiers surface that does exist is not
  symmetric either: `Deck` and `Workbook` have one, `Document` has none.

#### Excel's effective-properties guide — the third page, in one shape

`crates/mjx-xlsx/docs/effective_properties.md` joins `mjx-pptx`'s and `mjx-docx`'s, wired the same
way (a documentation-only module over `include_str!`, so its three examples are doctests and its
links are checked). It says the thing that makes Excel different rather than restating the others:
**Excel inherits nothing** — a cell carries an index, that index names a record, and that record
carries four more plus a fifth into a second table of the same records — which is why an
`EffectiveCellFormat` reports *which layer* answered where the other two report only a value.

`crates/mjx-ooxml/tests/effective_properties_shape.rs` is what keeps the three one shape: same
opening sentence, the same four load-bearing sections in the same order, real compiled examples on
each, each wired into its crate. Two `mjx-docx` headings were renamed to the shared spelling.

#### Binding parity, in both directions

`chart_series_references` was bound for `Workbook` and for neither `Deck` nor `Document` — a facade
method two languages could not reach. Both bindings grow it, the `.pyi` grows two entries, and with
those four **every `pub fn` on all three facade surfaces is bound in both languages**, the escape
hatches excepted.

The reason nothing caught it is the more useful finding: three of the six coverage suites carried an
explicit *"remove one binding and this goes red"* case and three did not — including **both halves
of the `Deck` pair**, which is the pair the specification names. The two missing guards are added.

#### Naming and shape

- `ChartLabelScope`'s `plot_idx`/`series_idx`/`point_idx` are spelled out and are `u32`; so are
  `ShapeInfo::index` and `LayoutInfo::{index, master_index}`. Both rows are in the *Unreleased —
  0.1.0* ledger. Python and TypeScript are unchanged: **both bindings already published these
  names and this width**, and five conversions are gone.
- `ErrorDetail` now states, as a table, what each of its five fields means for a slide, a paragraph
  and a cell — including the two Excel answers a caller would otherwise have to discover by
  experiment (a sheet is reported through `index`, and an Excel cell address populates neither `row`
  nor `column`).
- `mjx_sml::CellReference`'s constructors take `(column, row)` where thirty-odd methods elsewhere
  take `(row, column)`; the reason is now on the type rather than in a ticket. **Reordering remains
  the user's call.**

#### Counts that had already expired

Every count this child quotes was measured, and several it found were not: the Excel guide's page
count was written in three places as thirteen, fourteen and fifteen (it is seventeen — the numeral
is now in none of the three); `error.rs` under-counted `DocxError` by six variants and `XlsxError`
by seven; `README.md` still called the project PowerPoint-first and listed two test-only crates
where there are three; and three rustdoc sites still described
`crates/mjx-chart/src/workbook.rs`, which MJXOFF-99 deleted, one of them as the live rationale of a
test.

## [0.0.126] - 2026-09-07

**Excel's legacy surfaces** (MJXOFF-114, Phase E position 5): a cell comment, the Transitional VML
box that draws it, and the identifier hop from a sheet's modern markup to the legacy shape an OLE
object or a form control is drawn as.

### Added

- **`mjx_sml::comments`** — `CT_Comments`, `CT_Authors`, `CT_CommentList`, `CT_Comment`,
  `CT_CommentPr` and `CT_LegacyDrawing`, the last slot of `CT_Worksheet` that had an owner
  (rank 30). `commentPr@anchor` consumes MJXOFF-127's `ObjectAnchor` rather than modelling
  `CT_ObjectAnchor` a second time, and every placement goes through a generated child-order table.
- **The comment family on `mjx_xlsx::Workbook`** — `sheet_comments`, `comment_at`,
  `comments_markup`, `edit_comments_markup`, `add_comment`, `set_comment_text`, `remove_comment`,
  and the VML side: `sheet_vml_drawing_part`, `vml_drawing_markup`, `edit_vml_drawing_markup`,
  `with_vml_shape_for_comment`, `with_vml_shape_for_ole_object`,
  `with_vml_shape_for_form_control`. A comment is **two parts**, and `add_comment` writes all seven
  things that have to agree while `remove_comment` takes them away.
- **`SpreadsheetDefect::CommentWithoutABox` and `CommentBoxWithoutAComment`** — the two-halves
  invariant, checked by `Workbook::validate` over a saved package rather than asserted by the
  surface that writes it. Neither half of a comment names the other by relationship, so nothing in
  the packaging layer could ever have noticed half of one.
- **`mjx_vml::Drawing::shape_by_numeric_identifier`** and **`mjx_vml::shape_identifier_for_number`**
  — the shared half of the hop. SpreadsheetML names a shape by a *number* (`x:oleObject@shapeId`,
  `x:control@shapeId`, `x:comment@shapeId`) where PresentationML names it by the string that number
  appears in; the `_x0000_s` spelling and the three attributes producers put it in are stated once,
  in the crate both formats reach.
- **The same surface on `mjx_ooxml::Workbook` and on both bindings** — `sheet_comments`,
  `cell_comment`, `add_cell_comment`, `set_cell_comment_text`, `remove_cell_comment`,
  `vml_shape_id_for_ole_object`, `vml_shape_id_for_form_control`, `sheet_vml_part_bytes`, with
  `SheetCommentInfo` and `CommentBoxInfo`.
- **Three producer-written fixtures**, none of them this project's: `cell_comments.xlsx` and
  `legacy_form_control.xlsx` from **LibreOffice 25.8.7.3** driven headless over UNO, and
  `comments_third_party.xlsx` from **XlsxWriter 3.2.9**. The Excel guide gains
  *Cell comments and legacy content*, whose every snippet is a compiled doctest.

### Fixed

- **`mjx-opc` treated an edited VML part as though it were not XML.**
  `XML_CONTENT_TYPES_WITHOUT_SUFFIX` spelled its one entry `…vmlDrawing` while `is_xml_content_type`
  folds its argument to lower case, so the entry matched nothing and an authored `.vml` sat outside
  `Package::authored_xml_parts` — outside `Package::validate`'s relationship checks and outside
  every format layer's markup checks — from the day the list was written. A test now fails on any
  entry written in a spelling the fold would swallow.
- **Deleting one comment could delete every comment box on the sheet.** Removal matched the shape by
  its `@id`, and LibreOffice gives every comment shape in a part the same one. It matches by
  position now; the fixture that found it is the producer file, and the two-halves invariant is what
  reported it.

## [0.0.125] - 2026-09-06

**Charts on the Excel surface** (MJXOFF-111, Phase E position 4): the third host for one body of
chart logic, and the one chart case that exists nowhere else in this library — a chart whose data
source is a **live range in the same workbook** rather than an embedded copy.

### Added

- **The chart family on `mjx_xlsx::Workbook`** — fifty methods, every one of which resolves
  `(sheet, anchor)` to a chart part and then calls the identically-named function in
  `mjx_chart::chart_ops`. MJXOFF-103 moved that body down for Word; Excel is the third wrapper
  around it and adds no Excel-local chart path. The address is MJXOFF-107's anchor index, so
  `add_chart`'s return value is accepted by `remove_sheet_drawing_object` exactly as
  `add_two_cell_anchored_picture`'s is.
- **`Workbook::resolve_range_reference`** and the `ResolvedRange` / `ResolvedArea` /
  `ResolvedRangeCell` / `RangeCellValue` / `RangeProblem` report — a chart's `c:f`, resolved against
  this workbook's cells. It resolves a **reference** and does not evaluate a formula; a cell holding
  one answers with its cached value, as `cell_text` does. A quoted sheet name, absolute markers, a
  multi-area union, a 3-D span and a defined name (sheet-scoped winning over workbook-scoped, as
  §18.2.6 says) all resolve; every unresolvable case is a typed `RangeProblem` on the area it came
  from rather than a failure of the whole call.
- **`Workbook::chart_series_freshness`** — each series' cache set beside what its cells actually
  say, with **each named**. `values_agree` has three answers: `Some(true)`, `Some(false)`, and
  `None` for *cannot say* — the values are a literal, or the reference resolved to nothing. "The
  cells disagree" and "there are no cells" are different facts.
- **`Workbook::refresh_chart_cache_from_cells`** — the opt-in repair, and the exact counterpart of
  `refresh_chart_workbook` pointing the other way. Writing a cell deliberately leaves a chart's
  caches alone (this library recalculates nothing), so this is how a caller makes the chart draw
  what the sheet now says.
- **`Workbook::add_range_chart`** with `SheetChartSource` / `SheetChartSeries` — a chart whose `c:f`
  name cells in this workbook, whose caches are seeded from those cells, and which carries **no
  embedded workbook at all**. `Workbook::add_chart` writes the other kind, taking the same
  `ChartData` a slide and a document take.
- **`mjx_chart::ChartData::ranges`**, `ChartRanges` and `ChartSeriesRange` — where a chart's data
  lives, when it is not the companion embedded workbook. A source no range names is written as a
  **literal** (`c:numLit` / `c:strLit`) rather than falling back to `Sheet1!$A$2:$A$N`, which would
  name a part that is not in the package.
- **`mjx_chart::chart_ops::series_references`** and `ChartSeriesReferences` — where each series says
  its data lives, as against what its cache holds. On **all three** surfaces: `Deck`, `Document` and
  `Workbook` each gained `chart_series_references`.
- **`mjx_dml::spreadsheet_drawing::new_anchored_graphic_frame`** — the frame a chart sits in on a
  sheet. `CT_GraphicalObjectFrame` declares `xdr:xfrm` `minOccurs="1"`, unlike the `a:xfrm` a picture
  may omit, so it is written (all-zero, as Excel and LibreOffice both write for a two-cell anchor).
- **`mjx_sml::ReferenceAreas`** — the areas of a reference that names more than one, split on the
  commas that are not inside a quoted sheet name or an external-book bracket. `Copy` and
  allocation-free, like the rest of that module.
- **`PartKind::Chart`**, with `REL_CHART`, `REL_PACKAGE` and `CONTENT_TYPE_CHART`. The part
  inventory names a chart part instead of leaving it unclassified; twenty-seven part kinds became
  twenty-eight.
- **`tests/fixtures/chart_in_sheet.xlsx` and `chart_stale_cache.xlsx`** — two workbooks **written by
  LibreOffice 25.8.7.3**, not by this project. The first carries a chart over a live range with no
  `c:externalData` at all; the second is the same package with the sheet and string table of a
  second run spliced in, so its **caches and its cells disagree on every point**. A fixture whose
  cached values equalled its cell values would prove nothing about which source a reader used.
- **The Excel guide's chart page** (`crates/mjx-xlsx/docs/guide/charts.md`), four compiled
  doctests, and `examples/chart_range_cost.rs`, which asserts with the counting allocator that a
  resolution is bounded by the range rather than by the sheet: on a 30,000-cell sheet a three-cell
  range costs **2,109 bytes** beyond the sheet's own read, and four areas in one call cost one sheet
  parse where four calls cost four.
- **Both bindings** gain the family: `workbook.add_range_chart(...)` in Python,
  `workbook.addRangeChart(...)` in TypeScript, with `ChartRangeSeries`, `ChartSeriesReferences`,
  `ChartSeriesFreshnessInfo`, `SheetChartWorkbookInfo`, `ResolvedRangeInfo` and `RangeCellInfo`
  projected alongside.

### Fixed

- **`crates/mjx-ooxml/tests/chart_surface_parity.rs`'s strongest assertion was vacuous.** It compared
  `deck.chart_part_bytes(...)` against `document.chart_part_bytes(...)` after twelve edits, and
  `mjx_opc::Package::part_bytes` answers `None` for a part whose body is `Edited` — so the comparison
  had been `None == None` since MJXOFF-103 wrote it, and a chart part wired to the wrong bytes would
  have satisfied it. It now compares the parts of the **saved** packages and asserts all three are
  really there. The three surfaces do agree, byte for byte.

### Changed

- **`Workbook::detach_chart_workbook` removes the embedded workbook part**, unless another chart
  still names it. `mjx_docx::Document::detach_chart_workbook` leaves it in the package and says so;
  this surface cannot, because `Workbook::save` runs `Package::validate`, which refuses a package
  holding a SpreadsheetML part no relationship chain reaches — so a detach that left it behind would
  hand back a workbook this library then declines to write.
- **`XlsxError` gains five variants**: `ChartAccess` (wrapping `mjx_chart::ChartAccessError` whole,
  the `mjx-docx` shape rather than `mjx-pptx`'s eight restated variants), `ChartData`,
  `InvalidChartData`, `AnchorIsNotAChart` and `ChartHasNoExternalData`. The facade's `classify_xlsx`
  routes the first through the *same* `chart_access_code` `DocxError::ChartAccess` goes through, so
  the same index refused from a workbook, a document and a presentation answers the same
  `ErrorCode`.

## [0.0.124] - 2026-09-06

**Worksheet drawings** (MJXOFF-107, Phase E position 3): the `xl/drawings` part, the three anchor
modes, and the last place DrawingML reaches that this workspace had not.

### Added

- **`mjx_dml::spreadsheet_drawing`** — all seventeen complex types of
  `dml-spreadsheetDrawing.xsd`, as fidelity wrappers: `WorksheetDrawing` (`xdr:wsDr`), the three
  anchors, `CellMarker` (`xdr:from`/`xdr:to`), `AnchorClientData`, and the six things an anchor can
  hold. It sits in `mjx-dml` for the reason `wordprocessing_drawing` does — `xdr` is a DrawingML
  satellite schema whose content is DrawingML — and knows nothing about packages.
- **`WorksheetDrawing::insert_rows` and its three axis siblings** — each anchor mode does what it
  promises: a two-cell anchor moves *and* sizes, a one-cell anchor moves and keeps its size, an
  absolute anchor does neither. The returned `AnchorShift` per anchor includes `promise_kept`, which
  is `false` in exactly one case — a two-cell anchor resized while its own `@editAs` forbids it —
  rather than leaving that anchor silently wrong.
- **`mjx_sml::SheetAnchors`, `ColumnMetrics`, `GeometrySource` and `ResolvedAnchorBounds`** — an
  anchor resolved to a rectangle in EMU against a sheet's own column widths and row heights, and the
  honesty half of that answer. A row height is exact (points are 12,700 EMU); **a column width is a
  character count and cannot be a length** without a font measurement this library never makes, so
  the metrics are the caller's and every answer carries them. Where the sheet states nothing that
  could place the object — no `x:sheetFormatPr`, so no `@defaultRowHeight` — the answer is `None`.
- **`CT_Worksheet`'s last three owned slots**: rank 29 `drawing` (reusing MJXOFF-129's
  `SheetDrawing`), rank 34 `oleObjects` and rank 35 `controls`, with `EmbeddedObjects`,
  `EmbeddedObject`, `FormControls`, `FormControl` and `FormControlProperties`. Thirty-nine slots,
  **thirty-four modelled, five held**. `CT_ControlPr` repeats MJXOFF-127's trap exactly: six of its
  booleans default to `true`.
- **`mjx_xlsx::Workbook`'s drawing surface** — `sheet_drawing`, `drawing_markup`,
  `edit_drawing_markup`, `sheet_anchor_bounds`, the three `add_*_anchored_picture` calls,
  `remove_sheet_drawing_object` and the four axis shifts. Adding a picture writes six things
  together, including the image relationship **from the drawing part** rather than from the sheet:
  an `a:blip@r:embed` is resolved against the part that contains it. Every edit goes back through
  `ToXml::write_back` and the document the part was parsed from, so a shift that moves nothing
  re-emits the part byte for byte — prologue included, which for a file Apache POI wrote is
  `<?xml version="1.0" encoding="UTF-8"?>` and not this project's own declaration.
- **The whole of it on `mjx_ooxml::Workbook` and both bindings** (A10's rule) — twelve methods, four
  value types and two enumerations, with the committed `.pyi` stub extended.
- **`mjx_ooxml_types::spreadsheetdrawing`** — `ResizingBehavior`, `ColumnIdentifier` and
  `RowIdentifier`, generated. `ST_EditAs`'s members are named from §20.5.3.2's own enumeration-value
  titles (`MoveAndResizeWithAnchorCells`, `MoveWithCellsButDoNotResize`,
  `DoNotMoveOrResizeWithRowsOrColumns`), which say what happens to the object rather than naming the
  anchor shape the wire token is spelled after.
- **`tests/fixtures/worksheet_drawings.xlsx`** — a workbook with all three anchor modes, **written
  by Apache POI 5.5.1**, not by this project. Its `twoCellAnchor` carries `editAs="oneCell"`, a value
  that disagrees with the element's own name; its columns are 3.5, 20.75 and 12 characters wide and
  it states no `defaultColWidth`; its picture starts mid-cell; and it anchors a PNG on two anchors
  and a JPEG on the third. The extent POI computed for that first anchor — `cx="2085975"
  cy="885825"` — is what this project's own resolver is asserted against.
- **A guide page**, `Worksheet drawings`, whose every snippet is a compiled doctest.

### Changed

- **`dml-spreadsheetDrawing` moved from `CHILD_ORDER_SCHEMA_DEPENDENCIES` to
  `CHILD_ORDER_SCHEMAS`** — the third schema to make that move, after `dml-wordprocessingDrawing`
  and `shared-math` — and its `UNCOVERED_SCHEMAS` row is gone, because a schema covered in both
  tables has no row there.
- **`mjx-schema-gate` gains the `xdr` arm.** Without it a drawing part reports `Uncategorised`,
  which reads like a pass; that is how `mjx-vml` sat unvalidated. Both halves are proved live: a
  stray `xdr:col` inside a `twoCellAnchor` fails validation naming the part, and moving
  `xdr:clientData` to the front of an anchor turns the ordering audit red naming
  `CT_TwoCellAnchor`.
- **`NON_XML_CONTENT_TYPES_UNDER_XL` gains `image/jpeg`**, and `image/png`'s reason now names two
  kinds of part rather than one. The list is keyed on the content type rather than on where the part
  sits, so a PNG under `xl/media/` needed no row of its own — the fixture carries a JPEG so that the
  media path is not proved by one format and assumed for the rest.
- **`XlsxError::UnrecognizedImageFormat`** is new; A9's exhaustive `classify_xlsx` refused to compile
  without an arm for it.

## [0.0.123] - 2026-09-06

**Charts reach the Word surface** (MJXOFF-103, Phase E position 2): a `c:chart` inside a
`w:drawing`, read, authored and edited, under the method names `mjx-pptx` already uses.

### Added

- **`Document`'s chart family — 42 methods**, in `crates/mjx-docx/src/document/charts.rs`. Reading
  (`chart_series`, `chart_kinds`, `chart_axes`, `chart_title`, `chart_legend`, `chart_style_id`,
  `chart_data_labels`, `chart_point_formats`, `chart_trendlines`, `chart_error_bars`,
  `chart_dangling_decoration`), authoring (`add_chart`, `add_chart_placed`), the embedded workbook
  (`chart_workbooks`, `refresh_chart_workbook`, `detach_chart_workbook`) and the whole edit and
  decoration tier. A chart is addressed by its drawing's own `wp:docPr` id — the address MJXOFF-131
  already gave every Word drawing — rather than by a second scheme.
- **`mjx_chart::chart_ops`** — every read and every edit a host surface performs on a chart, stated
  **once**, over a `ChartSpace`. `mjx-pptx` and `mjx-docx` are both rank 3.0, so neither may reach
  the other; the shared body had to move *down* to the crate that owns `c:chartSpace` or be written
  twice and kept in step by hand. Both surfaces are now thin wrappers around it: resolve an address
  to a chart part, call the identically-named function, refresh the workbook. `ChartAccessError` is
  its error type, deliberately **not** `#[non_exhaustive]` so both hosts must map it exhaustively.
- **`mjx_dml::GraphicData::for_chart` / `chart_relationship_id`, and `CHART_GRAPHIC_URI`** — the
  DrawingML envelope a chart reference sits in, which is the same envelope in every format.
- **`ChartPlacement` / `ChartWrap`** — inline or floating, with three of `EG_WrapType`'s five wrap
  modes. `wrapTight`/`wrapThrough` are left off the authoring surface deliberately: both need a
  `wp:wrapPolygon` whose coordinate space ECMA-376 does not state for `CT_WrapPath`, and guessing one
  would put a wrong polygon in every document. Reading either is unaffected.
- **`mjx_dml::wordprocessing_drawing::WrapSquare::new` and `WrapTopAndBottom::new`** — MJXOFF-131
  modelled all five wrap modes and gave constructors to two of them; `Anchor::new` had no caller at
  all until this child became its first.
- **`tests/fixtures/chart_in_word.docx`** — a `.docx` carrying a chart, **written by Apache POI
  5.5.1**, not by this project. It numbers its drawing `0`, ships no `word/styles.xml`, spells
  booleans `false` where this library writes `0`, and names its workbook
  `Microsoft_Excel_Worksheet1.xlsx` where this library writes `Microsoft_Excel_Sheet1.xlsx`.
- **The Word guide's chart page** (`crates/mjx-docx/docs/guide/charts.md`), three compiled doctests.
- **Both bindings** gain the family: `document.add_chart(...)` in Python,
  `document.addChart(...)` in TypeScript, with `ChartWrap`, `DocumentChartWorkbook` and `WrapText`
  projected alongside.

### Fixed

- **`Document::remove_drawing` left a chart's relationship dangling.** It looked only for a
  *picture's* image relationship, so removing a chart drawing left `word/_rels/document.xml.rels`
  pointing at a chart part nothing referenced — which `Package::validate` reports as a defect on the
  next `save`. It now sweeps a chart's relationship, and with it the workbook that chart part alone
  referenced.
- **The child-order audit never descended into an embedded workbook, in any format.** The validation
  half of the schema gate has opened a chart's `.xlsx` since A5; the ordering half walked the outer
  package only. `sml` has been in `CHILD_ORDER_SCHEMAS` since MJXOFF-132 and `mjx-sml`'s writer
  composes those parts, so nothing was checking the order of markup this project writes.
  `audit_deck_order` now descends, under the same `…xlsx!/…` naming the validation half uses — which
  closes the hole for `mjx-pptx` in the same commit that found it from Word.

### Changed

- **`ChartSeriesData`, `ChartAxisData`, `ChartLegendData`, `ChartLabelScope`,
  `ChartPointFormatData`, `ChartTrendlineData` and `ChartErrorBarData` moved from `mjx-pptx` to
  `mjx-chart`** (`mjx_chart::view`). They were declared in `mjx-pptx` because a chart was reachable
  from one surface; two surfaces at the same rank cannot share a type that lives in either. **No
  public path changed**: `mjx-pptx` re-exports all seven, and `mjx-ooxml` now names them from
  `mjx-chart` instead.
- **`mjx-pptx`'s chart methods are delegations.** Every one keeps its signature, its error variants
  and its documentation, and calls `mjx_chart::chart_ops` for the body. `PptxError` gains an
  exhaustive `From<ChartAccessError>`.
- **A7d's chart-part re-flow limitation is gone, and the Word path inherits that.** MJXOFF-143
  carried the source span through `FromXml`/`ToXml`; measured here on a producer-written part,
  moving a chart's legend changes the one attribute and leaves the other 2,962 bytes identical.
  `crates/mjx-docx/tests/charts.rs` asserts it rather than the CHANGELOG claiming it.

## [0.0.122] - 2026-09-06

**The workspace's one sanctioned duplicate is deleted: a chart's embedded workbook is written by
`mjx-sml`** (MJXOFF-99, Phase E position 1).

### Removed

- **`crates/mjx-chart/src/workbook.rs`** — 686 lines of minimal SpreadsheetML writer, and the public
  items listed under *Unreleased — 0.1.0* above. It opened by naming its own executioner: *"a
  duplicate with a scheduled removal is a debt; a duplicate nobody removes is an architecture."*
  Written because a chart embeds a whole `.xlsx` package at `/ppt/embeddings/*.xlsx` and no
  SpreadsheetML crate existed, it proposed `mjx-xlsx` as its replacement — which would have been an
  **upward** edge (2.2 → 3.0). `mjx-sml` is rank 2.1, so `mjx-chart → mjx-sml` points down, and that
  is the edge the deletion rides on.
- **`crates/mjx-chart/tests/workbook_parity.rs`** — MJXOFF-112's gate, which existed only to compare
  the two writers byte for byte. With one writer left there is nothing to compare; everything it
  asserted about the surviving writer is also asserted in `crates/mjx-sml/tests/package_writer.rs`.
- **`mjx_chart`'s private `column_letters`.** A chart's `c:f` formulas name their columns through
  `mjx_sml::address::column_letters`, so the chart and its workbook cannot disagree about which
  column is which.

### Changed

- **`mjx-chart` holds no SpreadsheetML at all** — not an element name, not an `xl/` part name, not a
  namespace constant, not in a test. `crates/mjx-chart/src/embedding.rs` decides only *which cell* a
  chart's data belongs in and hands the rows to `mjx_sml::write::WorkbookPackage`. That is the whole
  crate's involvement with spreadsheets now.
- **`mjx-pptx` registers an embedded workbook with `mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE`**
  and gained a direct `mjx-sml` dependency for it (3.0 → 2.1, downward). `add_chart` and
  `refresh_chart_workbook` keep their shape exactly: everything fallible that does not touch the
  package still happens first, and a refresh still answers `false` rather than erroring for a chart
  with no `c:externalData`, an unresolvable relationship, an `External` target mode or a missing part.
- **`mjx_sml::write::WorkbookPackage::push_row` advances past a row that writes nothing.** Fixed
  forward here rather than worked around in `mjx-chart`. `Blank` and a non-finite number write no
  cell, so a row of them left no `<row>` behind — and the next row's number was measured off the
  *populated* rows, so it took the empty row's place and slid the whole grid up by one. A chart read
  back from a file whose series carry no `c:tx` has exactly that header row, and its own `c:f` says
  `Sheet1!$A$2:$A$3`. `AuthoredWorksheet::appended_row_count` is the new cursor, public and
  documented, and `set_cell_value` still counts, so mixing the two doors never overwrites.

### Fixed

- **`the_refreshed_workbook_holds_the_edited_values` could not fail for the thing it names.** It set
  a series' values *and* its categories, and `set_chart_series_categories` refreshes the workbook
  too — so removing `set_chart_series_values`'s refresh entirely left it green. Found by mutation
  while rerouting the writer. It is now one test per setter, each asserting that the labels or the
  numbers the fixture carried are *gone*, and each independently red when its own refresh is removed.
  A11's R4 (`editing_a_chart_dirties_only_the_chart_xml_and_its_workbook`) always caught the values
  case, so nothing was unguarded; one of the two guards was simply not the guard it read as.

### Documentation

- **The gaps page's standing paragraph is a closed *What used to be here* row**, naming `mjx-sml`
  rather than `mjx-xlsx` — the original sentence named the wrong crate, and the edge it implied was
  illegal.
- **`xtask/src/corpus/xlsx.rs` states, at the call site, why its hand-written worksheet stays.** It is
  the fourth writer of SpreadsheetML in this repository and the only one left; MJXOFF-93 reported it
  and left the decision open. It is kept on purpose: it is the *input* to a benchmark of the library's
  reader, so generating it through the library would make `docs/BENCHMARKS.md` a measurement of our
  reader against our own writer; it writes a file `WorkbookPackage` cannot (no shared strings, no
  styles, `t="inlineStr"`, `spans` on every row); building it through the model would pay the cost
  the harness exists to measure; and `xtask` is a host-only binary outside the ranked graph that
  ships nowhere. **So the workspace's "one sanctioned duplicate" claim is retired with the writer it
  described: exactly one SpreadsheetML writer ships, and the remaining hand-written one is tooling.**

## [0.0.121] - 2026-09-06

**Excel through the facade and both bindings — and one API decision made from a measurement rather
than from taste** (MJXOFF-137, Phase D position 20; **Phase D complete**).

### Added

- **`mjx_ooxml::Workbook`** — the curated Excel surface, eleven modules mirroring `deck/`'s and
  `document/`'s split. `detect_format` already answered `Format::Workbook`; it now yields a workbook
  that opens, reads, edits, validates and saves. Tabs, cells, geometry, cell formats, hyperlinks,
  tables, defined names, print setup, the preserved-part reports and the part graph, all with
  concrete types: **A1 text for every address** (`"B7"`, `"A1:C3"`), `u32` for every index, `&str`
  for every part name.
- **`mjx_ooxml::Workbook` in Python and TypeScript**, and the classes their arguments and results are
  made of — twenty-nine value classes and nineteen enumerations each. The walkthrough exists three
  times (`crates/mjx-ooxml/examples/build_a_workbook.rs`,
  `bindings/mjx-python/tests/test_build_a_workbook.py`,
  `bindings/mjx-wasm/tests/node/build_a_workbook.mjs`) and the two bindings are compared against the
  Rust one **part by part, byte for byte**.
- **[Through the facade and the bindings](crates/mjx-xlsx/docs/guide/through_the_facade.md)**, the
  Excel guide's fourteenth page: the translation table, the range decision, and what the facade does
  not carry.
- **`crates/mjx-ooxml/benches/workbook_boundary.rs`** — the instrument MJXOFF-135's figures did not
  have. `docs/BENCHMARKS.md`'s own harness drives `Package::part_tree_mut`, which `mjx-xlsx` never
  calls, so it could not see this cost at all.
- **`mjx_allocation_counter::total_allocated`** — a monotonic byte counter. `peak` and `live` cannot
  tell one parse from two hundred parses that each free before the next; this can, and it is what
  lets the boundary gate below be deterministic instead of a stopwatch.

### The range decision

**There is no per-cell reader or writer on the facade, or in either binding.** `mjx_xlsx::Workbook`
holds no parsed worksheet, so every per-sheet accessor re-parses the part: measured on the
300,000-cell corpus, in release, **405 ms to read one cell** against **12.0 ms to open the whole
file**, and a 4,000-cell write loop against 410 ms for the same 4,000 cells batched. The Rust answer
— hold the `WorksheetPart` yourself — cannot cross a foreign function boundary, so a facade with
`cell_value(sheet, "A1")` would ship the slow loop as the natural idiom in the one place a caller
cannot reach past it.

So the cell door is a **range** in both directions: `read_range`, `read_sheet` and `write_cells`
parse once each, whatever they are asked for. `crates/mjx-ooxml/tests/workbook_boundary.rs` holds
that to a **deterministic allocation ratio** — 209x for the write, 199x for the read — because a
fallback to per-cell would produce the same file, byte for byte, and no correctness gate could see
it. The per-cell calls stay reachable from Rust through `Workbook::workbook_mut`.

### Changed

- **`Format::is_editable` is now true for every format but `Format::WorkbookBinary`.** `.xlsb` is
  refused with its own message — its main part is the MS-XLSB binary record stream, not
  SpreadsheetML — and that refusal is a design decision rather than a schedule.
- **`mjx_ooxml::Error`'s mapping reaches three enumerations further.** `classify_xlsx` names every
  `XlsxError` variant and, through `sml_code` and `address_code`, every `SmlError` and
  `AddressError` variant, with **no wildcard arm anywhere** — so a new failure mode is a compile
  error rather than a silent `Unknown`.
- **`mjx_sml::GridAnomaly` is no longer `#[non_exhaustive]`**, for the reason the error enumerations
  never were: the facade's flat `GridAnomalyInfo` projection must be a compile-time gate.

### Fixed

- **`crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` and
  `crates/mjx-docx/docs/guide/fidelity_and_gaps.md` both claimed `mjx-docx`/`mjx-xlsx` "have no
  editing surface — they are scaffolds", and one promised a removal "once `mjx-xlsx` can write
  (`v0.3`)".** All false, and they are published docs.rs pages. Corrected to what is true today; the
  `mjx-chart` embedded-workbook writer's removal condition is now *met*, and the removal itself is
  MJXOFF-99's work.

### Handed over

- **`docs/EXCEL_FACADE_HANDOFF.md`** — twenty-six checklist entries for MJXOFF-128 (F2), **every one
  unmarked**, plus what has no runtime coverage anywhere. No agent has Excel.

## [0.0.120] - 2026-09-06

The Excel usage guide and its runnable examples — every snippet compiled, every example asserting
(MJXOFF-135, Phase D position 19).

### Added

- **Two guide pages**, bringing `crates/mjx-xlsx/docs/guide/` to thirteen. The other eleven were
  written by the children that shipped the features they describe; these two had no owner:
  - **[Large workbooks](crates/mjx-xlsx/docs/guide/large_workbooks.md)** — the memory model in a
    caller's terms. What a sparse sheet costs (nothing proportional to the grid), what a populated
    cell costs (**36.8 B**, or **76.8 B** with a formula, against a `RawElement` tree's **802 B**),
    and the awkward figure: on the 300,000-cell corpus workbook, `Workbook::open` is **14.8 ms and
    16.8 MB**, while **the first `worksheet_markup` is 507 ms with a 269 MB allocation peak — and it
    is paid again on every subsequent per-sheet call**, because `Workbook` holds no parsed worksheet.
    Filling a sheet through `set_cell_value` is therefore quadratic: 4,000 cells one at a time is
    **18.39 s** against **1.12 ms** for one read, N edits and one write, a measured 16,427×. The page
    says so, states the shape to reach for, and names the one `Arc` copy that cannot be avoided
    because `mjx_opc::Package::part_bytes` answers `&[u8]` rather than shared bytes.
  - **[Deliberate limitations](crates/mjx-xlsx/docs/guide/deliberate_limitations.md)** — the page to
    read before filing a bug. The two-crate split (which of `mjx-sml` and `mjx-xlsx` to reach for,
    and why the split is what makes `mjx-chart → mjx-sml` legal), the three standing refusals that
    all follow from having no calculation engine (stale cached values, unevaluated
    conditional-formatting conditions, unapplied filters/sorts/validation), the half of `sml.xsd`
    preserved rather than modelled, and a closing list of what is genuinely **absent and unowned**
    rather than deliberately refused.
- **Six runnable examples** under `crates/mjx-xlsx/examples/`, which had none at all. Each reopens
  its own output and asserts on it, and CI's `examples` job picks them up by directory:
  `build_a_workbook` (two tabs, shared strings, a format, resolved back through the reopened
  stylesheet), `edit_a_workbook` (**exactly two part payloads may differ** after one cell edit and
  one rename, checked part by part), `read_formulas` (eleven formula cells, the five-member shared
  group whose text lives on one of them, and the cached value that is **still 2** after its
  dependency became 50), `style_a_range` (one `xf`, nine cells, and a tenth outside that must not
  wear it), `table_and_autofilter` (a table part, its relationship, its `tablePart` entry, and a
  filter that hides no row) and `large_sparse_sheet` (a counting global allocator, one cell at
  `XFD1048576` under a 32 KiB bound, and 30,000 cells under the 48 B/cell bound).

### Changed

- **`README.md`, `PLAN.md` and the `mjx-ooxml` docs hub** point at the Excel guide. The README's
  guide and example sections had been left at PowerPoint's alone — Word's five pages and nine
  examples shipped in MJXOFF-150 without being linked — so all three formats are now listed, and the
  format-support table no longer calls Word and Excel *planned*.
- **`mjx-sml`'s crate documentation** gains a pointer to the two guide pages that explain it to a
  caller, and its rank table is corrected: it listed `mjx-xml` at rank 0.0 beside `mjx-ooxml-core`
  and `mjx-derive`, where `CLAUDE.md` and `xtask/tests/layering.rs` both put it at **0.1** in a row
  of its own.

### Fixed

- **Four stale prose counts and one stale forward reference**, all documentation:
  - the Excel guide README said *"Ten pages"* over a table of eleven, and its arc stopped at
    MJXOFF-127 (D16) although D17 and D18 had shipped;
  - `the_sheet_grid.md` said **eighteen** of `CT_Worksheet`'s thirty-nine slots were modelled and
    twenty-one held, a figure last true at MJXOFF-125; MJXOFF-127 and MJXOFF-129 have since taken it
    to **thirty-one modelled, eight held**, which is what `crates/mjx-sml/src/worksheet/frame.rs`
    itself says;
  - `authoring_a_workbook.md` said *"Setting a formula is MJXOFF-115's"*. MJXOFF-115 shipped, and
    modelled formulas for **reading and preservation only** — there is no `set_cell_formula` on
    `Workbook` or on `mjx_sml::SheetData`, and no later child owns adding one. The page now says that
    plainly and the limitations page lists it as an unowned gap;
  - `mjx_ooxml::FormatFamily::WordProcessing` was documented *"Detected, not yet editable"* after
    Word became fully editable, and `Spreadsheet` said the same of Excel.

## [0.0.119] - 2026-09-06

The half of `sml.xsd` this project deliberately does not model — recognised, reported and proved to
survive (MJXOFF-133, Phase D position 18).

### Added

- **`mjx_sml::preserved`** — read-only *identity views* over the parts of the nine unmodelled
  clusters: `PivotTableIdentity` (name, cache id, `CT_Location@ref`), `PivotCacheIdentity` /
  `PivotCacheSource`, `ExternalLinkIdentity` / `ExternalLinkTarget`, `ConnectionIdentity`,
  `QueryTableIdentity`, `XmlMapsIdentity` / `XmlMapIdentity`, `RevisionHeadersIdentity` /
  `RevisionSession`, and `SharedWorkbookUsersIdentity` / `SharedWorkbookUser`. **None of them
  implements `ToXml`**, so there is no path by which reading one changes a byte — which is the
  difference between identifying a part and modelling it.
- **`mjx_xlsx::Workbook::preserved_parts`** and `PreservedParts` — every preserved part of a
  workbook, resolved from relationships alone with **no markup parsed at all**. Beside it, the typed
  reports: `pivot_tables`, `external_links`, `connections`, `query_tables`, `xml_maps` and
  `revision_state`, with `SheetPivotTable`, `WorkbookExternalLink`, `WorkbookConnection`,
  `SheetQueryTable`, `WorkbookXmlMaps` and `RevisionState`. A caller can ask *"does this workbook
  have pivot tables, and where are they?"* and get the sheet, the range, the cache and every part
  name — **without a ninety-seven-type model** behind it.
- **`mjx_xlsx::PartKind` now names every ECMA-376 Part 1 §12.3 part type** — the six MJXOFF-91 left
  out are in: `CustomProperty` (§12.3.5), `CustomXmlMappings` (§12.3.6), `RevisionHeaders`
  (§12.3.16), `RevisionLog` (§12.3.17), `SharedWorkbookUserData` (§12.3.18) and
  `SingleCellTableDefinitions` (§12.3.19). Twenty-seven kinds in all. **Recognising a part is not
  modelling it:** every one of them is still carried through a save as the bytes it arrived as.
- **`PartKind::from_relationship_type`** and `parts::AMBIGUOUS_CONTENT_TYPES`. Two §12.3 part types
  are not identified by their content type — a Custom Property part carries *"any content, support
  for which is application-defined"*, and the Custom XML Mappings part carries plain
  `application/xml`, which in a real package is also the `Default` for every `.xml` part with no
  `Override`. `preserve::classify` therefore asks the content type first and the relationship graph
  second, and `from_content_type` refuses to answer from an ambiguous string rather than
  misidentifying `docProps/custom.xml` as an XML map.
- **New part-graph edges**: `WorkbookParts::custom_xml_mappings` / `revision_headers` /
  `shared_workbook_user_data`, `WorksheetParts::custom_properties` /
  `single_cell_table_definitions`, and the three sub-graphs `PivotTableParts` (a table to its
  cache), `PivotCacheParts` (a cache to its records) and `RevisionHeadersParts` (the headers part to
  its logs).
- **`SpreadsheetDefect::WorkbookReferenceTargetIsWrongKind`** — a `pivotCaches/pivotCache@r:id` or
  `externalReferences/externalReference@r:id` that leads to a part of the wrong kind. The direction
  packaging cannot see, for the two lists in `CT_Workbook`'s sequence that point outward at parts:
  those parts are preserved and unmodelled, which is exactly why nothing else here would notice one
  going stale.
- **`tests/fixtures/preserved_parts.xlsx`** — a workbook carrying one part of every cluster at once,
  and `crates/mjx-xlsx/tests/preserved_parts.rs`, which edits a cell **on the very sheet the pivot
  table sits on** and compares all fourteen preserved parts against the bytes they went in with.
  *A part nothing asserts on is a part that silently disappears.*

### Documented

- **The scope decision, in writing.** `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md`
  carries a cluster-by-cluster table: 184 of `sml.xsd`'s 367 complex types, what "preserved"
  guarantees, what it does not, and **why** for each. The pivot cluster alone is 97 types — a
  quarter of the schema — and it is derived data of a calculation model this project does not have:
  **if it is ever modelled it is a phase of its own**, not a gap for a later child. A documented gap
  is never a validation failure.
- **`CT_Worksheet`'s ownership table was stale and is now re-derived from the code.**
  MJXOFF-129 typed six slots (13, 19–22, 33) without updating
  `crates/mjx-sml/src/worksheet/frame.rs`'s module table, which still described 25 modelled and 14
  held. It is **31 modelled and 8 held**, and three of the eight — `phoneticPr` (15),
  `legacyDrawingHF` (31) and `drawingHF` (32) — belong to no ticket at all, each with the reason it
  does not. All nineteen of `CT_Workbook`'s slots are modelled.
- **A place where ECMA-376 contradicts its own schema**, recorded as a tolerated deviation rather
  than papered over: `CT_Schema` (`sml.xsd:340`) is one required child under a **strict** wildcard,
  and what §12.3.6's own example puts there is an inline XML Schema document, which no schema
  `sml.xsd` imports declares. `xl/xmlMaps.xml` is therefore a part no conformant instance can
  satisfy — omitting the child breaks `minOccurs`, supplying the specification's own one breaks the
  wildcard.

## [0.0.118] - 2026-09-06

Print setup, headers and footers, custom views, and the three sheet kinds that are not worksheets
(MJXOFF-129, Phase D position 17).

### Added

- **`mjx_sml::features::print`** — the markup every sheet *kind* carries. `CT_PrintOptions`
  (`PrintOptions`), `CT_PageMargins` (`PageMargins`, in inches, all six `use="required"`),
  `CT_PageSetup` (`PageSetup`), `CT_CsPageSetup` (`ChartSheetPageSetup` — the same element name on a
  chartsheet and a *different complex type*, without the six attributes that only mean something
  over a grid), `CT_HeaderFooter` (`HeaderFooter`, `HeaderFooterText`, `HeaderFooterSlot`,
  `HeaderFooterSection`) and `CT_SheetBackgroundPicture` (`SheetBackgroundPicture`).
  **Nothing paginates:** `fitToWidth` is reported, and where a page breaks is rendering.
- **`mjx_sml::features::custom_views`** — `CT_CustomSheetViews`/`CT_CustomSheetView`, filling
  `CT_Worksheet`'s rank 13. The nine children come from four existing clusters — MJXOFF-102's pane
  and selection, MJXOFF-117's breaks, MJXOFF-123's autofilter and this child's print block — and not
  one of them is modelled a second time. **A record, never applied:** a view's `@hiddenRows` is that
  view's memory, not the sheet's rows.
- **`mjx_sml::sheets`** — `CT_Chartsheet` (`ChartSheetPart` and its seven-type cluster),
  `CT_Dialogsheet` (`DialogSheetPart`) and `CT_Macrosheet` (`MacroSheetPart`), over one shared part
  frame with the same slot-level copy-on-write, generated placement and byte writer `WorksheetPart`
  has. **A chartsheet has no cell accessor at all** — the absence is in the type, so asking one for
  its cells does not compile.
- **Six more `CT_Worksheet` slots are typed** rather than held: `customSheetViews` (13),
  `printOptions` (19), `pageMargins` (20), `pageSetup` (21), `headerFooter` (22) and `picture` (33).
  Thirty-one of the thirty-nine are now modelled; `phoneticPr` (15) is the only one left that
  belongs to nobody.
- **`mjx_xlsx::SheetMarkup`** and `Workbook::sheet_markup` / `sheet_markup_of` /
  `write_sheet_markup` — the markup behind any tab, whichever of the four kinds it is. A macrosheet
  is dispatched on its **root element**, because ECMA-376 declares no content type for one, so
  `SheetKind` still reports the three kinds §12.3.23 names.
- **`Workbook::sheet_printer_settings` and `Workbook::sheet_background_image`** — the part a sheet's
  own `pageSetup@r:id` and `picture@r:id` reach, resolved against that sheet part's `.rels`. Neither
  part is ever opened: a printer-settings blob is a Windows `DEVMODE` ECMA-376 Part 1 §15.2.13
  places no requirement on, and an image is bytes.
- **`mjx_xlsx::REL_IMAGE`** and `WorksheetParts::background_image` — the image relationship a sheet's
  background picture reaches (Part 1 §15.2.14).
- **`SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType`** — a `pageSetup` or `picture`
  naming a relationship of the wrong *type*. The half `mjx_opc`'s dangling-reference check cannot
  see: the id is declared, so only a reader that knows what a `pageSetup` means can tell it points
  at the wrong kind of part. Reported for markup this library will write, never for markup it merely
  opened.
- **`tests/fixtures/print_and_sheet_kinds.xlsx`** — a worksheet with the full print block, a
  `customSheetView` carrying all nine of its children, two printer-settings blobs, a background
  image and a dialogsheet. Every header/footer string in it is a different shape (a quoted font
  name, a `&G`, a literal `&&`, a character reference, a CDATA section), because those are the five
  ways a re-serialising writer changes a file.
- **`crates/mjx-xlsx/docs/guide/print_setup_and_sheet_kinds.md`** — the guide page, six compiled
  doctests.

### Changed

- **`mjx-xlsx`'s `no_part_under_xl_is_skipped_as_foreign_or_uncategorised` now distinguishes two
  kinds of skip.** A part whose *payload is not XML* has nothing a schema could be applied to, so
  skipping it is correct; a part whose *root namespace has no arm* is the false green MJXOFF-110
  exists to close. The guard rejected both. It now accepts a `SkippedBinary` whose content type is
  on a pinned, reasoned allowlist and rejects every other outcome exactly as before — with two new
  cases proving it: one feeds the rule each shape of false green and asserts it is still rejected,
  the other fails an allowlist entry no committed fixture witnesses.

- **`tests/fixtures/hyperlinks.xlsx`'s custom-property part moves from `/customProperty1.bin` to
  `/xl/customProperty1.bin`.** MJXOFF-127 put it at the package root because the guard above
  rejected a binary part under `xl/`, and said so at the time. A Custom Property part's target is
  relative to the workbook (ECMA-376 Part 1 §12.3.5), so `xl/` is where it belongs; with the guard
  fixed, the workaround goes. The part's bytes and the sheet markup that names it are unchanged.

### Fixed

- **A header or footer string's own spelling survives an edit elsewhere in the part.** The
  `#[xml(text)]` escaping gap the epic recorded as latent becomes live here — `&#65;` decodes to
  `A`, a CDATA section decodes to its contents, and a rebuilt text node that differs from the
  original denies its element and every ancestor of it the verbatim source range subtree
  copy-on-write would give it. `HeaderFooterText` therefore has the hand-written `FromXml`/`ToXml`
  pair `DefinedName` has: it replays the file's own children until `set_text` replaces them. The gap
  itself is still in the derive, and still owned by no work item.

## [0.0.117] - 2026-09-06

Hyperlinks, the object-anchor vocabulary three Phase E children share, and the last small worksheet
children nothing else in Phase D had claimed (MJXOFF-127, Phase D position 16).

### Added

- **`mjx_sml::features::hyperlinks`** — `Hyperlinks` (`CT_Hyperlinks`, `sml.xsd:2739`) and
  `Hyperlink` (`CT_Hyperlink`, `sml.xsd:2744`), filling **rank 18 of `CT_Worksheet`**. `@ref` is an
  `ST_Ref`, so **a hyperlink covers a range and not a cell**, and nothing splits a `B4:D6` entry into
  one link per cell. `CT_Hyperlink` declares `@r:id` and `@location` both optional, so there are
  three shapes and not two — and **an entry carrying both is a real file Excel writes, not a defect
  to clean up**. Nothing here drops either because the other is present.
- **`mjx_sml::features::objects`** — `ObjectAnchor` (`CT_ObjectAnchor`, `sml.xsd:238`) and
  `ObjectProperties` (`CT_ObjectPr`, `sml.xsd:3063`). **Modelled in `mjx-sml` on purpose**: `sml.xsd`
  reaches `CT_ObjectAnchor` from `CT_CommentPr` (MJXOFF-114, E5), `CT_ObjectPr` (MJXOFF-107, E3) and
  `CT_ControlPr` (both), so one type in the shared-markup tier is what stops three Phase E children
  inventing three. The two `xdr:from` / `xdr:to` markers are **held as raw elements** — `CT_Marker`
  is `dml-spreadsheetDrawing.xsd`'s and MJXOFF-107 models it — while their *placement* goes through
  the generated `OBJECT_ANCHOR` table, which ranks them across a namespace boundary no `sml` local
  name reveals. Nine of `CT_ObjectPr`'s twelve attributes are booleans and **six default to `true`**,
  which is the opposite of every other flag family in `sml.xsd`.
- **`mjx_sml::features::annotations`** — `CellWatches`/`CellWatch` (rank 26),
  `IgnoredErrors`/`IgnoredError` (rank 27), and `SmartTags`/`CellSmartTags`/`CellSmartTag`/
  `CellSmartTagProperty` (rank 28). An `IgnoredError` says *do not draw the indicator*, never *there
  is no error*; nothing here evaluates a cell. `CT_CellSmartTag`'s `@deleted` is read through
  `smart_tag_was_deleted`, and `.github/scripts/check-suppress-naming.sh` gains an allow-list entry
  for it — scoped to that exact token in that exact file — because a tag the user removed is still
  written out, which is `CT_InputCells@deleted`'s situation and not the chart family's *"draw nothing
  here"*. The worksheet `smartTags` cluster is **not**
  `xl/workbook.xml`'s near-identically-named `smartTagTypes`, which MJXOFF-100 modelled.
- **`mjx_sml::features::publishing`** — `DataConsolidation`/`DataReferences`/`DataReference`
  (rank 12), `CustomProperties`/`CustomProperty` (rank 25) and `WebPublishItems`/`WebPublishItem`
  (rank 36). A consolidation is a **record**, never performed. A `webPublishItem@destinationFile` is
  an untrusted path on somebody else's disk, carried exactly and never opened.
- **Seven new `WorksheetPart` slots** — `hyperlinks`, `data_consolidation`, `custom_properties`,
  `cell_watches`, `ignored_errors`, `smart_tags` and `web_publish_items`, each with its `_mut` and
  `set_` companions, plus **`hyperlink_covering`, `hyperlink_position_covering`, `add_hyperlink` and
  `remove_hyperlink`**. **Twenty-five of `CT_Worksheet`'s thirty-nine slots are now modelled and
  fourteen held raw**, and the frame's own documentation now names the owner of every one of the
  fourteen.
- **`Workbook::sheet_hyperlinks`, `cell_hyperlink`, `set_cell_hyperlink`, `remove_cell_hyperlink`,
  `add_hyperlink_relationship`**, plus **`SheetHyperlink`**, **`HyperlinkTarget`** and
  **`HyperlinkKind`**. `HyperlinkTarget::Url` keeps `mjx_pptx::Hyperlink::Url`'s name exactly;
  Excel's *internal* kind is a `@location` string and **no relationship at all**, unlike
  PowerPoint's slide jump. **A hyperlink and its relationship are one thing**: setting one writes
  both, removing one removes both, and a relationship survives only while another entry still names
  it.
- **`SpreadsheetDefect::OrphanedHyperlinkRelationship`** — the half packaging cannot state.
  `mjx-opc` reports a dangling `r:id` and is explicit that a relationship nothing names is legal (a
  `comments` relationship is found by *type*), so this fires for the **`hyperlink` relationship type
  alone**, over worksheets this library will write.
- **`mjx_xlsx::parts::REL_HYPERLINK`** — the one relationship in that file that reaches no part.
- **Four generated child-order exports** — `WORKSHEET_IGNORED_ERRORS`, `OBJECT_ANCHOR`,
  `OBJECT_PROPERTIES` and `DATA_CONSOLIDATION` in `mjx_ooxml_types::child_order`, from `xtask`'s
  curated list.
- **`tests/fixtures/hyperlinks.xlsx`** — **all three hyperlink shapes on one sheet**, because one
  kind tests one branch: an internal `@location` over the multi-cell `B4:D6`, an external `@r:id`,
  and one carrying both. The two external entries name `rId2` then `rId1`, **the reverse of the
  `.rels` order**; the `mailto:` target spells its query `%20` and `&amp;`, so any normalisation of
  an untrusted URI shows; one `@display` is **single-quoted**; `dataConsolidate@function` is
  `stdDev`, whose generated variant is `SampleStandardDeviation`; `dataRefs@count` and
  `webPublishItems@count` are both **stale**; one `dataRef` states no attribute at all; the two
  `cellWatch`es are out of address order; `ignoredErrors` carries an `extLst` after its records; and
  the sheet holds an unmodelled `customSheetViews` (rank 13) and `phoneticPr` (rank 15) so the
  modelled and held slots genuinely interleave.
- **A guide page** — *Hyperlinks*, with three compiled doctests.

### Changed

- **`Workbook::next_sheet_relationship_id` is now `pub(crate)`** (it was private to
  `worksheet/tables.rs`). Hyperlinks allocate from the same sheet `.rels`, and a second allocator
  could hand out an id the first had already promised.

### Notes

- **`customSheetViews` (rank 13) is explicitly preserved, and it is MJXOFF-129 (D17)'s**, which names
  `CT_CustomSheetViews`/`CT_CustomSheetView` in its own work list. `CT_CustomSheetView` embeds
  `pageMargins`, `printOptions`, `pageSetup` and `headerFooter` — D17's own print block — so
  modelling it here would have meant a second copy of it or a model holding it raw twice over.
- **Three `CT_Worksheet` slots still have no owner**: `phoneticPr` (15), `legacyDrawingHF` (31) and
  `drawingHF` (32). `CT_PhoneticPr` is already modelled once as `PhoneticProperties`, a value decoded
  from the shared-string store's packed bytes rather than a `RawElement`-backed slot, so giving rank
  15 a type means unifying two call sites — a design question, not a slot to fill. The other two are
  the header/footer half of the drawing family and belong with E3's and E5's. All three round-trip
  verbatim today; `crates/mjx-sml/src/worksheet/frame.rs` records this table for MJXOFF-133 (D18).

## [0.0.116] - 2026-09-06

Worksheet tables — the first feature of Phase D that lives in a **part of its own**, and the first
place this library creates one (MJXOFF-125, Phase D position 15).

### Added

- **`mjx_sml::features::tables`** — `WorksheetTable` (`CT_Table`, `sml.xsd:3946`, the root of
  `xl/tables/tableN.xml`), `TableColumns`/`TableColumn`, `TableFormula` (`CT_TableFormula`),
  `XmlColumnProperties` (`CT_XmlColumnPr`), `TableStyleReference` (`CT_TableStyleInfo`), and
  `TableParts`/`TablePart` — the last of which fills **rank 37 of `CT_Worksheet`**, so eighteen of
  its thirty-nine slots are now modelled and twenty-one held raw. A table's `autoFilter` and
  `sortState` are MJXOFF-123's own `AutoFilter` and `SortState`; there is no second filter or sort
  model. **`sortState` now has three distinct homes** — rank 11 of `CT_Worksheet`, rank 1 of
  `CT_AutoFilter`, rank 1 of `CT_Table` — and they are three different elements.
- **`mjx_sml::styles::table_styles`** — `TableStyles` (`CT_TableStyles`), `TableStyleDefinition`
  (`CT_TableStyle`) and `TableStyleRegion` (`CT_TableStyleElement`). This is **rank 8 of
  `CT_Stylesheet`, the last of its eleven slots to be modelled**; only `extLst` is held raw now.
- **`builtin_table_style_name`, `BuiltInTableStyle`, `BuiltInTableStyleFamily`,
  `TableStyleLookup`, `TableStyleOrigin`** — the distinction the ticket names. Excel's 144 preset
  table styles are in **no `.xlsx` at all**, so a lookup has three answers and not two:
  locally defined, built-in, or genuinely undefined. The six contiguous families and their bounds
  are read off ECMA-376 Part 1's own `presetTableStyles.xml`, on
  `builtin_cell_style_name`'s precedent, and a unit test walks all 144 plus both boundaries.
- **`mjx_sml::WorksheetTableSpec`, `TableColumnSpec`, `TableStyleReferenceSpec`** — plain-data
  authoring descriptions with no interner, and **`mjx_sml::write::AuthoredTable`**, the whole-part
  writer, which seeds `<table xmlns="…"/>` as bytes and writes back the root it *read*.
- **`WorksheetPart::table_parts`/`table_parts_mut`/`set_table_parts`**, and
  **`WorksheetPart::bind_relationship_prefix`** — the second declares `xmlns:r` on a worksheet root
  that binds none, because a `tablePart` is nothing but an `r:id` and a sheet authored from nothing
  declares only the SpreadsheetML namespace. It never overwrites a binding the file made.
- **`StylesheetPart::table_styles`/`table_styles_mut`/`set_table_styles`**.
- **`Workbook::sheet_tables`, `table_markup`, `edit_table_markup`, `table_style_origin`,
  `next_table_id`, `add_table`**, plus the owned reports **`SheetTable`** and **`SheetTableColumn`**.
  `add_table` writes the **four things a table is** — the part, its content-type override, a `table`
  relationship from the *sheet* part, and a `tablePart` entry — in one call.
- **`SpreadsheetDefect::TablePartTargetIsNotATable`, `DuplicateTableId`,
  `DuplicateTableDisplayName`** — the directions packaging cannot see. The last two fault only a
  table **this library wrote**; a workbook that arrived with a collision still saves. A
  `tablePart@r:id` naming *no* relationship is deliberately not restated here: `mjx-opc` already
  reports it over exactly the same set of parts.
- **`SmlError::TableGeometryDoesNotFit`, `TableHasNoColumns`** — the two refusals.
- **`tests/fixtures/worksheet_tables.xlsx`** — **two tables on one sheet**, because one tests neither
  the id allocation nor the built-in/local distinction. They differ in every way that matters: ids
  **1 and 4** (a gap, so the next free id is 5 and not the table count plus one); one wears the
  preset `TableStyleMedium2` and the other the workbook's own `AcmeBlue`; one has a totals row with
  `sum`, `average` *and* `custom` and the other none; one declares `tableColumns@count="9"` against
  four columns. The sheet also lists the two **in the reverse of the relationship order**, and the
  calculated-column formula spells `>` as `&gt;` — an entity XML does not require, so re-escaping it
  would change the bytes.
- **Two generated child-order exports** — `WORKSHEET_TABLE` and `WORKSHEET_TABLE_COLUMN` in
  `mjx_ooxml_types::child_order`, from `xtask`'s curated list.
- **A guide page** — *Worksheet tables*, with four compiled doctests.

### Fidelity

- **A table's `@ref` and its two row counts move together or not at all.** There is deliberately no
  `set_range`: §18.5.1.2 makes the three one statement, so `WorksheetTable::resize` takes all three
  and refuses a combination the range cannot hold.
- **An unrelated cell edit never moves a table's boundary.** Setting a value inside a table rewrites
  one row of the worksheet part; the table part is not opened, and comes back byte for byte.
- **A table's `@id` is never reused and never renumbered.** `Workbook::add_table` allocates one past
  the highest id any table part in the package writes, and `spec.id` is ignored.
- **A calculated column is never expanded** into per-cell formulas, a totals row is never computed,
  and a structured reference (`Sales[[#This Row],[Q1]]`) is never parsed. Formulas are text on
  MJXOFF-115's terms, at two more doors.
- **A preset style name is not a missing style.** Reporting "not found" for `TableStyleMedium2`
  would be a confident wrong answer where the honest one is a distinction.
- **`tableColumns@count`, `tableStyles@count` and `tableParts@count` are producer caches** — refreshed
  when the collection is edited *and* the file declared one, never added to an element that wrote
  none, and never corrected on a read.
- **`xmlColumnPr` is preserved and never resolved**; the XML map part it names is modelled nowhere.

### Fixed

- **`crates/mjx-sml/src/styles/stylesheet.rs` said MJXOFF-127 owned `tableStyles`.** It is
  MJXOFF-125's (D15); MJXOFF-127 is D16. Corrected in three places.

## [0.0.115] - 2026-09-06

Data validation, autofilters and sort state — a cluster whose whole discipline is that **nothing in
it is ever applied**: a filter hides no row, a sort reorders none, and a `list` validation's range
source is text this library never resolves (MJXOFF-123, Phase D position 14).

### Added

- **`mjx_sml::features::filters`** — the autofilter cluster, all thirteen complex types of
  `sml.xsd:16-228`: `AutoFilter` (`CT_AutoFilter`, rank **10** of `CT_Worksheet`), `FilterColumn`,
  `Filters`/`Filter`/`DateGroupItem`, `CustomFilters`/`CustomFilter`, `Top10Filter`, `ColorFilter`,
  `IconFilter`, `DynamicFilter`, `SortState` and `SortCondition`. It is its own module, not part of
  the worksheet, because MJXOFF-125's `CT_Table` embeds `autoFilter` and `sortState` directly and
  MJXOFF-133's pivot filters reference the same types.
- **`mjx_sml::FilterKind`** — `CT_FilterColumn`'s `xsd:choice` as a Rust enum rather than six
  `Option` fields. **Six modelled filter kinds**, not seven: the choice has seven *members* and the
  seventh is `extLst`, the extension slot, which lands in `FilterKind::Raw` and round-trips byte for
  byte. `FilterColumn::set_filter` replaces whichever kind is there, in its position, which is what
  a choice asks for.
- **`mjx_sml::features::validation`** — `DataValidations` (`CT_DataValidations`, rank **17**) and
  `DataValidation` (`CT_DataValidation`), all thirteen attributes and both formula slots.
  `@count` is a producer's cache and is never rewritten, on read or after an append.
- **`mjx_sml::FormulaElement`** — the `ST_Formula` **element**, one type for the three slots that
  share it (`cfRule/formula`, `dataValidation/formula1`, `dataValidation/formula2`). It carries its
  own local name. See the breaking-change row below: this replaces
  `ConditionalFormattingFormula` rather than sitting beside it.
- **`WorksheetPart::auto_filter`/`auto_filter_mut`/`set_auto_filter`**,
  **`sort_state`/`sort_state_mut`/`set_sort_state`** and
  **`data_validations`/`data_validations_mut`/`set_data_validations`**, plus the curated
  `data_validation_rules`, `data_validations_for`, `add_data_validation` and
  `remove_data_validation`. Three more of `CT_Worksheet`'s thirty-nine slots are modelled —
  seventeen now, twenty-two held raw — and the third is the one easy to miss: **`sortState` is a
  slot of the worksheet at rank 11 as well as a child of `autoFilter` at its own rank 1**, two
  different elements of the same complex type.
- **`mjx_sml::AutoFilterSpec`** and its four companions (`FilterColumnSpec`, `FilterSpecKind`,
  `CustomFilterSpec`, `SortStateSpec`, `SortConditionSpec`) and **`mjx_sml::DataValidationSpec`** —
  plain-data authoring descriptions with no interner, on MJXOFF-105's precedent. Unlike
  `ConditionalRuleSpecKind`, **all six** filter kinds are describable, because every one of them is
  completely stated by what the caller passes.
- **`Workbook::auto_filter`, `data_validations`, `set_auto_filter`, `remove_auto_filter`,
  `add_data_validation`, `remove_data_validation`** — the package tier. Each authoring call rewrites
  the worksheet part and **nothing else**, which is asserted against the original file's bytes.
- **`tests/fixtures/validation_and_filters.xlsx`** — **all six filter kinds, one per column**, plus a
  seventh column holding only the choice's `extLst`; `@colId`s that do **not** ascend
  (`1, 0, 4, 2, 5, 3, 6`); a two-condition sort state; four validations including two `list` rules
  whose sources are a range reference and a quoted literal respectively; a hidden row beside a
  visible one; a deliberately stale `@count`; a `@sqref` with a double space in it; and an `x14`
  cross-sheet `dataValidations` in the worksheet `extLst`. One filter kind repeated four times would
  have tested one code path.
- **Five generated child-order exports** — `AUTO_FILTER`, `FILTER_COLUMN`, `FILTERS`, `SORT_STATE`
  and `DATA_VALIDATION` in `mjx_ooxml_types::child_order`, from `xtask`'s curated list. Every
  placement in this cluster goes through them. `FILTER_COLUMN` is the first `ContentModel::Choice`
  entry any model here consumes, and every one of its members ranks 0 — which is the schema's answer
  rather than a shortcoming of the table.
- **A guide page** — *Filters and data validation*, with four compiled doctests, one of which
  asserts that a filter matching no row hides no row.

### Changed

- **`mjx_sml::features`' subject modules are public**, as `mjx_sml::formula`'s and
  `mjx_sml::styles`' already were, so a reader who reaches one of these types through its re-export
  can reach the design record behind it. Purely additive.

### Fidelity

- **A filter never hides a row.** Reading, writing or removing an `autoFilter` leaves every row's
  `@hidden` exactly as the file wrote it. A hidden row is MJXOFF-117's row property, and this
  library did not put it there.
- **A sort state never reorders one.** `CT_SortState` records a sort that happened.
- **A `list` validation's source is never resolved.** `formula1` may hold `$G$2:$G$4`,
  `Lookups!$A$1:$A$9` or `"Low,Medium,High"`; all three go in and come out as the same bytes, and
  neither spelling is converted into the other.
- **Excel's caches are reported, never recomputed** — `top10@filterVal`, `dynamicFilter@val`/
  `@maxVal`, and `dataValidations@count`.
- **`@calendarType` and `@blank` on `CT_Filters` are modelled.** The first says which calendar the
  date groups are in; the second is *(Blanks)*, which cannot be expressed as a `filter` child.
- **The `x14` cross-sheet validations are preserved and not modelled**, prefix, `uri` and bytes
  intact, through an unrelated edit.

## [0.0.114] - 2026-09-05

Conditional formatting: the rule kinds, the priority order that runs *across* blocks, and the `dxf`
layer that is reported beside a cell's format rather than folded into it (MJXOFF-120, Phase D
position 13).

### Added

- **`mjx_sml::features`** — a directory of subject modules from the first commit:
  `conditional_rules` (`CT_ConditionalFormatting`, `CT_CfRule`, a rule's `formula`),
  `conditional_scales` (`CT_Cfvo`, `CT_ColorScale`, `CT_DataBar`, `CT_IconSet`),
  `conditional_chain` (the cross-block order and the two-layer answer) and `conditional_specs`
  (the plain-data authoring vocabulary). Nothing over 500 lines.
- **`mjx_sml::ConditionalFormatting`** — `CT_ConditionalFormatting` (`sml.xsd:2709`), at rank **16**
  of `CT_Worksheet`. It and `cols` are the **only two** of that type's thirty-nine children declared
  `maxOccurs="unbounded"`, so `WorksheetPart` grows a *list* surface for it —
  `conditional_formatting_blocks`, `conditional_formatting_block_mut`,
  `push_conditional_formatting`, `remove_conditional_formatting` — never an `Option`. Merging the
  blocks would change the file and would destroy the thing that makes the feature hard.
- **`mjx_sml::ConditionalFormattingRule`** — `CT_CfRule` (`2717`), all thirteen attributes and all
  four child kinds, with every accessor named from ECMA-376 Part 1 §18.3.1.10's own prose
  (`stops_lower_priority_rules`, `top_or_bottom_count`, `ranks_from_bottom`, `includes_the_average`).
- **`mjx_sml::ColorScale`, `DataBar`, `IconSet`, `ConditionalValueObject`** — `CT_ColorScale`
  (`2769`), `CT_DataBar` (`2775`), `CT_IconSet` (`2784`), `CT_Cfvo` (`2793`). `@iconSet`'s schema
  default `3TrafficLights1` is the **generated** `IconSetType::ThreeTrafficLights`; nothing here
  writes a table of icon-set names.
- **`mjx_sml::ConditionalFormattingFormula`** — `cfRule/formula`, on MJXOFF-115's terms exactly:
  text in, the same text out, never parsed and never rewritten. Its `FromXml`/`ToXml` pair is
  hand-written for the reason `DefinedName`'s is — minimal re-escaping is lossy for preservation.
- **`WorksheetPart::conditional_rules_for`** and **`mjx_sml::ConditionalRuleChain`** — every rule
  that applies to a cell, merged across every block whose `@sqref` covers it and sorted by
  `@priority` across all of them at once. Stable, so equal priorities keep document order.
- **`mjx_sml::ConditionalCellFormat`** and **`CellFormatResolver::conditional_cell_format`** — a
  cell's base format and its conditional candidates, **side by side, with no call that merges
  them**. MJXOFF-108 documented this seam; it is now filled *beside* the resolver rather than inside
  it, and `CellFormatResolver::differential_format` is the one addition to that type.
- **`StylesheetPart::append_differential_format`** and **`Workbook::append_differential_format`** —
  appends a `dxf` and answers the index it appended at. Appending is the table's only mutation:
  a `@dxfId` is a position, so inserting or reordering would silently repoint every rule above it.
- **`Workbook::conditional_rules_for`, `conditional_cell_format`, `add_conditional_formatting`** and
  **`SheetFormatting::conditional_cell_format`** — the package tier, which adds exactly one thing the
  markup tier cannot reach: conditional formatting spans *two* parts, and authoring a highlighted
  rule writes both.
- **`mjx_sml::ConditionalRuleSpec`** and its four companions (`ColorScaleSpec`, `DataBarSpec`,
  `IconSetSpec`, `ConditionalValueObjectSpec`, `DifferentialFormatSpec`) — plain-data descriptions
  with no interner, on MJXOFF-105's precedent. They cover the five rule kinds whose markup is
  *completely* determined by their arguments; the other thirteen `ST_CfType` members are authored
  through the model, because a spec that wrote only `type="top10"` would author markup known to be
  incomplete.
- **`tests/fixtures/conditional_formatting.xlsx`** — **four blocks whose priorities interleave**
  (block 0 holds 1 and 4, block 1 holds 2, block 2 holds 3, block 3 holds 2 again and 7), a
  multi-range `@sqref`, a duplicate priority, a gap, a `stopIfTrue`, all four rule kinds, two `dxf`
  entries, and an `x14` `extLst` in two places. A fixture with one block per priority range tests
  nothing: a per-block sort and a cross-block sort agree on it.
- **Four generated child-order exports** — `WORKSHEET_CONDITIONAL_FORMATTING`,
  `CONDITIONAL_FORMAT_RULE`, `CONDITIONAL_FORMAT_COLOR_SCALE`, `CONDITIONAL_FORMAT_DATA_BAR`, added
  to `xtask`'s curated list and regenerated. `CT_IconSet` and `CT_Cfvo` are deliberately absent:
  each declares a single repeating child or none, so both are appends with no order to hold.
- **`SmlError::ConditionalFormattingBlockHasNoRange`** and
  **`ConditionalFormattingRuleHasNoPriority`** — the two things a chain cannot be answered *around*.
  A block whose `@sqref` is missing might be the one covering the cell, and a rule with no
  `@priority` has no place in the order; a silently shortened chain would report the wrong rule as
  winning.
- **A guide page**, `crates/mjx-xlsx/docs/guide/conditional_formatting.md`, beside the formulas page
  that draws the same boundary.

### The position, stated rather than implied

- **Reporting which rules apply is in scope. Deciding whether a rule is *true* is not, ever.** That
  needs a calculation engine, which MJXOFF-115 settles as absent by design. `stopIfTrue` is
  therefore reported as a *position* in the chain and never applied as a truncation — §18.3.1.10
  makes the stop conditional on the rule firing.
- **Priorities are as read; nothing renumbers.** Excel's own files have gaps and duplicates, and
  renumbering changes which rule wins. The fixture has both, and the byte-identity gate is what
  keeps it that way.
- **A cell's conditional layer is reported alongside its base format, never folded in.** Every member
  of a `dxf` is optional and absent means *inherited*, so even a fold performed by a consumer that
  *could* evaluate would need both halves; producing one merged answer here would assert a rule
  fired.
- **The `x14` extensions round-trip and are not modelled.** Data bars with negative fills and
  icon-set overrides live in that namespace; they come back through an unrelated edit byte for byte,
  prefix, `uri` and GUIDs included.

## [0.0.113] - 2026-09-05

The sheet grid — merging, row and column geometry, outline levels, page breaks, sheet protection and
scenarios (MJXOFF-117, Phase D position 12).

> **Recorded late.** MJXOFF-117's own release commit (`c2b965f`) bumped the workspace to `0.0.113`
> and the wasm package with it, but added no entry here; the omission was found by MJXOFF-120 while
> writing the entry above. What follows is reconstructed from that child's own commits and code, so
> that the ledger has no hole in it.

### Added

- **Six more of `CT_Worksheet`'s slots modelled** — `sheetProtection` (7), `protectedRanges` (8),
  `scenarios` (9), `mergeCells` (14), `rowBreaks` (23) and `colBreaks` (24) — in a directory of
  subject modules under `mjx_sml::worksheet`: `merges`, `breaks`, `protection`, `scenarios`, `rows`
  and `anomalies`.
- **`WorksheetPart::merge_cells` / `unmerge_cells` / `merge_anchor` / `is_covered_by_merge` /
  `cell_span`**, and the same surface at the package tier, plus
  `Workbook::effective_merged_cell_format`.
- **Run-length column splitting** — setting one column's width inside a `col` run breaks it into up
  to three, and only the middle piece takes the new value.
- **`RowHeight` / `ColumnWidth`** — the `customHeight`/`customWidth` flag travels with its value in
  the type, so no call can write a height without saying where the number came from.
- **`WorksheetPart::grid_anomalies`** — overlapping merges, merges over populated cells, and
  conflicting `col` runs are described rather than repaired.
- **`tests/fixtures/sheet_grid.xlsx`**, and `SmlError::MergeOverlapsExistingMerge` /
  `DegenerateMerge`.

### Fixed

- **`Slot::rank` ranks a held element through the generated table.** MJXOFF-102 modelled ranks 0–6,
  a *prefix*, which was the only reason an unmodelled child could safely be treated as unrankable.
  Promoting ranks 7, 8, 9, 14, 23 and 24 broke the prefix, and a `mergeCells` (14) inserted beside a
  held `autoFilter` (10) would have landed in schema-invalid order. MJXOFF-120 depends on this fix:
  `conditionalFormatting` is rank 16, between a held `phoneticPr` (15) and a held `dataValidations`
  (17).

## [0.0.112] - 2026-09-05

Formulas, carried as text — and the written-down guarantee that nothing here ever acts on one
(MJXOFF-115, Phase D position 11).

### Added

- **`mjx_sml::formula`** — a directory of subject modules: `cell` (`CT_CellFormula`), `cached`
  (the `<v>` beside an `<f>`), `shared` (the `@si` grouping) and `calc_chain` (`CT_CalcChain` /
  `CT_CalcCell`). Nothing over 400 lines.
- **`mjx_sml::CellFormula`** — `CT_CellFormula` (`sml.xsd:2751`), all **twelve** attributes, as a
  *borrowed view over the `<f>` element's own bytes*. It adds **zero bytes per cell**: MJXOFF-95's
  cell store already kept the formula's byte range, and this gives those bytes a type rather than a
  second home. A decoded struct per cell — a `String` for the text, `Option<CellRange>` for `@ref`,
  seven `bool`s — was rejected on both counts it would fail: memory (a million formula cells) and
  fidelity (nothing decoded reproduces `&quot;`, a single-quoted value, or `si` written before `t`).
- **`mjx_sml::FormulaKind`** — `ST_CellFormulaType`'s four values, **re-exported from the generated
  `mjx_ooxml_types::spreadsheetml::CellFormulaType` rather than declared a second time.**
  `has_written_kind` is the distinction a round trip turns on: `t` is declared
  `use="optional" default="normal"`, so an absent `t` and `t="normal"` mean the same thing and must
  not be written the same way.
- **`mjx_sml::CachedValue`** and **`Cell::cached_value`** — the result a producer last computed, read
  through the `c@t` beside it. `None` for a `<v>` with no `<f>`: a value somebody typed and a result
  Excel computed are different facts about the file.
- **`mjx_sml::SharedFormulaGroups` / `SharedFormulaGroup`** and **`SheetData::shared_formula_groups`**
  — the `@si` grouping, indexed on demand and held nowhere. It reports the host, the group's range
  and its cell count; it deliberately does **not** answer "what is this member's formula", because
  that answer is the host's text shifted by the offset between the two cells, and shifting references
  is translation.
- **`mjx_sml::CalculationChain` / `CalculationChainCell`** and **`mjx_xlsx::Workbook::calculation_chain`**
  — `CT_CalcChain` (`sml.xsd:257`) and `CT_CalcCell` (`263`), with §18.6.1's two carry-forward rules
  (`@i` and `@s` take the previous entry's value when absent) resolved by `CalculationChain::resolved`
  and by nothing on the write path.
- **`tests/fixtures/formulas.xlsx`** — a normal formula writing no `t`, one writing `t='normal'`
  single-quoted, a shared group of five (host plus four text-less members, written four different
  ways), an array formula over a range whose other cells carry no `<f>` at all, a data-table formula
  with all six of its attributes, formula text carrying `&lt;`/`&amp;`/`&quot;`, and an
  `xl/calcChain.xml` of eleven entries.
- **A fifth case in `crates/mjx-sml/tests/cell_store_allocation.rs`** — 300,000 cells, *every one* a
  formula cell, 295,000 of them text-less shared-group members. Measured: **76.8 B/cell**, against
  36.8 for the same sheet of values and the 913 B/cell `docs/BENCHMARKS.md` (MJXOFF-147) records for
  a `RawElement` tree of it. The 40-byte difference is MJXOFF-95's `CellExtras`, which a formula cell
  already paid for; **a group member costs what any formula cell costs and nothing for its
  membership**.
- **A guide page**, `crates/mjx-xlsx/docs/guide/formulas_and_cached_values.md`, and a new section on
  the fidelity page — both stating the stale-cache limitation in prose and naming it deliberate.

### The position, stated rather than implied

- **Nothing here calculates, and a stale cached value is correct behaviour.** An edit that changes a
  cell a formula depends on leaves the cached `<v>` exactly as it was. Recalculating needs an engine
  this workspace does not have and will not grow; blanking the `<v>` destroys data in a file the
  caller opened to change a label; marking the workbook dirty writes into a part nobody asked to
  edit. `crates/mjx-sml/tests/formulas.rs` and `crates/mjx-xlsx/tests/formulas.rs` fail if any of the
  three ever starts happening.
- **A shared group's text distribution is preserved exactly.** The host carries the text and the
  members carry none. Expanding a group to per-cell text on write is a corruption and not an
  optimisation: it changes bytes nobody asked to change, and because a shared formula's references
  are written relative to the host, a copied expression states a different formula from the one that
  cell has.
- **`xl/calcChain.xml` is left exactly as found** — neither maintained nor dropped. Maintaining it
  means computing a dependency order, which means parsing expressions. Dropping it is an edit the
  caller did not ask for, made on every save, and it loses a record they may be reading the file to
  inspect. §18.6 says a consumer "is free to perform calculations in a different order at run time",
  which is what makes leaving a stale chain safe.

### Fixed

- `crates/mjx-xlsx/docs/guide/fidelity_and_the_part_graph.md` said "Styles and formulas are not
  modelled at all yet", which stopped being true at MJXOFF-105.

## [0.0.111] - 2026-09-05

The `mjx-sml` package writer that replaces `mjx_chart::EmbeddedWorkbook`, `Workbook::blank`, and the
Excel authoring surface (MJXOFF-112, Phase D position 10).

### Added

- **`mjx_sml::write`** — the SpreadsheetML **package writer**, a directory of subject modules:
  `constants` (part names, content types, relationship types), `workbook` (`xl/workbook.xml`),
  `sheet` (one authored worksheet), `stylesheet` (`xl/styles.xml` and its four appends),
  `style_specs` (the plain-data descriptions those appends take) and `package`
  ([`WorkbookPackage`], which assembles the whole `.xlsx`). It authors `[Content_Types].xml`,
  `_rels/.rels`, `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, `xl/worksheets/sheetN.xml`,
  `xl/sharedStrings.xml`, `xl/styles.xml` and — on request — `docProps/core.xml` and
  `docProps/app.xml`.
- **`mjx_sml::write::AuthoredCellValue`** — a cell value stated *before* there is a shared-string
  table to point into, which is the shape a caller actually holds. `WorkbookPackage::push_row`
  interns text in first-use order and writes nothing at all for a `Blank` or a non-finite number,
  which is what a grid with holes in it means; `WorkbookPackage::set_cell_value` takes the wire-shaped
  `CellValue` and **refuses** a non-finite number instead, because naming one cell is stating a value.
- **`mjx_sml::write::{PatternFillSpec, BorderSpec, BorderEdgeSpec, CellFormatSpec}`** — plain-data
  descriptions of the four `styles.xml` resources, with no interner and no lifetime. A `Border` is
  **nine** edges (`start`, `end`, `left`, `right`, `top`, `bottom`, `diagonal`, `vertical`,
  `horizontal`), not four. Fonts reuse `FontProperties`, which is already that description.
- **`mjx_xlsx::Workbook::blank`** and **`blank_with_properties`** — a workbook authored from nothing,
  through the `mjx-sml` writer. **There is no schema-valid empty workbook**: `CT_Workbook` has
  nineteen slots and only `sheets` is mandatory, and `CT_Sheets` requires at least one `sheet`, so
  `blank()` necessarily authors a worksheet part, its content type, its relationship and a
  `sheetData` as well. Deterministic — two calls produce byte-identical containers.
- **`mjx_xlsx::Workbook::add_sheet`, `set_cell_style`, `intern_shared_string`, `append_font`,
  `append_pattern_fill`, `append_border`, `append_cell_format`** — the authoring surface. Concrete
  types, no closures in a signature, nothing returning a borrowed view into the workbook.
- **`mjx_sml::SmlError::AuthoredPartSeedRejected`** and **`SheetIndexOutOfRange`**.
- **`mjx-chart` now depends on `mjx-sml`** — the legal downward edge (2.2 → 2.1) MJXOFF-99 needs, in
  place and green under `xtask/tests/layering.rs`. **Nothing is removed here**: `EmbeddedWorkbook`
  and `to_package_bytes` are untouched, and MJXOFF-99 owns their removal.

### Changed

- **`mjx_xlsx::parts` re-exports eight constants from `mjx_sml::write::constants`** rather than
  declaring them a second time — `REL_OFFICE_DOCUMENT`, `REL_WORKSHEET`, `REL_SHARED_STRINGS`,
  `REL_STYLES`, `CONTENT_TYPE_WORKBOOK`, `CONTENT_TYPE_WORKSHEET`, `CONTENT_TYPE_SHARED_STRINGS`,
  `CONTENT_TYPE_STYLES`. Same paths, same values, one definition. The other twenty-one stay this
  crate's: reading a part graph needs them and authoring one does not.
- **`SharedStringTable::authored` writes an empty table self-closing** (`<sst … count="0"
  uniqueCount="0"/>`), which is what Excel and `mjx-chart`'s writer both emit. The non-empty form is
  unchanged, and the parity gate compares the two byte for byte.

### Verified

- **The parity gate** (`crates/mjx-chart/tests/workbook_parity.rs`): the same grid through
  `EmbeddedWorkbook` and through `WorkbookPackage`, compared part by part. `[Content_Types].xml`,
  `_rels/.rels`, `xl/_rels/workbook.xml.rels`, `xl/workbook.xml`, `xl/worksheets/sheet1.xml` and
  `xl/sharedStrings.xml` are **byte-identical**. `xl/styles.xml` states the same six tables with the
  same counts and the same record behind every index-0 reference, and differs only in the order of a
  font's children — `CT_Font` is an `xsd:choice`, so the schema imposes none.
- **The writer needs nothing above `mjx-sml`** (`crates/mjx-sml/tests/package_writer.rs`): a whole
  package is authored, read back and asserted on from inside a crate whose manifest names no format
  crate, no facade and no `mjx-chart` — checked by reading that manifest rather than claimed.
- Every part `Workbook::blank` authors validates against `sml.xsd` and the OPC schemas, and is in
  `xsd:sequence` order (`crates/mjx-xlsx/tests/schema_gate.rs`).
- **The office-open canary now covers Excel.** `crates/mjx-xlsx/tests/office_open.rs` drives
  LibreOffice over the blank workbook, an authored-and-filled-in one, `sample.xlsx` and a
  round-tripped `sample.xlsx`; `.github/workflows/ci.yml` installs `libreoffice-calc` and names
  `-p mjx-xlsx` on the canary step. Schema validity is necessary and not sufficient — a package can
  satisfy every XSD and still be refused for a broken relationship graph — and a package filter that
  named only two of the three crates would have dropped these four cases with the job still green,
  which is precisely the failure MJXOFF-98 found for Word.

## [0.0.110] - 2026-09-05

`xl/styles.xml` part 2: the `xf` indirection, number formats, named styles, and the resolver that
answers what formatting a cell actually carries (MJXOFF-108, Phase D position 9).

### Added

- **`mjx_sml::styles::formats`** — `CT_Xf` (`sml.xsd:3598`) as `CellFormat`, and `CT_CellXfs` /
  `CT_CellStyleXfs` as one `CellFormatTable` with a `CellFormatTableKind` to say which slot an
  authored one stands in. The two complex types are character for character identical; declaring
  them twice would have been two copies of one index-identity discipline to keep in step.
  **`CT_Xf` carries thirteen attributes, not the fourteen its ticket claimed.**
- **`mjx_sml::ApplyFlag`** — `applyNumberFormat`, `applyFont`, `applyFill`, `applyBorder`,
  `applyAlignment` and `applyProtection` in **all three** of the states the schema gives them. Every
  one is declared `use="optional"` with **no `default=`**, so *absent*, `"1"` and `"0"` are three
  distinct values, and collapsing absent into false is a defect no schema validator can see — both
  spellings are valid documents. `ApplyFlag::participates` is where the meaning lives: absent
  participates, because §18.8.9's worked example contrasts a record that "does not express any
  'apply' attributes" (the `Normal` style, which is applied) with records that suppress by writing
  `applyX="0"`.
- **`mjx_sml::styles::number_formats`** — `CT_NumFmts` as `NumberFormatTable`, keyed by
  `@numFmtId` rather than by position (the only table in the part that is not an array), plus
  ECMA-376 Part 1 §18.8.30's **implied** format codes: `builtin_format_code` for the twenty-eight
  ids listed under *All Languages*, `builtin_format_code_in` and `NumberFormatLanguage` for the
  ids whose code depends on the UI language, and `is_locale_dependent`. The all-languages set is
  **not** `0..=49` — 5–8, 23–26 and 41–44 are not built in and §18.8.30 says so — and the
  locale-dependent ids run to **81**, not 49. Ids 37 and 38 carry a space before their semicolon
  while 39 and 40 do not; that asymmetry is the published table's and is reproduced exactly.
- **`mjx_sml::styles::named_styles`** — `CT_CellStyles` / `CT_CellStyle` as `NamedCellStyles` /
  `NamedCellStyle`, with **all six** of `CT_CellStyle`'s attributes (its ticket named three), and
  Annex G.2's fifty-one built-in names through `builtin_cell_style_name`. `builtinId` 1 and 2 are
  `RowLevel_` / `ColLevel_` plus the style's own `@iLevel`, so `BuiltInCellStyleName` keeps them
  apart from the forty-nine fixed names rather than inventing a level to concatenate.
- **`mjx_sml::styles::effective`** — the resolver. `CellFormatResolver` decodes every `xf` in both
  tables **once**, and then `EffectiveCellFormat` is `Copy` and per-cell resolution parses and
  allocates nothing. `cell_style_index` walks cell → row (gated on `customFormat`, per §18.3.1.73)
  → column → the default record and reports which layer answered as a `StyleIndexSource`;
  `ResolvedAspect` reports, per aspect, the `applyX` it saw, the `FormatLayer` that supplied the
  value and the resource index it points at. `ColumnStyles` decodes a sheet's `col@style` runs once.
  Reading takes `&self` throughout and cannot mark a part dirty.
- **`mjx_xlsx::Workbook::styles_markup`, `sheet_formatting`, `effective_cell_format`**, and
  `SheetFormatting` / `SheetFormatResolver` — the package tier finds the two parts and hands the
  `mjx-sml` resolver the bytes. **The resolution order is not repeated there**; a second walk would
  be a second answer to one question, free to drift.
- **`mjx_sml::SmlError::CellFormatIndexOutOfRange`** — a style index naming no `xf` is refused
  rather than answered with record 0. A dangling `@xfId` on a record that *does* exist is a
  different thing: that is a layer which is absent, reported as `FormatLayer::Neither`.
- **`STYLESHEET_CELL_FORMAT`** — the generated child-order table for `CT_Xf`'s three children.
- **`tests/fixtures/effective_cell_format.xlsx`** — a workbook whose `cellXfs` and `cellStyleXfs`
  entries **deliberately disagree, property by property**: different number format, font, fill,
  border, alignment and protection on the two layers of one cell, so reading the wrong layer gives a
  visibly wrong answer. Four `cellXfs` records name the same underlying record and the same four
  indices and differ only in their `applyX` — false, absent, true, and one record mixing false with
  true — because that is the only arrangement that can tell the three states apart. A fifth sits on a
  style record that suppresses `applyFont` itself. The worksheet sets a cell, its row and its column
  to three different fonts, and writes one row with `s` and **no** `customFormat` so the gate on that
  attribute is load-bearing.
- **`docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md`** — the comparison table for MJXOFF-122 (F1), handed
  over **unmarked**: twenty-eight rows of answers this workspace gives, with the *Excel says* and
  *Verdict* columns deliberately empty. Two rows are inferences from a worked example rather than
  from a normative sentence, and are labelled as such.

### Changed

- **`mjx_sml::StylesheetPart` now models nine of `CT_Stylesheet`'s eleven slots**, up from five.
  `numFmts` (rank 0), `cellStyleXfs` (4), `cellXfs` (5) and `cellStyles` (6) join the four resource
  tables and `colors`; only `tableStyles` (MJXOFF-127) and `extLst` are still held raw. The
  interleaving MJXOFF-105 found is narrower and has not gone away — `tableStyles` at rank 8 still
  sits between `dxfs` at 7 and `colors` at 9 — so placement still ranks unmodelled elements through
  the generated table by their own name.

## [0.0.109] - 2026-09-05

`xl/styles.xml` part 1: the resource tables a style index resolves into — fonts, fills, borders,
`dxf`s and the indexed-colour legacy (MJXOFF-105, Phase D position 8).

### Added

- **`mjx_sml::styles`** — `CT_Stylesheet` (`sml.xsd:3387`) as `StylesheetPart`, in eight subject
  modules. Five of the eleven slots are modelled — `FontTable`/`Font`, `FillTable`/`Fill`/
  `PatternFill`/`GradientFill`/`GradientStop`, `BorderTable`/`Border`/`BorderEdge`,
  `DifferentialFormats`/`DifferentialFormat` and `ColorTable`/`IndexedColors`/`MruColors`/`RgbColor`
  — and the other **six are held as the markup the file wrote, in their schema position**:
  `numFmts`, `cellStyleXfs`, `cellXfs` and `cellStyles` are MJXOFF-108's, `tableStyles` is
  MJXOFF-127's, and `extLst` is nobody's on purpose.
- **`mjx_sml::styles::palette`** — the indexed-colour legacy, sourced from ECMA-376 Part 1 §18.8.27:
  `IndexedColorPalette::DEFAULT` is the spec's own sixty-four rows (`0`–`7` are the spec's stated
  duplicates of `8`–`15`), `IndexedColor::SystemForeground`/`SystemBackground` are its `indexed="64"`
  and `"65"`, which have no ARGB, and a workbook's `indexedColors` block replaces the whole table.
  `theme_color_slot` is §20.1.6.2's index table — SpreadsheetML addresses a theme colour by
  *position*, not by a `schemeClr` token. `apply_tint` is §18.8.19's luminance algorithm, and
  `resolve_color` puts the three together, taking `mjx_dml::SchemeColors` so that a workbook's theme
  colour resolves to exactly what a DrawingML `a:schemeClr` on the same slot resolves to.
- **`mjx_sml::styles::cell_format`** — `CellAlignment`, `CellProtection` and `NumberFormat`
  (`CT_CellAlignment`, `CT_CellProtection`, `CT_NumFmt`), in a module belonging to neither subject
  because `CT_Dxf` needs all three now and `CT_Xf` needs them at MJXOFF-108.
- **`mjx_sml::ColorElement`** — `CT_Color` as an *element*, one type for all five local names it
  stands under (`color`, `fgColor`, `bgColor`, `tabColor`). Preservation; `Color` stays the decoded
  snapshot.
- **`mjx_dml::SchemeColors::rgb`** is public. The type exists to be the interner-free bridge between
  a colour in one part and a theme in another, and a bridge only `mjx-dml` can cross is a bridge to
  nowhere.
- **Four generated child-order tables** for the styles cluster: `STYLESHEET_BORDER`,
  `STYLESHEET_PATTERN_FILL`, `STYLESHEET_DIFFERENTIAL_FORMAT` and `STYLESHEET_COLOR_TABLE`.
- **`tests/fixtures/style_resources.xlsx`** — a styles part authored against this subject's own
  traps: **two byte-identical `<font>` entries**, so deduplicating on write breaks the
  index-identity case rather than passing it; a `<border>` exercising all **nine** edges, `start` and
  `end` included; four `dxf`s (fill only, font only, all six members, and `<dxf/>`); an
  `indexedColors` block differing from the default palette at exactly one row; and four of the six
  raw slots, a doubled space in two start tags, a single-quoted attribute, an element written
  `<top …></top>`, a comment between two slots and an `ext` in a foreign namespace. Schema-valid, so
  it needs no tolerance entry.

### Changed

- **`mjx_sml::TabColor` is gone; the slot is `ColorElement`.** MJXOFF-102 declared a `tabColor`
  attribute bag of its own; MJXOFF-105 found four more slots of the same complex type in
  `styles.xml`, and five bag types for one `CT_Color` is exactly the duplication this crate already
  has a scheduled child to undo once. `SheetProperties::tab_colour` and
  `SheetProperties::tab_color_element` are unchanged in meaning.
- **The styles frame ranks its unmodelled slots too**, which neither `WorkbookPart` nor
  `WorksheetPart` has to. Those two model a *prefix* of their sequence (ranks 0–17 of nineteen, 0–6
  of thirty-nine), so a modelled child always belongs before every raw one and unranked-means-stepped
  -over is harmless. `CT_Stylesheet`'s modelled ranks are **1, 2, 3, 7, 9** and its raw ones **0, 4,
  5, 6, 8, 10**: they interleave, so `StylesheetPart`'s setters take an `&Interner` and rank a raw
  element through the same generated table by its own name. Treating one as unranked put an inserted
  `colors` before the `numFmts` already in the file.

### Notes on the specification

- **The indexed palette has sixty-four rows, not fifty-six**, plus two indices that are not colours.
  §18.8.27's own note explains it: *"0-7 are redundant of 8-15 to preserve backwards
  compatibility"*, so the distinct BIFF palette is `8`..=`63` and a lookup table starting at `8`
  would answer nothing for the eight indices a file most often uses for black and white.
- **§18.8.19's third worked tint example rounds.** It prints
  `100 * .25 + (255 - 255 * .25) = 25 + (255 - 63) = 217`, but `255 * .25` is `63.75`; the exact
  result is `216.25`, and `217` comes of truncating an intermediate in integer HLS arithmetic. This
  crate computes on the unit interval — both branches are linear in luminance, so `HLSMAX` cancels —
  and rounds once, at the end.
- **`CT_Border` declares nine edges**, `start` and `end` among them, while §18.8.4's prose enumerates
  five and §18.8 carries no entry for either of the two. They are documented in WordprocessingML
  instead (§17.4.33 *Leading Edge Border*, §17.4.12 *Trailing Edge Border*), which is where this
  crate takes `Border::leading_edge`/`trailing_edge` from — the spelling `mjx_docx::Indentation`
  already uses for the same pair.

## [0.0.108] - 2026-09-05

The worksheet spine: `CT_Worksheet`'s thirty-nine slots, the widest content model in the schema
(MJXOFF-102, Phase D position 7).

### Added

- **`mjx_sml::worksheet`** — `CT_Worksheet` (`sml.xsd:2170`) as `WorksheetPart`, in four subject
  modules. Seven of the thirty-nine slots are modelled — `SheetProperties` (`sheetPr`, with
  `TabColor`, `OutlineProperties` and `PageSetupProperties`), `SheetDimension`, `SheetViews` /
  `SheetView` / `SheetPane` / `Selection` / `PivotSelection`, `SheetFormatProperties`,
  `ColumnBlock` / `ColumnRun`, `mjx_sml::SheetData` (MJXOFF-95's store) and
  `SheetCalculationProperties` — and the other **thirty-two are held as the markup the file wrote,
  in their schema position**. A worksheet whose `pageSetup` survives is proof the frame works, not
  proof `pageSetup` was modelled.
- **`mjx_xlsx::Workbook`** grows the worksheet surface: `worksheet_markup`, `worksheet_markup_of`,
  `write_worksheet_markup`, `set_cell_value`, `cell_text`, `shared_strings` and `package`. A third
  guide page, *Reading and editing cells*, covers them.
- **`mjx_xml::fidelity::serialize_start_tag` / `serialize_end_tag`** — an element's tags on their
  own. `serialize_element` already existed for a model that holds no tree but does hold whole
  elements; the worksheet frame holds no tree and its `sheetData` child is a packed byte store rather
  than a `RawElement`, so it writes `<worksheet …>`, then each slot's bytes, then `</worksheet>`.
- **`tests/fixtures/worksheet_spine.xlsx`** — a worksheet carrying one child from every later Phase D
  child's territory, **two `<cols>` blocks** rather than one (the slot is `maxOccurs="unbounded"`, so
  merging them changes the file), `mergeCells` present with `autoFilter` deliberately absent, a
  frozen pane with two selections, a comment between two slots, and — on one modelled slot and one
  unmodelled one — a doubled space inside a start tag that nothing but the verbatim source range
  reproduces. Schema-valid, so it needs no tolerance entry.
- **`mjx_sml::Color::read_attributes`** — `Color::read` for a caller holding the attribute list
  rather than the element, so `sheetPr/tabColor` can be *preserved* as an attribute bag and *decoded*
  on demand rather than stored as a lossy snapshot.

### Changed

- **`WorksheetPart` consumes the document it is read from**, rather than borrowing a tree the package
  caches. `docs/BENCHMARKS.md` records 913 bytes of peak resident set per cell for a 300,000-cell
  worksheet held as a `RawElement` tree; the packed store holds the same sheet in 36.8, and a frame
  that kept the tree alive beside it would hand the 25× straight back. Copy-on-write is therefore
  restated at a fourth granularity — per **slot** — with the same *exactly one door* rule the cell
  store uses: a modelled slot's verbatim bytes are given up by the `_mut` accessor and by the setter,
  and by nothing else. `crates/mjx-sml/tests/cell_store_allocation.rs` gains two cases that measure
  both claims through the real part.
- `mjx_sml`'s attribute-bag macro moved from `workbook/leaf.rs` to `leaf.rs`: nine more of the types
  it declares are the worksheet's, and the macro never belonged to the workbook.
- `crates/mjx-xlsx/tests/schema_gate.rs`'s "no part under `xl/` is skipped" rule now sweeps **every**
  committed `.xlsx` rather than `sample.xlsx` alone — `worksheet_spine.xlsx` is the first fixture with
  a part under `xl/` that `sample.xlsx` does not have, and pinning one fixture would have let it join
  the sweep as a skip.

### Fixed

- `crates/mjx-sml/tests/cell_store_fidelity.rs`'s corpus sweep listed every part under
  `/xl/worksheets/` as a worksheet, including the `_rels` streams. No committed fixture had a
  worksheet-level `.rels` until this child added one, so the defect was latent rather than wrong.

## [0.0.107] - 2026-09-05

`xl/workbook.xml`: the part that names every sheet (MJXOFF-100, Phase D position 6).

### Added

- **`mjx_sml::workbook`** — `CT_Workbook` (`sml.xsd:4097`) and the twenty-nine complex types its
  cluster runs to at `sml.xsd:4439`, in eight subject modules. `WorkbookPart` is the nineteen-slot
  sequence; `SheetList`/`SheetEntry`, `WorkbookProperties`, `FileVersion`, `FileSharing`,
  `WorkbookProtection`, `FileRecoveryProperties`, `EmbeddedObjectSize`, `BookViews`/`WorkbookView`,
  `CustomWorkbookViews`/`CustomWorkbookView`, `CalculationProperties`, `DefinedNames`/`DefinedName`,
  `ExternalReferences`/`ExternalReference`, `PivotCaches`/`PivotCache`,
  `FunctionGroups`/`FunctionGroup`, `SmartTagProperties`/`SmartTagTypes`/`SmartTagType`,
  `WebPublishing`/`WebPublishObjects`/`WebPublishObject` are the rest.
- **`mjx_sml::BuiltInName`** — the eight names ECMA-376 Part 1 §18.2.6 reserves
  (`_xlnm.Print_Area`, `_xlnm.Print_Titles`, `_xlnm.Criteria`, `_xlnm._FilterDatabase`,
  `_xlnm.Extract`, `_xlnm.Consolidate_Area`, `_xlnm.Database`, `_xlnm.Sheet_Title`), matched on the
  exact token and taken from the standard's own prose rather than guessed.
- **`mjx_xlsx::Workbook`** grows the navigation surface over that model: `sheet_index_by_name`,
  `sheet_by_name`, `worksheet_by_name`, `visible_sheets`, `defined_names`, `defined_name`,
  `print_area`, `date_system`, `calculation_settings`, `window_views`, `active_sheet`,
  `rename_sheet`, and the whole-part pair `workbook_markup` / `edit_workbook_markup`. New public
  types: `DateSystem`, `CalculationSettings`, `WorkbookWindow`, `DefinedNameEntry`,
  `DefinedNameScope`; new error variant `XlsxError::NoSuchSheet`.
- **`tests/fixtures/workbook_sheet_order.xlsx`** — three sheets whose list order, `@sheetId` order
  and relationship order **all disagree**, one `hidden` and one `veryHidden` tab, a global defined
  name, a sheet-scoped one, a `_xlnm.Print_Area` and a `@localSheetId` that names no sheet. A
  workbook whose orderings agree cannot tell a correct resolver from three wrong ones. Schema-valid,
  so it needs no tolerance entry.

### Changed

- `mjx_xlsx`'s sheet-list reader no longer walks the workbook tree by hand: it reads through
  `mjx_sml::WorkbookPart` and then resolves each entry's `r:id` against the package. That is the
  whole `mjx-sml` / `mjx-xlsx` seam in one function — the markup layer never names a part, and an
  embedded workbook inside a `.pptx` needs the first half without the second.
- `mjx-derive` gains its first user in `mjx-sml`, as that crate's manifest predicted it would.

### Fixed

- **A defined name's character-data spelling now survives an edit elsewhere in the part.**
  `mjx-derive`'s `#[xml(text)]` grammar decodes on read and re-escapes **minimally** on write, so
  `&apos;Sheet&apos;!$A$1` came back as `'Sheet'!$A$1` — the same string, different bytes — and a
  rebuilt text node that differs from the original denies its element, and every ancestor of it, the
  verbatim source range subtree copy-on-write would otherwise give it. Nothing notices while a part
  is untouched, because then the model never writes at all; renaming a *sheet* was enough to make it
  visible. `CT_DefinedName` therefore has a hand-written `FromXml`/`ToXml` pair that keeps the
  original children until `set_definition` replaces them. **The same property still holds for every
  other `#[xml(text)]` leaf in the workspace** (`a:t`, `w:t`, …); fixing it in the derive is a
  foundation change no ticket owns yet, and it is recorded here so that the next child to trip over
  it does not re-derive it.

## [0.0.106] - 2026-09-05

The shared string table: what a `t="s"` cell's index actually means (MJXOFF-97, Phase D position 5).

### Added

- **`mjx_sml::strings`** — `xl/sharedStrings.xml` and everything reached through it. `CT_Sst`
  (`SharedStringTable`), `CT_Rst` (`StringItem`), `CT_RElt` (`RichTextRun`), `CT_PhoneticRun`
  (`PhoneticRun`), `CT_PhoneticPr` (`PhoneticProperties`), the `<is>` of a `t="inlineStr"` cell
  (`InlineString`), and `RichTextRunSpec` for authoring. `crates/mjx-sml/docs/SHARED_STRINGS.md` is
  the decision record: the measurements, the alternatives that lost, and the two lifetime policies in
  full.
- **`mjx_sml::font`** — `FontProperties`, `FontPropertyOwner` and `Color`. `CT_RPrElt` (a run's
  `rPr`) and `CT_Font` (a `styles.xml` font-table entry) are the same fifteen slots over the same
  eight `val`-wrapper complex types, differing only in `rFont` vs `name` and in `family`'s declared
  type, so they are one Rust type with a two-valued owner. **MJXOFF-105 (D08) reuses this module
  rather than copying it**; a copy would arrive with no executioner, which is the debt MJXOFF-99
  exists to discharge for `mjx-chart`'s duplicate SpreadsheetML writer.
- **`tests/fixtures/shared_strings_rich_text.xlsx`** — authored to disagree with the naive answer:
  `count="9"`, `uniqueCount="6"` and **seven** entries, an entry nothing references, an
  `xml:space="preserve"` entry, three rich-text runs in two `rPr` shapes, an East Asian entry with
  `rPh` and `phoneticPr`, an empty `<t/>`, a duplicate entry, two `t="inlineStr"` cells and a `t="s"`
  cell whose index points past the end of the table.
- **`crates/mjx-sml/tests/shared_strings_fidelity.rs`** (31 cases) and
  **`crates/mjx-sml/tests/shared_string_allocation.rs`** — the fidelity contract, and a second
  `harness = false` memory gate. A global allocator is process-wide, so a second measurement inside
  MJXOFF-95's binary would have started from whatever that one left live.

### Changed

- **`crates/mjx-sml/src/arena/`** — the byte arena, the checked start-tag split and the attribute-run
  scanner move out of `cells/` to sit below both packed stores. `PLAN.md` names two bulk-data cases,
  not one; two copies of `decompose` would have been two copies of the invariant that a source range
  is a *claim about somebody else's buffer* and has to be re-checked before it is believed. Gains
  `span_over` (an authored range lives in the second half of the address space) and
  `attribute_run_of` (an element whose prefix the caller does not know).

### Notes

- **48.0 bytes per entry, against 660 for a `RawElement` tree of the same table**, and zero bytes
  authored by a table nobody has edited. The bound is not the gate, and this is worth stating: against
  twelve-character strings a `Vec<String>` costs 24 bytes of header plus the text — **less** than a
  48-byte record — so a bytes-per-entry bound would have passed the design this one rejects. The
  load-bearing assertion is that the table retains the same bytes *to the byte* for entries whose
  text is ten times longer, which an entry holding a span has and an entry owning its text cannot
  have at any string length.
- **`count` and `uniqueCount` are hints, and only one of them is knowable here.** `uniqueCount` is
  the entry count, which the table is; `count` is the number of `t="s"` cells in the workbook, which
  it cannot see. Both round-trip as read. Only a change to the entry list moves `uniqueCount`, and
  only if the file wrote the attribute at all; nothing ever derives `count`, and
  `set_reference_count` is the only thing that writes it.
- **Nothing is ever renumbered.** An index is written into cells in every sheet, so removing an entry
  rewrites the meaning of every later one. Entries are append-only and an unreferenced entry stays;
  `compact` is an explicit call that returns the old-to-new map the caller must then apply to every
  sheet itself. The consequence, stated rather than discovered later: a workbook edited many times
  accumulates dead entries, which is the cheaper of the two wrong answers.
- **`xml:space="preserve"` is written only where its absence would change the value.** `sml.xsd`
  types a `t` as the simple type `ST_Xstring`, which can carry no attribute, so the attribute both
  Excel and LibreOffice write does not validate — and without it a consumer may collapse leading and
  trailing whitespace and `"  total  "` becomes `"total"`. Losing the string is worse; confining the
  divergence to strings that need it keeps an ordinary authored table schema-valid and byte-identical
  to `mjx-chart`'s writer.
- **`CT_Color` is not `mjx_dml::Color`, and MJXOFF-97's ticket was wrong to say it could be.**
  DrawingML's colour is a choice of six *elements* whose name is the kind; SpreadsheetML's is one
  element with five *attributes*, and `indexed`, `theme` (a position, not a token) and `tint` have
  nowhere to go in the other. There is still exactly one spreadsheet colour type, shared with
  everything D08 colours.
- **`CT_RPrElt` is an `xsd:choice`.** The generated `child_order` table says so — every slot at rank
  zero — so nothing here imposes an order on a run's properties. The fixture writes them in a
  non-canonical order on purpose, and it round-trips.

## [0.0.105] - 2026-09-05

The cell store: `PLAN.md`'s hybrid memory model stops being theoretical (MJXOFF-95, Phase D
position 4).

### Added

- **`mjx_sml::cells`** — `CT_SheetData`, `CT_Row` and `CT_Cell`, held as three flat arrays over one
  byte arena rather than as a tree. `SheetData` (read, edit, write), the `Row` and `Cell` views,
  `CellValue`, `PayloadShape` and `SheetDataAnomaly`. `crates/mjx-sml/docs/CELL_STORE.md` is the
  decision record: every alternative that was costed, what each would have cost, and the machine the
  numbers came from.
- **`mjx_sml::SmlError::SheetDataTooLarge`** and **`::UnrepresentableNumber`** — the fifth and sixth
  variants: a worksheet whose bytes outgrow the store's `u32` address space, and a `NaN` or infinity
  asked of `CellValue::Number`. SpreadsheetML has no numeric spelling for the latter — Rust's `inf`
  is not `xsd:double`, `xsd:double`'s `INF` does not parse back, and Excel writes an error cell — so
  the store refuses and the message names `CellValue::Error("#NUM!")` as the answer. The enum stays
  deliberately exhaustive, so MJXOFF-137's facade mapping cannot silently file either under a
  wildcard.
- **`mjx_xml::fidelity::serialize_element` / `serialize_node`** — serialize one element or node
  against an interner and an optional source buffer, without a `RawDocument`. A model that holds
  rows rather than a tree has both and no document to put them in; the alternative was a second
  serializer in a crate that must not have one.
- **`crates/mjx-allocation-counter`** — the counting global allocator MJXOFF-146 wrote inside
  `xtask/src/fuzz/`, moved so that a `mjx-sml` test binary can install it too. Nothing may depend on
  `xtask`, and a second `unsafe impl GlobalAlloc` is the last thing a workspace with
  `unsafe_code = "deny"` should have. Dependency-free and outside the shipped graph, like
  `mjx-fixtures`; `CLAUDE.md`'s "two test-only crates" and "three places allow unsafe" both become
  three, and `xtask/tests/layering.rs` grows the tier that keeps the claim checked.
- **`tests/fixtures/row_spans_and_extensions.xlsx`** — a real package carrying `row@spans` on two
  rows and none on a third, plus a `c/extLst` of foreign markup. MJXOFF-93 could only assert
  `ST_CellSpans` against authored markup, `sample.xlsx` being LibreOffice-authored and carrying none.
- **`crates/mjx-sml/tests/cell_store_allocation.rs`** — the memory gate: one target, one `main`, one
  thread, `harness = false`, and a hard byte bound measured by the allocation counter.
- **`crates/mjx-sml/tests/cell_store_fidelity.rs`** — the round-trip, edit-isolation, unknown-bucket,
  `spans` and untrusted-input cases.

### Changed

- **`xtask`'s SpreadsheetML corpus writes `row@spans`**, as Excel does. The hint is advisory and
  changes nothing about the file's meaning; the worksheet part moves from 8,955,423 to 9,020,423
  bytes and the package from 1,233 to 1,235 KiB, and `docs/BENCHMARKS.md` says so where the figures
  are. Element and cell counts are unchanged.
- **`cargo run --release -p xtask -- corpus --mem xlsx` gained a fifth checkpoint** — what holding
  the corpus worksheet costs as a packed store rather than a tree — taken with the tree still alive,
  so the reading is the honest cumulative one.

### Notes

- **36.8 bytes per cell, against the 913 MJXOFF-147 measured for a `RawElement` tree of the same
  worksheet.** That benchmark also said where the 913 comes from — not the 72-byte element struct but
  the two small heap allocations every element carries — so this store has no per-cell allocation at
  all: 36 bytes a cell, 48 a row, 40 for the rare cell that carries something unusual, and, for a
  worksheet nobody has edited, not one byte of its own, because every value it preserves is a range
  into the part's buffer, shared with the package rather than copied. A sheet whose only populated
  cell is `XFD1048576` allocates 368 bytes and holds one row record.
- **Holding is 25x cheaper; opening is unchanged.** The store is built from a `RawElement` tree, so
  the +274 MiB first materialisation MJXOFF-147 recorded is still paid on open. Building it straight
  from the part's bytes would need a streaming reader in `mjx-xml`, `quick-xml` being allowed behind
  that crate and nowhere else, and is not in this child's scope.
- **The unknown bucket in a packed store.** `CLAUDE.md` states the rule as `extra: Vec<RawNode>`; a
  `Vec<RawNode>` per cell is precisely the allocation the 913 is made of. The same rule is kept in
  raw bytes, which is the stricter of the two — it preserves the whitespace inside a start tag, which
  a decomposed attribute list does not record. A cell's start tag keeps the file's bytes unless
  regenerating it from `r`, `s` and `t` would reproduce them, decided by doing the regeneration and
  comparing; editing such a cell rewrites the run in place, so an unmodelled attribute survives the
  edit as well as the row.
- **One thing a file can say is refused**: a `c@r` that is not a cell reference, because the store is
  keyed on it. Rows out of order, duplicated row numbers, a `c@r` naming a different row than its
  `row@r`, cells out of column order and a `t` that disagrees with the child element present are all
  preserved as read and described by `SheetData::anomalies`, never repaired.

## [0.0.104] - 2026-09-05

Cell addressing: the SpreadsheetML reference vocabulary eleven later Phase D children consume
(MJXOFF-93, Phase D position 3).

### Added

- **`mjx_sml::address`** — `crates/mjx-sml/src/address.rs` was a module slot with a note naming this
  child; it is now the workspace's one model of a cell address. `CellReference` (`ST_CellRef`),
  `CellRange` (`ST_Ref`) in all four forms (`A1`, `A1:C3`, `A:A`, `1:1`), `CellRangeList`
  (`ST_Sqref`), `CellSpans`/`CellSpan` (`ST_CellSpans`), `SheetQualifiedReference`/`SheetName`,
  `R1C1Reference`/`R1C1Range`/`R1C1Coordinate`, the bijective base-26 conversion both ways
  (`column_letters`, `column_index_from_letters`), `Anchoring`, `ColumnBound`, `RowBound`,
  `GridBounds`, `AddressText` and `AddressError`. `ReferenceMode` (`calcPr@refMode`) is re-exported
  from the generated `sml` simple types rather than declared a second time.
- **`mjx_sml::SmlError::Address`** — the fourth variant, carrying `AddressError`. The enum stays
  deliberately exhaustive, so MJXOFF-137's facade mapping cannot silently file it under a wildcard.
- **`crates/mjx-sml/tests/fixture_addressing.rs`** — every address in `tests/fixtures/sample.xlsx`
  parses and re-emits byte-identically, asserted against a **pinned** list of the thirteen the
  worksheet carries rather than against a count, so a scan that stops finding anything fails rather
  than passing vacuously. `sample.xlsx` carries no `row@spans`, so the other half of the `spans`
  rule — never drop one that was written — is asserted against authored `x:`-prefixed markup in the
  same suite.

### Notes

- **Eight bytes, `Copy`, no allocation on the parse path.** MJXOFF-95 (D04) will parse a reference
  for every cell of a sheet that may hold 1,048,576 x 16,384 of them, so `CellReference` is a `u32`
  row, a `u16` column and two one-byte anchorings with no padding waste; parsing is one forward pass
  over `&str` with no intermediate `String`, and formatting writes into `AddressText`, a `Copy`
  48-byte stack buffer. `Copy` is the proof rather than the decoration — a `Copy` type cannot own a
  heap allocation.
- **Round-trip is exact, not canonical.** `$A$1` writes `$A$1`, `A1` never widens to `A1:A1`,
  `C3:A1` stays backwards, `RC` never becomes `R[0]C[0]`, and a `sqref`'s separator run survives
  until the list is edited. The ordered view is a separate answer (`CellRange::normalized_bounds`),
  because a reference-formatting "improvement" is an edit-isolation failure.
- **Out-of-grid input is refused, never clamped.** `XFE` is an error, not `XFD`; a row number is
  accumulated with saturating arithmetic, so an absurd digit run is reported rather than wrapped.
- **Column letters are not case-folded.** `"a1"` is a typed error rather than a silent rewrite to
  `A1`. Every producer this workspace has read writes uppercase, and folding would canonicalize a
  file that said otherwise.
- **`mjx_chart::workbook::column_letters` is superseded but not yet retired.** It returns a `String`
  per call and has no parser; MJXOFF-112 (D10) switches `mjx-chart` over to `column_letters` here and
  MJXOFF-99 (E1) retires the copy. A second, independent copy lives in `xtask/src/corpus/xlsx.rs`.

## [0.0.103] - 2026-09-05

The `mjx-xlsx` package spine: the part graph, a workbook that opens and saves without touching a
byte, and the SpreadsheetML invariants a save is held to (MJXOFF-91, Phase D position 2).

### Added

- **`mjx_xlsx::Workbook`** — the format-tier half of the Excel split MJXOFF-132 opened.
  `crates/mjx-xlsx/src/lib.rs` was thirteen lines with zero public items and an
  `assert_eq!(2 + 2, 4)` placeholder; it is now a crate with a module tree, a resolved part graph and
  a byte-exact round trip. `open`/`from_package` find the workbook part through the package-root
  `officeDocument` relationship and identify it by its **root element** (never by its content type —
  which is what lets a macro-enabled workbook open at all, see below); `sheets` reads the `x:sheets`
  list, in document order, resolving each entry's `r:id`; `parts` and `worksheet(i).parts()` resolve
  the workbook-level and sheet-level part graphs; `part_inventory` says what this crate made of every
  part; `validate`/`save`/`save_unchecked` are the write path.
- **`mjx_xlsx::parts`** — the SpreadsheetML part graph: twenty-one `PartKind`s, each pairing a
  relationship type with its content type(s), plus `SheetKind`, `WorkbookParts` and `WorksheetParts`.
  Every string is quoted from ECMA-376 Part 1 §12.3 (the theme from §14.2.7, the VML drawing from
  **Part 4 §8.2**), with the Strict `purl.oclc.org` prefix substituted for the Transitional one every
  fixture in this workspace actually carries — the same convention `mjx_docx::constants` documents.
  The four a workbook cannot open without match `mjx-chart`'s own embedded-workbook writer string for
  string, which is what MJXOFF-112 will delete.
- **`mjx_xlsx::preserve`** — the tier-1 contract written down, and `PartClassification`. A part this
  crate cannot classify is **preserved, never rejected**: a `.xlsm`'s macro-enabled workbook, a custom
  XML mapping and a vendor's private sidecar all round-trip through a crate that knows nothing about
  any of them.
- **`mjx_xlsx::SpreadsheetDefect`** — five SpreadsheetML invariants on top of `mjx-opc`'s packaging
  ones, split by scope exactly as `mjx_pptx::validate` splits its own. Two are **graph** invariants,
  checked over the whole package: the package-root `officeDocument` relationship must still name the
  workbook part (§12.3.23), and no `…spreadsheetml.*` part may be unreachable from the root. That
  second one deliberately narrows `mjx-opc`'s "an unreferenced part is not a defect" for one family
  and one reason — a shared string table nothing relates to is not dead weight, it is every `t="s"`
  cell in the workbook indexing into a table no consumer loads. The other three are **markup**
  invariants over `Package::authored_xml_parts` only, so a workbook opened and saved untouched is
  never faulted for markup it arrived with: a `x:sheet` entry must lead to a sheet, a related sheet
  part must be listed, and `@sheetId`/`@name`/`r:id` must each be unique (§18.2.19).
- **`crates/mjx-xlsx/tests/roundtrip.rs`, `part_graph.rs`, `workbook_validation.rs`** — the tier-1
  proof over the directory-derived `.xlsx` corpus (part by part on decompressed payloads, never a
  container hash), the part-graph and preservation clauses, and the refusals proved on the real
  fixture through the public `Workbook::save`.
- **Three cases added to `crates/mjx-xlsx/tests/schema_gate.rs`** (MJXOFF-110 created the file,
  MJXOFF-132 added the ordering case): invalid worksheet markup — a `s:c` planted outside its
  `s:row`, which `CT_SheetData` rejects — must fail *naming `/xl/worksheets/sheet1.xml`*, which is
  the part every later Phase D child writes into; **no** part under `xl/` may be skipped rather than
  validated, which is the general form of the four-part clause the file already pinned by name; and a
  workbook opened and saved through `Workbook` (not through `Package`) is still schema-valid and
  still in child order.
- **A two-page guide** at `crates/mjx-xlsx/docs/guide/`, every snippet a compiled doctest.

### Notes

- **The macro-enabled content types are deliberately not declared.** `macroEnabled` appears nowhere
  in ECMA-376 Parts 1-4, so declaring one would be guessing a wire token. A `.xlsm` still opens (the
  workbook part is found by its root element) and still round-trips; its workbook part simply reports
  as unclassified. `parts.rs` carries a test that fails if a later child ever adds the string, so the
  note cannot go stale silently.
- Patch digit only. No existing public identifier changed name or shape: every item here is new, in a
  crate that previously had none.

## [0.0.102] - 2026-09-05

`mjx-sml`, the shared-markup layer Excel was missing, and the first mechanical check of the layering
rule (MJXOFF-132, Phase D position 1) — the first child of Phase D.

### Added

- **`crates/mjx-sml`**, a new workspace member at rank **2.1**: beside `mjx-dml`, beneath
  `mjx-chart`. It holds the SpreadsheetML *markup* — cells, rows, sheet data, shared strings, styles,
  number formats, formulas as text — while `mjx-xlsx` keeps the `Workbook` surface and the package
  graph in the format tier. **Excel is two crates, not one**, because an authored chart embeds a
  whole `.xlsx` inside a `.pptx` or a `.docx`: SpreadsheetML is shared markup even though the package
  is Excel's. That is what makes `mjx-chart → mjx-sml → mjx-dml` a chain of downward edges, and it is
  what lets MJXOFF-112 and MJXOFF-99 finally delete `mjx-chart`'s duplicate workbook writer — with
  one Excel crate the retirement would have needed `mjx-chart → mjx-xlsx`, which points **up**.
  This child **emits no markup and models nothing**: it is the crate, the module tree (each module a
  named home carrying the work item that fills it), `SmlError`, the generated `sml` ordering table and
  the layering test.
- **`xtask/tests/layering.rs`** — the layering rule, checked. `CLAUDE.md`'s downward-only rule was
  the one architectural rule in the repository with no mechanical check, and two queued children are
  specified to rely on it existing. The test reads the real graph out of `cargo metadata --no-deps`
  (rather than scanning manifests, where an unrecognised dependency spelling would drop an edge
  silently) and fails on any normal or build edge that does not point to a **strictly lower** rank,
  naming both crates and both ranks. Sideways is as illegal as upward. It also refuses to pass on
  fewer edges than the shipped graph has, so it cannot go green by reaching nothing.
- **The `sml` child-order table.** `xtask`'s `CHILD_ORDER_SCHEMAS` gains `"sml"`, so
  `mjx_ooxml_types::child_order` now carries the `xsd:sequence` position of every child of all 367
  SpreadsheetML complex types — `CT_Worksheet`'s **39 slots**, the largest sequence in the workspace,
  among them. Five curated exports name the types the Phase D children place children into first:
  `WORKSHEET`, `WORKBOOK`, `STYLESHEET`, `WORKSHEET_ROW` and `WORKSHEET_CELL`. `sml.xsd` reaches
  `dml-spreadsheetDrawing` through `CT_ObjectAnchor`, so that schema joins
  `CHILD_ORDER_SCHEMA_DEPENDENCIES` (parsed only; its own table stays MJXOFF-107's).

### Changed

- **Every `x:`-rooted part is now audited for child order.** `mjx-schema-gate`'s `sml` entry moves
  from `OrderingCoverage::Pending { owner: "MJXOFF-132", … }` to `Generated`, which is what
  `parts_that_must_be_audited` reads. Before this, the ordering gate on a `.xlsx` recognised only
  `/xl/theme/theme1.xml` — a DrawingML part that happens to live in a workbook — and passed. A new
  case in `mjx-xlsx` pins the four SpreadsheetML parts of `sample.xlsx` as required *and* audited
  non-vacuously, from both ends of the mechanism.
- **`CLAUDE.md`, `README.md` and `PLAN.md`** state the rank table, including the sub-ranks. Two of
  them were not previously written down: shared markup is not flat (2.0 `mjx-dml` → 2.1 `mjx-sml` →
  2.2 `mjx-chart`/`mjx-omml`/`mjx-vml`) and neither are the foundations (`mjx-xml` is built on
  `mjx-ooxml-core`, so 0.0 → 0.1). A flat foundations tier would have made a shipped edge illegal.

### Removed

- Two dead rows from `xtask`'s `UNCOVERED_SCHEMAS`: `sml` (this child covers it) and `shared-math`
  (MJXOFF-134 covered it and left its row behind). A row exists for a schema a `COVERAGE.md` table
  does *not* cover; a row for one that both tables cover is read by nothing. Regenerating with the
  rows gone produces a byte-identical `COVERAGE.md`, which is the proof they were dead.

## [0.0.101] - 2026-09-05

The Word usage guide, its examples, and the `wml` preserve-only ledger (MJXOFF-150, Phase C position
22) — the last child of Phase C.

### Added

- **Four Word guide pages**, so `crates/mjx-docx/docs/guide/` is now the five-pages-plus-a-README
  shape `crates/mjx-pptx/docs/guide/` settled on, in the same reading order:
  [`text_and_formatting`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/text_and_formatting/)
  (addressing a run, positions shifting under an insert, equations, run-level content that is not
  text, comments/notes/bookmarks, tracked changes read but never applied),
  [`tables_sections_and_headers`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/tables_sections_and_headers/)
  (both kinds of merge, the grid-discrepancy report, sections ending rather than starting at a
  `w:sectPr`, header inheritance, fields, structured content), [`styles_and_inheritance`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/styles_and_inheritance/)
  (the six-rung ladder, the `w:basedOn` chain and its typed cycle error, the toggle-property XOR
  rule, numbering, the table-cell rung) and [`fidelity_and_gaps`](https://docs.rs/mjx-docx/latest/mjx_docx/guide/fidelity_and_gaps/).
  `crates/mjx-docx/src/guide.rs` grows from two `include_str!` to six.
  **Every snippet is a compiled doctest that asserts on a value it computed** — 38 doctests in
  `mjx-docx`, up from 24 — and every one of the new ones runs against a real `Document::blank`
  rather than being `no_run`, so an assertion that stopped holding would go red rather than merely
  still compiling.
- **`crates/mjx-docx/docs/guide/fidelity_and_gaps.md`**, the artefact MJXOFF-128 (F2) named and no
  unit produced. Same structure as PowerPoint's: the round-trip guarantee, what `save` refuses,
  the content that is not WordprocessingML, and four lists under *The gaps* — **Non-goals** (twelve
  rows, each with the reason it is a decision), **Built, not yet verified against Office** (eight
  rows, F2's input; nothing here is marked verified, and no agent may mark it so), **Whole formats**
  and **What used to be here**. It also names, rather than leaves implied, that `mjx-docx` has no
  `DocumentDefect` counterpart to `mjx-pptx`'s `PresentationDefect`: Word's `save` runs the package
  graph check alone, and the WordprocessingML-level invariants are enforced at the point of each edit
  instead.
- **The `wml` preserve-only ledger**, in that page: the one complex type of 285 with no Rust type
  (`CT_ShapeDefaults`, whose whole content model is `xsd:any` in the VML office namespace), the
  elements typed as `Unmodeled` and why each is (most are `CT_Empty`, where "unmodelled" is the
  complete truth), the eight clusters whose *reference* is typed and whose *payload* is preserved
  (`w:altChunk` payloads, custom XML data, printer settings, embedded fonts, OLE streams, `w:subDoc`,
  charts and SmartArt inside a `w:drawing`, VML beyond `mjx-vml`'s coverage), and the one
  deliberately unmodelled recursion (`w:divsChild`).
- **Eight new `mjx-docx` examples**, taking the crate from one to nine and matching `mjx-pptx`'s
  eight: `read_document`, `edit_text`, `build_table`, `styles_and_numbering`, `sections_and_headers`,
  `fields_and_hyperlinks`, `annotations` and `structured_content`. Each takes a CLI argument with a
  `target/examples/` default, prints section banners, and **reopens what it wrote and asserts on
  it** — `read_document` compares every part's decompressed payload against the original to prove
  that reading dirties nothing, and `edit_text` proves the converse for four parts nothing addressed.
- **`crates/mjx-docx/examples/support/mod.rs`**, `mjx-pptx`'s four helpers (`fixture_dir`, `fixture`,
  `template`, `output_path`) by the same names and signatures, with `fixture_dir` delegating to
  `mjx_fixtures::fixtures_dir()` rather than recomputing the path — two spellings of one directory is
  the drift `mjx-fixtures` exists to end. `blank_document.rs` moves onto it, losing its own private
  copy of `output_path`.

### Changed

- `mjx-ooxml`'s crate-level *Guides* section lists all five Word pages by name, as it already did for
  PowerPoint's five, instead of naming only `building_a_document`.

## [0.0.100] - 2026-09-05

The Word facade, the error mapping and both bindings (MJXOFF-139, Phase C position 21) — closes
Phase C. `mjx_ooxml::Document` (`crates/mjx-ooxml/src/document.rs` + `document/`, 14 files) is the
curated Word surface over `mjx_docx::Document`, mirroring [`Deck`]'s own treatment of
`mjx_pptx::Presentation`: [`BlockPath`]/[`RunPath`] (`u32`-addressed) replace `impl Into<BlockPath>`/
`impl Into<RunPath>`, a concrete return type replaces every closure `mjx_docx::Document` takes to
read or edit a part, and `DocxError`'s 35 variants collapse into the existing eleven `ErrorCode`s —
none needed a twelfth.

**The curated surface**: lifecycle (`open`/`blank`/`save`/`save_unchecked`/`validate`/`conformance`),
paragraph and run reading/editing, effective properties (run, paragraph, table cell — a documented
subset of the full ladder, not every field), read-only style lookup, numbering attach/detach,
sections (page size/margins) and headers/footers, tables (dimensions, spans, merges, structural
insert/remove), fields, hyperlinks, comments, footnotes/endnotes, revisions (read-only), and inline
pictures. Mail merge, web settings, the font table, recipients, bookmarks, move ranges, custom-XML
data binding, `altChunk`, the glossary document, legacy form fields and equations stay reachable
through [`Document::document_mut`] — documented on [`Document`]'s own module doc, including why
content controls need no dedicated method (MJXOFF-138 already made paragraph/run addressing recurse
through one transparently, so nothing new was needed to reach a content control's own text).

**Both bindings** (`bindings/mjx-python`, `bindings/mjx-wasm`) project the same curated surface:
`mjx_ooxml.Document` in Python (identity-mapped, reusing the existing eleven exception classes) and
`Document` in the wasm package (camelCase, `free()`-mandatory, reusing `CellExtent`/`CellAddress`
for the `(rows, columns)`/`(row, column)` pairs `Deck` already projects the same way). New value
classes and enumerations extend the existing `value_class!`/`sealed_enums!`/`open_enums!` machinery
in both bindings rather than inventing a second pattern. The Python `.pyi` stub carries every new
class and method; `bindings/mjx-wasm/tests/node/document_surface.mjs` and (unverifiable locally —
`maturin`/`pytest` are absent) `bindings/mjx-python/tests/test_document_surface_coverage.py` are the
mis-wiring guards, the Word siblings of `surface.mjs`/`test_surface_coverage.py`.

**One walkthrough, three languages** — `crates/mjx-ooxml/examples/build_a_document.rs`,
`bindings/mjx-python/tests/test_build_a_document.py`,
`bindings/mjx-wasm/tests/node/build_a_document.mjs` — a document authored from `Document::blank`
through the curated surface only (paragraphs and runs, a numbered list, a hyperlink, a table, a
header, a comment, a footnote), saved and reopened.

**`Deck::open`'s Word refusal now names `Document::open`**, and `Format::is_editable` covers
WordprocessingML alongside PresentationML — Excel remains detected but not editable.

**Fixed while writing the Rust walkthrough, in `mjx-docx` (MJXOFF-124's own code, not this child's
facade)**: `Document::add_footnote`/`add_endnote` silently lost every entry — the two reserved
separator entries included — on save + reopen, whenever called on a document with no existing
`footnotes.xml`/`endnotes.xml`. `create_footnotes_part`/`create_endnotes_part` wrote a literal
`<w:footnotes xmlns:w="...">` template, then wrote back a *fresh* `Footnotes::blank(interner)` —
built with `attributes: Vec::new()` — over the just-parsed root to seed the two reserved entries,
discarding the `xmlns:w` the parse had just preserved; the saved bytes were schema-shaped XML with
every `w:footnote` child under a `w:footnotes` root that never declared its own `w:` prefix, so a
reopen resolved no `w:` element at all. `Footnotes::seed_reserved_entries`/
`Endnotes::seed_reserved_entries` push the reserved entries onto the already-parsed value instead of
replacing it — the same safe shape `create_comments_part`/`create_header_footer` already used.
Reproduced directly against `mjx-docx`'s own public API, independent of the facade; two regression
tests added, mutation-proved.

The `wml` ownership audit (MJXOFF-139's other deliverable, per MJXOFF-133's template) is in this
child's own pull request description: every member of `CT_Body`'s content groups
(`EG_BlockLevelElts`/`EG_BlockLevelChunkElts`/`EG_ContentBlockContent`/`EG_RunLevelElts`/
`EG_PContent`/`EG_ContentRunContent`/`EG_RPrBase`) and all fourteen of `wml.xsd`'s global elements,
each mapped to its owning MJXOFF id — nothing unowned.

## [0.0.99] - 2026-09-05

Content controls, custom XML, smart tags, `w:dir`/`w:bdo`, `w:altChunk` and the glossary document's
building blocks (MJXOFF-138, Phase C position 20) — `crates/mjx-docx/src/document/structured_content.rs`
(new). MJXOFF-69–74 named no owner for this cluster either; the other half of the third of `wml.xsd`
Phase C's own six specifications never allotted.

**`w:sdt` and `w:customXml` are both members of all four content groups** (`EG_ContentBlockContent`,
`EG_ContentRunContent`, `EG_ContentRowContent`, `EG_ContentCellContent`), so a content control or a
custom-XML wrapper can appear anywhere a paragraph, a run, a table row or a table cell can.
[`ContentControlBlock`]/[`ContentControlRun`]/[`ContentControlRow`]/[`ContentControlCell`] and
[`CustomXmlBlock`]/[`CustomXmlRun`]/[`CustomXmlRow`]/[`CustomXmlCell`] type every placement — and
each one's own content **reuses** the exact enum ([`BlockContent`], [`ParagraphContent`],
[`TableContent`], [`RowContent`]) its placement already has a container for, so MJXOFF-92's
paragraph/run APIs and MJXOFF-116's row/cell addressing reach through a wrapper unchanged.
[`Table::rows`]/[`Row::cells`] now recurse into a row- or cell-level wrapper, so `(row, column)`
addressing stays correct when a repeating-section control wraps one or more rows;
[`Paragraph::text`]/[`Paragraph::run`] now reach through a run-level wrapper the same way they
already reach through `w:hyperlink`.

**`w:sdtPr`** — [`ContentControlProperties`]: the twelve control kinds ([`ContentControlKind`]:
rich text, plain text, picture, combo box, drop-down, date, building-block gallery/list, group,
citation, bibliography, equation), the lock ([`Lock`]), the placeholder ([`Placeholder`]) and the
XML data binding ([`DataBinding`]) all get typed accessors, plus `set_lock`/`set_placeholder`/
`set_data_binding` writers that insert at `CT_SdtPr`'s own schema rank regardless of call order.
`w14:`/`w15:` extensions (checkbox, repeating section) round-trip through the unknown-element bucket
— dropping them would turn a working form into inert text.

**A content control's data binding is a two-part reference** —
[`Document::resolve_data_binding`] enumerates every related Custom XML Data Storage part
(`customXml/itemN.xml`), matches `storeItemID` against each one's own properties part
(`customXml/itemPropsN.xml`'s `ds:itemID`), and resolves `xpath` (the absolute, `[n]`-indexed
element path Word itself emits) against the matching part's own tree — [`resolve_xpath`]. A binding
naming a part the package does not carry reports [`DocxError::DataBindingPartNotFound`], never a
panic.

**`w:altChunk`** — [`AltChunk`]/[`AltChunkProperties`]: an embedded HTML/RTF/`.docx` part this crate
imports the relationship for and never converts. [`Document::add_alt_chunk`]/
[`Document::alt_chunk_payload`]/[`Document::alt_chunk_parts`] round-trip the payload, the relationship
and the content type byte-identically.

**The glossary document** — [`Document::glossary_document`] reads `word/glossary/document.xml`'s own
[`DocParts`]/[`BuildingBlock`] list; a building block's own content is an ordinary [`Body`]
([`BuildingBlock::body`]) — the exact same block-content API the main document body uses, no
glossary-specific duplicate.

**The adversarial fixture** `structured_content.docx` nests a run-level control inside a paragraph
inside a cell-level control inside a table inside a block-level control, with a `w:customXml`
row wrapper and a repeating-section row-level control wrapping two `w:tr` interleaved alongside it —
proved, with a mutation to each of the two recursion sites (row-level `w:customXml`, run-level
`w:sdt`) confirmed red and reverted by hand.

`mjx-schema-gate`: two new `PreservedForeignMarkup` entries — Custom XML Data Storage Properties'
own fixed namespace, and this fixture's own representative Custom XML Data Storage namespace —
classifying both as foreign markup rather than validating them, pinned by a test.

`Body::content`/`Paragraph::content` are now `pub`, matching the precedent
`Table::content`/`Row::content`/`Cell::content` already set (MJXOFF-116).

## [0.0.98] - 2026-09-05

`word/settings.xml`, `word/webSettings.xml`, `word/fontTable.xml` and `word/recipients.xml`
(MJXOFF-136, Phase C position 19) — `crates/mjx-docx/src/document/{settings,web_settings,
font_table,mail_merge}.rs` (all new). MJXOFF-69–74 allotted Word six children and, between them,
named no owner for a third of `wml.xsd`; this child takes the document-configuration half.

**`CT_Settings` — all 98 children modelled**, none silently dropped: [`DocumentSettings`] and
[`SettingsContent`]. Every `CT_OnOff` flag, `CT_DecimalNumber` and `CT_TwipsMeasure` leaf gets a
full, individually named get/set pair; every "own type" child (`w:view`, `w:zoom`,
`w:documentProtection`, `w:compat`, `w:docVars`, `w:rsids`, `w:mailMerge`, `w:captions`, …) gets a
full get/set pair over its own richly typed value. `w:compat`'s sixty-two individual flags are the
one deliberate exception: each is a bare `CT_OnOff` nothing in Phase C names by name, so they round
-trip exactly through the unknown-element bucket rather than each getting a near-identical bespoke
method; `w:compatSetting` (the schema's own generic escape hatch) is fully typed.
`sl:schemaLibrary` (a foreign Smart Tag schema namespace) is the only schema-known child left
read-only, preserved via the same bucket. `m:mathPr` wires in `mjx-omml`'s `MathProperties`
(MJXOFF-134's own module doc named this exact seam). `Document::even_and_odd_headers` now reads
through `DocumentSettings` instead of MJXOFF-113's ad-hoc raw-tree scan.

**`word/webSettings.xml`** — [`WebSettings`]: legacy framesets (`CT_Frameset`, recursively boxed)
and the `w:div`/`w:divs` tree `w:divId` (`CT_PPrBase`, C4) points into. `w:divsChild` (`CT_Div`'s own
recursive nesting) is deliberately unmodelled — see that module's own doc comment.

**`word/fontTable.xml`** — [`FontTable`]/[`Font`]: identity, PANOSE classification, character set,
family, pitch, signature, and the four embedded-font relationships. An embedded font's binary
payload and its `fontKey` obfuscation key are opaque — never re-encoded, never decoded.

**`word/recipients.xml`** — [`Recipients`]/[`RecipientData`], the Mail Merge Recipient Data part;
`DocumentParts::recipients` (C1 declared `PartKind::Recipients` but never resolved it).
`w:settings/w:mailMerge` and its ODSO (Office Data Source Object) cluster — [`MailMergeSettings`],
[`Odso`] — share the mail-merge vocabulary with this part.

**`w:documentProtection`'s password hash is preserved exactly, never recomputed, never cleared** —
typed as opaque text (the base64 wire form needs no decoding), proven by an edit to an unrelated
flag leaving the hash byte-identical.

**The adversarial fixture** `settings_document_configuration.docx` carries `w14:`/`w15:` elements
interleaved between modelled ones inside `word/settings.xml`, two `w:compatSetting` entries, a
`w:docVars` block, an embedded font, and both new parts — round-tripped byte-identically through the
typed model, with the unknown-bucket order proved to survive exactly (a mutation that drops it was
applied by hand, confirmed red, and reverted).

## [0.0.97] - 2026-09-04

Office MathML in Word: `mjx-omml` ends the Phase 0 scaffold deferral (MJXOFF-134, Phase C position
18) — `crates/mjx-omml/src/{support,leaf,arg,objects,math,properties}.rs` (all new), 2,638 lines from
13. `crates/mjx-docx/src/document/{body,mod,revisions}.rs` (Word-side integration).

**All 72 `shared-math.xsd` complex types are modelled.** `Math` (`m:oMath`), `MathParagraph`
(`m:oMathPara`), `Argument` (`CT_OMathArg`, the recursive core every object's operand slot bottoms
out at), `Run` (`m:r`), `Text` (`m:t`), and every math object — accent, bar, box, border box,
delimiter, equation array, fraction, function-apply, group character, lower/upper limit, matrix
(with its row/column/column-properties family), n-ary operator, phantom, radical, and the four
script forms — with its own paired `*Pr` properties type. Twenty leaf `CT_*` value types (`CT_OnOff`,
`CT_Shp`, `CT_Integer255`, …) collapse into one shared read/write mechanism rather than twenty
near-identical Rust types, and six single-`ctrlPr`-child `*Pr` types collapse into
`ControlOnlyProperties` — the same "one shape, many meanings" reuse `mjx-docx` already established
for `CT_OnOff`/`CT_String`. Consumes MJXOFF-144's generated `mjx-ooxml-types::officemath` simple
types throughout.

**The layering tension `CT_CtrlPr` poses — a `wml`-typed `w:rPr`/`w:ins`/`w:del` nested inside a
`shared-math` type, which the schema itself only resolves by importing `wml.xsd` — is resolved by
preserving `ControlProperties`'s children wholesale and raw**, the same mechanism `mjx-dml`'s
`WordprocessingGroup`/`WordprocessingCanvas` already use for their own WordprocessingML-typed member
content. `mjx-docx` (which depends on `mjx-omml`) adds typed accessors over a `ControlProperties`'s
raw children where it needs them: `MathControlInsert`/`MathControlDelete` (`CT_MathCtrlIns`/
`CT_MathCtrlDel`) and `math_control_properties`, the reachable call site MJXOFF-126 declined those
two types for want of.

**Word-side integration:** `ParagraphContent` grows `Math`/`MathParagraph` variants (`m:oMath`/
`m:oMathPara`, folded in from `EG_RunLevelElts`'s own `EG_MathContent` — a sibling of `w:r`, not
nested inside one), wired into all four `Vec<ParagraphContent>` hosts. `Paragraph` grows
`append_math`/`append_math_paragraph`/`equations`/`equations_mut`; `Document` grows `append_math`
(closure-based, mirroring `edit_numbering`) and `set_equation_run_text` (an edit several nesting
levels deep, through the same `ToXml::write_back` span-preserving path every other mutation in this
crate uses).

**A real bug the crate's own integration tests caught:** `shared-math.xsd` is
`attributeFormDefault="qualified"` (the only other modeled schema besides `wml.xsd` with this shape),
so every `val`/`alnAt` attribute is wire-qualified `m:val`/`m:alnAt`, never bare — fixed in
`crate::support`'s `VAL_ATTRIBUTE_PREFIX`. A freshly authored equation spliced into a blank
document's `word/document.xml` (which binds only `w:`/`r:`) also produced markup using the
undeclared `m:` prefix — `Document::append_math` now declares it on the newly inserted subtree's own
root, the same pattern `document/drawing.rs`'s `Drawing::new` already established for `wp:`/`a:`/
`pic:`.

**Codegen:** `shared-math` moves from `CHILD_ORDER_SCHEMA_DEPENDENCIES` to `CHILD_ORDER_SCHEMAS` (its
own generated child-order table) and gains 43 `CHILD_ORDER_EXPORTS` rows. No new entry was needed in
the schema gate's `schema_for_namespace`: `m:` never roots a part of its own — `wml.xsd` already
imports `shared-math.xsd`, so `word/document.xml`'s own validation against `wml.xsd` already covers
nested `m:` content transitively, proved by a mutation (`m:f` missing its required `m:num`) that
turns the sweep red naming the part.

## [0.0.96] - 2026-09-04

DrawingML in Word: `w:drawing`, `w:pict`, `w:object` and `w:control` (MJXOFF-131, Phase C position
17): `crates/mjx-dml/src/wordprocessing_drawing.rs` (new), `graphic.rs` (new), `picture.rs` (new),
`shape_properties.rs` (new), `nonvisual.rs` (new), `crates/mjx-docx/src/document/drawing.rs` (new).

**The `wp:` schema (`dml-wordprocessingDrawing.xsd`, 287 lines, 20 complexTypes) had no model
anywhere in `crates/` before this child.** Seventeen of the twenty land in `mjx-dml` —
`wp:inline`/`wp:anchor`, the five wrap modes and `wp:wrapPolygon`, `wp:graphicFrame`,
`wp:wgp`/`wp:wpc`, `wp:contentPart` — reusing `mjx-dml`'s existing `SolidFill`/`Transform2D`/
`PresetGeometry`/`CustomGeometry`/`LineSpec`/`EffectList` pieces through a new `ShapeProperties`
(`a:CT_ShapeProperties`, the type MJXOFF-107 and this ticket both found missing) and a new
`Picture`/`Graphic`/`GraphicData` (`pic:pic`, `a:graphic`/`a:graphicData`). The remaining three
(`CT_WordprocessingShape`, `CT_TextboxInfo`, `CT_TxbxContent`) live in `mjx-docx` instead: their
content is `w:EG_BlockLevelElts`, WordprocessingML's own vocabulary, so typing them below `mjx-docx`
would reach `mjx-dml` upward past its own tier. `TextBoxContent` reuses `body.rs`'s own
`BlockContent`/`block_paragraph*` mechanism as its sixth container (MJXOFF-126's "extend, don't
copy"), so a text box's paragraphs read through the same model every other container does.

**`Document::add_inline_picture`/`remove_drawing`** add the image-part/relationship/content-type
plumbing a picture needs (and sweep the media part on removal via `Package::
remove_unreferenced_parts`, so a removed picture leaves no orphan); **`Document::
paragraph_run_content`** is the new public reading surface for a paragraph's own `w:drawing`/
`w:pict`/`w:object`/`w:control` content. `w:pict` needed no new wrapper at all: it reads and writes
directly as `mjx_vml::Drawing`, the same type MJXOFF-113 already uses for a header's own `w:pict`.

**A cross-schema element-namespace bug found while building the fixture, fixed the same session:**
`NonVisualDrawingProps`/`ShapeProperties`/`PictureFill`'s constructors defaulted their element to
DrawingML-main (`a:`), which is correct only when the host schema genuinely is `dml-main.xsd` —
`cNvPr`/`blipFill`/`spPr` inside `pic:pic` and `docPr` inside `wp:inline`/`wp:anchor` are *local*
element declarations of their own host schema (`dml-picture.xsd`, `dml-wordprocessingDrawing.xsd`)
and take that schema's own namespace on the wire (`pic:cNvPr`, `wp:docPr`), never literally `a:`.
Each type now also has a `with_name`/`new_qualified` constructor taking the host's own qualified
name, used at every pic:/wp: call site.

**Codegen:** `dml-wordprocessingDrawing` moves from `CHILD_ORDER_SCHEMA_DEPENDENCIES` to
`CHILD_ORDER_SCHEMAS` (its own generated child-order table) and gains a `SimpleTypeModule` for its
five `ST_*` enums (`WrapText`, `HorizontalAlignment`, `HorizontalRelativeFrom`, `VerticalAlignment`,
`VerticalRelativeFrom` — the last four curated overrides on the shared naming table, since the
schema's own `H`/`V` suffixes are ECMA-376's own contraction for "Horizontal"/"Vertical"). No new
entry was needed in the schema gate's `schema_for_namespace`: `wp:` never roots a part of its own —
`wml.xsd` already imports `dml-wordprocessingDrawing.xsd`, so `word/document.xml`'s own validation
against `wml.xsd` already covers nested `wp:`/`pic:` content transitively.

## [0.0.95] - 2026-09-04

Word revision marks: tracked changes as a first-class case in every mutation path (MJXOFF-126,
Phase C position 16): `crates/mjx-docx/src/document/revisions.rs` (new).

**`w:ins`/`w:del`/`w:moveFrom`/`w:moveTo` are typed as run-level containers, recursively** — an
insertion nested inside a deletion is one `ParagraphContent::Ins` inside another `Del`'s own
content, exactly the shape a real tracked-change history produces. `crates/mjx-docx/src/document/
revisions.rs` adds `RunTrackChange` (`CT_RunTrackChange`), `TrackChangeMarker` (bare `CT_TrackChange`
— `w:cellIns`/`w:cellDel`, the paragraph mark's own `w:ins`/`w:del`/`w:moveFrom`/`w:moveTo`, the four
`customXml*RangeStart` elements), `MoveBookmark` (`CT_MoveBookmark`), `CellMergeTrackChange`,
`TrackChangeNumbering`, and eight `*Change` property wrappers (`RunPropertiesChange`,
`ParagraphPropertiesChange`, `ParagraphMarkPropertiesChange`, `SectionPropertiesChange`,
`TablePropertiesChange`, `TableExceptionPropertiesChange`, `TableGridChange`, `CellPropertiesChange`,
`RowPropertiesChange`), each reusing the live property type MJXOFF-94/96/109/119 already built for
its own "previous properties" payload rather than a parallel type.

**One rule, structurally enforced, for every mutation path in this crate:** `w:ins`/`w:del`/
`w:moveFrom`/`w:moveTo` are opaque containers to ordinary run/paragraph addressing, field scanning
and range resolution — content nested inside one is preserved exactly but never reached by the
editing surface, and every property setter already only ever replaces or inserts the one content
variant it owns, so a `*Change` sibling is never disturbed. `revisions.rs`'s own module doc carries
the full mutation-path table (MJXOFF-92/109/116/119/121/124, each with a stated and tested
behaviour). `Document::revisions`/`text_with_revisions_accepted`/`text_with_revisions_rejected` are
new read-only entry points (enumeration and computed accept/reject text — the ticket's own required
"Reading" bullet); mutating accept/reject operations are declined, with the reasoning recorded in
the same module doc.

**A malformed `w:date` is preserved verbatim, never normalised, and refused on authoring** — the
same fidelity-vs-validity split `fields.rs` (MJXOFF-121) established for over-long strings, now
applied to `ST_DateTime`, via a new `DocxError::MalformedDateTime`.

**Two corrections to this ticket's own pre-dispatch note**, both verified directly against
`wml.xsd`: `CT_TrackChangeNumbering` is *not* unreachable — it has two real use sites (`w:numPr`'s
own `numberingChange` and `w:fldChar`'s own), both already wired as `Unmodeled` placeholders by
MJXOFF-96/121 awaiting this child — and is modelled here. `CT_TrackChangeRange` genuinely *is*
unreachable (declared, never referenced anywhere in `wml.xsd`) and is not modelled.
`CT_MathCtrlIns`/`CT_MathCtrlDel` are reached only through `shared-math.xsd`, which no math content
in a Word run is typed against yet, so neither has a reachable call site in this crate either.

## [0.0.94] - 2026-09-04

Word comments, footnotes, endnotes and bookmarks (MJXOFF-124, Phase C position 15):
`crates/mjx-docx/src/document/annotations.rs` (new), `ranges.rs` (new).

**`word/comments.xml`, `word/footnotes.xml` and `word/endnotes.xml` are typed, read, round-tripped and
authored** — three of `wml.xsd`'s fourteen global elements that could not be reached at all before
this child. Each is read/edited through a `style_sheet`-shaped pair
(`Document::comments`/`edit_comments`, `footnotes`/`edit_footnotes`, `endnotes`/`edit_endnotes`),
created on demand with its content type and relationship, and `Document::add_comment`/`add_footnote`/
`add_endnote`/`add_bookmark` wrap the whole target paragraph in the matching range markers or
reference and assign a fresh id — never one already in use.

**One range-resolution mechanism (`ranges.rs`) serves both `w:bookmarkStart`/`w:bookmarkEnd` and
`w:commentRangeStart`/`w:commentRangeEnd`, pairing every marker by its own `id` attribute alone —
never by a stack.** ECMA-376 Part 1 §17.13.6.2 states the rule directly ("matched … by matching the
value of the id attribute"); a stack pairs whichever range opened most recently with the next end
marker it sees, which is wrong the instant two ranges overlap without nesting. A hand-built fixture
(`A` starts, `B` starts, `A` ends, `B` ends — no writer that only emits well-nested ranges can produce
it) proves this both ways: it resolves correctly against the shipped `id`-keyed implementation, and a
LIFO-stack mutation of the same function turns exactly that one test red. `RangeIndex::build` takes a
classifier closure rather than being hard-coded to one marker kind, so MJXOFF-126's own
`moveFromRangeStart`/`moveToRangeStart`/`customXml*RangeStart` reuse the same engine once they have
typed variants. Range resolution recurses into every table cell (not into a `w:hyperlink`'s own nested
content, a documented scope limit), so a bookmark starting inside a cell and ending after the table
resolves correctly.

**The reserved `separator`/`continuationSeparator`/`continuationNotice` footnote/endnote entries are
identified by `w:type`, never by `w:id`.** The ticket's own "conventionally ids `0`/`-1`" turned out
to be exactly that — a convention: ECMA-376 Part 1's own worked examples (§17.11.1, §17.11.23) use
`id="1"` and `id="0"`, not `-1`/`0`. `FootnoteEndnote::is_user_visible` and
`Footnotes::user_footnotes`/`Endnotes::user_endnotes` filter on `w:type` alone; a freshly authored
part always carries both reserved entries (Word repairs a file that lacks them), and the part itself
is never removed even once every user footnote is gone.

**MJXOFF-121's `Hyperlink::anchor` seam is closed, not left as a documented gap**:
`Document::resolve_bookmark` takes the raw anchor name `HyperlinkTarget::Anchor` carries and resolves
it against the body's own bookmark index, returning the bookmark's id and the text it covers, or
reporting an unmatched start (real files have these; ECMA-376 calls them non-conformant, not
impossible) rather than panicking.

Section-level `w:footnotePr`/`w:endnotePr` (`FootnoteProperties`/`EndnoteProperties`: position, number
format, start number, restart rule) — left `Unmodeled` by MJXOFF-109 for this child — are typed too,
via two new curated `mjx_ooxml_types::child_order` constants (`FOOTNOTE_PROPERTIES`,
`ENDNOTE_PROPERTIES`).

## [0.0.93] - 2026-09-04

Word fields, hyperlinks and form fields (MJXOFF-121, Phase C position 14): `crates/mjx-docx/src/document/fields.rs` (new), `hyperlinks.rs` (new).

**Both field wire forms — `w:fldSimple` and the `begin`/`separate`/`end` `w:fldChar` sequence — read
through one model, [`Field`], with instruction and cached result always distinct accessors.**
Nesting (a `TOC` field's cached result containing its own `PAGEREF` fields) is paired with a
recursive-descent stack, not a counter: a mutation that counts markers instead of nesting them turns
three tests red, including the committed `fields_and_hyperlinks.docx` fixture's own nested-`TOC`
case. An instruction split across several `w:instrText` runs concatenates for reading and, on write,
collapses to a single new run positioned at the edited field's own marker — every other field, and
every other part, stays byte-identical (proved both directions: editing an instruction leaves the
cached result untouched, and vice versa). A `w:fldChar` sequence that does not balance —
schema-valid markup ECMA-376 imposes no ordering constraint on — is a typed error
(`DocxError::UnbalancedField`), never a panic or a silent mispairing; a field with no `separate` (a
legal, resultless field) reads correctly and is not an error.

**Hyperlinks** (`Hyperlink`, typed for `r:id`/`anchor`/`tgtFrame`/`tooltip`/`docLocation`/`history`;
`Document::insert_hyperlink`/`remove_hyperlink`/`hyperlink_target`) wrap the runs they link, matching
WordprocessingML's own structural (not attribute) model. Adding one creates a valid external
relationship; removing one removes it — unless another hyperlink still names the same relationship —
and `Package::validate` reports no orphan either way. `w:anchor` (a bookmark name) is read
unresolved; MJXOFF-124 owns the bookmark index it would resolve against.

**Form fields** — `FormFieldData` (`w:ffData`) and its checkbox/drop-down-list/text-input kinds —
round-trip names, help/status text, macros and each kind's own options
(`Document::insert_form_field`/`edit_form_field`/`form_field`). Four `ST_*` members are
length-bounded strings, not enumerations (`ST_FFName` 65, `ST_FFHelpTextVal` 256,
`ST_FFStatusTextVal` 140, `ST_MacroName` 33); every setter refuses an over-long value with
`DocxError::ValueTooLong` at the API boundary rather than writing schema-invalid markup, while
reading an already-over-long value from an untrusted file is never rejected. `CT_FFCheckBox`,
`CT_FFDDList` and `CT_FFTextInput` are `xsd:sequence`-shaped (unlike `CT_FFData`'s own unordered
`xsd:choice`); every setter on the three places a new member at its schema rank via three curated
`mjx_ooxml_types::child_order` constants added for this child (`FORM_FIELD_CHECK_BOX`,
`FORM_FIELD_DROP_DOWN_LIST`, `FORM_FIELD_TEXT_INPUT`) — an append-only first draft of
`FormFieldDropDownList::set_selected_index` wrote `w:result` after every `w:listEntry`, which is
schema-invalid; the committed fixture's own schema-gate test catches the regression directly.

## [0.0.92] - 2026-09-04

Word table properties, table styles and conditional formatting (MJXOFF-119, Phase C position 13):
`crates/mjx-docx/src/document/table_properties.rs` (new), `table_regions.rs` (new), and the
`CT_TblPrBase`/`CT_TrPrBase`/`CT_TcPrBase` rungs `tables.rs`/`styles.rs` left opaque.

**Every remaining member of `CT_TblPrBase`, `CT_TblPrExBase`, `CT_TrPrBase` and `CT_TcPrBase` is
typed.** `Table`'s own `w:tblPr` carries a real `TableProperties` (was `Unmodeled`); `Row` gains
`w:tblPrEx` (`TableExceptionProperties`, the row-level override the table's own properties for that
row alone) and `w:trPr` (`RowProperties`); `CellProperties` grows from three typed members
(`gridSpan`/`hMerge`/`vMerge`, MJXOFF-116) to all fourteen. `w:style[@type='table']`'s own base
`w:tblPr`/`w:trPr`/`w:tcPr` and each `w:tblStylePr`'s own (MJXOFF-101's `TableStyleOverride`) reuse
these same three types directly — verified against `wml.xsd` that they are the identical complex
types, not merely similarly-shaped ones, so there is one table-formatting model, not two.

**Table-style conditional-formatting resolution** (`table_regions.rs`): which of a table style's
twelve regions (`ConditionalFormatRegion`, a reuse of the generated `TableStyleOverrideType`) cover a
cell is computed once per `(row, column)` from `w:tblLook`'s six flags and the band sizes
(`applicable_regions`), in the **application order ECMA-376 Part 1 §17.7.6.6 states verbatim** — whole
table, banded columns, banded rows, first/last row, first/last column, corners — so **column edges
beat row edges** and **row banding beats column banding**, both easy to get backwards and each pinned
by its own test (`tests/table_formatting.rs`). `w:tblLook`/`w:cnfStyle`'s legacy `val` bitmask is
preserved for round-trip and never consulted for region membership — Part 1's own prose for both
elements documents only the named `ST_OnOff` attributes.

**The table style joins the toggle-property XOR as a fourth term**, not a plain override rung:
`combine_toggle`/`recombine_toggles` (MJXOFF-106) gain a `table` operand alongside numbering,
paragraph-style and character-style, so a bold table style layered over a bold paragraph style
resolves to *not* bold (`true XOR true`), matching ECMA-376 Part 1 §17.7.3's own "true for an odd
number of levels" rule generalized to a fourth level.

`Document::effective_cell_fill`/`effective_cell_border`/`effective_cell_run_properties` are the three
new readers, named and shaped after `mjx_pptx::Presentation`'s own `effective_cell_*` trio (table
index in place of a slide surface plus shape path, since a `.docx` has no layout/master analogue).
`crates/mjx-docx/docs/effective_properties.md` documents the extended ladder and the region
precedence, with a compiled doctest.

## [0.0.91] - 2026-09-04

Word tables (MJXOFF-116, Phase C position 12): the grid, rows, cells, spans and structural edits —
`crates/mjx-docx/src/document/tables.rs`.

**`CT_Tbl`/`CT_Row`/`CT_Tc` are typed, and `w:tbl` stops being opaque.** `BlockContent::Table` (MJXOFF-92's
own enum, shared by `Body`, `HdrFtr` and now a table cell) carries the new `Table` model instead of
`Unmodeled`, so a table nests inside a cell for free — the same enum, no depth counter of its own,
bounded by the parse-time nesting limit `mjx-xml` already enforces. `BlockContent` also grows
`Properties(CellProperties)` for `w:tcPr`, mapped (never constructed) by `Body` and `HdrFtr` for the
same exhaustive-match reason `SectionProperties` already is.

**Word's vertical merge is a continuation model, not a span model**, and the grid resolution and
structural edits are built on that distinction (ECMA-376 Part 1 §17.4.84, Annex L.1.5.9): `w:gridSpan`
states a horizontal span in one place with no covered cell created, but `w:vMerge` states a vertical
merge by repeating a bare marker on every covered row's own `w:tc` — so a row's physical cell count is
not its column count, and `(row, column)` addressing (`Table::cell`/`cell_span`/`merge_anchor`) walks
each row accumulating `gridSpan` rather than indexing directly. `Document::cell_span`/
`merged_cell_anchor` mirror `mjx_pptx::Presentation`'s own names, `(row, column)` argument order and
return shape. `insert_row`/`remove_row` rewrite `w:vMerge` markers (a removed anchor row promotes the
cell below it, which takes over the whole removed cell's content); `insert_column`/`remove_column` grow
or shrink a straddled `w:gridSpan` — none of the four needs a `rowSpan` number to keep in step, because
Word's own model never has one. `Table::grid_discrepancies` is the active surface for a malformed grid
(a short row, an orphaned `w:vMerge` continuation, an empty row) real files can carry — exposed, never
panicked on.

Fixture: `tests/fixtures/ragged_table.docx`, a hand-authored, deliberately ragged 4×4 table (no
committed Word fixture carried a `w:tbl` before this) — a `w:gridSpan="2"` in a different place in
three of its four rows and a three-row `w:vMerge`, so cell index and grid column genuinely disagree
in three of four rows.

## [0.0.90] - 2026-09-04

Word headers and footers (MJXOFF-113, Phase C position 11): `CT_HdrFtr`, variant resolution, and the
legacy VML they carry — `crates/mjx-docx/src/document/headers.rs`.

**Header/footer parts reuse MJXOFF-92's block-content addressing rather than duplicating it.**
`body.rs`'s paragraph-vec logic (`paragraph`/`paragraph_mut`/`insert_paragraph`/`append_paragraph`/
`remove_paragraph`) is now five free functions (`block_paragraph[_mut]`, `block_insert_paragraph`,
`block_remove_paragraph`, …) operating on `&[BlockContent]`/`&mut Vec<BlockContent>`; `Body` delegates
to them, and the new `HdrFtr` (`CT_HdrFtr`, reusing `BlockContent` itself — `w:sectPr` is mapped in its
own `#[xml(children, …)]` list purely so the derive macro's exhaustive match compiles, never
constructed) uses the same functions. A header's paragraphs and runs are ordinary
`Paragraph`/`Run` — MJXOFF-94's run properties, MJXOFF-96's paragraph properties and MJXOFF-106's
effective-property ladder already work inside a header with no further wiring.

**Variant resolution — `Document::resolve_header`/`resolve_footer` — implements ECMA-376 Part 1
§17.10.1/.5/.2/.6, not a lookup.** A first/even query whose governing flag (`w:titlePg`/
`w:evenAndOddHeaders`) is off downgrades to the default (odd) query *before* the previous-section
inheritance walk runs — confirmed against the prose directly: *"If \[`titlePg`\] is set to false and a
first page header/footer is specified, then it shall be ignored and only the odd page header/footer
shall be displayed"* (§17.10.6), identically for `evenAndOddHeaders` (§17.10.1) and the even variant.
Inheritance is per-variant, from the nearest preceding section that states that specific type
(§17.10.5/.2, identical prose in both): *"If no headerReference for the \[…\] page header is specified
\[…\] the \[…\] page header shall be inherited from the previous section or, if this is the first
section in the document, a new blank header shall be created."* `w:evenAndOddHeaders` is read directly
from `word/settings.xml` (`Document::even_and_odd_headers`) — MJXOFF-136 models the part; this reads
only the one flag.

**`SectionProperties::remove_header_reference`/`remove_footer_reference` and
`ParagraphProperties::section_properties_mut` are new** — MJXOFF-109 built the field and its structural
push/read but not its removal, since resolution (and therefore "replace" and "remove") was this
child's own scope. `section_properties_mut` (the paragraph-level counterpart of `Body`'s own
`section_properties_mut`) exists so removing a reference never fabricates a `w:sectPr` a section did
not already carry.

**`mjx-vml` is a plain dependency of `mjx-docx` now, ungated** — unlike `mjx-pptx`'s `vml` feature
flag, which exists only to spare PresentationML callers a dependency they may never touch; Word headers
are the primary place VML watermarks and text boxes still appear in the wild.
`Document::header_footer_vml_drawings` resolves a header or footer's `mc:AlternateContent` via
`mjx-mce` (non-mutating) and reads every surviving `w:pict` through `mjx_vml::Drawing` — the first
consumer of MJXOFF-58's model outside PowerPoint.

**Two committed fixtures, both authored through this crate's own public API**
(`Document::blank`/`create_header`/`create_footer`/`edit_header_footer` — never a template):
`header_footer_variants.docx` (two sections; section 1 states all three header and footer variants
with `w:titlePg` absent, section 2 states none at all) and `header_watermark.docx` (one header holding
real, hand-authored `mc:AlternateContent`/`w:pict` VML — the one literal XML fragment in the change,
since this crate has no VML-authoring surface). Mutation-proved: neutralising the `w:titlePg` check,
the `w:evenAndOddHeaders` check, or the previous-section inheritance walk each turns a distinct set of
`crates/mjx-docx/tests/headers.rs` tests red; restored by re-editing.

Fixes a stale `document/mod.rs` module doc that still listed `styles.rs`, `numbering.rs`,
`effective.rs` and `sections.rs` among files "later children are expected to add" — all four already
existed.

## [0.0.89] - 2026-09-04

Word sections (MJXOFF-109, Phase C position 10): `w:sectPr`, page setup, columns, section breaks,
line numbering, header/footer references and `w:printerSettings` — `crates/mjx-docx/src/document/sections.rs`.

**A section's properties live at the END of the range they govern, not the start.** A `w:sectPr`
inside a paragraph's `w:pPr` ends a section *at* that paragraph; the body-level one is always the
document's last section. `SectionProperties::sections` (via the new `sections_in`) walks a body's
paragraphs and returns `SectionSpan`s accordingly. **A single-section fixture cannot catch a reader
that only ever looks at the body-level `w:sectPr`** — `tests/fixtures/three_section_document.docx`
is authored specifically to: section 1 (paragraphs 0–1, landscape A4), section 2 (paragraphs 2–3,
portrait A4, two equal-width columns), section 3 (paragraph 4, the body-level `w:sectPr`, portrait
A4, one column). Mutation-proved: neutralising the paragraph-level scan in `sections_in` turns five
tests red, including the mutation-gate test itself (`paragraph_to_section_assignment_is_correct_on_the_three_section_fixture`,
`left: 0, right: 1`); restored by re-editing.

**All 19 of `EG_SectPrContents` and `EG_HdrFtrReferences`** are modelled on `SectionProperties`:
`w:type`, `w:pgSz`/`w:pgMar` (bridged to the shared `PageSize`/`PageMargins` value types — see
below), `w:paperSrc`, `w:pgBorders` (reusing MJXOFF-94's `Border` model for `w:top`/`w:left`/
`w:bottom`/`w:right` via a `xsd:extension` — `Border::extension_attributes[_mut]`, a small
crate-visible escape hatch, rather than a fourth copy of `CT_Border`'s nine attributes),
`w:lnNumType`, `w:pgNumType`, `w:cols`, `w:formProt`/`w:noEndnote`/`w:titlePg`/`w:bidi`/`w:rtlGutter`
(reusing `Toggle`), `w:vAlign`, `w:textDirection` (reusing `ParagraphTextFlowDirection` directly —
`CT_TextDirection` is the identical type under the identical local name at both `w:pPr` and
`w:sectPr`), `w:docGrid`, `w:printerSettings` (reusing `RelationshipReference`), `w:headerReference`/
`w:footerReference` (the flag and the field are modelled here; *which* header/footer applies is
MJXOFF-113's), and `w:sectPrChange`/`w:footnotePr`/`w:endnotePr` (structure only, kept opaque —
MJXOFF-126/MJXOFF-124 own their semantics).

**`w:equalWidth="true"` wins over an explicit `w:col` list, confirmed against ECMA-376 Part 1
§17.6.4's own prose** ("If `equalWidth` is true, then the columns are defined using the data stored
as attributes of the `cols` element … If `equalWidth` is false, then the columns are defined using
the presence and data on each child `col` element", with a worked example describing the `w:col`
children as "ignored" once `equalWidth="1"`). `Columns` does not resolve this itself (no page-margin
knowledge to compute a width from) — it exposes `is_equal_width` and the explicit `columns()` list
independently, with the ruling written down once in `sections.rs`'s own module doc.

**`w:pgMar/w:header`/`w:footer` are measured from the page edge, not the text body** — confirmed
directly against ECMA-376 Part 1 §17.6.11 ("`header` … Specifies the distance … from the top edge of
the page to the top edge of the header"; "`footer` … from the bottom edge of the page to the bottom
edge of the footer"), restated on `PageMargins`'s own field docs.

**`PageOrientation` de-duplicated** (see Breaking changes): the public API now exposes exactly one
orientation type, the generated `mjx_ooxml_types::wordprocessingml::PageOrientation`, re-exported
from `mjx_docx::page`.

**`blank.rs`'s hand-written minimal `w:sectPr` is replaced by the real modelled writer.** The outer
skeleton (`<w:document>`/`<w:body>`/`<w:p/>`) is still a hand-written template — matching
`mjx_pptx::blank`'s own established convention for a part built from nothing — but the `w:sectPr`
fragment itself now comes from `SectionProperties::new` + its own setters, serialized on its own and
spliced in as bytes, never hand-formatted. Fixing this surfaced a real, previously-latent gap: a
`Document::blank`-authored document never declared `xmlns:r`, so any `r:`-prefixed attribute this
child's own new functionality can now write (`w:printerSettings@r:id`, `w:pgBorders`' corner
relationships, `w:headerReference`/`footerReference@r:id`) would have produced namespace-unbound,
invalid XML. `blank.rs` now declares `xmlns:r` alongside `xmlns:w` on the root, matching every real
Word/LibreOffice-authored document (`tests/fixtures/sample.docx` included).

**`w:printerSettings` never rewrites the binary part it references.** Proved on
`tests/fixtures/printer_settings_reference.docx` (authored — no fixture in the corpus carried a
Printer Settings part): editing an unrelated field of the *same* `w:sectPr` that carries
`w:printerSettings` leaves the referenced part's bytes and the relationship's id/target byte-identical.

**Splitting a document into a new section places the new `w:sectPr` inside the terminating
paragraph's own `w:pPr`, never appended to the body** — `Document::edit_section_properties` (get-or-
insert, unifying "change an existing section" and "create a new one") and
`Document::remove_section_properties`, both addressed by the new `SectionLocation` enum.

## [0.0.88] - 2026-09-04

Word effective-properties ladder (MJXOFF-106, Phase C position 9): `Document::effective_run_properties`
and `Document::effective_paragraph_properties` — every `EG_RPrBase` (38 fields) / `CT_PPrBase`
(32 fields) member resolved across `w:docDefaults` → the numbering level → the paragraph-style
`w:basedOn` chain → the character-style chain → direct formatting, with colours baked to concrete
`RRGGBB` through `mjx-dml`'s own theme model.

**The ladder order the ticket stated was wrong, verified against ECMA-376 Part 1 §17.7.2's own
prose, not assumed.** The ticket ordered the paragraph-style chain above the numbering level;
§17.7.2 states the opposite ("First, the document defaults … Next, … numbered item and paragraph
properties are applied … Next, paragraph and run properties are applied … as defined by the
paragraph style"). `tests/effective.rs`'s discriminating fixture (`w:sz` set to three different
values at docDefaults/numbering/the paragraph-style chain, moved one rung at a time across three
paragraphs) is built to fail under the ticket's own order; mutating the merge fold to that order
turns two tests red, pasted in the PR.

**Toggle properties combine by XOR across ladder tiers, and only twelve of them (ECMA-376 Part 1
§17.7.3), not every `CT_OnOff`-shaped member.** A run whose paragraph style and character style both
state `w:b="true"` renders **not bold** — `true XOR true = false` — the opposite of what a naive
override-based resolver (the same rule every other field correctly uses) would answer. Proved by
mutation: replacing the twelve-field XOR recombination with plain fallback turns the cancellation
test red.

**Theme colour and theme font resolve through `mjx-dml`'s own theme model — no second one.** Word's
`ST_ThemeColor` (17 wire tokens, including the `background1`/`text1`/`background2`/`text2` aliases)
maps onto DrawingML's `a:schemeClr` vocabulary; the `bg1`/`tx1`/`bg2`/`tx2` half of that mapping
reuses `mjx_dml::ColorMap::identity` directly rather than restating it, since its own default
pairing (`bg1→lt1`, `tx1→dk1`, …) is exactly what ECMA-376 Part 1 §17.15.1.20 states for Word's
`w:clrSchemeMapping` when absent (true of every fixture in this workspace — `word/settings.xml` is
not modelled by any child yet). `w:rFonts`'s theme attributes resolve the same way against the font
scheme's major/minor × Latin/East-Asian/complex-script slots.

**Cache design:** a chain, once resolved, is reused for every field of one `effective_*` call rather
than re-walked per field — `ChainCache`, memoized by `styleId`, scoped to a single call. It does not
survive across separate calls (`mjx_ooxml_core::Interner` is not `Clone`, and every `Document`
accessor already re-parses its part fresh); the guide states the caller-side alternative for a loop
over many runs.

**A gap found and fixed while wiring the ladder:** `StyleParagraphProperties` (MJXOFF-101) modelled
`w:spacing`/`w:ind` structurally (both round-tripped) but exposed no `spacing()`/`indentation()`
accessor at all — a caller could not read or write a style's own spacing/indentation. Added with the
same `value_property!` macro every sibling accessor already uses.

`crates/mjx-docx/docs/effective_properties.md` is wired into a real doctest gate the way
`mjx-pptx`'s own page is — `src/effective_properties.rs` is `#![doc = include_str!(...)]` with no
items of its own, so the guide's snippets are compiled by `cargo test --doc`, not merely present;
proved by breaking one assertion and watching the doctest go red before restoring it.

## [0.0.87] - 2026-09-04

Word numbering definitions (MJXOFF-104, Phase C position 8): `word/numbering.xml` in full —
abstract numbering definitions (`w:abstractNum`, up to nine `w:lvl` each), numbering instances
(`w:num`), per-instance level overrides (`w:lvlOverride`), picture bullets (`w:numPicBullet`), and
the two-hop resolution from a paragraph's `w:numPr` to the level it actually uses.

**Two-hop resolution, indexed by real key, never by position.** `w:numPr/w:numId` names a `w:num`;
that instance's own `w:abstractNumId` names a `w:abstractNum`; the abstract definition holds the
levels. `NumberingIndex` (built once from a `&Numbering` snapshot, the same design
`StyleIndex`/MJXOFF-101 already uses) indexes both hops by `numId`/`abstractNumId`, never by
document-order position — `numId` values need not be contiguous or ascending, and two instances may
share one abstract definition. `tests/fixtures/numbering_definitions.docx`, authored for this child
(no fixture in the corpus carried `word/numbering.xml` at all), seeds exactly that trap: `numId` 2
and 5 share one abstract definition, deliberately out of order against `numId` 9, and only `numId` 2
carries a `w:lvlOverride/w:startOverride`. Mutation-proved: neutralising the override handling turns
`numId` 5's own (un-overridden) resolved start wrong too, confirmed red and restored by re-editing.

**`numId = 0` is "no numbering", not a lookup failure; a genuinely dangling `numId` is a typed
error.** Proved against the real, already-committed `tests/fixtures/paragraph_properties.docx`
(MJXOFF-96), which carries a real `w:numPr` (`numId` 5) while relating to no `word/numbering.xml` at
all — not only against a synthetic case.

**`w:numStyleLink` resolves through `StyleIndex` — the seam between two OPC parts.**
`Document::resolve_numbering` follows the redirect (a numbering-type style's own `w:pPr/w:numPr`
substitutes for the numStyleLink-carrying definition's own, typically empty, level list), one part
parse at a time since each OPC part carries its own `Interner` and two cannot be held open on the
same `Package` at once; bounded (`MAX_NUM_STYLE_LINK_DEPTH`), the same design
`MAX_BASED_ON_CHAIN_DEPTH` already uses for `w:basedOn`.

**Displayed list numbers are not computed** — deliberately. Turning a resolved level into "1.2.3"
or a bullet glyph requires counting every preceding paragraph in the same list, `w:lvlRestart`,
restart on entering a higher level, and continuation across sections; a counter correct only for a
flat single-level list would be actively misleading. `numbering.rs`'s own module doc states the
boundary explicitly, in the style the PowerPoint effective-properties page already uses for its own
deliberate absences. Rendering a list's text remains MJXOFF-106's.

**Two ticket corrections, verified directly against `wml.xsd`:** `CT_NumRestart` and
`CT_TrackChangeNumbering` are both unreachable from `CT_Numbering` — the former is
footnote/endnote restart (`EG_FtnEdnNumProps`, MJXOFF-124's scope), the latter is `CT_NumPr`'s own
tracked-change wrapper (already opaque, MJXOFF-96) and `CT_FldChar`'s. Neither belongs to this
child.

`CT_Lvl`'s own `w:pPr`/`w:rPr` are `CT_PPrGeneral`/`CT_RPr` — confirmed against the schema, not
assumed — so `NumberingLevel` reuses `StyleParagraphProperties` (MJXOFF-101) and `RunProperties`
(MJXOFF-94) directly rather than restating either. Picture bullets preserve whichever payload a
real file carries (`w:pict` legacy VML, the common case, or `w:drawing`) as opaque, pending
MJXOFF-113/MJXOFF-131's own typed models. `Document::{numbering, edit_numbering,
attach_paragraph_to_list, detach_paragraph_from_list}` mirror the `styles.xml` authoring surface,
including creating `word/numbering.xml` — relationship and content type — on first use.

## [0.0.86] - 2026-09-04

Word style definitions (MJXOFF-101, Phase C position 7): `word/styles.xml` in full —
`w:docDefaults`, every `CT_Style` member, `w:basedOn` chain resolution with cycle safety, and
`w:latentStyles`.

**`CT_Style/w:pPr` is `CT_PPrGeneral`, not `CT_PPr`.** Verified directly against `wml.xsd`, not
assumed from the ticket's own text (which named `CT_PPrGeneral` as already built — it was not):
`CT_PPr` (a live paragraph's own `w:pPr`, MJXOFF-96) is `CT_PPrBase` plus `rPr` (`CT_ParaRPr`),
`sectPr` and `pPrChange`; `CT_PPrGeneral` — what a style definition, `w:pPrDefault` and
`w:tblStylePr` all actually carry — is `CT_PPrBase` plus `pPrChange` only. A style's own paragraph
properties may not carry a pilcrow's run properties or a section break, so `StyleParagraphProperties`
is its own container rather than `ParagraphProperties` reused; every one of its 33 leaf types
(`Toggle`, `FrameProperties`, `Spacing`, `ParagraphBorders`, …) is still the exact struct
MJXOFF-96 built, reused directly — only the wiring is new. `CT_Style/w:rPr`, by contrast, genuinely
is plain `CT_RPr` and reuses `RunProperties` with no wrapper at all. `w:tblPr`/`w:trPr`/`w:tcPr`
(on both `CT_Style` and `CT_TblStylePr`) stay opaque, the same treatment `w:pPrChange` already
gets — no shipped crate models table properties yet, and inventing a first model of them here would
be scope this child was not given.

**Cycle safety is a bounded depth, not a visited-set — and hitting the bound is a typed error,
never a silently truncated chain.** `StyleIndex::based_on_chain` walks `w:basedOn` from a style
upward, accumulating each ancestor into the `Vec` it must return anyway; that accumulation *is*
the bound (`MAX_BASED_ON_CHAIN_DEPTH = 64`) — a chain that has not terminated by then returns
`Err(DocxError::BasedOnChainTooDeep)`, never a partial `Ok` chain a later caller could resolve
properties against without anything going red. Proved by mutation: turning the bound check into a
silent `break` (an `Ok` chain of 64 repeated entries instead of an error) turns both cycle tests red.
`sample.docx`'s `Normal` style does **not** self-reference — checked directly against the fixture's
own bytes by two independent methods, refuting an earlier dispatch brief's claim — so the corpus has
no cycle to test against; `tests/fixtures/style_based_on_cycle.docx` (a self-reference and a mutual
pair) is the only cycle evidence in the suite, and `based_on_chain` is separately exercised against
`sample.docx`'s own real, non-cyclic chains to prove the depth cap never false-positives.

**The three-deep `basedOn` trap, closed with a discriminating fixture:** `Base → Middle → Leaf`,
where `Middle` overrides `Base`'s font size and `Leaf` overrides nothing, so `Leaf`'s correct
effective font size can only come from walking to `Middle` — reading only the leaf, only the base,
or only direct properties each gives a different wrong answer. Mutation-proved: neutralising the
chain walk (stop after the first push) turns this test red.

`w:styleId` matching is case-sensitive; `w:name` matching is case-insensitive (full Unicode case
fold), matching Word's own "apply style by name" UI — `sample.docx` already shows two producers
disagreeing on capitalisation (`PreformattedText` vs. `"Preformatted Text"`). `w:link` resolves in
both directions through `LinkedStyleResolution`, reporting a missing or wrong-kind target as a
value, never a panic.

`w:count` on `w:latentStyles` is preserved, never silently recomputed — `LatentStyles::sync_count`
is the explicit, opt-in way to keep it consistent with the exception list after an edit.
`tests/fixtures/style_latent_styles.docx` is the only committed coverage: `sample.docx` carries no
`w:latentStyles` at all (checked directly).

`Document::edit_style_sheet` creates `word/styles.xml` — content-type registration and the
`styles` relationship from the main document part — on first use for a document that has none (a
[`Document::blank`] document, among others), then runs the same parse/mutate/write-back shape
every other typed edit in this crate uses; `Document::style_sheet` is its read-only, closure-based
counterpart.

### Added

- **`mjx_docx::{StyleSheet, StyleDefinition, DocumentDefaults, DefaultRunProperties,
  DefaultParagraphProperties, LatentStyles, LatentStyleException, StyleParagraphProperties,
  TableStyleOverride, StyleString, RevisionSaveId}`** and their content enums — the full
  `word/styles.xml` model (`CT_Styles`, `CT_Style`, `CT_DocDefaults`, `CT_RPrDefault`,
  `CT_PPrDefault`, `CT_LatentStyles`, `CT_LsdException`, `CT_PPrGeneral`, `CT_TblStylePr`).
- **`mjx_docx::{StyleIndex, LinkedStyleResolution, MAX_BASED_ON_CHAIN_DEPTH}`** — the style index
  (built once from a `&StyleSheet` snapshot, reused for every lookup), `w:basedOn` chain walking,
  and `w:link` resolution.
- **`mjx_docx::Document::{style_sheet, edit_style_sheet}`** — reading and authoring
  `word/styles.xml`, creating it (with its relationship and content type) on first use.
- **`mjx_docx::DocxError::{UnknownStyleId, BasedOnChainTooDeep}`**.
- New fixtures: `tests/fixtures/style_based_on_chain.docx`, `style_based_on_cycle.docx`,
  `style_latent_styles.docx` — authored for this child; `sample.docx` supplies neither a
  three-deep override chain, a `basedOn` cycle, nor `w:latentStyles`.
- **`mjx_ooxml_types::child_order::{PARAGRAPH_PROPERTIES_GENERAL, DOCUMENT_DEFAULTS,
  DEFAULT_RUN_PROPERTIES, DEFAULT_PARAGRAPH_PROPERTIES, LATENT_STYLES, STYLE_DEFINITION, STYLES,
  TABLE_STYLE_OVERRIDE}`** — generated child-order tables for `CT_PPrGeneral`, `CT_DocDefaults`,
  `CT_RPrDefault`, `CT_PPrDefault`, `CT_LatentStyles`, `CT_Style`, `CT_Styles` and `CT_TblStylePr`
  (`xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`).

## [0.0.85] - 2026-09-04

Word document authoring from nothing (MJXOFF-98, Phase C position 6): `Document::blank` and
`Document::blank_with_properties`, mirroring `mjx_pptx::Presentation::blank`'s shape — one call, no
file, no template. On top of `mjx_opc::Package::empty` (the same OPC primitives `mjx-pptx`'s own
`blank.rs` uses), this writes `word/document.xml` (one empty paragraph and a body-level `w:sectPr`
naming the caller's page) plus `docProps/core.xml`/`docProps/app.xml` (MJXOFF-149's packaging-layer
decision, restated rather than re-derived). `PageSize`/`PageOrientation` give the caller `a4()` or
`us_letter()`, portrait or `landscape()`, refused with a typed `DocxError::InvalidPageSize` before
any byte is written if this crate's fixed "Normal" margins (1 inch, matching Word's own template)
would leave no printable area.

**Which optional parts a blank document gets, and why the answer differs from PowerPoint's own.**
`tests/fixtures/sample.docx` — LibreOffice's own output — ships ten parts, four beyond
`word/document.xml` and the two `docProps`: `styles.xml`, `fontTable.xml`, `settings.xml` and
`theme/theme1.xml`. None is schema-required (`wml.xsd`'s thirteen part-bearing global elements are
all `minOccurs="0"` from wherever they are reached). `mjx_pptx::blank`'s answer to the same "what
beyond the schema minimum" question is to include the master, layout and theme, because without them
a deck is *structurally* unusable — there is no layout to build a slide from. WordprocessingML has no
such dependency: a paragraph with no `w:pStyle` and a run with no `w:rStyle` are both legal, and every
real Word implementation falls back to a built-in appearance when a document names no style to
inherit from, so `Document::blank`'s body is fully usable through MJXOFF-92's `insert_paragraph` /
`append_run` / `set_run_text` with zero related parts. Writing even a throwaway `docDefaults`-only
`styles.xml` — legal under this ticket's own wording — would be work MJXOFF-101 replaces on day one,
so this module writes none of the four, and `crates/mjx-docx/src/blank.rs`'s module doc names every
inclusion and every deliberate absence.

**A ticket correction, caught by checking `wml.xsd` directly rather than trusting the brief's own
claim:** `w:sectPr`, `w:pgSz` and `w:pgMar` are *not* schema-required either — `CT_Body`'s `sectPr`,
and `pgSz`/`pgMar` inside `EG_SectPrContents`, are all `minOccurs="0"`. All three are included for the
same "not required, but what makes the result usable" reasoning `mjx_pptx::blank` uses for its
placeholders, not because the schema demands them. The one attribute-level claim that genuinely is
`use="required"` — `CT_PageMar`'s seven attributes (`top`, `right`, `bottom`, `left`, `header`,
`footer`, `gutter`), if `w:pgMar` is written at all — is proved by mutation, once per attribute, in
`tests/schema_gate.rs`.

**A second, unrelated defect the schema gate caught while building this:** the first draft of
`document_bytes` wrote `w:pgSz`'s and `w:pgMar`'s attributes with no `w:` prefix (`w="11906"` rather
than `w:w="11906"`) — `wml.xsd` is `attributeFormDefault="qualified"`, the same class of defect
MJXOFF-152 fixed for this crate's typed attribute accessors, this time in a hand-written XML
template rather than a codec. `xmllint` rejected it immediately (`the attribute 'w' is not allowed`),
before it ever reached a test file.

The LibreOffice open canary (`tests/office_open.rs`, mirroring `mjx-pptx`'s own) is implemented and
skips cleanly on a machine with no `soffice` installed; `crates/mjx-docx/examples/blank_document.rs`
and the crate's first guide page (`crates/mjx-docx/src/guide.rs`, `building_a_document`) are the
runnable and prose versions of the same story.

## [0.0.84] - 2026-09-04

Word paragraph properties (MJXOFF-96, Phase C position 5): `w:pPr` (`CT_PPr`) and all 33
`CT_PPrBase` children, plus the paragraph mark's own run properties (`w:pPr/w:rPr`, `CT_ParaRPr`).
`CT_PPrBase` is the other half of Word's direct formatting — the base MJXOFF-101 (styles) and
MJXOFF-109 (numbering levels) both build on.

Two traps this child exists to close: the paragraph-mark run properties are not a run's own — `w:b`
set through `Paragraph::paragraph_mark_properties_or_insert` can never touch a run's `w:rPr`, and
setting the paragraph's justification can never touch the pilcrow's — proved on bytes, not just by
type distinctness. And `w:spacing/@line` is meaningless without `@lineRule` (`auto` means 240ths of a
line, `exact`/`atLeast` mean twips): there is no `Spacing::line` accessor, only
`Spacing::line_spacing`, which always returns both together (`LineSpacing`), demonstrated by a
doctest.

`CT_ParaRPr` reuses `run_properties.rs`'s (MJXOFF-94) 39 `EG_RPrBase` leaf types directly —
`Toggle`, `Fonts`, `Color`, `Border`, `Shading`, … — rather than restating them; only `Toggle::new`
and `HalfPointMeasureValue::new` needed widening from private to `pub(crate)` to make that reuse
possible. `CT_PBdr`'s six borders and `w:pPr/w:shd` likewise reuse `CT_Border`/`CT_Shd`
(`super::run_properties::{Border, Shading}`) rather than defining a second border or shading type.

`CT_Ind`'s logical (`w:start`/`w:end`) and physical (`w:left`/`w:right`) spellings are both preserved
independently — nothing is normalised on write — with `Indentation::leading_edge`/`trailing_edge`
resolving between them when a file carries both: the logical spelling wins, since Annex M records it
as the later, Strict-compatible addition (ECMA-376 Part 1's own prose states no explicit precedence
here, unlike the `…Chars`-supersedes-twips rule it does state).

One correction to the ticket's own text: `w:kinsoku` inside `w:pPr` is `CT_OnOff` (a plain toggle),
not the two-attribute `CT_Kinsoku` complex type named in the ticket's "Complex types" list — that
type belongs to `w:noLineBreaksAfter`/`w:noLineBreaksBefore` in document settings, unrelated to
paragraph properties. `CT_DecimalNumberOrPrecent` (`w:summaryLength`) and `CT_ParaRPrOriginal`
(reachable only through `w:pPrChange`, MJXOFF-126's scope) are likewise not `CT_PPrBase` children and
have no home in this child.

New fixture: `tests/fixtures/paragraph_properties.docx` — the only committed `.docx` carrying
`w:line`, `w:tabs`, `w:ind`, `w:pBdr` or `w:framePr` before this child; its two paragraphs cover all
33 `CT_PPrBase` members between them, including the legacy physical indentation spelling and a
`w:tab` with `val="clear"` (which removes an inherited stop rather than adding one, so is preserved
structurally like any other stop).

Reachability: `Paragraph::properties`/`properties_mut`/`properties_or_insert` reach `w:pPr` from
`Paragraph`'s own public surface — MJXOFF-152 found `CT_R`'s legacy leaf types were correct but
unreachable through `Document`/`Body`/`Paragraph`/`Run`; this child does not repeat that gap for
`w:pPr` itself. `ParagraphProperties::section_properties`/`change` similarly reach `w:sectPr`/
`w:pPrChange` structurally (as `Unmodeled`), ahead of MJXOFF-106/MJXOFF-126 giving them real content.

### Added

- **`mjx_docx::ParagraphProperties`** (`CT_PPr`, `w:pPr`) — all 33 `CT_PPrBase` members plus the
  paragraph mark's own properties, the section this paragraph ends, and the tracked-change wrapper.
  Reached off `Paragraph::properties`/`properties_mut`/`properties_or_insert`.
- **`mjx_docx::ParagraphMarkRunProperties`** (`CT_ParaRPr`, `w:pPr/w:rPr`) — the pilcrow's own
  character formatting, distinct from a run's `w:rPr`.
- **`mjx_docx::{Spacing, LineSpacing, Indentation, FrameProperties, TabStops, TabStop,
  ParagraphBorders, NumberingProperties, ConditionalFormatting, ParagraphStyle, ParagraphAlignment,
  ParagraphTextFlowDirection, VerticalCharacterAlignment, TextBoxTightWrapSetting,
  DecimalNumberValue}`** and their content enums — the leaf and container types `CT_PPrBase`'s 33
  members are built from.
- **`mjx_ooxml_types::child_order::{PARAGRAPH_PROPERTIES, PARAGRAPH_MARK_RUN_PROPERTIES,
  PARAGRAPH_BORDERS, NUMBERING_PROPERTIES}`** — generated child-order tables for `CT_PPr`,
  `CT_ParaRPr`, `CT_PBdr` and `CT_NumPr` (`xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`).

## [0.0.83] - 2026-09-04

Fixes the defect 0.0.82's own changelog reported and left open (MJXOFF-152): `crates/mjx-docx/src/
document/body.rs`'s `Break`/`PositionalTab`/`Symbol`/`ProofingError`/`PermissionRangeStart`/
`PermissionRangeEnd` (MJXOFF-92) declared their attributes with no `prefix`, so every accessor
matched only a bare, unprefixed local name — but `wml.xsd` is `attributeFormDefault="qualified"`,
and real markup writes `w:font`, `w:alignment`, `w:type`, never bare. Every accessor on these six
types returned `None` (or `Missing`, for a required attribute) against a file that plainly carries
the value — confirmed against `run_content.docx`'s own `<w:sym w:font="Wingdings" w:char="F0E0"/>`
and `<w:ptab w:alignment="right" w:relativeTo="margin" w:leader="dot"/>`, committed since MJXOFF-92
and never once read correctly. Round-trip fidelity was unaffected throughout — the attribute vector
is retained and re-emitted verbatim regardless, which is why every byte-identity suite and the
schema gate stayed green; only the typed reads were broken, and nothing exercised them.

Audited every `#[xml(attribute(…))]` declaration in the file against `wml.xsd` by hand: 17 of 19
needed `prefix = "w"` added; the other two were already correct (`CT_Text`'s `xml:space`, prefix
`xml`; `CT_Rel`'s `id`, prefix `r` — a relationship reference into a different namespace's own
schema, not `wml`'s). **Do not blanket-add `w`** applied literally: those two stayed untouched.

Second, unrelated defect caught by the same audit: `body.rs`'s two local `AttributeCodec` tag types
(`WhitespacePreservation`, `ShortHex`) were private. That compiles inside the crate — same-module
visibility hides it — but `Text::preserve_whitespace` and `Symbol::character` name the private type
in their return type via `AttributeCodec::Value`, which is a hard compile error for any caller
outside this crate. The same class MJXOFF-94 found and fixed for `run_properties.rs`'s own seven
tag types the release before this one. Made both `pub` and re-exported.

A workspace-wide audit (MJXOFF-152's own scope, not just `mjx-docx`) confirmed `wml.xsd` and
`shared-math.xsd` are the *only* two of the schemas this project models that declare
`attributeFormDefault="qualified"`; `sml.xsd`, `pml.xsd` and `dml-main.xsd` declare no
`attributeFormDefault` at all (XSD's default is `unqualified`), and `dml-chart.xsd`,
`dml-diagram.xsd` and `vml-main.xsd` say `unqualified` explicitly. `mjx-dml`'s 318 attribute
declarations (5 of them correctly `prefix = "r"` for relationship references, the rest correctly
unprefixed) confirm the unqualified reading in practice; `mjx-chart`, `mjx-vml` and `mjx-pptx`
declare no typed attribute accessors yet, so there was nothing there to audit. **`shared-math.xsd`
being qualified is a live warning for MJXOFF-134** (`mjx-omml`, not yet written): its leaf types will
need the same `prefix = "w"`-style treatment `wml.xsd` needed here, from the first declaration,
recorded on that ticket.

### Fixed

- **`mjx_docx::{Break, PositionalTab, Symbol, ProofingError, PermissionRangeStart,
  PermissionRangeEnd}`** — every attribute accessor now reads the value real, `w:`-prefixed markup
  states, proved against `run_content.docx` (already committed) and a new
  `tests/fixtures/leaf_attributes.docx` (for the three elements — `w:br` with real values,
  `w:proofErr`, `w:permStart`/`w:permEnd` — neither existing fixture carries with attribute values
  set, so neither could discriminate this defect).
- **`mjx_docx::{WhitespacePreservation, ShortHex}`** — made `pub` and re-exported; both were private,
  which made `Text::preserve_whitespace` and `Symbol::character` uncallable (a compile error) from
  outside this crate.

## [0.0.82] - 2026-09-04

Run properties (MJXOFF-94): `w:rPr` and the character-formatting vocabulary — `EG_RPrBase`'s **39
members** (the ticket said 38 plus `oMath`; the schema gives the group exactly 39, and `oMath` is
`CT_OnOff`-shaped like nineteen of its siblings, not a fortieth special case). `EG_RPrBase` is the
most-referenced group in `wml.xsd`: `CT_RPr`, `CT_ParaRPr`, `CT_RPrOriginal`, `CT_ParaRPrOriginal`,
`CT_Style` and `CT_RPrDefault` all build on it, so MJXOFF-96, MJXOFF-101, MJXOFF-104, MJXOFF-119 and
MJXOFF-126 all needed this landed first.

### Added

- **`mjx_docx::RunProperties`** (`CT_RPr`), reached off `Run::run_properties`/`run_properties_mut`/
  `run_properties_or_insert` — the last placing a freshly authored `w:rPr` at its schema rank via the
  generated `wml` child-order table.
- **`mjx_docx::Toggle`** — the twenty `CT_OnOff`-shaped members (`b`, `bCs`, `caps`, `cs`, `dstrike`,
  `emboss`, `i`, `iCs`, `imprint`, `noProof`, `oMath`, `outline`, `rtl`, `shadow`, `smallCaps`,
  `snapToGrid`, `specVanish`, `strike`, `vanish`, `webHidden`) share one type, reused exactly as
  `mjx_docx::Text` is reused across four `EG_RunInnerContent` members. `val` is declared with the
  attribute grammar's `default = true` — ECMA-376 Part 1's own prose for every one of these elements
  ("if this element is present without a val attribute, its default value is true") — so
  `RunProperties`'s twenty per-property accessors (`bold`, `italic`, …) return `Option<bool>`: `None`
  for the element absent, `Some(true)`/`Some(false)` for present-and-on/present-and-off, never
  collapsed to a bare `bool`.
- **The other eighteen complex types**: `CharacterStyle`, `Fonts`, `Color`, `Underline`, `TextEffect`,
  `Border`, `Shading`, `VerticalAlignment`, `ManualRunWidth`, `Emphasis`, `Languages`,
  `EastAsianLayout`, `Highlight`, and three measure-value wrappers (`HalfPointMeasureValue`, reused
  across `sz`/`szCs`/`kern`; `SignedHalfPointMeasureValue` for `position`;
  `SignedTwipsMeasureValue` for `spacing`) and `TextScaleValue` for `w`. Colour (`Color`,
  `Underline`'s and `Border`'s and `Shading`'s own colour attributes) is Word's own four-attribute
  model (`val`, `themeColor`, `themeTint`, `themeShade`) — not DrawingML's `a:schemeClr` with child
  transforms.
- **`tests/fixtures/run_properties.docx`** — the three `w:rPr` emptiness states `sample.docx` and
  `run_content.docx` don't between them cover (self-closed, absent, and a separate end tag with no
  children), and a run carrying all 39 properties at once, including `w:b w:val="0"` (explicit off,
  distinct from absent), `w:rFonts` with only a hint and no font name, and `w:u` with `color` and
  `themeColor` alongside `val`.

### Fixed

- Caught while writing this child's own tests: `wml.xsd` is `attributeFormDefault="qualified"`, so
  every WordprocessingML attribute is written `w:val`, not `val` — but nothing in this new
  vocabulary's attribute declarations named a `prefix`, so every single one matched only the
  unprefixed spelling and silently fell through to its schema default. `crates/mjx-docx/src/document/
  body.rs`'s pre-existing `Break`/`PositionalTab`/`Symbol`/`ProofingError`/`PermissionRangeStart`/
  `PermissionRangeEnd` (MJXOFF-92) carry the same latent defect on their own attributes, untested for
  the same reason: their round-trip tests pass the whole attribute vector through verbatim and never
  call the typed accessors. Not fixed here — out of this child's scope — and reported on the ticket.

## [0.0.81] - 2026-09-04

The WordprocessingML block content model (MJXOFF-92): `mjx-docx` could open a `.docx` and name its
parts (MJXOFF-90) but could not read a single word of one. This gives it `w:body`'s block content —
paragraphs, runs, text and the rest of `EG_RunInnerContent`'s 33 members — the content spine every
later Word child hangs off.

### Added

- **`mjx_docx::{Body, Paragraph, Run, Text, Hyperlink}`** and the two content enums that hold them
  together — `BlockContent` (`EG_ContentBlockContent`, plus `w:sectPr`) and `ParagraphContent`
  (`EG_PContent`). `Paragraph`/`Run`/`Hyperlink` are typed for real reach — a `w:hyperlink`'s own
  runs stay reachable; `w:customXml`/`w:smartTag`/`w:sdt`/`w:dir`/`w:bdo`/`w:tbl` stay
  `mjx_docx::Unmodeled` (opaque, unowned) until a later child claims one.
- **`mjx_docx::RunInnerContent`** — all 33 `EG_RunInnerContent` members, every one with a variant now
  (`Break`, `Text` reused for `t`/`delText`/`instrText`/`delInstrText`, `RelationshipReference`,
  `Symbol`, `PositionalTab`, `PhoneticGuide` fully typed; the sixteen `CT_Empty`-based members and
  seven later-child payloads — `w:fldChar` (MJXOFF-121), `w:object`/`w:pict`/`w:drawing`
  (MJXOFF-131), `w:footnoteReference`/`w:endnoteReference`/`w:commentReference` — stay `Unmodeled`).
  Adding a variant later would be a breaking change to an enum fifteen children depend on; a variant
  whose payload is still `Unmodeled` is not.
- **`mjx_docx::{BlockPath, RunPath}`** — the address of a paragraph and of a run, in
  `crates/mjx-docx/src/address.rs`, mirroring `mjx_pptx::ShapePath`'s manners (a bare index for the
  common case, an array/slice/`Vec` to descend a level) for WordprocessingML's own kind of nesting —
  block containers for paragraphs, run containers (`w:hyperlink`, so far) for runs — rather than
  `p:grpSp` groups.
- **`Document::{paragraph_count, run_count, paragraph_text, run_text, set_run_text, insert_paragraph,
  append_paragraph, remove_paragraph, insert_run, append_run, remove_run}`** — reading and editing
  paragraphs and runs, each edit going through `ToXml::write_back` so only the touched subtree
  re-serializes.
- **The `xml:space` rule** for `w:t` (`Text::set_text`): writes `xml:space="preserve"` when the new
  text starts or ends with ASCII whitespace, and removes the attribute otherwise — reading never
  trims, regardless.
- **`tests/fixtures/run_content.docx`** — a fixture carrying `w:br`, `w:tab`, `w:sym`, `w:cr`,
  `w:noBreakHyphen`, `w:ptab`, `w:ruby`, a `w:t` with `xml:space="preserve"`, a `w:hyperlink`
  wrapping two runs, and a `w:fldChar` (a run-inner element whose payload is still `Unmodeled`),
  swept automatically into every byte-identity suite and the schema gate.

### Also in this release: the Word crate spine (MJXOFF-90)

MJXOFF-90 shipped without a version bump of its own, so its work reaches a release here rather than
in a `0.0.81` of its own. Recorded rather than renumbered — the history is linear and a rewrite
would cost more than the misfiled heading does.

- **`mjx_docx::{Document, PartKind, DocumentParts, DocxError}`** — `Document::open`/`save`/
  `save_unchecked`/`validate`, mirroring `Presentation`'s names so the Word method is guessable from
  the deck one, and the part graph over `wml.xsd`'s fourteen global elements. `crates/mjx-docx` was
  thirteen lines and zero public items before it.
- **`xtask`'s child-order generator resolves `xsd:complexContent` and `xsd:simpleContent`.** An
  extension splices the resolved base chain *before* the derived type's own particle; a restriction
  replaces it; `simpleContent` contributes nothing. This is why `wml` can have an ordering table at
  all — its schema uses `complexContent` in 41 derived types — and it is what unblocks MJXOFF-132
  (`sml`, 6 `simpleContent`) and MJXOFF-134 (`shared-math`, 2) without either repeating the work.
- **The `wml` child-order table**, generated and committed, with the ordering audit proved red then
  green on real WordprocessingML.

## [0.0.80] - 2026-09-04

Document properties (MJXOFF-149): the programme held two contradictory positions on `docProps/*` —
"deliberately absent" in `mjx-pptx`'s own blank-deck module doc, and already assumed in two Word/Excel
tickets' part lists. Settled in favour of authoring: every file real Office writes carries
`docProps/core.xml` and `docProps/app.xml`, and `mjx-schema-gate`'s three-category rule was written
anticipating exactly this flip.

### Added

- **`mjx_opc::doc_props`** — `CoreProperties` (`title`, `creator`, `created`, `modified`),
  `ExtendedProperties` (`application`) and `DocumentTimestamp` (built only from explicit calendar
  fields — there is no `now()`), plus the writer, part-name, content-type and relationship-type
  constants for `docProps/core.xml` (ECMA-376 Part 2's `opc-coreProperties.xsd`, Dublin Core) and
  `docProps/app.xml` (`shared-documentPropertiesExtended.xsd`). Packaging-layer, so `mjx-pptx` and
  the Word/Excel `blank()` constructors still to come share one implementation.
- **`mjx_pptx::Presentation::blank_with_properties`** — `blank` with document properties set, rather
  than left absent. `blank` itself now writes both parts on every call, all-`None` by default (a
  schema-valid, childless part, since both are `xs:all` groups with every child optional).

### Fixed

- `mjx-schema-gate`'s `opc-coreProperties` and `shared-documentPropertiesExtended` namespaces move
  from the preserved-foreign allowlist to the modelled-schema table: `docProps/core.xml` and
  `docProps/app.xml`, in every fixture and every authored deck alike, are now genuinely validated
  against ECMA-376 rather than skipped as foreign markup. `opc-coreProperties.xsd`'s Dublin Core
  imports (`dc:`, `dcterms:`, real network `schemaLocation`s, unlike `wml.xsd`'s bare `xml:` import)
  are resolved through a committed local XML catalog rather than a live fetch.

## [0.0.79] - 2026-09-03

The DrawingML diagram (SmartArt) model (MJXOFF-148): `add_diagram` authored `dgm:` markup this
project neither modelled nor ordered, which is exactly the condition MJXOFF-110 exists to make
impossible. Closes the hole.

### Added

- **`mjx_dml::diagram`** — a typed model of `dml-diagram.xsd`, 50 of its 58 complex types down to
  their attributes: the data part as a point-and-connection graph (`DataModel`, `PointList`/`Point`,
  `ConnectionList`/`Connection`), the layout definition's whole algorithm tree (`LayoutDefinition`,
  `LayoutNode`, `Algorithm`, `Constraint`, `NumericRule`, `Choose`), the quick style
  (`StyleDefinition`/`StyleLabel`) and the colour transform
  (`ColorTransform`/`StyleLabelColors`/`ColorList`). A handful of externally-defined DrawingML
  formatting groups (`spPr`, `style`, `txPr`, `bg`, `whole`, `scene3d`, `sp3d`) and the SmartArt
  gallery-catalog header types this project never authors or reads stay unmodelled, by name and
  reason, in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md`. Running a `dgm:layoutDef` to compute
  where a consumer draws each point remains a documented non-goal — a rendering concern.
- **`mjx_ooxml_types::diagram`** — the whole `ST_*` family of `dml-diagram.xsd` (66 simple types),
  comprehensively named; `dml-diagram` joins `CHILD_ORDER_SCHEMAS`, so an authored diagram's four
  parts are ordered by construction rather than emitted from a fixed template with no writer checking
  its sequence.

### Fixed

- `mjx-schema-gate`'s `dml-diagram` row now validates for real: `add_diagram`'s four parts were
  already checked against `dml-diagram.xsd`, and a new case proves the check is live by writing
  markup the schema rejects and asserting it is caught, naming the schema — not merely that markup
  this project already writes happens to pass.

## [0.0.78] - 2026-09-03

A performance baseline and a large-file corpus generator (MJXOFF-147) — the numbers MJXOFF-95 (the
Excel cell store) designs its memory budget against, and the numbers a later regression is compared
to instead of intuition. Not an optimisation pass: nothing here got faster, the point is knowing.

### Added

- **`cargo run -p xtask -- corpus`** — (re)builds a git-ignored large-file corpus into
  `target/corpus/`: a 300-slide `.pptx` (`mjx_pptx::Presentation`'s real edit surface), a
  20,000-paragraph `.docx` and a 300,000-cell `.xlsx` (raw WordprocessingML/SpreadsheetML on
  `mjx_opc::Package` — neither format has a model yet), and prints size/element/cell counts.
  `corpus --mem <pptx|docx|xlsx>` runs its peak-resident-set checkpoints (open / first-mutation
  materialisation / edit / save) in one process via `/proc/self/status`'s `VmHWM`, the kernel's own
  peak-RSS counter — chosen over a counting allocator because it answers the literal question asked
  ("peak resident set") rather than a proxy for it. Not a substitute for MJXOFF-130's Office-authored
  fixtures, and does not claim to be.

- **Criterion benchmarks** — `crates/{mjx-pptx,mjx-docx,mjx-xlsx}/benches/`, six operations per
  format (`open`, `first_mutation_materialisation`, `edit_after_materialised`, and the three save
  paths `save_untouched` / `save_lightly_edited` / `save_fully_materialized`, measured separately
  because the gap between them is the result), plus a seventh for `mjx-pptx` exercising the real
  `Presentation` edit surface rather than only the lower `Package` layer.

- **`docs/BENCHMARKS.md`** — the baseline: the machine, the (existing, previously undocumented)
  release profile, all four operations × three formats' time and peak RSS, the three save paths
  compared, A7d's `mjx248_measure` reproduced on this machine (matches within ~10–35%, one direction,
  explained by MJXOFF-143), the short-list of figures MJXOFF-95 designs against, and two measurement
  bugs this child caught in its own harness before trusting its numbers.

### Findings, filed rather than fixed here

- Materialising the 610,005-element / 300,000-cell worksheet costs **+274 MiB of peak RSS** over an
  8.54 MiB raw-XML part — roughly 32× the source bytes, ≈ 913 B/cell. Filed as **MJXOFF-151** (under
  MJXOFF-88) for MJXOFF-95 to design against (an arena/columnar layout, per `PLAN.md`'s hybrid model),
  not fixed in this child.

## [0.0.77] - 2026-09-03

The untrusted-input paths are fuzzed, and three defects they were hiding are fixed (MJXOFF-146).

`CLAUDE.md` has always said it: *no `unwrap`/`panic`/`expect` on untrusted input — inputs are
untrusted files.* Nothing in the repository proved it. A grep finds the obvious cases and says
nothing about a recursion depth, a slice index, or an allocation an attacker sizes. This adds a
campaign that tries, and it found three things a grep could not.

### Added

- **`cargo run -p xtask -- fuzz`** — a campaign against the three untrusted-input entry points
  (`mjx_xml::fidelity::parse`, `mjx_opc::Package::open`, `mjx_mce::resolve`) plus the round-trip
  oracle, in five targets. Run **on demand, not on every push**; `--list`, `--target`, `--seed`,
  `--iterations` and `--seconds` select and bound a run, and a seed makes one reproducible.

  It is stable Rust with no new dependency. `cargo-fuzz` needs a nightly toolchain for its sanitizer
  flags, and a gate only some machines can run is not a gate. It lives in `xtask`, which is host-only
  and which nothing depends on, so the harness cannot reach the shipped graph.

  It asserts properties rather than the absence of a crash: every input the reader accepts must
  re-serialize **byte-for-byte**; the same corpus is re-run with the document dirtied at its root,
  where a byte range that does not describe its element shows; a package written back and reopened
  must hold the same part bytes. Panics are caught per execution, a counting global allocator
  measures each execution's peak against a ceiling so unbounded allocation is a *finding* rather than
  an OOM kill, and a watchdog turns a hang into an abort that names its input.

- **`mjx_fixtures::adversarial_xml`** and **`adversarial_xml_dirtied_at_the_root`** — the hostile XML
  corpus, moved out of `crates/mjx-xml/tests/subtree_cow.rs` so the hand-written gate and the
  campaign read the same list instead of drifting apart.

- **`mjx_xml::fidelity::MAXIMUM_DEPTH`** and **`XmlError::DepthLimit`** — see below.

- Regression suites for every finding, in the crate that owns the path:
  `crates/mjx-xml/tests/untrusted_input.rs`, `crates/mjx-opc/tests/untrusted_input.rs`,
  `crates/mjx-mce/tests/untrusted_input.rs`, and the minimised container
  `tests/fixtures/declared_size_lie.zip`.

### Fixed

- **A 140 KB document could abort the process.** The reader is iterative and would build a tree of
  any depth; every walk *over* that tree recurses, because the data does — `Drop` and `Clone` are
  compiler-generated, the serializer descends a dirty element, and `mjx_mce::resolve` descends the
  whole document. `resolve` died first, overflowing the stack at a nesting depth reachable in about
  140 KB of `<a>`. Not a catchable panic: an abort. `fidelity::parse` now refuses to build a tree
  deeper than `MAXIMUM_DEPTH` (256), which bounds every walk downstream, including the ones Phase C
  and D have not written yet. The deepest part in the committed corpus is **13**.

- **A 757-byte container could ask for four gigabytes.** A ZIP entry's uncompressed size is a header
  field, attacker-controlled and checked against the data only after the data has arrived.
  `Package::open` reserved exactly that many bytes per part, so a container declaring 4 GiB for a
  four-byte payload allocated 4 GiB before it could return an error. The speculative reservation is
  now capped at 1 MiB and the buffer grows from bytes that actually arrive. **Nothing about what is
  accepted changed.**

- **`<!DoCTYPE a>` lost a byte and changed case.** The writer wraps a doctype in the constant
  `<!DOCTYPE` … `>`, so a source spelling the keyword any other way could not come back —
  sixteen bytes in, fifteen out. `quick-xml` accepts spellings XML 1.0 §2.8 does not, and the reader
  now refuses a doctype it could not reproduce rather than silently rewriting it.

- **An element name that could not be written back is now refused.** `quick-xml` scans an element
  name up to whitespace, so `<a" b"c="1"/>` produced an element literally named `a"`. Untouched it
  round-tripped; *rewritten*, the writer put that name between `<` and `>` and emitted markup that
  will not parse. Names carrying a byte that would end a name or the tag around it are refused; names
  XML would reject but that re-serialize exactly (a leading digit, say) are still preserved, because
  fidelity is the tie-breaker in both directions.

Every one of these fixes **tightens** what the readers accept. None loosens anything: trading a crash
for a corruption is the one thing this project exists to prevent.

## [0.0.76] - 2026-09-03

The SpreadsheetML vocabulary is generated (MJXOFF-145).

`sml.xsd` is the largest schema in the set — 4,439 lines, 367 complex types, 96 simple types — and
nothing in `mjx-ooxml-types` covered any of it. MJXOFF-132 (`mjx-sml`) is built on this vocabulary,
so without it an Excel crate would have invented its own cell-type and error-value enumerations.
This adds it **whole**, not as an allowlist:

- **`mjx_ooxml_types::spreadsheetml`** — all 96 simple types of `sml.xsd`, carrying all 559
  enumeration values of its named types. Cell types, formula kinds, the 18 conditional-format rule
  kinds and 17 icon sets, the 66 PivotTable filters, the 28 table-style elements, border and
  pattern fills, data-validation kinds and IME modes, the MDX cube vocabulary, and the rest.

Every item documents its original `ST_*` symbol and its exact wire token, and 149 of the values are
named from the ECMA-376 prose rather than from their token — `s` is a shared string and `str` a
formula string, `3TrafficLights1` is `ThreeTrafficLights`, `gray125` is 12.5% grey, and `stdDevp`
is the population standard deviation as against `stdDev`'s sample estimate. The `wire` suite grows
26 → 61 → **94** tests: one per overridden `ST_*` pinning its named variants to exact bytes in both
directions, plus an exhaustive pass over all 559 tokens.

### Fixed

- **The simple-type reader lost a type when one nested another.** `xtask`'s XSD reader closed a
  named `xsd:simpleType` on the first `</xsd:simpleType>` it saw, so an `xsd:union` written with
  inline anonymous members — `sml.xsd`'s `ST_TextRotation`, the only one in the emitted set — closed
  its own definition early, swallowed the type declared after it, and attributed the inner
  restrictions' base and facets to the type around them. The reader now tracks nesting depth.
- **A union of one number is now that number.** `ST_TextRotation` (0–180 degrees, or 255) would
  have been a `String` newtype. A union every member of which resolves to the same Rust primitive
  is emitted as that primitive, the way a plain numeric restriction already was.

### Changed

- `assert_every_token_round_trips!` in the `wire` suite is now
  `assert_every_token_round_trips_to_its_own_variant!`, and asserts that the number of distinct
  variants an enumeration reaches equals the number of values its schema declares — the failure two
  colliding naming-override rows would cause. It covers `wml`, `shared-math` and `sml` alike.

## [0.0.75] - 2026-09-03

The WordprocessingML and Office Math vocabularies are generated (MJXOFF-144).

`mjx-ooxml-types` covered the shared common simple types, a curated slice of `dml-main` and a
curated slice of `pml`. Word and equations had nothing, so MJXOFF-90 (`mjx-docx`) and MJXOFF-134
(`mjx-omml`) would each have invented their own enumerations and the naming convention would have
fractured across two crates at once. This adds both vocabularies **whole**, not as an allowlist:

- **`mjx_ooxml_types::wordprocessingml`** — all 110 simple types of `wml.xsd`, carrying all 733
  enumeration values. Justification, underline kinds, the 193 border styles, shading patterns,
  section breaks, the 63 numbering formats, theme colours, text-flow direction, table-style
  overrides, the glossary-document galleries, and the rest.
- **`mjx_ooxml_types::officemath`** — all 14 simple types of `shared-math.xsd`, carrying all 30
  enumeration values.

Every item documents its original `ST_*` symbol and its exact wire token, and 183 of the values are
named from the ECMA-376 prose rather than from their token — `pct12` is 12.5%, `neCell` is the top
**right** table cell, `ideographZodiac` is the zodiac ideograph format, and `--`/`-+`/`+-` would
otherwise have collapsed onto one identifier. The `wire` suite round-trips all 763 tokens and pins
every one of those 183 names to its exact bytes in both directions.

### The naming tables are now per schema

An `ST_*` symbol is scoped to the schema that declares it, and OOXML reuses symbols: `ST_Jc` is
declared by both `wml.xsd` and `shared-math.xsd`, and `ST_Direction` by both `wml.xsd` (`ltr`/`rtl`)
and `pml.xsd` (`horz`/`vert`, already emitted as `Orientation`). One flat override table keyed on the
bare symbol cannot hold two meanings. `xtask`'s naming data is therefore partitioned the way the
symbols are — one `NameEngine` per emitted module — and the engine in `naming.rs` is unchanged: it
already took its tables by reference. Adding a schema still means growing the tables. The existing
`shared`, `drawingml` and `presentationml` output is byte-identical.

### The generator refuses names that would lose a token

Two `ST_*` types that reach one Rust type name, or two values of one enumeration that reach one Rust
variant, are now hard errors in `xtask` rather than Rust that compiles with a wire token nobody can
write back. So is a naming-override row that matched nothing — a misspelled symbol used to do
nothing at all, silently leaving the mechanical name it was written to replace.

### `COVERAGE.md` reports every schema

The generated manifest listed six schemas of the twenty-six in the Transitional set, and printed
`pending` for `wml`, `sml` and `shared-math` from a hard-coded string — so it would have kept saying
`pending` after the work was done. It now has a row for **every** schema in **both** tables, with
each status derived: the simple-type column from the generator's module table, the child-order column
from `CHILD_ORDER_SCHEMAS`. A pending row names the work item that owns it, and
`mjx-schema-gate`'s `the_declared_owners_agree_with_the_generated_coverage_document` fails if the
document and the gate's `OrderingCoverage::Pending` name different owners. A schema that is in
neither table and has no written reason fails the generator.

No `CHILD_ORDER_SCHEMAS` row was added: those belong to the children that start authoring the markup
(MJXOFF-90 for `wml`, MJXOFF-134 for `shared-math`, MJXOFF-132 for `sml`).

## [0.0.74] - 2026-09-03

A typed model's round trip no longer re-flows the part it came from (MJXOFF-143).

0.0.64 gave every parsed element the byte range it came from, so a serializer copies untouched
subtrees rather than rebuilding them. It stopped at the typed layer, and that left one limitation in
`fidelity_and_gaps.md`: **a model is a view**, so a `from_xml` / `to_xml` pass rebuilds every element
it looked at — including the ones nothing changed — and `*slot = value.to_xml(interner)` throws the
range of each of them away. Three surfaces read a whole part that way (`edit_vml_drawing`,
`edit_chart`, and the table-style list), and a dozen more read a single element that way. Editing one
word of a chart title re-flowed the whole chart.

Both halves of the loss were deliberate design rather than oversight, which is why this needed a
decision rather than a patch. `RawElement`'s `Clone` drops the range because a range means nothing
against another document's buffer, and cloning is how a subtree leaves the document that owns one.
`RawElement::new` records none because a newly authored element has no original.

### The design

**`RawElement::replace_preserving_verbatim_source`**, and `ToXml::write_back` over it. Instead of
assigning the rebuild over the original, the two are walked together in one pass, and a range is
moved onto a rebuilt node **only where that node compares equal to the one it replaces**. Two facts
discharge the burden of proof, and both are structural rather than remembered:

- *The bytes still describe the element.* `RawElement`'s `PartialEq` compares name, self-closing
  style, attributes in order with their quoting, and, recursively, children — precisely the
  properties an element's markup determines. So "equal" **is** "these bytes spell this element".
- *The buffer is the right one.* The range comes from the element being overwritten and lands on its
  replacement at that same position, so the destination document is by construction the one that
  measured it. A caller cannot pair an original from one document with a rebuild bound for another,
  because the original *is* the destination.

The three candidates it was chosen over: a `clone_within_document` on `RawElement` would give ranges
back only to the markup a model does *not* understand — everything it does model is rebuilt after the
clone, so a wrapped `v:shape` would still re-flow — and its soundness ("only while the clone stays in
this document") is a convention no type can check. `FromXml` taking its element by value moves the
content instead of cloning it, but breaks every implementor and every call site while giving the same
partial answer, and the read-only surfaces (`with_chart`, `with_vml_drawing`) hold a shared reference
and could not give ownership at all. A retained element plus a dirty flag reaches everything, but the
flag must be cleared by every mutator in three crates, and one missed mutator writes the wrong bytes
— the single failure mode this design exists to make impossible. Here there is no flag to forget:
cleanliness is *computed*, against the element still sitting in the document.

`mjx-xml`'s writer is unchanged and still checks every range before trusting it — it must fit, open
with `<` plus the element's qualified name, and close the way `empty` says it closes — so a range
that reached it wrongly degrades to a re-flow rather than to wrong bytes. `RawElement` does not grow:
the eight-byte budget test is untouched.

### What changed for callers

- `ToXml` gains a **provided** method, `write_back`, so no implementor changes. Every whole-part and
  sub-element edit surface in `mjx-pptx` now goes through it — 19 call sites.
- `mjx_vml::DrawingPart` keeps the `RawDocument` it parsed instead of scattering its pieces, because
  a standalone part has no document to write back into otherwise. It costs the parsed tree alongside
  the typed model; a caller that already owns the part's `RawDocument` (through
  `mjx_opc::Package::part_tree_mut`) should use `Drawing::from_xml` and `write_back` directly and pay
  nothing.
- The *Limitations* table in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` is gone — it had one
  row and this was it. The row is recorded under "What used to be here", so a reader can tell "gone"
  from "quietly dropped".

`crates/mjx-vml/tests/drawing.rs`'s `attributes_wrapped_across_lines_reflow_when_the_part_is_re_serialized`
asserted the re-flow *happened* and instructed its reader to replace it with byte identity the moment
it stopped. It has stopped. Every new case is written against a fixture whose start tags are wrapped
across lines — `vmlDrawing1.vml`'s with CRLF — because a part that is already on one line
reconstructs to its own bytes and would pass with the mechanism deleted.

Tests 1,676 → 1,690 default and 1,690 → 1,705 with `--all-features`.

## [0.0.73] - 2026-09-03

`mjx-dml`'s composite tiers on the attribute grammar — geometry, tables, text and the colour
resolver (MJXOFF-142).

0.0.72 put the seven shared property tiers on the grammar. This completes the crate: **the 107
remaining `attr_*` call sites across `geometry/`, `table/`, `text/` and `resolve.rs` became 0**, and
with them every `dml_attr`, `prefixed_attr`, `push_*`, `set_attr`, `angle_to_wire`,
`parse_percentage` and `parse_angle`. `crates/mjx-dml/src/build.rs` no longer mentions attributes at
all: what is left there builds and finds *elements*.

**There is one path from a wire attribute to a typed value in `mjx-dml`, and one back**, and both go
through `mjx_xml::attribute::{read, write}`. A helper family with two callers left is the
half-migrated family CI's naming check exists to warn about, so the family is gone rather than
reduced.

### Two shapes, one grammar

The tiers here have the same split 0.0.72 found: some types retain an attribute vector and declare
on themselves; most sites are *value projections* over elements the crate has no type for — an
`a:pt`, an `a:arcTo`, an `a:tab`, an `a:buChar`, an `a:hlinkClick`, a colour transform's `@val` —
and declare on a generic attribute face reached through `AsRef<[RawAttribute]>` / `AsMut<Vec<..>>`.
Twenty-two such faces are declared here.

Two attributes are declared as `Text` rather than as an `Enumeration<T>` *deliberately*:
`a:prstGeom@prst` and `a:cell3D@prstMaterial` each expose **both** readings of the same bytes — the
typed one and the raw token — which is what lets a shape kind or a material this build does not know
still be named. The typed reading layers the generated enumeration's own `from_wire` over the one
read, so the token → enum mapping still has one implementation.

### New

- **`mjx_dml::codec`** gains five: `TextFontSize` (`ST_TextFontSize`), `TextPointSize`
  (`ST_TextPoint`), `TextIndentLevel` (`ST_TextIndentLevelType`, whose `0..=8` range is enforced),
  `PercentageWithPercentSign` (the `111%` spelling `a:buSzPct@val` is written in), and
  `EmuOrGuideName` / `AngleOrGuideName` for the two `ST_Adj*` unions custom geometry places points
  with.
- **`crates/mjx-dml/tests/in_context_roundtrip.rs`** grows from 16 cases to 28: a table lifted out
  of `tables.pptx` and `table_extensions.pptx` and asserted **at the outermost container**, so a
  cell's attributes must survive being rebuilt as part of a row rebuilt as part of a table; a
  paragraph-level body and a preset geometry out of `text_levels.pptx`; a transform read and written
  back; and five hand-written literals in forms this project's writer never emits, for the run
  properties, paragraph properties, table, custom geometry and transform tiers.

### Changed behaviour: a boolean a setter writes is spelled `true`

`TableProperties::set_part` and `TableCell::set_merged` wrote `1`; they now write `true`, the one
canonical `ST_OnOff` spelling every other boolean in the workspace is written in, because they go
through the same `OnOff` codec. Reading is unchanged and still accepts all six spellings, and an
attribute **nobody assigns to keeps its own spelling** — a file that says `firstRow="1"` still says
`firstRow="1"` after an unrelated edit. (`mjx_pptx`'s `add_table` builds a fresh `a:tblPr` from a
literal template and still writes `firstRow="1" bandRow="1"`, as PowerPoint does.)

### Breaking changes

As in 0.0.72: an accessor over a declared attribute reports a malformed value instead of silently
reading `None`, so it returns `Result<Option<T>, AttributeError>` — or `Result<T, AttributeError>`
where the attribute is `use="required"` — and a text-valued one returns a `Cow<str>` (entity
references in the file are decoded) where it returned `&str`.

| Was | Is |
|-----|----|
| `AdjustPoint::{x, y}` → `Option<AdjustCoordinate>` | `Result<AdjustCoordinate, AttributeError>` |
| `Path2D::{width, height, fill, stroke, extrusion_ok}` → `Option<T>` | `Result<Option<T>, _>` |
| `GeometryGuide::{name, formula}` → `Option<&str>` | `Result<Cow<str>, _>` |
| `PresetGeometry::preset_token` → `Option<&str>` | `Result<Cow<str>, _>` |
| `TableColumn::width`, `TableRow::height` → `Option<Emu>` | `Result<Option<Emu>, _>` |
| `TableColumn::set_width`, `TableRow::set_height` took `Emu` | take `Option<Emu>` (`None` removes) |
| `TableCellProperties`'s eight accessors → `Option<T>` | `Result<Option<T>, _>` |
| `TableCellProperties::{set_anchor, set_text_direction, set_horizontal_overflow}` took a value | take an `Option` |
| `TableCell::id`, `TextField::{id, field_type}` → `Option<&str>` | `Result<Option<Cow<str>>, _>` |
| `TableStyleList::default_style_id`, `TableStyle::{style_id, style_name}` → `Option<&str>` | `Result<Cow<str>, _>` |
| `FontReference::index` → `Option<FontCollectionIndex>` | `Result<Option<..>, _>` |
| `Cell3D::preset_material` → `Option<&str>` | `Result<Option<Cow<str>>, _>` |
| `CharacterProperties`'s ten attribute accessors → `Option<T>` | `Result<Option<T>, _>` |
| `CharacterProperties::{hyperlink_rel_id, hyperlink_action}`, `TextRun::hyperlink_rel_id` → `Option<&str>` | `Option<String>` |
| `ParagraphProperties`'s eight attribute accessors → `Option<T>` | `Result<Option<T>, _>` |
| `ResolvedGuides::define` took `&'a str` | takes `impl Into<Cow<'a, str>>` |

`Transform2D::read`, `CustomGeometry`'s spec readers, `TableCell::{column_span, row_span,
merged_horizontally, merged_vertically}`, `TableProperties::part`, `TableStyleTextStyle::{bold,
italic}`, `PresetGeometry::preset`, `Cell3D::material` and every `resolve_*` function keep their
shape: each is a **total** projection with a documented answer for "the file does not say", and a
value this model cannot read is the file not saying. That decision is written down in
`resolve.rs`'s module docs, where it is load-bearing — a renderer that refused to draw a shape over
one malformed colour transform would be worse than one that drew it without the transform.

## [0.0.72] - 2026-09-03

`mjx-dml`'s shared property tiers on the attribute grammar — colour, fill, outline, effects, 3-D,
theme and style (MJXOFF-141).

MJXOFF-140 proved the `#[xml(attribute(..))]` grammar on a synthetic type inside `mjx-derive`'s own
tests. This release is the first time anything shipped uses it: the seven files every other
DrawingML tier reaches through no longer parse an attribute by hand. **86 calls to the `attr_*`
family became 0 in those files**, and the four hand-written measure readers and writers they were
the last users of are deleted.

### There is now exactly one path from a wire attribute to a typed value

`mjx_xml::attribute::read` and `mjx_xml::attribute::write` are that path — find, decode, hand to a
codec; encode, escape for the quote in use, set or remove. Every accessor
`#[derive(XmlAttributes)]` generates is one call to one of them, `mjx-dml`'s remaining `attr_*` /
`push_*` helpers (which the tiers MJXOFF-142 owns still use) are one call to one of them, and a model
reading an element it has no type for calls them directly. Two implementations of "attribute to
value" is the duplicate this workstream exists to prevent; there is one.

### A declaration no longer requires a type that owns its attributes

`#[derive(XmlAttributes)]` reaches the vector through `AsRef<[RawAttribute]>` to read and
`AsMut<Vec<RawAttribute>>` to write, so the `attributes` field may be a `Vec`, a `&[RawAttribute]`
view (getters only — the bound that would give it setters is simply not satisfied), a
`&mut Vec<RawAttribute>` cursor, or generic over all of them.

That last form is what `mjx-dml`'s **value projections** use. An effect, a bevel, a camera, a
gradient stop, a line end and a blip are facts read out of an element the crate does not model as a
type; a conduit generic over its attribute container declares them once and serves both directions —
`{ attributes: &element.attributes }` to read, which copies nothing, and `{ attributes: Vec::new() }`
to write the vector the new element will own.

### New

- **`mjx_dml::codec`** — `EmuCoordinate`, `EmuLineWidth`, `SixtyThousandthsOfADegree` and
  `Percentage`, the four measure codecs. A crate that owns a measure owns its codec.
- **`mjx_ooxml_core::Number<T>`** — an alias for `Enumeration<T>`, so a numeric attribute is declared
  `codec = Number<u32>` rather than claiming an integer is an enumeration.
- **`RawElement::rebuilt`** — the single construction point every `ToXml` now goes through. Identical
  to `new` today; it exists so that carrying a source range through a typed round trip (MJXOFF-143)
  is one edit rather than one per `to_xml` in the workspace.
- **`crates/mjx-dml/tests/in_context_roundtrip.rs`** — the generalised in-context harness (was
  `txbody_roundtrip.rs`), now covering seven types out of real parts plus a corpus of hand-written
  literals in forms this project's writer never emits, and both tier-3 isolation cases.

### Breaking changes

An accessor over a declared attribute reports a malformed value instead of silently reading `None`,
so several `mjx-dml` accessors return `Result<Option<T>, AttributeError>` where they returned
`Option<T>`, and the text-valued ones return a `Cow<str>` (entity references in the file are decoded)
where they returned `&str`. The affected methods are `Color::{value, hex}`,
`LineProperties::{width, cap, compound, pen_alignment}`, `GradientFill::{flip, rot_with_shape}`,
`PatternFill::preset` and `Shape3D::{z, extrusion_height, contour_width, material}`.
`PictureFill::{image_rel_id, image_link_id}` return `Option<String>`: they read through the blip's
attribute face, which does not outlive the call, and both callers copied the id anyway. The value
tiers (`LineSpec`, `Shape3DSpec`, every effect) are unchanged — a spec is a value description and
still drops what it cannot represent.

## [0.0.71] - 2026-09-03

An attribute grammar for `mjx-derive` — accessors over the retained attribute vector, not a lifting
form (MJXOFF-140).

`mjx-derive` modeled elements, children and text; **attributes it did not model at all.** Every typed
type in the workspace reached into its own `Vec<RawAttribute>` and parsed the value by hand.
DrawingML survived that because Phase A grew it a tier at a time, but `wml.xsd` has 110 simple types
and `sml.xsd` 96, both far more attribute-dense than `pml`, and hand-parsing each one across two new
format crates is where the `ST_OnOff` spellings would quietly go wrong.

Nothing shipped changes. No emitted byte moves; this release adds a way to declare what a hand-written
accessor already does, and the additions are new items beside the existing ones.

### `#[derive(XmlAttributes)]`

A third derive, independent of `FromXml` / `ToXml` and composing with them, with a hand-written pair
of impls, or with neither. It asks only for the retained `attributes: Vec<RawAttribute>` field and
generates **one getter and one setter per declared attribute** over that vector:

```rust
#[derive(FromXml, ToXml, XmlAttributes)]
#[xml(attribute(local = "val", codec = HexColorRgb, accessor = color, required))]
#[xml(attribute(local = "rtlCol", codec = OnOff, default = false))]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
#[xml(attribute(local = "embed", prefix = "r", codec = Text, accessor = image_relationship))]
struct SolidColor { /* .. */ }
```

`local` and `codec` are required; `prefix` matches and writes a prefixed attribute; `accessor` names
the Rust method (the default is the wire name in snake case, which the naming convention will usually
want overriding); `required` makes an absent attribute a typed error and `default` gives it a schema
default. Writing neither makes it optional — the third case, whose getter returns `Option`.

**The accessor form is the point.** A grammar that lifted attributes into struct fields would make
the writer *reconstruct* the attribute list, and reconstruction is how unknown attributes, their
order, their prefixes and their quote characters get lost. Nothing in the generated code builds an
attribute list: a getter borrows the vector, a setter reaches exactly one element of it.

### Read never normalizes; a write does

A getter takes `&self`, so it cannot change the file: `rtlCol='on'` that nobody assigned to still
writes `on`, single-quoted, in the position it was read from, and `val='50%'` stays `50%`. The one
canonical form is written only by a setter — `set_rtl_col(true)` writes `true` — which rewrites the
attribute **in place**, keeping its position and the quote character the file used, and escaping the
new value for *that* quote. An attribute that was not there is appended, double-quoted.

A grammar that canonicalized on read would rewrite every file it opened, and would do it invisibly,
because our reader and our writer would agree with each other.

### The codecs

`mjx_ooxml_core::AttributeCodec` is the wire ⇄ Rust conversion for one *kind* of value — a type-level
tag, never constructed. `mjx-ooxml-core` ships the XML-generic ones (`Text`, `Enumeration<T>`, which
covers every generated `ST_*` enumeration because they all spell themselves with `FromStr` +
`Display`); `mjx-ooxml-types` ships the OOXML-specific ones (`OnOff`, `TrueFalse`, `TrueFalseBlank`,
`HexColorRgb`), consuming the `support` normalizers rather than re-deriving them. A crate that owns a
measure type owns its codec, in about fifteen lines — which is how `mjx-dml` will carry `Emu` and
`Fraction` across the seam.

A malformed value is `AttributeError`, never a panic: these are attacker-controlled files.

### Also

`mjx_xml::attribute` — `find`, `decoded_value`, `set`, `remove`: the four in-place operations a typed
accessor is made of, usable by hand. `mjx_xml::text::escape_attribute_in` escapes for a given quote
character (`'` → `&apos;`), which is what lets a setter keep a single-quoted attribute single-quoted
without being able to emit `attr='it's'`. `FromXmlError` gains an `Attribute` variant.

## [0.0.70] - 2026-09-03

The gates reach Word and Excel — before a line of `wml` or `sml` model code exists (MJXOFF-110).

`sample.docx` and `sample.xlsx` have been in `tests/fixtures/` since the first phase and **nothing
had ever schema-validated either of them.** A `w:` part with no arm in the schema table was reported
"skipped, foreign namespace"; the suite counted the remaining parts, found four of them valid, and
reported green. The sentence "the schema gate covers Word" was true and empty at the same time. So
were the ordering half (`assert_deck_is_in_schema_order` asserted only that *some* part had been
audited, and `word/theme/theme1.xml` satisfied it) and the byte-identity half (three suites carried
hand-maintained fixture lists that between them omitted six of the fifteen committed fixtures).

Nothing shipped changes. Every byte this library writes is identical before and after; this release
is test and CI infrastructure, and the round-trip suites did not move.

### The harness is a crate

`crates/mjx-schema-gate` is a new **test-only** crate (`publish = false`, a `dev-dependency` of
`mjx-pptx`, `mjx-docx` and `mjx-xlsx` and of nothing else). An integration test compiles only into
its own crate, so the harness that lived in `mjx-pptx/tests/schema_validity.rs` could never be
reached from the two crates Phases C and D will fill. `crates/mjx-fixtures` is a second, entirely
dependency-free test-only crate holding the committed corpus, so `mjx-opc`'s byte-identity suites —
which sit *below* the gate in the layering — can read the same corpus without an upward edge.

### The three-category rule, with no fourth branch

`mjx_schema_gate::categories` is the only place the line is drawn. Markup we model is **validated**
against its XSD; foreign markup we only preserve (VML, InkML, ActiveX, and the two `docProps`
streams) is **skipped with a written reason**; a root element in a namespace on neither list is a
**hard failure naming the namespace and the part**. There is no "skip anything we have no arm for"
fallback, because that fallback is the hole being closed.

`WordprocessingML` joins the table, so `sample.docx`'s `word/document.xml`, `word/styles.xml`,
`word/fontTable.xml` and `word/settings.xml` are validated against `wml.xsd` for the first time.

### `wml.xsd` can now be compiled at all

`wml.xsd:21` and `shared-math.xsd:13` import `http://www.w3.org/XML/1998/namespace` with no
`schemaLocation`, and the Transitional set ships no `xml.xsd`, so libxml2 could not resolve
`xml:space` and both schemas failed to *compile*. A bare import gives libxml2 no URI, so a catalog
has nothing to rewrite. `crates/mjx-schema-gate/schemas/xml.xsd` — hand-written for this repository,
no third-party licence, nothing fetched at build time — is paired with each XSD through a generated
driver schema. Every validation goes through one, so `shared-math.xsd` inherits the fix.

### Markup compatibility is resolved, not skipped

A part carrying `mc:AlternateContent` or `mc:Ignorable` used to be skipped, which is why LibreOffice's
`word/document.xml` could never be reached. The gate now resolves it with the existing `mjx-mce`
crate — the winning `mc:Choice` selected, ignorable markup in namespaces ECMA-376 does not define
dropped — and validates that view. Only parts that actually carry markup compatibility are
re-serialized; every other part is validated as the exact bytes the package holds.

### Two pre-existing divergences in `sample.xlsx`, now recorded

LibreOffice writes `xml:space="preserve"` on every `s:t` (which `sml.xsd` types as a simple type that
can carry no attribute) and `dateCompatibility` on `s:workbookPr` (not in the 5th-edition
Transitional schema). Both are inputs this project preserves verbatim, so both are recorded as
tolerated deviations with their reasons, matched error-by-error: a *new* defect in either part still
fails.

### The corpus is the directory

`crates/mjx-opc/tests/{roundtrip,tree_roundtrip,package_validation}.rs` and the schema gate all read
`tests/fixtures/` instead of a list. All fifteen fixtures are now inside all four contracts; a file
whose extension is on no list fails, naming it.

### CI

A new required `test (--all-features)` job runs `cargo clippy --workspace --all-targets
--all-features` and `cargo test --workspace --all-features --no-fail-fast`; the existing test steps
gain `--no-fail-fast`; the `schema-validity` job runs the Word and Excel gates beside the PowerPoint
one. `.github/scripts/merge-when-checks-pass.sh` makes the merge step a command whose exit status
gates the merge rather than a sentence instructing a person to look at one.

## [0.0.69] - 2026-09-03

The chart `delete_*` family is `suppress_*` — the naming question v0.0.66's API review raised and
left open, settled before Word and Excel copy the shape (MJXOFF-89).

Twelve public identifiers change across `mjx-chart`, `mjx-pptx` and `mjx-ooxml`, and three of them
are re-projected by each binding. Three `is_deleted` accessors (`DataLabel`, `DataLabels`, `Axis`)
and two `deleted` fields (`DataLabelSettings`, `ChartAxisData`) become `is_suppressed` /
`suppressed`; `delete_chart_data_labels` (on both `Presentation` and `Deck`),
`delete_plot_data_labels`, `delete_data_labels`, `delete_point_label` and `delete_label_for_point`
take a `suppress_` prefix; and `auto_title_deleted` becomes `auto_title_suppressed`. The crate-private
`DataLabels::delete_all` and the two private `clear_delete` helpers move with them. Python sees the
same names (the binding is the identity mapping); TypeScript sees `suppressChartDataLabels` and a
`suppressed` getter in place of `deleteChartDataLabels` and `deleted`.

Nothing else changes. This is a rename: the bytes written for any file are identical before and
after, and no test was added, removed or skipped.

The spelling is now enforced rather than remembered. `.github/scripts/check-suppress-naming.sh` — a
new `naming` job in CI, plus a step in `wasm-pack` over the generated `.d.ts` — fails the build if
any identifier under `crates/*/src`, `bindings/*/src`, the committed `.pyi` or the generated
TypeScript declarations spells this concept `delete`. The wire token is untouched and explicitly
permitted: `flag("delete")`, `"autoTitleDeleted"`, `c:delete` in prose, and the generated ordering
tables in `mjx-ooxml-types` all pass, and each item's docs still name the exact element it writes.

## [0.0.68] - 2026-09-02

Python (PyO3) and WebAssembly/TypeScript (wasm-bindgen) bindings — the facade, projected whole
(MJX-210).

`mjx-ooxml` has been "the binding-ready public API" since v0.0.67. Nothing was bound to it. Two
workspace members now are, and both project the **whole** surface rather than a sample of it:

- **`bindings/mjx-python`** — PyO3, module `mjx_ooxml`, abi3-py39 wheels. 253 methods on `Deck`
  (257 with the `vml` feature), 192 classes, a committed `.pyi` plus `py.typed`, and an exception
  hierarchy of eleven classes rooted at `OoxmlError` — each carrying `.code` and the coordinates
  `.surface`, `.shape`, `.row`, `.column`, `.index`, with `IndexOutOfRangeError` also an
  `IndexError`. The mapping is the **identity**: nothing is renamed except the `None` member of nine
  enumerations, which Python's grammar will not permit.
- **`bindings/mjx-wasm`** — wasm-bindgen, one npm package with conditional exports for a bundler
  build and a browser build. The same surface in **camelCase**, `Uint8Array` in and out, and
  failures as real `Error` objects with `name === "OoxmlError"`, a stable `code` and a `detail`
  object. `deck.free()` is mandatory and the documentation says so in every place a reader might
  look.

### The acceptance test

`crates/mjx-ooxml/examples/build_a_deck.rs` — the guide's whole walkthrough — now exists three
times: once in Rust, once as `test_build_a_deck.py`, once as `build_a_deck.mjs`. Each of the two
bindings runs the Rust one and compares its own deck **part by part, byte for byte**. That is what
proves the curated subset is sufficient, and it is what would catch a method wired to the wrong
`Deck` method: nothing about the types would complain, and one part payload would differ.

### Added to `mjx-ooxml`

Ten types were reachable from the re-exported vocabulary but not themselves re-exported, so a
binding could hold a value it could not name: `AdjustHandle`, `ConnectionSite`, `ColorKind`,
`FontSlot`, `TableStyleBorder`, `ThemeFontReference`, `GuideFormulaError`, `AxisKind`, and
`AdjustmentSpec` with `AdjustmentAxis` / `AdjustmentBound`. `tests/vocabulary_closure.rs` uses all
ten through the facade, so the list stays closed.

`PartialEq` was added to `mjx_pptx::TableStyleFormat`, `mjx_pptx::TableStyleDefinition` and
`mjx_chart::ChartData`, whose siblings all had it; and the two terse "Delegates to …" doc summaries
on `Deck::set_cell_run_properties` / `set_cell_text_range_properties` were written out, because the
bindings use those summaries as their docstrings.

### The recorded divergence

`PLAN.md`, `README.md` and `CLAUDE.md` said bindings would live in a **separate cargo project** on a
**UniFFI → wasm → C-ABI** stack targeting Kotlin, Swift, JavaScript and C, deferred to Phase 7. They
do not, and it is not. All three files now say what was built and why — see the "Recorded
divergence" section of `PLAN.md`.

### `unsafe`

The two binding crates are the first in this workspace to carry `#![allow(unsafe_code)]`, and the
first use of the `deny`-not-`forbid` escape hatch the workspace lints were written to permit. The
justification is that no `unsafe` is hand-written: every unsafe block is generated by `#[pyclass]`
or `#[wasm_bindgen]`. CI greps `bindings/*/src` and `bindings/*/tests` for `unsafe` outside a
comment and fails if it finds any, so the justification cannot quietly become false.

### Measured, not gated

The WebAssembly payload is **2,484,641 bytes raw and 848,380 gzipped (828 KiB)** with `lto = true`,
`codegen-units = 1`, `strip = "debuginfo"` and `wasm-opt -Oz`. The specification estimated
400–700 KB and asked for a measurement before a budget; this is the measurement, and CI reports it
on every run rather than failing on a number nobody has justified yet. `panic = "unwind"` is kept
workspace-wide because PyO3 needs it to turn a panic into a Python exception rather than a process
abort.

### CI

Three new jobs — `bindings-build` (the `unsafe` check, both crates built the way they ship),
`wasm-pack` (headless Chrome, Node, both npm targets, the size report) and `python-wheel` (abi3
wheels on Linux, macOS and Windows, installed from the wheel, then `pytest` and `mypy --strict`, and
the same wheel re-checked on a much later interpreter). The cross-build matrix excludes both binding
members: a PyO3 `cdylib` needs a host interpreter and a wasm `cdylib` means nothing off `wasm32`.
The `examples` job now runs every crate's examples, not only `mjx-pptx`'s.

## [0.0.67] - 2026-09-02

The `mjx-ooxml` facade — `detect_format`, `Deck`, FFI-shaped errors, the curated surface (MJX-210).

`mjx-ooxml` had been **62 lines of documentation and no code** since the workspace was laid out,
while the docs called it "the binding-ready public API". It is now that API.

**`detect_format` reads the package, not the filename.** It opens the OPC container, follows the root
`officeDocument` relationship and maps the main part's content type against the fifteen ECMA-376 and
macro-enabled types. That is the only way `.pptm` and `.potx` — the same PresentationML markup under a
different declaration — can be told from `.pptx`, and the only answer that survives a renamed file.
Word and Excel are recognized and refused by name (`ErrorCode::UnsupportedFormat`), so a caller who
hands a `.docx` to a PowerPoint library is told it is a Word document rather than that some part
failed to parse. Detection working before editing does is the whole point.

**`Deck` restates 251 of `Presentation`'s 273 methods in types a foreign function boundary can
express**: `impl Into<Surface>` becomes a concrete `Surface`, `impl Into<ShapePath>` a concrete
`ShapePath`, `usize` becomes `u32` on every parameter and return, `&PartName` becomes `&str`, and a
borrowed `Option<&[u8]>` becomes an owned `Option<Vec<u8>>`. The facade owns its own `Surface` and
`ShapePath` — carrying `u32`, converting at the boundary, allocation-free for the top-level case —
because adding `From<u32>` beside `From<usize>` on `mjx-pptx`'s would have made every bare integer
literal in `deck.shape_fill(0, 2)` ambiguous across the workspace.

Sixteen methods are deliberately absent, each unreachable across FFI or reachable another way:
`Presentation::shape` (returns a cursor borrowing the deck), the five closure-taking table-style and
VML readers, the four surface `*_part` accessors and the six `*_rel_id` accessors (part-graph
identity for content that is already reachable by index or by bytes). `Deck::presentation_mut` is the
Rust-only door to all of them; there is no `Deck::package`, because handing out `&mut Package` would
give a caller the whole part graph and make every invariant `save` enforces unenforceable.

**One exclusion the specification proposed was checked and reversed.** The per-cell formatting
setters were to be dropped as reachable through `format_cells(Cells, &CellFormat)`. They are not:
`format_cells` deliberately skips a cell covered by a merge, so only what renders is touched, while
`set_cell_fill` reaches a covered cell — whose own formatting reappears when the region is unmerged.
Dropping them would have dropped that, so all fifteen are exposed, and a test asserts the two
spellings really are different calls.

**One `Error`, eleven stable codes.** `Error { code, message, detail, source }` collapses all 65
`PptxError` variants and all 9 `OpcError` variants into `Io`, `MalformedDocument`, `InvalidDocument`,
`IndexOutOfRange`, `WrongKind`, `NotFound`, `NothingToRead`, `InvalidArgument`, `StructureConflict`,
`UnsupportedContent` and `UnsupportedFormat`, plus the human message and the `surface` / `shape` /
`row` / `column` / `index` coordinates a binding turns into exception attributes. Rust callers lose
nothing: `source()` downcasts back to the `PptxError`. The classification is an exhaustive `match`
with no wildcard arm — which is why `PptxError` stopped being `#[non_exhaustive]` — so a new variant
fails the build until it is classified.

**`Deck::save` inherits the validation `Presentation::save` performs** rather than routing around it.
A facade that widened what a caller could break would be a regression, so this is tested on a deck
that is genuinely invalid: `save` refuses it and `save_unchecked` writes it.

Also here: `Presentation::{remove_unused_parts, external_links, retarget_external_link}` — package
hygiene as three thin delegates rather than an exposed `package()` — and `SlideSize::{widescreen,
standard, from_emu}`, so a caller building a deck from nothing states a size by name instead of by
struct literal.

`examples/build_a_deck.rs` is the guide's walkthrough written through the facade, **naming no crate
below `mjx-ooxml`**; if the re-export list were insufficient it would not compile.

## [0.0.66] - 2026-09-02

API review and reorganisation — the last iteration before `v0.1` freezes the surface (MJX-37).

`crates/mjx-pptx/src/presentation.rs` had reached **12,771 lines and 266 public methods** in a single
`impl Presentation` block. It is the file the whole PowerPoint surface lives in, and the file
`mjx-docx` and `mjx-xlsx` will copy on day one of Phases C and D, so its shape is worth more than its
size suggests.

**The split changes no path a caller imports.** `presentation/` is sixteen modules along the seams
the guide already reads in — deck addressing, slide lifecycle, the shape tree, notes, text,
hyperlinks, table cells, table structure, bounds, appearance, the effective readers, charts, chart
decoration, pictures, legacy content, and the element builders shared by more than one of them. Every
method stays an inherent method on the one re-exported `Presentation`; the helpers that moved out are
`pub(super)`, visible only inside `presentation`. The public-item lines before and after are identical
as a set, and the workspace suite was unchanged at 1,528 passing tests across the move — which is what
a reorganisation is supposed to look like. It is committed separately from every behaviour change so a
reviewer can see that the move moved nothing.

`tests/public_paths.rs` guards that from outside the crate: one authored deck driven through every
seam using only `mjx_pptx::` paths, because a test that reached into `crate::presentation::text` would
prove nothing a caller can rely on.

**The three named inconsistencies are settled.** `cell_span` answers `(rows, columns)` like
everything else on the table surface. The eight DrawingML effects each take what the schema makes
required in `new` and name the rest with `with_` — so a shadow's distance no longer costs eight
`None`s — while an attribute the builder does not name stays unset, and an unset attribute is not
written. And the three `#[allow(clippy::too_many_arguments)]` sites, re-examined now that `Cells` and
`CellFormat` exist: one was dead and is gone, and the two that remain — eight distinct cell
coordinates apiece — are `#[expect]` with their reason, so the day the list fits, the attribute fails
the build instead of quietly outliving its cause.

**A loop in a doc example is a design defect.** Three remained across the guide, the README-adjacent
pages and the examples, all the same shape: `for i in 0..count()` with a fallible accessor inside,
rebuilding a list the deck already has and re-borrowing the part once per entry.
`Presentation::layouts` answers the layout inventory as `Vec<LayoutInfo>`, `Presentation::shapes`
answers a surface's shapes as `Vec<ShapeInfo>` — index, kind, and the placeholder slot each fills —
in one read, and `Presentation::shape_for_placeholder` answers the search *where did this template put
the title?*. Every loop still standing in a doc example iterates a collection the API handed over,
with no `?` inside it.

**`Presentation::from_package` is public**, as the facade needs: the constructor for a caller who
already holds the package — one `mjx-opc` opened directly, or one a facade opened once and dispatched
on by content type rather than handing the bytes back to each format crate to re-open.
`mjx_opc::Package` is re-exported from `mjx-pptx` alongside it, on the same reasoning the chart types
already are: a caller should not have to name another crate to state a parameter type.

**The naming sweep** covered all 1,561 public identifiers of the eleven merged children. Its nine
breaks are tabulated under **Unreleased — 0.1.0** above; the summary is that `blip` is not a word,
that an abbreviation named after an attribute is still an abbreviation, and that a struct a caller
reads and the struct it writes back should name the same field the same way.

Fidelity is unchanged and was the acceptance criterion throughout: per-part byte identity, modeled
round-trips, and edit isolation all hold, `MJX_REQUIRE_SCHEMA=1` passes 51 (52 with `--features
vml`), and all eight examples verify their own output.

## [0.0.65] - 2026-09-02

Chart decoration — data labels, per-point formatting, trendlines and error bars (MJX-116).

`c:dLbls`, `c:dLbl`, `c:dPt`, `c:trendline` and `c:errBars` were preserved verbatim and had no typed
surface at all. A5 closed the chart *data* half completely — every plot type's series, literal and
multi-level sources, axes, gridlines, titles, legend and series fill/outline — and stopped at the
decoration deliberately rather than half-modelling it. That was right for its scope; leaving it
unowned was not. **Data labels are the part of a chart a reader actually reads**, and until this
release a caller could not ask what one said, could not switch a series from value to percentage, and
could not author a chart that labelled itself.

All four families now **read, author and edit**. `crates/mjx-chart/src/decoration.rs` adds
`DataLabels`, `DataLabel`, `DataPointFormat`, `Trendline` and `ErrorBars`, each with the same
ordered-`content` + `Raw` shape as everything else in the crate, so an element nothing touched still
re-emits byte-for-byte. `c:plus` and `c:minus` are the same `CT_NumDataSource` a series' `c:val` is,
so a custom error bar's lengths read and write through the existing `NumericData`.

**The three tiers of a data label.** ECMA-376 §21.2.2.49 says `c:dLbls` states the settings "for an
entire series **or the entire chart**", and a `c:dLbl` overrides them for one point — so a label
resolves over three tiers, and `DataLabelSettings::inherit` merges them **per setting**, not per
tier: a series that only says `c:showVal` still takes its plot's `c:dLblPos`. A `c:delete`
short-circuits the chain, because `CT_DLbls` puts it in one `xsd:choice` with the settings group and
an element carrying one cannot carry the other. There is deliberately no fourth tier — `CT_Chart`
declares no `c:dLbls` of its own — and the model says so rather than inventing one.
`ChartLabelScope` names the three tiers on the `mjx-pptx` surface, so "label this series" and "label
this point" cannot be the same call, and three verbs separate three intentions: *state settings*,
*draw nothing here* (`c:delete`), and *say nothing here* (remove the element, inherit again).

**A `c:dPt`'s `c:idx` is never renumbered.** Per-point formatting is anchored by index into the
series; renumbering one when a series changes length would move a point's colour silently onto a
different point, which is worse than leaving it dangling. Nothing in this release rewrites an index
except an explicit `set_index`. `Series::decoration_beyond_data` and
`Presentation::chart_dangling_decoration` *report* the anchors an edit left past the end;
`drop_chart_dangling_decoration` removes them, and nothing removes them on a caller's behalf. A
`c:idx` that is not a number — `-1`, or a value past `u32::MAX` — addresses no point, is never
matched by a lookup, is never renumbered, and rides through a round-trip untouched. Writing past the
end is the same rule from the other side: it is refused with a typed error rather than written as an
anchor that names nothing.

**Writing is bound to the owning plot's kind, and both the placement and the refusal come from the
schema.** `SeriesDecoration` carries a `ChartKind` because `CT_BarSer` puts `c:dPt` at rank 6 and
`CT_PieSer` at rank 5, and because `CT_PieSer` declares no `c:trendline` and no `c:errBars` while
`CT_SurfaceSer` declares no decoration at all. Both questions are asked of the generated
`child_order` tables, which gain 29 named constants for this — the five decoration types, the eight
`CT_*Ser` and the sixteen `CT_*Chart` — rather than of a list written by hand. `ChartDataError` gains
seven variants, every one raised **before anything is written**, the way `ChartData::validate`
already refused a shape the schema rejects: a point index past the end of a series, a decoration the
series type does not declare, leader lines on one point's label (only `Group_DLbls` declares them),
an `ST_Order` outside 2–6, an `ST_Period` below 2, a non-finite measure, and custom error bars whose
length nothing determines.

Every name is sourced from the ECMA-376 Part 1 prose, never guessed: §21.2.3.11 for `ST_DLblPos`
(`ctr` → `Center`, `inEnd` → `InsideEnd`, `bestFit` → `BestFit`), §21.2.3.50 for `ST_TrendlineType`
(`movingAvg` → `MovingAverage`, `exp` → `Exponential`), §§21.2.3.12–14 for the error bars
(`cust` → `Custom`, `stdErr` → `StandardError`). The exact wire token appears in each item's docs, and
a token the schema does not admit reads as `None` rather than as a guess. The schema's own defaults
are honoured: a bare `<c:showVal/>` is `true`, `<c:trendlineType/>` is `linear`, `<c:errBarType/>` is
`both`, `<c:order/>` and `<c:period/>` are 2.

`ChartData::data_labels` lets a chart label itself the moment it is authored, refused by `validate`
for the two surface kinds, which declare no `c:dLbls`.

**Preservation gained reach and changed nothing.** The `mjx-opc` round-trip suites and tier-3 edit
isolation are unchanged, and decorating a chart dirties `chart1.xml` and *nothing else* — not even
the embedded workbook, because decoration is not data. With `MJX_REQUIRE_SCHEMA=1`, every authored
decoration validates against `dml-chart.xsd` under `xmllint`, including the two cases the ranks make
distinct: a pie chart, whose `CT_PieSer` places `c:dPt` differently, and a scatter chart with two sets
of error bars, which `CT_ScatterSer` admits and `CT_BarSer` does not.

The *Limitations* row in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` naming these four families
is **removed**, not softened — it moves to "what used to be here", where rows go when they close by
being done.

## [0.0.64] - 2026-09-02

Subtree copy-on-write — the copy-on-write `mjx-opc` does per part, now done per subtree (MJX-248).

A6 found that the fidelity reader records each attribute's name, value and quote but **not the
whitespace separating it from the previous one**, so a start tag Office wrapped across lines
re-emitted on one. It pinned the shortfall in `KNOWN_REFLOWS` and stopped, because the obvious fix —
a whitespace field on `RawAttribute` and a trailing-whitespace field on `RawElement` — costs a value
at every construction site, adds size to the hottest data structure in the library, and buys exactly
one preserved property. The next one (entity spelling, comment placement, self-closing style) would
cost the same again.

**The tree is span-preserving instead.** Every element parsed by `mjx_xml::fidelity` remembers the
byte range it came from; the document keeps the buffer; and the serializer writes an unmodified
element by copying that range rather than rebuilding it. One field subsumes the whole family:
whitespace between attributes, whitespace before `/>`, quote style, the spelling of a character
reference (`&#38;` stays `&#38;`), and the placement of comments and processing instructions inside a
subtree. `KNOWN_REFLOWS` is **deleted**, not emptied — its two-way pin fires when a part starts
round-tripping, so the entry could not have been left behind — and `crates/mjx-opc/tests/roundtrip.rs`
and `tree_roundtrip.rs` now have no exceptions at all.

**The invariant is structural, not remembered.** A range is only sound while the element still is
what was parsed, and `RawElement`'s fields are public, so there is nowhere to hook a "clear the span"
call. `RawElement` therefore keeps its attribute and child lists in a `RawElementContent` it
`Deref`s to: reads are unchanged (`element.children`, `element.attributes` still resolve), and any
*mutable* access goes through `DerefMut`, which drops the range. Because mutable descent into a child
passes through every ancestor's child list, that drops the range along the whole path from the root —
which is exactly "a mutation clears the span on that node and every ancestor", obtained by
construction rather than by discipline. `Clone` drops it too, so a subtree copied into another
document can never be written from the buffer it left behind, and `PartialEq` ignores it, so
`RawElement` equality still means "the same markup".

**The one way this could corrupt a file is namespaces**, and it is pinned first. A verbatim subtree
carries prefixes but not the `xmlns:` declarations that bind them; if a rewritten ancestor pruned a
declaration, every descendant beneath it would come silently unbound. It cannot, and the reason is
structural: the reader keeps `xmlns` declarations as ordinary attributes in document order and the
writer emits every attribute an element holds without inspecting any of them.
`crates/mjx-xml/tests/subtree_cow.rs` opens with the test that fails if that ever stops being true —
it namespace-resolves the *output*, the way a consumer does.

**The range is untrusted on the way out.** It is sliced fallibly, and then checked against the
element it claims to describe: the bytes must open with `<` plus that element's qualified name
followed by a delimiter, and close the way the element says it closes. That is what catches a mutated
`name` or `empty` — the two fields deliberately left outside the `Deref` because navigation reads
them constantly — and it means a wrong range degrades to a re-flow, never to wrong bytes. Adversarial
cases are pinned: out-of-bounds, inverted, pointing at a different element, and the one a naive
`starts_with` gets wrong (`<a>` must not claim `<abbr>`'s range).

Measured on a synthetic 2.3 MiB slide (80,004 elements, `cargo run --release -p mjx-xml --example
mjx248_measure`): `size_of::<RawElement>()` 64 → 72 bytes, **+8 bytes per element** — the span packs
into a `u32` start plus a `NonZeroU32` end, so `Option` needs no discriminant, and moving the lists
behind the `Deref` costs nothing. Serializing that part after editing one attribute of one element:
**4.59 ms → 0.27 ms, 17x faster**; untouched, 0.08 ms. A part read but not edited retains no extra
memory at all — `mjx-opc` now shares one `Arc<[u8]>` between the bytes it re-emits and the tree that
indexes into them — and an edited part holds its source buffer, which
`Package::release_unused_part_sources` reclaims once nothing can be copied from it.

One byte-fidelity defect the new adversarial corpus found is fixed with it: `<!DOCTYPE a>` lost the
space after `<!DOCTYPE`, because quick-xml trims it and the writer rebuilt the wrapper. The doctype's
inner bytes now come out of the source.

`crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` states the stronger guarantee — every subtree you
did not touch is byte-for-byte what it was — and drops the re-flow limitation, which is gone. It
gains a narrower one in its place: three surfaces (`edit_vml_drawing`, `edit_chart`, the table-style
list) read a whole part into a typed model and write the whole part back, so subtree copy-on-write
does not reach inside those; slide edits, which navigate in place, are unaffected.

## [0.0.63] - 2026-09-02

Schema-order emission — children are written in `xsd:sequence` order by construction (MJX-248).
OOXML complex types are overwhelmingly sequences, and **children in the wrong order are invalid even
when every child is present and every child is itself correct** — a repair-dialog defect, not a
cosmetic one. Nothing in the workspace enforced that on write: order was whatever each hand-written
serializer happened to do, so correctness rested on the author having read the XSD for that type and
on a fixture happening to exercise it. Fourteen separate hand-copied rank tables had grown across
`mjx-dml`, `mjx-chart` and `mjx-pptx`, one per type, each added by whoever noticed.

**The order now comes from the schema.** `cargo run -p xtask -- codegen` reads the `xsd:complexType`
content models of `dml-main.xsd`, `pml.xsd` and `dml-chart.xsd`, flattens each one — resolving
`xsd:group` references across schemas — and commits
`mjx-ooxml-types::child_order`: every child of every complex type of those schemas, with the position
it occupies. Alternatives of an `xsd:choice` *share* a position, which is exactly the "either an
`a:solidFill` or an `a:noFill`, and whichever is there is the one to replace" question a writer asks.
A type whose own model is `xsd:choice` or `xsd:all` — `CT_Path2D`'s repeating path commands, for
instance — is recorded as unordered rather than given a false order.

### The boundary this does not cross

Placement is a write-side operation and only ever runs on a child a caller asked to write. **Nothing
reads a document and rewrites it into schema order.** A real file may carry children in an order the
schema permits but this table would not have chosen, and re-ordering it would be corruption of the
caller's document rather than a fix. Existing children are never sorted; a new child is inserted
after the last sibling that must precede it. Markup the table does not name — an unmodelled element,
a foreign namespace, a comment, an `mc:AlternateContent` — is invisible to placement: it never moves,
and it never moves the insertion point, so it keeps its position relative to its known neighbours.

### Added

- `mjx-ooxml-types::child_order` — `ChildOrder`, `ChildSlot`, `ContentModel`, `TypeReference`, the
  placement primitives (`ChildOrder::replace_or_insert`, `insert`, `insert_index`,
  `insert_index_of_names`, `rank_of`, `slot`), the ordering audit (`ChildOrder::first_out_of_order`,
  `audit_tree`, `TreeAudit`, `OutOfOrderChild`), the by-symbol lookups (`find`, `root_element`), the
  three generated tables (`DML_MAIN_TYPES`, `PML_TYPES`, `DML_CHART_TYPES`) and twenty-eight named
  constants for the types this workspace writes.
- A child-order audit inside the schema-validity suite that runs on **every** authored-deck case,
  with or without `References/`: it walks every element of every part whose root the tables name and
  fails on the first child out of its type's sequence. `xmllint` catches an ordering fault only for
  the shape some case happens to author, and only where the schemas are installed.

### Changed

- Every insertion path in `mjx-dml`, `mjx-chart` and `mjx-pptx` now places children through the
  generated table. The fourteen hand-written rank tables are gone.

### Removed

- `TableStylePart::rank` and `CellBorder::rank`. Both existed only to feed a hand-written ordering
  table; the generated one is now the single source, and a second copy of a sequence is the thing
  this release exists to remove. `TableStylePart::all` and `CellBorder::all` are unchanged.

## [0.0.62] - 2026-09-02

Package invariant validation — a deck that would need repair is not written (MJX-248). This library
was very good at *not touching* what it does not understand, and had essentially no defence for what
it *does* write. `Package::save` performed no package-level check of any kind, so markup naming a
relationship its `.rels` never declared, a relationship pointing at a part that was not there, a part
no content-type rule covered, and duplicate identifiers where the format requires uniqueness could all
be written and shipped. Every one of them makes PowerPoint say it "found a problem with the content
and needs to repair", and none of them is visible to the schema gate: A1/A2 validate each part against
its XSD in isolation, and every one of these defects is a property of the package *graph*, perfectly
schema-valid part by part.

**`save` now validates first, and the check is not opt-in.** A check you have to remember is a check
that ships the fault it was meant to catch.

- `mjx-opc`: `Package::validate`, `Package::save_unchecked`, and `PackageDefect` — one variant per
  invariant, each naming the part, relationship and identifier at fault:
  `PartWithoutContentType` (ECMA-376 Part 2 §6.2.3), `RelationshipTargetMissing`,
  `UnresolvableRelationshipTarget`, `DuplicateRelationshipId` (§6.5.3),
  `UndeclaredRelationshipReference` — every attribute in the shared relationship-reference namespace,
  not `r:id` alone, because `shared-relationshipReference.xsd` types all fourteen of them
  `ST_RelationshipId` — and `PartIsNotWellFormedXml`. Reached through `OpcError::Invalid`.
- `mjx-pptx`: `Presentation::validate`, `Presentation::save_unchecked`, and `PresentationDefect`:
  `DuplicateShapeId`, `DuplicateListEntryId`, `DuplicateListEntryReference`,
  `ListEntryTargetHasWrongContentType` and `UnlistedRelationship` — the `p:sldIdLst` /
  `p:sldMasterIdLst` / `p:sldLayoutIdLst` agreement with a part's relationships, in both directions.
  Reached through `PptxError::InvalidPresentation`.
- `mjx-opc`: `Package::authored_xml_parts`, `ZipEntry::provenance`, `ZipEntry::tree` and
  `PartProvenance` — the validation scope, defined once so both layers agree on it.

**The scope is the markup this library will write.** A part still holding the bytes it was opened with
is re-emitted verbatim and is never faulted, so a file that arrives broken can still be written back,
and *reading* a part can never change whether a package saves. The moment an edit makes those bytes
ours, the same defect is refused. `save_unchecked` is the deliberate escape hatch.

**A corrupting bug the validator found on its first run.** `add_ole_object` gave its snapshot picture
a hard-coded `p:cNvPr@id` of `0` while the frame took an allocated id, so two OLE objects on one slide
wrote two shapes with the same non-visual id — a duplicate PowerPoint repairs. Fixed, with a
regression test that asserts the ids rather than only that the save succeeded.

The cost, measured on the largest fixture (`charts.pptx`, 43 entries, 39 relationships): **35.7 µs**
to validate against 3.2 ms to write the container — about 1% of a save. Nothing that arrived as
container bytes is ever tokenised.

## [0.0.61] - 2026-09-02

The remaining model gaps, and an honest gap table (MJX-43). The guide's gap list had accumulated
rows that were no longer true, rows that were real, and rows that were deliberate decisions filed as
though they were oversights. This release closes the real ones, restates the decisions as decisions
with their reasoning, and rewrites the page around the difference.

**A shape's own list style is authorable.** Tier 3 of the text ladder — `a:lstStyle` on a shape's
text body, the tier that says *every paragraph at this indent level, in this shape* — could be read
and resolved through since the ladder was written, and could not be stated. It now can:

- `mjx-pptx`: `Presentation::shape_list_style_level`, `set_shape_list_style_level`,
  `clear_shape_list_style_level`, `shape_list_style_default`, `set_shape_list_style_default`,
  `clear_shape_list_style_default`, and `clear_shape_list_style` for the whole element. The setters
  merge, as every other setter does; a clear that finds nothing changes nothing and does not dirty the
  part.
- `mjx-dml`: `TextListStyle::new`, `set_level`, `set_default_properties`, `remove_level`,
  `remove_default_properties`; `TextBody::set_list_style` and `remove_list_style`. A new level is
  placed by `CT_TextListStyle`'s sequence and a new `a:lstStyle` by `CT_TextBody`'s — between
  `a:bodyPr` and the first `a:p` — because order is validity, not style.

**The gap table is now two lists.** Non-goals, each with the reason it is a decision, and *built but
not yet verified against Office*, each with the work that will verify it. Four rows closed outright:
merge-aware selections (already true in the code and now proven by the cases that discriminate — a
merge anchored outside the selection, and the text and paragraph formatters, not just the cell
formatter), the `a:lstStyle` setter above, `Scene3D::backdrop`, and a font slot the theme does not
define — which was correct behaviour listed as a gap. `extLst` is restated as what it has always
been: the schema's own unknown bucket (`CT_OfficeArtExtension` is a required `uri` plus
`xsd:any processContents="lax"`), preserved verbatim through an edit and pinned there by tests at
both tiers rather than merely asserted.

- New fixture `tests/fixtures/table_extensions.pptx` — a table whose `a:tblPr` and one `a:tcPr` carry
  a vendor extension — registered with the OPC round-trip suites, the fidelity-tree suite and the
  schema gate.

## [0.0.60] - 2026-09-02

Typed surfaces for the content that is not DrawingML (MJX-140, absorbing MJX-139). Five kinds of
content — OLE objects, ActiveX controls, ink, SmartArt diagrams and legacy VML — round-tripped
perfectly and could be *read*, and that was all. There was no authoring, no editing, and no way to
answer the question that makes any of it useful: **which shape is this?** An InkML part was findable
but untraceable; a diagram was `GraphicFrameKind::Diagram` and nothing more; `mjx-vml` had been 69
lines since Phase 0, its own doc comment deferring "rich modeling and shape-level references" to "a
later phase" that had no owner and no date.

Every one of the five now has read, author **and** edit coverage. Nothing about the round-trip
guarantee changes: modelling a type only adds reach, and the edit-isolation tier over each fixture is
the gate this release is measured against.

`mjx-vml` — from 69 lines to a real model:

- `Drawing` (the `<xml>` root of a `vmlDrawingN.vml`, or any element holding VML shapes — a Word
  `w:pict`, an `mc:Fallback` branch), `Shape`, `ShapeTemplate`, `ShapeGroup`, `ImageData`, `TextBox`,
  `Fill`, `Stroke`, `ShapePath`, `DiagramText`; the Office extensions that carry the references —
  `ShapeLayout` / `ShapeIdMap`, `EmbeddedOleObject`, `Ink`, `ShapeProtections`; and
  `AttachedObjectData`, the legacy form control's own record. `DrawingPart` reads and writes a whole
  part.
- The point of it is one hop: `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name a
  VML shape's `id`, and `Drawing::shape_by_identifier` resolves it.
- Names come from the ECMA-376 Part 4 §19 prose, never the wire token — `v:shapetype` is a
  `ShapeTemplate`, `o:idmap` a `ShapeIdMap`, `x:ClientData` an `AttachedObjectData` — and
  `ST_ObjectType`'s nineteen values expand to `PushButton`, `DropdownBox`, `AuditingLine` and the rest.

`mjx-pptx`:

- **Ink.** `ink_references` ties every InkML part to the content part that names it, finding both
  PresentationML's `p:contentPart` and the `p14:contentPart` producers wrap in `mc:AlternateContent`;
  `ink_part_for_shape` and `shape_for_ink_part` walk it either way. `add_ink` writes the part and the
  reference; `set_ink_content` replaces the strokes without touching the slide. Both check the root
  namespace, so a package cannot end up declaring `application/inkml+xml` over something else.
- **SmartArt.** `diagram_relationship_ids` and `diagram_parts` expose the whole graph — the four parts
  a `dgm:relIds` names plus the cached drawing, which hangs off the *data* part rather than the frame.
  `add_diagram` writes all four with their relationships and the frame; `DiagramContent::vertical_list`
  generates a working diagram from a list of labels, `from_parts` takes four documents of your own.
  `set_diagram_part` replaces one of them in place.
- **OLE and ActiveX.** `add_ole_object` (an embedded stream, a whole embedded package, or a link) and
  `add_activex_control` (the `ax:ocx` part, its `.bin` state and the `p:controls` container), plus
  `set_ole_prog_id`, `set_ole_object_data`, `set_ole_snapshot_image`, `set_activex_control_name`,
  `set_activex_state`, `set_activex_snapshot_image` and `remove_activex_control`. Reading gains
  `activex_class_id` and `activex_persistence`. Both kinds can be bound to their legacy fallback with
  `set_ole_legacy_shape_id` / `set_activex_control_shape_id` and read back with
  `ole_legacy_shape_id` / `activex_control_shape_id`.
- **VML** (behind the `vml` feature): `vml_drawing_part`, `with_vml_drawing`, `edit_vml_drawing`,
  `add_vml_drawing`, and the headline `with_vml_shape_for_ole_object` /
  `with_vml_shape_for_activex_control`, which walk from the modern frame to the legacy shape that
  draws it. The feature's boundary is unchanged and now documented: it decides only whether *this*
  crate re-exposes the surface, since `mjx-vml` is a normal crate `mjx-docx` will depend on directly.

Verification: the schema gate gains a DrawingML-diagram arm, because this project now writes those
four parts — `dml-diagram.xsd` joins the markers `harness()` requires, and a new case pins that all
four are *validated* rather than skipped. Seven new schema cases cover the authored diagram, OLE
object, ActiveX control, ink and VML deck plus an edited OLE object. `mjx-opc`'s `tree_roundtrip` now
covers the four legacy fixtures.

One limitation surfaced and is recorded rather than hidden: the fidelity reader does not preserve the
whitespace *between* attributes, so a start tag whose attributes were wrapped across lines re-flows
onto one line when its part is edited. It never touches a part nobody edited — those keep their
original bytes and are never re-serialised — but Office wraps VML start tags far more often than it
wraps a slide's, so it shows there first. `KNOWN_REFLOWS` in `crates/mjx-opc/tests/tree_roundtrip.rs`
pins it, and the fidelity guide states it. Fixing it means adding a field to `RawAttribute` and
`RawElement` in `mjx-ooxml-core` and touching ~140 construction sites across every crate, which is an
architectural decision rather than a fix to take inside this change.

The guide's "preserved but not modelled" table is gone, replaced by a read/author/edit table for the
five and five honest non-goal rows (InkML strokes, the SmartArt layout engine, `ax:ocxPr`, VML path
evaluation, and the re-flow above). A seventh example, `legacy_content`, exercises all five and runs
in both feature modes.

Still open from MJX-140: **producer-authentic validation**. Every fixture here is still hand-crafted,
so what the schema gate proves is that our reader agrees with our writer against markup we wrote.
Obtaining decks Microsoft PowerPoint actually produced needs Office, and belongs with the runtime
verification work.

## [0.0.59] - 2026-07-31

The usage guide and the first runnable examples (MJX-209). The repository documented every *item* —
every public item has rustdoc, `missing_docs` is a lint, a strict rustdoc job gates CI — and one
*concept*, the effective-properties page. It documented no *task*: nothing answered "I have a `.pptx`
and want to change the title", nothing answered "I want to produce a deck", and there was no
`examples/` directory or runnable program anywhere in the workspace.

First of three workstreams to `v0.1`: **documentation → external application surface → validation**.

`mjx-pptx`:

- A five-page guide under `crates/mjx-pptx/docs/guide/`, surfaced as the doc-only `mjx_pptx::guide`
  module tree: *building a deck* (the whole story once, end to end), *shapes and text*, *tables,
  charts and pictures*, *inheritance, layouts and masters*, and *fidelity and the known gaps*. The
  last is a candour page listing every deliberate gap with its issue — no embedded chart workbook, no
  guide-formula evaluator, selections that are not merge-aware, colour transforms implemented from the
  prose but unverified against Office, and the fact that no test in this repository reads a file
  PowerPoint wrote. **All 48 doctests in the guides compile against the real API.**
- Six examples under `crates/mjx-pptx/examples/`, each reopening what it wrote and asserting something
  about it: `build_a_deck`, `read_deck` (which re-saves and proves all 17 parts stayed byte-identical),
  `edit_text` (which reports that retitling a slide dirties exactly one part), `style_shapes`,
  `build_table`, `charts_and_media`. `anyhow` is added as a dev-dependency; examples are the one place
  file I/O belongs, because the library is bytes-in/bytes-out and the caller reads and writes.

CI: a new `examples` job runs all six, and the office-open job now feeds `build_a_deck`'s output
through LibreOffice — so "the guide's headline example produces a deck Office opens" is a merge gate.

Also: the README gains a quickstart, a guide table and the example commands; the `mjx-ooxml` facade
gains the guide ladder; and PLAN.md's Phase 3b, which still described tables as in progress and
speaker notes as open, records what actually shipped and adds Phase 3c.

Two API observations surfaced while writing the examples, recorded for the `v0.1` review (MJX-37):
`cell_span` answers `(columns, rows)` while `table_dimensions` answers `(rows, columns)`, and
`OuterShadowEffect` has no `Default` though `EffectListSpec` does. Both are documented where they
bite rather than worked around silently. No behaviour change in this release.

## [0.0.58] - 2026-07-31

Paragraph-hierarchy audit (MJX-22, closing MJX-38). The seven-tier text ladder passed its tests, but
those tests reached the interesting cases by mutating a deck through the builder API rather than by
reading a file, so two disagreements with ECMA-376 Part 1 had gone unnoticed. Both are fixed here,
each cited to the prose that settles it.

`mjx-pptx`:

- **A list-style tier now contributes its `a:defPPr` beneath its level.** `TextListStyle::default_properties`
  had existed since the text model landed and resolution never called it, so a tier supplying nothing
  but an `a:defPPr` contributed nothing and a paragraph at a level its style does not define came back
  empty. §21.1.2.2.2 defines `a:defPPr` as the properties applied "when no other paragraph properties
  have been specified"; §21.1.2.2.6 says the same of a paragraph. The audit also confirms there is
  **no** fallback to `a:lvl1pPr` — §21.1.2.4.13 keys the nine level elements strictly to `a:pPr@lvl`,
  so the existing level behaviour was already right.
- **A shape that is not a placeholder now takes a master text style.** Tier 5 was gated on `p:ph`;
  §19.3.1.35 instead splits by kind — `p:bodyStyle` for a text box (`p:cNvSpPr@txBox`), `p:otherStyle`
  for any other non-placeholder shape. Tier 4 keeps its gate: without a slot there is nothing to
  match. This changes what effective text a deck containing plain shapes or text boxes reports. Real
  PowerPoint is believed to match the previous behaviour, so it is isolated in one commit and tracked
  for validation against an Office-saved deck.
- New `slide::shape_is_text_box`, the reader counterpart of the `txBox="1"` the text-box builder
  already writes.
- The effective-properties guide records both rungs.

Tests: a new hand-authored `tests/fixtures/text_levels.pptx` in which every tier owns a facet no other
tier touches — nine body levels' worth of structure with `a:lvl5pPr` deliberately absent, a layout
overriding only two levels, a shape-level `a:lstStyle` no public setter can author, a footer, a text
box and a plain autoshape. `crates/mjx-pptx/tests/paragraph_hierarchy.rs` pins fifteen rungs against
it; the fixture is registered in the `mjx-opc` round-trip suites and in the LibreOffice open canary.
`layouts.pptx` is untouched.

## [0.0.57] - 2026-07-31

The effective-properties guide (MJX-23). Ten `effective_*` readers had shipped and nothing explained
the idea behind them: the knowledge was spread across ten per-method doc comments and the frozen
`docs/*_HANDOFF.md` files, which are history rather than user documentation. Documentation only — no
behaviour, no API change.

`mjx-pptx`:

- New guide at `crates/mjx-pptx/docs/effective_properties.md`, pulled in with `include_str!` on a
  documentation-only `effective_properties` module, so it reads as prose on a source host and renders
  as its own page in `cargo doc`. It covers: *what a file states* versus *what a renderer shows*; the
  one candidate walk every shape resolver is built on, and why a shape that is not a placeholder
  inherits nothing; the three-source ladder fill, outline and effects share; why a transform is
  inherited whole while text merges tier by tier; the seven text tiers and the level axis cutting
  across them; the shorter table-cell ladder (MJX-33's `effective_cell_*` trio); why colours bake to
  concrete `RRGGBB`; every stop condition, including why text answers with an empty spec where a fill
  answers `None`; and what one read costs.
- Each of the ten readers gains a link to the guide. Their own doc comments stay authoritative for
  their own ladders and stop conditions.

Also: the workspace README grows a guides list, and the `mjx-ooxml` facade — the crate its own docs
name as the entry point for reading the docs — grows a Guides section.

## [0.0.56] - 2026-07-30

Cell 3-D review and direct-cell authoring (MJX-109, closing the last code follow-up of MJX-38). D4
(MJX-100) left `Cell3D` with two material accessors pending a decision and gave a typed 3-D surface
only to the table-*style* cell3D (`a:tcStyle > a:cell3D`); a direct cell's `a:tcPr > a:cell3D` had
none. Both are settled here.

`mjx-dml`:

- `Cell3D::material` (typed) and `Cell3D::preset_material` (raw wire token) are kept as a deliberate
  pair — the typed accessor is the normal path and mirrors `Shape3D::material`; the raw one is an
  escape hatch for a producer value outside `ST_PresetMaterialType`. Docs rewritten to say so; no API
  change.
- `TableCellProperties` gains typed `cell_3d()` / `set_cell_3d()`, the direct-cell counterpart of
  `TableStyleCellStyle`'s, reusing the same `Cell3D` model and honoring `CT_TableCellProperties`
  schema order (`cell3D` after the borders, before the fill).

`mjx-pptx`:

- `CellFormat` gains `with_cell_material` / `with_cell_bevel` / `with_cell_light_rig`, mirroring
  `TableStyleFormat`. `format_cells` now authors a direct cell's `a:cell3D`; any facet set gives the
  cell a `cell3D` with the schema-required bevel. Additive, non-breaking.

- `docs/CUSTOM_GEOMETRY_HANDOFF.md` records the four shipped atoms (CG1–CG4), the design decisions,
  the verified schema, the known follow-ups (chiefly a guide-formula evaluator), and the 3-D audit
  that found `a:scene3d` / `a:sp3d` already complete — so MJX-44's opaque-geometry gap is closed.

## [0.0.54] - 2026-07-30

Custom geometry, the PowerPoint surface (MJX-44 CG4). The `mjx-dml` custom-geometry model (CG1–CG3)
now reaches `.pptx`: one accessor reads and writes both preset and custom geometry.

`mjx-pptx`:

- New `Geometry` enum — `Preset(ShapeGeometry)` | `Custom(CustomGeometrySpec)` | `Inherited`.
- **Breaking:** `Presentation::shape_geometry` now returns `Geometry` (was `ShapeGeometry`), and
  `set_shape_geometry` / the cursor's `.geometry(..)` now take a `Geometry` (was `ShapeGeometry`).
  Migrate a preset call by wrapping it: `Geometry::Preset(ShapeGeometry::…)`. `shape_geometry` no
  longer errors when a shape declares no geometry — it returns `Geometry::Inherited` — so
  `PptxError::ShapeHasNoGeometry` is no longer produced by these methods.
- `shape_geometry` now reads `a:custGeom` (as `Geometry::Custom`) as well as `a:prstGeom`;
  `set_shape_geometry` writes either, converts between them (the two are mutually exclusive), and for
  `Geometry::Inherited` removes the shape's own geometry element so an inherited one takes over.

Pre-`v0.1`, so the API is still unstable; this is the deliberate unification MJX-44 called for.

## [0.0.53] - 2026-07-30

Custom geometry, the container and auxiliary lists (MJX-44 CG3). Completes the `mjx-dml` model of
`a:custGeom` — the path list (CG2) now sits inside the whole `CT_CustomGeometry2D`, with its guides,
adjust handles, connection sites, and text rectangle.

`mjx-dml`:

- `CustomGeometry` (`a:custGeom`, `CT_CustomGeometry2D`) — a fidelity wrapper reading every child
  typed (`adjust_values`/`guides`/`adjust_handles`/`connection_sites`/`text_rectangle`/`paths`) and
  round-tripping byte-for-byte (an unmodeled child such as `extLst` re-emits verbatim).
- Interner-free value types: `GuideSpec` (`a:gd` name + formula), `AdjustHandle` (`a:ahXY` / `a:ahPolar`
  with their `gdRef*` / min / max bounds), `ConnectionSite` (`a:cxn` angle + position), and
  `Rectangle` (`a:rect` edges).
- `CustomGeometrySpec` + `to_custom_geometry` — the interner-free read/author surface; builds children
  in schema order, omits empty auxiliary lists, always writes the required `a:pathLst`.

Additive and non-breaking.

## [0.0.52] - 2026-07-30

Custom geometry, the path list (MJX-44 CG2). The drawing commands a freeform `a:custGeom` is traced
from — the render-critical core, on top of the CG1 value types.

`mjx-dml`:

- `Path2DList` (`a:pathLst`, `CT_Path2DList`) and `Path2D` (`a:path`, `CT_Path2D`) — fidelity wrappers
  that read their paths / flags typed and round-trip byte-for-byte (an unmodeled child re-emits
  verbatim). `Path2D` exposes `width`/`height`/`fill`/`stroke`/`extrusion_ok` (each `None` when
  unstated, distinct from the schema default) and `commands`.
- `DrawCommand` — the interner-free, ordered instruction a renderer follows: `MoveTo`, `LineTo`,
  `ArcTo { width_radius, height_radius, start_angle, swing_angle }`, `QuadBezierTo`, `CubicBezierTo`,
  `Close` (the `a:path` choice group `close`/`moveTo`/`lnTo`/`arcTo`/`quadBezTo`/`cubicBezTo`).
- `Point` — an interner-free `(x, y)` of `AdjustCoordinate`s; `AdjustPoint::value` resolves one.
- `Path2DSpec` (with `to_path_2d`) and `Path2DList::new` / `paths` / `specs` — the read/author surface.

Additive and non-breaking.

## [0.0.51] - 2026-07-30

Custom geometry, foundation types (MJX-44 CG1). Groundwork for a typed surface over `a:custGeom`
(`CT_CustomGeometry2D`) — the freeform path list a hand-drawn PowerPoint shape uses, until now
preserved only opaquely. This iteration adds the value types every piece of a custom geometry is
expressed in; the path list, guide/handle/connection lists, and the pptx accessor follow.

`mjx-ooxml-types`:

- Generated `PathFillMode` (`ST_PathFillMode`: `none`/`norm`→`Normal`/`lighten`/`lightenLess`/
  `darken`/`darkenLess`) — how a freeform `a:path` is filled (`a:path@fill`). Added to the DrawingML
  codegen allowlist.

`mjx-dml`:

- `AdjustCoordinate` (`ST_AdjCoordinate`) and `AdjustAngle` (`ST_AdjAngle`) — each a union of a
  numeric literal (`Emu` / `Angle`) and a geometry-guide reference by name (`Guide`), the two forms a
  custom-geometry coordinate or angle can take.
- `AdjustPoint` (`a:pt` / `a:pos`, `CT_AdjPoint2D`) — the `(x, y)` a path command, adjust handle, or
  connection site is drawn through; a fidelity leaf that reads its coordinates typed and round-trips
  byte-for-byte. Re-exported alongside `PathFillMode` from the crate root.

Additive and non-breaking.

## [0.0.50] - 2026-07-30

Inaccessible external sources — audio/video media (MJX-201 P4, **completing MJX-201**). A slide can
reference audio or video that lives online/externally and is unreachable on another platform. Every
media carrier — `a:videoFile`/`a:audioFile@r:link`, the `a14:media` fallback, `p:snd`/`p:sndTgt`
timing/transition sounds — resolves through a media-typed relationship in the slide's `.rels`, so a
media reference is neutralized by redirecting that relationship.

`mjx-pptx`:

- `Presentation::replace_media_with_placeholder` inserts a placeholder media part and retargets the
  relationship at it (`mjx_opc::Package::retarget_relationship`), so every carrier that named it
  resolves inside the package; the poster image is untouched. The placeholder is caller-supplied bytes
  or a built-in one matching the kind — `default_placeholder_audio()` (a minimal valid silent WAV) or
  `default_placeholder_video()` (a minimal structurally valid MP4 with an empty video track). A
  non-media relationship yields the new `PptxError::NotAMediaReference`.
- `Presentation::media_references` lists a surface's audio/video/media relationships (by id, with kind,
  target, and whether external) — the discovery surface for what to replace. `MediaKind` and
  `MediaReference` are the reported types.

Additive and non-breaking.

## [0.0.49] - 2026-07-30

Inaccessible external sources — OLE objects (MJX-201 P3). An OLE object can reference embedded (or
linked/external) data that is unreachable on another platform. Unlike a chart, an OLE object has no
cached fallback — but it is displayed via its snapshot image and its data stream is read only on
activation, so it is neutralized by redirecting the reference to an in-package placeholder.

`mjx-pptx`:

- `Presentation::replace_ole_object_with_placeholder` inserts a placeholder object part and retargets
  the OLE frame's data relationship at it (`mjx_opc::Package::retarget_relationship`, this feature's
  first consumer), so the object resolves inside the package. The placeholder is caller-supplied bytes
  or the new `default_placeholder_ole()` — a minimal but structurally valid MS-CFB compound file (an
  empty root storage). The `p:oleObj` markup is untouched; a replaced embedded part is left
  unreferenced and can be swept with `Package::remove_unreferenced_parts`. A non-OLE shape yields the
  new `PptxError::ShapeIsNotAnOleObject`.
- `Presentation::ole_objects` lists the OLE frames on a surface, each with its data target, `progId`,
  and whether the reference is external — the discovery surface for what to replace.

Additive and non-breaking. Next: audio/video media (P4).

## [0.0.48] - 2026-07-30

Inaccessible external sources — chart backing workbook (MJX-201 P2). A chart can reference a workbook
that lives online/externally; that reference can be unreachable on another platform. A chart renders
entirely from its cached data (`c:numCache`/`c:strCache`), so the workbook is only needed to *edit* the
data — which means the reference can simply be detached.

`mjx-pptx`:

- `Presentation::detach_chart_workbook` removes a chart's `c:externalData` reference — the element and
  its relationship — leaving the chart to render from its cache (the same cache-only shape a freshly
  authored chart has). An embedded workbook part is left unreferenced and can be swept with
  `Package::remove_unreferenced_parts`. A non-chart shape yields `PptxError::ShapeIsNotAChart`; a chart
  with no backing workbook yields the new `PptxError::ChartHasNoExternalData`.
- `Presentation::chart_workbooks` lists the charts on a surface that reference a workbook, each with its
  target and whether the reference is external — the discovery surface for what to detach.

Additive and non-breaking. Follow-up phases extend to OLE objects and media, where (unlike charts)
there is no cached fallback and the P1 redirect-to-placeholder is used.

## [0.0.47] - 2026-07-30

Inaccessible external sources — foundation + linked-image placeholder (MJX-201 P1, spun out of MJX-42).
Many element sources can be external/online (linked images, a chart's backing workbook, OLE, media),
and an unreachable target can crash a consumer. This begins the caller-driven capability to neutralize
one by substituting an in-package placeholder of the same kind; the library does no external I/O, so
the caller decides which references are inaccessible.

`mjx-opc` gains the general redirect lever:

- `Package::external_relationships` lists every `TargetMode::External` relationship (with its owning
  part) — the discovery surface for what might be unreachable.
- `Package::retarget_relationship` repoints a relationship at a new target/mode while keeping its id
  and its `.rels` position (editing the control tree and the navigation view in tandem). The recipe:
  `insert_part` a placeholder, then retarget the external relationship at it as `Internal` — so the
  binding element resolves in-package without touching its own markup, which is what the many unmodeled
  element kinds need.

`mjx-pptx` applies it to images (the one modeled kind, via element rewrite):

- `Presentation::replace_linked_image_with_placeholder` embeds a placeholder — caller-supplied bytes or
  the new `DEFAULT_PLACEHOLDER_IMAGE` — into a picture that links an external image, rewriting
  `@r:link` → `@r:embed` and dropping the dangling link relationship. An embedded picture yields
  `PptxError::PictureImageNotLinked`.
- `Presentation::linked_images` lists the linked pictures on a surface (with their targets) so callers
  need not walk the shapes.

Additive and non-breaking. Follow-up phases extend the same redirect to the chart workbook, OLE, and
media.

## [0.0.46] - 2026-07-30

Linked images become addressable (MJX-42, second of two package-gap fixes). A picture that *links* its
image (`p:blipFill > a:blip@r:link`) rather than embedding it was invisible to the API:
`picture_image_rel_id` read only `@r:embed` and returned `None`, so a linked image could not be
reached even though it round-tripped fine.

- `Presentation::picture_image_rel_id` now falls back to the link id, returning whichever relationship
  binds the image (embed preferred when both are present).
- New `Presentation::picture_image_link_target` returns where a linked image points — the relationship
  target string, external path/URL or in-package part alike — so a linked image is fully addressable.
- `picture_image_bytes` consequently reaches linked images: an embedded image or an internal link
  resolves to bytes; an external link reports `PptxError::ExternalTarget` (its bytes live outside the
  package).

Additive and non-breaking — an embedded picture reads exactly as before. Also elides a needless
lifetime flagged by newer stable clippy and unwraps single-literal `concat!`/drops unused imports in
`mjx-dml` tests, keeping the workspace clippy-clean under the current toolchain.

## [0.0.45] - 2026-07-30

Orphaned-part sweep (MJX-42, first of two package-gap fixes). Replacing an image, deleting a slide, or
any edit that unwires a relationship can leave a part with nothing pointing at it — a legal but dead
media blob. Until now nothing removed them; `remove_part_cascading` only walks downward from one named
part.

New `Package::remove_unreferenced_parts` on `mjx-opc` is the package-wide garbage collector. It returns
the swept part names and is conservative by construction: a part survives if it is reachable by
following `Internal` relationships transitively from the package root (`_rels/.rels`), so a media part
reached only through a live slide stays, and OPC-required roots (core properties, thumbnail) stay
because the root relationships name them. Control parts are never removed — `[Content_Types].xml` is
not a part, and every `.rels` part is spared. Reference cycles terminate.

The relationship-resolution logic shared by the reachability walk and the existing reference checks is
unified behind one `resolve_rel` helper (root-vs-part base).

## [0.0.44] - 2026-07-29

Run coalescing (MJX-41, third and last text-model gap — **completing MJX-41**) — formatting a
sub-range with `set_text_range_properties` splits a run, and repeatedly formatting overlapping ranges
leaves a paragraph with more runs than it needs. Nothing merged them back; now an explicit pass does.

New `Presentation::coalesce_paragraph_runs` and `coalesce_shape_runs` merge adjacent runs that would
render identically, returning the number of runs merged away. Two adjacent runs merge only when
**both** hold, so the paragraph reads exactly the same afterwards:

- their **effective** formatting is identical — resolved through the full inheritance ladder, so a run
  that sets a property explicitly merges with a neighbour that inherits the same value (this compares
  meaning, not raw XML); and
- neither carries distinguishing state this model does not describe — a hyperlink, an `rtl`, an
  `a:extLst`, a foreign attribute — so nothing is dropped by the merge (`dirty`/`err`/`smtClean`
  housekeeping is ignored and never blocks a merge).

A line break or field between two runs keeps them apart. When nothing merges, the call changes nothing
and does not dirty the part.

```rust
let merged = pres.coalesce_paragraph_runs(surface, shape, para)?;   // runs removed
let total = pres.coalesce_shape_runs(surface, shape)?;              // across the whole body
```

The supporting pieces are in `mjx-dml`: `CharacterProperties::unmodeled_state_eq` /
`has_only_modeled_state` (the safety gate) and `Paragraph::coalesce_adjacent_runs` (the content-vec
merge). Every part still round-trips byte-for-byte.

## [0.0.43] - 2026-07-29

`a:br` / `a:fld` addressability (MJX-41, second of three text-model gaps) — a line break (`a:br`) and
a text field (`a:fld`) are paragraph children like a run, but until now both fell into the opaque
`Raw` bucket, so a slide-number or date field's text could not be read and a break could not be
located.

Both are now typed: new `TextLineBreak` (`CT_TextLineBreak`, an optional `a:rPr`) and `TextField`
(`CT_TextField` — `@id`/`@type` and optional `a:rPr`/`a:pPr`/`a:t`) fidelity wrappers, added as
`ParagraphContent::LineBreak` / `Field` variants. Following the decision recorded on the issue, they
get **their own accessors** rather than joining the run index space, so this is **non-breaking** —
`runs()`, run indices, and `Paragraph::text()` are unchanged. New `Paragraph::line_breaks()` /
`fields()` (and `_mut`) enumerate them; `TextField::text()` reads the field's cached rendering.

`mjx-pptx` gains a read surface mirroring `run_text`/`run_count` — `paragraph_field_count`,
`paragraph_field_text`, and `paragraph_field_type` on `Presentation` — so a field's cached value and
kind are readable at the format level (a new `PptxError::FieldIndexOutOfRange` reports a bad index).

```rust
let count = pres.paragraph_field_count(surface, shape, para)?;
let text = pres.paragraph_field_text(surface, shape, para, 0)?;   // e.g. "1/27/13"
let kind = pres.paragraph_field_type(surface, shape, para, 0)?;   // e.g. Some("datetimeFigureOut")
```

Every part still round-trips byte-for-byte; reading a field dirties nothing.

## [0.0.42] - 2026-07-29

Underline line/fill groups (MJX-41, first of three text-model gaps) — the underline line group
(`a:uLn` / `a:uLnTx`) and fill group (`a:uFill` / `a:uFillTx`) on a run's `a:rPr` now have a typed
surface, so an underline can be recoloured and restyled independently of the text it sits under.
Previously both were preserved opaquely with no way to read or set them.

Each group is a three-state choice — unset (inherited), *follow text* (the marker element), or an
explicit value — modeled as `UnderlineLine` / `UnderlineFill`. The explicit forms reuse the existing
line and fill models (`LineSpec` for `a:uLn`, `FillSpec` for `a:uFill`), and the two members of a
group are mutually exclusive: writing one replaces the other in place. The groups flow through the
whole run-formatting surface for free — `CharacterPropertiesSpec` builders (`with_underline_line` /
`with_underline_fill`), `merge_under`, `set_text_range_properties`, and `effective_run_properties`
(where the colours are baked like any other fill or outline).

```rust
use mjx_dml::{CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth, UnderlineFill,
    UnderlineLine};

let spec = CharacterPropertiesSpec::new()
    .with_underline_line(UnderlineLine::Explicit(LineSpec::solid(
        LineWidth::from_points(1.0),
        ColorSpec::Srgb("FF0000".into()),
    )))
    .with_underline_fill(UnderlineFill::FollowText);
```

Additive and non-breaking; every untouched part still round-trips byte-for-byte.

## [0.0.41] - 2026-07-29

Ink (MJX-138, third and last tier of MJX-135) — **preserve-first** recognition of legacy ink (InkML)
content parts, **completing MJX-135**. Handwriting ink is carried as an InkML part
(`/ppt/ink/inkN.xml`, `application/inkml+xml`) referenced from the shape tree by a `p14:contentPart`.
Producers wrap that reference in `mc:AlternateContent` — a shape-tree child in the Markup-Compatibility
namespace that the shape index space cannot reach — so, like VML, ink is recognized by its content type
rather than navigated from a shape. The InkML markup is carried through a round-trip verbatim, not
modeled. Unconditional, like the OLE and ActiveX tiers.

```rust
use mjx_pptx::Presentation;

let deck = Presentation::open(&bytes)?;
for part in deck.ink_part_names() {
    let inkml = deck.ink_part_bytes(&part); // raw InkML, verbatim
}
```

### Added

- **`Presentation::ink_part_names`** — every InkML part in the package, recognized by content type.
- **`Presentation::ink_part_bytes`** — an ink part's bytes, verbatim and non-dirtying.
- Constants `REL_INK` (the shared `customXml` relationship type) and `CONTENT_TYPE_INKML`.

### Scope

Recognition + preserve + a read window only — no authoring, and the ink is not modeled (a typed stroke
surface, trace points → paths, is deferred). Per-shape association (`p14:contentPart@r:id`) and the
`mc:Fallback` snapshot are deferred with it. **MJX-135 (OLE / ActiveX / Ink) is now complete**;
producer-authentic fixture validation across all three tiers is a follow-up (MJX-140).

## [0.0.40] - 2026-07-26

ActiveX controls (MJX-137, second tier of MJX-135) — **preserve-first** recognition of legacy ActiveX
form controls. Unlike an OLE object (a graphic frame in the shape tree), a control lives in
`p:cSld > p:controls > p:control` — beside the shape tree — so it is addressed per-slide by a control
index. Its persisted state is a **two-hop** chain: `p:control@r:id` names the control part
(`/ppt/activeX/activeXN.xml`, `ax:ocx` markup), which in turn relates to its binary blob
(`/ppt/activeX/activeXN.bin`). The control markup, its binary, and its fallback snapshot image are each
carried through a round-trip verbatim, none modeled. Unconditional, like OLE.

```rust
use mjx_pptx::Presentation;

let mut deck = Presentation::open(&bytes)?;
for i in 0..deck.activex_control_count(slide)? {
    let name = deck.activex_control_name(slide, i)?;          // e.g. "CommandButton1"
    let ocx = deck.activex_part_bytes(slide, i)?;             // ax:ocx markup, verbatim
    let blob = deck.activex_binary_bytes(slide, i)?;          // persisted state (.bin), two-hop
    let snapshot = deck.activex_snapshot_image_bytes(slide, i)?; // fallback image for rendering
}
```

### Added

- **`Presentation::activex_control_count`** — the number of ActiveX controls on a surface.
- **`Presentation::activex_control_rel_id` / `activex_control_name`** — a control's control-part
  relationship id and its declared `name`.
- **`Presentation::activex_part_bytes`** — the `ax:ocx` control part's verbatim bytes.
- **`Presentation::activex_binary_bytes`** — the control's binary blob, resolved across the two-hop
  `activeXControlBinary` chain.
- **`Presentation::activex_snapshot_rel_id` / `activex_snapshot_image_bytes`** — the fallback snapshot
  image a renderer draws in place of the (never-executed) control.
- Constants `REL_CONTROL`, `REL_ACTIVEX_CONTROL_BINARY`, `CONTENT_TYPE_ACTIVEX`,
  `CONTENT_TYPE_ACTIVEX_BINARY`.

### Scope

Recognition + preserve + a read window only — no authoring, and the control is not modeled (opaque
`ax:ocx` markup + binary state). The last MJX-135 tier is ink (MJX-138); producer-authentic fixture
validation is a follow-up (MJX-140).

## [0.0.39] - 2026-07-26

OLE objects (MJX-136, first tier of MJX-135) — **preserve-first** recognition of legacy embedded OLE
objects. An OLE object is an embedded document (a legacy `.xls`/`.doc` or an OLE `.bin` stream)
referenced from a `p:graphicFrame` via `p:oleObj@r:id`, drawn from a fallback image snapshot. Such a
frame previously surfaced only as the opaque `GraphicFrameKind::Other`; it now reads as `OleObject`,
with accessors for the embedded object's bytes and the snapshot image — both carried through a
round-trip verbatim, neither modeled. Unlike VML this is **not** feature-gated: an OLE frame is ordinary
PresentationML, so it mirrors the (unconditional) chart surface.

```rust
use mjx_pptx::{GraphicFrameKind, Presentation};

let mut deck = Presentation::open(&bytes)?;
if deck.graphic_frame_kind(slide, shape)? == Some(GraphicFrameKind::OleObject) {
    let prog = deck.ole_prog_id(slide, shape)?;               // e.g. "Excel.Sheet.12"
    let object = deck.ole_object_part_bytes(slide, shape)?;   // embedded object, verbatim
    let snapshot = deck.ole_snapshot_image_bytes(slide, shape)?; // fallback image for rendering
}
```

### Added

- **`GraphicFrameKind::OleObject`** — a graphic frame framing a `p:oleObj` (refines the former `Other`).
- **`Presentation::ole_object_rel_id` / `ole_object_part_bytes`** — the embedded object's relationship
  and its verbatim bytes (`/ppt/embeddings/oleObjectN.bin` or an embedded package).
- **`Presentation::ole_snapshot_rel_id` / `ole_snapshot_image_bytes`** — the fallback snapshot image a
  renderer draws in place of the (never-executed) object.
- **`Presentation::ole_prog_id`** — the owning application's `progId`.
- Constants `REL_OLE_OBJECT`, `REL_PACKAGE`, `CONTENT_TYPE_OLE_OBJECT`.

### Scope

Recognition + preserve + a read window only — no authoring, and the embedded object is not modeled
(it is an opaque OLE stream or embedded document). The `p:oleObj` is reached through its
`mc:AlternateContent` wrapper (preferring the `mc:Choice` branch) by a bounded structural descent, not
by running full MCE resolution. The remaining MJX-135 tiers are ActiveX controls (MJX-137) and ink
(MJX-138); producer-authentic fixture validation is a follow-up (MJX-140).

## [0.0.38] - 2026-07-26

VML, tier V1 (MJX-115) — **preserve-first** legacy VML round-trip. VML is the Transitional-only drawing
markup producers still emit for OLE-object fallbacks, comment shapes, ink and legacy controls, carried
as standalone `vmlDrawingN.vml` parts. Such parts already round-trip byte-identically through the
generic part-level copy-on-write; this release adds a **recognition surface** so callers can find and
read them — behind the new `vml` crate feature (opt-in, off by default). The VML XML is **not modeled**.

```rust
// with `mjx-pptx` (or `mjx-ooxml`) built with the `vml` feature
let deck = Presentation::open(&bytes)?;
for part in deck.vml_part_names() {
    let xml = deck.vml_part_bytes(&part); // raw legacy VML, verbatim
}
```

### Added

- **`mjx-vml`** becomes real (was a scaffold stub): the VML vocabulary — `CONTENT_TYPE_VML`,
  `REL_VML_DRAWING`, `VML_DEFAULT_EXTENSION` — and an `is_vml_content_type` recognition predicate.
- **`Presentation::vml_part_names`** / **`vml_part_bytes`** (behind the `vml` feature) — enumerate the
  legacy VML drawing parts a package carries (by content type, so VML referenced from any part is
  found) and read a part's bytes verbatim, without dirtying anything.
- **`vml` Cargo feature** on `mjx-pptx` (the repo's first), re-exposed by the `mjx-ooxml` facade.

### Scope

Preserve-first only: VML is recognized and readable as raw bytes, never parsed or modeled, and never
authored. Recognition is package-level (content type), not yet shape/relationship-level — the OLE /
ActiveX / ink references that cite a specific VML shape are the next tier (MJX-135), and Word-side
legacy VML (`w:pict`, header/footer fallback) is tracked under the Word slice (MJX-139). The fixture is
hand-crafted; validation against genuine producer decks is a follow-up (MJX-140). This completes the
chart + VML arc (MJX-47) except for the chart embedded workbook (MJX-116).

## [0.0.37] - 2026-07-25

Charts, tier C4 (MJX-114) — **authoring** a brand-new chart. C0–C3 recognized, modeled and edited an
existing chart; this release creates one from scratch. A chart is described fluently with `ChartData`
(a kind, shared categories, named series) and added to a slide with `Presentation::add_chart`, which
writes a new chart part (`ppt/charts/chartN.xml`) and a `p:graphicFrame` that references it. All six
kinds are supported: bar, line, area, pie, doughnut and scatter.

```rust
use mjx_pptx::{ChartData, ChartKind, ShapeBounds};

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("Revenue", [10.0, 20.5, 15.0])
    .series("Cost", [5.0, 8.0, 7.25]);
let shape = deck.add_chart(slide, &chart, ShapeBounds::from_inches(1.0, 1.0, 6.0, 4.0))?;
```

### Added

- **`Presentation::add_chart`** — authors a chart on a surface from a `ChartData`, returning its shape
  index. Creates the chart part with its `CONTENT_TYPE_CHART` Override and a `REL_CHART` relationship
  from the slide; every pre-existing part stays byte-identical.
- **`ChartData`** (re-exported from `mjx-pptx`, alongside `ChartKind`) — a fluent builder
  (`new(kind).categories(...).series(name, values)`) that serializes a complete `c:chartSpace` part.
- Error **`InvalidChartData`** — a chart with no series (or only empty series) is refused at creation.

### Scope

Authoring writes **cached data only** (`c:strCache`/`c:numCache`, with synthesized `c:f` formulas so
the references are schema-valid) and **no embedded workbook**: the chart renders everywhere from its
cache, while PowerPoint's "Edit Data" is degraded until the embedded-workbook follow-up (MJX-116).
Scatter's shared categories become numeric X values, falling back to the point position for a
non-numeric label. This completes the chart arc except for VML (V1) and the embedded workbook.

## [0.0.36] - 2026-07-25

Charts, tier C3 (MJX-113) — the first **mutating** chart tier. C1/C2 modeled a chart read-only; this
release rewrites a series' cached values and category labels (`c:numCache` / `c:strCache`) on an
existing chart, through a `mjx-pptx` surface. The cached values are what **render**; a chart's
embedded workbook is **not** rewritten and goes stale (a separate follow-up). Only the edited chart
part is dirtied — every other part, the embedded workbook included, is left byte-identical.

```rust
// read the series, then rewrite the first series' values
for s in deck.chart_series(slide, shape)? {           // name, categories, values per series
    println!("{:?}: {:?} = {:?}", s.name, s.categories, s.values);
}
deck.set_chart_series_values(slide, shape, 0, &[1.0, 2.5, 3.0])?;
deck.set_chart_series_categories(slide, shape, 0, &["Q1", "Q2", "Q3"])?;
```

### Added

- **`Presentation::chart_series`** — each series of a chart as a `ChartSeriesData` (`name`,
  `categories`, `values`; a scatter series' `xVal`/`yVal`), flattened across the chart's plots.
  Non-dirtying.
- **`Presentation::set_chart_series_values` / `set_chart_series_categories`** — rewrite the cached
  data of the `series_idx`-th series (0-based across the plots), dirtying only the chart part.
  `set_chart_series_values` targets `c:val`, or a scatter series' `c:yVal`.
- **`ChartSeriesData`** — the read DTO.
- **`mjx-chart` mutation** — `NumberCache::set_values` / `StringCache::set_labels`, the
  reference/data-source `set_values`/`set_labels`, `Series::set_values`/`set_categories`, and the
  mutable navigation (`series_mut`, `all_series_mut`, `ChartSpace::series_mut`/`series_count`).
- Errors **`ShapeIsNotAChart`**, **`ChartSeriesOutOfRange`**, **`ChartSeriesNotEditable`**.

### Fidelity

A cache edit rebuilds only its `c:pt` points and its `c:ptCount`; the `c:formatCode` and everything
outside the edited cache (the axes, other series, styling) survive verbatim. A rewritten number is
formatted with Rust's shortest round-trip representation (the exact inverse of the read parse); a
non-finite value, which has no valid spelling, is skipped. `mjx-pptx` gains a dependency on
`mjx-chart` (both shared-markup/format tiers, cycle-free).

## [0.0.35] - 2026-07-25

Charts, tier C2 (MJX-112) — the remaining common plot types. C1 modeled the bar plot; this release
extends the same read-only, byte-identical model to **line** (`c:lineChart`), **pie** (`c:pieChart`),
**area** (`c:areaChart`), **scatter** (`c:scatterChart`) and **doughnut** (`c:doughnutChart`), and to
**combo charts** — a `c:plotArea` may legitimately hold more than one plot.

```rust
use mjx_ooxml_core::FromXml;

let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;
let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
for kind in space.chart_kinds() {          // e.g. [Bar, Line] for a combo chart
    println!("{kind:?}");
}
if let Some(scatter) = space.plot_area().and_then(|p| p.scatter_chart()) {
    for series in scatter.series() {
        let xs = series.x_data().map(|x| x.values());   // c:xVal, not c:cat
        let ys = series.y_data().map(|y| y.values());   // c:yVal, not c:val
    }
}
```

### Added

- **Plot types** — `LineChart`, `PieChart`, `AreaChart`, `ScatterChart`, `DoughnutChart` alongside the
  existing `BarChart`, each with `series()`/`series_at()`/`series_count()`/`kind()`. `ChartKind` gains
  `Line`, `Pie`, `Area`, `Scatter`, `Doughnut`.
- **`PlotArea` accessors** — `line_chart()`/`pie_chart()`/`area_chart()`/`scatter_chart()`/
  `doughnut_chart()` beside `bar_chart()`; `chart_kinds()` (one entry per plot, for combo charts) and
  `all_series()` (every plot's series, flattened). `ChartSpace::chart_kinds()` mirrors it.
- **Scatter data** — `Series::x_data()` (`c:xVal`) and `y_data()` (`c:yVal`), plus
  `CategoryData::values()` (the numeric companion to `labels()`), for the one series type that carries
  X/Y data instead of `c:cat`/`c:val`.

### Fidelity

Each plot type is its own struct but they share one `Series` type and one `PlotContent` bucket; every
plot preserves its own element name and buckets its type-specific scalars (`barDir`, `grouping`,
`firstSliceAng`, `holeSize`, `scatterStyle`) and axes into `Raw`, so a chart of any modeled type — or
a combo — round-trips byte-for-byte. Unmodeled plot types (radar, bubble, 3-D, …) ride through `Raw`.

## [0.0.34] - 2026-07-25

Charts, tier C1 (MJX-111) — the chart XML gets a typed home. C0 recognized a chart frame and handed
back the chart part's raw bytes; this release **models** that part in `mjx-chart` (until now a
scaffold stub). It derives the chart-space spine `c:chartSpace → c:chart → c:plotArea` and one plot
type end to end — the bar/column plot (`c:barChart` / `c:ser` / `c:cat` / `c:val`) — with read-only
accessors for a chart's kind, its series, and each series' category labels and values, read down
through the `c:strCache` / `c:numCache`.

```rust
use mjx_ooxml_core::FromXml;

let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;      // the /ppt/charts/chartN.xml bytes
let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
if let Some(bar) = space.bar_chart() {                       // c:chart → c:plotArea → c:barChart
    for series in bar.series() {
        let name = series.name();                            // "Sales", from c:tx
        let labels = series.categories().map(|c| c.labels()); // ["North", "South", "West"]
        let values = series.values().map(|v| v.values());     // [19.2, 21.4, 16.7]
    }
}
```

### Added

- **`mjx-chart` chart model** — `ChartSpace` (`c:chartSpace`), `Chart`, `PlotArea`, `BarChart`,
  `Series`, and the data layer (`NumericData`/`CategoryData`/`SeriesText`,
  `NumberReference`/`StringReference`, `NumberCache`/`StringCache`, `DataPoint`, `Value`, `Formula`),
  each parsed with `FromXml` and re-emitted byte-for-byte with `ToXml`.
- **Read accessors** — `ChartSpace::{chart, plot_area, bar_chart, chart_kind}`;
  `BarChart::{series, series_at, series_count, direction, grouping}`;
  `Series::{name, categories, values, index, order}`; `CategoryData::labels`, `NumericData::values`,
  and the underlying reference/cache/point accessors. Chart kinds are the extensible `ChartKind`
  enum; a bar plot's `BarDirection` and `BarGrouping` are typed.

### Fidelity

Every modeled container keeps an ordered `content` list of typed children plus a `Raw` catch-all
(mirroring the `mjx-dml` table model), so the axes, text properties, an external-data reference, a
literal data source or an `extLst` this tier does not interpret round-trip byte-for-byte. A cached
value is parsed on demand from its point's preserved wire text — never reformatted on write.

### Scope

Read-only, bar plot only. Cached data (`c:numCache` / `c:strCache`) is the read path; a literal
source (`c:numLit` / `c:strLit`) or a multi-level category rides through the `Raw` bucket for now.
Other plot types are tier C2; editing (C3) and authoring (C4) are later tiers.

## [0.0.33] - 2026-07-25

Charts, tier C0 (MJX-47) — the first step of the chart workstream. A `p:graphicFrame` that frames a
chart (its `a:graphicData@uri` is the chart URI and its payload is a `c:chart`) points at a **separate
part** (`/ppt/charts/chartN.xml`) by relationship id, unlike a table, whose `a:tbl` is inline. This
release recognizes such a frame, resolves that relationship, and reads the chart part's bytes — the
chart XML itself is not modeled yet; it and its satellites (an embedded workbook, colour and style
parts) are carried through a round-trip **verbatim**.

```rust
if deck.graphic_frame_kind(slide, shape)? == Some(GraphicFrameKind::Chart) {
    let rel = deck.chart_rel_id(slide, shape)?;        // the slide relationship the frame names
    let xml = deck.chart_part_bytes(slide, shape)?;    // the /ppt/charts/chartN.xml bytes, borrowed
}
```

### Added

- **`Presentation::chart_rel_id`** — the relationship id a chart frame names
  (`p:graphicFrame > a:graphic > a:graphicData > c:chart@r:id`), or `None` for any shape that frames
  no chart. The `c:chart` element is looked for rather than the frame's `uri` trusted — the payload
  decides. Reading is non-dirtying.
- **`Presentation::chart_part_bytes`** — the raw XML of the chart part that frame references, borrowed
  from the package exactly as stored (never re-serialized), or `None` when the shape frames no chart.
  The read window onto a chart until `mjx-chart` models it.
- **`constants::REL_CHART` and `constants::CONTENT_TYPE_CHART`** — the chart relationship type and the
  chart part's content type, for the authoring tiers to come.
- A `tests/fixtures/charts.pptx` fixture (two slides, one clustered-column chart with an embedded
  workbook) and integration tests proving a chart deck round-trips byte-identically, that reading a
  chart dirties nothing, and that editing another slide leaves every chart part untouched.

### Changed

- The private `image_part_for_rel` helper is generalized to `part_for_rel` (it resolves any
  relationship id to its part), now shared by the image and chart read paths.

## [0.0.32] - 2026-07-24

DrawingML 3-D, part 3 (MJX-49 D4) — and with it the 3-D workstream is complete. `Cell3D`
(`CT_Cell3D`, a table cell's 3-D corner), until now a fidelity wrapper that kept its `a:bevel` /
`a:lightRig` opaque, becomes the **first consumer** of the typed model: it reads and authors them
through the same `Bevel` / `LightRig` the shape surface uses.

```rust
// a header row whose cells stand up in metal, bevelled and lit
deck.format_table_style_part(style_id, TableStylePart::FirstRow,
    &TableStyleFormat::new()
        .with_cell_material(PresetMaterial::Metal)
        .with_cell_bevel(Bevel { width: Some(Emu::from_emu(76_200)), ..Bevel::default() })
        .with_cell_light_rig(LightRig { rig: LightRigType::ThreePoint, direction: LightRigDirection::Top, rotation: None }))?;
```

### Added

- **`mjx-dml`: `Cell3D` decomposed** — typed `material()` (a `PresetMaterial`, alongside the retained
  raw `preset_material()`), `bevel()` and `light_rig()` accessors, and authoring via `Cell3D::new`
  (seeded with the schema-required empty bevel) + `set_material` / `set_bevel` / `set_light_rig`, with
  `TableStyleCellStyle::set_cell_3d` placing the child at its schema rank. The `a:bevel` / `a:lightRig`
  wire helpers are shared with the shape surface; `extLst` stays opaque, so a `cell3D` still
  round-trips byte-for-byte.
- **`mjx-pptx`: cell-3-D on the table-style builder** — `TableStyleFormat::with_cell_material`,
  `with_cell_bevel` and `with_cell_light_rig` give a styled part's cells an `a:cell3D`, applied
  through the shared and inline `tableStyles` paths alike.

## [0.0.31] - 2026-07-24

DrawingML 3-D, part 2 (MJX-49 D3) — the `mjx-pptx` shape surface. The typed 3-D model from 0.0.30
gains its `Presentation` accessors, a 1:1 mirror of the shape-effects surface (E3): a shape's 3-D
scene and its own 3-D properties are now readable, writable and clearable, on a group member as on a
top-level shape.

```rust
deck.set_shape_scene_3d(0, shape, &Scene3DSpec { camera, light_rig })?;   // how it is lit and viewed
deck.set_shape_3d_properties(0, shape, &Shape3DSpec { extrusion_height, bevel_top, .. })?;
deck.shape(0, shape)?.scene_3d(scene).shape_3d_properties(props).apply()?; // or fluently, one commit
```

### Added

- **`Presentation::shape_scene_3d` / `set_shape_scene_3d` / `clear_shape_scene_3d`** — a shape's
  `p:spPr > a:scene3d` (`CT_Scene3D`) as an interner-free [`Scene3DSpec`]. Reading is non-dirtying and
  returns `None` when the shape is flat (3-D has no inheritance chain) or the scene omits a
  schema-required camera/light rig. Setting rebuilds the element in `CT_ShapeProperties` order — after
  any fill, outline and effects, before `a:sp3d`. Clearing **removes** the element (an empty
  `a:scene3d` would be schema-invalid), a no-op when absent.
- **`Presentation::shape_3d_properties` / `set_shape_3d_properties` / `clear_shape_3d_properties`** —
  a shape's `p:spPr > a:sp3d` (`CT_Shape3D`: extrusion, contour, bevels, material, edge colors) as a
  [`Shape3DSpec`]. `a:sp3d` is the last visual property, so it lands after everything else and before
  any `a:extLst`. An unstated attribute reads `None`, not the schema default.
- **`ShapeCursor::scene_3d` / `clear_scene_3d` / `shape_3d_properties` / `clear_shape_3d_properties`**
  — the same edits recorded on the fluent cursor, applied in one commit alongside fill/outline/effects.
- All six flat methods and the cursor take `impl Into<ShapePath>`, so a group member is addressed the
  same as a top-level shape.

## [0.0.30] - 2026-07-24

DrawingML 3-D, part 1 of the workstream (MJX-49 D1+D2) — the `a:scene3d` / `a:sp3d` subsystem, until
now round-tripped opaquely, gains a typed model. Mirrors the effects/outline/fill workstreams:
generated preset enums, then the `mjx-dml` value types and fidelity wrappers. The `mjx-pptx` shape
surface (D3) and the `Cell3D` upgrade (D4) follow.

### Added

- **`mjx-ooxml-types::drawingml`** — five generated preset enums: `BevelPreset` (12),
  `LightRigType` (27), `LightRigDirection` (8), `PresetMaterial` (15) and `PresetCamera` (62). Each
  cryptic token is expanded to a self-explanatory name sourced from the ECMA-376 token (the light
  direction's compass abbreviations, `threePt`/`twoPt`, `dkEdge`, `softmetal`).
- **`mjx-dml`: the 3-D model** — value types `Bevel`, `SphereCoordinates`, `Camera` (preset view +
  field of view + zoom + rotation) and `LightRig`; fidelity wrappers `Scene3D` (`CT_Scene3D`) and
  `Shape3D` (`CT_Shape3D`) with typed accessors and interner-free `Scene3DSpec` / `Shape3DSpec`. The
  camera and light rig, and a shape's bevels, extrusion/contour colors and material, are read typed;
  the rarer `a:backdrop` and any `extLst` stay opaque, so an element round-trips byte-for-byte. Every
  measure is `Option`, so an unstated attribute reads `None`, not the schema default.

## [0.0.29] - 2026-07-24

Group descent, part 4 — **group structure**, and with it the group workstream is complete. A
`p:grpSp` is now addressable, measurable, editable *and* something a caller can make, dissolve and
move shapes through.

```rust
let group = deck.group_shapes(0, &[1.into(), 2.into()])?;  // select these, group them
deck.move_shape_into_group(0, 3, &group)?;                 // and take that one too
deck.set_shape_fill(0, group.child(0), &navy)?;
```

### Added

- **`Presentation::group_shapes(surface, members)`** — wraps sibling shapes in a new group, returning
  its [`ShapePath`]. The group's box is the union of the members' own boxes (how ECMA-376 Part 1
  §L.4.7.4 defines a child bounding box) and its child space is set **identical** to it, so the
  mapping is the identity: the members keep their coordinates exactly, with no rounding anywhere. The
  group takes the earliest member's z-order slot, and the members keep their relative order inside it
  whatever order they were named in.
- **`Presentation::ungroup(surface, group)`** — dissolves a group, returning where its members now
  are. Each keeps its absolute placement, the group's mapping unwound into its own transform.
- **`Presentation::move_shape_into_group` / `move_shape_out_of_group`** — move one shape one level,
  in or out. The shape does not move on screen: its transform is restated for its new coordinate
  system, **mirrors and rotation included**, so joining a scaled, turned or flipped group leaves it
  exactly where it was.
- **`ShapeCursor::into_group` / `out_of_group` / `group_with` / `ungroup`** — the same, said mid-chain
  and following the shape. Each is a **commit point**: it writes what has been recorded so far,
  performs the change, then re-anchors, so no recorded edit is ever applied against a tree it was not
  recorded against.
- **`ShapePath::child` / `parent`** — step down to a member or up to the enclosing group, which is
  how the group returned by `group_shapes` is addressed.
- **`ShapeBounds::union`** — the smallest rectangle containing both.
- `PptxError::GroupNeedsTwoShapes`, `ShapesAreNotSiblings`, `ShapeCannotContainItself`,
  `ShapeHasNoBounds`.

### Notes

There is deliberately **no empty-group constructor**. §L.4.7.4 records that a group with no shapes is
degenerate and produces no visible output, and one with a single shape "has no representational power
beyond that of the one shape" — and an empty group has no honest `chOff`/`chExt` to be given. Every
group these create is well-formed by construction.

## [0.0.28] - 2026-07-24

Group descent, part 3 — a group member now says **where it is on the slide**. Addressing it, styling
it and measuring it are finally the same three things they are for a top-level shape.

### Changed

- **`Presentation::shape_bounds` answers in absolute slide EMU for a group member**, composing every
  enclosing group's child coordinate space instead of returning the member's raw `a:off` / `a:ext`.
  `set_shape_bounds` takes the same absolute rectangle and maps it back, so read and write stay in
  one space. This is a **behaviour change** for nested addresses only — a top-level shape reads and
  writes exactly as before, because composing over no ancestors is the identity. `shape_transform` /
  `set_shape_transform` are untouched and remain the accessors for what the file literally states, in
  the shape's own space. `effective_shape_bounds` / `effective_shape_transform` compose too, after
  resolving placeholder inheritance; the latter is where the composed rotation and mirror flags are
  read, since an axis-aligned `ShapeBounds` cannot hold a rotation.
- The shape cursor's `.bounds(…)` is slide-absolute to match, and runs the same conversion;
  `.transform(…)` still writes verbatim in the shape's own space.

### Added

- **`mjx-dml`: `Transform2D::child_scale` / `child_to_parent` / `parent_to_child`** — one rung of the
  mapping between a group's child coordinate space and its parent's, and its exact inverse.
- `PptxError::ShapeCannotBePlaced`, when an enclosing group states no `a:chOff` / `a:chExt`: there is
  then no mapping to invert, so the member reads as unplaced and the write is refused rather than
  putting the shape somewhere wrong.

### Notes

The composition follows **ECMA-376 Part 1 §L.4.7.4**, not the naive "apply each ancestor transform in
turn": a nested shape is scaled and mirrored by the *product* of its ancestors' factors, rotated by
their *sum*, and translated so its **centre** lands where the whole chain — rotations included — puts
it. A mirrored or rotated group therefore places its members correctly, which composing corners would
not. Round-tripping `set_shape_bounds(shape_bounds(…))` is exact whenever the groups' scales are, and
within a few EMU — millionths of an inch — when they are not.

## [0.0.27] - 2026-07-24

Group descent, part 2 — the **shape cursor**: a shape is addressed once and edited fluently, and a
group is restyled in one expression.

```rust
deck.shape(0, 2)?                                  // the group at top-level index 2
    .effects(shadow)
    .member(0)?.fill(navy).outline(rule)           // its first member
    .sibling(1)?.fill(gold).text("Q3").all_run_properties(bold)
    .apply()?;                                     // one write pass, one dirty part
```

### Added

- **`Presentation::shape(surface, path)` → `ShapeCursor`** — the ergonomic layer over the
  `set_shape_*` methods. Edit methods record intent and return the cursor; `.apply()` consumes it,
  writes every edit in the order it was recorded, and marks the part dirty once. A cursor that is
  never applied changes nothing, so it is `#[must_use]`. Every edit it records is executed by the
  code the mirrored flat method calls — a cursor is a way of *saying* the edits, not a second way of
  doing them.
- **Moving through a group** — `.member(i)` descends into a `p:grpSp`, `.sibling(i)` moves to another
  shape in the same container, `.parent()` steps back out; `.kind()`, `.member_count()` and `.path()`
  say where the cursor is. Each move checks the address as it lands, so a bad one fails where it was
  written. Recorded edits stay bound to the address they were recorded at, so one `.apply()` commits
  work spread over a group and its members.
- **What a cursor records** — the `p:spPr` surface (`fill` / `no_fill`, `outline` / `no_outline`,
  `effects` / `no_effects`, `geometry`, `bounds`, `transform`), `text`, the text-formatting specs
  (`run_properties`, `paragraph_run_properties`, `all_run_properties`, `end_run_properties`,
  `paragraph_properties`, `text_range_properties` and its `_by_grapheme` sibling), the shape's own
  `hyperlink` / `clear_hyperlink`, and a picture's `image`. Hyperlinks on a *run* or a text range are
  addressed by paragraph and run and stay on the flat API.
- **`Presentation::set_shape_text_content(surface, shape, text)`** — replaces a shape's whole text
  with one paragraph per line, each holding one run, so `shape_text` reads back exactly what was
  written. Only the paragraphs are swapped: the body's own `a:bodyPr` and `a:lstStyle` survive, so
  restating a placeholder's text does not disturb how it is laid out.
- **`Presentation::shape_member_count(surface, shape)`** — how many members a group holds (`0` for
  anything that is not a group).
- `PptxError::ShapeIsNotAGroup` and `PptxError::ShapeHasNoParent`, the two ways a cursor move is
  refused.

### Changed

- The per-shape element edits (fill, outline, effects, preset geometry, a picture's blip) moved into
  `slide.rs` as primitives taking an already-resolved shape, so the flat setters and the cursor share
  one implementation each. No behaviour change.

## [0.0.26] - 2026-07-22

Group descent, part 1 — shapes inside a `p:grpSp` are now addressable. Every shape API takes an
address as `impl Into<ShapePath>`: a bare index is a top-level shape (unchanged), and an array
`[group, member, …]` descends into nested groups. A group member can be read, edited and removed
exactly like a top-level shape; `shape_count` still counts only the top level, and a member's
`shape_bounds` are its own `a:off`/`a:ext` in the group's child space (the absolute-rectangle mapping
lands in a later atom). `PptxError::ShapeIndexOutOfRange` now carries the requested `ShapePath` and
the shape count of the container where the address ran out of range.

## [0.0.25] - 2026-07-22

Hyperlinks on runs and shapes — set, read, and clear links, external URLs and slide jumps.

### Added

- **`Hyperlink`** — a resolved link: `Hyperlink::Url(String)` (an external target) or
  `Hyperlink::Slide(usize)` (a jump to another slide in the deck). The relationship indirection
  (`r:id` → external URL or internal slide part) stays inside `Presentation`.
- **`Presentation::run_hyperlink` / `set_run_hyperlink` / `clear_run_hyperlink`** — the click
  hyperlink on a run, read back as a `Hyperlink`; setting adds its relationship (creating the run's
  `a:rPr` if absent), clearing removes the relationship once nothing else in the part still names it.
- **`Presentation::set_text_range_hyperlink`** — links a scalar range, splitting runs at the
  boundaries so exactly the selected text carries the link (one shared relationship).
- **`Presentation::shape_hyperlink` / `set_shape_hyperlink` / `clear_shape_hyperlink`** — the same on
  a shape's own `p:cNvPr > a:hlinkClick`.
- `mjx-dml`: `CharacterProperties` and `TextRun` gain `hyperlink_rel_id` / `set_hyperlink` (the raw
  `a:hlinkClick` accessors the packaging layer drives).

## [0.0.24] - 2026-07-22

Speaker notes, part 2 — the ergonomic notes surface: read, set, and clear a slide's notes.

### Added

- **`Presentation::notes_text(slide)`** — the speaker notes of a slide, read from its notes slide's
  `body` placeholder by kind (the caller never needs the shape index); `None` when the slide has no
  notes.
- **`Presentation::set_notes_text(slide, text)`** — sets the notes, **creating the notes slide on
  demand** (and, when the deck has none, **synthesizing a notes master** for it to follow) with its
  relationships and content-type overrides. Creating a notes slide adds exactly that part, its
  `.rels`, the slide → notes-slide relationship and the override — every pre-existing part stays
  byte-identical.
- **`Presentation::clear_notes(slide)`** — removes a slide's notes slide (and its `.rels` and
  override); the shared notes master and the slide survive. A no-op when the slide has no notes.

This completes MJX-34 — the last feature before the `v0.1` PowerPoint milestone.

## [0.0.23] - 2026-07-22

Speaker notes, part 1 — a notes slide and the notes master become addressable surfaces.

### Added

- **`Surface::Notes(slide)`** and **`Surface::NotesMaster`** — a slide's notes slide carries the same
  `p:cSld > p:spTree` a slide does, so every existing shape, text, fill, outline, effect, transform and
  table method now works on it unchanged, addressed by the slide it belongs to. `Surface::NotesMaster`
  addresses the single notes master every notes slide inherits from.
- Notes text inherits from the notes master's **`p:notesStyle`** exactly as slide text inherits from a
  slide master's `p:txStyles`; `color_map` and `theme` resolve through the notes master too.

### Changed

- `Presentation::surface_part` now returns an owned `PartName` (a notes part is resolved lazily by
  relationship, not stored), simplifying every call site that previously cloned the borrow.

## [0.0.22] - 2026-07-22

Author inline table styles — a lean, self-contained styling path.

### Added

- **`Presentation::set_inline_table_style(surface, shape, &TableStyleDefinition)`** — gives a table its
  own **inline** `a:tableStyle`, replacing any inline or referenced style. The whole look is declared
  up front and travels with the table: no shared `tableStyles.xml` part, relationship, content-type or
  referenced GUID. Plus the incremental **`format_inline_table_style_part`**.
- **`TableStyleDefinition`** — a declarative builder (`with_name` / `with_id` / `with_part`) reusing
  `TableStyleFormat`; the vestigial `styleId` / `styleName` default.
- **`TableProperties::set_inline_style`** (`mjx-dml`) — writes the style as `a:tableStyle` at its rank,
  replacing any `a:tableStyle` / `a:tableStyleId`.

### Notes

- The style resolves and renders through the existing `with_table_style` and `effective_cell_*`
  readers exactly as a shared one does — an inline style is the same `CT_TableStyle`, spelled out on
  the table.
- **Flags stay the caller's job**: a styled part renders only when its `a:tblPr` flag is on
  (`set_table_part`; `add_table` sets `firstRow`/`bandRow`).

## [0.0.21] - 2026-07-22

Table gaps closed — merge-aware formatting, inline styles, accessibility headers, and more.

### Added

- **`Presentation::cell_headers` / `set_cell_headers`** — the accessibility header associations of a
  cell (`a:tcPr > a:headers`), plus `TableCellProperties::headers` / `set_headers` and `TableCell::id`
  in `mjx-dml`.
- **`Presentation::visible_cell_text`** — the text that renders at a position: the cell's own, or its
  merge anchor's when it is covered.
- **`Presentation::graphic_frame_kind`** returning **`GraphicFrameKind`** (`Table` / `Chart` /
  `Diagram` / `Other`) — tells "not a table" from "a graphic not modeled yet"; a chart or diagram
  frame still answers `ShapeIsNotATable` to the table methods.
- **`TableProperties::inline_style`** (`mjx-dml`) — reports a style defined **inline** on the table
  (`a:tableStyle`); `with_table_style` and the effective-formatting resolvers now resolve an inline
  style as well as a referenced one.

### Changed

- **Formatting a cell selection is merge-aware**: `format_cells` / `format_cell_text` /
  `format_cell_paragraphs` skip merge-covered cells (which render nothing), so unmerging restores a
  covered cell's own formatting. Merging and unmerging still reach covered cells; single-cell methods
  addressed by `(row, column)` are unchanged.

## [0.0.20] - 2026-07-22

Effective cell formatting — what a table cell actually renders as. Closes the tables workstream.

### Added

- **`applicable_parts`** and **`TableStyleFlags`** (`mjx-dml`) — the style parts that cover a cell,
  most specific first, per the ECMA-376 §17.7.6 layering (corner cells > first/last column >
  first/last row > row bands > column bands > `wholeTbl`), with banding over data cells only.
- **`Presentation::effective_cell_fill` / `effective_cell_border`** — the fill or border a cell
  renders, resolving the cell's own `a:tcPr`, then the applicable style parts (explicit or a theme
  `fillRef`/`lnRef`), then the theme, colours baked to concrete `RRGGBB`. A border takes the outer
  edge for a rim cell and the interior edge (`insideH`/`insideV`) for one within the table.
- **`Presentation::effective_cell_run_properties`** — a cell's text run resolved down a
  table-specific ladder: the run's own `a:rPr`, the paragraph default, the table style's `a:tcTxStyle`
  for each applicable part (bold / italic / colour), then the presentation `p:defaultTextStyle`.

### Notes

- This is what the modeled `tableStyles.xml` exists for: everything before reported what a file
  *states*; this resolves what a renderer would show. An explicit property on the cell always wins.
- Reading resolves nothing into the file — every effective read leaves the package byte-identical.

## [0.0.19] - 2026-07-22

The `tableStyles.xml` part is modeled, and table styles can be authored and resolved.

### Added

- **The table-style model** (`mjx-dml`) — `TableStyleList`, `TableStyle`, the thirteen part slots
  (`TableStylePart`) plus `tblBg`, and the `TablePartStyle` / `TableStyleTextStyle` /
  `TableStyleCellStyle` / `TableCellBorderStyle` / `TableBackgroundStyle` / `FontReference` / `Cell3D`
  leaves. Every accessor reuses the DrawingML already modeled (fills, `LineProperties`, `Color`,
  `EffectList`, theme references via `StyleMatrixReference`). Two new generated types: the tri-state
  `OnOffStyle` (`on`/`off`/`def`) and `FontCollectionIndex`. `Cell3D`'s `a:bevel`/`a:lightRig` are
  preserved opaque pending the 3-D workstream.
- **Authoring the style tree** (`mjx-dml`) — constructors and setters that build a style from parts
  (fill, borders, text emphasis), each merge-not-rebuild and default-dropping.
- **`Presentation` surface** (`mjx-pptx`):
  - the seven `a:tblPr` flags — `table_part` / `set_table_part` (`TablePart`).
  - `table_style_id` / `set_table_style` — read and assign a table's `a:tableStyleId`.
  - `create_table_style`, creating the `tableStyles.xml` part on demand (relationship + content-type
    wired like an image part), and `format_table_style_part` with the new `TableStyleFormat` builder.
  - `with_table_style` — resolve a table's style through the shared part.
  - `PptxError::TableStyleNotFound`.
- **`tests/fixtures/tables.pptx`** — a deck carrying a real `tableStyles.xml` and a table naming its
  style.

### Notes

- A table style is layered formatting keyed by which part of the table a cell is in; modeling the
  part is what makes a `tableStyleId` resolve — the basis for effective cell formatting (next).
- Authoring a style touches exactly the content-types manifest, the presentation's relationships, and
  the new part; every other part stays byte-identical, and reading a styled table dirties nothing.

### Added

- **`Presentation::insert_row`, `remove_row`, `insert_column`, `remove_column`** — an index equal to
  the current count appends; beyond it is `TableCellOutOfRange`. A new row copies the height of the
  row beside it and a new column the width of the column beside it; the frame's own bounds are left
  alone, as PowerPoint leaves them.
- **`Table::insert_row`, `remove_row`, `insert_column`, `remove_column`** (`mjx-dml`) — the
  span-adjustment logic, plus `TableColumn::new`, `TableRow::new`,
  `TableCell::set_body_and_properties`, and grid/row/cell insert-and-remove helpers.

### Notes

- **The grid and every row stay in step.** A column edit changes `a:tblGrid` and one `a:tc` in every
  row together, so the rows never disagree with the width the grid declares.
- **Merges are adjusted, not left dangling.** A merge the new line falls inside grows by one; a merge
  the removed line lies inside shrinks by one; a merge whose **anchor** is removed promotes the next
  cell of the region, which takes over the anchor's `a:txBody` and `a:tcPr` and the reduced span so
  the table looks unchanged — including a region merged in both directions at once.
- **Removing the last row or column is refused** with `InvalidTableSize`: PowerPoint will not open a
  table with no cells.
- **Insert then remove is byte-identical to no change** — a span that falls back to one loses its
  attribute rather than being written as `gridSpan="1"`.
- The structural edit runs on the typed `Table` (parse, mutate, write back), not the raw tree:
  unlike a single-cell text edit it touches every row anyway, so parsing the whole table costs
  nothing extra and the merge logic is expressed in terms of the model.

## [0.0.17] - 2026-07-22

Cells can be merged, and unmerged.

### Added

- **`Presentation::merge_cells`** — takes a `Cells` selection, since every selection is a rectangle
  and a rectangle is the only shape a merged region can take.
- **`Presentation::unmerge_cells`** — given **any** cell of a region, not only its anchor.
- **`TableCell::set_spans`, `set_merged`, `clear_merge`** (`mjx-dml`).
- **`PptxError::TableMergeCrossesSelection`.**

### Notes

- **Merging never removes a cell.** The anchor states how far it reaches; the covered cells stay in
  the table, each stating that something to its left or above owns it. So the grid stays
  rectangular, `(row, column)` addressing keeps working, and a covered cell **keeps its own text** —
  invisible until unmerged, which is what makes unmerging give everything back.
- **A merge then an unmerge is byte-identical to no change at all.** A default is *removed* rather
  than written: `gridSpan="1"` and `hMerge="0"` are what the schema already assumes.
- **A selection that would cut an existing merge in half is refused.** Truncating it would leave the
  table claiming a span that no longer fits, and growing the selection would merge cells the caller
  never named. A region wholly inside the selection is absorbed instead.
- Merging one cell, or none, changes nothing rather than writing a span of one.

## [0.0.16] - 2026-07-21

Say it once. The table surface stops needing loops.

### Added

- **`Cells`** — which cells an operation is about: `one`, `row`, `column`, `rectangle`, `all`.
- **`CellFormat`** — a builder naming the cell properties to write (`with_fill`, `with_border`,
  `with_outline`, `with_margins`, `with_anchor`, `with_text_direction`), plus `without_fill` /
  `without_border` / `without_borders` for removal.
- **`Presentation::format_cells`, `format_cell_text`, `format_cell_paragraphs`** — apply a spec
  across a selection in one call.

### Notes

- Styling a header row took nine calls in a loop and read like nine things rather than the one thing
  it is. In the office-open canary this change turns twenty-two lines and four loops into nine lines
  and none.
- **Neither half is a new pattern.** The crate already builds specs with `with_`-prefixed setters
  (`CharacterPropertiesSpec`, `LineSpec`), and `set_shape_run_properties` already means "every run in
  this much of the shape". Tables simply never got either.
- **A format writes only what it names**, so recolouring a region cannot flatten borders it never
  mentioned. A format naming nothing writes nothing — not even an empty `a:tcPr`.
- `without_fill` is not `with_fill(FillSpec::None)`: removing lets the table style decide again,
  stating "none" stops it. Same for borders.
- The table is located **once** and the selection walked within it, so formatting a whole table is
  one traversal rather than one per cell.
- The per-cell, per-property setters remain for the single-property case; both paths now share one
  get-or-create for `a:tcPr`.
- Selecting nothing (`Cells::rectangle(1..1, ..)`) is well-formed and changes nothing; a selection
  reaching past an edge reports the table's real dimensions.

## [0.0.15] - 2026-07-21

A table can be made to look like something.

### Added

- **Cell formatting on `Presentation`** — `cell_fill` / `set_cell_fill` / `clear_cell_fill`,
  `cell_border` / `set_cell_border` / `clear_cell_border` (all six edges, both diagonals included),
  `cell_margins` / `set_cell_margins`, `cell_anchor` / `set_cell_anchor`, and
  `cell_text_direction` / `set_cell_text_direction`.
- **`CellMargins`** (`mjx-pptx`) — the four insets, each optional.
- **`TableCellProperties` can now be written** (`mjx-dml`): `set_border`, `set_fill`, `set_margins`,
  `set_anchor`, `set_text_direction`, `set_horizontal_overflow`, plus the matching typed reads.
- **`TextAnchoring`, `TextDirection`, `TextHorizontalOverflow`** — generated from
  `ST_TextAnchoringType`, `ST_TextVerticalType` and `ST_TextHorzOverflowType`.

### Notes

- **A border is an `a:ln` under another name** — same `CT_LineProperties` content, different tag —
  which is why one `LineSpec` describes all six edges and no border type was needed.
- **Merge, not rebuild.** `a:tcPr` carries a `cell3D`, a `headers` and an `extLst` this tier does not
  model, so a child is replaced in place or inserted at its rank in the schema's sequence. Setting
  one border cannot disturb the other five.
- **Removing a fill is not writing `FillSpec::None`.** The first lets the table style decide again;
  the second states that the cell is deliberately unfilled and stops the style. Same for borders.
- **An unstated margin is absent, not zero.** The schema defaults are `0.1"` horizontally and
  `0.05"` vertically, so the two are different facts; `CellMargins` keeps every field optional, and
  a `None` on write leaves that inset exactly as it was.
- `ST_TextVerticalType` is named **`TextDirection`** because its own values include `horz`
  (Horizontal) — it selects which way text flows, so a "vertical" name would misdescribe most of its
  range. `wordArtVertRtl` is `VerticalWordArtRightToLeft`, the title ECMA gives it, even though it
  reads oddly beside `WordArtVertical`.
- The seven `a:tblPr` flags are deliberately **not** here: they emphasize nothing on their own, they
  tell a table style which parts to treat specially, and they land with the `tableStyles.xml` part.

## [0.0.14] - 2026-07-21

Tables exist on the deck — created, sized, and filled in.

### Added

- **`Presentation::add_table`** — builds the whole `p:graphicFrame`: the grid, every row and every
  cell, ready for text. A table is a shape on the existing index space, so it is positioned with
  `set_shape_bounds` and dropped with `remove_shape`.
- **`table_dimensions`, `column_width` / `set_column_width`, `row_height` / `set_row_height`,
  `cell_span`, `merged_cell_anchor`** — the table's shape, and which cell renders where.
- **Thirteen `cell_*` text methods** — `cell_text`, `set_cell_text`, the paragraph and run readers,
  and the formatting setters including the run-splitting `set_cell_text_range_properties`. Each is
  the corresponding shape method addressed at a cell instead: same operation, same errors.
- **`PptxError::ShapeIsNotATable`, `TableCellOutOfRange`, `InvalidTableSize`.**

### Changed

- The private text-body locator now takes a *site* — a shape's `p:txBody` or a cell's `a:txBody` —
  and every text operation is a named function both spellings call. `shape_text` and
  `set_shape_text` inlined their own copy of the locate and are folded in. No behaviour change; the
  text suites pass untouched.

### Notes

- **A cell's `a:txBody` is the same `CT_TextBody` as a shape's**, which is why the cell surface is
  delegation rather than a second implementation — a future text feature stays one change.
- Reaching a cell **walks the raw tree** rather than parsing the table, so editing one cell costs
  what editing a shape costs; only the addressed `a:txBody` is parsed and rebuilt.
- The column count comes from `a:tblGrid`, never from counting a row's cells.
- A new table's columns share the frame width evenly with the **last absorbing the rounding**, so
  they sum to exactly the frame rather than leaving it a few EMU short.
- A new table carries `firstRow` and `bandRow`, as PowerPoint's does: they claim nothing about
  appearance on their own, they tell a table style which parts to emphasize.
- `set_column_width` does **not** resize the frame — a table whose columns no longer sum to its
  frame is what PowerPoint itself produces when a column is dragged.
- Creating a table adds no parts and no relationships: only the slide changes.
- Effective (inherited) cell formatting is not here — a cell inherits from the table style, which
  needs the `tableStyles.xml` part, later in this workstream.

## [0.0.13] - 2026-07-21

The table, modeled. The first tier of the tables workstream.

### Added

- **`Table`, `TableProperties`, `TableGrid`, `TableColumn`, `TableRow`, `TableCell`,
  `TableCellProperties`** (`mjx-dml`) — `a:tbl` and everything under it, typed for the first time.
  A `p:graphicFrame` could already be positioned; now what it frames can be read.
- **`TablePart`** — the seven `a:tblPr` flags (`firstRow`, `bandRow`, …), which do not draw anything
  themselves but tell the table style which parts to emphasize.
- **`CellBorder`** — the six `CT_LineProperties` edges of a cell, including the two diagonals.

### Notes

- **How little of this is new.** A cell's content is a `CT_TextBody` — the *same* type a shape's
  `p:txBody` is — so the whole text tree and its formatting model apply inside a cell unchanged.
  Cell borders are `LineProperties`; cell and table fills are the fill model; widths, heights and
  margins are `Emu`. The genuinely new part is the two-dimensional shape.
- **Merging never removes a cell.** A merged region is anchored at its top-left cell, which carries
  `gridSpan`/`rowSpan`; every covered cell remains present carrying `hMerge`/`vMerge`. So a row holds
  as many `a:tc` as the grid has `a:gridCol`, `(row, column)` addressing has no holes, and
  `Table::merge_anchor` answers which cell actually renders at a position by walking left then up.
- The **grid** is the authority on column count: `a:tblGrid` is where a table declares its width.
  A table missing it reports no columns rather than inferring one from the rows.
- A cell's four margins have **non-zero schema defaults** (0.1" horizontal, 0.05" vertical), so an
  unstated margin is not a zero one; the accessors report what the file states and the defaults are
  exposed as constants.
- `a:tableStyleId` is **reported but not resolved** — the `tableStyles.xml` part it names is a later
  tier of this workstream.
- Nothing in `mjx-pptx` uses this yet: creating a table, reaching cell text, and formatting cells
  are the next PRs.

## [0.0.12] - 2026-07-21

Where a shape actually renders. The transform workstream is complete.

### Added

- **`Presentation::effective_shape_bounds`** and **`Presentation::effective_shape_transform`** — the
  position a shape *renders* at, not the one it declares. A placeholder that places itself nowhere
  resolves through the same-slot placeholder on its layout, and failing that its master.

### Changed

- The candidate walk every effective property starts with — the addressed shape, then the same-slot
  placeholder on each part the surface inherits from — is now **one** private helper
  (`placeholder_candidates` + `candidate_shape`) rather than a copy inside `effective_shape_fill`,
  `_outline` and `_effects`. Behaviour is unchanged; those suites pass untouched.

### Notes

- **Inheritance is all-or-nothing at the `a:xfrm` level.** Text formatting merges tier by tier, each
  supplying what the ones above left unset; a transform does not. A shape cannot take its position
  from the layout and its size from the master, so the first tier that states anything wins whole.
- **A present-but-empty `<a:xfrm/>` states nothing**, so resolution steps past it exactly as it steps
  past a tier with no transform element at all — what `Transform2D::is_empty` exists for.
- A shape that is **not a placeholder** has no tier to inherit from, so its effective transform is
  its explicit one.
- A tier that answers with only a rotation yields `effective_shape_bounds == None`: bounds are all
  four numbers, and the all-or-nothing rule means no other tier is consulted.
- `tests/fixtures/layouts.pptx`'s `slideLayout2` title placeholder no longer declares an `a:xfrm`,
  so it defers to the master — ordinary in real decks, and the only way the master tier becomes
  reachable. A slide built from that layout now resolves its title at the master and its body at the
  layout.
- `docs/TRANSFORM_HANDOFF.md` closes the workstream; `PLAN.md` now names **tables** and **speaker
  notes** as what remains before `v0.1`.

## [0.0.11] - 2026-07-21

A shape can be moved. The transform reaches the deck.

### Added

- **`Presentation::shape_bounds` / `set_shape_bounds`** — read, move and resize any shape. Until now
  `ShapeBounds` was written once, at shape creation, and could be neither read back nor changed.
- **`Presentation::shape_transform` / `set_shape_transform`** — the whole `a:xfrm`: position, size,
  rotation, the two mirror flags, and a group's child coordinate space. Rotation and flips had no
  expression at all before this.
- **`ShapeBounds::from_transform` / `to_transform`** — the bridge to `mjx_dml::Transform2D`.
- **`PptxError::ShapeCannotBePositioned`** — names the one shape kind (`p:contentPart`) whose schema
  has nowhere to put a transform, instead of reporting a missing element.

### Notes

- **A transform is not in the same place for every shape kind**, which is what made this its own
  piece of work: `p:spPr > a:xfrm` for a shape, picture or connector; `p:grpSpPr > a:xfrm` for a
  group (a `CT_GroupTransform2D`, carrying `a:chOff`/`a:chExt`); and `p:xfrm` for a graphic frame —
  PresentationML's namespace, a direct child, and required rather than optional. Only the wrapper
  differs; the `a:off`/`a:ext` inside are DrawingML in every case.
- **`None` from `shape_bounds` is not "at the origin"** — it means the shape places itself nowhere,
  and a placeholder's real position is on its layout or master. Resolving that is the next PR.
- **Setting bounds cannot disturb anything else.** `to_transform` names only position and size, and
  `Transform2D::apply` writes only named fields, so moving a shape leaves its rotation alone and
  moving a group keeps the child space its members are laid out in. Resizing a group does rescale
  its members — a group maps its child space onto its own extent, which is what PowerPoint does.
- Shape creation now emits its `a:xfrm` through the same writer as shape editing, so the two cannot
  drift apart. The bytes are unchanged.
- `tests/fixtures/layouts.pptx` gained a `p:grpSp` and a `p:graphicFrame` (holding a real one-cell
  table) on slide 2, appended so existing shape indices keep their meaning — the two exotic locator
  paths now meet a real file, and the tables workstream inherits a fixture.
- Group members are still not addressable, so bounds are always in the parent tree's coordinate
  space. Computing an absolute rectangle for a shape inside a group needs group descent.

## [0.0.10] - 2026-07-21

Where a shape sits, and which way up — the model tier of the transform workstream.

### Added

- **`Transform2D`, `Position` and `Size`** (`mjx-dml`) — `a:xfrm` typed for the first time: an offset
  (`a:off`), an extent (`a:ext`), a rotation (`@rot`) and the two mirror flags (`@flipH` / `@flipV`).
  One type covers both `CT_Transform2D` and a group's `CT_GroupTransform2D`, whose `a:chOff` /
  `a:chExt` child coordinate space is the same sequence with two more members.
- **`Transform2D::apply`** — writes only the fields a caller names, editing the element in place.

### Notes

- **Every field is optional, and absent is not zero.** A placeholder that declares no `a:xfrm` is
  asking its layout where it goes; a transform that read as "origin, zero-sized" could not be told
  from one that means *ask someone else*, and the inheritance walk depends on telling them apart.
- `apply` **merges rather than rebuilds**, because an `a:xfrm` carries content this model does not
  describe — a group's child coordinate space, an `extLst`, unknown attributes on the `a:off` itself.
  Rebuilding it wholesale would move every member of a group whose position was changed. New children
  are inserted at their rank in the schema's sequence (`off` → `ext` → `chOff` → `chExt`).
- A transform reads the same whether its wrapper is DrawingML's `a:xfrm` or the `p:xfrm` a
  `p:graphicFrame` holds — the wrapper's namespace differs, its children do not.
- The measure attribute readers/writers (`attr_emu`, `push_angle`, …) moved from `effect.rs` to
  `build.rs`: a measure-valued attribute is not an effect's idea, and now has one spelling on read
  and one on write rather than one per module.
- Nothing in `mjx-pptx` uses this yet — reading and writing a shape's bounds is the next PR.

## [0.0.9] - 2026-07-21

What the text actually renders as. The text-formatting workstream is complete.

### Added

- **`Presentation::effective_run_properties`** and **`Presentation::effective_paragraph_properties`**
  — the formatting a run and a paragraph *render* with, not the formatting they declare. Seven tiers
  resolve, each contributing only what the tiers above left unset: the run's `a:rPr`, the paragraph's
  `a:defRPr`, the shape's `a:lstStyle`, the same-slot placeholder's on the layout and master, the
  master's `p:txStyles`, `p:defaultTextStyle`, and the theme font scheme.
- **`p:txStyles` and `p:defaultTextStyle` are read** for the first time — the tiers where a
  placeholder's real size, bullet and alignment have always lived.

### Notes

- The paragraph's level is read **once**, before the walk, and selects which `a:lvlNpPr` every tier
  from the third down contributes: a level-2 paragraph that declares nothing answers with the master
  `bodyStyle`'s `a:lvl3pPr`.
- Colors bake to concrete `RRGGBB`, consistent with `effective_shape_fill`.
- A shape that is **not a placeholder** takes no master text style; it falls through to
  `p:defaultTextStyle`, as PowerPoint does. A font slot the theme leaves undefined keeps its
  `+mj-lt` reference rather than inventing a font.
- `tests/fixtures/layouts.pptx` gained three distinct `bodyStyle` levels and a layout-placeholder
  `a:lstStyle`, so the level axis and the placeholder tier are demonstrable on a real deck.

## [0.0.8] - 2026-07-21

What "inherited" means, made explicit — the merge one tier of the text-formatting ladder performs.

### Added

- **`CharacterPropertiesSpec::merge_under`** and **`ParagraphPropertiesSpec::merge_under`**
  (`mjx-dml`) — merge a lower inheritance tier under a spec: the receiver is the higher tier and
  wins, and the argument supplies only what the receiver leaves unset. Folding from the top reads as
  the ladder does: `run.merge_under(&paragraph).merge_under(&shape)`.

### Notes

- Properties merge as **whole values**, so an explicit "off" — `b="0"`, `a:noFill`, `<a:buNone/>` —
  is a present value that blocks the tier below rather than an absence that falls through it.
- Four fields are not a plain field-wise fallback: fonts merge **per script slot**, tab stops as one
  **list** (`a:tabLst` replaces wholesale), `a:defRPr` **recursively**, and each of the four bullet
  groups **as a unit**.
- These are the merge halves of effective text formatting; the inheritance walk that calls them
  follows.

## [0.0.7] - 2026-07-21

The theme's font scheme — where a typeface of `+mj-lt` finally leads.

### Added

- **`FontScheme`** (`mjx-dml`) — `a:fontScheme` modeled as `{ name, major, minor }`, on both `Theme`
  and the interner-free `ThemeInfo` (`Theme::font_scheme` / `ThemeInfo::font_scheme`), so a deck's
  font scheme is reachable through the existing `Presentation::theme`.
- **`FontCollection`** — one collection's latin / East Asian / complex-script fonts, keyed by the
  existing `FontSlot` (`FontSlot::Symbol` is always absent: a collection has no `a:sym`), plus its
  `SupplementalFont` per-script fallbacks, looked up by ISO 15924 script tag.
- **Theme font references** — `TextFont::theme_reference` parses the six spellings the schema
  defines (`+mj-lt`, `+mj-ea`, `+mj-cs`, `+mn-lt`, `+mn-ea`, `+mn-cs`) into a `ThemeFontReference`;
  anything else, including other `+…` strings, is not a reference. `FontScheme::resolve` answers
  what a font is actually drawn with — itself when literal, the scheme's font when a reference.

### Notes

- The theme part stays read-only: the font scheme is a parsed value view, with no write path.
- This is the last piece the effective-text-formatting resolution needs; the inheritance walk that
  consumes it follows.

## [0.0.6] - 2026-07-21

Text formatting reaches the deck. Everything the previous four releases modeled is now callable on a
real `.pptx`, at every scope a user can select.

### Added

- **The paragraph axis** on `Presentation` — `paragraph_count`, `run_count`, `paragraph_text`,
  `run_text`. Run indices are paragraph-local, matching the document tree. The existing flat
  `set_shape_text` is unchanged.
- **Reading formatting** — `paragraph_properties`, `run_properties`, `end_run_properties`. Reading
  never dirties a part.
- **Writing formatting, one call per selection granularity**:
  - `set_run_properties` — one run.
  - `set_paragraph_run_properties` — every run in a paragraph, and its paragraph mark.
  - `set_shape_run_properties` — every run in the shape, and every mark.
  - `set_text_range_properties` — an arbitrary character range, splitting runs where the range cuts
    across them.
  - `set_text_range_properties_by_grapheme` — the same, addressed in grapheme clusters, so an emoji
    and its modifier are one unit.
  - `set_paragraph_properties` — a paragraph's layout (alignment, level, margins, spacing, bullet).
  - `set_end_run_properties` — the format of an **empty** paragraph, which is what a placeholder
    added but not yet typed into holds.
- **`TextRun::split_at` / `Paragraph::split_run_at`** in `mjx-dml` — divide a run's text, giving both
  halves the original's formatting, so splitting alone changes nothing about how the text renders.
- **`Paragraph::set_end_properties`** — the write half of the `a:endParaRPr` surface.

### Notes

- Formatting a paragraph or a shape also formats the paragraph mark, so text typed at the end takes
  the same formatting — what "select and restyle" means to a user.
- Runs are split but never merged, keeping each edit minimal. A range already aligned to run
  boundaries splits nothing, so repeated edits do not accumulate runs.

## [0.0.5] - 2026-07-21

Bullets and numbering — the marks that express a deck's paragraph hierarchy.

### Added

- **`Bullet`** — what marks a paragraph: `None` (an explicit "no bullet", which overrides an
  inherited one), `Character` (a literal glyph), `AutoNumber` (a scheme plus where its sequence
  starts), or `Picture` (an image by relationship id).
- **`BulletColor`, `BulletSize`, `BulletTypeface`** — the bullet's colour, size and font, each with a
  `FollowText` variant for the schema's "match the text" arm. All four groups are set and inherited
  **independently**, as the schema defines them.
- **Builder support** on `ParagraphPropertiesSpec`: `with_bullet`, `with_bullet_color`,
  `with_bullet_size`, `with_bullet_typeface`, plus `with_bullet_character("•")` and
  `without_bullet()` for the common cases.

### Notes

- A bullet percentage is written in the form both schemas specify and ECMA §21.1.2.4.9 illustrates
  (`val="111%"`); the integer spelling found in some files is still read.
- Setting one bullet group never disturbs the others, and a group left unnamed keeps whatever the
  file had.

## [0.0.4] - 2026-07-21

Paragraph formatting: how a paragraph is laid out, and the per-level styles it inherits from.

### Added

- **`ParagraphProperties`** (`CT_TextParagraphProperties`) — indent level, alignment, left/right
  margins, first-line indent, default tab size, reading direction and font alignment, plus line
  spacing, space before/after, tab stops, and the `a:defRPr` a paragraph's runs default to. One type
  serves `a:pPr`, `a:defPPr` and `a:lvl1pPr`…`a:lvl9pPr`; the line-breaking attributes, bullets and
  anything unknown round-trip verbatim.
- **`ParagraphPropertiesSpec`** — the builder, matching the character-properties conventions.
  Margins, indents and tab stops are stated **in points**; EMU is the file's unit and stays reachable
  through `Emu`.
- **`IndentLevel`** — the 0–8 nesting level a paragraph's inherited bullet, size and indent are
  selected by. `IndentLevel::of(2)` for a literal, `::new(raw)` for a value off the wire, `::TOP` for
  the outermost.
- **`TextSpacing`** — a proportion of the line height (`a:spcPct`) or a fixed distance (`a:spcPts`),
  kept apart because they are different measurements. **`TabStop`** — position and alignment.
- **`TextListStyle`** (`a:lstStyle`) — the paragraph properties a container offers at each level, by
  `level(IndentLevel)`. The same type covers a shape's own list style, a placeholder's, and each of a
  master's three text styles.
- **Typed access from the text tree** — `Paragraph::properties` / `set_properties` and
  `TextBody::list_style`, so `a:pPr` and `a:lstStyle` are no longer opaque.

## [0.0.3] - 2026-07-20

Text formatting begins: the vocabulary and the run-level model. A run's appearance — its size, weight,
slant, underline, colour, font — can now be read and written. (Reaching it through a `Presentation`,
and resolving what a run *inherits*, come next.)

### Added

- **Text simple types** — `TextUnderline`, `TextStrike`, `TextCapitalization`, `TextAlignment`,
  `FontAlignment`, `TabAlignment` and `AutonumberScheme` (41 bullet-numbering schemes), generated from
  `dml-main.xsd` and named from the ECMA-376 §20.1.10 enumeration tables.
- **`FontSize` and `TextPoint`** — text measures stated **in points** (`from_points` / `points`), the
  unit every size control uses. The file's hundredths of a point are reachable only through
  `from_wire` / `to_wire`.
- **`CharacterProperties`** (`CT_TextCharacterProperties`) — size, bold, italic, underline, strike,
  capitalization, spacing, kerning, baseline, language, plus the text fill, glyph outline, effects,
  highlight and the four script fonts. One type serves `a:rPr`, `a:defRPr` and `a:endParaRPr`, and
  everything it does not model — hyperlinks, `dirty`/`err`/`smtClean`, unknown children — round-trips
  verbatim.
- **`CharacterPropertiesSpec`** — an interner-free builder:
  `CharacterPropertiesSpec::new().with_size_points(28.0).with_bold(true).with_color(…)`. Naming a
  property sets it; leaving it unnamed means *inherit*, so `with_bold(false)` and
  `with_underline(TextUnderline::None)` are how a caller overrides an inherited value.
- **`TextFont`** — a typeface reference, whether a literal name or a `+mj-lt`-style theme reference.
- **`resolve_character_properties`** — bakes a run's colours (text fill, glyph outline, effects,
  highlight) down to concrete RGB against a theme scheme and colour map.
- **Typed access from the text tree** — `TextRun::properties` / `set_properties` and
  `Paragraph::end_properties`, so `a:rPr` and `a:endParaRPr` are no longer opaque.

### Notes

- Setting a run's properties **merges** onto its existing `a:rPr` rather than replacing it, so the
  state this model does not describe (`lang`, `dirty`, a hyperlink) survives a restyle. An unset
  property means "leave it alone", never "clear it".

## [0.0.2] - 2026-07-20

The PowerPoint slice — Phases 2 and 3. A real `.pptx` can now be opened, read, edited, built up from
its own layouts and pruned back down, and written out so PowerPoint and LibreOffice open it with every
untouched part byte-identical. Phase 3 closes here; Word (Phase 4) is next.

### Added

- **De/serialization (Phase 2)** — `FromXml`/`ToXml` in `mjx-ooxml-core::convert` and the
  `#[derive(FromXml, ToXml)]` proc-macro in `mjx-derive`. Every modeled type keeps an unknown-content
  bucket, so what we do not model survives a round trip.
- **DrawingML text (Phase 2)** — `mjx-dml`'s `TextBody`/`Paragraph`/`TextRun`/`Text`, with a mutation
  surface.
- **PresentationML (Phase 2)** — `mjx-pptx::Presentation`: `open`/`save`, slide inventory, shape
  enumeration, `shape_text`/`set_shape_text`, and construction — `add_text_box`, `add_shape`,
  `add_slide`. The **office-open canary** (LibreOffice headless must render the produced deck to a
  valid PDF) became a CI gate.
- **Preset geometry (Phase 3)** — all 187 `ST_ShapeType` values generated, and the 117 adjustable
  shapes given **named, spec-sourced control parameters** (a rounded rectangle exposes
  `corner_radius`, never `adj1`), with the meaning derived from `presetShapeDefinitions.xml`.
- **Color, theme and the `spPr` visual trilogy (Phase 3)** — theme (`clrScheme`/`fmtScheme`) with
  color resolution to concrete RGB, and **fill**, **outline** (`a:ln`) and **effects**
  (`a:effectLst`), each modeled both *explicitly* and *effectively* — resolved through style
  references and placeholder inheritance to what actually renders.
- **Images (Phase 3)** — `add_image` media parts (de-duplicated by content, format identified by
  magic bytes), `add_picture` `p:pic` shapes, and picture read/replace — on one shape index space
  covering every shape kind.
- **Layouts and masters (Phase 3)** — the layout/master inventory, generated PresentationML simple
  types, **`Surface` addressing** (every shape call works on a slide, a layout or a master, so editing
  a layout reaches every slide inheriting it), and `add_slide_from_layout`, which returns a slide
  carrying the layout's placeholders ready to fill.
- **Removal (Phase 3)** — `remove_shape` on any surface, and `remove_slide`, which unwires
  `p:sldIdLst` → relationship → part and takes with it every part only that slide referenced (its
  notes slide, unshared media) while sparing anything the rest of the deck still uses.
- **Packaging** — `Package::{insert_part, remove_part, remove_part_cascading,
  set_content_type_default/override, add_relationship, remove_relationship}` over a copy-on-write part
  body, plus `PartName::{resolve, resolve_from_root, relative_target}` — the part-name algebra Word
  and Excel will share.

### Fixed

- `add_shape` / `add_text_box` built a paragraph with no run, so the shape they returned could not be
  filled by `set_shape_text`. Every paragraph they create now holds exactly one run, blank lines
  included.
- `add_slide_from_layout` cloned the date, footer and slide-number placeholders. Those render *from
  the layout* for slides that do not declare them, so the clones suppressed the layout's rendering and
  showed as empty boxes; they are now skipped, as PowerPoint does.

### Notes

- The round-trip contract is unchanged and continuously asserted: per-part decompressed-payload byte
  identity plus structural container identity. Reading dirties nothing; an edit re-serializes only its
  own part.
- Public API remains unstable until `v0.1`.

## [0.0.1] - 2026-07-15

First versioned snapshot. Establishes the workspace, the packaging + fidelity + compatibility core,
the schema-type generator, and full documentation. No format models yet.

### Added

- **Packaging (Phase 0)** — `mjx-opc`: load an OOXML package fully into RAM as an ordered part graph,
  parse `[Content_Types].xml` and `_rels/*.rels`, and re-zip with per-part decompressed-byte identity.
  Minimal namespace-resolving reader in `mjx-xml`.
- **Schema codegen (Phase 0)** — `xtask` generates `mjx-ooxml-types` (namespace table +
  `shared-commonSimpleTypes`) with comprehensive, self-explanatory names and exact wire tokens;
  output is deterministic and committed.
- **Fidelity layer (Phase 1)** — `mjx-ooxml-core` string interner + the `RawDocument` preservation
  tree, and `mjx-xml::fidelity`, a byte-preserving reader + hand-written writer. Parsing then
  re-serializing any part reproduces the source **byte-for-byte** (verified on real `.pptx`/`.docx`/
  `.xlsx` fixtures).
- **Markup Compatibility (Phase 1)** — `mjx-mce`: preserve mode (the untouched tree) and a
  non-mutating resolve mode (`AlternateContent` Choice/Fallback, `Ignorable`, `ProcessContent`,
  `MustUnderstand`).
- **Documentation** — comprehensive rustdoc across all crates (crate guides + runnable examples), a
  facade docs hub (`mjx-ooxml`), enforced via `missing_docs` and a strict-rustdoc CI job.
- **Project** — CI (fmt/clippy/test + wasm/Android/iOS/macOS/Windows cross-compile build matrix),
  dual `MIT OR Apache-2.0` license, and the contributor/agent guides.

### Notes

- Cross-platform: pure-Rust dependency graph; the library crates cross-compile to
  `wasm32-unknown-unknown`, `aarch64-linux-android`, and Apple/Windows targets.
- A broader multi-producer sample corpus and fuzzing are planned for later iterations.

[0.0.9]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.9
[0.0.8]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.8
[0.0.7]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.7
[0.0.6]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.6
[0.0.5]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.5
[0.0.4]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.4
[0.0.3]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.3
[0.0.2]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.2
[0.0.1]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.1
