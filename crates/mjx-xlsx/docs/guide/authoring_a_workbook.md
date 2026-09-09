# Authoring a workbook from nothing

Everything on the other three pages starts from a `.xlsx` somebody already has. This page is the
other direction: a workbook this library writes from code, and the surface for filling it in.

## `Workbook::blank`

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let workbook = mjx_xlsx::Workbook::blank()?;
assert_eq!(workbook.sheets().len(), 1);
assert_eq!(workbook.sheets()[0].name, "Sheet1");

let bytes = workbook.save()?;
assert_eq!(&bytes[..2], b"PK");
# Ok(())
# }
```

[`Workbook::blank_with_properties`] is the same constructor with `docProps/core.xml` and
`docProps/app.xml` filled in.

Two things about it are worth knowing before you plan around it.

**There is no schema-valid empty workbook.** `CT_Workbook` has nineteen slots and exactly one of them
is not `minOccurs="0"`:

```xml
<xsd:element name="sheets" type="CT_Sheets" minOccurs="1" maxOccurs="1"/>
<!-- and CT_Sheets: <xsd:element name="sheet" … minOccurs="1" maxOccurs="unbounded"/> -->
```

`sheets` is required and it requires at least one `sheet`; `CT_Worksheet` then requires a
`sheetData`. So `blank()` is not a shell — it authors `xl/workbook.xml`, a worksheet part with its
own content type and relationship, `xl/styles.xml` (or every `@fontId`, `@fillId`, `@borderId` and
`c@s` in the file dangles), `xl/sharedStrings.xml` and `xl/theme/theme1.xml`.

The theme is there for the same reason `styles.xml` is, and it took MJXOFF-200 to see it. Nothing in
ECMA-376 or OPC requires a theme in a SpreadsheetML package — but the package is not finished when it
is valid, it is finished when every reference its own content makes resolves, and this one makes two
before a caller does anything: font 0 says `<color theme="1"/>` and `<scheme val="minor"/>`, which are
what make the default font *follow* the document's theme instead of pinning `Calibri` over it. Add a
chart and there is a third, because a series states no `c:spPr` and takes its fill from
`accent1…accent6`.

**A workbook you opened is not touched.** A theme is authored only into a package that carries none;
one that arrives with its own keeps it, byte for byte, because supplying a default in place of the
user's own would override the branding of whoever opens the file.

**It is deterministic.** Two calls produce byte-identical containers. Nothing here reads a clock or a
random number, which is what lets a round-trip assertion downstream be an equality rather than a
tolerance.

## The markup is `mjx-sml`'s

Every byte of it comes from [`mjx_sml::write::WorkbookPackage`], one tier below this crate. That is
not an implementation detail to route around: a PowerPoint chart embeds a whole workbook package at
`/ppt/embeddings/*.xlsx` and cannot depend on a format crate, so the writer lives where both callers
can reach it and there is exactly one of it. What this crate adds is the seam — the package the
writer produced goes straight into `Workbook::from_package`, so a workbook built from nothing is
resolved by the same code that resolves one read off disk.

A caller who wants the writer directly — to build a workbook without ever holding a [`Workbook`] —
can use it directly; it needs nothing from this crate.

## Filling one in

Six methods, all concrete-typed:

| Call | What it writes |
|---|---|
| [`Workbook::add_sheet`] | a new worksheet part, its content type, its relationship, and a `sheet` entry |
| [`Workbook::rename_sheet`] | `sheet@name`, and nothing else — a tab's name is not in its own markup |
| [`Workbook::set_cell_value`] | one `<c>` |
| [`Workbook::set_cell_style`] | `c@s` on one cell, creating a blank cell if there is none |
| [`Workbook::intern_shared_string`] | one `<si>` in `xl/sharedStrings.xml`, in first-use order |
| [`Workbook::append_font`] and the three beside it | one entry in one `xl/styles.xml` table |

```
use mjx_sml::write::{CellFormatSpec, CellFormatTarget, PatternFillSpec};
use mjx_sml::{CellReference, CellValue, FontProperties};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut workbook = mjx_xlsx::Workbook::blank()?;

// A shared string, and the cell that points at it.
let north = workbook.intern_shared_string("North")?;
let a1 = CellReference::parse("A1")?;
workbook.set_cell_value(0, a1, CellValue::SharedString(north))?;

// A format: a bold font, a yellow fill, and the `xf` that names both.
let bold = workbook.append_font(&FontProperties {
    font_name: Some("Calibri".to_owned()),
    size_in_points: Some(11.0),
    bold: Some(true),
    ..FontProperties::default()
})?;
let yellow = workbook.append_pattern_fill(&PatternFillSpec::solid("FFFF00"))?;
let highlight = workbook.append_cell_format(
    CellFormatTarget::CellFormats,
    &CellFormatSpec {
        font_index: Some(bold),
        fill_index: Some(yellow),
        applies_font: Some(true),
        applies_fill: Some(true),
        ..CellFormatSpec::skeleton_cell_format()
    },
)?;
workbook.set_cell_style(0, a1, Some(highlight))?;

let reopened = mjx_xlsx::Workbook::open(&workbook.save()?)?;
assert_eq!(reopened.cell_text(0, a1)?, Some("North".to_owned()));
assert_eq!(
    reopened.effective_cell_format(0, a1)?.map(|format| format.font().resource_index),
    Some(Some(bold)),
);
# Ok(())
# }
```

## Which colour constructor to reach for

Every colour in the authoring vocabulary is written one of two ways, and the choice is not a matter
of taste.

| Reach for | When |
|---|---|
| `PatternFillSpec::solid_from_theme(slot, tint)`, `Color::from_theme_slot(slot, tint)`, `ColorScaleSpec::two_color_from_theme`, `DataBarSpec::spanning_the_range_from_theme`, `DifferentialFormatSpec::highlight_from_theme` | **by default** — a heading band, a banded row, a data bar, a highlight. The colour is *decorative*: it is there to look like the rest of the document, and the document decides what that is |
| `PatternFillSpec::solid(hex)`, `Color::from_opaque_rgb(hex)`, `ColorScaleSpec::two_color`, `DataBarSpec::spanning_the_range`, `DifferentialFormatSpec::highlight` | when the colour **itself is the datum** — a traffic light where red means failing, a brand's exact red in a logo cell, a colour a caller was given and must reproduce |

The two write different markup for the same call. A theme slot writes
`<fgColor theme="4"/>`, which is a *reference*: open the workbook in a template branded green and it
comes out green. A literal writes `<fgColor rgb="FF1F3864"/>`, which is a *value*: it comes out navy
in every document forever, including the one whose owner has rebranded everything around it.

That is `mjx-ooxml`'s standing rule — where OOXML lets a value inherit, let it inherit — applied to
an API rather than to a file, and until MJXOFF-235 only the second column existed as a one-liner. A
`tint` of `-1.0 ..= 1.0` darkens (negative) or lightens (positive) the slot, so one accent covers a
whole family: `Some(0.8)` is Excel's own *"Accent 1, Lighter 80%"*.

The slot is DrawingML's [`ColorSchemeSlot`](mjx_dml::ColorSchemeSlot) rather than SpreadsheetML's
`@theme` **position**, because `4` means `accent1` only to a reader with §20.1.6.2 open.
`Color::from_theme(index, tint)` still takes the position, and is what a *reader* of a file needs,
since a file may state a position the twelve-slot table does not name.

**One thing a slot cannot express is contrast.** Text that has to stay legible against a fill is a
constraint between two colours, and a slot states one — so `crates/mjx-ooxml/examples/build_a_workbook.rs`
follows the theme for its heading *fill* and keeps a literal for the text on top of it. Following the
theme is the default; it does not outrank being readable.

## Indices are identity

Nothing in a workbook names a font, a fill, a border or an `xf`. Each is addressed by its **position**
in its table, so appending is the only mutation any of them offers: reordering, deduplicating or
garbage-collecting a table would silently repaint every cell that referred to anything after the
entry that moved. `append_*` returns the index it just created, and every earlier entry stays exactly
where it was.

The same is true of `xl/sharedStrings.xml`, which is why
[`Workbook::intern_shared_string`] appends in **first-use** order rather than sorted order, and why
it reuses only a plain `<si><t>…</t></si>` — an entry carrying rich-text runs or phonetic markup
displays the same characters and is not the same value.

## What it does not do

* **No formulas.** A formula's *text* round-trips because nothing rewrites a cell it was not asked
  to, and [`mjx_sml::CellFormula`] reads one — but there is **no call anywhere that writes one**.
  MJXOFF-115 (D11) modelled formulas for reading and preservation and did not add an authoring path,
  and no later child owns adding one. See
  [Deliberate limitations](deliberate_limitations#gaps-rather-than-decisions).
* **No theme part.** See above.
* **No `calcChain.xml`.** There is no calculation engine here, and a stale chain is worse than none —
  Excel rebuilds it.
* **No sheet removal.** Removing a tab means removing a part, its relationship, its entry and every
  defined name scoped to it; that is a decision, not a convenience method.
