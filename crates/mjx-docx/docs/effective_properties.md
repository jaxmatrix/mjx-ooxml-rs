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
character style → direct formatting**. Table-style conditional formatting is [`MJXOFF-119`]'s (this
reader leaves that seam exactly where it is — see "Where this reader stops" below), so the ladder this
crate implements today is:

```text
docDefaults  →  numbering level  →  paragraph-style chain  →  character-style chain  →  direct
(lowest)                                                                              (highest)
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
wins outright; failing that, a `true` at `docDefaults` wins outright; failing that, the numbering
level's, the paragraph-style chain's (already resolved via plain fallback within the chain), and the
character-style chain's (ditto) own values combine by XOR, a tier with no opinion simply not
contributing.

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
| Table-style conditional formatting, effective cell formatting | [`MJXOFF-119`]'s — the table-style rung between `docDefaults` and `numbering` in §17.7.2's own order is real and simply not implemented here; a table cell's effective run/paragraph properties will be missing that rung until that child lands. |
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
[`MJXOFF-119`]: https://github.com/jaxmatrix/mjx-ooxml-rs
[`MJXOFF-109`]: https://github.com/jaxmatrix/mjx-ooxml-rs
