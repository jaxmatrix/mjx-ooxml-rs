# Fidelity and the part graph

Read this before relying on anything here in production. It says what this crate guarantees, what it
does not model, and what it will refuse to write.

## The guarantee

**A part nothing dirtied re-emits its decompressed bytes verbatim, and the container's entry set and
order are unchanged.** That is the whole of MJXOFF-91's deliverable, and it is proved part by part —
never by a container hash, which would pass a container whose parts were all subtly rewritten in
compensating ways — in `crates/mjx-xlsx/tests/roundtrip.rs`, over every `.xlsx` in the committed
fixture corpus.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_opc::Package;
use mjx_xlsx::Workbook;

let original = mjx_fixtures::fixture("sample.xlsx");
let saved = Workbook::open(&original)?.save()?;

let before = Package::open(&original)?;
let after = Package::open(&saved)?;
for (before, after) in before.entries().iter().zip(after.entries()) {
    assert_eq!(before.name, after.name);
    assert_eq!(before.bytes(), after.bytes(), "{} changed", before.name);
}
# Ok(())
# }
```

One call in this crate's surface can dirty a part: [`Workbook::set_cell_value`] (MJXOFF-102), and it
dirties exactly the worksheet it was pointed at. That is the constraint every later Phase D child
inherits, and it is checked rather than asserted — `crates/mjx-xlsx/tests/worksheet_part.rs` sets one
cell of `worksheet_spine.xlsx` and requires the list of parts whose bytes changed to be **exactly**
`["/xl/worksheets/sheet1.xml"]`, with every other worksheet child inside that part still equal to the
file's own bytes.

Reading is still not mutating: [`Workbook::worksheet_markup`] takes `&self`, reads the part's bytes,
and leaves the package holding them.

## Classification is not a gate

[`Workbook::part_inventory`] reports what this crate made of each part. A part it cannot classify is
reported [`PartClassification::Unclassified`] and is carried through a save untouched — it is never
an error, and never a reason to refuse a file.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::{PartClassification, PartKind, Workbook};

let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let inventory = workbook.part_inventory();

let workbook_row = inventory
    .iter()
    .find(|row| row.part.as_str() == "/xl/workbook.xml")
    .expect("the workbook part is in the inventory");
assert_eq!(
    workbook_row.classification,
    PartClassification::Classified(PartKind::Workbook),
);

// Document properties are an OPC concept, not a SpreadsheetML one: this crate does not classify
// them, and they round-trip untouched all the same.
let core_properties = inventory
    .iter()
    .find(|row| row.part.as_str() == "/docProps/core.xml")
    .expect("the fixture carries one");
assert_eq!(core_properties.classification, PartClassification::Unclassified);
# Ok(())
# }
```

## What is deliberately not modelled

### Inside a worksheet: eight of thirty-nine slots

`mjx-sml` models cells (MJXOFF-95), shared strings (MJXOFF-97), the workbook part (MJXOFF-100) and
thirty-one of the worksheet's thirty-nine slots. **Eight are held as the markup the file wrote, not
modelled**: `phoneticPr`, the drawing family (`drawing`, `legacyDrawing`, `legacyDrawingHF`,
`drawingHF`, `oleObjects`, `controls`) and `extLst`.
`crates/mjx-sml/src/worksheet/frame.rs` names the disposition of every one — including the three
(`phoneticPr`, `legacyDrawingHF`, `drawingHF`) that belong to no ticket at all, each with the reason
it does not.

Held is not dropped. A worksheet whose `pageSetup` survives a save is proof the frame works, not
proof `pageSetup` was modelled, and that is exactly what the round-trip suites check.

### Whole parts: half of `sml.xsd`

`sml.xsd` declares **367** complex types. Nine clusters of them — **184 types, half the schema** —
describe features this library recognises, preserves and does not model. That is a decision, written
down here so that nobody has to re-derive it and so that a validation pass can look each one up
rather than file it.

| Cluster | Types | Parts | Why it is not modelled | What you can still ask |
|---|---|---|---|---|
| **Pivot tables and caches** | **97** | `pivotTableDefinition`, `pivotCacheDefinition`, `pivotCacheRecords` | It is **derived data of a calculation model this library does not have**. A cache is a snapshot of a source range and a table is an aggregation over that snapshot; a model that held all ninety-seven types and could not say what one data field aggregated to would be decoration over bytes that already round-trip. **If it is ever modelled it is a phase of its own**, with a calculation model beneath it — not a gap for a later child to close | [`Workbook::pivot_tables`]: the name, the sheet, the `CT_Location@ref` range, the cache and every part |
| **Shared-workbook revisions** | 22 | `headers`, `revisions`, `users` | A revision log is a list of undo records; replaying one means recomputing every cell it touches. **This library never writes a revision either** — editing a cell appends nothing to a log, so a workbook saved after an edit has a history that no longer describes it. The logs' bytes survive; their meaning is the file's | [`Workbook::revision_state`]: whether the workbook is shared, its sessions, and who shares it |
| **External workbook references** | 18 | `externalLink` | Resolving one means **opening another workbook** — I/O, and a programme non-goal | [`Workbook::external_links`]: which book, its cached sheet names, the URI, and the `[n]` index a formula uses |
| **Cell metadata** | 16 | `metadata` | OLAP cell provenance, meaningful only to a consumer that can evaluate the MDX it names | Presence and part name, through [`Workbook::preserved_parts`] |
| **Data connections** | 12 | `connections` | Refreshing one means running a query against a database, a web page or a cube. Credentials the part carries are **deliberately not surfaced** | [`Workbook::connections`]: each connection's id, name, description and stated source |
| **Query tables** | 6 | `queryTable` | The same, on behalf of a connection. `refreshOnLoad` is **reported, never obeyed** | [`Workbook::query_tables`]: the name, the connection it reads, and its refresh flag |
| **Volatile dependencies** | 5 | `volTypes` | A real-time-data dependency graph, which only a calculation engine can evaluate | Presence and part name |
| **Single-cell XML tables** | 4 | `singleXmlCells` | The cell end of an XML map; modellable, and simply not worth its weight against what a caller asks for | Presence and part name |
| **Custom XML mappings** | 4 | `MapInfo` | A `Schema` entry's body is **an XML Schema document in another language**; modelling it means modelling XSD | [`Workbook::xml_maps`]: the selection namespaces, the schema ids, and each map's name and root element |

Two more parts are recognised and never opened, for reasons the specification gives rather than this
library: a **Custom Property** part (§12.3.5) carries *"any content, support for which is
application-defined"*, and a **printer settings** part (§15.2.13) carries a blob on which the
specification places no requirement at all.

**Preserved is not ignored.** Every one of these part kinds has a named test proving its bytes
survive an edit — `crates/mjx-xlsx/tests/preserved_parts.rs`, which edits a cell on the very sheet a
pivot table sits on and then compares all fourteen preserved parts of
`tests/fixtures/preserved_parts.xlsx` against the bytes they went in with, byte for byte.

**A documented gap is not a validation failure.** Nothing on this table is a defect to be filed; each
row is a scope decision with its reason beside it.

One further thing is not modelled and will not be: the macro-enabled content types. `macroEnabled`
appears nowhere in ECMA-376, so this crate declines to guess the string — a `.xlsm` still opens,
because the workbook part is found by its root element instead.

## A cached value goes stale, and that is deliberate

**This is the one behaviour on this page that looks like a defect and is not.** A formula's `<v>` is
the result a producer last computed. Change a cell that formula depends on, and this library leaves
the `<v>` exactly as it was — out of date, and byte-identical to what was read:

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

The three things a library could do instead were each considered and rejected:

| Instead | Why not |
|---|---|
| Recalculate | There is no calculation engine here and there will not be one; `PLAN.md` settles it as scope |
| Blank the `<v>` | It destroys data in a file the caller opened to change a label, in cells they never named, and the saved file cannot be undone |
| Mark the workbook dirty for calculation | It writes `fullCalcOnLoad` into a part the caller did not ask to edit. If you want that, set it yourself through `mjx_sml::CalculationProperties` |

Excel recalculates on open when it needs to. The same rule covers `xl/calcChain.xml`, which is left
exactly as found, and `x:dimension`, which is reported as written rather than recomputed on a read.
[The formulas page](formulas_and_cached_values) has the whole of it, shared groups included.

### The same boundary, twice more

**A conditional-formatting rule is reported, never resolved (MJXOFF-120).**
[`Workbook::conditional_rules_for`] answers *which* rules apply to a cell, merged across blocks and
in priority order, and [`Workbook::conditional_cell_format`] reports the `dxf` each would impose —
beside the base format, never folded into it. Whether a rule's condition is **true** is not answered,
because a `cfRule/formula` is a formula on exactly the terms above. `stopIfTrue` is reported as a
position in the chain rather than applied as a truncation, for the same reason: applying it means
knowing which earlier rule fired. `@priority` is preserved exactly — gaps and duplicates included,
never renumbered — and `dxfs` is appended to, never reordered.

**A filter, a sort and a validation rule are recorded, never applied (MJXOFF-123).** Setting an
autofilter hides no row; removing one unhides none. Row visibility is `row@hidden`, which is the
file's own statement, and writing it because a filter was added would edit cells nobody named. A
recorded `sortState` is the sort a producer last performed, not an instruction to perform it, and a
data-validation rule is a constraint this library never enforces against a value you write.
`@filterVal` on a `top10` and `@val`/`@maxVal` on a `dynamicFilter` are Excel's caches of bounds it
derived — reported, never recomputed, exactly as a cached value is.

| Deliberate limitation | Child | Where it is documented in full |
|---|---|---|
| No calculation engine: a cached `<v>` goes stale after an edit | MJXOFF-115 (D11) | [Formulas and cached values](formulas_and_cached_values) |
| Conditional-formatting conditions are never evaluated | MJXOFF-120 (D13) | [Conditional formatting](conditional_formatting) |
| Filters, sorts and validation rules are never applied | MJXOFF-123 (D14) | [Filters and data validation](filters_and_data_validation) |
| Half of `sml.xsd` preserved rather than modelled | MJXOFF-133 (D18) | the table above |

All four are gathered, with their reasons and their workarounds, on
[Deliberate limitations](deliberate_limitations) — the page to read before filing a bug.

## What a save refuses

[`Workbook::save`] runs [`Workbook::validate`] first. On top of `mjx-opc`'s packaging invariants,
this crate checks what only SpreadsheetML knows:

| Refused | Because |
|---|---|
| The package-root `officeDocument` relationship no longer names the workbook part | §12.3.23: a consumer finds the workbook through that one edge and nowhere else |
| A `…spreadsheetml.*` part no relationship chain from the root reaches | Such a part has no consumer but the workbook graph; an unreachable `sharedStrings.xml` makes every `t="s"` cell index into nothing |
| A `x:sheet` entry whose relationship leads to a part that is not a sheet | §12.3.24: the `r:id` "shall reference the desired worksheet part" |
| A sheet part the workbook relates to that `x:sheets` never lists | A tab no consumer will ever show |
| Two `x:sheet` entries sharing an `@sheetId`, a `@name`, or an `r:id` | §18.2.19: both identifiers "shall be unique", and one part is one tab |
| A `x:pivotCache` or `x:externalReference` whose relationship leads to the wrong kind of part | §12.3.12 and §12.3.9 say what each edge reaches; the parts are preserved and unmodelled, which is exactly why nothing else here would notice one going stale |

The last five are checked only over the markup **this library will write** — `mjx-opc` defines that
set and this crate does not get to disagree with it — so a workbook opened and saved untouched is
never faulted for markup it arrived with. The first two are graph invariants and are checked over the
whole package, exactly as `mjx-opc`'s own relationship checks are.

[`Workbook::save_unchecked`] skips all of it, for writing back a container that arrived broken.

## What "byte-identical" does not mean

It does not mean the two ZIP files are identical. Compression settings, entry metadata and the
central directory may differ; the contract is per-part **decompressed**-payload identity plus
structural container identity, which is the thing a consumer actually reads.
