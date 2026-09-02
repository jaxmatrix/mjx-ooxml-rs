//! Where a shape is, how big it is, and what outline it draws.
//!
//! # `ShapeGeometry`, and why it is one class rather than 117 constructors
//!
//! DrawingML's preset shapes each carry their own named adjustments: a rounded rectangle has a
//! `corner_radius`, an arc has a `start_angle` and an `end_angle`, a callout has four vertices. The
//! Rust enumeration spells all 117 out, because Rust can pattern-match them.
//!
//! Projecting that as 117 static constructors would give TypeScript 117 methods whose parameter
//! names are the adjustment names anyway — the same strings, spread over 117 signatures, with no way
//! to ask what a shape's adjustments are called. So the projection keeps the *table* instead:
//! `ShapeGeometry.of` takes the preset and a record from adjustment name to value,
//! `ShapeGeometry.adjustments` hands the same record back, and `ShapeGeometry.adjustmentNames`
//! says what a given preset's adjustments are called. Nothing is lost: every variant is constructible and every variant is readable, and the
//! one generated table below is the whole of it.
//!
//! The **values keep their units**: an adjustment is an [`Fraction`] or an [`Angle`], never a bare
//! number, and handing a shape an angle where it wanted a proportion raises rather than silently
//! writing sixty times the wrong value.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::support::invalid_argument;

use crate::enums::{AdjustmentAxis, PathFillMode, PresetShapeType, SlideSizeKind};
use crate::measures::{Angle, Emu, Fraction};

value_class! {
    /// A shape's rectangle on its surface, in English Metric Units.
    ShapeBounds(ooxml::ShapeBounds), derive(Copy, PartialEq, Eq);

    /// The slide size: the extent in EMU, plus the paper or screen kind it names.
    SlideSize(ooxml::SlideSize), derive(Copy, PartialEq, Eq);

    /// A table cell's four inner margins.
    CellMargins(ooxml::CellMargins), derive(Copy, PartialEq, Eq);

    /// A point in EMU — the offset half of a transform.
    Position(ooxml::Position), derive(Copy, PartialEq, Eq);

    /// An extent in EMU — the size half of a transform.
    Size(ooxml::Size), derive(Copy, PartialEq, Eq);

    /// A shape's full `a:xfrm`: offset, extent, rotation, flips, and the child space a group maps
    /// its members through.
    Transform2D(ooxml::Transform2D), derive(PartialEq);

    /// A point in a custom geometry path, whose coordinates may be literal or guide-relative.
    Point(ooxml::Point), derive(PartialEq, Eq);

    /// A coordinate in a custom geometry: an absolute length, or the name of a guide.
    AdjustCoordinate(ooxml::AdjustCoordinate), derive(PartialEq, Eq);

    /// An angle in a custom geometry: an absolute angle, or the name of a guide.
    AdjustAngle(ooxml::AdjustAngle), derive(PartialEq);

    /// One guide: a name, and the formula that computes it.
    GuideSpec(ooxml::GuideSpec), derive(PartialEq, Eq);

    /// The width and height a guide formula's `w` and `h` variables stand for.
    GuideContext(ooxml::GuideContext), derive(Copy, PartialEq);

    /// A rectangle in a custom geometry, whose edges may be guide-relative.
    Rectangle(ooxml::Rectangle), derive(PartialEq, Eq);

    /// One command in a custom geometry path.
    DrawCommand(ooxml::DrawCommand), derive(PartialEq);

    /// One path of a custom geometry: its own coordinate space, and the commands that draw it.
    Path2DSpec(ooxml::Path2DSpec), derive(PartialEq);

    /// A draggable handle on a custom geometry, and the guide it drives.
    AdjustHandle(ooxml::AdjustHandle), derive(PartialEq);

    /// A point a connector can attach to, and the direction a connector leaves it in.
    ConnectionSite(ooxml::ConnectionSite), derive(PartialEq);

    /// A whole `a:custGeom`: guides, handles, connection sites, text rectangle and paths.
    CustomGeometrySpec(ooxml::CustomGeometrySpec), derive(PartialEq);

    /// A preset shape with its named adjustments, or an unmodelled preset carrying only its name.
    ShapeGeometry(ooxml::ShapeGeometry), derive(PartialEq);

    /// What draws a shape's outline: a preset, a custom path, or whatever it inherits.
    Geometry(ooxml::Geometry), derive(PartialEq);

    /// One adjustment of a preset shape, with the range it is allowed to move in.
    BoundedAdjustment(ooxml::BoundedAdjustment), derive(Copy, PartialEq);

    /// What the specification says about one adjustment of one preset shape.
    AdjustmentSpec(AdjustmentSpecRef), derive(Copy, PartialEq, Eq);

    /// One end of an adjustment's range: a literal value, or the name of a guide.
    AdjustmentBound(ooxml::AdjustmentBound), derive(Copy, PartialEq, Eq);

    /// A point with every coordinate resolved to EMU.
    ResolvedPoint(ooxml::ResolvedPoint), derive(Copy, PartialEq, Eq);

    /// A rectangle with every edge resolved to EMU.
    ResolvedRectangle(ooxml::ResolvedRectangle), derive(Copy, PartialEq, Eq);

    /// A path command with every coordinate resolved.
    ResolvedDrawCommand(ooxml::ResolvedDrawCommand), derive(Copy, PartialEq);

    /// A path with every command resolved.
    ResolvedPath(ooxml::ResolvedPath), derive(PartialEq);

    /// A connection site with its point and angle resolved.
    ResolvedConnectionSite(ooxml::ResolvedConnectionSite), derive(Copy, PartialEq);

    /// An adjust handle with its point and limits resolved.
    ResolvedAdjustHandle(ooxml::ResolvedAdjustHandle), derive(PartialEq);

    /// A custom geometry with every formula evaluated — what a renderer would draw.
    ResolvedCustomGeometry(ooxml::ResolvedCustomGeometry), derive(PartialEq);
}

/// The `&'static AdjustmentSpec` a [`BoundedAdjustment`] carries, as a value this crate can own.
///
/// The reference points into the generated preset tables, which live for the whole program, so
/// carrying it by value is free and the class needs no lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustmentSpecRef(&'static ooxml::AdjustmentSpec);

// ---------------------------------------------------------------------------------------------
// Bounds, sizes and transforms
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl ShapeBounds {
    /// A rectangle stated directly in English Metric Units.
    #[wasm_bindgen(constructor)]
    pub fn new(offset_x_emu: i64, offset_y_emu: i64, width_emu: i64, height_emu: i64) -> Self {
        Self(ooxml::ShapeBounds::new(
            offset_x_emu,
            offset_y_emu,
            width_emu,
            height_emu,
        ))
    }

    /// A rectangle in inches — the unit slide layouts are usually reasoned about in.
    #[wasm_bindgen(js_name = "fromInches")]
    pub fn from_inches(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self(ooxml::ShapeBounds::from_inches(x, y, width, height))
    }

    /// The rectangle that contains both this one and `other`.
    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.union(other.0))
    }

    /// The bounds a transform states, when it states both an offset and an extent.
    #[wasm_bindgen(js_name = "fromTransform")]
    pub fn from_transform(transform: &Transform2D) -> Option<Self> {
        ooxml::ShapeBounds::from_transform(&transform.0).map(Self)
    }

    /// This rectangle as a transform.
    #[wasm_bindgen(js_name = "toTransform")]
    pub fn to_transform(&self) -> Transform2D {
        Transform2D(self.0.to_transform())
    }

    /// The left edge, in EMU.
    #[wasm_bindgen(getter, js_name = "offsetXEmu")]
    pub fn offset_x_emu(&self) -> i64 {
        self.0.offset_x_emu
    }

    /// The top edge, in EMU.
    #[wasm_bindgen(getter, js_name = "offsetYEmu")]
    pub fn offset_y_emu(&self) -> i64 {
        self.0.offset_y_emu
    }

    /// The width, in EMU.
    #[wasm_bindgen(getter, js_name = "widthEmu")]
    pub fn width_emu(&self) -> i64 {
        self.0.width_emu
    }

    /// The height, in EMU.
    #[wasm_bindgen(getter, js_name = "heightEmu")]
    pub fn height_emu(&self) -> i64 {
        self.0.height_emu
    }
}

#[wasm_bindgen]
impl SlideSize {
    /// 13⅓ by 7½ inches — the 16∶9 size PowerPoint has defaulted to since 2013.
    pub fn widescreen() -> Self {
        Self(ooxml::SlideSize::widescreen())
    }

    /// 10 by 7½ inches — the older 4∶3 size.
    pub fn standard() -> Self {
        Self(ooxml::SlideSize::standard())
    }

    /// A custom size in English Metric Units.
    #[wasm_bindgen(js_name = "fromEmu")]
    pub fn from_emu(width_emu: i64, height_emu: i64) -> Self {
        Self(ooxml::SlideSize::from_emu(width_emu, height_emu))
    }

    /// The width, in EMU.
    #[wasm_bindgen(getter, js_name = "widthEmu")]
    pub fn width_emu(&self) -> i64 {
        self.0.width_emu
    }

    /// The height, in EMU.
    #[wasm_bindgen(getter, js_name = "heightEmu")]
    pub fn height_emu(&self) -> i64 {
        self.0.height_emu
    }

    /// The paper or screen kind `p:sldSz@type` names.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<SlideSizeKind, JsValue> {
        SlideSizeKind::from_model(self.0.kind)
    }
}

#[wasm_bindgen]
impl CellMargins {
    /// Four margins, each optional; an unstated one is inherited.
    #[wasm_bindgen(constructor)]
    pub fn new(
        left: Option<Emu>,
        right: Option<Emu>,
        top: Option<Emu>,
        bottom: Option<Emu>,
    ) -> Self {
        Self(ooxml::CellMargins {
            left: left.map(|value| value.0),
            right: right.map(|value| value.0),
            top: top.map(|value| value.0),
            bottom: bottom.map(|value| value.0),
        })
    }

    /// The same margin on all four sides.
    pub fn uniform(margin: &Emu) -> Self {
        Self(ooxml::CellMargins::uniform(margin.0))
    }

    /// The left margin, when stated.
    #[wasm_bindgen(getter, js_name = "left")]
    pub fn left(&self) -> Option<Emu> {
        self.0.left.map(Emu)
    }

    /// The right margin, when stated.
    #[wasm_bindgen(getter, js_name = "right")]
    pub fn right(&self) -> Option<Emu> {
        self.0.right.map(Emu)
    }

    /// The top margin, when stated.
    #[wasm_bindgen(getter, js_name = "top")]
    pub fn top(&self) -> Option<Emu> {
        self.0.top.map(Emu)
    }

    /// The bottom margin, when stated.
    #[wasm_bindgen(getter, js_name = "bottom")]
    pub fn bottom(&self) -> Option<Emu> {
        self.0.bottom.map(Emu)
    }
}

#[wasm_bindgen]
impl Position {
    /// A point in English Metric Units.
    #[wasm_bindgen(constructor)]
    pub fn new(x: &Emu, y: &Emu) -> Self {
        Self(ooxml::Position { x: x.0, y: y.0 })
    }

    /// A point given as two raw EMU values.
    #[wasm_bindgen(js_name = "fromEmu")]
    pub fn from_emu(x: i64, y: i64) -> Self {
        Self(ooxml::Position::from_emu(x, y))
    }

    /// The horizontal coordinate.
    #[wasm_bindgen(getter, js_name = "x")]
    pub fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical coordinate.
    #[wasm_bindgen(getter, js_name = "y")]
    pub fn y(&self) -> Emu {
        Emu(self.0.y)
    }
}

#[wasm_bindgen]
impl Size {
    /// An extent in English Metric Units.
    #[wasm_bindgen(constructor)]
    pub fn new(width: &Emu, height: &Emu) -> Self {
        Self(ooxml::Size {
            width: width.0,
            height: height.0,
        })
    }

    /// An extent given as two raw EMU values.
    #[wasm_bindgen(js_name = "fromEmu")]
    pub fn from_emu(width: i64, height: i64) -> Self {
        Self(ooxml::Size::from_emu(width, height))
    }

    /// The width.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Emu {
        Emu(self.0.width)
    }

    /// The height.
    #[wasm_bindgen(getter, js_name = "height")]
    pub fn height(&self) -> Emu {
        Emu(self.0.height)
    }
}

#[wasm_bindgen]
impl Transform2D {
    /// A transform. Every part is optional, and an unstated one is inherited.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        position: Option<Position>,
        size: Option<Size>,
        rotation: Option<Angle>,
        flip_horizontal: Option<bool>,
        flip_vertical: Option<bool>,
        child_position: Option<Position>,
        child_size: Option<Size>,
    ) -> Self {
        Self(ooxml::Transform2D {
            position: position.map(|value| value.0),
            size: size.map(|value| value.0),
            rotation: rotation.map(|value| value.0),
            flip_horizontal,
            flip_vertical,
            child_position: child_position.map(|value| value.0),
            child_size: child_size.map(|value| value.0),
        })
    }

    /// The offset, when stated.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Option<Position> {
        self.0.position.map(Position)
    }

    /// The extent, when stated.
    #[wasm_bindgen(getter, js_name = "size")]
    pub fn size(&self) -> Option<Size> {
        self.0.size.map(Size)
    }

    /// The rotation, when stated.
    #[wasm_bindgen(getter, js_name = "rotation")]
    pub fn rotation(&self) -> Option<Angle> {
        self.0.rotation.map(Angle)
    }

    /// Whether the shape is mirrored horizontally, when stated.
    #[wasm_bindgen(getter, js_name = "flipHorizontal")]
    pub fn flip_horizontal(&self) -> Option<bool> {
        self.0.flip_horizontal
    }

    /// Whether the shape is mirrored vertically, when stated.
    #[wasm_bindgen(getter, js_name = "flipVertical")]
    pub fn flip_vertical(&self) -> Option<bool> {
        self.0.flip_vertical
    }

    /// A group's child-space offset, when stated.
    #[wasm_bindgen(getter, js_name = "childPosition")]
    pub fn child_position(&self) -> Option<Position> {
        self.0.child_position.map(Position)
    }

    /// A group's child-space extent, when stated.
    #[wasm_bindgen(getter, js_name = "childSize")]
    pub fn child_size(&self) -> Option<Size> {
        self.0.child_size.map(Size)
    }

    /// Whether this transform states nothing at all.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The horizontal scale a group applies to its members, when it states both extents.
    #[wasm_bindgen(getter, js_name = "childScaleX")]
    pub fn child_scale_x(&self) -> Option<f64> {
        self.0.child_scale().map(|(x, _)| x)
    }

    /// The vertical scale a group applies to its members, when it states both extents.
    #[wasm_bindgen(getter, js_name = "childScaleY")]
    pub fn child_scale_y(&self) -> Option<f64> {
        self.0.child_scale().map(|(_, y)| y)
    }

    /// A point in a group's child space, mapped to the surface.
    #[wasm_bindgen(js_name = "childToParent")]
    pub fn child_to_parent(&self, point: &Position) -> Option<Position> {
        self.0.child_to_parent(point.0).map(Position)
    }

    /// A point on the surface, mapped into a group's child space.
    #[wasm_bindgen(js_name = "parentToChild")]
    pub fn parent_to_child(&self, point: &Position) -> Option<Position> {
        self.0.parent_to_child(point.0).map(Position)
    }
}

// ---------------------------------------------------------------------------------------------
// Custom geometry
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl AdjustCoordinate {
    /// A literal length.
    pub fn emu(value: &Emu) -> Self {
        Self(ooxml::AdjustCoordinate::Emu(value.0))
    }

    /// The value of a named guide.
    pub fn guide(name: &str) -> Self {
        Self(ooxml::AdjustCoordinate::Guide(name.to_owned()))
    }

    /// A coordinate parsed from the wire form — a number, or a guide name.
    #[wasm_bindgen(js_name = "fromWire")]
    pub fn from_wire(value: &str) -> Self {
        Self(ooxml::AdjustCoordinate::from_wire(value))
    }

    /// The literal length, when this is one.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::AdjustCoordinate::Emu(value) => Some(Emu(*value)),
            ooxml::AdjustCoordinate::Guide(_) => None,
        }
    }

    /// The guide's name, when this names one.
    #[wasm_bindgen(getter, js_name = "guideName")]
    pub fn guide_name(&self) -> Option<String> {
        match &self.0 {
            ooxml::AdjustCoordinate::Guide(name) => Some(name.clone()),
            ooxml::AdjustCoordinate::Emu(_) => None,
        }
    }

    /// The wire form, exactly as it is written.
    #[wasm_bindgen(js_name = "toWire")]
    pub fn to_wire(&self) -> String {
        self.0.to_wire().to_owned()
    }
}

#[wasm_bindgen]
impl AdjustAngle {
    /// A literal angle.
    pub fn angle(value: &Angle) -> Self {
        Self(ooxml::AdjustAngle::Angle(value.0))
    }

    /// The value of a named guide.
    pub fn guide(name: &str) -> Self {
        Self(ooxml::AdjustAngle::Guide(name.to_owned()))
    }

    /// An angle parsed from the wire form — a number of sixtieths of a degree, or a guide name.
    #[wasm_bindgen(js_name = "fromWire")]
    pub fn from_wire(value: &str) -> Self {
        Self(ooxml::AdjustAngle::from_wire(value))
    }

    /// The literal angle, when this is one.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::AdjustAngle::Angle(value) => Some(Angle(*value)),
            ooxml::AdjustAngle::Guide(_) => None,
        }
    }

    /// The guide's name, when this names one.
    #[wasm_bindgen(getter, js_name = "guideName")]
    pub fn guide_name(&self) -> Option<String> {
        match &self.0 {
            ooxml::AdjustAngle::Guide(name) => Some(name.clone()),
            ooxml::AdjustAngle::Angle(_) => None,
        }
    }

    /// The wire form, exactly as it is written.
    #[wasm_bindgen(js_name = "toWire")]
    pub fn to_wire(&self) -> String {
        self.0.to_wire().to_owned()
    }
}

#[wasm_bindgen]
impl Point {
    /// A point whose coordinates may each be literal or guide-relative.
    #[wasm_bindgen(constructor)]
    pub fn new(x: &AdjustCoordinate, y: &AdjustCoordinate) -> Self {
        Self(ooxml::Point {
            x: x.0.clone(),
            y: y.0.clone(),
        })
    }

    /// A point at two literal EMU coordinates.
    #[wasm_bindgen(js_name = "fromEmu")]
    pub fn from_emu(x: i64, y: i64) -> Self {
        Self(ooxml::Point::from_emu(x, y))
    }

    /// The horizontal coordinate.
    #[wasm_bindgen(getter, js_name = "x")]
    pub fn x(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.x.clone())
    }

    /// The vertical coordinate.
    #[wasm_bindgen(getter, js_name = "y")]
    pub fn y(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.y.clone())
    }
}

#[wasm_bindgen]
impl GuideSpec {
    /// A guide: the name other formulas refer to it by, and the formula that computes it.
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, formula: &str) -> Self {
        Self(ooxml::GuideSpec {
            name: name.to_owned(),
            formula: formula.to_owned(),
        })
    }

    /// The guide's name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// The formula, in the seventeen-operator prefix language `a:gd@fmla` uses.
    #[wasm_bindgen(getter, js_name = "formula")]
    pub fn formula(&self) -> String {
        self.0.formula.clone()
    }
}

#[wasm_bindgen]
impl GuideContext {
    /// The extent a formula's `w` and `h` stand for.
    #[wasm_bindgen(js_name = "fromExtents")]
    pub fn from_extents(width: &Emu, height: &Emu) -> Self {
        Self(ooxml::GuideContext::from_extents(width.0, height.0))
    }

    /// The same, from a `Size`.
    #[wasm_bindgen(js_name = "fromSize")]
    pub fn from_size(size: &Size) -> Self {
        Self(ooxml::GuideContext::from_size(size.0))
    }

    /// The width `w` stands for.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Emu {
        Emu(self.0.width())
    }

    /// The height `h` stands for.
    #[wasm_bindgen(getter, js_name = "height")]
    pub fn height(&self) -> Emu {
        Emu(self.0.height())
    }

    /// The value of one built-in variable — `w`, `h`, `l`, `t`, `r`, `b`, `hc`, `vc`, `ss`, `ls`,
    /// `ssd2`… — or `None` if that is not a variable name.
    pub fn variable(&self, name: &str) -> Option<f64> {
        self.0.variable(name)
    }
}

#[wasm_bindgen]
impl Rectangle {
    /// A rectangle whose four edges may each be literal or guide-relative.
    #[wasm_bindgen(constructor)]
    pub fn new(
        left: &AdjustCoordinate,
        top: &AdjustCoordinate,
        right: &AdjustCoordinate,
        bottom: &AdjustCoordinate,
    ) -> Self {
        Self(ooxml::Rectangle {
            left: left.0.clone(),
            top: top.0.clone(),
            right: right.0.clone(),
            bottom: bottom.0.clone(),
        })
    }

    /// The left edge.
    #[wasm_bindgen(getter, js_name = "left")]
    pub fn left(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.left.clone())
    }

    /// The top edge.
    #[wasm_bindgen(getter, js_name = "top")]
    pub fn top(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.top.clone())
    }

    /// The right edge.
    #[wasm_bindgen(getter, js_name = "right")]
    pub fn right(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.right.clone())
    }

    /// The bottom edge.
    #[wasm_bindgen(getter, js_name = "bottom")]
    pub fn bottom(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.bottom.clone())
    }
}

#[wasm_bindgen]
impl DrawCommand {
    /// Close the current subpath.
    pub fn close() -> Self {
        Self(ooxml::DrawCommand::Close)
    }

    /// Start a new subpath at a point.
    #[wasm_bindgen(js_name = "moveTo")]
    pub fn move_to(point: &Point) -> Self {
        Self(ooxml::DrawCommand::MoveTo(point.0.clone()))
    }

    /// Draw a straight segment to a point.
    #[wasm_bindgen(js_name = "lineTo")]
    pub fn line_to(point: &Point) -> Self {
        Self(ooxml::DrawCommand::LineTo(point.0.clone()))
    }

    /// Draw an elliptical arc.
    #[wasm_bindgen(js_name = "arcTo")]
    pub fn arc_to(
        width_radius: &AdjustCoordinate,
        height_radius: &AdjustCoordinate,
        start_angle: &AdjustAngle,
        swing_angle: &AdjustAngle,
    ) -> Self {
        Self(ooxml::DrawCommand::ArcTo {
            width_radius: width_radius.0.clone(),
            height_radius: height_radius.0.clone(),
            start_angle: start_angle.0.clone(),
            swing_angle: swing_angle.0.clone(),
        })
    }

    /// Draw a quadratic Bézier through one control point.
    #[wasm_bindgen(js_name = "quadBezierTo")]
    pub fn quad_bezier_to(control: &Point, end: &Point) -> Self {
        Self(ooxml::DrawCommand::QuadBezierTo(
            control.0.clone(),
            end.0.clone(),
        ))
    }

    /// Draw a cubic Bézier through two control points.
    #[wasm_bindgen(js_name = "cubicBezierTo")]
    pub fn cubic_bezier_to(first: &Point, second: &Point, end: &Point) -> Self {
        Self(ooxml::DrawCommand::CubicBezierTo(
            first.0.clone(),
            second.0.clone(),
            end.0.clone(),
        ))
    }

    /// Which command this is: `"close"`, `"move_to"`, `"line_to"`, `"arc_to"`, `"quad_bezier_to"`
    /// or `"cubic_bezier_to"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::DrawCommand::Close => "close".to_owned(),
            ooxml::DrawCommand::MoveTo(_) => "move_to".to_owned(),
            ooxml::DrawCommand::LineTo(_) => "line_to".to_owned(),
            ooxml::DrawCommand::ArcTo { .. } => "arc_to".to_owned(),
            ooxml::DrawCommand::QuadBezierTo(..) => "quad_bezier_to".to_owned(),
            ooxml::DrawCommand::CubicBezierTo(..) => "cubic_bezier_to".to_owned(),
        }
    }

    /// The points this command names, in order; empty for `close` and for `arc_to`.
    #[wasm_bindgen(getter, js_name = "points")]
    pub fn points(&self) -> Vec<Point> {
        match &self.0 {
            ooxml::DrawCommand::MoveTo(point) | ooxml::DrawCommand::LineTo(point) => {
                vec![Point(point.clone())]
            }
            ooxml::DrawCommand::QuadBezierTo(control, end) => {
                vec![Point(control.clone()), Point(end.clone())]
            }
            ooxml::DrawCommand::CubicBezierTo(first, second, end) => vec![
                Point(first.clone()),
                Point(second.clone()),
                Point(end.clone()),
            ],
            _ => Vec::new(),
        }
    }

    /// The arc's horizontal radius, when this is an arc.
    #[wasm_bindgen(getter, js_name = "widthRadius")]
    pub fn width_radius(&self) -> Option<AdjustCoordinate> {
        match &self.0 {
            ooxml::DrawCommand::ArcTo { width_radius, .. } => {
                Some(AdjustCoordinate(width_radius.clone()))
            }
            _ => None,
        }
    }

    /// The arc's vertical radius, when this is an arc.
    #[wasm_bindgen(getter, js_name = "heightRadius")]
    pub fn height_radius(&self) -> Option<AdjustCoordinate> {
        match &self.0 {
            ooxml::DrawCommand::ArcTo { height_radius, .. } => {
                Some(AdjustCoordinate(height_radius.clone()))
            }
            _ => None,
        }
    }

    /// The arc's start angle, when this is an arc.
    #[wasm_bindgen(getter, js_name = "startAngle")]
    pub fn start_angle(&self) -> Option<AdjustAngle> {
        match &self.0 {
            ooxml::DrawCommand::ArcTo { start_angle, .. } => Some(AdjustAngle(start_angle.clone())),
            _ => None,
        }
    }

    /// The arc's swing angle, when this is an arc.
    #[wasm_bindgen(getter, js_name = "swingAngle")]
    pub fn swing_angle(&self) -> Option<AdjustAngle> {
        match &self.0 {
            ooxml::DrawCommand::ArcTo { swing_angle, .. } => Some(AdjustAngle(swing_angle.clone())),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl Path2DSpec {
    /// One path of a custom geometry.
    #[wasm_bindgen(constructor)]
    pub fn new(
        commands: Vec<DrawCommand>,
        width: Option<Emu>,
        height: Option<Emu>,
        fill: Option<PathFillMode>,
        stroke: Option<bool>,
        extrusion_ok: Option<bool>,
    ) -> Self {
        Self(ooxml::Path2DSpec {
            width: width.map(|value| value.0),
            height: height.map(|value| value.0),
            fill: fill.map(Into::into),
            stroke,
            extrusion_ok,
            commands: commands.into_iter().map(|command| command.0).collect(),
        })
    }

    /// The path's own coordinate width, when stated.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Option<Emu> {
        self.0.width.map(Emu)
    }

    /// The path's own coordinate height, when stated.
    #[wasm_bindgen(getter, js_name = "height")]
    pub fn height(&self) -> Option<Emu> {
        self.0.height.map(Emu)
    }

    /// How the path is filled, when stated.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Result<Option<PathFillMode>, JsValue> {
        self.0.fill.map(PathFillMode::from_model).transpose()
    }

    /// Whether the path is stroked, when stated.
    #[wasm_bindgen(getter, js_name = "stroke")]
    pub fn stroke(&self) -> Option<bool> {
        self.0.stroke
    }

    /// Whether the path may be extruded in 3-D, when stated.
    #[wasm_bindgen(getter, js_name = "extrusionOk")]
    pub fn extrusion_ok(&self) -> Option<bool> {
        self.0.extrusion_ok
    }

    /// The commands that draw the path, in order.
    #[wasm_bindgen(getter, js_name = "commands")]
    pub fn commands(&self) -> Vec<DrawCommand> {
        self.0.commands.iter().cloned().map(DrawCommand).collect()
    }
}

#[wasm_bindgen]
impl ConnectionSite {
    /// A point a connector can attach to, and the direction it leaves in.
    #[wasm_bindgen(constructor)]
    pub fn new(angle: &AdjustAngle, position: &Point) -> Self {
        Self(ooxml::ConnectionSite {
            angle: angle.0.clone(),
            position: position.0.clone(),
        })
    }

    /// The direction a connector leaves the site in.
    #[wasm_bindgen(getter, js_name = "angle")]
    pub fn angle(&self) -> AdjustAngle {
        AdjustAngle(self.0.angle.clone())
    }

    /// Where the site is.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Point {
        Point(self.0.position.clone())
    }
}

#[wasm_bindgen]
impl AdjustHandle {
    /// A handle that moves in two dimensions, driving one guide per axis.
    #[allow(clippy::too_many_arguments)]
    pub fn xy(
        position: &Point,
        guide_ref_x: Option<String>,
        min_x: Option<AdjustCoordinate>,
        max_x: Option<AdjustCoordinate>,
        guide_ref_y: Option<String>,
        min_y: Option<AdjustCoordinate>,
        max_y: Option<AdjustCoordinate>,
    ) -> Self {
        Self(ooxml::AdjustHandle::Xy {
            position: position.0.clone(),
            guide_ref_x,
            min_x: min_x.map(|value| value.0),
            max_x: max_x.map(|value| value.0),
            guide_ref_y,
            min_y: min_y.map(|value| value.0),
            max_y: max_y.map(|value| value.0),
        })
    }

    /// A handle that moves in polar coordinates, driving a radius guide and an angle guide.
    #[allow(clippy::too_many_arguments)]
    pub fn polar(
        position: &Point,
        guide_ref_radius: Option<String>,
        min_radius: Option<AdjustCoordinate>,
        max_radius: Option<AdjustCoordinate>,
        guide_ref_angle: Option<String>,
        min_angle: Option<AdjustAngle>,
        max_angle: Option<AdjustAngle>,
    ) -> Self {
        Self(ooxml::AdjustHandle::Polar {
            position: position.0.clone(),
            guide_ref_radius,
            min_radius: min_radius.map(|value| value.0),
            max_radius: max_radius.map(|value| value.0),
            guide_ref_angle,
            min_angle: min_angle.map(|value| value.0),
            max_angle: max_angle.map(|value| value.0),
        })
    }

    /// Which kind this is: `"xy"` or `"polar"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::AdjustHandle::Xy { .. } => "xy".to_owned(),
            ooxml::AdjustHandle::Polar { .. } => "polar".to_owned(),
        }
    }

    /// Where the handle sits.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Point {
        match &self.0 {
            ooxml::AdjustHandle::Xy { position, .. }
            | ooxml::AdjustHandle::Polar { position, .. } => Point(position.clone()),
        }
    }

    /// The guide the first axis drives — `gdRefX` or `gdRefR` — when the handle names one.
    #[wasm_bindgen(getter, js_name = "firstGuide")]
    pub fn first_guide(&self) -> Option<String> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { guide_ref_x, .. } => guide_ref_x.clone(),
            ooxml::AdjustHandle::Polar {
                guide_ref_radius, ..
            } => guide_ref_radius.clone(),
        }
    }

    /// The guide the second axis drives — `gdRefY` or `gdRefAng` — when the handle names one.
    #[wasm_bindgen(getter, js_name = "secondGuide")]
    pub fn second_guide(&self) -> Option<String> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { guide_ref_y, .. } => guide_ref_y.clone(),
            ooxml::AdjustHandle::Polar {
                guide_ref_angle, ..
            } => guide_ref_angle.clone(),
        }
    }

    /// The lower limit of the first axis, when stated.
    #[wasm_bindgen(getter, js_name = "firstMinimum")]
    pub fn first_minimum(&self) -> Option<AdjustCoordinate> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { min_x, .. } => min_x.clone().map(AdjustCoordinate),
            ooxml::AdjustHandle::Polar { min_radius, .. } => {
                min_radius.clone().map(AdjustCoordinate)
            }
        }
    }

    /// The upper limit of the first axis, when stated.
    #[wasm_bindgen(getter, js_name = "firstMaximum")]
    pub fn first_maximum(&self) -> Option<AdjustCoordinate> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { max_x, .. } => max_x.clone().map(AdjustCoordinate),
            ooxml::AdjustHandle::Polar { max_radius, .. } => {
                max_radius.clone().map(AdjustCoordinate)
            }
        }
    }

    /// An `xy` handle's lower second-axis limit, when stated. A `polar` handle's second axis is an
    /// angle, reported by `secondAngleMinimum`.
    #[wasm_bindgen(getter, js_name = "secondMinimum")]
    pub fn second_minimum(&self) -> Option<AdjustCoordinate> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { min_y, .. } => min_y.clone().map(AdjustCoordinate),
            ooxml::AdjustHandle::Polar { .. } => None,
        }
    }

    /// An `xy` handle's upper second-axis limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondMaximum")]
    pub fn second_maximum(&self) -> Option<AdjustCoordinate> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { max_y, .. } => max_y.clone().map(AdjustCoordinate),
            ooxml::AdjustHandle::Polar { .. } => None,
        }
    }

    /// A `polar` handle's lower angular limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondAngleMinimum")]
    pub fn second_angle_minimum(&self) -> Option<AdjustAngle> {
        match &self.0 {
            ooxml::AdjustHandle::Polar { min_angle, .. } => min_angle.clone().map(AdjustAngle),
            ooxml::AdjustHandle::Xy { .. } => None,
        }
    }

    /// A `polar` handle's upper angular limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondAngleMaximum")]
    pub fn second_angle_maximum(&self) -> Option<AdjustAngle> {
        match &self.0 {
            ooxml::AdjustHandle::Polar { max_angle, .. } => max_angle.clone().map(AdjustAngle),
            ooxml::AdjustHandle::Xy { .. } => None,
        }
    }
}

#[wasm_bindgen]
impl CustomGeometrySpec {
    /// A custom geometry. Only `paths` is usually needed; the rest describe the guides and handles
    /// PowerPoint's own editor manipulates.
    #[wasm_bindgen(constructor)]
    pub fn new(
        paths: Vec<Path2DSpec>,
        adjust_values: Vec<GuideSpec>,
        guides: Vec<GuideSpec>,
        adjust_handles: Vec<AdjustHandle>,
        connection_sites: Vec<ConnectionSite>,
        text_rectangle: Option<Rectangle>,
    ) -> Self {
        Self(ooxml::CustomGeometrySpec {
            adjust_values: adjust_values.into_iter().map(|guide| guide.0).collect(),
            guides: guides.into_iter().map(|guide| guide.0).collect(),
            adjust_handles: adjust_handles.into_iter().map(|handle| handle.0).collect(),
            connection_sites: connection_sites.into_iter().map(|site| site.0).collect(),
            text_rectangle: text_rectangle.map(|rectangle| rectangle.0),
            paths: paths.into_iter().map(|path| path.0).collect(),
        })
    }

    /// The adjust values (`a:avLst`), each a named guide with a default.
    #[wasm_bindgen(getter, js_name = "adjustValues")]
    pub fn adjust_values(&self) -> Vec<GuideSpec> {
        self.0
            .adjust_values
            .iter()
            .cloned()
            .map(GuideSpec)
            .collect()
    }

    /// The computed guides (`a:gdLst`).
    #[wasm_bindgen(getter, js_name = "guides")]
    pub fn guides(&self) -> Vec<GuideSpec> {
        self.0.guides.iter().cloned().map(GuideSpec).collect()
    }

    /// The adjust handles (`a:ahLst`).
    #[wasm_bindgen(getter, js_name = "adjustHandles")]
    pub fn adjust_handles(&self) -> Vec<AdjustHandle> {
        self.0
            .adjust_handles
            .iter()
            .cloned()
            .map(AdjustHandle)
            .collect()
    }

    /// The connection sites (`a:cxnLst`).
    #[wasm_bindgen(getter, js_name = "connectionSites")]
    pub fn connection_sites(&self) -> Vec<ConnectionSite> {
        self.0
            .connection_sites
            .iter()
            .cloned()
            .map(ConnectionSite)
            .collect()
    }

    /// The text rectangle (`a:rect`), when stated.
    #[wasm_bindgen(getter, js_name = "textRectangle")]
    pub fn text_rectangle(&self) -> Option<Rectangle> {
        self.0.text_rectangle.clone().map(Rectangle)
    }

    /// The paths (`a:pathLst`), in order.
    #[wasm_bindgen(getter, js_name = "paths")]
    pub fn paths(&self) -> Vec<Path2DSpec> {
        self.0.paths.iter().cloned().map(Path2DSpec).collect()
    }

    /// Every guide's value at the given size, as a record from guide name to number.
    ///
    /// Throws an `OoxmlError` with code `MalformedDocument` if a formula does not parse or refers
    /// to a guide that is not defined.
    #[wasm_bindgen(js_name = "guideValues")]
    pub fn guide_values(&self, context: &GuideContext) -> Result<js_sys::Object, JsValue> {
        let resolved = self.0.guide_values(context.0).map_err(|error| {
            crate::errors::to_js_error(&ooxml::Error::from(ooxml::PptxError::from(error)))
        })?;
        let record = js_sys::Object::new();
        for (name, value) in resolved.iter() {
            let _ =
                js_sys::Reflect::set(&record, &JsValue::from_str(name), &JsValue::from_f64(value));
        }
        Ok(record)
    }

    /// This geometry with every formula evaluated at the given size — what a renderer would draw.
    pub fn resolve(&self, context: &GuideContext) -> Result<ResolvedCustomGeometry, JsValue> {
        self.0
            .resolve(context.0)
            .map(ResolvedCustomGeometry)
            .map_err(|error| {
                crate::errors::to_js_error(&ooxml::Error::from(ooxml::PptxError::from(error)))
            })
    }
}

// ---------------------------------------------------------------------------------------------
// Resolved geometry
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl ResolvedPoint {
    /// The horizontal coordinate.
    #[wasm_bindgen(getter, js_name = "x")]
    pub fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical coordinate.
    #[wasm_bindgen(getter, js_name = "y")]
    pub fn y(&self) -> Emu {
        Emu(self.0.y)
    }
}

#[wasm_bindgen]
impl ResolvedRectangle {
    /// The left edge.
    #[wasm_bindgen(getter, js_name = "left")]
    pub fn left(&self) -> Emu {
        Emu(self.0.left)
    }

    /// The top edge.
    #[wasm_bindgen(getter, js_name = "top")]
    pub fn top(&self) -> Emu {
        Emu(self.0.top)
    }

    /// The right edge.
    #[wasm_bindgen(getter, js_name = "right")]
    pub fn right(&self) -> Emu {
        Emu(self.0.right)
    }

    /// The bottom edge.
    #[wasm_bindgen(getter, js_name = "bottom")]
    pub fn bottom(&self) -> Emu {
        Emu(self.0.bottom)
    }
}

#[wasm_bindgen]
impl ResolvedDrawCommand {
    /// Which command this is, in the same vocabulary [`DrawCommand.kind`](DrawCommand::kind) uses.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::ResolvedDrawCommand::Close => "close".to_owned(),
            ooxml::ResolvedDrawCommand::MoveTo(_) => "move_to".to_owned(),
            ooxml::ResolvedDrawCommand::LineTo(_) => "line_to".to_owned(),
            ooxml::ResolvedDrawCommand::ArcTo { .. } => "arc_to".to_owned(),
            ooxml::ResolvedDrawCommand::QuadBezierTo(..) => "quad_bezier_to".to_owned(),
            ooxml::ResolvedDrawCommand::CubicBezierTo(..) => "cubic_bezier_to".to_owned(),
        }
    }

    /// The points this command names, in order.
    #[wasm_bindgen(getter, js_name = "points")]
    pub fn points(&self) -> Vec<ResolvedPoint> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::MoveTo(point)
            | ooxml::ResolvedDrawCommand::LineTo(point) => vec![ResolvedPoint(*point)],
            ooxml::ResolvedDrawCommand::QuadBezierTo(control, end) => {
                vec![ResolvedPoint(*control), ResolvedPoint(*end)]
            }
            ooxml::ResolvedDrawCommand::CubicBezierTo(first, second, end) => vec![
                ResolvedPoint(*first),
                ResolvedPoint(*second),
                ResolvedPoint(*end),
            ],
            _ => Vec::new(),
        }
    }

    /// The arc's horizontal radius, when this is an arc.
    #[wasm_bindgen(getter, js_name = "widthRadius")]
    pub fn width_radius(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::ArcTo { width_radius, .. } => Some(Emu(*width_radius)),
            _ => None,
        }
    }

    /// The arc's vertical radius, when this is an arc.
    #[wasm_bindgen(getter, js_name = "heightRadius")]
    pub fn height_radius(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::ArcTo { height_radius, .. } => Some(Emu(*height_radius)),
            _ => None,
        }
    }

    /// The arc's start angle, when this is an arc.
    #[wasm_bindgen(getter, js_name = "startAngle")]
    pub fn start_angle(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::ArcTo { start_angle, .. } => Some(Angle(*start_angle)),
            _ => None,
        }
    }

    /// The arc's swing angle, when this is an arc.
    #[wasm_bindgen(getter, js_name = "swingAngle")]
    pub fn swing_angle(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::ArcTo { swing_angle, .. } => Some(Angle(*swing_angle)),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl ResolvedPath {
    /// The path's own coordinate width, when stated.
    #[wasm_bindgen(getter, js_name = "width")]
    pub fn width(&self) -> Option<Emu> {
        self.0.width.map(Emu)
    }

    /// The path's own coordinate height, when stated.
    #[wasm_bindgen(getter, js_name = "height")]
    pub fn height(&self) -> Option<Emu> {
        self.0.height.map(Emu)
    }

    /// How the path is filled, when stated.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Result<Option<PathFillMode>, JsValue> {
        self.0.fill.map(PathFillMode::from_model).transpose()
    }

    /// Whether the path is stroked, when stated.
    #[wasm_bindgen(getter, js_name = "stroke")]
    pub fn stroke(&self) -> Option<bool> {
        self.0.stroke
    }

    /// Whether the path may be extruded in 3-D, when stated.
    #[wasm_bindgen(getter, js_name = "extrusionOk")]
    pub fn extrusion_ok(&self) -> Option<bool> {
        self.0.extrusion_ok
    }

    /// The resolved commands, in order.
    #[wasm_bindgen(getter, js_name = "commands")]
    pub fn commands(&self) -> Vec<ResolvedDrawCommand> {
        self.0
            .commands
            .iter()
            .copied()
            .map(ResolvedDrawCommand)
            .collect()
    }
}

#[wasm_bindgen]
impl ResolvedConnectionSite {
    /// The direction a connector leaves the site in.
    #[wasm_bindgen(getter, js_name = "angle")]
    pub fn angle(&self) -> Angle {
        Angle(self.0.angle)
    }

    /// Where the site is.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> ResolvedPoint {
        ResolvedPoint(self.0.position)
    }
}

#[wasm_bindgen]
impl ResolvedAdjustHandle {
    /// Which kind this is: `"xy"` or `"polar"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { .. } => "xy".to_owned(),
            ooxml::ResolvedAdjustHandle::Polar { .. } => "polar".to_owned(),
        }
    }

    /// Where the handle sits.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> ResolvedPoint {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { position, .. }
            | ooxml::ResolvedAdjustHandle::Polar { position, .. } => ResolvedPoint(*position),
        }
    }

    /// The guide the first axis drives, when the handle names one.
    #[wasm_bindgen(getter, js_name = "firstGuide")]
    pub fn first_guide(&self) -> Option<String> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { guide_ref_x, .. } => guide_ref_x.clone(),
            ooxml::ResolvedAdjustHandle::Polar {
                guide_ref_radius, ..
            } => guide_ref_radius.clone(),
        }
    }

    /// The guide the second axis drives, when the handle names one.
    #[wasm_bindgen(getter, js_name = "secondGuide")]
    pub fn second_guide(&self) -> Option<String> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { guide_ref_y, .. } => guide_ref_y.clone(),
            ooxml::ResolvedAdjustHandle::Polar {
                guide_ref_angle, ..
            } => guide_ref_angle.clone(),
        }
    }

    /// The lower resolved limit of the first axis, when stated.
    #[wasm_bindgen(getter, js_name = "firstMinimum")]
    pub fn first_minimum(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { min_x, .. } => min_x.map(Emu),
            ooxml::ResolvedAdjustHandle::Polar { min_radius, .. } => min_radius.map(Emu),
        }
    }

    /// The upper resolved limit of the first axis, when stated.
    #[wasm_bindgen(getter, js_name = "firstMaximum")]
    pub fn first_maximum(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { max_x, .. } => max_x.map(Emu),
            ooxml::ResolvedAdjustHandle::Polar { max_radius, .. } => max_radius.map(Emu),
        }
    }

    /// An `xy` handle's lower resolved second-axis limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondMinimum")]
    pub fn second_minimum(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { min_y, .. } => min_y.map(Emu),
            ooxml::ResolvedAdjustHandle::Polar { .. } => None,
        }
    }

    /// An `xy` handle's upper resolved second-axis limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondMaximum")]
    pub fn second_maximum(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { max_y, .. } => max_y.map(Emu),
            ooxml::ResolvedAdjustHandle::Polar { .. } => None,
        }
    }

    /// A `polar` handle's lower resolved angular limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondAngleMinimum")]
    pub fn second_angle_minimum(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Polar { min_angle, .. } => min_angle.map(Angle),
            ooxml::ResolvedAdjustHandle::Xy { .. } => None,
        }
    }

    /// A `polar` handle's upper resolved angular limit, when stated.
    #[wasm_bindgen(getter, js_name = "secondAngleMaximum")]
    pub fn second_angle_maximum(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Polar { max_angle, .. } => max_angle.map(Angle),
            ooxml::ResolvedAdjustHandle::Xy { .. } => None,
        }
    }
}

#[wasm_bindgen]
impl ResolvedCustomGeometry {
    /// The resolved paths, in order.
    #[wasm_bindgen(getter, js_name = "paths")]
    pub fn paths(&self) -> Vec<ResolvedPath> {
        self.0.paths.iter().cloned().map(ResolvedPath).collect()
    }

    /// The resolved text rectangle, when the geometry states one.
    #[wasm_bindgen(getter, js_name = "textRectangle")]
    pub fn text_rectangle(&self) -> Option<ResolvedRectangle> {
        self.0.text_rectangle.map(ResolvedRectangle)
    }

    /// The resolved connection sites.
    #[wasm_bindgen(getter, js_name = "connectionSites")]
    pub fn connection_sites(&self) -> Vec<ResolvedConnectionSite> {
        self.0
            .connection_sites
            .iter()
            .copied()
            .map(ResolvedConnectionSite)
            .collect()
    }

    /// The resolved adjust handles.
    #[wasm_bindgen(getter, js_name = "adjustHandles")]
    pub fn adjust_handles(&self) -> Vec<ResolvedAdjustHandle> {
        self.0
            .adjust_handles
            .iter()
            .cloned()
            .map(ResolvedAdjustHandle)
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Preset adjustments
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl AdjustmentBound {
    /// A literal bound, in the adjustment's own native units.
    #[wasm_bindgen(getter, js_name = "literal")]
    pub fn literal(&self) -> Option<i32> {
        match self.0 {
            ooxml::AdjustmentBound::Literal(value) => Some(value),
            ooxml::AdjustmentBound::Guide(_) => None,
        }
    }

    /// The guide that computes the bound, when it is not a literal.
    #[wasm_bindgen(getter, js_name = "guide")]
    pub fn guide(&self) -> Option<String> {
        match self.0 {
            ooxml::AdjustmentBound::Guide(name) => Some(name.to_owned()),
            ooxml::AdjustmentBound::Literal(_) => None,
        }
    }
}

#[wasm_bindgen]
impl AdjustmentSpec {
    /// The adjustment's name on the wire — `"adj"`, `"adj1"`, `"adj2"`…
    #[wasm_bindgen(getter, js_name = "wireName")]
    pub fn wire_name(&self) -> String {
        self.0 .0.wire_name.to_owned()
    }

    /// Which axis the adjustment moves along.
    #[wasm_bindgen(getter, js_name = "axis")]
    pub fn axis(&self) -> Result<AdjustmentAxis, JsValue> {
        AdjustmentAxis::from_model(self.0 .0.axis)
    }

    /// The value the shape uses when it states none.
    #[wasm_bindgen(getter, js_name = "default")]
    pub fn default(&self) -> i32 {
        self.0 .0.default
    }

    /// The lower end of the adjustment's range.
    #[wasm_bindgen(getter, js_name = "minimum")]
    pub fn minimum(&self) -> AdjustmentBound {
        AdjustmentBound(self.0 .0.min)
    }

    /// The upper end of the adjustment's range.
    #[wasm_bindgen(getter, js_name = "maximum")]
    pub fn maximum(&self) -> AdjustmentBound {
        AdjustmentBound(self.0 .0.max)
    }
}

#[wasm_bindgen]
impl BoundedAdjustment {
    /// What the specification says about this adjustment.
    #[wasm_bindgen(getter, js_name = "spec")]
    pub fn spec(&self) -> AdjustmentSpec {
        AdjustmentSpec(AdjustmentSpecRef(self.0.spec))
    }

    /// The value the shape states, or the specification's default when it states none.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> f64 {
        self.0.value
    }

    /// Whether the shape states a value of its own.
    #[wasm_bindgen(getter, js_name = "isOverridden")]
    pub fn is_overridden(&self) -> bool {
        self.0.is_overridden
    }

    /// The lower end of the range, resolved at the shape's own size.
    #[wasm_bindgen(getter, js_name = "minimum")]
    pub fn minimum(&self) -> f64 {
        self.0.minimum
    }

    /// The upper end of the range, resolved at the shape's own size.
    #[wasm_bindgen(getter, js_name = "maximum")]
    pub fn maximum(&self) -> f64 {
        self.0.maximum
    }

    /// The value clamped into the range — what a consumer would actually draw.
    #[wasm_bindgen(getter, js_name = "pinnedValue")]
    pub fn pinned_value(&self) -> f64 {
        self.0.pinned_value()
    }
}

// ---------------------------------------------------------------------------------------------
// Preset shape geometry — the generated table
// ---------------------------------------------------------------------------------------------

/// One adjustment's value, with its unit still attached.
#[derive(Debug, Clone, Copy)]
enum Adjustment {
    /// A proportion of the shape's own extent.
    Fraction(ooxml::Fraction),
    /// An angle.
    Angle(ooxml::Angle),
}

impl Adjustment {
    /// Wraps a proportion.
    fn fraction(value: ooxml::Fraction) -> Self {
        Self::Fraction(value)
    }

    /// Wraps an angle.
    fn angle(value: ooxml::Angle) -> Self {
        Self::Angle(value)
    }

    /// The value as the class a caller receives — a `Fraction` or an `Angle`, never a bare number.
    fn into_js_value(self) -> JsValue {
        match self {
            Self::Fraction(value) => Fraction(value).into(),
            Self::Angle(value) => Angle(value).into(),
        }
    }
}

/// The record a caller passes to `ShapeGeometry.of`, with the two lookups the generated table needs.
///
/// A plain JavaScript object, read by key. Reading it here rather than eagerly converting it means
/// a value of the wrong unit is reported against the adjustment that wanted it, by name.
struct Values<'a> {
    given: Option<&'a js_sys::Object>,
}

impl Values<'_> {
    /// The named adjustment as a proportion.
    ///
    /// Read through the class's own getter rather than by unwrapping it: `wasm-bindgen`'s
    /// `JsValue` → class conversion takes ownership, and a record whose values were freed by
    /// building one geometry could not be used to build a second.
    fn fraction(&self, name: &str, shape: &str) -> Result<ooxml::Fraction, JsValue> {
        let value = self.lookup(name, shape)?;
        number(&value, "ratio")
            .map(ooxml::Fraction::from_ratio)
            .ok_or_else(|| {
                invalid_argument(format!(
                    "the `{name}` adjustment of a {shape} is a proportion, so it takes a Fraction"
                ))
            })
    }

    /// The named adjustment as an angle.
    fn angle(&self, name: &str, shape: &str) -> Result<ooxml::Angle, JsValue> {
        let value = self.lookup(name, shape)?;
        number(&value, "degrees")
            .map(ooxml::Angle::from_degrees)
            .ok_or_else(|| {
                invalid_argument(format!(
                    "the `{name}` adjustment of a {shape} is an angle, so it takes an Angle"
                ))
            })
    }

    /// The value given for `name`, or a message naming what the shape wanted.
    fn lookup(&self, name: &str, shape: &str) -> Result<JsValue, JsValue> {
        let missing = || {
            invalid_argument(format!(
                "a {shape} needs an adjustment called `{name}`; \
                 ShapeGeometry.adjustmentNames names them all"
            ))
        };
        let given = self.given.ok_or_else(missing)?;
        let value = js_sys::Reflect::get(given, &JsValue::from_str(name)).map_err(|_| missing())?;
        if value.is_undefined() || value.is_null() {
            return Err(missing());
        }
        Ok(value)
    }

    /// Refuses a name the shape does not have, rather than ignoring it.
    fn reject_unknown(&self, expected: &[&str], shape: &str) -> Result<(), JsValue> {
        let Some(given) = self.given else {
            return Ok(());
        };
        for key in js_sys::Object::keys(given).iter() {
            let Some(name) = key.as_string() else {
                continue;
            };
            if !expected.contains(&name.as_str()) {
                return Err(invalid_argument(format!(
                    "a {shape} has no adjustment called `{name}`; it has {expected:?}"
                )));
            }
        }
        Ok(())
    }
}

/// One numeric property of a measure class, or `None` when the value is not that class.
///
/// A `Fraction` publishes `ratio`; an `Angle` publishes `degrees`. Reading the wrong one gives
/// `undefined`, which is how a value of the wrong unit is caught — and a freed object throws inside
/// wasm, which surfaces here as a `Reflect` failure and is reported the same way.
fn number(value: &JsValue, property: &str) -> Option<f64> {
    if !value.is_object() {
        return None;
    }
    js_sys::Reflect::get(value, &JsValue::from_str(property))
        .ok()
        .and_then(|found| found.as_f64())
}

/// The 117 preset shapes that carry named adjustments, and what each adjustment is called.
///
/// Every name and unit here is the one `mjx_dml::ShapeGeometry` states, which is in turn the one
/// ECMA-376 Part 1's prose gives the shape's `a:gd` — so `corner_radius` is the rounded rectangle's
/// `adj`, and the table is the mapping between the two. Adding a shape upstream is a compile error
/// in `parts` until it appears here.
macro_rules! shape_geometries {
    ($( $variant:ident { $($field:ident : $unit:ident),* $(,)? } )*) => {
        impl ShapeGeometry {
            /// The preset this geometry names, and its adjustments, or `None` for a preset this
            /// build carries by name alone.
            fn parts(&self) -> Option<(ooxml::PresetShapeType, Vec<(&'static str, Adjustment)>)> {
                match &self.0 {
                    $(
                        ooxml::ShapeGeometry::$variant { $($field),* } => Some((
                            ooxml::PresetShapeType::$variant,
                            vec![ $( (stringify!($field), Adjustment::$unit(*$field)) ),* ],
                        )),
                    )*
                    ooxml::ShapeGeometry::Unmodeled(_) => None,
                }
            }

            /// The geometry for one preset and the values given for its adjustments.
            fn build(
                preset: ooxml::PresetShapeType,
                values: &Values<'_>,
            ) -> Result<ooxml::ShapeGeometry, JsValue> {
                Ok(match preset {
                    $(
                        ooxml::PresetShapeType::$variant => {
                            values.reject_unknown(
                                &[$(stringify!($field)),*],
                                stringify!($variant),
                            )?;
                            ooxml::ShapeGeometry::$variant {
                                $($field: values.$unit(
                                    stringify!($field),
                                    stringify!($variant),
                                )?),*
                            }
                        }
                    )*
                    // Every other preset carries no modelled adjustment: it is written as a bare
                    // `a:prstGeom`, and whatever `a:avLst` the document held is preserved.
                    other => {
                        values.reject_unknown(&[], "shape without modelled adjustments")?;
                        ooxml::ShapeGeometry::Unmodeled(other)
                    }
                })
            }

            /// What one preset's adjustments are called, outermost first.
            fn names(preset: ooxml::PresetShapeType) -> Vec<&'static str> {
                match preset {
                    $( ooxml::PresetShapeType::$variant => vec![ $(stringify!($field)),* ], )*
                    _ => Vec::new(),
                }
            }
        }
    };
}

include!("shape_geometry_table.rs");

#[wasm_bindgen]
impl ShapeGeometry {
    /// The geometry of one preset shape, with values for the adjustments it carries.
    ///
    /// ```js
    /// ShapeGeometry.of(PresetShapeType.RoundedRectangle, { corner_radius: Fraction.of(0.25) });
    /// ShapeGeometry.of(PresetShapeType.Arc, {
    ///   start_angle: Angle.fromDegrees(0),
    ///   end_angle: Angle.fromDegrees(90),
    /// });
    /// ShapeGeometry.of(PresetShapeType.Ellipse);   // an ellipse has no adjustments
    /// ```
    ///
    /// The keys are the adjustment names, which stay `snake_case`: they are data — the names
    /// ECMA-376's prose gives each `a:gd` — rather than method names, and renaming data would make
    /// `adjustmentNames` disagree with the record it describes.
    ///
    /// Throws an `OoxmlError` for a missing or unrecognised adjustment name, or a value of the
    /// wrong unit.
    #[wasm_bindgen(js_name = "of")]
    pub fn of(
        preset: PresetShapeType,
        adjustments: Option<js_sys::Object>,
    ) -> Result<ShapeGeometry, JsValue> {
        let values = Values {
            given: adjustments.as_ref(),
        };
        Self::build(preset.into(), &values).map(Self)
    }

    /// The preset this geometry names.
    #[wasm_bindgen(getter, js_name = "preset")]
    pub fn preset(&self) -> Result<PresetShapeType, JsValue> {
        let preset = match &self.0 {
            ooxml::ShapeGeometry::Unmodeled(preset) => *preset,
            _ => match self.parts() {
                Some((preset, _)) => preset,
                // Unreachable: `parts` returns `None` only for `Unmodeled`, matched above.
                None => return Err(invalid_argument("this geometry names no preset")),
            },
        };
        PresetShapeType::from_model(preset)
    }

    /// The adjustments this geometry states, as a record from name to `Fraction` or `Angle`.
    #[wasm_bindgen(getter, js_name = "adjustments")]
    pub fn adjustments(&self) -> js_sys::Object {
        let record = js_sys::Object::new();
        if let Some((_, adjustments)) = self.parts() {
            for (name, value) in adjustments {
                let _ =
                    js_sys::Reflect::set(&record, &JsValue::from_str(name), &value.into_js_value());
            }
        }
        record
    }

    /// What a preset's adjustments are called — the keys `of` expects, in the order the
    /// specification lists them.
    #[wasm_bindgen(js_name = "adjustmentNames")]
    pub fn adjustment_names(preset: PresetShapeType) -> Vec<String> {
        Self::names(preset.into())
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

#[wasm_bindgen]
impl Geometry {
    /// One of the presets, with its adjustments.
    pub fn preset(geometry: &ShapeGeometry) -> Self {
        Self(ooxml::Geometry::Preset(geometry.0))
    }

    /// A path the document draws itself.
    pub fn custom(geometry: &CustomGeometrySpec) -> Self {
        Self(ooxml::Geometry::Custom(geometry.0.clone()))
    }

    /// Whatever the shape's placeholder chain says — the shape states no geometry of its own.
    pub fn inherited() -> Self {
        Self(ooxml::Geometry::Inherited)
    }

    /// Which kind this is: `"preset"`, `"custom"` or `"inherited"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::Geometry::Preset(_) => "preset".to_owned(),
            ooxml::Geometry::Custom(_) => "custom".to_owned(),
            ooxml::Geometry::Inherited => "inherited".to_owned(),
        }
    }

    /// The preset geometry, when this is a preset.
    #[wasm_bindgen(getter, js_name = "presetGeometry")]
    pub fn preset_geometry(&self) -> Option<ShapeGeometry> {
        match &self.0 {
            ooxml::Geometry::Preset(geometry) => Some(ShapeGeometry(*geometry)),
            _ => None,
        }
    }

    /// The custom geometry, when this is one.
    #[wasm_bindgen(getter, js_name = "customGeometry")]
    pub fn custom_geometry(&self) -> Option<CustomGeometrySpec> {
        match &self.0 {
            ooxml::Geometry::Custom(geometry) => Some(CustomGeometrySpec(geometry.clone())),
            _ => None,
        }
    }
}
