# The sheet grid

Merged ranges, row and column geometry, outline levels, page breaks, sheet protection and scenarios —
everything that changes what the grid *is*, rather than what a cell contains.

## A merge is a list, and it touches no cell

SpreadsheetML records merging in exactly one place: a flat list of ranges near the end of the
worksheet. No cell says "I am merged". So merging a range **adds a range to that list and changes
nothing else** — it does not create the covered cells, and it does not clear the values already in
them.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellRange, CellReference};
use mjx_xlsx::Workbook;

let cell = |text: &str| CellReference::parse(text).expect("a reference");
let mut workbook = Workbook::open(&mjx_fixtures::fixture("sheet_grid.xlsx"))?;

// A7:C7 is merged, and it is readable from *every* cell in it — not only from the top left.
for name in ["A7", "B7", "C7"] {
    assert_eq!(
        workbook.merged_range_containing(0, cell(name))?.map(|range| range.text().as_str().to_owned()),
        Some("A7:C7".to_owned()),
    );
}

// Merging a range that overlaps one already there is a typed error, not a silent overwrite:
// Excel would open such a workbook with a repair dialog.
assert!(workbook.merge_cells(0, CellRange::parse("B7:D8").expect("a range")).is_err());

// A range that overlaps nothing is written.
workbook.merge_cells(0, CellRange::parse("A2:A4").expect("a range"))?;
assert_eq!(workbook.merged_ranges(0)?.len(), 3);
# Ok(())
# }
```

ECMA-376 Part 1 §18.3.1.55 states the rule that gives a merge its meaning: *"The formatting and
content for the merged range is always stored in the top left cell."* So a covered cell renders
nothing of its own, and asking for its format has two different right answers depending on which
question you meant:

* [`Workbook::effective_cell_format`] resolves the cell's **own** record — what it would look like if
  the merge were removed;
* [`Workbook::effective_merged_cell_format`] resolves the cell that actually **renders** there, which
  for a covered cell is the merge's top left.

## A height without its flag is a height Excel recomputes

`row@ht` on its own says *a consumer worked this out*, and a consumer is free to work it out again.
`row@customHeight="1"` beside it says *a person set this*. The two state one fact between them, so
this library has **no call that takes a bare height**: the number arrives inside a
[`mjx_sml::RowHeight`], and the caller says which claim is being made.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{ColumnWidth, RowHeight};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::open(&mjx_fixtures::fixture("sheet_grid.xlsx"))?;

// A height a person set. `customHeight` is written beside it, because it has to be.
workbook.set_row_height(0, 5, Some(RowHeight::Custom(33.0)))?;

// The auto-fitted spelling is still expressible — Excel writes it — but you have to ask for it.
workbook.set_row_height(0, 7, Some(RowHeight::Fitted(15.75)))?;

let sheet = workbook.worksheet_markup(0)?.expect("a worksheet");
let store = sheet.sheet_data().expect("sheetData");
assert_eq!(store.row(5).expect("row 5").height(), Some(33.0));
assert!(store.row(5).expect("row 5").uses_custom_height());
assert!(!store.row(7).expect("row 7").uses_custom_height());
# Ok(())
# }
```

[`mjx_sml::ColumnWidth`] is the same shape for the same reason.

## A `col` is a run, so setting one width splits it

`<col min="2" max="6" width="12.5"/>` is **five columns**, written once. Setting the width of column
`D` inside that run cannot edit the run — it has to break it into three, and hand only the middle
piece the new width. Get that wrong and every column in the sheet changes width.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellSpan, ColumnWidth};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::open(&mjx_fixtures::fixture("sheet_grid.xlsx"))?;

// Columns are zero-based here, as every column index in `mjx-sml` is: 3 is `D`.
workbook.set_column_width(0, CellSpan::new(3, 3).expect("a span"), Some(ColumnWidth::Custom(40.0)))?;

let sheet = workbook.worksheet_markup(0)?.expect("a worksheet");
let width = |column: u16| {
    sheet
        .column_run_covering(column)
        .expect("the runs have bounds")
        .and_then(|run| run.width(sheet.interner()).expect("@width"))
};
assert_eq!(width(3), Some(40.0), "D took the new width");
for other in [1_u16, 2, 4, 5] {
    assert_eq!(width(other), Some(12.5), "and nothing else in the run moved");
}
# Ok(())
# }
```

The four cases are the three-way split, the left edge, the right edge, and an exact match that is
edited in place with no split at all. Columns no run covers get a fresh run each, grouped into
contiguous stretches. **Adjacent runs are never merged back together**, even when they end up
agreeing: the number of `col` elements is part of the file.

## Sheet protection is not security, and this library never says otherwise

Nothing here computes a hash, verifies one, or asks whether a password is right — there is no call in
this workspace that takes a password. `password`, `algorithmName`, `hashValue`, `saltValue` and
`spinCount` are read as the text the file wrote and written back as the same text, byte for byte,
through any number of unrelated edits.

Two separate reasons, and the second is the one that matters. A hash this library recomputed would be
a claim it cannot make: Excel derives it from a user-supplied password, and a library editing an
unrelated part has neither. And **protection is a user-interface convenience, not access control** —
every flag is advisory, and the sheet's bytes are readable by anyone holding the file whatever it
says.

One thing about the flags is not inferable from their names, so it is written here as well as on
[`mjx_sml::SheetProtection`]: **every one of them is a lock**, not a permission. §18.3.1.85 states
each in the same form — *"If 1 or true then formatting cells should not be allowed when the sheet is
protected"* — so `formatCells="1"` **forbids** formatting cells, and eleven of the fifteen locks
default to `true`. The accessors are named `locks_…` for exactly that reason.

## Nothing is repaired on read

A merge overlapping another, a merge laid over populated cells, an `outlineLevelRow` no row reaches,
two `col` runs claiming the same column: all of these are real in files Excel wrote or repaired, all
of them are preserved exactly as read, and none of them is corrected. A helpful correction the caller
did not ask for is the defect.

What you get instead is a description you can choose to ask for:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("sheet_grid.xlsx"))?;
assert_eq!(workbook.grid_anomalies(0)?, []);
# Ok(())
# }
```

The same split runs through the whole surface. Setting an outline level **raises**
`sheetFormatPr@outlineLevelRow` if it is too shallow, because writing a level past the declared
maximum would author the disagreement — but nothing ever *lowers* it, and nothing recomputes it,
until [`mjx_sml::WorksheetPart::recompute_outline_levels`] is called. That is the same contract
`recompute_dimension` has had since MJXOFF-102.

## What is still held rather than modelled

Thirteen of `CT_Worksheet`'s thirty-nine slots are modelled as of MJXOFF-117. The other twenty-six —
conditional formatting, data validation, autofilters, hyperlinks, print setup, drawings, tables —
are held as the markup the producer wrote, in the position it wrote it, and come back byte for byte.
Held is not dropped: a worksheet whose `conditionalFormatting` survives an edit to its column widths
is proof the frame works.
