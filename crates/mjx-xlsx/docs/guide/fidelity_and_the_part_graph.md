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

Most of it. `mjx-sml` now models cells (MJXOFF-95), shared strings (MJXOFF-97), the workbook part
(MJXOFF-100) and the worksheet's own thirty-nine slot frame (MJXOFF-102) — but **thirty-two of those
thirty-nine slots are held as the markup the file wrote, not modelled**: merged cells, conditional
formatting, data validation, hyperlinks, print setup, drawings, tables and the rest. MJXOFF-105 (D08)
through MJXOFF-133 (D18) fill them, and each module's own documentation names the child that does.
Styles are modelled as of MJXOFF-105 and MJXOFF-108, and formulas as of MJXOFF-115 — as **text**,
which is the whole of what this workspace ever does with one; see the section below.

Held is not dropped. A worksheet whose `pageSetup` survives a save is proof the frame works, not
proof `pageSetup` was modelled, and that is exactly what the round-trip suites check.

Two things are not modelled *and will not be*, and are recorded rather than left to be discovered:
the macro-enabled content types (`macroEnabled` appears nowhere in ECMA-376, so this crate declines
to guess the string — the workbook part is found by its root element instead), and the shared-workbook
revision parts, which MJXOFF-133 (D18) writes down as deliberately out of scope. Both are still
preserved byte for byte.

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

The last four are checked only over the markup **this library will write** — `mjx-opc` defines that
set and this crate does not get to disagree with it — so a workbook opened and saved untouched is
never faulted for markup it arrived with. The first two are graph invariants and are checked over the
whole package, exactly as `mjx-opc`'s own relationship checks are.

[`Workbook::save_unchecked`] skips all of it, for writing back a container that arrived broken.

## What "byte-identical" does not mean

It does not mean the two ZIP files are identical. Compression settings, entry metadata and the
central directory may differ; the contract is per-part **decompressed**-payload identity plus
structural container identity, which is the thing a consumer actually reads.
