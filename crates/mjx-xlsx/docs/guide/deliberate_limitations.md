# Deliberate limitations

**Read this before filing a bug.** Everything on this page is a scope decision with a reason beside
it, not an omission waiting to be closed. Each entry says what the library does instead and what a
caller can do about it.

There is also a second list at the end: things that are genuinely absent and that no work item owns.
Those are gaps rather than decisions, and they are named as gaps.

## First: which crate is which

Excel is **two** crates, and a caller will meet both.

| Crate | Owns | Reach for it when |
|---|---|---|
| **`mjx-sml`** | SpreadsheetML *markup*: what a cell, a row, a shared string, an `xf`, a formula or a conditional-formatting rule **is** | You are holding a [`mjx_sml::WorksheetPart`] and want the model under it |
| **`mjx-xlsx`** (here) | OPC structure: parts, content types, relationships, the ZIP, and the [`Workbook`] a caller holds | You have a `.xlsx` and want to open, read, edit or save it |

The split is not tidiness. An authored PowerPoint chart embeds a whole `.xlsx` at
`/ppt/embeddings/*.xlsx` — that package is what **Edit Data** opens — so `mjx-chart` needs a
SpreadsheetML writer, and `mjx-chart` sits in the shared-markup tier where an edge to a format crate
would point *upward*. Putting the markup one tier down makes `mjx-chart → mjx-sml → mjx-dml` a chain
of legal downward edges. `xtask/tests/layering.rs` checks it against the real dependency graph rather
than trusting this paragraph.

**Start with `mjx-xlsx`.** Everything on the [Opening and saving](opening_and_saving) page is here,
and `mjx-sml` is what you reach when a report does not carry what you need.

## Nothing here is ever evaluated

The three limitations below are one decision wearing three hats: **this library has no calculation
engine, and there will not be one.** `PLAN.md` records it as settled scope. What follows from that is
worth spelling out, because each of the three looks like a defect from a different angle.

### 1 · A cached value goes stale after an edit

A formula's `<v>` is the result a producer last computed. Change a cell that formula depends on, and
the `<v>` stays exactly as it was — out of date, and byte-identical to what was read:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

let reference = |text: &str| CellReference::parse(text).expect("a reference");
let mut workbook = Workbook::open(&mjx_fixtures::fixture("formulas.xlsx"))?;
// B2 holds `=A2*2`; A2 holds 1; the cached result is 2.
workbook.set_cell_value(0, reference("A2"), CellValue::Number(50.0))?;
assert_eq!(workbook.cell_text(0, reference("B2"))?.as_deref(), Some("2"));
# Ok(())
# }
```

**Why not blank the `<v>` instead?** Because it destroys data in a file the caller opened to change a
label, in cells they never named, and the saved file cannot be undone. **Why not set
`fullCalcOnLoad`?** Because it writes into a part the caller did not ask to edit. Excel recalculates
on open when it needs to.

**What you can do:** set it yourself, deliberately, through
[`mjx_sml::CalculationProperties`](mjx_sml::CalculationProperties) — the decision is available, it is
simply not made on your behalf. The whole story, shared groups included, is on
[Formulas and cached values](formulas_and_cached_values).

### 2 · A conditional-formatting rule is reported here, and evaluated one tier up

⚠ **This refusal was absolute until MJXOFF-173 and is now scoped.** What follows is true of *this
library* — the reader and writer — and is no longer true of the workspace.

[`Workbook::conditional_rules_for`] answers **which rules apply to a cell**, merged across blocks and
in priority order, and [`Workbook::conditional_cell_format`] reports the `dxf` each would impose —
*beside* the base format, never folded into it. What none of them answers is **whether a rule's
condition is true**, because a `cfRule/formula` is a formula on exactly the terms above.

`stopIfTrue` is reported as a position in the chain rather than applied as a truncation, for the same
reason: applying it means knowing which earlier rule fired.

**Why it stops here.** A rule that has been *decided* is a rendering fact, not a document fact.
Folding a `dxf` into a cell's format inside this crate would put an answer into the write path that
depends on the cell's current value — so a workbook opened, resolved and saved would carry formats
its author never wrote. That is a fidelity failure, and it is the reason the two layers are
[deliberately never merged](conditional_formatting).

**Where it is evaluated.** `mjx-layout-xlsx`'s `condfmt` module (rank 3.6, above the format tier)
decides all eighteen members of `ST_CfType`, composes the `dxf` layers in `@priority` order, applies
`stopIfTrue`, and interpolates colour scales, data bars and icon sets. It consumes the two accessors
above and re-models nothing.

**Two paths there stay `partial`, and are recorded rather than faked.** An `expression` rule's
condition is a formula, and so is a `cellIs` operand or a `cfvo` `@val` that is not a literal. Those
are reported unevaluated, with their reason and their text, and **nothing is painted** for them —
because a rule that quietly did not fire and one that could not be evaluated look identical on a
screen. `crates/mjx-layout-xlsx/tests/the_conditional_ledger_is_computed.rs` prints that ledger on
every run rather than restating it in prose that could rot.

**What you can do here:** read the rules, evaluate them yourself against the cell values this library
does give you, and decide — or reach for the box model, which has.
[Conditional formatting](conditional_formatting) has the ordering rules and the `x14` extension slot.

### 3 · A filter, a sort and a validation rule are recorded, never applied

Setting an autofilter hides no row. Removing one unhides none. A recorded `sortState` is the sort a
producer last performed, not an instruction to perform it. A data-validation rule is a constraint
this library will never enforce against a value you write.

Row visibility is `row@hidden`, which is the file's own statement; writing it because a filter was
added would edit cells nobody named. `@filterVal` on a `top10` and `@val`/`@maxVal` on a
`dynamicFilter` are Excel's caches of bounds it derived, reported and never recomputed — the same
rule a formula's cached value follows.

**What you can do:** set `row@hidden` yourself through [`Workbook::set_row_hidden`] if hiding is what
you want. [Filters and data validation](filters_and_data_validation) has the six filter kinds.

## Half of `sml.xsd` is preserved rather than modelled

`sml.xsd` declares **367** complex types. Nine clusters — **184 of them** — describe features this
library recognises, reports the identity of, and does not model: pivot tables and their caches,
shared-workbook revisions, external workbook references, cell metadata, data connections, query
tables, volatile dependencies, single-cell XML tables and custom XML mappings.

**Preserved is not ignored, and it is not lost.** Every part of every one of those clusters comes back
byte for byte through an unrelated edit, and [`Workbook::preserved_parts`] inventories them from
relationships alone with no markup parsed at all. [`Workbook::pivot_tables`],
[`Workbook::external_links`], [`Workbook::connections`], [`Workbook::query_tables`],
[`Workbook::xml_maps`] and [`Workbook::revision_state`] report what each says.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("preserved_parts.xlsx"))?;
let pivot_tables = workbook.pivot_tables()?;
// The name, the sheet, the range and the cache — without a ninety-seven-type model behind it.
assert!(!pivot_tables.is_empty());
for table in &pivot_tables {
    assert!(!table.sheet_name.is_empty(), "every pivot table names the tab it sits on");
    assert!(table.cache_definition_part.is_some(), "and the cache it reads");
}
# Ok(())
# }
```

The cluster-by-cluster table, each row with the reason it is not modelled and what you can still ask,
is on [Fidelity and the part graph](fidelity_and_the_part_graph). **A documented gap is not a
validation failure**: nothing on that table is a defect to be filed.

Inside a worksheet the same rule applies at slot level: `CT_Worksheet` has **thirty-nine** slots,
**thirty-five** are modelled and **four** are held as the markup the file wrote — `phoneticPr`, the
header/footer halves of the drawing family (`legacyDrawingHF`, `drawingHF`), and `extLst`.
`legacyDrawing` at rank 30 joined the modelled set with cell comments; see
[Cell comments and legacy content](cell_comments_and_legacy_content).

## Two more standing refusals

**No I/O, ever.** [`Workbook::open`] takes `&[u8]` and [`save`](Workbook::save) returns `Vec<u8>`.
Nothing here reads a file, opens a socket or reads a clock, which is also why the same code compiles
to WebAssembly. So an external hyperlink is never fetched, an `externalLink`'s workbook is never
opened, and a `queryTable`'s `refreshOnLoad` is reported and never obeyed.

**Nothing is repaired on read.** A `dimension` that disagrees with the cells, a duplicate `row@r`, a
merge that overlaps another, a table whose `totalsRowCount` does not fit inside its `@ref` — each is
reported (as a [`SpreadsheetDefect`], as a `GridAnomaly`, or as `None` from a method that cannot
answer) and none is corrected. Correcting a file to match what this library expects is how a
fidelity library loses the argument it exists to win.

## Gaps rather than decisions

These are absent, they are not the consequence of a decision above, and no work item owns them today.
They are listed here so nobody plans around a surface that is not present.

| Absent | What exists instead |
|---|---|
| **Writing a formula into a cell.** [`mjx_sml::CellFormula`] is a read-only view over a cell's `<f>` bytes; there is no `set_cell_formula` on [`Workbook`] or on [`mjx_sml::SheetData`] | A formula's text round-trips because nothing rewrites a cell it was not asked to. Authoring one means writing the `<c>` markup yourself |
| **Removing a sheet.** [`Workbook::add_sheet`] has no opposite | Removing a tab means removing a part, its relationship, its entry and every defined name scoped to it — a decision rather than a convenience method |
| **Authoring a theme part.** [`Workbook::blank`] writes no `xl/theme/theme1.xml` | An indexed or `rgb` colour needs no theme; a `theme`-referencing colour in a file you opened resolves against the theme that file carries |
| **A `DocumentDefect` equivalent for `mjx-docx`** | `mjx-xlsx` and `mjx-pptx` both report structural anomalies rather than repairing them; Word does not yet |

## Built, not yet verified against Excel

A third list, and it is neither of the two above. Everything here **works** and is tested against
markup *we wrote*; what none of it has is a run through real Microsoft Excel. It is not a limitation
and not a defect — it is a statement about the evidence.

Every fixture under `tests/fixtures/` was written by this project or by LibreOffice, so every gate in
the workspace proves that our reader agrees with our writer. What now exists is the **road** for
changing that: `tests/office-authored/` with its redistribution rule,
`cargo run -p xtask -- validation-artefacts --ingest` to report on a file before it is committed, and
`xtask/tests/office_corpus.rs`, which holds whatever lands there to per-part byte identity at the
container *and* through [`Workbook`], to the fidelity tree, to `mjx_opc::Package::validate` and to the
child-order audit. **The corpus is empty**, and no agent may fill it: a file's value here is entirely
its provenance. `docs/validation/06-the-office-pass.md` is how a person with Excel fills it.

| Not yet verified | What is in place | Where the check lives |
|---|---|---|
| **Whether Excel is content with a workbook authored from nothing** | [`Workbook::blank`] writes `workbook.xml`, one worksheet, `styles.xml`, both `docProps` parts and no theme; every part is schema-valid, in child order, and reopens unchanged | `V-XLSX-01.1` — and the Excel list's own note is that the format is the least forgiving of the three about structural detail, so a repair prompt here invalidates everything under it |
| **The two-layer `xf` indirection**, resolved the way Excel resolves it | [`Workbook::effective_cell_format`] walks `cellXfs` over `cellStyleXfs` from ECMA-376 §18.8.45's prose; `docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md` holds 28 rows of it with its **Excel says** column deliberately empty | `V-XLSX-02.1`, `V-XLSX-02.2`, `V-XLSX-02.4` |
| **A number-format code against what Excel actually renders** | The code and its id are reported; nothing here formats a value | `V-XLSX-02.3` |
| **Shared and array formulas surviving an edit** | The group's master and its `@ref` round-trip, and an edit never rewrites a cell it was not asked to | `V-XLSX-01.4` |
| **Conditional-formatting rule priority** | `@priority` is read and reported here, never resolved — a rule is described, not evaluated. It **is** resolved one tier up, in `mjx-layout-xlsx`, whose own gate proves the composition order on two overlapping rules whose wrong order gives a different colour | `V-XLSX-02.5` |
| **A chart reading a live range** | The chart's cached values and its `c:f` references are both preserved; Excel recalculates from the range | `V-XLSX-04.1`, `V-XLSX-04.2` |

**One deviation is expected on the first real workbook, and it is ours.** The schema gate validates
the markup-compatibility-resolved view of a part; resolution removes an ignorable element together
with its content; and `sml.xsd`'s `CT_Extension` declares its wildcard as a bare
`<xsd:any processContents="lax"/>`, whose `minOccurs` therefore defaults to 1. Every modern Excel file
writes `x14`/`x15` extensions under `mc:Ignorable`, so the emptied `<ext>` is rejected with *Missing
child element(s)*. That is a defect in how the two compose — not a property of the file, and
deliberately not a tolerance — filed as **MJXOFF-196**, and reproduced from markup authored for the
purpose in `xtask/tests/office_corpus.rs`.

## Where each of these is written down

| Limitation | Page |
|---|---|
| Stale cached values, shared groups, `calcChain` | [Formulas and cached values](formulas_and_cached_values) |
| Rules reported, conditions unevaluated *in this crate* | [Conditional formatting](conditional_formatting) |
| Filters, sorts and validation recorded, not applied | [Filters and data validation](filters_and_data_validation) |
| The nine unmodelled clusters, and what a save refuses | [Fidelity and the part graph](fidelity_and_the_part_graph) |
| Held-not-repaired grid geometry | [The sheet grid](the_sheet_grid) |
| What an edit costs at scale | [Large workbooks](large_workbooks) |
