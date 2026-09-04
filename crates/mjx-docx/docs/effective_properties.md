# Effective properties — what a file states versus what Word renders

A `.docx` states remarkably little about how a run actually looks. A run whose paragraph carries no
direct formatting still renders in a specific typeface, at a specific size, in a specific colour — the
answer lives in up to four other places: the document's own defaults, the numbering level the
paragraph belongs to, the paragraph style's own inheritance chain, and the character style the run
refers to.

Two families of readers answer two different questions:

- The **declared** readers — [`RunProperties`], [`ParagraphProperties`], [`StyleSheet`], and their
  kin — answer *what this part says*. A property the part does not state comes back `None`. They are
  the right readers for editing: they show what an edit would overwrite.
- The **effective** readers — [`Document::effective_run_properties`] and
  [`Document::effective_paragraph_properties`] — answer *what Word renders*. They walk `w:docDefaults`,
  the numbering level, the paragraph-style chain, and (for a run) the character-style chain, and hand
  back a self-contained, colour-baked answer.

No `effective_*` read dirties a part. Resolution parses parts the package had not needed yet — which
is why these methods take `&mut self` — but it never marks one modified.

## The APIs

| Reader | Answers |
|---|---|
| [`Document::effective_run_properties`] | the character formatting one run renders with |
| [`Document::effective_paragraph_properties`] | the layout one paragraph renders with |
| [`Document::effective_cell_fill`] | the background shading one table cell renders with |
| [`Document::effective_cell_border`] | the border on one edge of one table cell |
| [`Document::effective_cell_run_properties`] | the character formatting one run *inside a table cell* renders with — [`Document::effective_run_properties`]'s ladder plus the table-style rung |

Each returns a plain, `Default`-able struct — [`EffectiveCharacterProperties`] (38 fields, one per
`EG_RPrBase` member other than `w:rStyle`) and [`EffectiveParagraphProperties`] (32 fields, one per
`CT_PPrBase` member other than `w:pStyle`) — rather than an `Option` per property that might itself be
absent-vs-unset; every field is independently `Option`, and the struct as a whole is never itself
absent, mirroring [`mjx_pptx`](https://docs.rs/mjx-pptx)'s own choice for text formatting (see that
crate's [effective-properties guide](https://docs.rs/mjx-pptx/latest/mjx_pptx/effective_properties)):
an empty struct is a real answer ("nothing anywhere had an opinion"), not a failure to find one.

## The ladder order — verified against ECMA-376 Part 1, not assumed

This child's own ticket stated the order as `docDefaults → the paragraph style chain → the numbering
level → the character style → direct`. **That order is wrong**, and the mistake is exactly the one the
ticket itself warned was easy to make: it places the numbering level *after* the paragraph-style
chain.

ECMA-376 Part 1 §17.7.2 ("Style Hierarchy") states the application order explicitly, as a table, and
then in prose:

> First, the document defaults are applied to all runs and paragraphs in the document. Next, the table
> style properties are applied to each table in the document, following the conditional formatting
> inclusions and exclusions specified per table. Next, numbered item and paragraph properties are
> applied to each paragraph formatted with a numbering style. Next, paragraph and run properties are
> applied to each paragraph as defined by the paragraph style. Next, run properties are applied to
> each run with a specific character style applied. Finally, we apply direct formatting…

Lowest priority to highest: **document defaults → table style → numbering → paragraph style →
character style → direct formatting**. [`Document::effective_run_properties`]/
[`Document::effective_paragraph_properties`] (a plain paragraph, not inside a table cell) implement
five of those six rungs — no table applies outside a cell, so that rung simply contributes nothing:

```text
docDefaults  →  numbering level  →  paragraph-style chain  →  character-style chain  →  direct
(lowest)                                                                              (highest)
```

[`Document::effective_cell_run_properties`] (a run *inside* a table cell) adds the table-style rung
at its own place in the order — see "Table-style conditional formatting" below. It does not yet
resolve a cell paragraph's own numbering (no fixture in this crate's own table coverage carries one —
see "Where this reader stops"), so its own ladder is:

```text
docDefaults  →  table style  →  paragraph-style chain  →  character-style chain  →  direct
(lowest)                                                                          (highest)
```

Note that the numbering level sits **below** the paragraph-style chain, not above it — the opposite of
the ticket's own claim. A caller who trusted the ticket's ordering would get the wrong answer for any
document whose numbering level and paragraph style disagree about the same property, which is exactly
the scenario `tests/effective.rs`'s discriminating fixtures build.

### Within one style's own `w:basedOn` chain

§17.7.1 ("Style Inheritance") describes a **different** rule for combining a style with its ancestors:
walk from the referenced style up through `w:basedOn` to a root, and "when properties conflict, they
are overridden by each subsequent level" — i.e. plain fallback, the leaf's own value winning, an
unstated property falling through to the nearest ancestor that states it. [`StyleIndex::based_on_chain`]
(MJXOFF-101) already returns exactly this chain, leaf first; this crate's own
[`EffectiveCharacterProperties::merge_under`]/[`EffectiveParagraphProperties::merge_under`] fold over
it in that order.

## Toggle properties — combined by XOR, not by override, and only twelve of them

ECMA-376 Part 1 §17.7.3 ("Toggle Properties") states a **second, different** combination rule for a
named list of run-level Boolean properties:

> If a toggle property is explicitly set in direct formatting … its value … shall be used. Otherwise,
> the instances of that toggle property in the styles that affect the content shall be combined … If
> the value specified by the document defaults is `true`, the effective value is `true`. Otherwise, the
> values are combined by a Boolean XOR.

The twelve properties this applies to, named explicitly in §17.7.3, are: `w:b`, `w:bCs`, `w:caps`,
`w:emboss`, `w:i`, `w:iCs`, `w:imprint`, `w:outline`, `w:shadow`, `w:smallCaps`, `w:strike`, `w:vanish`.
Every other `CT_OnOff`-shaped `EG_RPrBase`/`CT_PPrBase` member — `w:dstrike`, `w:noProof`,
`w:snapToGrid`, `w:webHidden`, `w:rtl`, `w:cs`, `w:specVanish`, `w:oMath`, and all eighteen paragraph-
level on/off members — is **not** on that list, despite the identical wire shape, and combines by plain
override like every other property.

Concretely, `combine_toggle` implements exactly this: a direct value
wins outright; failing that, a `true` at `docDefaults` wins outright; failing that, the table
style's (see below — `None` outside a table cell, so it drops out of the XOR entirely there), the
numbering level's, the paragraph-style chain's (already resolved via plain fallback within the
chain), and the character-style chain's (ditto) own values combine by XOR, a tier with no opinion
simply not contributing — "an odd number of levels of the style hierarchy" (§17.7.3's own phrase)
generalizes to however many levels genuinely apply, not a fixed count.

### The trap this proves

A **naive implementation treats every `CT_OnOff` property as plain override** — the same rule as every
other field. That is wrong for exactly these twelve, and the case where it goes wrong is not "a bold
style based on a bold style" in the loose sense (within one chain, plain fallback already gives the
right, unsurprising answer): it is a run whose **paragraph style** states `w:b` and whose **character
style** (via `w:rStyle`) *also* states `w:b`, both `true`. A naive override-based resolver — character
style beats paragraph style, both agree, so the run is bold — is wrong. Word's own rule (`true XOR
true = false`) renders the run **not bold**. `tests/effective.rs`'s
`a_paragraph_style_and_character_style_both_bold_cancel_to_not_bold` builds exactly this fixture and
asserts the XOR answer; forcing `combine_toggle` to plain-override
instead turns it red (see that test's own doc comment for the pasted failure).

## Table-style conditional formatting

A table style (`w:style[@type='table']`) can carry, beyond its own base `w:tblPr`/`w:pPr`/`w:rPr`,
up to twelve **conditionally-formatted regions** (`w:tblStylePr`, one per [`ConditionalFormatRegion`]
— first row, last row, first/last column, the four corners, and odd/even row and column bands).
Which regions cover a given cell is computed once per `(row, column)` from the table's own
`w:tblLook` flags, band sizes and dimensions ([`applicable_regions`]) — never re-derived per property
read, the cost discipline "Cost and caching" already states for the rest of this ladder.

**The precedence, verified against ECMA-376 Part 1 §17.7.6.6 itself, not assumed:**

> When specified, these conditional formats shall be applied in the following order (therefore
> subsequent formats override properties on previous formats): Whole table; Banded columns, even
> column banding; Banded rows, even row banding; First row, last row; First column, last column;
> Top left, top right, bottom left, bottom right.

[`applicable_regions`] returns regions in exactly this order — least specific first — and every
`effective_cell_*` reader folds them left to right, each later region's stated properties overriding
the earlier ones'. Two consequences that read backwards on a first guess: **column edges beat row
edges** (a first-row region and a first-column region disagreeing resolve to the *column*'s answer —
`tests/table_formatting.rs`'s `first_column_wins_over_first_row_when_both_regions_disagree` proves
it, with the region push order reversed as the mutation that turns it red), and **row banding beats
column banding**.

A cell's own direct formatting (`w:tcPr/w:shd`, `w:tcPr/w:tcBorders`, a run's own `w:rPr`) still wins
outright over every region, exactly as direct formatting always wins the rest of this ladder.

**`w:tblLook`/`w:cnfStyle`'s `val` is never the authority for region membership.** Both elements
carry a legacy bitmask-shaped `val` attribute in `wml.xsd`'s Transitional schema alongside their
named `ST_OnOff` flags (`firstRow`, `oddHBand`, …) — but ECMA-376 Part 1's own prose for both
(§17.3.1.8, §17.4.7, §17.4.8, §17.4.55) documents only the named attributes, `val` is never mentioned,
and every worked example writes the named attributes directly. This crate reads region membership
exclusively from the named flags; `val` round-trips for fidelity and nothing else consults it.

## Theme colours and fonts

`w:color`/`w:u`/`w:bdr`/`w:shd`'s own `themeColor` (+ `themeTint`/`themeShade`) and `w:rFonts`'s
`asciiTheme`/`hAnsiTheme`/`eastAsiaTheme`/`cstheme` are resolved against this document's
`word/theme/themeN.xml` through `mjx-dml`'s own theme model — **the same model
[`mjx_pptx`](https://docs.rs/mjx-pptx) uses**, not a second one. `grep -rn 'struct.*Theme\|enum.*Theme'
crates/mjx-docx/src/` finds exactly two matches, neither a model: `ThemeHexDigit`
(`run_properties.rs`, MJXOFF-94 — an attribute *codec* for a two-digit hex byte, unrelated to a
theme part) and this module's own `ThemeContext`, which **holds** `mjx_dml::SchemeColors` and
`mjx_dml::FontScheme` directly rather than defining color-scheme slots or font collections of its
own — a bridge, not a second model. `ST_ThemeColor`'s seventeen wire tokens map onto
DrawingML's `a:schemeClr` vocabulary (`crate::document::effective::word_theme_color_to_scheme_color`),
and the `background1`/`text1`/`background2`/`text2` half of that mapping goes through
[`mjx_dml::ColorMap::identity`] directly — its own doc comment states the identical default pairing
(`bg1→lt1`, `tx1→dk1`, `bg2→lt2`, `tx2→dk2`) that ECMA-376 Part 1 §17.15.1.20 states for Word's
`w:clrSchemeMapping` element when it is absent, which is the case for every fixture in this workspace
(`word/settings.xml`'s `w:clrSchemeMapping` is not modelled by any child yet — see "Where this reader
stops"). `ST_Theme`'s eight wire tokens (`w:rFonts`'s theme attributes) map onto the font scheme's
major/minor × Latin/East-Asian/complex-script slots the same way `mjx_pptx` resolves `+mj-lt`-style
references, with `majorAscii`/`majorHAnsi` (and their minor counterparts) both naming the theme's one
Latin typeface — DrawingML's font scheme has no separate "High ANSI" slot, because that distinction is
WordprocessingML's own.

A theme reference the theme does not define keeps its unresolved form — the file points somewhere the
theme does not go, and that is the honest answer, not a guess.

## Where this reader stops

| Rung / concern | Status |
|---|---|
| `word/settings.xml`'s `w:clrSchemeMapping` | **Not modelled.** No child has built `settings.xml` yet. This reader always applies the *default* mapping (see above); a document that overrides it would resolve a theme colour differently in real Word than this reader reports. |
| `w:themeTint`/`w:themeShade` | Read back on [`EffectiveColor`]'s own theme-colour siblings but **not baked into the resolved `RRGGBB`** — baking them would mean either reimplementing DrawingML's tint/shade transform outside `mjx-dml` or constructing synthetic XML carrying it, and `mjx-dml`'s own transform application is `pub(crate)`, reachable only through a `Color` value that already carries the transform as a parsed child. A caller who needs the exact shade applies it from the raw tint/shade byte. |
| A table cell paragraph's own numbering (`w:numPr`) | **Not resolved by [`Document::effective_cell_run_properties`]** — no fixture in this crate's own table coverage carries one; a caller with that need extends it alongside [`Document::effective_run_properties`]'s own numbering resolution. |
| A table cell's *paragraph* properties (indentation, spacing, borders, shading beyond `w:tcPr/w:shd`) resolved through the table-style ladder | Only the three named readers exist — [`Document::effective_cell_fill`], [`Document::effective_cell_border`] and [`Document::effective_cell_run_properties`], matching `mjx_pptx::Presentation`'s own three cell readers. A cell's *paragraph-level* effective properties (as opposed to the cell's own fill/border, or a run inside it) are not separately resolved through the table-style ladder. |
| Section-level property resolution (`w:sectPr`) | [`MJXOFF-109`]'s. |
| Computed list numbers (`1.`, `a)`, …) | **Deliberately not computed** — inherited boundary from numbering.xml's own module doc (MJXOFF-104): counting preceding paragraphs, `w:lvlRestart`, restart-on-higher-level and section continuation are all needed and none is modelled. `EffectiveParagraphProperties`'s own `numbering` field hands back *which* numbering definition and level apply, never the rendered digit/letter. |
| The paragraph mark's own run properties (`w:pPr/w:rPr`, `CT_ParaRPr`) | Not resolved by either reader — a genuinely different question (the pilcrow's own appearance, not a run's or the paragraph's own layout). |
| Field results, table-layout-dependent widths, anything needing line breaking | Not modelled anywhere in this workspace; this reader answers what the *properties* resolve to, not what a line-breaking layout engine would place where. |

## Cost and caching

One `effective_*` call reads `word/document.xml`, `word/styles.xml` and, when the paragraph is in a
list, `word/numbering.xml` — each already parsed once and kept by
[`mjx_opc::Package::part_tree`], so no XML is re-parsed on a second call. What **is** rebuilt each
call is the *typed* [`StyleIndex`] and the `w:basedOn` chain walk each field needs: the ladder resolves
all 38 (or 32) fields of one call by walking each relevant chain **exactly once** — see
`crate::document::effective::ChainCache`, a memo keyed by `styleId` scoped to a single `effective_*`
call — never once per field, which is the allocation the ticket's own brief warned against.

That cache does **not** survive across separate `effective_run_properties`/
`effective_paragraph_properties` calls, and there is nothing to "invalidate": it is dropped when the
call returns. The reason is structural, not a missed optimization: [`mjx_ooxml_core::Interner`] does
not implement `Clone`, and every `Document` accessor already re-parses its part fresh on each call (see
[`Document::style_sheet`]'s own doc comment) — a cache that outlived one call would need to own a
`StyleSheet` *and* its `Interner` together, which is exactly the shape `Interner`'s missing `Clone`
rules out without a larger change to this crate's own architecture. A caller resolving many runs
across one document and wanting to avoid the per-call rebuild can call [`Document::style_sheet`] once,
hold the returned `(&StyleSheet, &Interner)` for the loop's own duration, and drive
[`StyleIndex::build`]/[`StyleIndex::based_on_chain`] directly — the same two calls this ladder makes
internally, just hoisted to the caller's own scope.

## Examples

Declared versus effective, on a run that states nothing of its own:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.append_run(0, "Hello, document.")?;

// What the run says: nothing.
assert!(document.run_text(0, 0)?.len() > 0);

// What it renders as: whatever docDefaults/numbering/styles add up to (nothing, for a blank
// document with no styles.xml at all — every field comes back `None`).
let effective = document.effective_run_properties(0, 0)?;
assert_eq!(effective.bold, None);
# Ok(())
# }
```

The paragraph ladder, on the same blank document:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let paragraph = document.effective_paragraph_properties(0)?;
assert_eq!(paragraph.alignment, None);
# Ok(())
# }
```

A table cell, on a blank document with no table style referenced at all — every cell reader degrades
to "the cell's own direct formatting, nothing more", the same "no style, no opinion" behaviour every
other tier in this ladder already has:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let table = document.append_table(1, 1)?;
document.set_cell_text(table, 0, 0, "Hello, cell.")?;

// No `w:tblStyle`, so no region contributes anything; the cell states no `w:tcPr/w:shd` either.
assert_eq!(document.effective_cell_fill(table, 0, 0)?, None);

// Same story for the cell's one run: no table style, no paragraph/character style, no direct
// formatting.
let run = document.effective_cell_run_properties(table, 0, 0, 0, 0)?;
assert_eq!(run.bold, None);
# Ok(())
# }
```

[`RunProperties`]: crate::RunProperties
[`ParagraphProperties`]: crate::ParagraphProperties
[`StyleSheet`]: crate::StyleSheet
[`StyleIndex`]: crate::StyleIndex
[`StyleIndex::based_on_chain`]: crate::StyleIndex::based_on_chain
[`StyleIndex::build`]: crate::StyleIndex::build
[`Document::effective_run_properties`]: crate::Document::effective_run_properties
[`Document::effective_paragraph_properties`]: crate::Document::effective_paragraph_properties
[`Document::style_sheet`]: crate::Document::style_sheet
[`EffectiveCharacterProperties`]: crate::EffectiveCharacterProperties
[`EffectiveCharacterProperties::merge_under`]: crate::EffectiveCharacterProperties::merge_under
[`EffectiveParagraphProperties`]: crate::EffectiveParagraphProperties
[`EffectiveParagraphProperties::merge_under`]: crate::EffectiveParagraphProperties::merge_under
[`EffectiveColor`]: crate::EffectiveColor
[`mjx_dml::ColorMap::identity`]: https://docs.rs/mjx-dml/latest/mjx_dml/struct.ColorMap.html
[`MJXOFF-109`]: https://github.com/jaxmatrix/mjx-ooxml-rs
[`Document::effective_cell_fill`]: crate::Document::effective_cell_fill
[`Document::effective_cell_border`]: crate::Document::effective_cell_border
[`Document::effective_cell_run_properties`]: crate::Document::effective_cell_run_properties
[`ConditionalFormatRegion`]: crate::ConditionalFormatRegion
[`applicable_regions`]: crate::applicable_regions
