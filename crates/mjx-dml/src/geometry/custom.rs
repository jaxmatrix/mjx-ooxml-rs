//! `a:custGeom` (`CT_CustomGeometry2D`) — a shape drawn from an explicit path list rather than a
//! preset. This is what a freeform shape, drawn by hand in PowerPoint, uses; a renderer needs the
//! path list to draw it.
//!
//! The module models the geometry in full, built up in layers: the value types every piece is
//! expressed in (below), the drawing commands (`a:pathLst`), and the auxiliary guide / adjust-handle
//! / connection-site lists. Each complex type is a fidelity wrapper (mirroring
//! [`GeometryGuide`](super::GeometryGuide) and [`Scene3D`](crate::shape3d::Scene3D)): it preserves
//! its attributes and unmodeled children verbatim, so it round-trips byte-for-byte, and reads the
//! modeled facets through typed accessors.
//!
//! # Value types
//!
//! Custom geometry places points and turns with two union-valued measures the rest of the crate does
//! not use elsewhere:
//! - [`AdjustCoordinate`] (`ST_AdjCoordinate`) — a length in EMU **or** a reference to a geometry
//!   guide by name. Point coordinates (`a:pt@x`/`@y`) and rect edges (`a:rect@l`…) are these.
//! - [`AdjustAngle`] (`ST_AdjAngle`) — an angle **or** a guide reference. An arc's start / sweep
//!   (`a:arcTo@stAng`/`@swAng`) and a connection site's outgoing angle (`a:cxn@ang`) are these.
//!
//! Each is the union of a numeric member type and `ST_GeomGuideName`. The numeric member is read
//! whenever the value parses as an integer (the form PowerPoint always writes); anything else is
//! taken as a guide name. Fidelity does not hinge on that split — the containing element preserves
//! the attribute verbatim — so an exotic non-integer numeric literal (a universal measure such as
//! `2.5cm`, which no producer emits here) reading as a guide name changes only the typed view.
//!
//! [`AdjustPoint`] (`a:pt` / `a:pos`, `CT_AdjPoint2D`) is the `(x, y)` each command, handle and
//! connection site is drawn through. Like [`GeometryGuide`](super::GeometryGuide) it is an
//! attribute-only leaf.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml};

use crate::build::{
    angle_to_wire, attr_bool, attr_emu, attr_str, dml_attr, dml_child, dml_element, dml_name,
    fidelity_element_impls, is_dml, push_bool, push_emu,
};
use crate::geometry::formula::ANGLE_UNITS_PER_DEGREE;
use crate::geometry::{Angle, Emu, GeometryGuide};

pub use mjx_ooxml_types::drawingml::PathFillMode;

/// `ST_AdjCoordinate` — an adjustable coordinate: a literal length in EMU, or the name of a geometry
/// guide (`a:gdLst`) whose formula computes it.
///
/// The union is `ST_Coordinate | ST_GeomGuideName`. An integer literal reads as [`Emu`](Self::Emu);
/// any other token reads as a [`Guide`](Self::Guide) reference, resolved by the guide evaluator (a
/// rendering-phase concern, not modeled here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdjustCoordinate {
    /// A literal coordinate, in EMU (`ST_Coordinate`'s integer form).
    Emu(Emu),
    /// A reference to a geometry guide by name (`ST_GeomGuideName`).
    Guide(String),
}

impl AdjustCoordinate {
    /// Reads an `ST_AdjCoordinate` attribute value: an integer EMU literal, otherwise a guide name.
    #[must_use]
    pub fn from_wire(value: &str) -> Self {
        match value.trim().parse::<i64>() {
            Ok(emu) => Self::Emu(Emu::from_emu(emu)),
            Err(_) => Self::Guide(value.to_string()),
        }
    }

    /// This coordinate's wire form: the EMU integer, or the guide name verbatim.
    #[must_use]
    pub fn to_wire(&self) -> String {
        match self {
            Self::Emu(emu) => emu.emu().to_string(),
            Self::Guide(name) => name.clone(),
        }
    }
}

/// `ST_AdjAngle` — an adjustable angle: a literal angle, or the name of a geometry guide.
///
/// The union is `ST_Angle | ST_GeomGuideName`. An integer literal (sixtieths of a thousandth of a
/// degree) reads as [`Angle`](Self::Angle); any other token reads as a [`Guide`](Self::Guide).
#[derive(Debug, Clone, PartialEq)]
pub enum AdjustAngle {
    /// A literal angle (`ST_Angle`, in 60000ths of a degree on the wire).
    Angle(Angle),
    /// A reference to a geometry guide by name (`ST_GeomGuideName`).
    Guide(String),
}

impl AdjustAngle {
    /// Reads an `ST_AdjAngle` attribute value: an integer angle literal, otherwise a guide name.
    #[must_use]
    pub fn from_wire(value: &str) -> Self {
        match value.trim().parse::<i64>() {
            Ok(native) => Self::Angle(Angle::from_degrees(native as f64 / ANGLE_UNITS_PER_DEGREE)),
            Err(_) => Self::Guide(value.to_string()),
        }
    }

    /// This angle's wire form: the native 60000ths-of-a-degree integer, or the guide name verbatim.
    #[must_use]
    pub fn to_wire(&self) -> String {
        match self {
            Self::Angle(angle) => angle_to_wire(*angle),
            Self::Guide(name) => name.clone(),
        }
    }
}

/// `a:pt` / `a:pos` (`CT_AdjPoint2D`) — a point an `x` and a `y` [`AdjustCoordinate`] place.
///
/// A path command draws through `a:pt`s; an adjust handle and a connection site position themselves
/// with an `a:pos`. Only the local name differs, so one type reads and builds both. Like
/// [`GeometryGuide`](super::GeometryGuide) it is an attribute-only leaf: its attributes, self-closing
/// flag, and any unexpected children re-emit verbatim, so it round-trips byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustPoint {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl AdjustPoint {
    /// Builds a point `<a:{local} x=".." y=".."/>` — `local` is `pt` for a path command, `pos` for an
    /// adjust handle or connection site.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        local: &str,
        x: &AdjustCoordinate,
        y: &AdjustCoordinate,
    ) -> Self {
        Self {
            name: dml_name(interner, local),
            attributes: vec![
                dml_attr(interner, "x", &x.to_wire()),
                dml_attr(interner, "y", &y.to_wire()),
            ],
            children: Vec::new(),
            empty: true,
        }
    }

    /// The point's `x` coordinate, or `None` if the (schema-required) attribute is absent.
    #[must_use]
    pub fn x(&self, interner: &Interner) -> Option<AdjustCoordinate> {
        attr_str(&self.attributes, interner, "x").map(AdjustCoordinate::from_wire)
    }

    /// The point's `y` coordinate, or `None` if the (schema-required) attribute is absent.
    #[must_use]
    pub fn y(&self, interner: &Interner) -> Option<AdjustCoordinate> {
        attr_str(&self.attributes, interner, "y").map(AdjustCoordinate::from_wire)
    }

    /// The point's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// This point as an interner-free [`Point`]. The `x`/`y` are schema-required; an absent one reads
    /// as the origin coordinate (`0` EMU) rather than failing, so a malformed point still resolves.
    #[must_use]
    pub fn value(&self, interner: &Interner) -> Point {
        Point {
            x: self.x(interner).unwrap_or(Point::ORIGIN_COORD),
            y: self.y(interner).unwrap_or(Point::ORIGIN_COORD),
        }
    }
}

fidelity_element_impls!(AdjustPoint);

/// An interner-free `(x, y)` in a custom geometry's coordinate space — the resolved form of an
/// [`AdjustPoint`] a path command, adjust handle, or connection site is drawn through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    /// The horizontal coordinate (`a:pt@x`).
    pub x: AdjustCoordinate,
    /// The vertical coordinate (`a:pt@y`).
    pub y: AdjustCoordinate,
}

impl Point {
    /// The `0` EMU coordinate an absent (schema-required) `x` / `y` reads as.
    const ORIGIN_COORD: AdjustCoordinate = AdjustCoordinate::Emu(Emu::from_emu(0));

    /// A point at literal EMU coordinates.
    #[must_use]
    pub fn from_emu(x: i64, y: i64) -> Self {
        Self {
            x: AdjustCoordinate::Emu(Emu::from_emu(x)),
            y: AdjustCoordinate::Emu(Emu::from_emu(y)),
        }
    }
}

/// The zero angle an absent (schema-required) `a:arcTo` angle reads as.
const ZERO_ANGLE: AdjustAngle = AdjustAngle::Angle(Angle::from_radians(0.0));

/// One drawing command of a [`Path2D`] — the interner-free, ordered instruction a renderer follows to
/// trace the outline. Mirrors the `a:path` choice group (`CT_Path2D`), one variant per element.
///
/// Coordinates are [`Point`]s / [`AdjustCoordinate`]s, so each may be a literal or a guide reference.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    /// `a:close` — close the current subpath back to its start.
    Close,
    /// `a:moveTo` — start a new subpath at a point, drawing nothing.
    MoveTo(Point),
    /// `a:lnTo` — draw a straight line to a point.
    LineTo(Point),
    /// `a:arcTo` — draw an elliptical arc, given the ellipse radii and the start / swing angles.
    ArcTo {
        /// The ellipse's horizontal radius (`@wR`).
        width_radius: AdjustCoordinate,
        /// The ellipse's vertical radius (`@hR`).
        height_radius: AdjustCoordinate,
        /// The angle the arc starts at (`@stAng`).
        start_angle: AdjustAngle,
        /// The angle the arc sweeps through (`@swAng`).
        swing_angle: AdjustAngle,
    },
    /// `a:quadBezTo` — a quadratic Bézier curve: one control point, then the end point.
    QuadBezierTo(Point, Point),
    /// `a:cubicBezTo` — a cubic Bézier curve: two control points, then the end point.
    CubicBezierTo(Point, Point, Point),
}

/// `a:path` (`CT_Path2D`) — one subpath's drawing commands, over an optional local coordinate box
/// (`@w`/`@h`) and fill / stroke flags.
///
/// A fidelity wrapper: the commands and flags are read typed (through [`commands`](Self::commands) and
/// the attribute accessors), while any unmodeled child re-emits verbatim, so the path round-trips
/// byte-for-byte. Every flag follows the crate rule — an unstated one reads `None`, distinct from the
/// schema default (`fill`=`norm`, `stroke`=`extrusionOk`=`true`, `w`=`h`=`0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path2D {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Path2D {
    /// The width of the path's own coordinate box (`@w`, EMU; schema default `0`).
    #[must_use]
    pub fn width(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "w")
    }

    /// The height of the path's own coordinate box (`@h`, EMU; schema default `0`).
    #[must_use]
    pub fn height(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "h")
    }

    /// How the path is filled (`@fill`; schema default `norm`).
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<PathFillMode> {
        attr_str(&self.attributes, interner, "fill").and_then(PathFillMode::from_wire)
    }

    /// Whether the path is stroked (`@stroke`; schema default `true`).
    #[must_use]
    pub fn stroke(&self, interner: &Interner) -> Option<bool> {
        attr_bool(&self.attributes, interner, "stroke")
    }

    /// Whether the path may be extruded in 3-D (`@extrusionOk`; schema default `true`).
    #[must_use]
    pub fn extrusion_ok(&self, interner: &Interner) -> Option<bool> {
        attr_bool(&self.attributes, interner, "extrusionOk")
    }

    /// The path's drawing commands, in order. Unmodeled children are skipped in this view (they still
    /// round-trip); a command with a missing point / radius reads that value as the origin / zero.
    #[must_use]
    pub fn commands(&self, interner: &Interner) -> Vec<DrawCommand> {
        self.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) if is_dml(&element.name, interner) => {
                    read_command(element, interner)
                }
                _ => None,
            })
            .collect()
    }

    /// This path as an interner-free [`Path2DSpec`].
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> Path2DSpec {
        Path2DSpec {
            width: self.width(interner),
            height: self.height(interner),
            fill: self.fill(interner),
            stroke: self.stroke(interner),
            extrusion_ok: self.extrusion_ok(interner),
            commands: self.commands(interner),
        }
    }
}

fidelity_element_impls!(Path2D);

/// An interner-free description of one `a:path`: its flags and its ordered [`DrawCommand`]s. Convert
/// with [`Path2D::spec`] / [`Path2DSpec::to_path_2d`]. Rebuilding from a spec drops any opaque
/// unmodeled children a read [`Path2D`] preserved.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path2DSpec {
    /// The width of the path's coordinate box (`@w`).
    pub width: Option<Emu>,
    /// The height of the path's coordinate box (`@h`).
    pub height: Option<Emu>,
    /// How the path is filled (`@fill`).
    pub fill: Option<PathFillMode>,
    /// Whether the path is stroked (`@stroke`).
    pub stroke: Option<bool>,
    /// Whether the path may be extruded (`@extrusionOk`).
    pub extrusion_ok: Option<bool>,
    /// The path's drawing commands, in order.
    pub commands: Vec<DrawCommand>,
}

impl Path2DSpec {
    /// Builds the fidelity [`Path2D`] for this description, interning against `interner`. Attributes
    /// are written in `CT_Path2D`'s schema order (`w`, `h`, `fill`, `stroke`, `extrusionOk`).
    #[must_use]
    pub fn to_path_2d(&self, interner: &mut Interner) -> Path2D {
        let element = build_path(interner, self);
        Path2D::from_xml(&element, interner).expect("built path is well-formed")
    }
}

/// `a:pathLst` (`CT_Path2DList`) — the ordered list of `a:path`s a custom geometry is drawn from.
///
/// A fidelity wrapper: the `a:path`s are read typed; anything else re-emits verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path2DList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Path2DList {
    /// Builds an `a:pathLst` from `paths` in order (self-closing `<a:pathLst/>` when empty).
    #[must_use]
    pub fn new(interner: &mut Interner, paths: &[Path2DSpec]) -> Self {
        let children = paths
            .iter()
            .map(|spec| RawNode::Element(build_path(interner, spec)))
            .collect();
        let element = dml_element(interner, "pathLst", Vec::new(), children);
        Path2DList::from_xml(&element, interner).expect("built pathLst is well-formed")
    }

    /// The typed paths (`a:path`), in order.
    #[must_use]
    pub fn paths(&self, interner: &Interner) -> Vec<Path2D> {
        self.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element)
                    if is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "path" =>
                {
                    Path2D::from_xml(element, interner).ok()
                }
                _ => None,
            })
            .collect()
    }

    /// The paths as interner-free [`Path2DSpec`]s, in order.
    #[must_use]
    pub fn specs(&self, interner: &Interner) -> Vec<Path2DSpec> {
        self.paths(interner)
            .iter()
            .map(|path| path.spec(interner))
            .collect()
    }
}

fidelity_element_impls!(Path2DList);

// ---------------------------------------------------------------------------------------------
// Reading & building path commands
// ---------------------------------------------------------------------------------------------

/// Reads one `a:path` child element as a [`DrawCommand`], or `None` if its local name is not a command.
fn read_command(element: &RawElement, interner: &Interner) -> Option<DrawCommand> {
    Some(match interner.resolve(element.name.local) {
        "close" => DrawCommand::Close,
        "moveTo" => DrawCommand::MoveTo(nth_point(element, interner, 0)),
        "lnTo" => DrawCommand::LineTo(nth_point(element, interner, 0)),
        "arcTo" => DrawCommand::ArcTo {
            width_radius: adjust_coordinate(element, interner, "wR"),
            height_radius: adjust_coordinate(element, interner, "hR"),
            start_angle: adjust_angle(element, interner, "stAng"),
            swing_angle: adjust_angle(element, interner, "swAng"),
        },
        "quadBezTo" => DrawCommand::QuadBezierTo(
            nth_point(element, interner, 0),
            nth_point(element, interner, 1),
        ),
        "cubicBezTo" => DrawCommand::CubicBezierTo(
            nth_point(element, interner, 0),
            nth_point(element, interner, 1),
            nth_point(element, interner, 2),
        ),
        _ => return None,
    })
}

/// The `n`th `a:pt` child of a command element, as a [`Point`]; the origin if there are fewer than
/// `n + 1` (a malformed command must still resolve rather than fail the read).
fn nth_point(element: &RawElement, interner: &Interner, n: usize) -> Point {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(child)
                if is_dml(&child.name, interner) && interner.resolve(child.name.local) == "pt" =>
            {
                Some(child)
            }
            _ => None,
        })
        .nth(n)
        .map(|pt| read_point(pt, interner))
        .unwrap_or_else(|| Point::from_emu(0, 0))
}

/// Reads an `a:pt` element's `x`/`y` as a [`Point`] (an absent coordinate reads as the origin).
fn read_point(element: &RawElement, interner: &Interner) -> Point {
    Point {
        x: adjust_coordinate(element, interner, "x"),
        y: adjust_coordinate(element, interner, "y"),
    }
}

/// Reads a named `ST_AdjCoordinate` attribute (an absent one reads as `0` EMU).
fn adjust_coordinate(element: &RawElement, interner: &Interner, name: &str) -> AdjustCoordinate {
    attr_str(&element.attributes, interner, name)
        .map(AdjustCoordinate::from_wire)
        .unwrap_or(Point::ORIGIN_COORD)
}

/// Reads a named `ST_AdjAngle` attribute (an absent one reads as `0`).
fn adjust_angle(element: &RawElement, interner: &Interner, name: &str) -> AdjustAngle {
    attr_str(&element.attributes, interner, name)
        .map(AdjustAngle::from_wire)
        .unwrap_or(ZERO_ANGLE)
}

/// Builds an `a:path` element from a spec.
fn build_path(interner: &mut Interner, spec: &Path2DSpec) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "w", spec.width);
    push_emu(&mut attrs, interner, "h", spec.height);
    if let Some(fill) = spec.fill {
        attrs.push(dml_attr(interner, "fill", fill.to_wire()));
    }
    push_bool(&mut attrs, interner, "stroke", spec.stroke);
    push_bool(&mut attrs, interner, "extrusionOk", spec.extrusion_ok);
    let children = spec
        .commands
        .iter()
        .map(|command| RawNode::Element(build_command(interner, command)))
        .collect();
    dml_element(interner, "path", attrs, children)
}

/// Builds one `a:path` command element from a [`DrawCommand`].
fn build_command(interner: &mut Interner, command: &DrawCommand) -> RawElement {
    match command {
        DrawCommand::Close => dml_element(interner, "close", Vec::new(), Vec::new()),
        DrawCommand::MoveTo(point) => {
            let pt = RawNode::Element(build_point(interner, point));
            dml_element(interner, "moveTo", Vec::new(), vec![pt])
        }
        DrawCommand::LineTo(point) => {
            let pt = RawNode::Element(build_point(interner, point));
            dml_element(interner, "lnTo", Vec::new(), vec![pt])
        }
        DrawCommand::ArcTo {
            width_radius,
            height_radius,
            start_angle,
            swing_angle,
        } => {
            let attrs = vec![
                dml_attr(interner, "wR", &width_radius.to_wire()),
                dml_attr(interner, "hR", &height_radius.to_wire()),
                dml_attr(interner, "stAng", &start_angle.to_wire()),
                dml_attr(interner, "swAng", &swing_angle.to_wire()),
            ];
            dml_element(interner, "arcTo", attrs, Vec::new())
        }
        DrawCommand::QuadBezierTo(control, end) => {
            let pts = vec![
                RawNode::Element(build_point(interner, control)),
                RawNode::Element(build_point(interner, end)),
            ];
            dml_element(interner, "quadBezTo", Vec::new(), pts)
        }
        DrawCommand::CubicBezierTo(control1, control2, end) => {
            let pts = vec![
                RawNode::Element(build_point(interner, control1)),
                RawNode::Element(build_point(interner, control2)),
                RawNode::Element(build_point(interner, end)),
            ];
            dml_element(interner, "cubicBezTo", Vec::new(), pts)
        }
    }
}

/// Builds an `<a:pt x=".." y=".."/>` element from a [`Point`].
fn build_point(interner: &mut Interner, point: &Point) -> RawElement {
    build_named_point(interner, "pt", point)
}

/// Builds a point element with the given local name (`pt` for a path command, `pos` for a handle /
/// connection site) from a [`Point`].
fn build_named_point(interner: &mut Interner, local: &str, point: &Point) -> RawElement {
    let attrs = vec![
        dml_attr(interner, "x", &point.x.to_wire()),
        dml_attr(interner, "y", &point.y.to_wire()),
    ];
    dml_element(interner, local, attrs, Vec::new())
}

// ---------------------------------------------------------------------------------------------
// Auxiliary lists — guides, adjust handles, connection sites, the text rectangle (interner-free)
// ---------------------------------------------------------------------------------------------

/// One guide of an `a:avLst` / `a:gdLst` (`a:gd`, `CT_GeomGuide`), interner-free: a `name` and the
/// `fmla` formula that computes it. The formula language (`val`, `*/`, `+-`, `pin`, …) is the guide
/// evaluator's concern, kept verbatim here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuideSpec {
    /// The guide's name (`@name`), referenced by an adjust handle or a coordinate.
    pub name: String,
    /// The guide's formula (`@fmla`), e.g. `val 25000` or `*/ w 1 2`.
    pub formula: String,
}

/// `a:rect` (`CT_GeomRect`) — the rectangle text is laid out in, its four edges given as
/// [`AdjustCoordinate`]s. Interner-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rectangle {
    /// The left edge (`@l`).
    pub left: AdjustCoordinate,
    /// The top edge (`@t`).
    pub top: AdjustCoordinate,
    /// The right edge (`@r`).
    pub right: AdjustCoordinate,
    /// The bottom edge (`@b`).
    pub bottom: AdjustCoordinate,
}

/// `a:cxn` (`CT_ConnectionSite`) — a point a connector can attach to, with the angle a connector
/// leaves it at. Interner-free.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionSite {
    /// The angle a connector leaves the site at (`@ang`, required).
    pub angle: AdjustAngle,
    /// Where the site sits (`a:pos`).
    pub position: Point,
}

/// `a:ahXY` / `a:ahPolar` (`CT_XYAdjustHandle` / `CT_PolarAdjustHandle`) — a handle a user drags to
/// change the shape, bound to a guide and clamped to a range. Interner-free.
///
/// Each handle names the guide(s) it drives (`gdRef*`) and the min/max the dragged value is clamped
/// to; every one is optional. A Cartesian handle ([`Xy`](Self::Xy)) drives an `x`/`y` guide pair; a
/// polar handle ([`Polar`](Self::Polar)) drives a radius/angle pair.
#[derive(Debug, Clone, PartialEq)]
pub enum AdjustHandle {
    /// A Cartesian (`x`/`y`) adjust handle (`a:ahXY`).
    Xy {
        /// The handle's position (`a:pos`).
        position: Point,
        /// The guide the horizontal drag drives (`@gdRefX`).
        guide_ref_x: Option<String>,
        /// The minimum horizontal value (`@minX`).
        min_x: Option<AdjustCoordinate>,
        /// The maximum horizontal value (`@maxX`).
        max_x: Option<AdjustCoordinate>,
        /// The guide the vertical drag drives (`@gdRefY`).
        guide_ref_y: Option<String>,
        /// The minimum vertical value (`@minY`).
        min_y: Option<AdjustCoordinate>,
        /// The maximum vertical value (`@maxY`).
        max_y: Option<AdjustCoordinate>,
    },
    /// A polar (radius/angle) adjust handle (`a:ahPolar`).
    Polar {
        /// The handle's position (`a:pos`).
        position: Point,
        /// The guide the radial drag drives (`@gdRefR`).
        guide_ref_radius: Option<String>,
        /// The minimum radius (`@minR`).
        min_radius: Option<AdjustCoordinate>,
        /// The maximum radius (`@maxR`).
        max_radius: Option<AdjustCoordinate>,
        /// The guide the angular drag drives (`@gdRefAng`).
        guide_ref_angle: Option<String>,
        /// The minimum angle (`@minAng`).
        min_angle: Option<AdjustAngle>,
        /// The maximum angle (`@maxAng`).
        max_angle: Option<AdjustAngle>,
    },
}

// ---------------------------------------------------------------------------------------------
// CustomGeometry — the fidelity wrapper over `a:custGeom`
// ---------------------------------------------------------------------------------------------

/// `a:custGeom` (`CT_CustomGeometry2D`) — a shape's geometry given as an explicit path list rather
/// than a preset, with the guides, adjust handles, connection sites, and text rectangle that go with
/// it.
///
/// A fidelity wrapper: every modeled child is read typed (through the accessors); an unmodeled child
/// (an `extLst`, say) re-emits verbatim, so the element round-trips byte-for-byte. The children a
/// schema places, in order: `avLst`, `gdLst`, `ahLst`, `cxnLst`, `rect`, and the required `pathLst`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGeometry {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl CustomGeometry {
    /// The adjust-value guides (`a:avLst` `a:gd`s) — the shape's adjustable seeds.
    #[must_use]
    pub fn adjust_values(&self, interner: &Interner) -> Vec<GuideSpec> {
        read_guides(&self.children, interner, "avLst")
    }

    /// The computed guides (`a:gdLst` `a:gd`s) — formulas derived from the adjust values and the
    /// shape's size.
    #[must_use]
    pub fn guides(&self, interner: &Interner) -> Vec<GuideSpec> {
        read_guides(&self.children, interner, "gdLst")
    }

    /// The adjust handles (`a:ahLst`), in order.
    #[must_use]
    pub fn adjust_handles(&self, interner: &Interner) -> Vec<AdjustHandle> {
        let Some(list) = dml_child(&self.children, interner, "ahLst") else {
            return Vec::new();
        };
        list.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) if is_dml(&element.name, interner) => {
                    match interner.resolve(element.name.local) {
                        "ahXY" => Some(read_xy_handle(element, interner)),
                        "ahPolar" => Some(read_polar_handle(element, interner)),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect()
    }

    /// The connection sites (`a:cxnLst`), in order.
    #[must_use]
    pub fn connection_sites(&self, interner: &Interner) -> Vec<ConnectionSite> {
        let Some(list) = dml_child(&self.children, interner, "cxnLst") else {
            return Vec::new();
        };
        list.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element)
                    if is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "cxn" =>
                {
                    Some(ConnectionSite {
                        angle: adjust_angle(element, interner, "ang"),
                        position: child_position(element, interner),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// The text rectangle (`a:rect`), or `None` if absent.
    #[must_use]
    pub fn text_rectangle(&self, interner: &Interner) -> Option<Rectangle> {
        let rect = dml_child(&self.children, interner, "rect")?;
        Some(Rectangle {
            left: adjust_coordinate(rect, interner, "l"),
            top: adjust_coordinate(rect, interner, "t"),
            right: adjust_coordinate(rect, interner, "r"),
            bottom: adjust_coordinate(rect, interner, "b"),
        })
    }

    /// The path list (`a:pathLst`) as typed [`Path2DSpec`]s. The schema requires the element; an
    /// absent one (a malformed geometry) reads as an empty list rather than failing.
    #[must_use]
    pub fn paths(&self, interner: &Interner) -> Vec<Path2DSpec> {
        dml_child(&self.children, interner, "pathLst")
            .and_then(|list| Path2DList::from_xml(list, interner).ok())
            .map(|list| list.specs(interner))
            .unwrap_or_default()
    }

    /// This geometry as an interner-free [`CustomGeometrySpec`]. Rebuilding from the spec drops any
    /// opaque unmodeled child (e.g. an `extLst`) this wrapper preserved.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> CustomGeometrySpec {
        CustomGeometrySpec {
            adjust_values: self.adjust_values(interner),
            guides: self.guides(interner),
            adjust_handles: self.adjust_handles(interner),
            connection_sites: self.connection_sites(interner),
            text_rectangle: self.text_rectangle(interner),
            paths: self.paths(interner),
        }
    }
}

fidelity_element_impls!(CustomGeometry);

/// An interner-free description of an `a:custGeom`: its guides, adjust handles, connection sites, text
/// rectangle, and path list. Convert with [`CustomGeometry::spec`] /
/// [`CustomGeometrySpec::to_custom_geometry`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomGeometrySpec {
    /// The adjust-value guides (`a:avLst`).
    pub adjust_values: Vec<GuideSpec>,
    /// The computed guides (`a:gdLst`).
    pub guides: Vec<GuideSpec>,
    /// The adjust handles (`a:ahLst`).
    pub adjust_handles: Vec<AdjustHandle>,
    /// The connection sites (`a:cxnLst`).
    pub connection_sites: Vec<ConnectionSite>,
    /// The text rectangle (`a:rect`).
    pub text_rectangle: Option<Rectangle>,
    /// The path list (`a:pathLst`).
    pub paths: Vec<Path2DSpec>,
}

impl CustomGeometrySpec {
    /// Builds the fidelity [`CustomGeometry`] for this description, interning against `interner`.
    /// Children are written in `CT_CustomGeometry2D`'s schema order; an empty auxiliary list is
    /// omitted, but the required `a:pathLst` is always written (even when it holds no paths).
    #[must_use]
    pub fn to_custom_geometry(&self, interner: &mut Interner) -> CustomGeometry {
        let mut children = Vec::new();
        if !self.adjust_values.is_empty() {
            children.push(RawNode::Element(build_guide_list(
                interner,
                "avLst",
                &self.adjust_values,
            )));
        }
        if !self.guides.is_empty() {
            children.push(RawNode::Element(build_guide_list(
                interner,
                "gdLst",
                &self.guides,
            )));
        }
        if !self.adjust_handles.is_empty() {
            let handles = self
                .adjust_handles
                .iter()
                .map(|handle| RawNode::Element(build_handle(interner, handle)))
                .collect();
            children.push(RawNode::Element(dml_element(
                interner,
                "ahLst",
                Vec::new(),
                handles,
            )));
        }
        if !self.connection_sites.is_empty() {
            let sites = self
                .connection_sites
                .iter()
                .map(|site| RawNode::Element(build_connection_site(interner, site)))
                .collect();
            children.push(RawNode::Element(dml_element(
                interner,
                "cxnLst",
                Vec::new(),
                sites,
            )));
        }
        if let Some(rect) = &self.text_rectangle {
            children.push(RawNode::Element(build_rect(interner, rect)));
        }
        let path_list = Path2DList::new(interner, &self.paths).to_xml(interner);
        children.push(RawNode::Element(path_list));

        let element = dml_element(interner, "custGeom", Vec::new(), children);
        CustomGeometry::from_xml(&element, interner).expect("built custGeom is well-formed")
    }
}

// ---------------------------------------------------------------------------------------------
// Reading & building the auxiliary lists
// ---------------------------------------------------------------------------------------------

/// The `a:gd`s of a named guide list child (`avLst` / `gdLst`) as [`GuideSpec`]s; empty if the list is
/// absent. A guide missing its `name` or `fmla` is skipped.
fn read_guides(children: &[RawNode], interner: &Interner, local: &str) -> Vec<GuideSpec> {
    let Some(list) = dml_child(children, interner, local) else {
        return Vec::new();
    };
    list.children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element)
                if is_dml(&element.name, interner)
                    && interner.resolve(element.name.local) == "gd" =>
            {
                Some(GuideSpec {
                    name: attr_str(&element.attributes, interner, "name")?.to_owned(),
                    formula: attr_str(&element.attributes, interner, "fmla")?.to_owned(),
                })
            }
            _ => None,
        })
        .collect()
}

/// The `a:pos` of a handle / connection site as a [`Point`] (the origin if it is absent).
fn child_position(element: &RawElement, interner: &Interner) -> Point {
    dml_child(&element.children, interner, "pos")
        .map(|pos| read_point(pos, interner))
        .unwrap_or_else(|| Point::from_emu(0, 0))
}

/// Reads an `a:ahXY` (`CT_XYAdjustHandle`).
fn read_xy_handle(element: &RawElement, interner: &Interner) -> AdjustHandle {
    AdjustHandle::Xy {
        position: child_position(element, interner),
        guide_ref_x: opt_str(element, interner, "gdRefX"),
        min_x: opt_coordinate(element, interner, "minX"),
        max_x: opt_coordinate(element, interner, "maxX"),
        guide_ref_y: opt_str(element, interner, "gdRefY"),
        min_y: opt_coordinate(element, interner, "minY"),
        max_y: opt_coordinate(element, interner, "maxY"),
    }
}

/// Reads an `a:ahPolar` (`CT_PolarAdjustHandle`).
fn read_polar_handle(element: &RawElement, interner: &Interner) -> AdjustHandle {
    AdjustHandle::Polar {
        position: child_position(element, interner),
        guide_ref_radius: opt_str(element, interner, "gdRefR"),
        min_radius: opt_coordinate(element, interner, "minR"),
        max_radius: opt_coordinate(element, interner, "maxR"),
        guide_ref_angle: opt_str(element, interner, "gdRefAng"),
        min_angle: opt_angle(element, interner, "minAng"),
        max_angle: opt_angle(element, interner, "maxAng"),
    }
}

/// An optional string attribute, owned.
fn opt_str(element: &RawElement, interner: &Interner, name: &str) -> Option<String> {
    attr_str(&element.attributes, interner, name).map(str::to_owned)
}

/// An optional `ST_AdjCoordinate` attribute.
fn opt_coordinate(
    element: &RawElement,
    interner: &Interner,
    name: &str,
) -> Option<AdjustCoordinate> {
    attr_str(&element.attributes, interner, name).map(AdjustCoordinate::from_wire)
}

/// An optional `ST_AdjAngle` attribute.
fn opt_angle(element: &RawElement, interner: &Interner, name: &str) -> Option<AdjustAngle> {
    attr_str(&element.attributes, interner, name).map(AdjustAngle::from_wire)
}

/// Builds a named guide list (`avLst` / `gdLst`) from its guides.
fn build_guide_list(interner: &mut Interner, local: &str, guides: &[GuideSpec]) -> RawElement {
    let children = guides
        .iter()
        .map(|guide| {
            let gd = GeometryGuide::new(interner, &guide.name, &guide.formula);
            RawNode::Element(gd.to_xml(interner))
        })
        .collect();
    dml_element(interner, local, Vec::new(), children)
}

/// Builds an `a:ahXY` / `a:ahPolar` from an [`AdjustHandle`], writing only the attributes that are set,
/// in schema order.
fn build_handle(interner: &mut Interner, handle: &AdjustHandle) -> RawElement {
    match handle {
        AdjustHandle::Xy {
            position,
            guide_ref_x,
            min_x,
            max_x,
            guide_ref_y,
            min_y,
            max_y,
        } => {
            let mut attrs = Vec::new();
            push_opt_str(&mut attrs, interner, "gdRefX", guide_ref_x);
            push_opt_coordinate(&mut attrs, interner, "minX", min_x);
            push_opt_coordinate(&mut attrs, interner, "maxX", max_x);
            push_opt_str(&mut attrs, interner, "gdRefY", guide_ref_y);
            push_opt_coordinate(&mut attrs, interner, "minY", min_y);
            push_opt_coordinate(&mut attrs, interner, "maxY", max_y);
            let pos = RawNode::Element(build_named_point(interner, "pos", position));
            dml_element(interner, "ahXY", attrs, vec![pos])
        }
        AdjustHandle::Polar {
            position,
            guide_ref_radius,
            min_radius,
            max_radius,
            guide_ref_angle,
            min_angle,
            max_angle,
        } => {
            let mut attrs = Vec::new();
            push_opt_str(&mut attrs, interner, "gdRefR", guide_ref_radius);
            push_opt_coordinate(&mut attrs, interner, "minR", min_radius);
            push_opt_coordinate(&mut attrs, interner, "maxR", max_radius);
            push_opt_str(&mut attrs, interner, "gdRefAng", guide_ref_angle);
            push_opt_angle(&mut attrs, interner, "minAng", min_angle);
            push_opt_angle(&mut attrs, interner, "maxAng", max_angle);
            let pos = RawNode::Element(build_named_point(interner, "pos", position));
            dml_element(interner, "ahPolar", attrs, vec![pos])
        }
    }
}

/// Builds an `a:cxn` from a [`ConnectionSite`].
fn build_connection_site(interner: &mut Interner, site: &ConnectionSite) -> RawElement {
    let attrs = vec![dml_attr(interner, "ang", &site.angle.to_wire())];
    let pos = RawNode::Element(build_named_point(interner, "pos", &site.position));
    dml_element(interner, "cxn", attrs, vec![pos])
}

/// Builds an `a:rect` from a [`Rectangle`] (all four edges required).
fn build_rect(interner: &mut Interner, rect: &Rectangle) -> RawElement {
    let attrs = vec![
        dml_attr(interner, "l", &rect.left.to_wire()),
        dml_attr(interner, "t", &rect.top.to_wire()),
        dml_attr(interner, "r", &rect.right.to_wire()),
        dml_attr(interner, "b", &rect.bottom.to_wire()),
    ];
    dml_element(interner, "rect", attrs, Vec::new())
}

/// Pushes an optional string attribute when set.
fn push_opt_str(
    attrs: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    name: &str,
    value: &Option<String>,
) {
    if let Some(value) = value {
        attrs.push(dml_attr(interner, name, value));
    }
}

/// Pushes an optional `ST_AdjCoordinate` attribute when set.
fn push_opt_coordinate(
    attrs: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    name: &str,
    value: &Option<AdjustCoordinate>,
) {
    if let Some(value) = value {
        attrs.push(dml_attr(interner, name, &value.to_wire()));
    }
}

/// Pushes an optional `ST_AdjAngle` attribute when set.
fn push_opt_angle(
    attrs: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    name: &str,
    value: &Option<AdjustAngle>,
) {
    if let Some(value) = value {
        attrs.push(dml_attr(interner, name, &value.to_wire()));
    }
}
