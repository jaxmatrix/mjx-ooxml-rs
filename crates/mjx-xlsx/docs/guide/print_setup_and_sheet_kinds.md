# Print setup, headers and footers, and the four sheet kinds

Every tab in a workbook carries the same **print block** — margins, print options, page setup, a
header and a footer — and not every tab is a grid of cells. Both halves of that sentence are what
this page is about.

Nothing here paginates. `fitToWidth="2"` is a number this library hands back; **where a page actually
breaks is rendering**, and no call in this workspace answers it.

## Reading a sheet's print setup

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml_types::spreadsheetml::{CellComments, PrintError, PrintOrientation};
use mjx_xlsx::{SheetMarkup, Workbook};

let workbook = Workbook::open(&mjx_fixtures::fixture("print_and_sheet_kinds.xlsx"))?;
let SheetMarkup::Worksheet(sheet) = workbook.sheet_markup(0)?.expect("tab 0 reaches a part") else {
    panic!("tab 0 is a worksheet");
};
let interner = sheet.interner();

let margins = sheet.page_margins().expect("x:pageMargins");
assert_eq!(margins.left_inches(interner)?, 0.7);
assert_eq!(margins.bottom_inches(interner)?, 1.25);

let setup = sheet.page_setup().expect("x:pageSetup");
assert_eq!(setup.orientation(interner)?, PrintOrientation::Landscape);
assert_eq!(setup.cell_comment_printing(interner)?, CellComments::AtEnd);
assert_eq!(setup.error_printing(interner)?, PrintError::NotAvailable);

// Both scaling modes are reported, because the file states both. Which one a consumer honours is
// decided by `sheetPr/pageSetUpPr/@fitToPage`, in a different slot — so nothing here clears one
// because the other is set.
assert_eq!(setup.scale_percentage(interner)?, 85);
assert_eq!(setup.pages_wide(interner)?, 2);
assert_eq!(setup.pages_tall(interner)?, 0); // "as many as it takes"
# Ok(())
# }
```

Margins are in **inches** — ECMA-376 Part 1 §18.3.1.62, which the schema itself does not say — and
the accessor names carry the unit for that reason. All six are `use="required"`, so each returns a
`Result` rather than an `Option`: an absent `@left` is a defect in the file, and answering "no left
margin" would be inventing a value.

## Headers and footers are opaque strings, and this library never re-writes one

A header is one string in Excel's formatting-code language: `&L` `&C` `&R` pick a section, `&P` `&N`
`&D` `&T` `&F` `&A` substitute a value, `&"Arial,Bold"` selects a font, `&G` draws the image in the
sheet's `drawingHF` part, and `&&` is a literal ampersand.

**Two strings can mean the same thing and be different strings**, so this library holds the file's
own bytes for one and never assembles a new one out of parts. There is no `set_section`, and there
will not be: writing one section back means re-emitting the other two.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{HeaderFooterSection, HeaderFooterSlot};
use mjx_xlsx::{SheetMarkup, Workbook};

let workbook = Workbook::open(&mjx_fixtures::fixture("print_and_sheet_kinds.xlsx"))?;
let SheetMarkup::Worksheet(sheet) = workbook.sheet_markup(0)?.expect("a part") else {
    panic!("tab 0 is a worksheet");
};
let header_footer = sheet.header_footer().expect("x:headerFooter");

// Six slots: odd, even and first, header and footer in each pair. With `@differentOddEven` unset,
// the odd pair is *every* page's.
assert!(header_footer.odd_and_even_pages_differ(sheet.interner())?);

let odd = header_footer
    .string(HeaderFooterSlot::OddHeader)
    .expect("an oddHeader");
assert_eq!(odd.text(), r#"&L&"Arial,Bold"Q1&C&G&R&P of &N"#);

// The reading accessors are slices of that one string. Nothing is parsed into a representation
// that could be written back.
assert_eq!(
    odd.section_runs(HeaderFooterSection::Left).collect::<Vec<_>>(),
    vec![r#"&"Arial,Bold"Q1"#],
);
assert!(odd.contains_drawing_reference()); // the `&G`

// `&&` is a literal ampersand, not a section code and not a drawing reference.
let footer = header_footer
    .string(HeaderFooterSlot::OddFooter)
    .expect("an oddFooter");
assert_eq!(
    footer.section_runs(HeaderFooterSection::Center).collect::<Vec<_>>(),
    vec!["Smith && Sons"],
);
assert!(!footer.contains_drawing_reference());
# Ok(())
# }
```

ECMA-376 Part 1 §18.3.1.46 permits — but does not require — an implementation to concatenate several
specifiers for the same section, so `section_runs` yields the runs the file wrote and never joins
them. Replacing a whole string with `HeaderFooterText::set_text` is the one mutation there is, and it
is the point at which the file's own spelling is given up.

## The printer-settings blob and the background picture

A `pageSetup` may carry an `r:id` to a **printer settings** part, and a `picture` an `r:id` to an
**image**. Both are opaque bytes this library carries and never interprets — a printer-settings part
is a Windows `DEVMODE` on which ECMA-376 Part 1 §15.2.13 places no requirement at all.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_xlsx::Workbook;

let workbook = Workbook::open(&mjx_fixtures::fixture("print_and_sheet_kinds.xlsx"))?;

assert_eq!(
    workbook.sheet_printer_settings(0)?.map(|p| p.as_str().to_owned()),
    Some("/xl/printerSettings/printerSettings1.bin".to_owned()),
);
assert_eq!(
    workbook.sheet_background_image(0)?.map(|p| p.as_str().to_owned()),
    Some("/xl/media/image1.png".to_owned()),
);
# Ok(())
# }
```

These read the `r:id` **off the sheet's own markup** and resolve it against that sheet part's
`.rels`. `Worksheet::parts()` answers a different question — *what does this sheet relate to* — and
the two can disagree: a `pageSetup` naming a relationship whose type is not `printerSettings` is a
[`SpreadsheetDefect::SheetReferenceHasTheWrongRelationshipType`], reported for markup this library
will write and preserved untouched in markup it merely opened.

## Custom views record a sheet's state; they never restore it

Excel's **Custom Views** save a named snapshot of a sheet: hidden rows, split panes, the selection,
the print setup, the filter. Each one writes an `x:customSheetView`, keyed by a `@guid` matching an
entry in the workbook's own `x:customWorkbookViews`.

**Reading one changes nothing.** `@hiddenRows` on a view is that view's memory of what was hidden
when it was saved; the sheet's actual hidden rows are the rows' own property and are untouched.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_xlsx::{SheetMarkup, Workbook};

let workbook = Workbook::open(&mjx_fixtures::fixture("print_and_sheet_kinds.xlsx"))?;
let SheetMarkup::Worksheet(sheet) = workbook.sheet_markup(0)?.expect("a part") else {
    panic!("tab 0 is a worksheet");
};
let views = sheet.custom_sheet_views().expect("x:customSheetViews");
let view = views.views().next().expect("one saved view");

assert!(view.has_hidden_rows(sheet.interner())?); // what the view remembers…
assert!(sheet.rows().all(|row| !row.is_hidden())); // …and what the sheet actually says

// A saved view carries its own pane, breaks, print block and filter — each the same type the sheet
// itself uses, not a second model of one.
assert!(view.pane().is_some());
assert_eq!(view.row_breaks().expect("rowBreaks").manual_count(sheet.interner()), 1);
assert_eq!(view.page_setup().expect("pageSetup").scale_percentage(sheet.interner())?, 60);
assert_eq!(view.auto_filter().expect("autoFilter").columns().count(), 1);
# Ok(())
# }
```

## Four sheet kinds, and the one that has no cells

`x:sheets` in `xl/workbook.xml` is one list over four part shapes:

| Kind | Complex type | Cells? |
|---|---|---|
| Worksheet | `CT_Worksheet` | yes |
| Chartsheet | `CT_Chartsheet` | **no `sheetData` at all** |
| Dialogsheet | `CT_Dialogsheet` | **no `sheetData` at all** |
| Macrosheet | `CT_Macrosheet` | held verbatim; not modelled |

[`Workbook::sheet_markup`] hands back whichever it is, as a [`SheetMarkup`]. **Reaching a cell means
matching out the `Worksheet` variant**: the other three carry types with no cell accessor at all, so
asking a chartsheet for its cells does not compile. That is the whole mechanism — there is no method
that answers `None`, and no empty collection to mistake for an empty sheet.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_xlsx::{SheetKind, SheetMarkup, Workbook};

let workbook = Workbook::open(&mjx_fixtures::fixture("print_and_sheet_kinds.xlsx"))?;
assert_eq!(workbook.sheets()[1].kind, Some(SheetKind::Dialogsheet));

match workbook.sheet_markup(1)?.expect("a part") {
    SheetMarkup::Worksheet(sheet) => {
        let _cells = sheet.cells().count(); // only reachable here
    }
    SheetMarkup::DialogSheet(dialog) => {
        assert!(dialog.page_setup().is_some());
        // `dialog.cells()` does not exist.
    }
    other => panic!("tab 1 is a dialogsheet; it reported {}", other.root_element()),
}
# Ok(())
# }
```

A **macrosheet** is the odd one: ECMA-376 declares no global element, no content type and no
relationship type for `CT_Macrosheet`, so [`SheetKind`] — which is defined by content type — stays at
the three kinds §12.3.23 names, and the entry's `kind` is `None`. `sheet_markup` dispatches on the
part's **root element** instead, so the markup is still there to read. A workbook holding one opens,
reports it, and round-trips.

## What is not here

A chartsheet's `drawing` relationship is reported; the chart *inside* the part it names is not
modelled by this crate. Nor are the sheet drawing part itself, OLE objects or form controls — all of
which are preserved byte for byte regardless, by the part-level copy-on-write
[Fidelity and the part graph](fidelity_and_the_part_graph) describes.
