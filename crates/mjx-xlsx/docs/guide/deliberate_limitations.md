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

### 2 · A conditional-formatting rule is reported, never resolved

[`Workbook::conditional_rules_for`] answers **which rules apply to a cell**, merged across blocks and
in priority order, and [`Workbook::conditional_cell_format`] reports the `dxf` each would impose —
*beside* the base format, never folded into it. What none of them answers is **whether a rule's
condition is true**, because a `cfRule/formula` is a formula on exactly the terms above.

`stopIfTrue` is reported as a position in the chain rather than applied as a truncation, for the same
reason: applying it means knowing which earlier rule fired.

**What you can do:** read the rules, evaluate them yourself against the cell values this library does
give you, and decide. [Conditional formatting](conditional_formatting) has the ordering rules and the
`x14` extension slot.

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
**thirty-four** are modelled and **five** are held as the markup the file wrote — `phoneticPr`, the
legacy and header/footer halves of the drawing family (`legacyDrawing`, `legacyDrawingHF`,
`drawingHF`), and `extLst`.

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

## Where each of these is written down

| Limitation | Page |
|---|---|
| Stale cached values, shared groups, `calcChain` | [Formulas and cached values](formulas_and_cached_values) |
| Rules reported, conditions unevaluated | [Conditional formatting](conditional_formatting) |
| Filters, sorts and validation recorded, not applied | [Filters and data validation](filters_and_data_validation) |
| The nine unmodelled clusters, and what a save refuses | [Fidelity and the part graph](fidelity_and_the_part_graph) |
| Held-not-repaired grid geometry | [The sheet grid](the_sheet_grid) |
| What an edit costs at scale | [Large workbooks](large_workbooks) |
