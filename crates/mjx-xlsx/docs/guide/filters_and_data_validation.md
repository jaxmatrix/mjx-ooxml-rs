# Filters and data validation

**This library records an autofilter, a sort state and a validation rule. It never applies one.**
That is the same boundary the [Formulas and cached values](formulas_and_cached_values) and
[Conditional formatting](conditional_formatting) pages draw, and it has three faces here, each worth
naming plainly so nobody plans around a behaviour this crate does not have:

* **a filter hides no row.** Excel hides rows when a filter is applied; this library writes the
  filter and leaves every row's `hidden` flag exactly as it found it;
* **a sort state reorders nothing.** `sortState` is a record of a sort that already happened, not an
  instruction to perform one;
* **a validation rejects nothing.** No call here compares a cell's value against a rule, and a
  `list` rule's source is formula text that is never resolved into the values it names.

## Reading an autofilter

`autoFilter` carries the filtered range and one `filterColumn` per column the user narrowed. A
column's filter is an **`xsd:choice` of six kinds** — values, top-10, a custom comparison pair, a
dynamic period, a colour, an icon — plus `extLst`, the extension slot. So the model is a Rust enum,
and a column that carries a filter this crate does not model still round-trips.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::FilterKind;
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("validation_and_filters.xlsx"))?;

let kinds = workbook
    .auto_filter(0, |part, filter| {
        filter
            .expect("this sheet has an autoFilter")
            .columns()
            .map(|column| {
                (
                    // `@colId` is an offset into the autofilter's own `@ref`, not a sheet column.
                    column.column_offset(part.interner()).expect("a colId"),
                    column.filter().and_then(FilterKind::local),
                )
            })
            .collect::<Vec<_>>()
    })?
    .expect("the tab reaches a worksheet part");

assert_eq!(
    kinds,
    vec![
        (1, Some("customFilters")),
        (0, Some("filters")),
        (4, Some("top10")),
        (2, Some("dynamicFilter")),
        (5, Some("colorFilter")),
        (3, Some("iconFilter")),
        // A column whose only child is the choice's `extLst`: no modelled kind, and nothing lost.
        (6, None),
    ]
);
# Ok(())
# }
```

Two things in that list are decisions rather than accidents:

* **the columns come back in document order**, which is not `@colId` order. A producer may write them
  in any order the schema allows, and sorting them would change the file;
* **`@filterVal` on a `top10`, and `@val`/`@maxVal` on a `dynamicFilter`, are Excel's caches.** They
  are the bounds Excel derived when it last applied the filter. They are reported and never
  recomputed, exactly as a formula's cached value is.

## Adding a filter hides nothing

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{AutoFilterSpec, CellRange, FilterColumnSpec, FilterSpecKind};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::open(&mjx_fixtures::fixture("validation_and_filters.xlsx"))?;
let range = CellRange::parse("A1:G20").expect("a range");

// Nothing in the sheet is `Atlantis`, so a library that *applied* this filter would hide every
// row. This one records it.
workbook.set_auto_filter(
    0,
    &AutoFilterSpec::over(range)
        .with_column(FilterColumnSpec::new(0, FilterSpecKind::values(["Atlantis"]))),
)?;

let hidden = workbook
    .worksheet_markup(0)?
    .expect("a worksheet part")
    .rows()
    .filter(|row| row.is_hidden())
    .filter_map(|row| row.number())
    .collect::<Vec<_>>();

// Row 4 was hidden in the file and still is; nothing else was touched either way.
assert_eq!(hidden, vec![4]);
# Ok(())
# }
```

`Workbook::remove_auto_filter` is the same promise in reverse: removing a filter unhides nothing,
because a `hidden` row is the file's statement about that row and this library did not put it there.

## A list validation's source is text

`dataValidation type="list"` states its source in `formula1`, and that source is one of two things —
a **range reference** (`$G$2:$G$4`, or `Lookups!$A$1:$A$9`, or a defined name) or a **quoted literal
list** (`"Low,Medium,High"`, outer quotes included). Both are legal, and **neither is ever converted
into the other**:

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("validation_and_filters.xlsx"))?;

let sources = workbook
    .data_validations(0, |_part, rules| {
        rules
            .iter()
            .filter_map(|rule| rule.first_formula().map(|formula| formula.text().to_owned()))
            .collect::<Vec<_>>()
    })?
    .expect("the tab reaches a worksheet part");

assert_eq!(
    sources,
    vec![
        // The two spellings, side by side, exactly as the file wrote them.
        "$G$2:$G$4".to_owned(),
        "\"Ada,Grace,Alan\"".to_owned(),
        "0".to_owned(),
        "AND($E2>0,$E2<\"10000\")".to_owned(),
    ]
);
# Ok(())
# }
```

Resolving the first into the second would freeze a list Excel recomputes every time the drop-down
opens, and would need a cross-sheet read for `Lookups!`. So it does not happen — on read, on write,
or on authoring.

## Authoring a validation

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_sml::{CellRangeList, DataValidationSpec};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::blank()?;
let ranges = CellRangeList::parse("A2:A100").expect("a sqref");

workbook.add_data_validation(
    0,
    &DataValidationSpec::list(ranges, "\"North,South,East,West\"")
        .with_error("Not a region", "Pick one of the four"),
)?;

let rules = workbook
    .data_validations(0, |part, rules| {
        rules
            .iter()
            .map(|rule| {
                (
                    rule.kind(part.interner()).expect("a @type"),
                    rule.first_formula().expect("a formula1").text().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    })?
    .expect("the tab reaches a worksheet part");

assert_eq!(
    rules,
    vec![(
        mjx_ooxml_types::spreadsheetml::DataValidationType::List,
        "\"North,South,East,West\"".to_owned(),
    )]
);
# Ok(())
# }
```

One more cache is left alone here: `dataValidations@count`. It is the producer's number, and
appending a rule does not rewrite it — correcting a cache nobody asked about is the same class of
unrequested change as hiding a row.
