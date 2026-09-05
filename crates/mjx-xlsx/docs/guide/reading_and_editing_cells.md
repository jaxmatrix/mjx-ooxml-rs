# Reading and editing cells

A worksheet is reached by its **tab index** — its position in the workbook's `x:sheets` list, which
is tab order. [`Workbook::sheets`] gives you the list; everything on this page takes an index into
it.

## One value out

[`Workbook::cell_text`] is the shortest path from a container to a string. It resolves a shared
string through `xl/sharedStrings.xml` for you, which is the one thing the markup layer cannot do on
its own: a `t="s"` cell holds an *index into another part*, and `mjx-sml` has never heard of a
package.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let a1 = CellReference::parse("A1").expect("A1 parses");

// A1 is `t="s"`, so its value in the part is the index `0`; the text is in another part.
assert_eq!(workbook.cell_text(0, a1)?.as_deref(), Some("name"));
// A number answers from its own `<v>`, with no other part involved.
let c2 = CellReference::parse("C2").expect("C2 parses");
assert_eq!(workbook.cell_text(0, c2)?.as_deref(), Some("9.99"));
// A cell nothing populated is absent.
let z99 = CellReference::parse("Z99").expect("Z99 parses");
assert_eq!(workbook.cell_text(0, z99)?, None);
# Ok(())
# }
```

`None` means *this cell is not populated*, which is not the same as an empty string: a cell holding
`<v></v>` answers `Some("")`.

## The whole sheet

[`Workbook::worksheet_markup`] hands back an owned [`mjx_sml::WorksheetPart`] — the sheet's
thirty-nine slot `CT_Worksheet`, with its cell store inside. It takes `&self` and dirties nothing.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let sheet = workbook.worksheet_markup(0)?.expect("tab 0 is a worksheet");

assert_eq!(sheet.row_count(), 3);
assert_eq!(sheet.cell_count(), 9);
assert_eq!(sheet.cells().count(), 9);
assert_eq!(sheet.rows().count(), 3);

let b2 = CellReference::parse("B2").expect("B2 parses");
assert_eq!(sheet.cell(b2).expect("B2 is populated").number(), Some(3.0));

// The cached bounding box, as the file wrote it.
let dimension = sheet.dimension().expect("sample.xlsx writes one");
assert_eq!(
    dimension.range(sheet.interner()).expect("@ref").text().as_str(),
    "A1:C3",
);
# Ok(())
# }
```

`Ok(None)` from `worksheet_markup` means the tab is **not a worksheet** — a chartsheet or a
dialogsheet. That is an answer, not a failure: such a workbook opens, and
[`Sheet::kind`](crate::Sheet) says which kind it is.

### `dimension` is a cached value

`<dimension ref="A1:C3"/>` is a cached bounding box, in exactly the sense a formula's `<v>` is a
cached result. This library reports it as the file wrote it and never recomputes it on a read, even
where the cells disagree — Excel repairs a wrong one silently, so a "helpful" recompute would cost
fidelity and hide nothing.

Two things do change it: writing a cell outside the recorded box widens it (the stale cache would
otherwise be *this library's*), and `mjx_sml::WorksheetPart::recompute_dimension` replaces it on
request.

## One value in

[`Workbook::set_cell_value`] reads the part, sets the cell, and writes the part back.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let b2 = CellReference::parse("B2").expect("B2 parses");
workbook.set_cell_value(0, b2, CellValue::Number(42.0))?;

// Read it back out of the saved container, not out of the model that was just written.
let reopened = Workbook::open(&workbook.save()?)?;
let sheet = reopened.worksheet_markup(0)?.expect("a worksheet");
assert_eq!(sheet.cell(b2).expect("B2 is populated").number(), Some(42.0));
assert_eq!(sheet.cell_count(), 9, "the other eight cells are untouched");
# Ok(())
# }
```

**Only the worksheet changes.** Every other part of the container keeps its bytes; inside the
worksheet, every row but the edited one and every child but `sheetData` keep theirs. That is not a
promise this page is making on the library's behalf — `crates/mjx-xlsx/tests/worksheet_part.rs`
requires the set of parts whose bytes differ after an edit to be exactly one, and compares the
untouched children against the committed fixture's own bytes.

For anything beyond one cell, take the model out with `worksheet_markup`, edit it, and put it back
with [`Workbook::write_worksheet_markup`] — one read and one write instead of one of each per cell.

## What a worksheet holds that this crate does not yet model

`CT_Worksheet` has thirty-nine slots and **seven** of them are modelled: `sheetPr`, `dimension`,
`sheetViews`, `sheetFormatPr`, `cols`, `sheetData` and `sheetCalcPr`. The other thirty-two —
`mergeCells`, `conditionalFormatting`, `dataValidations`, `hyperlinks`, `pageSetup`, `headerFooter`,
`drawing`, `tableParts` and the rest — are held as the markup the producer wrote, in the position it
wrote it, and come back byte for byte.

So you can open a workbook with conditional formatting on it, change a number, and save: the
conditional formatting is still there, still exactly as Excel wrote it. You just cannot ask this
library *what* it says yet.
