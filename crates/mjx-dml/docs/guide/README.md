# Guide

**DrawingML is the vocabulary the other three formats draw in.** A shape on a slide, a picture in a
Word paragraph and a chart floating over an Excel sheet are three different host wrappers around the
same fills, outlines, colours, transforms and text bodies — and this crate is where those live, once,
so that a fill written into a `.pptx` and a fill written into an `.xlsx` are the same code.

`mjx-dml` is **rank 2.0**: it may reach `mjx-ooxml-types`, `mjx-mce`, `mjx-xml` and `mjx-ooxml-core`,
and nothing above. `mjx-pptx`, `mjx-docx`, `mjx-xlsx`, `mjx-sml` and `mjx-chart` all reach *down* into
it, which is why it holds the host wrappers for schemas none of them can name alone — and why it can
never type a payload (a Word text box, a chart's `c:chart`) that belongs to a crate above it.

Six pages, in reading order.

| Page | Read it when |
|---|---|
| [Reaching the shared types](reaching_the_shared_types) | You are wiring a format crate onto this one, or wondering which crate owns a wrapper |
| [Filling, outlining and colouring](filling_outlining_and_colour) | You have a shape and want it to look like something |
| [Geometry and placement](geometry_and_placement) | You want it to be *somewhere*, or to be a different shape |
| [Text bodies](text_bodies) | The shape has words in it |
| [The theme](the_theme) | A colour is a name rather than a value, or a package needs a theme part |
| [Fidelity and the known gaps](fidelity_and_gaps) | Before you rely on something in production |

Two design notes sit behind pages 3 and 2: `docs/DRAWINGML_PRESET_SHAPES.md` is the preset-shape
catalogue, and `docs/DRAWINGML_FILL_HANDOFF.md` the fill model's original hand-off.
`crates/mjx-ooxml/docs/shared_markup_reachability.md` answers a different question from page 1's —
which of these types a *facade* caller can reach, rather than which crate models what.

## The shape of the API, in one page

There are a thousand-odd public items here and you do not need to meet them. Four facts explain the
lot.

### 1 · Nothing is a document; every type is a view over one element

There is no `Drawing` object that owns a tree. [`mjx_dml::Fill`](crate::Fill),
[`mjx_dml::Color`](crate::Color), [`mjx_dml::LineProperties`](crate::LineProperties),
[`mjx_dml::TextBody`](crate::TextBody) and their sixty-odd siblings each wrap **one element** of a
part someone else owns: each is built from a [`RawElement`](mjx_ooxml_core::RawElement) with
[`FromXml`](mjx_ooxml_core::FromXml) and rebuilt with [`ToXml`](mjx_ooxml_core::ToXml), and the format
crate above puts the rebuilt element back where it came from. That is what lets one fill model serve
`p:spPr`, `pic:spPr` and `xdr:spPr` without any of them knowing about the others.

It is also why every type keeps four things it does not model — the element's qualified **name with
its source prefix**, **all** its attributes verbatim, its **self-closing flag**, and every child it
has no accessor for. [Fidelity and the known gaps](fidelity_and_gaps) is the mechanism and its
exceptions.

### 2 · Interner-bound value, interner-free spec

Strings in a preservation tree are symbols in that part's [`Interner`](mjx_ooxml_core::Interner), so a
type that holds one is only meaningful beside that interner. Every such type a caller might
reasonably want to *describe* has a plain-Rust twin whose name ends in `Spec`:

| The view | The description |
|---|---|
| [`mjx_dml::Fill`](crate::Fill) | [`mjx_dml::FillSpec`](crate::FillSpec) |
| [`mjx_dml::Color`](crate::Color) | [`mjx_dml::ColorSpec`](crate::ColorSpec) |
| [`mjx_dml::LineProperties`](crate::LineProperties) | [`mjx_dml::LineSpec`](crate::LineSpec) |
| [`mjx_dml::EffectList`](crate::EffectList) | [`mjx_dml::EffectListSpec`](crate::EffectListSpec) |
| [`mjx_dml::Scene3D`](crate::Scene3D) / [`mjx_dml::Shape3D`](crate::Shape3D) | [`mjx_dml::Scene3DSpec`](crate::Scene3DSpec) / [`mjx_dml::Shape3DSpec`](crate::Shape3DSpec) |
| [`mjx_dml::CharacterProperties`](crate::CharacterProperties) | [`mjx_dml::CharacterPropertiesSpec`](crate::CharacterPropertiesSpec) |
| [`mjx_dml::ParagraphProperties`](crate::ParagraphProperties) | [`mjx_dml::ParagraphPropertiesSpec`](crate::ParagraphPropertiesSpec) |
| [`mjx_dml::CustomGeometry`](crate::CustomGeometry) / [`mjx_dml::Path2D`](crate::Path2D) | [`mjx_dml::CustomGeometrySpec`](crate::CustomGeometrySpec) / [`mjx_dml::Path2DSpec`](crate::Path2DSpec) |

A spec is a **value description, not a fidelity view**. Converting a view to a spec and back rebuilds
the element from its key values and drops the opaque internals — a gradient's shade path, a blip's
effect chain, a fill rectangle. [`mjx_dml::Fill::spec`](crate::Fill::spec) says so on itself; the same
caution applies to every row above.

### 3 · Three verbs, and the difference between them matters

```text
value.spec(interner)          →  read: what does this element say?
spec.to_fill(interner)        →  build a fresh element from the spec alone
value.apply(&spec, interner)  →  merge the spec onto the element that is already there
```

[`mjx_dml::CharacterProperties::apply`](crate::CharacterProperties::apply) and
[`mjx_dml::ParagraphProperties::apply`](crate::ParagraphProperties::apply) write only the fields the
spec names and leave everything else — `lang`, `dirty`, a hyperlink, an unmodelled child — exactly
where it was. **An unset field means "don't touch", never "remove".** That is what makes bolding a
run PowerPoint wrote a non-destructive operation. To *clear* what an element carried, build a fresh
one with [`mjx_dml::CharacterPropertiesSpec::to_properties`](crate::CharacterPropertiesSpec::to_properties)
instead. [`mjx_dml::Transform2D::apply`](crate::Transform2D::apply) is the same contract for `a:xfrm`,
down to inserting a missing child at its rank in the schema's sequence rather than appending it.

### 4 · The measures are named, and the wire unit is rarely the surface unit

| Type | Surface | Wire |
|---|---|---|
| [`mjx_dml::Emu`](crate::Emu), [`mjx_dml::Position`](crate::Position), [`mjx_dml::Size`](crate::Size) | EMU or points | EMU (`914400` per inch, `12700` per point) |
| [`mjx_dml::LineWidth`](crate::LineWidth) | EMU or points | EMU |
| [`mjx_dml::Angle`](crate::Angle) | **radians** or degrees | 60000ths of a degree |
| [`mjx_dml::FontSize`](crate::FontSize), [`mjx_dml::TextPoint`](crate::TextPoint) | points | hundredths of a point |
| [`mjx_dml::Fraction`](crate::Fraction) | `1.0` is 100% | `100000` or `100%`, both accepted on read |
| [`mjx_dml::IndentLevel`](crate::IndentLevel) | `0..=8` | `0..=8`; an out-of-range wire value is rejected |

The ranges the schema states are documented rather than enforced, deliberately: a file may carry an
out-of-range value and reading one must not fail. They are all in
`crates/mjx-dml/src/geometry/measures.rs`.

## What this crate does not do

It does not open a package, and it has no idea what a part is — `mjx-opc` is a *lower* rank than this
crate and models the container, not the markup. It does not render:
[`mjx_dml::resolve_color`](crate::resolve_color) and the guide-formula evaluator answer *what does
this value actually come to*, which is the arithmetic a renderer needs and not the drawing. And it
types no payload belonging to a crate above it, which is why a chart's `c:chart` and a Word shape's
text box both stay raw here. [Reaching the shared types](reaching_the_shared_types) has the full list.
