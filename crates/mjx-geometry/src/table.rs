//! The shape of a preset path table, and the lookup into it.
//!
//! # Why these types exist at all, when `DrawCommand` already does
//!
//! MJXOFF-201 §2 is explicit that [`DrawCommand`] is *the* command vocabulary and a second one must
//! not be invented. It is not invented here. What is here is `DrawCommand` written in a form a
//! `static` can hold: a [`DrawCommand::Guide`](mjx_dml::geometry::AdjustCoordinate::Guide)
//! coordinate owns a `String`, so a table of 187 shapes' paths cannot be a `&'static [DrawCommand]`
//! — it would have to be built at run time, once per process, behind a lock, or rebuilt per frame.
//!
//! So [`PresetPathStep`] has **exactly one variant per `DrawCommand` variant, spelled the same**,
//! and [`PresetPathStep::to_draw_command`] is the total mapping between them. The names match so
//! that a reader comparing the two enumerations can see at a glance that neither has a case the
//! other lacks, and `tests/a_preset_renders_as_itself.rs` asserts the mapping over all seven.
//!
//! # What a row holds, and what it deliberately does not
//!
//! A [`PresetShapeDefinition`] is the shape's `gdLst` (as
//! [`PresetGuide`]s — the *same* type `mjx-ooxml-types` already generates
//! `adjustment_bound_guides_of` in, so MJXOFF-203 emits one kind of guide row rather than two) and
//! its `pathLst`. A [`PresetPath`] carries the path's own coordinate box (`@w`/`@h`), which
//! [`crate::resolve`] applies, and its steps.
//!
//! It does **not** carry `@fill` or `@stroke`, and that is a decision rather than an omission.
//! Nothing consumes them: the display-list seam carries one command list and one
//! [`FillRule`](mjx_scene::FillRule) per outline, and whether a shape is filled or stroked is
//! decided by the *scene builder* from the fragment's decoration, not by the geometry. A flag
//! extracted and read by nobody is the exact defect this loop keeps finding — `SceneMesh::provenance`
//! was written once and read zero times — so the flags arrive with their consumer, in MJXOFF-203,
//! and not before. `arc` is the shape that will need them: its only path is `fill="none"`, and
//! filling an open contour is visibly wrong.

use mjx_dml::geometry::{AdjustAngle, AdjustCoordinate, DrawCommand, Emu, Point};
use mjx_ooxml_core::measure::Angle;
use mjx_ooxml_types::drawingml::{PresetGuide, PresetShapeType};

use crate::seed::SEEDED_SHAPES;
use crate::Derivation;

/// How many angular units the wire uses per degree — `a:gd` angles and `a:arcTo@stAng` are both in
/// 60000ths of a degree (ECMA-376 Part 1 §20.1.10.56 states `cd4` as `5400000`, "equivalent to 90
/// degrees").
pub(crate) const ANGLE_UNITS_PER_DEGREE: f64 = 60_000.0;

/// A coordinate in a preset path: a literal in the shape's own units, or the name of a guide.
///
/// The `static`-holdable form of [`AdjustCoordinate`] (`ST_AdjCoordinate`), and the only reason it
/// is a separate type — see the [module documentation](self).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresetCoordinate {
    /// A literal coordinate, in EMU (`ST_Coordinate`'s integer form).
    Emu(i64),
    /// A reference to a geometry guide, or to a built-in variable, by name (`ST_GeomGuideName`).
    Guide(&'static str),
}

impl PresetCoordinate {
    /// This coordinate as `mjx-dml`'s own [`AdjustCoordinate`].
    #[must_use]
    pub fn to_adjust_coordinate(self) -> AdjustCoordinate {
        match self {
            Self::Emu(emu) => AdjustCoordinate::Emu(Emu::from_emu(emu)),
            Self::Guide(name) => AdjustCoordinate::Guide(name.to_owned()),
        }
    }
}

/// An angle in a preset path: a literal in 60000ths of a degree, or the name of a guide.
///
/// The `static`-holdable form of [`AdjustAngle`] (`ST_AdjAngle`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresetAngle {
    /// A literal angle in the wire scale — 60000ths of a degree, so a right angle is `5_400_000`.
    Native(i64),
    /// A reference to a geometry guide, or to a built-in angular constant (`cd4`, `3cd4`, …), by
    /// name.
    Guide(&'static str),
}

impl PresetAngle {
    /// This angle as `mjx-dml`'s own [`AdjustAngle`].
    #[must_use]
    pub fn to_adjust_angle(self) -> AdjustAngle {
        match self {
            Self::Native(native) => {
                AdjustAngle::Angle(Angle::from_degrees(native as f64 / ANGLE_UNITS_PER_DEGREE))
            }
            Self::Guide(name) => AdjustAngle::Guide(name.to_owned()),
        }
    }
}

/// A point in a preset path (`a:pt`, `CT_AdjPoint2D`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresetPoint {
    /// The horizontal coordinate (`@x`).
    pub x: PresetCoordinate,
    /// The vertical coordinate (`@y`).
    pub y: PresetCoordinate,
}

impl PresetPoint {
    /// A point whose coordinates are both guide references — the overwhelmingly common case in a
    /// preset path, and a constructor so a table row reads as geometry rather than as punctuation.
    #[must_use]
    pub const fn at(x: &'static str, y: &'static str) -> Self {
        Self {
            x: PresetCoordinate::Guide(x),
            y: PresetCoordinate::Guide(y),
        }
    }

    /// This point as `mjx-dml`'s own [`Point`].
    #[must_use]
    pub fn to_point(self) -> Point {
        Point {
            x: self.x.to_adjust_coordinate(),
            y: self.y.to_adjust_coordinate(),
        }
    }
}

/// One drawing step of a preset path.
///
/// **One variant per [`DrawCommand`] variant, spelled the same**, and
/// [`to_draw_command`](Self::to_draw_command) is the whole of the relationship between them. This
/// is not a second command vocabulary; it is the one vocabulary in a form a `static` can hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresetPathStep {
    /// `a:close` — close the current subpath back to its start.
    Close,
    /// `a:moveTo` — start a new subpath at a point, drawing nothing.
    MoveTo(PresetPoint),
    /// `a:lnTo` — draw a straight line to a point.
    LineTo(PresetPoint),
    /// `a:arcTo` — draw an elliptical arc, given the ellipse radii and the start / swing angles.
    ArcTo {
        /// The ellipse's horizontal radius (`@wR`).
        width_radius: PresetCoordinate,
        /// The ellipse's vertical radius (`@hR`).
        height_radius: PresetCoordinate,
        /// The angle the arc starts at (`@stAng`).
        start_angle: PresetAngle,
        /// The angle the arc sweeps through (`@swAng`).
        swing_angle: PresetAngle,
    },
    /// `a:quadBezTo` — a quadratic Bézier curve: one control point, then the end point.
    QuadBezierTo {
        /// The control point.
        control: PresetPoint,
        /// Where the curve ends.
        end: PresetPoint,
    },
    /// `a:cubicBezTo` — a cubic Bézier curve: two control points, then the end point.
    CubicBezierTo {
        /// The control point nearer the start.
        first_control: PresetPoint,
        /// The control point nearer the end.
        second_control: PresetPoint,
        /// Where the curve ends.
        end: PresetPoint,
    },
}

impl PresetPathStep {
    /// This step as the [`DrawCommand`] it *is*.
    ///
    /// Allocating, because `DrawCommand` owns its guide names; a path is a handful of steps and the
    /// allocation is bounded by the path's length, which is why the table is stored this way rather
    /// than the whole of it being built once and cached behind a lock.
    #[must_use]
    pub fn to_draw_command(self) -> DrawCommand {
        match self {
            Self::Close => DrawCommand::Close,
            Self::MoveTo(point) => DrawCommand::MoveTo(point.to_point()),
            Self::LineTo(point) => DrawCommand::LineTo(point.to_point()),
            Self::ArcTo {
                width_radius,
                height_radius,
                start_angle,
                swing_angle,
            } => DrawCommand::ArcTo {
                width_radius: width_radius.to_adjust_coordinate(),
                height_radius: height_radius.to_adjust_coordinate(),
                start_angle: start_angle.to_adjust_angle(),
                swing_angle: swing_angle.to_adjust_angle(),
            },
            Self::QuadBezierTo { control, end } => {
                DrawCommand::QuadBezierTo(control.to_point(), end.to_point())
            }
            Self::CubicBezierTo {
                first_control,
                second_control,
                end,
            } => DrawCommand::CubicBezierTo(
                first_control.to_point(),
                second_control.to_point(),
                end.to_point(),
            ),
        }
    }
}

/// One `a:path` of a preset shape: its own coordinate box, and its ordered steps.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresetPath {
    /// The width of the path's own coordinate box (`@w`), or `None` when the path is written
    /// directly in the shape's own space.
    ///
    /// ECMA-376 Part 1 states it as *"the maximum x coordinate that should be used for within the
    /// path coordinate system"*, so a declared box means every `x` in this path is a fraction of it
    /// rather than a length: [`crate::resolve`] scales by `within.width() / w` instead of by the
    /// shape's extents. Absent, or zero, means no box and no scaling.
    pub width: Option<i64>,
    /// The height of the path's own coordinate box (`@h`). As [`width`](Self::width).
    pub height: Option<i64>,
    /// The steps, in order.
    pub steps: &'static [PresetPathStep],
}

/// One preset shape's geometry: the guide list its coordinates are written against, and its paths.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresetShapeDefinition {
    /// Which shape (`a:prstGeom@prst`).
    pub preset: PresetShapeType,
    /// Where the geometry came from, and how independently — see [`Derivation`].
    pub derivation: Derivation,
    /// The prose this row was written from, and the reasoning that turned it into formulas.
    ///
    /// Present so MJXOFF-203's diff has something to read when the two routes disagree: a mismatch
    /// between a generated row and a hand-written one is only useful if the hand-written one says
    /// what it thought it was doing.
    pub source: &'static str,
    /// The shape's `a:gdLst`, in declaration order — which is evaluation order, and therefore the
    /// whole of the cycle defence (ECMA-376 Part 1 §20.1.9.11).
    pub guides: &'static [PresetGuide],
    /// The shape's `a:pathLst`, in order.
    pub paths: &'static [PresetPath],
}

/// Every preset this build has a path table for, in `PresetShapeType` order.
///
/// Six today, from [`crate::seed`]; 187 once MJXOFF-203 generates them.
#[must_use]
pub fn seeded_shapes() -> &'static [PresetShapeDefinition] {
    SEEDED_SHAPES
}

/// The path table for one preset, or `None` if this build has not got it.
///
/// A linear scan rather than a `match`, because the table is data and MJXOFF-203 will make it
/// generated data: a `match` would have to be regenerated in step with the rows, and a scan over
/// 187 rows costs less than the guide evaluation that follows it.
#[must_use]
pub fn definition_of(preset: PresetShapeType) -> Option<&'static PresetShapeDefinition> {
    SEEDED_SHAPES
        .iter()
        .find(|definition| definition.preset == preset)
}
