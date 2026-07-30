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

use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::{angle_to_wire, attr_str, dml_attr, dml_name, fidelity_element_impls};
use crate::geometry::{Angle, Emu};

pub use mjx_ooxml_types::drawingml::PathFillMode;

/// The native wire scale of `ST_Angle` — sixtieths of a thousandth of a degree (`21_600_000` is a
/// full turn).
const ANGLE_UNITS_PER_DEGREE: f64 = 60_000.0;

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
}

fidelity_element_impls!(AdjustPoint);
