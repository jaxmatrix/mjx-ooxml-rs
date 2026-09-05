# Hyperlinks

A hyperlink in Excel is **two records that have to agree**, and they live in two different parts. The
entry — which cells are linked, what the tooltip says — is in the worksheet; the target of an
*external* link is in the worksheet's `.rels`. Either half without the other is a file Excel offers
to repair, so this crate writes and removes them together and never lets you write one alone.

## The two kinds you can author

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellRange, CellReference};
use mjx_xlsx::{HyperlinkKind, HyperlinkTarget, Workbook};

let mut workbook = Workbook::blank()?;

// External: a relationship is added, and the entry names it.
workbook.set_cell_hyperlink(
    0,
    CellRange::parse("A1")?,
    &HyperlinkTarget::Url("https://example.invalid/spec".to_owned()),
)?;

// Internal: a `@location` and **no relationship at all** — the destination is a cell in this
// same workbook, so there is no part to reach.
workbook.set_cell_hyperlink(
    0,
    CellRange::parse("B2:D4")?,
    &HyperlinkTarget::Location("Sheet1!A1".to_owned()),
)?;

let external = workbook
    .cell_hyperlink(0, CellReference::parse("A1")?)?
    .expect("A1 is linked");
assert_eq!(external.kind(), HyperlinkKind::External);
assert_eq!(external.target.as_deref(), Some("https://example.invalid/spec"));

let internal = workbook
    .cell_hyperlink(0, CellReference::parse("C3")?)?
    .expect("C3 is inside B2:D4");
assert_eq!(internal.kind(), HyperlinkKind::Internal);
assert_eq!(internal.relationship_id, None);
# Ok(())
# }
```

Note the second call: `@ref` is a **range**, not a cell. One entry can link `B2:D4`, and
`cell_hyperlink` answers for every cell inside it. Nothing here splits such an entry into one per
cell, because that would change the file and lose the entry's identity as one link.

## A third shape exists, and it is not a defect

`CT_Hyperlink` declares `@r:id` and `@location` **both optional**, so a file may write both — an
external target with a fragment, or a link that once pointed outside and was repointed inside. That
is a real shape Excel writes, and this crate reports it as its own kind rather than tidying it into
one of the other two:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellRange, CellReference, Hyperlink};
use mjx_xlsx::{HyperlinkKind, Workbook};

let mut workbook = Workbook::blank()?;
let part = workbook.sheets()[0].part.clone().expect("the tab reaches a part");
let relationship_id =
    workbook.add_hyperlink_relationship(&part, "https://example.invalid/spec#q1")?;

// Build an entry carrying both halves and put it on the sheet.
let mut markup = workbook.worksheet_markup(0)?.expect("a worksheet");
let prefix = markup.bind_relationship_prefix();
let element_prefix = markup.element_prefix().map(str::to_owned);
let interner = markup.interner_mut();
let mut entry = Hyperlink::new(interner, element_prefix.as_deref());
entry.set_range(interner, CellRange::parse("A1")?);
entry.set_relationship_id(interner, &prefix, &relationship_id);
entry.set_location(interner, Some("Sheet1!B2"));
markup.add_hyperlink(entry);
workbook.write_worksheet_markup(0, &markup)?;

let both = workbook
    .cell_hyperlink(0, CellReference::parse("A1")?)?
    .expect("A1 is linked");
assert_eq!(both.kind(), HyperlinkKind::ExternalWithLocation);
assert_eq!(both.relationship_id.as_deref(), Some(relationship_id.as_str()));
assert_eq!(both.location.as_deref(), Some("Sheet1!B2"));

// Neither half is dropped because the other is there, and the package is valid.
workbook.validate()?;
# Ok(())
# }
```

`HyperlinkKind::Unresolved` is the fourth: an entry that names a range and nothing to go to. Valid
markup that points nowhere, reported as exactly that.

## Removing one removes both

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellRange, CellReference};
use mjx_xlsx::{HyperlinkTarget, Workbook};

let mut workbook = Workbook::blank()?;
workbook.set_cell_hyperlink(
    0,
    CellRange::parse("A1")?,
    &HyperlinkTarget::Url("https://example.invalid/".to_owned()),
)?;

assert!(workbook.remove_cell_hyperlink(0, CellReference::parse("A1")?)?);
// Both halves went together, so the package is still valid.
workbook.validate()?;
assert!(workbook.cell_hyperlink(0, CellReference::parse("A1")?)?.is_none());
# Ok(())
# }
```

The relationship survives if **another entry in the same sheet still names it** — two cells sharing
one link is a shape Excel writes, and removing the first must not break the second. It goes with its
last user.

A relationship this library nonetheless leaves behind is a
[`SpreadsheetDefect::OrphanedHyperlinkRelationship`], reported by [`Workbook::validate`] and by
[`Workbook::save`]. The rule is deliberately narrow: it fires for the `hyperlink` relationship type
and no other, because a `comments` relationship is found by *type* and named by no markup at all, so
a general "unreferenced relationship" rule would fault every commented worksheet in existence.

It is also scoped to worksheets this library will write. A workbook opened and saved untouched is
never faulted for markup it arrived with, and [`Workbook::save_unchecked`] writes a container back
exactly as it came regardless.

## What this will never do to a target

**An external target is an untrusted URI.** It is carried exactly as the `.rels` wrote it, and
exactly as you hand it over:

- never percent-decoded or re-encoded;
- never resolved against a base, and never made absolute;
- never lower-cased or otherwise normalised;
- and **never fetched**. `mjx-ooxml-rs` performs no network or filesystem access on a workbook's
  behalf, and a hyperlink is the most obvious place a reader might assume otherwise.

The same applies to `webPublishItem@destinationFile`, which is a path on somebody else's disk.

## What is not written for you

Setting a hyperlink **touches no cell**. `@display` is not written from the cell's value and the
cell's value is not written from the target; making them agree is yours, because doing it silently
would overwrite data nobody asked to lose. Excel does not keep them in step either.

## The markup, if you need more than the report

[`Workbook::sheet_hyperlinks`] answers with owned [`SheetHyperlink`] reports. For anything they do
not carry — an entry's `@tooltip` in the middle of a larger edit, say — reach
[`mjx_sml::Hyperlinks`] through [`Workbook::worksheet_markup`] and write it back with
[`Workbook::write_worksheet_markup`]. That path does **not** manage relationships: it is the markup
tier, which has never heard of a package, and the two-halves rule is yours to keep there.
