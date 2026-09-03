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

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, Interner, RawAttribute, RawElement,
    RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::support::OnOff;

use crate::build::{dml_child, dml_element, dml_name, fidelity_element_impls, is_dml};
use crate::codec::{AngleOrGuideName, EmuCoordinate, EmuOrGuideName};
use crate::geometry::{Angle, Emu, GeometryGuide};

pub use mjx_ooxml_types::drawingml::PathFillMode;

// ---------------------------------------------------------------------------------------------
// The attribute faces
// ---------------------------------------------------------------------------------------------
//
// Custom geometry is the crate's densest *value-projection* tier: an `a:pt`, an `a:arcTo`, an
// `a:rect`, an `a:cxn` and the two adjust handles are read out of elements the crate has no type
// for, and are written by building a fresh element. So each declares once, generically over its
// attribute container, and serves both directions — `{ attributes: &element.attributes }` to read,
// which copies nothing, and `{ attributes: Vec::new() }` to build the vector a new element owns.

/// `a:pt` / `a:pos` (`CT_AdjPoint2D`) — the attribute face of a point.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "x", codec = EmuOrGuideName, accessor = x, required))]
#[xml(attribute(local = "y", codec = EmuOrGuideName, accessor = y, required))]
struct PointAttributes<A> {
    attributes: A,
}

/// `a:arcTo` (`CT_Path2DArcTo`) — the attribute face of an elliptical arc command.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "wR", codec = EmuOrGuideName, accessor = width_radius, required))]
#[xml(attribute(local = "hR", codec = EmuOrGuideName, accessor = height_radius, required))]
#[xml(attribute(local = "stAng", codec = AngleOrGuideName, accessor = start_angle, required))]
#[xml(attribute(local = "swAng", codec = AngleOrGuideName, accessor = swing_angle, required))]
struct ArcAttributes<A> {
    attributes: A,
}

/// `a:path` (`CT_Path2D`) — the attribute face of one subpath.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "w", codec = EmuCoordinate, accessor = width))]
#[xml(attribute(local = "h", codec = EmuCoordinate, accessor = height))]
#[xml(attribute(local = "fill", codec = Enumeration<PathFillMode>, accessor = fill))]
#[xml(attribute(local = "stroke", codec = OnOff, accessor = stroke))]
#[xml(attribute(local = "extrusionOk", codec = OnOff, accessor = extrusion_ok))]
struct PathAttributes<A> {
    attributes: A,
}

/// `a:rect` (`CT_GeomRect`) — the attribute face of the text rectangle's four edges.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "l", codec = EmuOrGuideName, accessor = left, required))]
#[xml(attribute(local = "t", codec = EmuOrGuideName, accessor = top, required))]
#[xml(attribute(local = "r", codec = EmuOrGuideName, accessor = right, required))]
#[xml(attribute(local = "b", codec = EmuOrGuideName, accessor = bottom, required))]
struct RectangleAttributes<A> {
    attributes: A,
}

/// `a:cxn` (`CT_ConnectionSite`) — the attribute face of a connection site.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "ang", codec = AngleOrGuideName, accessor = angle, required))]
struct ConnectionSiteAttributes<A> {
    attributes: A,
}

/// `a:ahXY` (`CT_XYAdjustHandle`) — the attribute face of a Cartesian adjust handle.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "gdRefX", codec = Text, accessor = guide_ref_x))]
#[xml(attribute(local = "minX", codec = EmuOrGuideName, accessor = min_x))]
#[xml(attribute(local = "maxX", codec = EmuOrGuideName, accessor = max_x))]
#[xml(attribute(local = "gdRefY", codec = Text, accessor = guide_ref_y))]
#[xml(attribute(local = "minY", codec = EmuOrGuideName, accessor = min_y))]
#[xml(attribute(local = "maxY", codec = EmuOrGuideName, accessor = max_y))]
struct XyHandleAttributes<A> {
    attributes: A,
}

/// `a:ahPolar` (`CT_PolarAdjustHandle`) — the attribute face of a polar adjust handle.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "gdRefR", codec = Text, accessor = guide_ref_radius))]
#[xml(attribute(local = "minR", codec = EmuOrGuideName, accessor = min_radius))]
#[xml(attribute(local = "maxR", codec = EmuOrGuideName, accessor = max_radius))]
#[xml(attribute(local = "gdRefAng", codec = Text, accessor = guide_ref_angle))]
#[xml(attribute(local = "minAng", codec = AngleOrGuideName, accessor = min_angle))]
#[xml(attribute(local = "maxAng", codec = AngleOrGuideName, accessor = max_angle))]
struct PolarHandleAttributes<A> {
    attributes: A,
}

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
    ///
    /// One call to [`EmuOrGuideName`], the codec that is this union's single wire mapping — the same
    /// one every declared `ST_AdjCoordinate` attribute reads through. It rejects nothing (a token
    /// that is not an integer is a guide reference), so the fallback below is unreachable and is
    /// written as a value rather than a panic.
    #[must_use]
    pub fn from_wire(value: &str) -> Self {
        EmuOrGuideName::decode(Cow::Borrowed(value))
            .unwrap_or_else(|_| Self::Guide(value.to_owned()))
    }

    /// This coordinate's wire form: the EMU integer, or the guide name verbatim.
    #[must_use]
    pub fn to_wire(&self) -> String {
        EmuOrGuideName::encode(self).into_owned()
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
    ///
    /// One call to [`AngleOrGuideName`], this union's single wire mapping; as with
    /// [`AdjustCoordinate::from_wire`] it rejects nothing, so the fallback is unreachable.
    #[must_use]
    pub fn from_wire(value: &str) -> Self {
        AngleOrGuideName::decode(Cow::Borrowed(value))
            .unwrap_or_else(|_| Self::Guide(value.to_owned()))
    }

    /// This angle's wire form: the native 60000ths-of-a-degree integer, or the guide name verbatim.
    #[must_use]
    pub fn to_wire(&self) -> String {
        AngleOrGuideName::encode(self).into_owned()
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
            attributes: point_attributes(
                interner,
                &Point {
                    x: x.clone(),
                    y: y.clone(),
                },
            ),
            children: Vec::new(),
            empty: true,
        }
    }

    /// The point's `x` coordinate (`@x`).
    ///
    /// # Errors
    /// [`AttributeError::Missing`] if the schema-required attribute is absent, or another
    /// [`AttributeError`] if its bytes are not readable as text.
    pub fn x(&self, interner: &Interner) -> Result<AdjustCoordinate, AttributeError> {
        PointAttributes {
            attributes: &self.attributes,
        }
        .x(interner)
    }

    /// The point's `y` coordinate (`@y`).
    ///
    /// # Errors
    /// As [`x`](Self::x).
    pub fn y(&self, interner: &Interner) -> Result<AdjustCoordinate, AttributeError> {
        PointAttributes {
            attributes: &self.attributes,
        }
        .y(interner)
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
        read_point_attributes(&self.attributes, interner)
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
    /// This path's attribute face, borrowed — the one declaration every accessor below reads
    /// through, and the one [`build_path`] writes through.
    fn attribute_face(&self) -> PathAttributes<&[RawAttribute]> {
        PathAttributes {
            attributes: &self.attributes,
        }
    }

    /// The width of the path's own coordinate box (`@w`, EMU; schema default `0`), or `None` if
    /// unstated.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but is not a whole number of EMU.
    pub fn width(&self, interner: &Interner) -> Result<Option<Emu>, AttributeError> {
        self.attribute_face().width(interner)
    }

    /// The height of the path's own coordinate box (`@h`, EMU; schema default `0`), or `None` if
    /// unstated.
    ///
    /// # Errors
    /// As [`width`](Self::width).
    pub fn height(&self, interner: &Interner) -> Result<Option<Emu>, AttributeError> {
        self.attribute_face().height(interner)
    }

    /// How the path is filled (`@fill`; schema default `norm`), or `None` if unstated.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but is not an `ST_PathFillMode`.
    pub fn fill(&self, interner: &Interner) -> Result<Option<PathFillMode>, AttributeError> {
        self.attribute_face().fill(interner)
    }

    /// Whether the path is stroked (`@stroke`; schema default `true`), or `None` if unstated.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but is not an `ST_OnOff`.
    pub fn stroke(&self, interner: &Interner) -> Result<Option<bool>, AttributeError> {
        self.attribute_face().stroke(interner)
    }

    /// Whether the path may be extruded in 3-D (`@extrusionOk`; schema default `true`), or `None`
    /// if unstated.
    ///
    /// # Errors
    /// As [`stroke`](Self::stroke).
    pub fn extrusion_ok(&self, interner: &Interner) -> Result<Option<bool>, AttributeError> {
        self.attribute_face().extrusion_ok(interner)
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
        // A spec is a value description: an attribute it cannot represent — absent, or malformed —
        // is simply not part of the description, which is what `None` says here.
        Path2DSpec {
            width: self.width(interner).ok().flatten(),
            height: self.height(interner).ok().flatten(),
            fill: self.fill(interner).ok().flatten(),
            stroke: self.stroke(interner).ok().flatten(),
            extrusion_ok: self.extrusion_ok(interner).ok().flatten(),
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
        "arcTo" => {
            let arc = ArcAttributes {
                attributes: &element.attributes,
            };
            // Every attribute is schema-required; a malformed arc still resolves, to the same
            // origin/zero an absent one does, because a command that cannot be read must not stop
            // the rest of the path being read.
            DrawCommand::ArcTo {
                width_radius: arc.width_radius(interner).unwrap_or(Point::ORIGIN_COORD),
                height_radius: arc.height_radius(interner).unwrap_or(Point::ORIGIN_COORD),
                start_angle: arc.start_angle(interner).unwrap_or(ZERO_ANGLE),
                swing_angle: arc.swing_angle(interner).unwrap_or(ZERO_ANGLE),
            }
        }
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
    read_point_attributes(&element.attributes, interner)
}

/// Reads a point's `@x` / `@y` out of an attribute vector. Both are schema-required; one that is
/// absent or malformed reads as the origin coordinate, so a malformed point still resolves.
fn read_point_attributes(attributes: &[RawAttribute], interner: &Interner) -> Point {
    let point = PointAttributes { attributes };
    Point {
        x: point.x(interner).unwrap_or(Point::ORIGIN_COORD),
        y: point.y(interner).unwrap_or(Point::ORIGIN_COORD),
    }
}

/// Builds the `@x` / `@y` attribute vector of a fresh point element, in schema order.
fn point_attributes(interner: &mut Interner, point: &Point) -> Vec<RawAttribute> {
    let mut attributes = PointAttributes {
        attributes: Vec::new(),
    };
    attributes.set_x(interner, &point.x);
    attributes.set_y(interner, &point.y);
    attributes.attributes
}

/// Builds an `a:path` element from a spec.
fn build_path(interner: &mut Interner, spec: &Path2DSpec) -> RawElement {
    let mut path = PathAttributes {
        attributes: Vec::new(),
    };
    path.set_width(interner, spec.width);
    path.set_height(interner, spec.height);
    path.set_fill(interner, spec.fill);
    path.set_stroke(interner, spec.stroke);
    path.set_extrusion_ok(interner, spec.extrusion_ok);
    let attrs = path.attributes;
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
            let mut arc = ArcAttributes {
                attributes: Vec::new(),
            };
            arc.set_width_radius(interner, width_radius);
            arc.set_height_radius(interner, height_radius);
            arc.set_start_angle(interner, start_angle);
            arc.set_swing_angle(interner, swing_angle);
            dml_element(interner, "arcTo", arc.attributes, Vec::new())
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
    let attrs = point_attributes(interner, point);
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
                        angle: ConnectionSiteAttributes {
                            attributes: &element.attributes,
                        }
                        .angle(interner)
                        .unwrap_or(ZERO_ANGLE),
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
        let edges = RectangleAttributes {
            attributes: &rect.attributes,
        };
        // All four edges are schema-required; one absent or malformed reads as the origin, so a
        // malformed rectangle still resolves rather than making the whole geometry unreadable.
        Some(Rectangle {
            left: edges.left(interner).unwrap_or(Point::ORIGIN_COORD),
            top: edges.top(interner).unwrap_or(Point::ORIGIN_COORD),
            right: edges.right(interner).unwrap_or(Point::ORIGIN_COORD),
            bottom: edges.bottom(interner).unwrap_or(Point::ORIGIN_COORD),
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
                // Read through the type that declares `a:gd`'s two attributes, rather than
                // re-declaring them here: a guide is two small attributes and no children, and
                // both values are copied into the owned `GuideSpec` on the next line anyway.
                let guide = GeometryGuide::from_xml(element, interner).ok()?;
                Some(GuideSpec {
                    name: guide.name(interner).ok()?.into_owned(),
                    formula: guide.formula(interner).ok()?.into_owned(),
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

/// Reads an `a:ahXY` (`CT_XYAdjustHandle`). Every attribute is optional, and one that is present
/// but unreadable is reported the same way an absent one is — a handle is a hint about how the
/// shape may be dragged, so a malformed bound is a bound the model simply does not know.
fn read_xy_handle(element: &RawElement, interner: &Interner) -> AdjustHandle {
    let handle = XyHandleAttributes {
        attributes: &element.attributes,
    };
    AdjustHandle::Xy {
        position: child_position(element, interner),
        guide_ref_x: owned(handle.guide_ref_x(interner)),
        min_x: handle.min_x(interner).ok().flatten(),
        max_x: handle.max_x(interner).ok().flatten(),
        guide_ref_y: owned(handle.guide_ref_y(interner)),
        min_y: handle.min_y(interner).ok().flatten(),
        max_y: handle.max_y(interner).ok().flatten(),
    }
}

/// Reads an `a:ahPolar` (`CT_PolarAdjustHandle`); see [`read_xy_handle`].
fn read_polar_handle(element: &RawElement, interner: &Interner) -> AdjustHandle {
    let handle = PolarHandleAttributes {
        attributes: &element.attributes,
    };
    AdjustHandle::Polar {
        position: child_position(element, interner),
        guide_ref_radius: owned(handle.guide_ref_radius(interner)),
        min_radius: handle.min_radius(interner).ok().flatten(),
        max_radius: handle.max_radius(interner).ok().flatten(),
        guide_ref_angle: owned(handle.guide_ref_angle(interner)),
        min_angle: handle.min_angle(interner).ok().flatten(),
        max_angle: handle.max_angle(interner).ok().flatten(),
    }
}

/// A text-valued optional attribute as an owned `String` — an [`AdjustHandle`] is interner-free, so
/// the borrowed read cannot outlive the call.
fn owned(read: Result<Option<Cow<'_, str>>, AttributeError>) -> Option<String> {
    read.ok().flatten().map(Cow::into_owned)
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
            let mut handle = XyHandleAttributes {
                attributes: Vec::new(),
            };
            handle.set_guide_ref_x(interner, guide_ref_x.as_deref());
            handle.set_min_x(interner, min_x.as_ref());
            handle.set_max_x(interner, max_x.as_ref());
            handle.set_guide_ref_y(interner, guide_ref_y.as_deref());
            handle.set_min_y(interner, min_y.as_ref());
            handle.set_max_y(interner, max_y.as_ref());
            let pos = RawNode::Element(build_named_point(interner, "pos", position));
            dml_element(interner, "ahXY", handle.attributes, vec![pos])
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
            let mut handle = PolarHandleAttributes {
                attributes: Vec::new(),
            };
            handle.set_guide_ref_radius(interner, guide_ref_radius.as_deref());
            handle.set_min_radius(interner, min_radius.as_ref());
            handle.set_max_radius(interner, max_radius.as_ref());
            handle.set_guide_ref_angle(interner, guide_ref_angle.as_deref());
            handle.set_min_angle(interner, min_angle.as_ref());
            handle.set_max_angle(interner, max_angle.as_ref());
            let pos = RawNode::Element(build_named_point(interner, "pos", position));
            dml_element(interner, "ahPolar", handle.attributes, vec![pos])
        }
    }
}

/// Builds an `a:cxn` from a [`ConnectionSite`].
fn build_connection_site(interner: &mut Interner, site: &ConnectionSite) -> RawElement {
    let mut attributes = ConnectionSiteAttributes {
        attributes: Vec::new(),
    };
    attributes.set_angle(interner, &site.angle);
    let pos = RawNode::Element(build_named_point(interner, "pos", &site.position));
    dml_element(interner, "cxn", attributes.attributes, vec![pos])
}

/// Builds an `a:rect` from a [`Rectangle`] (all four edges required).
fn build_rect(interner: &mut Interner, rect: &Rectangle) -> RawElement {
    let mut edges = RectangleAttributes {
        attributes: Vec::new(),
    };
    edges.set_left(interner, &rect.left);
    edges.set_top(interner, &rect.top);
    edges.set_right(interner, &rect.right);
    edges.set_bottom(interner, &rect.bottom);
    dml_element(interner, "rect", edges.attributes, Vec::new())
}
