# Worksheet tables

A worksheet table — Excel's *Format as Table*, the thing formulas call `Sales[Region]` — is the one
sheet-level feature in this crate that **lives outside the worksheet part**. It is
`xl/tables/tableN.xml`, a part of its own, and the sheet reaches it through a relationship.

That makes a table four things that have to agree:

| thing | where it lives |
|---|---|
| the table | `xl/tables/tableN.xml`, an `x:table` |
| its content type | an `Override` in `[Content_Types].xml` |
| the edge to it | a `table` relationship in `xl/worksheets/_rels/sheetN.xml.rels` |
| the sheet's claim on it | `x:tableParts/tablePart@r:id` in the worksheet |

[`Workbook::add_table`] writes all four in one call, and [`Workbook::validate`] refuses to save a
workbook in which they have stopped agreeing.

## Reading a sheet's tables

[`Workbook::sheet_tables`] answers with owned [`SheetTable`] values — no closure, no borrow, no
interner:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("../../tests/fixtures/worksheet_tables.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;

let tables = workbook.sheet_tables(0)?;
let names: Vec<&str> = tables.iter().map(|table| table.display_name.as_str()).collect();
assert_eq!(names, vec!["Regions", "Sales"]);

let sales = &tables[1];
assert_eq!(sales.range.to_string(), "A1:D6");
assert_eq!(sales.header_row_count, 1);
assert_eq!(sales.totals_row_count, 1);
// `@ref` includes the header and totals rows, so four of the six are data rows.
assert_eq!(sales.data_row_count(), Some(4));

let headings: Vec<&str> = sales.columns.iter().map(|c| c.name.as_str()).collect();
assert_eq!(headings, vec!["Region", "Q1", "Q2", "Total"]);
# Ok(())
# }
```

The list is the order the **sheet** states in `x:tableParts`, which is not necessarily the
relationship order. `Worksheet::parts().tables` answers the latter, and is the right call for *which
table parts does this sheet relate to at all*.

Everything the report does not carry is reached through [`Workbook::table_markup`], which hands you
the [`mjx_sml::WorksheetTable`] and the interner it was parsed with.

## A built-in style name is not a missing style

A table's style is a **name**, and the name is usually one Excel supplies rather than one the file
defines. `TableStyleMedium2` is in no `.xlsx` anywhere: ECMA-376 publishes all 144 presets as a
separate artifact, and §18.5.1.5 tells a consumer to fall back to its default only when the name
matches nothing at all.

So there are three answers, not two, and [`Workbook::table_style_origin`] gives all three:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::TableStyleOrigin;

let bytes = std::fs::read("../../tests/fixtures/worksheet_tables.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;

// A preset. Not in the file, and **not missing**.
assert_eq!(workbook.table_style_origin("TableStyleMedium2")?, TableStyleOrigin::BuiltIn);
// The workbook's own, defined in `xl/styles.xml`.
assert_eq!(workbook.table_style_origin("AcmeBlue")?, TableStyleOrigin::LocallyDefined);
// Neither — the one case a consumer really does fall back on.
assert_eq!(workbook.table_style_origin("AcmeGreen")?, TableStyleOrigin::Undefined);
# Ok(())
# }
```

Reporting *"style not found"* for `TableStyleMedium2` would be a confident wrong answer where the
honest one is a distinction, so this library does not offer one.

## Creating a table

[`Workbook::add_table`] takes a [`mjx_sml::WorksheetTableSpec`] — plain data, no interner — and
answers the [`SheetTable`] it created:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellRange, CellReference, CellValue, WorksheetTableSpec, TableStyleReferenceSpec};

let mut workbook = mjx_xlsx::Workbook::blank()?;
for (address, value) in [
    ("A1", CellValue::InlineString("Region")),
    ("B1", CellValue::InlineString("Units")),
    ("A2", CellValue::InlineString("North")),
    ("B2", CellValue::Number(1200.0)),
] {
    workbook.set_cell_value(0, CellReference::parse(address)?, value)?;
}

let mut spec = WorksheetTableSpec::new("Sales", CellRange::parse("A1:B2")?, &["Region", "Units"]);
spec.style = Some(TableStyleReferenceSpec::named("TableStyleMedium2"));

let created = workbook.add_table(0, &spec)?;
assert_eq!(created.id, 1);
assert_eq!(created.part.as_str(), "/xl/tables/table1.xml");

// The four things agree, so the workbook saves.
let saved = workbook.save()?;
assert_eq!(&saved[..2], b"PK");
# Ok(())
# }
```

Two things it deliberately does not do.

**It writes no cell.** The headings in the spec go into the table part, not into row 1 of the sheet.
Writing them would mean overwriting values nobody asked to lose; making the two agree is yours.

**It never reuses or renumbers an `@id`.** A table's `@id` is unique across the *workbook*
(§18.5.1.2) and other records name a table by it, so `add_table` allocates one past the highest id
any table part in the package already writes — never a gap left by a deleted table, and never an id
an existing table is using. `spec.id` is ignored for exactly that reason: only something holding the
package knows which ids are free.

## Resizing: all three attributes, or none

A table's extent is one statement in three attributes — `@ref`, `@headerRowCount` and
`@totalsRowCount` — because §18.5.1.2 says the reference *"shall include the totals row if it is
shown"*. Change the range alone and the totals row falls outside the table.

So there is no `set_range`. [`mjx_sml::WorksheetTable::resize`] takes all three and refuses a
combination that does not fit:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellRange, SmlError};

let bytes = std::fs::read("../../tests/fixtures/worksheet_tables.xlsx")?;
let mut workbook = mjx_xlsx::Workbook::open(&bytes)?;
let part = workbook.sheet_tables(0)?[1].part.clone();
let too_short = CellRange::parse("A1:D3")?;
let taller = CellRange::parse("A1:D12")?;

let refused = workbook.edit_table_markup(&part, |table, interner| {
    // Three rows cannot hold two header rows, two totals rows and any data.
    Ok(table.resize(interner, too_short, 2, 2).is_err())
})?;
assert!(refused);

let resized = workbook.edit_table_markup(&part, |table, interner| {
    table.resize(interner, taller, 1, 2)?;
    Ok(table.data_row_count(interner)?)
})?;
assert_eq!(resized, Some(9));
# Ok(())
# }
```

**An unrelated cell edit never moves a table's boundary.** Setting a value inside a table rewrites
one row of the worksheet part; the table part is not even opened, and its `@ref` comes back byte for
byte.

## What is carried and never interpreted

* **A calculated-column formula is text.** `Sales[[#This Row],[Q1]]+Sales[[#This Row],[Q2]]` is
  never expanded into per-cell `<f>` elements, never evaluated, and the structured reference in it is
  never parsed. That is [`mjx_sml::formula`]'s contract at one more door.
* **A totals row is never computed.** A column's `@totalsRowFunction` says *what Excel would show*;
  nothing here sums anything.
* **A table's own `autoFilter` and `sortState` are records.** They are the same types the sheet's own
  are — see *[Filters and data validation](../filters_and_data_validation)* — and they hide no row
  and reorder none here either. Note that `sortState` appears in **three** places and they are three
  different elements: the worksheet's, an autofilter's, and a table's.
* **Caches are left alone.** `tableColumns@count` disagreeing with the columns present is a producer's
  business; this library reports what the file says and rewrites the attribute only when it edits the
  collection *and* the file already declared one.
* **`xmlColumnPr` is preserved, not resolved.** The XML map part it names is not modelled anywhere in
  this workspace, and is carried verbatim like every other part nothing here claims.

## What `validate` will refuse

Beside the packaging invariants, [`Workbook::validate`] reports:

* a `tablePart@r:id` leading to a part that is not a table — [`SpreadsheetDefect::TablePartTargetIsNotATable`];
* two tables sharing an `@id` — [`SpreadsheetDefect::DuplicateTableId`];
* two tables sharing a `@displayName` — [`SpreadsheetDefect::DuplicateTableDisplayName`].

The last two fault only a table **this library wrote**: a workbook that arrived with a collision is
not ours to fault, and [`Workbook::save_unchecked`] writes it back exactly as it came.

A `tablePart@r:id` naming no relationship at all is not in that list on purpose. It is a dangling
relationship reference like any other, and the packaging layer already reports it over exactly the
same set of parts; restating it here would be one rule with two implementations.

**One known gap, written down rather than hidden.** §18.5.1.2 requires a `@displayName` to be unique
*"amongst all other displayNames and definedNames in the workbook"*. The check here compares table
names against each other and **not** against the workbook's defined names.
