//! Six preset shapes, transcribed by hand — the data this crate's machine was built and gated
//! against before `presetShapeDefinitions.xml` was available.
//!
//! # Why these are hand-written, and what that is *for*
//!
//! MJXOFF-203 extracts all 187 presets mechanically out of `presetShapeDefinitions.xml`. Its
//! strongest available gate is diffing that extraction against a transcription that did **not**
//! come from the same file: two routes to one answer. That is the whole purpose of this module, and
//! it is worth exactly as much as its independence, so every row records how independent it really
//! was in [`Derivation`] and says so in prose in its `source` field.
//!
//! **Read that honestly before relying on it.** Nothing here was copied out of the XML, but the
//! workspace already contains two tables that *were* generated from it —
//! [`adjustments_of`](mjx_ooxml_types::drawingml::adjustments_of) (defaults and domains) and
//! [`adjustment_bound_guides_of`](mjx_ooxml_types::drawingml::adjustment_bound_guides_of) (the
//! guides a domain depends on) — and four of the six rows below take a constant or a formula from
//! them rather than inventing one. Those four are marked
//! [`ConstantsFromTheGeneratedTables`](Derivation::ConstantsFromTheGeneratedTables). The *paths*
//! are independent in all six.
//!
//! One further caveat that no enumeration can carry: `ellipse` is drawn as four 90° `a:arcTo`s
//! starting from the left-middle point, which is the only structure DrawingML's own arc semantics
//! make natural, and it is very likely the structure the XML uses too. Its independence is
//! therefore *structural coincidence*, not a second measurement — and MJXOFF-203 should treat
//! agreement there as weaker evidence than agreement on `rightArrow`.
//!
//! # How to read a row
//!
//! Coordinates are in the shape's own space, where `l`/`t` are `0`, `r`/`b` are `w`/`h` and `hc`,
//! `vc`, `ss`, `wd2`, `hd2`, `cd2`, `cd4` and `3cd4` are the predefined guides of ECMA-376 Part 1
//! §20.1.10.56. Formulas are the language of §20.1.9.11. `y` increases **downward**, so a positive
//! angle turns clockwise — which is why `cd2` (180°) is the *left* of a shape and `3cd4` (270°) is
//! the top.
//!
//! # Arc angles are true angles, not parametric ones
//!
//! Every `a:arcTo` below relies on it and it is not obvious, so it is stated once here and
//! implemented once in [`crate::arc`]. `a:arcTo@stAng` is the angle a ray from the ellipse's centre
//! makes, not the parameter of `(a·cos t, b·sin t)`. The spec's own `arc` shape is the proof, and
//! `mjx-dml` already quotes it on
//! [`GuideOperator::CosineArcTangent`](mjx_dml::geometry::GuideOperator::CosineArcTangent): the
//! start point is `hc + cat2 wd2 ht1 wt1` where `wt1 = sin wd2 stAng` and `ht1 = cos hd2 stAng`,
//! which is `hc + (w/2)·cos(atan2((w/2)·sin θ, (h/2)·cos θ))`. That lands the arc's centre exactly
//! on `(hc, vc)` **only** under the true-angle reading; under the parametric one the ellipse would
//! be off-centre by however much the shape is not square. Every row here is written on that basis,
//! and [`crate::arc`] converts.

use mjx_ooxml_types::drawingml::{PresetGuide, PresetShapeType};

use crate::table::{
    PresetAngle, PresetCoordinate, PresetPath, PresetPathStep, PresetPoint, PresetShapeDefinition,
};
use crate::Derivation;

/// A right angle in the wire scale, `cd4`'s value — 60000ths of a degree.
const QUARTER_TURN: i64 = 5_400_000;

/// The steps of `rect`.
const RECTANGLE_STEPS: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
    PresetPathStep::LineTo(PresetPoint::at("r", "t")),
    PresetPathStep::LineTo(PresetPoint::at("r", "b")),
    PresetPathStep::LineTo(PresetPoint::at("l", "b")),
    PresetPathStep::Close,
];

/// One 90° arc of `ellipse`, sweeping clockwise from `start`.
const fn ellipse_quadrant(start: PresetAngle) -> PresetPathStep {
    PresetPathStep::ArcTo {
        width_radius: PresetCoordinate::Guide("wd2"),
        height_radius: PresetCoordinate::Guide("hd2"),
        start_angle: start,
        swing_angle: PresetAngle::Native(QUARTER_TURN),
    }
}

/// The steps of `ellipse`: from the left-middle point, four quadrants clockwise.
const ELLIPSE_STEPS: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("l", "vc")),
    ellipse_quadrant(PresetAngle::Guide("cd2")),
    ellipse_quadrant(PresetAngle::Guide("3cd4")),
    ellipse_quadrant(PresetAngle::Native(0)),
    ellipse_quadrant(PresetAngle::Guide("cd4")),
    PresetPathStep::Close,
];

/// The guides of `triangle`: pin the adjustment into its domain, then place the apex.
const TRIANGLE_GUIDES: &[PresetGuide] = &[
    PresetGuide {
        wire_name: "a",
        formula: "pin 0 adj 100000",
    },
    PresetGuide {
        wire_name: "x1",
        formula: "*/ w a 100000",
    },
];

/// The steps of `triangle`: bottom-left, apex, bottom-right.
const TRIANGLE_STEPS: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("l", "b")),
    PresetPathStep::LineTo(PresetPoint::at("x1", "t")),
    PresetPathStep::LineTo(PresetPoint::at("r", "b")),
    PresetPathStep::Close,
];

/// The guides of `roundRect`: the corner radius, and the two far edges it insets.
const ROUNDED_RECTANGLE_GUIDES: &[PresetGuide] = &[
    PresetGuide {
        wire_name: "a",
        formula: "pin 0 adj 50000",
    },
    PresetGuide {
        wire_name: "x1",
        formula: "*/ ss a 100000",
    },
    PresetGuide {
        wire_name: "x2",
        formula: "+- r 0 x1",
    },
    PresetGuide {
        wire_name: "y2",
        formula: "+- b 0 x1",
    },
];

/// One rounded corner: a quarter circle of radius `x1`, swept clockwise from `start`.
const fn rounded_corner(start: PresetAngle) -> PresetPathStep {
    PresetPathStep::ArcTo {
        width_radius: PresetCoordinate::Guide("x1"),
        height_radius: PresetCoordinate::Guide("x1"),
        start_angle: start,
        swing_angle: PresetAngle::Native(QUARTER_TURN),
    }
}

/// The steps of `roundRect`. The left edge is drawn by the `close`, which is why there are three
/// `lnTo`s and four corners rather than four of each.
const ROUNDED_RECTANGLE_STEPS: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("l", "x1")),
    rounded_corner(PresetAngle::Guide("cd2")),
    PresetPathStep::LineTo(PresetPoint::at("x2", "t")),
    rounded_corner(PresetAngle::Guide("3cd4")),
    PresetPathStep::LineTo(PresetPoint::at("r", "y2")),
    rounded_corner(PresetAngle::Native(0)),
    PresetPathStep::LineTo(PresetPoint::at("x1", "b")),
    rounded_corner(PresetAngle::Guide("cd4")),
    PresetPathStep::Close,
];

/// The guides of `rightArrow`: the head's length, then the shaft's two edges.
const RIGHT_ARROW_GUIDES: &[PresetGuide] = &[
    PresetGuide {
        wire_name: "maxAdj2",
        formula: "*/ 100000 w ss",
    },
    PresetGuide {
        wire_name: "a1",
        formula: "pin 0 adj1 100000",
    },
    PresetGuide {
        wire_name: "a2",
        formula: "pin 0 adj2 maxAdj2",
    },
    PresetGuide {
        wire_name: "dx1",
        formula: "*/ ss a2 100000",
    },
    PresetGuide {
        wire_name: "x1",
        formula: "+- r 0 dx1",
    },
    PresetGuide {
        wire_name: "dy1",
        formula: "*/ h a1 200000",
    },
    PresetGuide {
        wire_name: "y1",
        formula: "+- vc 0 dy1",
    },
    PresetGuide {
        wire_name: "y2",
        formula: "+- vc dy1 0",
    },
];

/// The steps of `rightArrow`: seven points, clockwise from the shaft's top-left.
const RIGHT_ARROW_STEPS: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("l", "y1")),
    PresetPathStep::LineTo(PresetPoint::at("x1", "y1")),
    PresetPathStep::LineTo(PresetPoint::at("x1", "t")),
    PresetPathStep::LineTo(PresetPoint::at("r", "vc")),
    PresetPathStep::LineTo(PresetPoint::at("x1", "b")),
    PresetPathStep::LineTo(PresetPoint::at("x1", "y2")),
    PresetPathStep::LineTo(PresetPoint::at("l", "y2")),
    PresetPathStep::Close,
];

/// The guides of `pie`: the two angles, the swing between them, and the point on the rim the swing
/// starts at.
const PIE_GUIDES: &[PresetGuide] = &[
    PresetGuide {
        wire_name: "stAng",
        formula: "pin 0 adj1 21599999",
    },
    PresetGuide {
        wire_name: "enAng",
        formula: "pin 0 adj2 21599999",
    },
    PresetGuide {
        wire_name: "sw1",
        formula: "+- enAng 0 stAng",
    },
    PresetGuide {
        wire_name: "sw2",
        formula: "+- sw1 21600000 0",
    },
    PresetGuide {
        wire_name: "swAng",
        formula: "?: sw1 sw1 sw2",
    },
    PresetGuide {
        wire_name: "wt1",
        formula: "sin wd2 stAng",
    },
    PresetGuide {
        wire_name: "ht1",
        formula: "cos hd2 stAng",
    },
    PresetGuide {
        wire_name: "dx1",
        formula: "cat2 wd2 ht1 wt1",
    },
    PresetGuide {
        wire_name: "dy1",
        formula: "sat2 hd2 ht1 wt1",
    },
    PresetGuide {
        wire_name: "x1",
        formula: "+- hc dx1 0",
    },
    PresetGuide {
        wire_name: "y1",
        formula: "+- vc dy1 0",
    },
];

/// The steps of `pie`: out from the centre to the rim, around, and closed back to the centre.
const PIE_STEPS: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("hc", "vc")),
    PresetPathStep::LineTo(PresetPoint::at("x1", "y1")),
    PresetPathStep::ArcTo {
        width_radius: PresetCoordinate::Guide("wd2"),
        height_radius: PresetCoordinate::Guide("hd2"),
        start_angle: PresetAngle::Guide("stAng"),
        swing_angle: PresetAngle::Guide("swAng"),
    },
    PresetPathStep::Close,
];

/// A path with no coordinate box of its own — the shape's space is the path's space.
const fn whole_shape(steps: &'static [PresetPathStep]) -> PresetPath {
    PresetPath {
        width: None,
        height: None,
        steps,
    }
}

/// The six seeded shapes, in `PresetShapeType` declaration order.
pub(crate) const SEEDED_SHAPES: &[PresetShapeDefinition] = &[
    PresetShapeDefinition {
        preset: PresetShapeType::Ellipse,
        derivation: Derivation::FromFirstPrinciples,
        source: "ECMA-376 Part 1 §20.1.10.56, ST_ShapeType value `ellipse`. The ellipse inscribed \
                 in the shape's box: centre (hc, vc), radii wd2 and hd2. Drawn as four 90° a:arcTo \
                 quadrants clockwise from the left-middle point (l, vc) — cd2 to 3cd4 to 0 to cd4 \
                 — because a:arcTo places its ellipse from the current point and the four quadrant \
                 boundaries are the only angles at which the true-angle and parametric readings of \
                 stAng coincide, so the structure is forced rather than chosen. That also means \
                 agreement with a generated row here is weak evidence: there is no other sensible \
                 way to write it.",
        guides: &[],
        paths: &[whole_shape(ELLIPSE_STEPS)],
    },
    PresetShapeDefinition {
        preset: PresetShapeType::Pie,
        derivation: Derivation::ConstantsFromTheGeneratedTables,
        source:
            "ECMA-376 Part 1 §20.1.10.56, ST_ShapeType value `pie`: the wedge of the inscribed \
                 ellipse between a start angle (adj1) and an end angle (adj2). Both are angular \
                 adjustments in 60000ths of a degree; their defaults (0 and 16200000) and domain \
                 (0..21599999) are read from the generated adjustments_of table, which came from \
                 the same XML MJXOFF-203 reads. sw1 is the naive difference and sw2 the same \
                 difference brought back above zero by a whole turn, so `?: sw1 sw1 sw2` sweeps \
                 the long way round when the end angle precedes the start — and, at adj1 == adj2, \
                 sweeps a whole turn and draws the entire ellipse. The rim point (x1, y1) uses the \
                 cat2/sat2 idiom quoted in mjx-dml's own GuideOperator::CosineArcTangent \
                 documentation, which quotes the spec's `arc` shape: it converts the true angle \
                 stAng into the parametric one, which is what puts the arc's centre on (hc, vc). \
                 That idiom is therefore NOT independently derived.",
        guides: PIE_GUIDES,
        paths: &[whole_shape(PIE_STEPS)],
    },
    PresetShapeDefinition {
        preset: PresetShapeType::Rectangle,
        derivation: Derivation::FromFirstPrinciples,
        source: "ECMA-376 Part 1 §20.1.10.56, ST_ShapeType value `rect`. The shape's box itself: \
                 four corners clockwise from (l, t), no guides and no adjustments. There is one \
                 way to write it, so this row is fully independent and proves the least.",
        guides: &[],
        paths: &[whole_shape(RECTANGLE_STEPS)],
    },
    PresetShapeDefinition {
        preset: PresetShapeType::RightArrow,
        derivation: Derivation::ConstantsFromTheGeneratedTables,
        source: "ECMA-376 Part 1 §20.1.10.56, ST_ShapeType value `rightArrow`: a horizontal shaft \
                 ending in a triangular head that points right. adj1 is the shaft's thickness as a \
                 fraction of the height (0..100000, default 50000) and adj2 the head's length as a \
                 fraction of the shorter side (0..maxAdj2, default 50000); both domains and the \
                 formula `*/ 100000 w ss` for maxAdj2 are read from the generated \
                 adjustment_bound_guides_of table and are NOT independent. The head cannot be \
                 longer than the shape is wide, which is what that bound says: dx1 is a2 of the \
                 shorter side, so pinning a2 at 100000·w/ss pins dx1 at w. The shaft is centred on \
                 vc and dy1 is half its thickness, hence the 200000 divisor. Seven points \
                 clockwise from the shaft's top-left corner.",
        guides: RIGHT_ARROW_GUIDES,
        paths: &[whole_shape(RIGHT_ARROW_STEPS)],
    },
    PresetShapeDefinition {
        preset: PresetShapeType::RoundedRectangle,
        derivation: Derivation::ConstantsFromTheGeneratedTables,
        source:
            "ECMA-376 Part 1 §20.1.10.56, ST_ShapeType value `roundRect`: the shape's box with \
                 four quarter-circle corners. adj is the corner radius as a fraction of the \
                 shorter side; its domain (0..50000) and default (16667) are read from the \
                 generated adjustments_of table and are NOT independent. The upper bound is half \
                 the shorter side because two corners of more than that would overlap, which is \
                 what makes 50000 the only bound the shape could have. Four arcs and three lines: \
                 the left edge is drawn by the close, not by a lnTo.",
        guides: ROUNDED_RECTANGLE_GUIDES,
        paths: &[whole_shape(ROUNDED_RECTANGLE_STEPS)],
    },
    PresetShapeDefinition {
        preset: PresetShapeType::Triangle,
        derivation: Derivation::ConstantsFromTheGeneratedTables,
        source: "ECMA-376 Part 1 §20.1.10.56, ST_ShapeType value `triangle`: a triangle standing \
                 on the box's bottom edge with its apex on the top edge. adj places the apex \
                 horizontally as a fraction of the width; its domain (0..100000) and default \
                 (50000, the centre, which is the isosceles case the shape is pictured as) are \
                 read from the generated adjustments_of table and are NOT independent. Three \
                 points: bottom-left, apex, bottom-right.",
        guides: TRIANGLE_GUIDES,
        paths: &[whole_shape(TRIANGLE_STEPS)],
    },
];
