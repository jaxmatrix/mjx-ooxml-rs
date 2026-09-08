# Filling, outlining and colouring

You have a shape — a `p:sp` on a slide, an `xdr:sp` on a sheet, a `pic:pic` in a paragraph — and you
want it to look like something. Everything in this page hangs off one element: its `spPr`.

## The one element: `spPr`

[`mjx_dml::ShapeProperties`](crate::ShapeProperties) is `CT_ShapeProperties` — transform, geometry,
fill, outline, effects, 3-D — and it is a **fidelity wrapper**: the storage is the raw child list, and
every accessor reads out of it on demand. Nothing is separated out, so nothing can be lost.

```text
sp_pr.fill(interner)        → Option<Fill>            a:noFill | a:solidFill | a:gradFill
sp_pr.set_fill(…)                                     | a:blipFill | a:pattFill | a:grpFill
sp_pr.line(interner)        → Option<LineProperties>  a:ln
sp_pr.set_line(…)
sp_pr.effects(interner)     → Option<EffectList>      a:effectLst
sp_pr.transform(interner)   → Option<Transform2D>     a:xfrm
sp_pr.geometry(interner)    → Option<ShapeGeometryChoice>
```

A setter that finds the child already there **edits it in place**; one that does not inserts the new
child at its rank in `CT_ShapeProperties`'s sequence, taken from the generated
[`SHAPE_PROPERTIES`](mjx_ooxml_types::child_order::SHAPE_PROPERTIES) table rather than from a rank
list written by hand here. Child order in DrawingML is validity, not style: a fill written after an
outline makes the part reject.

## Fill

[`mjx_dml::Fill`](crate::Fill) is the `EG_FillProperties` choice — six element names, six variants,
each of them a fidelity wrapper of its own:

| Variant | Element | Type |
|---|---|---|
| `None` | `a:noFill` | [`mjx_dml::NoFill`](crate::NoFill) |
| `Solid` | `a:solidFill` | [`mjx_dml::SolidFill`](crate::SolidFill) |
| `Gradient` | `a:gradFill` | [`mjx_dml::GradientFill`](crate::GradientFill) |
| `Picture` | `a:blipFill` | [`mjx_dml::PictureFill`](crate::PictureFill) |
| `Pattern` | `a:pattFill` | [`mjx_dml::PatternFill`](crate::PatternFill) |
| `Group` | `a:grpFill` | [`mjx_dml::GroupFill`](crate::GroupFill) |

[`mjx_dml::Fill::from_xml`](crate::Fill) dispatches on the element's **local name**, and an unknown or
malformed name lands in `Group` rather than erroring — a `GroupFill` is the shallowest of the six
wrappers, so an unrecognised fill still round-trips byte for byte instead of being rejected.
[`mjx_dml::Fill::is_fill_local`](crate::Fill::is_fill_local) is the predicate a host uses to find one
among an `spPr`'s children.

The describable half is [`mjx_dml::FillSpec`](crate::FillSpec), with
[`mjx_dml::FillSpec::solid`](crate::FillSpec::solid),
[`linear_gradient`](crate::FillSpec::linear_gradient) and [`pattern`](crate::FillSpec::pattern) as its
constructors and [`mjx_dml::FillSpec::to_fill`](crate::FillSpec::to_fill) to build the element.

**A spec is not a view.** `fill.spec(i).to_fill(i)` is *not* the identity: the spec carries the stops,
the angle, the relationship id and the mode, and drops the gradient's shade path, the blip's effect
chain and the tile/fill rectangles. Read a fill through the spec; edit one through the wrapper.

## Outline

[`mjx_dml::LineProperties`](crate::LineProperties) is `a:ln`. Width is a
[`mjx_dml::LineWidth`](crate::LineWidth) (EMU on the wire, points if you ask for points, because every
UI that shows a line weight shows points); the dash, cap, join, compound kind and both line ends are
typed. [`mjx_dml::LineSpec`](crate::LineSpec) is the interner-free description, with
[`solid`](crate::LineSpec::solid) as its one constructor and
[`to_line`](crate::LineSpec::to_line) / [`to_line_named`](crate::LineSpec::to_line_named) to build —
`to_line_named` because an outline element is `a:ln` in most hosts and `a:lnRef`-adjacent elsewhere,
and the host is the one that knows.

## Colour, and the honest account of what it can say

[`mjx_dml::Color`](crate::Color) is any one of the six `EG_ColorChoice` elements — `a:srgbClr`,
`a:schemeClr`, `a:sysClr`, `a:prstClr`, `a:scrgbClr`, `a:hslClr`. The **element name is the kind**,
which is why it is a fidelity wrapper rather than a derived container: `@val` is shared by all six and
means something different in each, so [`mjx_dml::Color::hex`](crate::Color::hex) and
[`mjx_dml::Color::scheme_color`](crate::Color::scheme_color) interpret it only once
[`mjx_dml::Color::kind`](crate::Color::kind) has said which kind this is.

Its **children are colour transforms** — `a:lumMod`, `a:lumOff`, `a:shade`, `a:tint`, `a:alpha`,
`a:satMod`, `a:comp`, `a:gray`, `a:gamma`, `a:invGamma` and the rest of `EG_ColorTransform`. A `Color`
keeps every one of them verbatim, in order, and hands them back through
[`mjx_dml::Color::transforms`](crate::Color::transforms).

### Authoring one

[`mjx_dml::ColorSpec`](crate::ColorSpec) is the interner-free description, and therefore the only
thing an authoring caller hands in. Until MJXOFF-219 it had three variants and **none of them carried
a transform**, so nothing in this workspace could author one and validation entry `V-PPTX-02.4` — the
third-highest risk item in the repository — had no file to exercise it. It now carries them, in the
one shape that keeps the schema's own claim that a transform is a *child of a colour* rather than a
different kind of colour:

```rust,ignore
pub enum ColorSpec {
    Srgb(String),                                  // a:srgbClr @val
    Scheme(SchemeColor),                           // a:schemeClr @val
    Other { kind: ColorKind, value: Option<String> },
    Transformed { base: Box<ColorSpec>, transforms: Vec<ColorTransform> },
}
```

You do not build that fourth variant by hand. The builders do, and they keep it exactly one level
deep:

```rust
use mjx_dml::{ColorSpec, ColorTransform, Fraction, SchemeColor};

// What PowerPoint writes for "Accent 1, Lighter 40 %".
let lighter = ColorSpec::Scheme(SchemeColor::Accent1)
    .with_luminance_modulation(Fraction::from_ratio(0.6))
    .with_luminance_offset(Fraction::from_ratio(0.4));

// Anything else in EG_ColorTransform goes through the generic builder.
let inverted = ColorSpec::Srgb("4472C4".into()).with_transform(ColorTransform::InverseGamma);

assert_eq!(lighter.base(), &ColorSpec::Scheme(SchemeColor::Accent1));
assert_eq!(lighter.transforms().len(), 2);
assert_eq!(inverted.transforms().len(), 1);
```

Six conveniences — [`with_tint`](crate::ColorSpec::with_tint),
[`with_shade`](crate::ColorSpec::with_shade), [`with_alpha`](crate::ColorSpec::with_alpha),
[`with_luminance_modulation`](crate::ColorSpec::with_luminance_modulation),
[`with_luminance_offset`](crate::ColorSpec::with_luminance_offset) and
[`with_saturation_modulation`](crate::ColorSpec::with_saturation_modulation) — cover the transforms
a real file actually contains. **All twenty-eight members of the group are reachable** through
[`with_transform`](crate::ColorSpec::with_transform) and
[`mjx_dml::ColorTransform`](crate::ColorTransform); [`ColorTransformKind`](crate::ColorTransformKind)
names them without their values, which is how the two bindings reach the group from languages with no
payload enumerations.

Two things about that surface are load-bearing:

* **Every builder appends.** `EG_ColorTransform` is an unbounded `xsd:choice` applied left to right,
  so the same transforms in a different order are a **different colour**. A builder that merged into a
  set would quietly write a different file.
* **A transform this model cannot read still round-trips.** An element the group does not name, or one
  it does whose `@val` is missing or unparseable, becomes
  [`ColorTransform::Other`](crate::ColorTransform::Other) and comes back out under the name and value
  it went in with — the same bargain [`ColorSpec::Other`](crate::ColorSpec::Other) makes for the
  colour element itself.

Reading was always complete, and it now closes with the write path: a `Color` parsed from a file keeps
its transforms and re-serializes them byte for byte;
[`mjx_dml::resolve_color`](crate::resolve_color) *applies* them at every level of the chain, including
the theme slot's own and the substituted `phClr`'s; and
[`mjx_dml::Color::spec`](crate::Color::spec) now carries them into the description, so
`spec()` → [`from_spec`](crate::Color::from_spec) keeps a producer's transforms instead of dropping
them.

One further caution on the resolver, from its own module doc: `lumMod`/`lumOff`/`shade`/`tint`/
`alpha`/`sat*` follow the widely-adopted Apache-POI and LibreOffice algorithm and are value-pinned in
`crates/mjx-dml/tests/color_model.rs`; `comp`/`gray`/`gamma`/`invGamma` follow a documented
interpretation and are **not** guaranteed pixel-identical to Microsoft Office's renderer.

## From a name to a number

A `a:schemeClr val="accent1"` is not a colour, it is a lookup. Resolving one takes three things, and
they live in three different places:

```text
Color (the shape's own interner)
  └─ schemeClr "accent1"
        └─ ColorMap        p:clrMap / p:clrMapOvr  — the slide's logical→slot mapping
              └─ SchemeColors    the theme's slots, already baked to RGB
```

[`mjx_dml::SchemeColors`](crate::SchemeColors) exists because the theme lives in a *different part*
with a *different interner*, so the scheme is flattened to slot→RGB once and then
[`mjx_dml::resolve_color`](crate::resolve_color) works entirely in the shape's own interner.
[`mjx_dml::ColorMap`](crate::ColorMap) is the `bg1`/`tx1`/`bg2`/`tx2` + accents mapping;
`dk1`/`lt1`/`dk2`/`lt2` name a slot directly and bypass it, and `phClr` is not a scheme colour at all
— it is the placeholder a [`mjx_dml::StyleMatrixReference`](crate::StyleMatrixReference) substitutes.
[The theme](the_theme) is the whole of that story.

Nothing in `crates/mjx-dml/src/resolve.rs` returns an error, deliberately: every entry point answers
*what does this actually look like*, and "the file does not say" and "the file says something
unreadable" are the same answer to that question.
