//! Where a shape is, how big it is, and what outline it draws.
//!
//! # `ShapeGeometry`, and why it is one class rather than 117 constructors
//!
//! DrawingML's preset shapes each carry their own named adjustments: a rounded rectangle has a
//! `corner_radius`, an arc has a `start_angle` and an `end_angle`, a callout has four vertices. The
//! Rust enumeration spells all 117 out, because Rust can pattern-match them.
//!
//! Projecting that as 117 static constructors would give Python 117 methods whose parameter names
//! are the adjustment names anyway — the same strings, spread over 117 signatures, with no way to
//! ask what a shape's adjustments are called. So the projection keeps the *table* instead:
//! `ShapeGeometry.of` takes the preset and a mapping from adjustment name to value,
//! `ShapeGeometry.adjustments` hands the same mapping back, and `ShapeGeometry.adjustment_names`
//! says what a given preset's adjustments are
//! called. Nothing is lost: every variant is constructible and every variant is readable, and the
//! one generated table below is the whole of it.
//!
//! The **values keep their units**: an adjustment is an [`Fraction`] or an [`Angle`], never a bare
//! number, and handing a shape an angle where it wanted a proportion raises rather than silently
//! writing sixty times the wrong value.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::enums::{AdjustmentAxis, PathFillMode, PresetShapeType, SlideSizeKind};
use crate::errors::to_py_err;
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

#[pymethods]
impl ShapeBounds {
    /// A rectangle stated directly in English Metric Units.
    #[new]
    fn new(offset_x_emu: i64, offset_y_emu: i64, width_emu: i64, height_emu: i64) -> Self {
        Self(ooxml::ShapeBounds::new(
            offset_x_emu,
            offset_y_emu,
            width_emu,
            height_emu,
        ))
    }

    /// A rectangle in inches — the unit slide layouts are usually reasoned about in.
    #[staticmethod]
    fn from_inches(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self(ooxml::ShapeBounds::from_inches(x, y, width, height))
    }

    /// The rectangle that contains both this one and `other`.
    fn union(&self, other: &Self) -> Self {
        Self(self.0.union(other.0))
    }

    /// The bounds a transform states, when it states both an offset and an extent.
    #[staticmethod]
    fn from_transform(transform: &Transform2D) -> Option<Self> {
        ooxml::ShapeBounds::from_transform(&transform.0).map(Self)
    }

    /// This rectangle as a transform.
    // `&self` rather than `self`: a `#[pymethods]` instance method has no by-value receiver, so
    // the convention this lint asks for is not expressible here.
    #[expect(
        clippy::wrong_self_convention,
        reason = "a pyclass method takes `&self`"
    )]
    fn to_transform(&self) -> Transform2D {
        Transform2D(self.0.to_transform())
    }

    /// The left edge, in EMU.
    #[getter]
    fn offset_x_emu(&self) -> i64 {
        self.0.offset_x_emu
    }

    /// The top edge, in EMU.
    #[getter]
    fn offset_y_emu(&self) -> i64 {
        self.0.offset_y_emu
    }

    /// The width, in EMU.
    #[getter]
    fn width_emu(&self) -> i64 {
        self.0.width_emu
    }

    /// The height, in EMU.
    #[getter]
    fn height_emu(&self) -> i64 {
        self.0.height_emu
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl SlideSize {
    /// 13⅓ by 7½ inches — the 16∶9 size PowerPoint has defaulted to since 2013.
    #[staticmethod]
    fn widescreen() -> Self {
        Self(ooxml::SlideSize::widescreen())
    }

    /// 10 by 7½ inches — the older 4∶3 size.
    #[staticmethod]
    fn standard() -> Self {
        Self(ooxml::SlideSize::standard())
    }

    /// A custom size in English Metric Units.
    #[staticmethod]
    fn from_emu(width_emu: i64, height_emu: i64) -> Self {
        Self(ooxml::SlideSize::from_emu(width_emu, height_emu))
    }

    /// The width, in EMU.
    #[getter]
    fn width_emu(&self) -> i64 {
        self.0.width_emu
    }

    /// The height, in EMU.
    #[getter]
    fn height_emu(&self) -> i64 {
        self.0.height_emu
    }

    /// The paper or screen kind `p:sldSz@type` names.
    #[getter]
    fn kind(&self) -> PyResult<SlideSizeKind> {
        SlideSizeKind::from_model(self.0.kind)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl CellMargins {
    /// Four margins, each optional; an unstated one is inherited.
    #[new]
    #[pyo3(signature = (left = None, right = None, top = None, bottom = None))]
    fn new(left: Option<Emu>, right: Option<Emu>, top: Option<Emu>, bottom: Option<Emu>) -> Self {
        Self(ooxml::CellMargins {
            left: left.map(|value| value.0),
            right: right.map(|value| value.0),
            top: top.map(|value| value.0),
            bottom: bottom.map(|value| value.0),
        })
    }

    /// The same margin on all four sides.
    #[staticmethod]
    fn uniform(margin: Emu) -> Self {
        Self(ooxml::CellMargins::uniform(margin.0))
    }

    /// The left margin, when stated.
    #[getter]
    fn left(&self) -> Option<Emu> {
        self.0.left.map(Emu)
    }

    /// The right margin, when stated.
    #[getter]
    fn right(&self) -> Option<Emu> {
        self.0.right.map(Emu)
    }

    /// The top margin, when stated.
    #[getter]
    fn top(&self) -> Option<Emu> {
        self.0.top.map(Emu)
    }

    /// The bottom margin, when stated.
    #[getter]
    fn bottom(&self) -> Option<Emu> {
        self.0.bottom.map(Emu)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Position {
    /// A point in English Metric Units.
    #[new]
    fn new(x: Emu, y: Emu) -> Self {
        Self(ooxml::Position { x: x.0, y: y.0 })
    }

    /// A point given as two raw EMU values.
    #[staticmethod]
    fn from_emu(x: i64, y: i64) -> Self {
        Self(ooxml::Position::from_emu(x, y))
    }

    /// The horizontal coordinate.
    #[getter]
    fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical coordinate.
    #[getter]
    fn y(&self) -> Emu {
        Emu(self.0.y)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Size {
    /// An extent in English Metric Units.
    #[new]
    fn new(width: Emu, height: Emu) -> Self {
        Self(ooxml::Size {
            width: width.0,
            height: height.0,
        })
    }

    /// An extent given as two raw EMU values.
    #[staticmethod]
    fn from_emu(width: i64, height: i64) -> Self {
        Self(ooxml::Size::from_emu(width, height))
    }

    /// The width.
    #[getter]
    fn width(&self) -> Emu {
        Emu(self.0.width)
    }

    /// The height.
    #[getter]
    fn height(&self) -> Emu {
        Emu(self.0.height)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Transform2D {
    /// A transform. Every part is optional, and an unstated one is inherited.
    #[new]
    #[pyo3(signature = (
        position = None,
        size = None,
        rotation = None,
        flip_horizontal = None,
        flip_vertical = None,
        child_position = None,
        child_size = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
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
    #[getter]
    fn position(&self) -> Option<Position> {
        self.0.position.map(Position)
    }

    /// The extent, when stated.
    #[getter]
    fn size(&self) -> Option<Size> {
        self.0.size.map(Size)
    }

    /// The rotation, when stated.
    #[getter]
    fn rotation(&self) -> Option<Angle> {
        self.0.rotation.map(Angle)
    }

    /// Whether the shape is mirrored horizontally, when stated.
    #[getter]
    fn flip_horizontal(&self) -> Option<bool> {
        self.0.flip_horizontal
    }

    /// Whether the shape is mirrored vertically, when stated.
    #[getter]
    fn flip_vertical(&self) -> Option<bool> {
        self.0.flip_vertical
    }

    /// A group's child-space offset, when stated.
    #[getter]
    fn child_position(&self) -> Option<Position> {
        self.0.child_position.map(Position)
    }

    /// A group's child-space extent, when stated.
    #[getter]
    fn child_size(&self) -> Option<Size> {
        self.0.child_size.map(Size)
    }

    /// Whether this transform states nothing at all.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The scale a group applies to its members, when it states both a child and an outer extent.
    #[getter]
    fn child_scale(&self) -> Option<(f64, f64)> {
        self.0.child_scale()
    }

    /// A point in a group's child space, mapped to the surface.
    fn child_to_parent(&self, point: Position) -> Option<Position> {
        self.0.child_to_parent(point.0).map(Position)
    }

    /// A point on the surface, mapped into a group's child space.
    fn parent_to_child(&self, point: Position) -> Option<Position> {
        self.0.parent_to_child(point.0).map(Position)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// ---------------------------------------------------------------------------------------------
// Custom geometry
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl AdjustCoordinate {
    /// A literal length.
    #[staticmethod]
    fn emu(value: Emu) -> Self {
        Self(ooxml::AdjustCoordinate::Emu(value.0))
    }

    /// The value of a named guide.
    #[staticmethod]
    fn guide(name: &str) -> Self {
        Self(ooxml::AdjustCoordinate::Guide(name.to_owned()))
    }

    /// A coordinate parsed from the wire form — a number, or a guide name.
    #[staticmethod]
    fn from_wire(value: &str) -> Self {
        Self(ooxml::AdjustCoordinate::from_wire(value))
    }

    /// The literal length, when this is one.
    #[getter]
    fn value(&self) -> Option<Emu> {
        match &self.0 {
            ooxml::AdjustCoordinate::Emu(value) => Some(Emu(*value)),
            ooxml::AdjustCoordinate::Guide(_) => None,
        }
    }

    /// The guide's name, when this names one.
    #[getter]
    fn guide_name(&self) -> Option<&str> {
        match &self.0 {
            ooxml::AdjustCoordinate::Guide(name) => Some(name),
            ooxml::AdjustCoordinate::Emu(_) => None,
        }
    }

    /// The wire form, exactly as it is written.
    fn to_wire(&self) -> String {
        self.0.to_wire()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl AdjustAngle {
    /// A literal angle.
    #[staticmethod]
    fn angle(value: Angle) -> Self {
        Self(ooxml::AdjustAngle::Angle(value.0))
    }

    /// The value of a named guide.
    #[staticmethod]
    fn guide(name: &str) -> Self {
        Self(ooxml::AdjustAngle::Guide(name.to_owned()))
    }

    /// An angle parsed from the wire form — a number of sixtieths of a degree, or a guide name.
    #[staticmethod]
    fn from_wire(value: &str) -> Self {
        Self(ooxml::AdjustAngle::from_wire(value))
    }

    /// The literal angle, when this is one.
    #[getter]
    fn value(&self) -> Option<Angle> {
        match &self.0 {
            ooxml::AdjustAngle::Angle(value) => Some(Angle(*value)),
            ooxml::AdjustAngle::Guide(_) => None,
        }
    }

    /// The guide's name, when this names one.
    #[getter]
    fn guide_name(&self) -> Option<&str> {
        match &self.0 {
            ooxml::AdjustAngle::Guide(name) => Some(name),
            ooxml::AdjustAngle::Angle(_) => None,
        }
    }

    /// The wire form, exactly as it is written.
    fn to_wire(&self) -> String {
        self.0.to_wire()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Point {
    /// A point whose coordinates may each be literal or guide-relative.
    #[new]
    fn new(x: AdjustCoordinate, y: AdjustCoordinate) -> Self {
        Self(ooxml::Point { x: x.0, y: y.0 })
    }

    /// A point at two literal EMU coordinates.
    #[staticmethod]
    fn from_emu(x: i64, y: i64) -> Self {
        Self(ooxml::Point::from_emu(x, y))
    }

    /// The horizontal coordinate.
    #[getter]
    fn x(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.x.clone())
    }

    /// The vertical coordinate.
    #[getter]
    fn y(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.y.clone())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl GuideSpec {
    /// A guide: the name other formulas refer to it by, and the formula that computes it.
    #[new]
    fn new(name: &str, formula: &str) -> Self {
        Self(ooxml::GuideSpec {
            name: name.to_owned(),
            formula: formula.to_owned(),
        })
    }

    /// The guide's name.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// The formula, in the seventeen-operator prefix language `a:gd@fmla` uses.
    #[getter]
    fn formula(&self) -> &str {
        &self.0.formula
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl GuideContext {
    /// The extent a formula's `w` and `h` stand for.
    #[staticmethod]
    fn from_extents(width: Emu, height: Emu) -> Self {
        Self(ooxml::GuideContext::from_extents(width.0, height.0))
    }

    /// The same, from a `Size`.
    #[staticmethod]
    fn from_size(size: Size) -> Self {
        Self(ooxml::GuideContext::from_size(size.0))
    }

    /// The width `w` stands for.
    #[getter]
    fn width(&self) -> Emu {
        Emu(self.0.width())
    }

    /// The height `h` stands for.
    #[getter]
    fn height(&self) -> Emu {
        Emu(self.0.height())
    }

    /// The value of one built-in variable — `w`, `h`, `l`, `t`, `r`, `b`, `hc`, `vc`, `ss`, `ls`,
    /// `ssd2`… — or `None` if that is not a variable name.
    fn variable(&self, name: &str) -> Option<f64> {
        self.0.variable(name)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Rectangle {
    /// A rectangle whose four edges may each be literal or guide-relative.
    #[new]
    fn new(
        left: AdjustCoordinate,
        top: AdjustCoordinate,
        right: AdjustCoordinate,
        bottom: AdjustCoordinate,
    ) -> Self {
        Self(ooxml::Rectangle {
            left: left.0,
            top: top.0,
            right: right.0,
            bottom: bottom.0,
        })
    }

    /// The left edge.
    #[getter]
    fn left(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.left.clone())
    }

    /// The top edge.
    #[getter]
    fn top(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.top.clone())
    }

    /// The right edge.
    #[getter]
    fn right(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.right.clone())
    }

    /// The bottom edge.
    #[getter]
    fn bottom(&self) -> AdjustCoordinate {
        AdjustCoordinate(self.0.bottom.clone())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl DrawCommand {
    /// Close the current subpath.
    #[staticmethod]
    fn close() -> Self {
        Self(ooxml::DrawCommand::Close)
    }

    /// Start a new subpath at a point.
    #[staticmethod]
    fn move_to(point: Point) -> Self {
        Self(ooxml::DrawCommand::MoveTo(point.0))
    }

    /// Draw a straight segment to a point.
    #[staticmethod]
    fn line_to(point: Point) -> Self {
        Self(ooxml::DrawCommand::LineTo(point.0))
    }

    /// Draw an elliptical arc.
    #[staticmethod]
    fn arc_to(
        width_radius: AdjustCoordinate,
        height_radius: AdjustCoordinate,
        start_angle: AdjustAngle,
        swing_angle: AdjustAngle,
    ) -> Self {
        Self(ooxml::DrawCommand::ArcTo {
            width_radius: width_radius.0,
            height_radius: height_radius.0,
            start_angle: start_angle.0,
            swing_angle: swing_angle.0,
        })
    }

    /// Draw a quadratic Bézier through one control point.
    #[staticmethod]
    fn quad_bezier_to(control: Point, end: Point) -> Self {
        Self(ooxml::DrawCommand::QuadBezierTo(control.0, end.0))
    }

    /// Draw a cubic Bézier through two control points.
    #[staticmethod]
    fn cubic_bezier_to(first: Point, second: Point, end: Point) -> Self {
        Self(ooxml::DrawCommand::CubicBezierTo(first.0, second.0, end.0))
    }

    /// Which command this is: `"close"`, `"move_to"`, `"line_to"`, `"arc_to"`, `"quad_bezier_to"`
    /// or `"cubic_bezier_to"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::DrawCommand::Close => "close",
            ooxml::DrawCommand::MoveTo(_) => "move_to",
            ooxml::DrawCommand::LineTo(_) => "line_to",
            ooxml::DrawCommand::ArcTo { .. } => "arc_to",
            ooxml::DrawCommand::QuadBezierTo(..) => "quad_bezier_to",
            ooxml::DrawCommand::CubicBezierTo(..) => "cubic_bezier_to",
        }
    }

    /// The points this command names, in order; empty for `close` and for `arc_to`.
    #[getter]
    fn points(&self) -> Vec<Point> {
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

    /// The arc's two radii, when this is an arc.
    #[getter]
    fn radii(&self) -> Option<(AdjustCoordinate, AdjustCoordinate)> {
        match &self.0 {
            ooxml::DrawCommand::ArcTo {
                width_radius,
                height_radius,
                ..
            } => Some((
                AdjustCoordinate(width_radius.clone()),
                AdjustCoordinate(height_radius.clone()),
            )),
            _ => None,
        }
    }

    /// The arc's start and swing angles, when this is an arc.
    #[getter]
    fn angles(&self) -> Option<(AdjustAngle, AdjustAngle)> {
        match &self.0 {
            ooxml::DrawCommand::ArcTo {
                start_angle,
                swing_angle,
                ..
            } => Some((
                AdjustAngle(start_angle.clone()),
                AdjustAngle(swing_angle.clone()),
            )),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Path2DSpec {
    /// One path of a custom geometry.
    #[new]
    #[pyo3(signature = (
        commands,
        width = None,
        height = None,
        fill = None,
        stroke = None,
        extrusion_ok = None,
    ))]
    fn new(
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
    #[getter]
    fn width(&self) -> Option<Emu> {
        self.0.width.map(Emu)
    }

    /// The path's own coordinate height, when stated.
    #[getter]
    fn height(&self) -> Option<Emu> {
        self.0.height.map(Emu)
    }

    /// How the path is filled, when stated.
    #[getter]
    fn fill(&self) -> PyResult<Option<PathFillMode>> {
        self.0.fill.map(PathFillMode::from_model).transpose()
    }

    /// Whether the path is stroked, when stated.
    #[getter]
    fn stroke(&self) -> Option<bool> {
        self.0.stroke
    }

    /// Whether the path may be extruded in 3-D, when stated.
    #[getter]
    fn extrusion_ok(&self) -> Option<bool> {
        self.0.extrusion_ok
    }

    /// The commands that draw the path, in order.
    #[getter]
    fn commands(&self) -> Vec<DrawCommand> {
        self.0.commands.iter().cloned().map(DrawCommand).collect()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ConnectionSite {
    /// A point a connector can attach to, and the direction it leaves in.
    #[new]
    fn new(angle: AdjustAngle, position: Point) -> Self {
        Self(ooxml::ConnectionSite {
            angle: angle.0,
            position: position.0,
        })
    }

    /// The direction a connector leaves the site in.
    #[getter]
    fn angle(&self) -> AdjustAngle {
        AdjustAngle(self.0.angle.clone())
    }

    /// Where the site is.
    #[getter]
    fn position(&self) -> Point {
        Point(self.0.position.clone())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl AdjustHandle {
    /// A handle that moves in two dimensions, driving one guide per axis.
    #[staticmethod]
    #[pyo3(signature = (
        position,
        guide_ref_x = None,
        min_x = None,
        max_x = None,
        guide_ref_y = None,
        min_y = None,
        max_y = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn xy(
        position: Point,
        guide_ref_x: Option<String>,
        min_x: Option<AdjustCoordinate>,
        max_x: Option<AdjustCoordinate>,
        guide_ref_y: Option<String>,
        min_y: Option<AdjustCoordinate>,
        max_y: Option<AdjustCoordinate>,
    ) -> Self {
        Self(ooxml::AdjustHandle::Xy {
            position: position.0,
            guide_ref_x,
            min_x: min_x.map(|value| value.0),
            max_x: max_x.map(|value| value.0),
            guide_ref_y,
            min_y: min_y.map(|value| value.0),
            max_y: max_y.map(|value| value.0),
        })
    }

    /// A handle that moves in polar coordinates, driving a radius guide and an angle guide.
    #[staticmethod]
    #[pyo3(signature = (
        position,
        guide_ref_radius = None,
        min_radius = None,
        max_radius = None,
        guide_ref_angle = None,
        min_angle = None,
        max_angle = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn polar(
        position: Point,
        guide_ref_radius: Option<String>,
        min_radius: Option<AdjustCoordinate>,
        max_radius: Option<AdjustCoordinate>,
        guide_ref_angle: Option<String>,
        min_angle: Option<AdjustAngle>,
        max_angle: Option<AdjustAngle>,
    ) -> Self {
        Self(ooxml::AdjustHandle::Polar {
            position: position.0,
            guide_ref_radius,
            min_radius: min_radius.map(|value| value.0),
            max_radius: max_radius.map(|value| value.0),
            guide_ref_angle,
            min_angle: min_angle.map(|value| value.0),
            max_angle: max_angle.map(|value| value.0),
        })
    }

    /// Which kind this is: `"xy"` or `"polar"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::AdjustHandle::Xy { .. } => "xy",
            ooxml::AdjustHandle::Polar { .. } => "polar",
        }
    }

    /// Where the handle sits.
    #[getter]
    fn position(&self) -> Point {
        match &self.0 {
            ooxml::AdjustHandle::Xy { position, .. }
            | ooxml::AdjustHandle::Polar { position, .. } => Point(position.clone()),
        }
    }

    /// The guide the first axis drives — `gdRefX` or `gdRefR` — when the handle names one.
    #[getter]
    fn first_guide(&self) -> Option<&str> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { guide_ref_x, .. } => guide_ref_x.as_deref(),
            ooxml::AdjustHandle::Polar {
                guide_ref_radius, ..
            } => guide_ref_radius.as_deref(),
        }
    }

    /// The guide the second axis drives — `gdRefY` or `gdRefAng` — when the handle names one.
    #[getter]
    fn second_guide(&self) -> Option<&str> {
        match &self.0 {
            ooxml::AdjustHandle::Xy { guide_ref_y, .. } => guide_ref_y.as_deref(),
            ooxml::AdjustHandle::Polar {
                guide_ref_angle, ..
            } => guide_ref_angle.as_deref(),
        }
    }

    /// The first axis's limits, when stated.
    #[getter]
    fn first_limits(&self) -> (Option<AdjustCoordinate>, Option<AdjustCoordinate>) {
        match &self.0 {
            ooxml::AdjustHandle::Xy { min_x, max_x, .. } => (
                min_x.clone().map(AdjustCoordinate),
                max_x.clone().map(AdjustCoordinate),
            ),
            ooxml::AdjustHandle::Polar {
                min_radius,
                max_radius,
                ..
            } => (
                min_radius.clone().map(AdjustCoordinate),
                max_radius.clone().map(AdjustCoordinate),
            ),
        }
    }

    /// The second axis's limits, when stated. An `xy` handle's are coordinates; a `polar` handle's
    /// are angles, reported through [`second_angle_limits`](Self::second_angle_limits).
    #[getter]
    fn second_limits(&self) -> (Option<AdjustCoordinate>, Option<AdjustCoordinate>) {
        match &self.0 {
            ooxml::AdjustHandle::Xy { min_y, max_y, .. } => (
                min_y.clone().map(AdjustCoordinate),
                max_y.clone().map(AdjustCoordinate),
            ),
            ooxml::AdjustHandle::Polar { .. } => (None, None),
        }
    }

    /// A polar handle's angular limits, when stated.
    #[getter]
    fn second_angle_limits(&self) -> (Option<AdjustAngle>, Option<AdjustAngle>) {
        match &self.0 {
            ooxml::AdjustHandle::Polar {
                min_angle,
                max_angle,
                ..
            } => (
                min_angle.clone().map(AdjustAngle),
                max_angle.clone().map(AdjustAngle),
            ),
            ooxml::AdjustHandle::Xy { .. } => (None, None),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl CustomGeometrySpec {
    /// A custom geometry. Only `paths` is usually needed; the rest describe the guides and handles
    /// PowerPoint's own editor manipulates.
    #[new]
    #[pyo3(signature = (
        paths = Vec::new(),
        adjust_values = Vec::new(),
        guides = Vec::new(),
        adjust_handles = Vec::new(),
        connection_sites = Vec::new(),
        text_rectangle = None,
    ))]
    fn new(
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
    #[getter]
    fn adjust_values(&self) -> Vec<GuideSpec> {
        self.0
            .adjust_values
            .iter()
            .cloned()
            .map(GuideSpec)
            .collect()
    }

    /// The computed guides (`a:gdLst`).
    #[getter]
    fn guides(&self) -> Vec<GuideSpec> {
        self.0.guides.iter().cloned().map(GuideSpec).collect()
    }

    /// The adjust handles (`a:ahLst`).
    #[getter]
    fn adjust_handles(&self) -> Vec<AdjustHandle> {
        self.0
            .adjust_handles
            .iter()
            .cloned()
            .map(AdjustHandle)
            .collect()
    }

    /// The connection sites (`a:cxnLst`).
    #[getter]
    fn connection_sites(&self) -> Vec<ConnectionSite> {
        self.0
            .connection_sites
            .iter()
            .cloned()
            .map(ConnectionSite)
            .collect()
    }

    /// The text rectangle (`a:rect`), when stated.
    #[getter]
    fn text_rectangle(&self) -> Option<Rectangle> {
        self.0.text_rectangle.clone().map(Rectangle)
    }

    /// The paths (`a:pathLst`), in order.
    #[getter]
    fn paths(&self) -> Vec<Path2DSpec> {
        self.0.paths.iter().cloned().map(Path2DSpec).collect()
    }

    /// Every guide's value at the given size.
    ///
    /// Raises `MalformedDocumentError` if a formula does not parse or refers to a guide that is not
    /// defined.
    fn guide_values(&self, context: GuideContext) -> PyResult<HashMap<String, f64>> {
        let resolved = self
            .0
            .guide_values(context.0)
            .map_err(|error| to_py_err(ooxml::Error::from(ooxml::PptxError::from(error))))?;
        Ok(resolved
            .iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect())
    }

    /// This geometry with every formula evaluated at the given size — what a renderer would draw.
    fn resolve(&self, context: GuideContext) -> PyResult<ResolvedCustomGeometry> {
        self.0
            .resolve(context.0)
            .map(ResolvedCustomGeometry)
            .map_err(|error| to_py_err(ooxml::Error::from(ooxml::PptxError::from(error))))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// ---------------------------------------------------------------------------------------------
// Resolved geometry
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl ResolvedPoint {
    /// The horizontal coordinate.
    #[getter]
    fn x(&self) -> Emu {
        Emu(self.0.x)
    }

    /// The vertical coordinate.
    #[getter]
    fn y(&self) -> Emu {
        Emu(self.0.y)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ResolvedRectangle {
    /// The left edge.
    #[getter]
    fn left(&self) -> Emu {
        Emu(self.0.left)
    }

    /// The top edge.
    #[getter]
    fn top(&self) -> Emu {
        Emu(self.0.top)
    }

    /// The right edge.
    #[getter]
    fn right(&self) -> Emu {
        Emu(self.0.right)
    }

    /// The bottom edge.
    #[getter]
    fn bottom(&self) -> Emu {
        Emu(self.0.bottom)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ResolvedDrawCommand {
    /// Which command this is, in the same vocabulary [`DrawCommand.kind`](DrawCommand::kind) uses.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::ResolvedDrawCommand::Close => "close",
            ooxml::ResolvedDrawCommand::MoveTo(_) => "move_to",
            ooxml::ResolvedDrawCommand::LineTo(_) => "line_to",
            ooxml::ResolvedDrawCommand::ArcTo { .. } => "arc_to",
            ooxml::ResolvedDrawCommand::QuadBezierTo(..) => "quad_bezier_to",
            ooxml::ResolvedDrawCommand::CubicBezierTo(..) => "cubic_bezier_to",
        }
    }

    /// The points this command names, in order.
    #[getter]
    fn points(&self) -> Vec<ResolvedPoint> {
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

    /// The arc's two radii, when this is an arc.
    #[getter]
    fn radii(&self) -> Option<(Emu, Emu)> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::ArcTo {
                width_radius,
                height_radius,
                ..
            } => Some((Emu(*width_radius), Emu(*height_radius))),
            _ => None,
        }
    }

    /// The arc's start and swing angles, when this is an arc.
    #[getter]
    fn angles(&self) -> Option<(Angle, Angle)> {
        match &self.0 {
            ooxml::ResolvedDrawCommand::ArcTo {
                start_angle,
                swing_angle,
                ..
            } => Some((Angle(*start_angle), Angle(*swing_angle))),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ResolvedPath {
    /// The path's own coordinate width, when stated.
    #[getter]
    fn width(&self) -> Option<Emu> {
        self.0.width.map(Emu)
    }

    /// The path's own coordinate height, when stated.
    #[getter]
    fn height(&self) -> Option<Emu> {
        self.0.height.map(Emu)
    }

    /// How the path is filled, when stated.
    #[getter]
    fn fill(&self) -> PyResult<Option<PathFillMode>> {
        self.0.fill.map(PathFillMode::from_model).transpose()
    }

    /// Whether the path is stroked, when stated.
    #[getter]
    fn stroke(&self) -> Option<bool> {
        self.0.stroke
    }

    /// Whether the path may be extruded in 3-D, when stated.
    #[getter]
    fn extrusion_ok(&self) -> Option<bool> {
        self.0.extrusion_ok
    }

    /// The resolved commands, in order.
    #[getter]
    fn commands(&self) -> Vec<ResolvedDrawCommand> {
        self.0
            .commands
            .iter()
            .copied()
            .map(ResolvedDrawCommand)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ResolvedConnectionSite {
    /// The direction a connector leaves the site in.
    #[getter]
    fn angle(&self) -> Angle {
        Angle(self.0.angle)
    }

    /// Where the site is.
    #[getter]
    fn position(&self) -> ResolvedPoint {
        ResolvedPoint(self.0.position)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ResolvedAdjustHandle {
    /// Which kind this is: `"xy"` or `"polar"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { .. } => "xy",
            ooxml::ResolvedAdjustHandle::Polar { .. } => "polar",
        }
    }

    /// Where the handle sits.
    #[getter]
    fn position(&self) -> ResolvedPoint {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { position, .. }
            | ooxml::ResolvedAdjustHandle::Polar { position, .. } => ResolvedPoint(*position),
        }
    }

    /// The guide the first axis drives, when the handle names one.
    #[getter]
    fn first_guide(&self) -> Option<&str> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { guide_ref_x, .. } => guide_ref_x.as_deref(),
            ooxml::ResolvedAdjustHandle::Polar {
                guide_ref_radius, ..
            } => guide_ref_radius.as_deref(),
        }
    }

    /// The guide the second axis drives, when the handle names one.
    #[getter]
    fn second_guide(&self) -> Option<&str> {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { guide_ref_y, .. } => guide_ref_y.as_deref(),
            ooxml::ResolvedAdjustHandle::Polar {
                guide_ref_angle, ..
            } => guide_ref_angle.as_deref(),
        }
    }

    /// The first axis's resolved limits, when stated.
    #[getter]
    fn first_limits(&self) -> (Option<Emu>, Option<Emu>) {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { min_x, max_x, .. } => {
                (min_x.map(Emu), max_x.map(Emu))
            }
            ooxml::ResolvedAdjustHandle::Polar {
                min_radius,
                max_radius,
                ..
            } => (min_radius.map(Emu), max_radius.map(Emu)),
        }
    }

    /// An `xy` handle's second-axis limits, when stated.
    #[getter]
    fn second_limits(&self) -> (Option<Emu>, Option<Emu>) {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Xy { min_y, max_y, .. } => {
                (min_y.map(Emu), max_y.map(Emu))
            }
            ooxml::ResolvedAdjustHandle::Polar { .. } => (None, None),
        }
    }

    /// A `polar` handle's angular limits, when stated.
    #[getter]
    fn second_angle_limits(&self) -> (Option<Angle>, Option<Angle>) {
        match &self.0 {
            ooxml::ResolvedAdjustHandle::Polar {
                min_angle,
                max_angle,
                ..
            } => (min_angle.map(Angle), max_angle.map(Angle)),
            ooxml::ResolvedAdjustHandle::Xy { .. } => (None, None),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ResolvedCustomGeometry {
    /// The resolved paths, in order.
    #[getter]
    fn paths(&self) -> Vec<ResolvedPath> {
        self.0.paths.iter().cloned().map(ResolvedPath).collect()
    }

    /// The resolved text rectangle, when the geometry states one.
    #[getter]
    fn text_rectangle(&self) -> Option<ResolvedRectangle> {
        self.0.text_rectangle.map(ResolvedRectangle)
    }

    /// The resolved connection sites.
    #[getter]
    fn connection_sites(&self) -> Vec<ResolvedConnectionSite> {
        self.0
            .connection_sites
            .iter()
            .copied()
            .map(ResolvedConnectionSite)
            .collect()
    }

    /// The resolved adjust handles.
    #[getter]
    fn adjust_handles(&self) -> Vec<ResolvedAdjustHandle> {
        self.0
            .adjust_handles
            .iter()
            .cloned()
            .map(ResolvedAdjustHandle)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// ---------------------------------------------------------------------------------------------
// Preset adjustments
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl AdjustmentBound {
    /// A literal bound, in the adjustment's own native units.
    #[getter]
    fn literal(&self) -> Option<i32> {
        match self.0 {
            ooxml::AdjustmentBound::Literal(value) => Some(value),
            ooxml::AdjustmentBound::Guide(_) => None,
        }
    }

    /// The guide that computes the bound, when it is not a literal.
    #[getter]
    fn guide(&self) -> Option<&'static str> {
        match self.0 {
            ooxml::AdjustmentBound::Guide(name) => Some(name),
            ooxml::AdjustmentBound::Literal(_) => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl AdjustmentSpec {
    /// The adjustment's name on the wire — `"adj"`, `"adj1"`, `"adj2"`…
    #[getter]
    fn wire_name(&self) -> &'static str {
        self.0 .0.wire_name
    }

    /// Which axis the adjustment moves along.
    #[getter]
    fn axis(&self) -> PyResult<AdjustmentAxis> {
        AdjustmentAxis::from_model(self.0 .0.axis)
    }

    /// The value the shape uses when it states none.
    #[getter]
    fn default(&self) -> i32 {
        self.0 .0.default
    }

    /// The lower end of the adjustment's range.
    #[getter]
    fn minimum(&self) -> AdjustmentBound {
        AdjustmentBound(self.0 .0.min)
    }

    /// The upper end of the adjustment's range.
    #[getter]
    fn maximum(&self) -> AdjustmentBound {
        AdjustmentBound(self.0 .0.max)
    }

    fn __repr__(&self) -> String {
        format!("AdjustmentSpec({})", self.0 .0.wire_name)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl BoundedAdjustment {
    /// What the specification says about this adjustment.
    #[getter]
    fn spec(&self) -> AdjustmentSpec {
        AdjustmentSpec(AdjustmentSpecRef(self.0.spec))
    }

    /// The value the shape states, or the specification's default when it states none.
    #[getter]
    fn value(&self) -> f64 {
        self.0.value
    }

    /// Whether the shape states a value of its own.
    #[getter]
    fn is_overridden(&self) -> bool {
        self.0.is_overridden
    }

    /// The lower end of the range, resolved at the shape's own size.
    #[getter]
    fn minimum(&self) -> f64 {
        self.0.minimum
    }

    /// The upper end of the range, resolved at the shape's own size.
    #[getter]
    fn maximum(&self) -> f64 {
        self.0.maximum
    }

    /// The value clamped into the range — what a consumer would actually draw.
    #[getter]
    fn pinned_value(&self) -> f64 {
        self.0.pinned_value()
    }

    fn __repr__(&self) -> String {
        format!(
            "BoundedAdjustment({}={})",
            self.0.spec.wire_name, self.0.value
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
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

    /// The value as the class a caller receives.
    fn into_py_object(self, python: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(match self {
            Self::Fraction(value) => Fraction(value).into_pyobject(python)?.into_any().unbind(),
            Self::Angle(value) => Angle(value).into_pyobject(python)?.into_any().unbind(),
        })
    }
}

/// The mapping a caller passes to [`ShapeGeometry.of`](ShapeGeometry::of), with the two lookups the
/// generated table needs.
struct Values<'py> {
    given: HashMap<String, Bound<'py, PyAny>>,
}

impl<'py> Values<'py> {
    /// The named adjustment as a proportion.
    fn fraction(&self, name: &str, shape: &str) -> PyResult<ooxml::Fraction> {
        let value = self.lookup(name, shape)?;
        value
            .extract::<Fraction>()
            .map(|fraction| fraction.0)
            .map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err(format!(
                    "the `{name}` adjustment of a {shape} is a proportion, so it takes a Fraction"
                ))
            })
    }

    /// The named adjustment as an angle.
    fn angle(&self, name: &str, shape: &str) -> PyResult<ooxml::Angle> {
        let value = self.lookup(name, shape)?;
        value.extract::<Angle>().map(|angle| angle.0).map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "the `{name}` adjustment of a {shape} is an angle, so it takes an Angle"
            ))
        })
    }

    /// The value given for `name`, or a message naming what the shape wanted.
    fn lookup(&self, name: &str, shape: &str) -> PyResult<&Bound<'py, PyAny>> {
        self.given.get(name).ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "a {shape} needs an adjustment called `{name}`; \
                 ShapeGeometry.adjustment_names names them all"
            ))
        })
    }

    /// Refuses a name the shape does not have, rather than ignoring it.
    fn reject_unknown(&self, expected: &[&str], shape: &str) -> PyResult<()> {
        for name in self.given.keys() {
            if !expected.contains(&name.as_str()) {
                return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                    "a {shape} has no adjustment called `{name}`; it has {expected:?}"
                )));
            }
        }
        Ok(())
    }
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
            ) -> PyResult<ooxml::ShapeGeometry> {
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

#[pymethods]
impl ShapeGeometry {
    /// The geometry of one preset shape, with values for the adjustments it carries.
    ///
    /// ```python
    /// ShapeGeometry.of(PresetShapeType.RoundedRectangle, {"corner_radius": Fraction.of(0.25)})
    /// ShapeGeometry.of(PresetShapeType.Arc, {
    ///     "start_angle": Angle.from_degrees(0),
    ///     "end_angle": Angle.from_degrees(90),
    /// })
    /// ShapeGeometry.of(PresetShapeType.Ellipse)   # an ellipse has no adjustments
    /// ```
    ///
    /// Raises `KeyError` for a missing or unrecognised adjustment name, and `TypeError` for a value
    /// of the wrong unit.
    #[staticmethod]
    #[pyo3(signature = (preset, adjustments = None))]
    fn of(
        preset: PresetShapeType,
        adjustments: Option<HashMap<String, Bound<'_, PyAny>>>,
    ) -> PyResult<Self> {
        let values = Values {
            given: adjustments.unwrap_or_default(),
        };
        Self::build(preset.into(), &values).map(Self)
    }

    /// The preset this geometry names.
    #[getter]
    fn preset(&self) -> PyResult<PresetShapeType> {
        let preset = match &self.0 {
            ooxml::ShapeGeometry::Unmodeled(preset) => *preset,
            _ => self
                .parts()
                .map(|(preset, _)| preset)
                .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("unreachable"))?,
        };
        PresetShapeType::from_model(preset)
    }

    /// The adjustments this geometry states, by name.
    #[getter]
    fn adjustments(&self, python: Python<'_>) -> PyResult<HashMap<String, Py<PyAny>>> {
        let Some((_, adjustments)) = self.parts() else {
            return Ok(HashMap::new());
        };
        adjustments
            .into_iter()
            .map(|(name, value)| Ok((name.to_owned(), value.into_py_object(python)?)))
            .collect()
    }

    /// What a preset's adjustments are called — the keys
    /// [`of`](ShapeGeometry::of) expects, in the order the specification lists them.
    #[staticmethod]
    fn adjustment_names(preset: PresetShapeType) -> Vec<&'static str> {
        Self::names(preset.into())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl Geometry {
    /// One of the presets, with its adjustments.
    #[staticmethod]
    fn preset(geometry: ShapeGeometry) -> Self {
        Self(ooxml::Geometry::Preset(geometry.0))
    }

    /// A path the document draws itself.
    #[staticmethod]
    fn custom(geometry: CustomGeometrySpec) -> Self {
        Self(ooxml::Geometry::Custom(geometry.0))
    }

    /// Whatever the shape's placeholder chain says — the shape states no geometry of its own.
    #[staticmethod]
    fn inherited() -> Self {
        Self(ooxml::Geometry::Inherited)
    }

    /// Which kind this is: `"preset"`, `"custom"` or `"inherited"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match &self.0 {
            ooxml::Geometry::Preset(_) => "preset",
            ooxml::Geometry::Custom(_) => "custom",
            ooxml::Geometry::Inherited => "inherited",
        }
    }

    /// The preset geometry, when this is a preset.
    #[getter]
    fn preset_geometry(&self) -> Option<ShapeGeometry> {
        match &self.0 {
            ooxml::Geometry::Preset(geometry) => Some(ShapeGeometry(*geometry)),
            _ => None,
        }
    }

    /// The custom geometry, when this is one.
    #[getter]
    fn custom_geometry(&self) -> Option<CustomGeometrySpec> {
        match &self.0 {
            ooxml::Geometry::Custom(geometry) => Some(CustomGeometrySpec(geometry.clone())),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Adds every class in this module to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ShapeBounds>()?;
    module.add_class::<SlideSize>()?;
    module.add_class::<CellMargins>()?;
    module.add_class::<Position>()?;
    module.add_class::<Size>()?;
    module.add_class::<Transform2D>()?;
    module.add_class::<Point>()?;
    module.add_class::<AdjustCoordinate>()?;
    module.add_class::<AdjustAngle>()?;
    module.add_class::<GuideSpec>()?;
    module.add_class::<GuideContext>()?;
    module.add_class::<Rectangle>()?;
    module.add_class::<DrawCommand>()?;
    module.add_class::<Path2DSpec>()?;
    module.add_class::<AdjustHandle>()?;
    module.add_class::<ConnectionSite>()?;
    module.add_class::<CustomGeometrySpec>()?;
    module.add_class::<ShapeGeometry>()?;
    module.add_class::<Geometry>()?;
    module.add_class::<BoundedAdjustment>()?;
    module.add_class::<AdjustmentSpec>()?;
    module.add_class::<AdjustmentBound>()?;
    module.add_class::<ResolvedPoint>()?;
    module.add_class::<ResolvedRectangle>()?;
    module.add_class::<ResolvedDrawCommand>()?;
    module.add_class::<ResolvedPath>()?;
    module.add_class::<ResolvedConnectionSite>()?;
    module.add_class::<ResolvedAdjustHandle>()?;
    module.add_class::<ResolvedCustomGeometry>()
}
