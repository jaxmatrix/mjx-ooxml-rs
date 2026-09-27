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
//! [`PresetTextRectangle`] and [`PresetConnectionSite`] are the same construction applied to
//! `mjx-dml`'s [`Rectangle`] and [`ConnectionSite`], for the same reason and with the same total
//! mapping ([`to_rectangle`](PresetTextRectangle::to_rectangle),
//! [`to_connection_site`](PresetConnectionSite::to_connection_site)). Neither introduces a
//! coordinate type: both are written in the [`PresetCoordinate`], [`PresetAngle`] and
//! [`PresetPoint`] this module already had, and both resolve through the *same* guide environment
//! and the *same* affine map the paths do — see [`crate::resolve`].
//!
//! # What a row holds, and what it deliberately does not
//!
//! A [`PresetShapeDefinition`] is the shape's `gdLst` (as
//! [`PresetGuide`]s — the *same* type `mjx-ooxml-types` already generates
//! `adjustment_bound_guides_of` in, so MJXOFF-203 emits one kind of guide row rather than two), its
//! `a:rect`, its `a:cxnLst` and its `pathLst`. A [`PresetPath`] carries the path's own coordinate
//! box (`@w`/`@h`), which [`crate::resolve`] applies, and its steps.
//!
//! It **does** carry `@fill`, `@stroke` and `@extrusionOk`, and MJXOFF-202 deliberately did not —
//! its reason was that nothing consumed them, and *"a flag extracted and read by nobody is the
//! exact defect this loop keeps finding"*. That reason has expired rather than been overruled:
//! MJXOFF-203 brought the consumer with the flags. [`crate::resolve::contours_of_definition`] is a
//! per-`a:path` answer that carries each contour's own treatment, and
//! [`crate::outline_of_definition`] **acts** on the pair — a contour that is neither filled nor
//! stroked draws nothing at all and is left out of the single command list the display-list seam
//! takes. Exactly one path in the whole of `presetShapeDefinitions.xml` is in that state
//! (`flowChartMultidocument`'s third), which is why the rule is gated by name rather than by
//! sampling.
//!
//! `@extrusionOk` is the one flag with no consumer here, and it is carried rather than dropped for
//! a stated reason: MJXOFF-201 §7 puts 3-D — `a:sp3d`, bevels and extrusion — outside this epic
//! entirely, and re-opening the extractor later to fetch one attribute costs more than emitting it
//! now. **Its reader is named in MJXOFF-211**, not left to be discovered.

use mjx_dml::geometry::{
    AdjustAngle, AdjustCoordinate, ConnectionSite, DrawCommand, Emu, Point, Rectangle,
};
use mjx_ooxml_core::measure::Angle;
use mjx_ooxml_types::drawingml::{PathFillMode, PresetGuide, PresetShapeType};

use crate::generated::GENERATED_SHAPES;
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

    /// A point whose coordinates are stated one at a time — a literal, a guide, or one of each.
    ///
    /// The general constructor [`at`](Self::at) is the shorthand for. The generated table writes
    /// this form wherever either coordinate is a literal, which is about one point in eight.
    #[must_use]
    pub const fn new(x: PresetCoordinate, y: PresetCoordinate) -> Self {
        Self { x, y }
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

/// One preset shape's text rectangle (`a:rect`, `CT_GeomRect`) — where text goes **inside** the
/// shape, rather than against the box the shape is drawn in.
///
/// The `static`-holdable form of [`mjx_dml::geometry::Rectangle`], for the same reason
/// [`PresetPathStep`] is `DrawCommand`'s: an [`AdjustCoordinate`] owns its guide name.
/// [`to_rectangle`](Self::to_rectangle) is the whole of the relationship between them, and there is
/// no second rectangle vocabulary in this crate.
///
/// **Why it matters, and why its absence is not the same as its default.** A rounded rectangle's
/// text starts `29.289 %` of the corner radius in from the corner; a chevron's starts past the
/// notch; a callout's sits in the body and not in the tail. A renderer that laid text against the
/// shape's *bounding box* instead would put every one of them in the wrong place and still draw
/// text, which is why [`crate::TextRectangle`] makes "this shape declares none" a different answer
/// from "this shape declares one that has no value here" — and neither of them the bounding box
/// unless a caller asks for it by name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresetTextRectangle {
    /// The left edge (`@l`).
    pub left: PresetCoordinate,
    /// The top edge (`@t`).
    pub top: PresetCoordinate,
    /// The right edge (`@r`).
    pub right: PresetCoordinate,
    /// The bottom edge (`@b`).
    pub bottom: PresetCoordinate,
}

impl PresetTextRectangle {
    /// This text rectangle as `mjx-dml`'s own [`Rectangle`].
    #[must_use]
    pub fn to_rectangle(self) -> Rectangle {
        Rectangle {
            left: self.left.to_adjust_coordinate(),
            top: self.top.to_adjust_coordinate(),
            right: self.right.to_adjust_coordinate(),
            bottom: self.bottom.to_adjust_coordinate(),
        }
    }
}

/// One place a connector can attach to a preset shape (`a:cxn`, `CT_ConnectionSite`): a point on
/// the outline, and the angle a connector leaves it at.
///
/// The `static`-holdable form of [`mjx_dml::geometry::ConnectionSite`], and
/// [`to_connection_site`](Self::to_connection_site) is the whole of the relationship.
///
/// The **angle** is what makes a site more than a point. An elbow connector leaving the top of a
/// box must go *up* before it turns, and a curved one must leave along its tangent; both read
/// `@ang`, in the wire scale of 60000ths of a degree, clockwise from the positive `x` axis. Two
/// hundred and eight of the table's 856 sites state it as a literal and 648 name a guide — usually
/// one of the circle constants `cd4`, `cd2`, `3cd4` — so both arms of [`PresetAngle`] are exercised
/// by real data rather than only by a probe. The **positions** are not so evenly split: all 1 712
/// of their coordinates are guide names and not one is a literal, which
/// `crates/mjx-geometry/tests/a_connector_lands_on_the_outline.rs` asserts rather than assumes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresetConnectionSite {
    /// The angle a connector leaves the site at (`@ang`).
    pub angle: PresetAngle,
    /// Where the site sits, in the shape's own space (`a:pos`).
    pub position: PresetPoint,
}

impl PresetConnectionSite {
    /// This site as `mjx-dml`'s own [`ConnectionSite`].
    #[must_use]
    pub fn to_connection_site(self) -> ConnectionSite {
        ConnectionSite {
            angle: self.angle.to_adjust_angle(),
            position: self.position.to_point(),
        }
    }
}

/// One `a:path` of a preset shape: its own coordinate box, the treatment it declares, and its
/// ordered steps.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PresetPath {
    /// The width of the path's own coordinate box (`@w`), or `None` when the path is written
    /// directly in the shape's own space.
    ///
    /// ECMA-376 Part 1 states it as *"the maximum x coordinate that should be used for within the
    /// path coordinate system"*, so a declared box means every `x` in this path is a fraction of it
    /// rather than a length: [`crate::resolve`] scales by `within.width() / w` instead of by the
    /// shape's extents. Absent, or zero, means no box and no scaling.
    ///
    /// Thirty-one of the generated shapes declare one, and the boxes are small — `2`, `5`, `10`,
    /// `21600` — so a coordinate inside such a path is a *proportion*. A guide name may still
    /// appear there, but only ever an **angular** one (`cd2`, `cd4`, `3cd4`): an angle is
    /// dimensionless and means the same in either space, while a length guide is computed in EMU
    /// and would be meaningless against a box of ten. That is a property of the file, and
    /// `xtask`'s extraction gate asserts it rather than assuming it.
    pub width: Option<i64>,
    /// The height of the path's own coordinate box (`@h`). As [`width`](Self::width).
    pub height: Option<i64>,
    /// How this path is filled (`@fill`; the schema default is
    /// [`Normal`](PathFillMode::Normal)).
    ///
    /// [`PathFillMode::None`] is not "no fill colour" but *"do not fill this contour"* — the
    /// stroked outline of a shape whose filled body is a different path. `arc` is the clearest
    /// case: its only stroked path is `fill="none"` and filling an open contour draws a chord
    /// nothing asked for.
    pub fill: PathFillMode,
    /// Whether this path is stroked (`@stroke`; the schema default is `true`).
    ///
    /// `false` is the *body* half of the same pairing: a contour that carries the shape's fill and
    /// must not be outlined, because the outline belongs to a sibling path.
    pub stroke: bool,
    /// Whether this path may be extruded in 3-D (`@extrusionOk`; the schema default is `true`).
    ///
    /// Carried, and read by nobody in this crate — the one flag in the row that is not acted on
    /// here, for the reason the [module documentation](self) gives: MJXOFF-201 §7 puts 3-D outside
    /// this epic and MJXOFF-211 names who reads it.
    pub extrusion_ok: bool,
    /// The steps, in order.
    pub steps: &'static [PresetPathStep],
}

impl PresetPath {
    /// Whether this path contributes a filled region (`@fill` is anything but
    /// [`None`](PathFillMode::None)).
    #[must_use]
    pub fn is_filled(self) -> bool {
        self.fill != PathFillMode::None
    }

    /// Whether this path draws nothing at all — neither filled nor stroked.
    ///
    /// True for exactly one path of one shape in ECMA-376's geometry file
    /// (`flowChartMultidocument`'s third), and [`crate::outline_of_definition`] leaves such a
    /// contour out of the single command list the display-list seam takes. That is the whole of
    /// what makes [`fill`](Self::fill) and [`stroke`](Self::stroke) fields something acts on rather
    /// than fields something merely stores.
    #[must_use]
    pub fn draws_nothing(self) -> bool {
        !self.is_filled() && !self.stroke
    }
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
    /// The shape's `a:avLst`, in declaration order: every adjustable value it defines, with the
    /// `val N` seed the file gives it.
    ///
    /// **Not the same list as [`adjustments_of`](mjx_ooxml_types::drawingml::adjustments_of)**, and
    /// the difference is load-bearing rather than cosmetic. That table holds the *user-facing*
    /// adjustments — the `a:avLst` entries some `a:ahLst` handle references — and deliberately drops
    /// the rest as constants. But a dropped one is still a name the shape's own `gdLst` reads:
    /// `pentagon`'s first guide is `*/ wd2 hf 100000` and `hf` is an `avLst` entry no handle points
    /// at, so a guide environment seeded from the handled subset alone cannot evaluate the shape at
    /// all. Nine shapes are in that position — the five- to ten-sided regular polygons and stars,
    /// plus `wedgeRoundRectCallout`'s third adjustment — and they resolve because this field carries
    /// the whole list.
    ///
    /// Evaluated before [`guides`](Self::guides), with any `a:avLst` override from the document
    /// applied on top; see [`crate::resolve`].
    pub adjustment_values: &'static [PresetGuide],
    /// The shape's `a:gdLst`, in declaration order — which is evaluation order, and therefore the
    /// whole of the cycle defence (ECMA-376 Part 1 §20.1.9.11).
    pub guides: &'static [PresetGuide],
    /// The shape's `a:rect` — where text goes inside it — or `None` for the five presets that
    /// declare none.
    ///
    /// `None` is *"this shape says nothing about where its text goes"* and is a different fact from
    /// *"this shape says, and the answer has no value at these adjustments"*; the two are told
    /// apart by [`crate::TextRectangle`] and never by an empty rectangle. The five are `chartPlus`,
    /// `chartStar`, `chartX`, `line` and `lineInv` — three tick marks and two bare lines, none of
    /// which is a shape text is laid inside.
    pub text_rectangle: Option<PresetTextRectangle>,
    /// The shape's `a:cxnLst`, in order.
    ///
    /// Empty for thirteen presets, and the emptiness is meaningful rather than missing: the four
    /// `bentConnector*`, the four `curvedConnector*` and `straightConnector1` are **themselves**
    /// connectors, and a connector has nothing to connect to. The other four are the three
    /// `chart*` marks and `funnel`.
    pub connection_sites: &'static [PresetConnectionSite],
    /// The shape's `a:pathLst`, in order.
    pub paths: &'static [PresetPath],
}

/// Every preset this build has a path table for, in `presetShapeDefinitions.xml` order.
///
/// **186 shapes**, mechanically extracted from ECMA-376's own geometry file by
/// `cargo run -p xtask -- codegen` — not the six hand-transcribed rows of [`crate::seed`], which
/// stayed behind as the differential reference the extraction was checked against.
///
/// 186 and not 187, and that is the file's arithmetic rather than this crate's: `ST_ShapeType`
/// declares 187 values and `presetShapeDefinitions.xml` defines geometry for 186 of them. The one
/// it omits is named in [`PRESETS_WITHOUT_GEOMETRY`](crate::PRESETS_WITHOUT_GEOMETRY).
#[must_use]
pub fn seeded_shapes() -> &'static [PresetShapeDefinition] {
    GENERATED_SHAPES
}

/// The path table for one preset, or `None` if this build has not got it.
///
/// A linear scan rather than a `match`, because the table is generated data: a `match` would have
/// to be regenerated in step with the rows, and a scan over 186 rows costs less than the guide
/// evaluation that follows it.
#[must_use]
pub fn definition_of(preset: PresetShapeType) -> Option<&'static PresetShapeDefinition> {
    GENERATED_SHAPES
        .iter()
        .find(|definition| definition.preset == preset)
}
