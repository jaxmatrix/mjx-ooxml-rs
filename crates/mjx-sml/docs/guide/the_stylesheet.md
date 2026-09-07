# The stylesheet

`xl/styles.xml` is what makes SpreadsheetML structurally unlike PowerPoint and Word: **a cell carries
a style *index*, not a style**, and the index points into tables that are themselves indexed by other
tables. This page is that indirection, the four spellings a colour can take, and the rules on
authoring into it.

`crates/mjx-xlsx/docs/effective_properties.md` walks the resolution end to end for a caller who wants
to know what one cell actually looks like. This page is the markup underneath it.

## The eleven slots

[`mjx_sml::StylesheetPart`](crate::StylesheetPart) is a [slot frame](the_slot_frame) over
`CT_Stylesheet`: **eleven slots, ten modelled, one held** — `extLst`, the unknown bucket. The split
is derived by `every_slot_of_the_generated_sequence_is_accounted_for` in
`crates/mjx-sml/src/styles/stylesheet.rs`, which reads a part holding one of every slot and asks the
reader which of them it typed.

| rank | element | modelled as | what it is |
|---|---|---|---|
| 0 | `numFmts` | [`mjx_sml::NumberFormatTable`](crate::NumberFormatTable) | custom format codes, by **id** rather than by position |
| 1 | `fonts` | [`mjx_sml::FontTable`](crate::FontTable) | a resource table |
| 2 | `fills` | [`mjx_sml::FillTable`](crate::FillTable) | a resource table |
| 3 | `borders` | [`mjx_sml::BorderTable`](crate::BorderTable) | a resource table |
| 4 | `cellStyleXfs` | [`mjx_sml::CellFormatTable`](crate::CellFormatTable) | the records the **named styles** are made of |
| 5 | `cellXfs` | [`mjx_sml::CellFormatTable`](crate::CellFormatTable) | the records a cell's `@s` indexes |
| 6 | `cellStyles` | [`mjx_sml::NamedCellStyles`](crate::NamedCellStyles) | "Normal", "Comma", "Heading 1" — each naming a `cellStyleXfs` record |
| 7 | `dxfs` | [`mjx_sml::DifferentialFormats`](crate::DifferentialFormats) | *deltas*, used by conditional formatting and table styles |
| 8 | `tableStyles` | [`mjx_sml::TableStyles`](crate::TableStyles) | the styles the workbook defines, and the two it prefers |
| 9 | `colors` | [`mjx_sml::ColorTable`](crate::ColorTable) | the legacy indexed palette, and the most-recently-used list |
| 10 | `extLst` | held raw | markup no schema in this workspace types |

The two `xf` tables are one Rust type in two slots, told apart by
[`mjx_sml::CellFormatTableKind`](crate::CellFormatTableKind) rather than by type — and
`the_two_xf_slots_are_told_apart_by_variant_and_not_by_type` exists because a copy-paste in the slot
macro would otherwise compile and pass every assertion about "the part has a `cellXfs`".

## Indices are identity

Nothing in a workbook names a font, a fill, a border or a `dxf`. Each is addressed by its **position**
in its table — `fontId="3"` is the fourth `<font>` — so **reordering, deduplicating or
garbage-collecting a table silently repaints every cell that referred to anything after the entry
that moved**.

Every table therefore offers exactly three operations: read one by index, iterate them, and
**append**. There is no remove, no sort, no dedupe. Two byte-identical `<font>` entries stay two
entries, and `tests/fixtures/style_resources.xlsx` writes exactly that so that
`font_indices_are_positions_and_appending_never_moves_them` has something real to fail on.

`@count` moves with an append, and **only when the file declared one** — it is a hint, not a fact,
and inventing one on an element that wrote none would change a file nobody asked to change.

`numFmts` is the exception that proves the rule: a `numFmt` is addressed by its `@numFmtId`, not by
its position, and ids below 164 are the **implied** codes of ECMA-376 §18.8.30 that no file writes at
all. [`mjx_sml::builtin_format_code`](crate::builtin_format_code) is that table, and
[`mjx_sml::is_locale_dependent`](crate::is_locale_dependent) says which of them a consumer renders
differently by locale — because reporting a locale-dependent code as if it were absolute would be a
quiet lie.

## The `xf` indirection, in one paragraph

A cell's `@s` is a position in `cellXfs`. That `xf` carries a `@fontId`, `@fillId`, `@borderId` and
`@numFmtId`, an optional `@xfId` naming a record in `cellStyleXfs`, and a set of three-state
`applyX` flags. **Absent, `true` and `false` are three different states** on those flags, which is
why every one of them reads as `Option<bool>` through
[`mjx_sml::ApplyFlag`](crate::ApplyFlag) rather than as `bool`.
[`mjx_sml::CellFormatResolver`](crate::CellFormatResolver) walks it and answers with
[`mjx_sml::EffectiveCellFormat`](crate::EffectiveCellFormat), which reports for each aspect *which
layer won* ([`mjx_sml::FormatLayer`](crate::FormatLayer)) rather than only the value — because "the
font is Calibri" and "the font is Calibri because the named style said so" are different answers to
a caller deciding whether to override it.

A cell with no `@s` is not unformatted: it names `xf` 0, which is a real record.

## Colour has four spellings and a tint

`CT_Color` is **one element with five attributes** — `auto`, `indexed`, `rgb`, `theme`, `tint` — and
no children at all, which is why [`mjx_sml::Color`](crate::Color) exists rather than
[`mjx_dml::Color`](mjx_dml::Color): DrawingML's colour is an element *choice* whose name is the kind,
and it has no representation for `indexed`, for a positional `theme` or for `tint`. Routing them
through `ColorSpec::Other` would store `indexed="8"` under an element kind that does not exist. That
is data loss dressed as reuse.

The element is named for its **slot** — `color` inside a font or a run's `rPr`, `fgColor` and
`bgColor` inside a pattern fill, `tabColor` on a sheet — never for its kind, which is why
[`mjx_sml::ColorElement`](crate::ColorElement) keeps the name it was read with.

| Spelling | Means | Resolve it with |
|---|---|---|
| `@rgb="FFFF0000"` | `ST_UnsignedIntHex`: **eight** digits, alpha first | the value itself |
| `@indexed="8"` | a row of the legacy 56-entry palette | [`mjx_sml::styles::palette::resolve_color`](crate::styles::palette::resolve_color) against the workbook's own `indexedColors` |
| `@theme="4"` | a **position** in `theme1.xml`'s colour scheme — not a `SchemeColor` token | the same resolver, against the theme |
| `@auto="1"` | the system foreground/background, whatever that is at render time | nothing here |

`@tint` shifts whichever of the four was given towards white (positive) or black (negative), and
[`mjx_sml::apply_tint`](crate::apply_tint) implements §18.8.19's HSL formula.
`a_theme_colour_resolves_to_what_drawingml_resolves_for_the_same_slot`, in
`crates/mjx-sml/tests/style_resources.rs`, holds this crate's resolver and `mjx-dml`'s to the same
answer for the same slot — one theme, two vocabularies, one colour.

### Authoring a colour: two things to know

**`@rgb` is eight digits.** [`mjx_sml::Color::from_opaque_rgb`](crate::Color::from_opaque_rgb) takes
the six-digit `RRGGBB` form a caller thinks in and supplies the opaque alpha; it also accepts an
eight-digit ARGB and leaves it alone, and drops a leading `#`. It prefixed `FF` *unconditionally*
until MJXOFF-220, so `"FFFF0000"` — the spelling `Color::rgb`'s own documentation gives — produced a
ten-character `@rgb` that `sml.xsd` rejects.
`every_authored_colour_is_a_valid_unsigned_int_hex` builds thirty-five colours through the five
constructors that take a hex literal and holds each result to eight hexadecimal digits.

**The theme-following path is the one nobody takes.** Every convenience —
[`mjx_sml::PatternFillSpec::solid`](crate::PatternFillSpec::solid),
[`mjx_sml::ColorScaleSpec::two_color`](crate::ColorScaleSpec::two_color),
[`mjx_sml::DataBarSpec::spanning_the_range`](crate::DataBarSpec::spanning_the_range),
[`mjx_sml::DifferentialFormatSpec::highlight`](crate::DifferentialFormatSpec::highlight) — takes a
hex literal, so the shortest path pins a colour into a file whose owner may have rebranded it.
Following the theme is one line longer and is what the epic's standing design rule asks for:

```
use mjx_ooxml_types::spreadsheetml::PatternType;
use mjx_sml::{Color, PatternFillSpec};

let follows_the_theme = PatternFillSpec {
    pattern: Some(PatternType::Solid),
    foreground: Some(Color::from_theme(4, Some(-0.25))),
    ..PatternFillSpec::default()
};
assert_eq!(follows_the_theme.foreground.expect("a colour").theme, Some(4));
```

## Absent is not default, and a `dxf` is a delta

A [`mjx_sml::DifferentialFormat`](crate::DifferentialFormat) states a **change**, not a format: an
absent member means *inherit*, never *take the default*. `Option` is load-bearing on every one of its
accessors, and [`mjx_sml::DifferentialFormat::inherits_everything`](crate::DifferentialFormat::inherits_everything)
is the state `<dxf/>` decodes to — which is a real thing a file writes and a different thing from the
element being absent.

The same rule governs authoring. Every field of every `…Spec` is an `Option`, and `None` writes no
attribute and no element, because `<xf/>` is a meaningful record naming font 0, fill 0, border 0 and
`General` by omission, and `<patternFill/>` states a fill whose pattern is inherited.

## One font family, two subjects

`CT_Font` — a font-table entry — is character for character the same fifteen property slots as
`CT_RPrElt`, a rich-text run's `rPr`, differing only in `rFont` versus `name` and in `family`'s
declared type. That family is modelled **once**, as
[`mjx_sml::FontProperties`](crate::FontProperties) and
[`mjx_sml::FontPropertyOwner`](crate::FontPropertyOwner) in `crates/mjx-sml/src/font/`, deliberately
outside both subjects so that neither the stylesheet nor [the shared string table](shared_strings)
has to reach into the other. `a_font_table_entry_and_a_run_decode_to_one_type` is what says the two
really do decode through the same accessors.

That is also why fonts have no `FontSpec`: `FontProperties` *is* the interner-free description, and
[`mjx_sml::Font::from_properties`](crate::Font::from_properties) is its build step.

## Table styles: the 144 that are not in the file

`tableStyles` holds the styles a workbook **defines**, plus `@defaultTableStyle` and
`@defaultPivotStyle` naming the ones it prefers. Excel's built-in table styles —
`TableStyleMedium2` and its 143 siblings — are named by those attributes and appear in no part at
all. [`mjx_sml::BuiltInTableStyle`](crate::BuiltInTableStyle) and
[`mjx_sml::builtin_table_style_name`](crate::builtin_table_style_name) are that vocabulary, so a
caller can tell "this workbook defines a style called X" from "this workbook asks for a style the
consumer supplies", and [`mjx_sml::TableStyleLookup`](crate::TableStyleLookup) reports which of the
two a name resolves through.
