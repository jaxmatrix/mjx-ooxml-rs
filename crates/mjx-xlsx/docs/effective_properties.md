# Effective properties — what a file states versus what Excel shows

An `.xlsx` states remarkably little *at* the cell. A cell that renders bold, right-aligned, on a pale
blue ground, with two decimal places and a thin border, very often carries exactly one attribute of
its own: `c@s="4"`. Everything the reader can see lives in `xl/styles.xml`, at the end of an index.

Two families of readers answer the two different questions this raises:

- The **declared** readers — [`Workbook::styles_markup`], [`Workbook::worksheet_markup`], and the
  models they hand back — answer *what this part says*. A cell that writes no `@s` comes back
  without one. They are the right readers for editing: they show what an edit would overwrite.
- The **effective** readers — [`Workbook::effective_cell_format`],
  [`Workbook::effective_merged_cell_format`] and the per-sheet
  [`SheetFormatResolver::effective_cell_format`] behind them — answer *what Excel shows*. They pick
  the style index actually in force, walk both `xf` tables, and hand back a self-contained
  [`EffectiveCellFormat`] that says, aspect by aspect, which record supplied it.

No effective read dirties a part. Resolution parses parts the workbook had not needed yet, but it
never marks one modified, so a workbook opened, fully resolved and saved is byte-identical to the one
that went in.

## Excel resolves unlike the other two formats, and the difference is the whole page

PowerPoint inherits down a placeholder chain; Word inherits down a style ladder. **Excel inherits
nothing.** A cell carries an index, that index names a record, and that record carries four more
indices, a fifth into a *second* table of the same records, and six flags deciding which of the two
layers each aspect comes from. Everything is dereferenced; nothing is walked up.

That is why this page's middle section is a table of indices where
[`mjx_pptx::effective_properties`](https://docs.rs/mjx-pptx/latest/mjx_pptx/effective_properties/)
and
[`mjx_docx::effective_properties`](https://docs.rs/mjx-docx/latest/mjx_docx/effective_properties/)
each have a ladder — and why an [`EffectiveCellFormat`] reports a
[`FormatLayer`] and a [`StyleIndexSource`] where the other two report only a value. In Excel, *where
the answer came from* is a question a caller has to be able to ask, because both layers are in the
file at once and §18.8.10 requires reading both.

## The APIs

| Reader | Answers |
|---|---|
| [`Workbook::effective_cell_format`] | the format one cell renders with, from a tab index and a [`CellReference`] |
| [`Workbook::effective_merged_cell_format`] | the same, resolved at the **anchor** of the merged region the reference falls in |
| [`Workbook::sheet_formatting`] | both parts, read once, for a caller resolving more than a handful of cells |
| [`SheetFormatResolver::effective_cell_format`] | one cell, against an already-decoded pair of tables — the call that is free of parsing |
| [`CellFormatResolver::font`] / `fill` / `border` / `alignment` / `protection` / `number_format` | the record an [`EffectiveCellFormat`]'s index names |
| [`CellFormatResolver::format_code`] | the number-format code in force, built-in table included |
| [`CellFormatResolver::named_style`] | the `cellStyles` entry (`Normal`, `Explanatory`) the `xfId` layer belongs to |
| [`CellFormatResolver::differential_format`] | one `dxf` by index — the conditional-formatting layer, handed over *beside* the answer above and never merged into it |

Every one of these returns a plain value; nothing is `Option` because the resolution failed to find
something. A `None` from `font` means the winning record named no font, which is a real answer about
a real file.

## The indirection, exactly as implemented

```text
  c@s ─────────────► cellXfs[s] ─── @numFmtId ──► numFmts / §18.8.30's implied table
   │  (or row@s      (the direct    ─── @fontId ────► fonts[…]
   │   with          layer)         ─── @fillId ────► fills[…]
   │   customFormat,                ─── @borderId ──► borders[…]
   │   or col@style,                ─── <alignment>, <protection>
   │   or 0)                        │
   │                                └── @xfId ──► cellStyleXfs[xfId]  (the layer beneath)
   │                                                the same five, for every aspect whose
   └─ StyleIndexSource says which   applyX on the direct record is `applyX="0"`
```

**Step 1 — which `xf` to start from** ([`StyleIndexSource`]), in this order:

| source | condition | index |
|---|---|---|
| `Cell` | the cell writes `@s` | that `@s` |
| `Row` | the row writes `customFormat="1"` | the row's `@s` |
| `Column` | a `col` run covers the column | that run's `@style` |
| `Default` | none of the above | `0` |

The row's gate is normative: §18.3.1.73 defines `row@s` as *"Index to style record for the row (only
applied if `customFormat` attribute is '1')"*. The column's is `col@style`, *"Default style for the
affected column(s)"*.

**Step 2 — which layer supplies each aspect**, independently for all six of [`FormatAspect::ALL`](mjx_sml::FormatAspect::ALL):

1. If the direct record's `applyX` [participates](mjx_sml::ApplyFlag::participates) — it is `Applied` **or**
   `Unstated` — the aspect comes from the direct record ([`FormatLayer::Direct`]).
2. Otherwise, if the direct record names a `cellStyleXfs` record through `@xfId` and **that**
   record's own `applyX` participates, the aspect comes from it ([`FormatLayer::CellStyle`]).
3. Otherwise nothing supplies it ([`FormatLayer::Neither`]).

**Step 3 — dereference.** The winning record's `@numFmtId`, `@fontId`, `@fillId` or `@borderId` is
reported as [`ResolvedAspect::resource_index`], and `<alignment>`/`<protection>` are reached through
the resolver.

## Where the specification stops and this reading begins

Two things in step 2 are **not** normative sentences, and are marked as such rather than presented as
if they were:

- **`applyX` absent behaves as applied.** §18.8.45 defines each flag in one sentence and says nothing
  at all about absence. The reading comes from §18.8.9's worked example — *"the 0th record does not
  express any 'apply' attributes, while the other records do"* — where the record expressing none is
  `Normal` and is applied.
- **The `cellStyleXfs` record's own `applyX` is honoured too.** §18.8.9 says master formatting
  records *"also specify whether to apply or ignore particular aspects of formatting"* and its
  example is entirely about `cellStyleXfs` records that suppress — so honouring them is the faithful
  reading. But no sentence says what happens when **both** layers suppress, and
  [`FormatLayer::Neither`] is this workspace's answer rather than the specification's.

## The `dxf` layer sits beside this, never inside it

A conditionally formatted cell has a `dxf` applied **on top of** everything above (§18.8.15: *"to be
applied on top of or in addition to any formatting already present"*). That seam is filled *beside*
this resolver rather than inside it: [`Workbook::effective_cell_format`] answers exactly what
`styles.xml` says a cell's format is, and [`CellFormatResolver::differential_format`] hands out a
`dxf` by index for a caller that wants one. The two are put next to each other, and never merged,
because merging them would assert that a rule *fired* — and whether a rule's condition holds needs a
calculation engine this workspace does not have.

## Where resolution stops

| Reader / concern | Stops with |
|---|---|
| font, fill, border, number format, alignment, protection | a [`ResolvedAspect`] whose `format_index` is `None` — both layers declined, which is [`FormatLayer::Neither`] and a real answer |
| a theme colour (`<color theme="N"/>`) | **the slot number, never an `RRGGBB`.** Resolving it needs `xl/theme/theme1.xml`, which `mjx-sml` says is `mjx-xlsx`'s to fetch and `mjx-xlsx` does not fetch. This is the one place the three formats' effective readers genuinely differ in *power* rather than in vocabulary — see `mjx_ooxml::shared_markup_reachability`'s `dml-theme` note |
| an indexed colour | resolved, through §18.8.27's legacy palette, with the tint applied |
| applying a format code to a value (`0.00` + `3.14159` → `"3.14"`) | **not done, ever.** [`CellFormatResolver::format_code`] reports the code in force and stops; formatting a number is a rendering feature and a programme non-goal, not a gap |
| whether a conditional-formatting rule fires | not evaluated — no calculation engine; see the `dxf` section above |
| a cell in a `chartsheet` or `dialogsheet` | `Ok(None)` — those parts carry no `sheetData`, so there is no cell to resolve |
| a workbook with no `xl/styles.xml` at all | `Ok(None)` — there is nothing to resolve *against*, which is a question rather than an error |

## Cost

[`Workbook::effective_cell_format`] **reads and decodes both parts on every call.** That is the right
shape for one cell and the wrong shape for a thousand: for more than a handful, take one
[`SheetFormatting`] from [`Workbook::sheet_formatting`] and one [`SheetFormatResolver`] from it, and
resolve against that. Both `xf` tables and every `col` run are decoded once there, and per-cell
resolution is then index arithmetic with no parsing and no allocation.

The asymmetry is deliberate and is the same one
`crates/mjx-xlsx/docs/guide/large_workbooks.md` records for the rest of this crate: a convenience
call that re-reads is honest about it, and the hoisted form is one line away.

## Examples

Declared versus effective, on a cell that states nothing of its own:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellReference, CellValue};
use mjx_sml::{FormatLayer, StyleIndexSource};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::blank()?;
let a1 = CellReference::parse("A1")?;
workbook.set_cell_value(0, a1, CellValue::Number(1.0))?;

// What the sheet says about A1's format: nothing — it writes no `@s`.
let format = workbook
    .effective_cell_format(0, a1)?
    .expect("a blank workbook relates a styles part");
assert_eq!(format.style_index(), 0);
assert_eq!(format.style_index_source(), StyleIndexSource::Default);

// And what Excel shows: whichever record `cellXfs[0]` is, aspect by aspect.
assert_ne!(format.font().layer, FormatLayer::Neither);
# Ok(())
# }
```

One `xf`, pointed at from a cell — and the two layers reported separately:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::write::{CellFormatSpec, CellFormatTarget};
use mjx_sml::{FormatLayer, StyleIndexSource};
use mjx_sml::{CellReference, CellValue, FontProperties};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::blank()?;
let b2 = CellReference::parse("B2")?;
workbook.set_cell_value(0, b2, CellValue::Number(42.0))?;

let font = workbook.append_font(&FontProperties {
    bold: Some(true),
    ..FontProperties::default()
})?;
let xf = workbook.append_cell_format(
    CellFormatTarget::CellFormats,
    &CellFormatSpec {
        font_index: Some(font),
        applies_font: Some(true),
        ..CellFormatSpec::skeleton_cell_format()
    },
)?;
workbook.set_cell_style(0, b2, Some(xf))?;

let format = workbook
    .effective_cell_format(0, b2)?
    .expect("a styles part");
assert_eq!(format.style_index(), xf);
assert_eq!(format.style_index_source(), StyleIndexSource::Cell);
// The direct record states the font, so the direct layer supplies it.
assert_eq!(format.font().layer, FormatLayer::Direct);
assert_eq!(format.font().resource_index, Some(font));
# Ok(())
# }
```

Resolving many cells without re-reading the parts:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

let mut workbook = Workbook::blank()?;
for row in 0..16u32 {
    workbook.set_cell_value(0, CellReference::relative(0, row)?, CellValue::Number(row.into()))?;
}

// Both parts read once, both `xf` tables and every `col` run decoded once.
let formatting = workbook.sheet_formatting(0)?.expect("a styles part");
let resolver = formatting.resolver()?;
for row in 0..16u32 {
    let format = resolver.effective_cell_format(CellReference::relative(0, row)?)?;
    assert_eq!(format.style_index(), 0);
}
# Ok(())
# }
```

[`Workbook::styles_markup`]: crate::Workbook::styles_markup
[`Workbook::worksheet_markup`]: crate::Workbook::worksheet_markup
[`Workbook::effective_cell_format`]: crate::Workbook::effective_cell_format
[`Workbook::effective_merged_cell_format`]: crate::Workbook::effective_merged_cell_format
[`Workbook::sheet_formatting`]: crate::Workbook::sheet_formatting
[`SheetFormatting`]: crate::SheetFormatting
[`SheetFormatResolver`]: crate::SheetFormatResolver
[`SheetFormatResolver::effective_cell_format`]: crate::SheetFormatResolver::effective_cell_format
[`CellReference`]: mjx_sml::CellReference
[`EffectiveCellFormat`]: mjx_sml::EffectiveCellFormat
[`FormatLayer`]: mjx_sml::FormatLayer
[`FormatLayer::Direct`]: mjx_sml::FormatLayer::Direct
[`FormatLayer::CellStyle`]: mjx_sml::FormatLayer::CellStyle
[`FormatLayer::Neither`]: mjx_sml::FormatLayer::Neither
[`StyleIndexSource`]: mjx_sml::StyleIndexSource
[`ResolvedAspect`]: mjx_sml::ResolvedAspect
[`ResolvedAspect::resource_index`]: mjx_sml::ResolvedAspect::resource_index
[`CellFormatResolver::font`]: mjx_sml::CellFormatResolver::font
[`CellFormatResolver::format_code`]: mjx_sml::CellFormatResolver::format_code
[`CellFormatResolver::named_style`]: mjx_sml::CellFormatResolver::named_style
[`CellFormatResolver::differential_format`]: mjx_sml::CellFormatResolver::differential_format
