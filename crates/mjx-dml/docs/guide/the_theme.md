# The theme

A theme part is DrawingML, and all three formats carry the identical thing under a different name:
`/ppt/theme/theme1.xml`, `/word/theme/theme1.xml`, `/xl/theme/theme1.xml`. It holds the twelve
colours a document's palette is made of, two font collections, and three "style matrices" a shape can
point into instead of stating its own fill, outline or effects.

Almost every colour in a real file is a *reference into this part*, which is why reading a theme is
the difference between "this shape says `accent1`" and "this shape is `#4472C4`".

## Reading one

[`mjx_dml::Theme`](crate::Theme) is a **read-only projection**, and that is the first thing to know
about it. It implements [`FromXml`](mjx_ooxml_core::FromXml) and no
[`ToXml`](mjx_ooxml_core::ToXml) at all: it keeps the colour scheme, the font scheme and the three
style-matrix lists, and drops `a:bgFillStyleLst`, the `@name` attributes and every unknown child,
because resolving a fill never needs them. Serializing that view would emit a theme with holes in it,
so nothing can — the type has no writer to misuse. `crates/mjx-dml/tests/serialization_ledger.rs` is
where that claim is checked rather than asserted.

```text
Theme::color_scheme()    → ColorScheme      twelve slots: dk1 lt1 dk2 lt2, six accents, two hyperlink
Theme::font_scheme()     → FontScheme       major + minor, three scripts each
Theme::fill_style(idx)   → Fill             a:fmtScheme > a:fillStyleLst,   1-based
Theme::line_style(idx)   → LineProperties   a:fmtScheme > a:lnStyleLst,     1-based
Theme::effect_style(idx) → EffectList       a:fmtScheme > a:effectStyleLst, 1-based
```

The style-matrix indices are **1-based** because `a:fillRef@idx` is, and `0` means *no reference* —
[`mjx_dml::Theme::fill_style`](crate::Theme::fill_style) answers `None` for it rather than silently
returning the first entry. [`mjx_dml::ColorSchemeSlot`](crate::ColorSchemeSlot) is the twelve, and it
is generated rather than written here: `Dark1`, `Light1`, `Dark2`, `Light2`, `Accent1`..`Accent6`,
`Hyperlink`, `FollowedHyperlink`.

[`mjx_dml::ThemeInfo`](crate::ThemeInfo) is the interner-free twin, and the one to reach for across a
part boundary: the same content as [`ColorSpec`](crate::ColorSpec)s,
[`FillSpec`](crate::FillSpec)s, [`LineSpec`](crate::LineSpec)s and
[`EffectListSpec`](crate::EffectListSpec)s. It is a *spec*, so the same caution applies as everywhere
else — a fill style's key values survive, its opaque internals and colour transforms do not, which is
exactly why [`Theme`](crate::Theme) keeps the interner-bound values for the colour resolver to use.

## Resolving a colour through it

```text
a:schemeClr val="accent1"      the shape says a name
      │
      ├── ColorMap             p:clrMap on the master, p:clrMapOvr on the slide
      │                        maps bg1/tx1/bg2/tx2 and the accents onto slots
      │                        dk1/lt1/dk2/lt2 name a slot directly and skip this
      ▼
   ColorSchemeSlot             one of the twelve
      │
      ▼
   SchemeColors                the theme's slots, already baked to RGB
```

[`mjx_dml::SchemeColors`](crate::SchemeColors) exists because of the interner boundary and nothing
else: the theme is a different part, so its symbols mean nothing in the shape's interner. Flatten the
scheme once, then [`mjx_dml::resolve_color`](crate::resolve_color) runs entirely in the shape's own
interner and never has to hold both.

`phClr` is the odd one out. It is not a scheme colour at all — it is a *placeholder* inside a theme
style, substituted with the colour the shape's own
[`mjx_dml::StyleMatrixReference`](crate::StyleMatrixReference) carries. That is how one theme fill
style serves six accent colours.

Colour transforms are applied at **every level of that chain** — the reference's own, the
placeholder's, and each scheme slot's — in document order.
[`mjx_dml::ResolvedColor`](crate::ResolvedColor) is the answer, and
[`mjx_dml::resolve_fill`](crate::resolve_fill), [`resolve_line`](crate::resolve_line),
[`resolve_effects`](crate::resolve_effects) and
[`resolve_character_properties`](crate::resolve_character_properties) are the same walk for the four
things a shape can carry.

## Fonts

[`mjx_dml::FontScheme`](crate::FontScheme) holds two [`FontCollection`](crate::FontCollection)s —
major (headings) and minor (body) — each with a Latin, East-Asian and complex-script typeface plus a
list of [`SupplementalFont`](crate::SupplementalFont)s per script. A run's typeface may be a
*reference* rather than a name: `+mj-lt` means "the major collection's Latin font".
[`mjx_dml::TextFont::theme_reference`](crate::TextFont::theme_reference) parses one into a
[`mjx_dml::ThemeFontReference`](crate::ThemeFontReference), and
[`mjx_dml::FontScheme::font`](crate::FontScheme::font) answers it.

## Authoring one — and the rule that governs it

A document opened from disk **keeps the theme it came with**. A package that has none, and needs one,
gets exactly one: [`mjx_dml::default_theme_xml`](crate::default_theme_xml).

That is a **byte producer**, not a writer over [`Theme`](crate::Theme), for the reason above — the
view has holes and a serialization of it would too. It returns a complete `a:theme` document carrying
the Office 2013 palette, so a file built here looks like a file built in Word rather than like a
debugging artefact, and it is deterministic: two calls return the same bytes.
[`mjx_dml::DEFAULT_THEME_XML`](crate::DEFAULT_THEME_XML) is the same string without the copy.

Three choices inside it are deliberate and worth knowing before you copy it:

* `dk1`/`lt1` are plain `a:srgbClr`, not `a:sysClr`, so the value is the same everywhere — which is
  what the effective-colour readers resolve against.
* The three fill styles are one colour at three strengths, with `phClr` as the placeholder each
  shape's `a:fillRef` substitutes; the three line styles are three widths.
* The three effect styles are **empty**. `a:effectStyle` requires an effect group, and an empty
  `a:effectLst` is the honest way to say "no effect" rather than inventing a shadow nobody asked for.

### Author only where the package has none

Four crates place this part, and the rule is the same in all four:

| Crate | When |
|---|---|
| `mjx-pptx` | `blank()` — a deck built from nothing always gets one |
| `mjx-sml` | an authored workbook always gets one |
| `mjx-docx` | `ensure_theme_part` — **only if the package carries none** |
| `mjx-xlsx` | `ensure_theme_part` — **only if the package carries none** |

The two `ensure_theme_part` functions decide "has a theme" **by content type over the whole
package**, not by the document part's own relationships: a theme reached from a header, from a
glossary document or from something the crate does not classify is still a theme the document came
with, and a rule that only looked at one part's relationships would author a second one beside it.

The reason is the standing design rule of this project, and it is not a detail: *supply a default
only in the absence of the user's own, never in place of it.* A theme decides what every `accent1` in
the file resolves to, so writing one over a document that already had one would silently re-brand
every shape in it.
