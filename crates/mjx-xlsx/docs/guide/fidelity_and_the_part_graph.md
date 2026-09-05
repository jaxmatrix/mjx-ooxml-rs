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

Nothing in this crate's current surface can dirty a part, so the guarantee is easy to keep today. It
is written down here because it is the constraint every later Phase D child inherits: the moment
something *does* dirty a part, only that part may change.

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

Everything. There is no cell, no shared string, no style and no formula in this crate or in `mjx-sml`
yet — MJXOFF-95 (D04) through MJXOFF-129 (D17) build them, and each module's own documentation names
the child that fills it. What exists today is the package a model is reached through.

Two things are not modelled *and will not be*, and are recorded rather than left to be discovered:
the macro-enabled content types (`macroEnabled` appears nowhere in ECMA-376, so this crate declines
to guess the string — the workbook part is found by its root element instead), and the shared-workbook
revision parts, which MJXOFF-133 (D18) writes down as deliberately out of scope. Both are still
preserved byte for byte.

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
