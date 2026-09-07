# Geometry and placement

Two questions, two elements. *Where is it?* is `a:xfrm`. *What shape is it?* is `a:prstGeom` or
`a:custGeom`. Both hang off the same `spPr` as the fill and the outline.

## Where: `a:xfrm`

[`mjx_dml::Transform2D`](crate::Transform2D) is a plain struct of seven `Option`s, not a fidelity
wrapper — it is a *value* read out of an element and written back onto one:

```rust,ignore
pub struct Transform2D {
    pub position: Option<Position>,        // a:off
    pub size: Option<Size>,                // a:ext
    pub rotation: Option<Angle>,           // @rot,   clockwise about the centre
    pub flip_horizontal: Option<bool>,     // @flipH
    pub flip_vertical: Option<bool>,       // @flipV
    pub child_position: Option<Position>,  // a:chOff — groups only
    pub child_size: Option<Size>,          // a:chExt — groups only
}
```

`None` everywhere means *the file does not say*, and that is never an error:
[`mjx_dml::Transform2D::read`](crate::Transform2D::read) cannot fail, because an attribute this model
cannot parse must still leave the file readable. A shape with no `a:xfrm` at all is not broken either
— it inherits one from a placeholder or from the group above it.

[`mjx_dml::Transform2D::apply`](crate::Transform2D::apply) is the write side, and it is the merge verb
rather than the build verb: it edits `a:off`/`a:ext`/`a:chOff`/`a:chExt` **in place** when they are
there, keeping any attribute this type does not itself set, and inserts a missing one at its rank in
the schema's sequence (`off` → `ext` → `chOff` → `chExt`). Applying to a freshly built empty `a:xfrm`
is how a shape that had no transform gets one.

The last two fields are the group story: a group maps its members' coordinate space (`chOff`/`chExt`)
onto its own (`off`/`ext`), which is what lets a whole group move without touching a single member.

## What shape: the preset catalogue

```xml
<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>
```

That is the whole of a preset shape: a token and a list of adjustment overrides. There are **two
layers over it**, and picking the right one is most of what this section is for.

### The fidelity layer — round-trips anything

[`mjx_dml::PresetGeometry`](crate::PresetGeometry) (`a:prstGeom`),
[`mjx_dml::GeometryGuideList`](crate::GeometryGuideList) (`a:avLst`) and
[`mjx_dml::GeometryGuide`](crate::GeometryGuide) (`a:gd`). `@prst` is declared as text rather than as
an enumeration **deliberately**: a token from a future version of the format still round-trips, and
[`mjx_dml::PresetGeometry::preset`](crate::PresetGeometry::preset) layers the typed reading on top by
calling `PresetShapeType::from_wire` and answering `None` when it does not know the token.
[`mjx_dml::PresetGeometry::adjustment`](crate::PresetGeometry::adjustment) and
[`set_adjustment`](crate::PresetGeometry::set_adjustment) address a guide by its **wire name**
(`adj`, `adj1`, `adj2`), which works for every one of the 187 tokens whether or not anything has
named it.

### The named layer — meaningful parameters

[`mjx_dml::ShapeGeometry`](crate::ShapeGeometry) is the same information with the cryptic part gone:

```rust,ignore
ShapeGeometry::RoundedRectangle { corner_radius: Fraction }   // roundRect, adj
ShapeGeometry::SnipSingleCornerRectangle { snip_size }        // snip1Rect, adj
ShapeGeometry::FourPointStar { inner_radius }                 // star4, adj
```

[`mjx_dml::PresetGeometry::shape`](crate::PresetGeometry::shape) reads one and
[`set_shape`](crate::PresetGeometry::set_shape) writes one. The type is **total**: a shape nobody has
named yet reads as `ShapeGeometry::Unmodeled(PresetShapeType)`, so the enum never loses a shape and
the mechanical API above still reaches its guides.

The numeric domain each named parameter is clamped to is not written down here either — it is
computed. [`mjx_dml::PresetGeometry::adjustments_for_size`](crate::PresetGeometry::adjustments_for_size)
evaluates the shape's own bound guides out of `presetShapeDefinitions.xml` through the formula
evaluator, so the range comes from the specification rather than from a table someone typed.

### 187 tokens, 186 shapes

`ST_ShapeType` declares **187** values, and `PresetShapeType` has 187 variants. ECMA-376's
`presetShapeDefinitions.xml` has 187 blocks but only **186 distinct shapes**: `upDownArrow` is defined
twice, byte-for-byte identically, and `upArrow` has **no geometry block at all** even though it is a
legal token. Both anomalies are recorded in `docs/DRAWINGML_PRESET_SHAPES.md`, and every earlier
"187 shapes" in this project's tracker is the enumeration count being mistaken for the geometry count.

Two tokens are deliberately **not** named: `teardrop` and `sun` both read as
`ShapeGeometry::Unmodeled` and always have. Each is single-adjustment and would look easy, and each
was deferred in Phase A as spec-ambiguous — the ECMA prose does not pin what the adjustment means
well enough to name it, and naming a parameter wrongly is worse than leaving it mechanical.
`crates/mjx-dml/tests/shape_model.rs`'s `unported_shape_is_unmodeled_and_unknown_prst_is_none` is what
keeps that honest, and both shapes still round-trip byte for byte through the fidelity layer.

## Custom geometry, and the language its coordinates are written in

[`mjx_dml::CustomGeometry`](crate::CustomGeometry) (`a:custGeom`) is a guide list, an adjust-handle
list, a connection-site list and a [`mjx_dml::Path2DList`](crate::Path2DList) of drawing commands
([`mjx_dml::DrawCommand`](crate::DrawCommand): move, line, arc, cubic and quadratic Bézier, close).

A coordinate in one of those may be a *formula* rather than a number:

```xml
<a:gd name="x1" fmla="*/ w adj1 100000"/>
```

[`mjx_dml::geometry::formula`](crate::geometry::formula) is the evaluator —
[`mjx_dml::GuideOperator`](crate::GuideOperator) is the closed set of **seventeen** operators, each
carrying its wire token and semantics quoted from ECMA-376 Part 1 §20.1.9.11 rather than inferred
from the token's spelling. Evaluation is in declaration order, which makes a cyclic guide list
*impossible* rather than merely detectable. Angles are in 60000ths of a degree inside a formula, EMU
for lengths, and [`mjx_dml::GuideError`](crate::GuideError) is a typed error rather than a panic
because an `a:gdLst` in an untrusted file is untrusted input.

[`mjx_dml::CustomGeometrySpec::resolve`](crate::CustomGeometrySpec::resolve) turns a whole geometry
into a [`mjx_dml::ResolvedCustomGeometry`](crate::ResolvedCustomGeometry) — every point an
[`Emu`](crate::Emu), every arc angle an [`Angle`](crate::Angle) — given a
[`mjx_dml::GuideContext`](crate::GuideContext) of the shape's width and height. That is the arithmetic
a renderer needs; the drawing itself is nothing to do with this crate.

## Placing the whole thing on a page

The transform says where a shape is *inside its host*. What pins the host is the host's own wrapper,
and two of those live here:

* **Word** — [`mjx_dml::wordprocessing_drawing::Inline`](crate::wordprocessing_drawing::Inline) sits in
  the text flow; [`Anchor`](crate::wordprocessing_drawing::Anchor) floats, with a
  [`HorizontalPosition`](crate::wordprocessing_drawing::HorizontalPosition) /
  [`VerticalPosition`](crate::wordprocessing_drawing::VerticalPosition) pair and a
  [`Wrap`](crate::wordprocessing_drawing::Wrap) mode.
* **Excel** — [`mjx_dml::Anchor`](crate::Anchor) is the three-way choice
  ([`TwoCellAnchor`](crate::TwoCellAnchor), [`OneCellAnchor`](crate::OneCellAnchor),
  [`AbsoluteAnchor`](crate::AbsoluteAnchor)), and the **element decides the resizing behaviour**: a
  one-cell and an absolute anchor carry no `@editAs` at all, so
  [`mjx_dml::Anchor::resizing_behavior`](crate::Anchor::resizing_behavior) answers from the element
  kind for two of the three and from the attribute for the third.
  [`mjx_dml::WorksheetDrawing::insert_rows`](crate::WorksheetDrawing::insert_rows) and its three
  siblings return a [`mjx_dml::AnchorShift`](crate::AnchorShift) per anchor — a *report* of what the
  markers now hold, not a confirmation of what was asked for.
