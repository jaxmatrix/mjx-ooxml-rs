# Opening and saving a workbook

The whole of the current surface, in the order you meet it.

## Open

[`Workbook::open`] takes the container's bytes. It finds the workbook part through the package-root
`officeDocument` relationship, checks that part's root element really is `x:workbook`, and resolves
the part graph reachable from there.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let bytes = mjx_fixtures::fixture("sample.xlsx");
let workbook = Workbook::open(&bytes)?;
assert_eq!(workbook.workbook_part().as_str(), "/xl/workbook.xml");
# Ok(())
# }
```

The workbook part is identified by its **root element**, never by its content type. That is what lets
a macro-enabled workbook (`.xlsm`) open even though ECMA-376 declares no content type for one — see
[the part-graph module](crate::parts)'s own documentation for why this crate declines to invent
that string.

## The sheets

[`Workbook::sheets`] returns the workbook's tabs in the order `x:sheets` lists them — which is the
*only* place that order exists. The relationships in `xl/_rels/workbook.xml.rels` say which parts a
workbook has, never which tab comes first.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::{SheetKind, Workbook};

let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let sheets = workbook.sheets();

assert_eq!(sheets.len(), 1);
assert_eq!(sheets[0].name, "sample");
assert_eq!(sheets[0].sheet_id, Some(1));
assert!(sheets[0].is_visible());
assert_eq!(sheets[0].kind, Some(SheetKind::Worksheet));
assert_eq!(
    sheets[0].part.as_ref().map(|part| part.as_str()),
    Some("/xl/worksheets/sheet1.xml"),
);
# Ok(())
# }
```

[`Workbook::worksheet`] takes an index into that list and resolves the sheet's *own* part graph — its
drawings, its comments, the legacy VML that draws a comment's pop-up box, its tables and its saved
printer settings.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let sheet = workbook.worksheet(0)?.expect("the fixture's one sheet");

assert_eq!(sheet.part().as_str(), "/xl/worksheets/sheet1.xml");
assert_eq!(sheet.entry().name, "sample");
// LibreOffice wrote this fixture with nothing hanging off the sheet at all.
assert!(sheet.parts().comments.is_none());
assert!(sheet.parts().tables.is_empty());
# Ok(())
# }
```

## The workbook's own parts

[`Workbook::parts`] is the other half of the graph: what the *workbook* relates to, rather than what
a sheet does.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("sample.xlsx"))?;
let parts = workbook.parts();

assert_eq!(
    parts.styles.as_ref().map(|part| part.as_str()),
    Some("/xl/styles.xml"),
);
assert_eq!(
    parts.shared_strings.as_ref().map(|part| part.as_str()),
    Some("/xl/sharedStrings.xml"),
);
assert_eq!(
    parts.theme.as_ref().map(|part| part.as_str()),
    Some("/xl/theme/theme1.xml"),
);
assert!(parts.pivot_cache_definitions.is_empty());
# Ok(())
# }
```

## Validate, then save

[`Workbook::save`] validates first. [`Workbook::validate`] runs `mjx-opc`'s packaging checks and then
this crate's SpreadsheetML ones; both are read-only and neither changes the package.

```
# fn main() -> Result<(), mjx_xlsx::XlsxError> {
use mjx_xlsx::Workbook;

let original = mjx_fixtures::fixture("sample.xlsx");
let workbook = Workbook::open(&original)?;
workbook.validate()?;

let saved = workbook.save()?;
// A container, not a copy of the input: the ZIP encoding may differ. What does not differ is any
// part's decompressed bytes — see the next page.
assert_eq!(&saved[..2], b"PK");
# Ok(())
# }
```
