//! The typed shape-geometry tier: [`ShapeGeometry`] and [`PresetGeometry::shape`] / [`set_shape`].
//!
//! Each variant names a preset shape's adjustments in friendly units (a [`Fraction`]), converted from
//! the mechanical tier's native values. Meanings/units are **derived from `presetShapeDefinitions.xml`**
//! (see the derivation table in the batch plan), never guessed; shapes not yet ported fall through to
//! [`ShapeGeometry::Unmodeled`], so the type is total and still fully round-trips via the fidelity model.
//!
//! [`set_shape`]: PresetGeometry::set_shape

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::drawingml::PresetShapeType;

use super::measures::Fraction;
use super::PresetGeometry;

/// Native denominator for a fraction of a geometric reference (shorter side / width / height / radius);
/// adjustment values are in 1000ths of a percent, so `100_000` is 100%. The *reference* varies per
/// shape (documented on each variant); the denominator does not.
const FRACTION_DENOM: i32 = 100_000;
/// Native denominator for a star's inner-radius fraction (the spec computes the inner vertex as
/// `a / 50000` of the outer point radius).
const STAR_DENOM: i32 = 50_000;

/// A preset shape with its adjustments named in friendly units.
///
/// Returned by [`PresetGeometry::shape`] and consumed by [`PresetGeometry::set_shape`]. Every field is
/// a [`Fraction`] whose meaning is the field's name (e.g. a corner radius as a fraction of the shorter
/// side). Only shapes ported to the typed tier get a dedicated variant; every other preset is
/// [`Unmodeled`](Self::Unmodeled) and can still be read/set by wire name via the mechanical API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeGeometry {
    /// `roundRect` — a rectangle with all four corners rounded. `corner_radius` is a fraction of the
    /// shorter side (`adj`; `x1 = */ ss a 100000`, drawn as `arcTo wR=x1 hR=x1`).
    RoundedRectangle {
        /// Corner arc radius, as a fraction of the shorter side (0..≈0.5).
        corner_radius: Fraction,
    },
    /// `round1Rect` — a rectangle with one corner rounded. `corner_radius` is a fraction of the
    /// shorter side (`adj`).
    RoundSingleCornerRectangle {
        /// Corner arc radius, as a fraction of the shorter side (0..≈0.5).
        corner_radius: Fraction,
    },
    /// `snip1Rect` — a rectangle with one corner snipped (straight cut). `snip_size` is a fraction of
    /// the shorter side (`adj`; `dx1 = */ ss a 100000`).
    SnipSingleCornerRectangle {
        /// Length of the snipped corner cut, as a fraction of the shorter side (0..≈0.5).
        snip_size: Fraction,
    },
    /// `octagon` — a rectangle with all corners chamfered. `corner_cut` is a fraction of the shorter
    /// side (`adj`; `x1 = */ ss a 100000`).
    Octagon {
        /// Length of each chamfer, as a fraction of the shorter side (0..≈0.5).
        corner_cut: Fraction,
    },
    /// `plaque` — a rectangle with concave (inward-arced) corners. `corner_size` is a fraction of the
    /// shorter side (`adj`; `x1 = */ ss a 100000`).
    Plaque {
        /// Size of each concave corner, as a fraction of the shorter side (0..≈0.5).
        corner_size: Fraction,
    },
    /// `foldedCorner` — a rectangle with a folded (dog-ear) bottom-right corner. `fold_size` is a
    /// fraction of the shorter side (`adj`; `dy2 = */ ss a 100000`).
    FoldedCorner {
        /// Size of the folded corner, as a fraction of the shorter side (0..≈0.5).
        fold_size: Fraction,
    },
    /// `frame` — a rectangular picture frame. `border_thickness` is a fraction of the shorter side
    /// (`adj1`; `x1 = */ ss a1 100000`).
    Frame {
        /// Width of the frame border, as a fraction of the shorter side (0..≈0.5).
        border_thickness: Fraction,
    },
    /// `star4` — a 4-point star. `inner_radius` is the inner-vertex radius as a fraction of the outer
    /// point radius (`adj`; `iwd2 = */ wd2 a 50000`).
    FourPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star5` — a 5-point star. `inner_radius` is a fraction of the outer point radius.
    FivePointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star6` — a 6-point star. `inner_radius` is a fraction of the outer point radius.
    SixPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star7` — a 7-point star. `inner_radius` is a fraction of the outer point radius.
    SevenPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star8` — an 8-point star. `inner_radius` is a fraction of the outer point radius.
    EightPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star10` — a 10-point star. `inner_radius` is a fraction of the outer point radius.
    TenPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star12` — a 12-point star. `inner_radius` is a fraction of the outer point radius.
    TwelvePointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star16` — a 16-point star. `inner_radius` is a fraction of the outer point radius.
    SixteenPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star24` — a 24-point star. `inner_radius` is a fraction of the outer point radius.
    TwentyFourPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star32` — a 32-point star. `inner_radius` is a fraction of the outer point radius.
    ThirtyTwoPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `bracketPair` — a matched pair of square brackets. `corner_radius` is a fraction of the shorter
    /// side (`adj`; `x1 = */ ss a 100000`).
    BracketPair {
        /// Bracket corner arc radius, as a fraction of the shorter side.
        corner_radius: Fraction,
    },
    /// `bracePair` — a matched pair of curly braces. `curl_radius` is a fraction of the shorter side
    /// (`adj`; `x1 = */ ss a 100000`).
    BracePair {
        /// Brace curl arc radius, as a fraction of the shorter side.
        curl_radius: Fraction,
    },
    /// `leftBracket` — a single left square bracket. `corner_radius` is a fraction of the shorter side
    /// (`adj`; `y1 = */ ss a 100000`).
    LeftBracket {
        /// Corner arc radius, as a fraction of the shorter side.
        corner_radius: Fraction,
    },
    /// `rightBracket` — a single right square bracket. `corner_radius` is a fraction of the shorter side
    /// (`adj`; `y1 = */ ss a 100000`).
    RightBracket {
        /// Corner arc radius, as a fraction of the shorter side.
        corner_radius: Fraction,
    },
    /// `mathMinus` — a minus sign. `bar_thickness` is a fraction of the **height** (`adj1`;
    /// `dy1 = */ h a1 200000`, so the full bar is `h·a1/100000`).
    MathMinus {
        /// Bar thickness, as a fraction of the height.
        bar_thickness: Fraction,
    },
    /// `mathPlus` — a plus sign. `arm_thickness` is a fraction of the shorter side (`adj1`;
    /// `dx2 = */ ss a1 200000`, so the full arm is `ss·a1/100000`).
    MathPlus {
        /// Cross-arm thickness, as a fraction of the shorter side.
        arm_thickness: Fraction,
    },
    /// `mathMultiply` — a multiplication sign. `stroke_thickness` is a fraction of the shorter side
    /// (`adj1`; `th = */ ss a1 100000`).
    MathMultiply {
        /// Diagonal stroke thickness, as a fraction of the shorter side.
        stroke_thickness: Fraction,
    },
    /// `hexagon` — a hexagon. `point_inset` is the horizontal inset of the side vertices, as a fraction
    /// of the shorter side (`adj`; `x1 = */ ss a 100000`).
    Hexagon {
        /// Horizontal inset of the side vertices, as a fraction of the shorter side.
        point_inset: Fraction,
    },
    /// `trapezoid` — a trapezoid. `top_inset` is the horizontal inset of the top edge, as a fraction of
    /// the shorter side (`adj`; `x2 = */ ss a 100000`).
    Trapezoid {
        /// Horizontal inset of the top edge, as a fraction of the shorter side.
        top_inset: Fraction,
    },
    /// `triangle` — an isosceles triangle. `apex_x` is the apex's horizontal position, as a fraction of
    /// the **width** (`adj`; `x2 = */ w a 100000`; `0` = left, `0.5` = centered, `1` = right).
    Triangle {
        /// Apex horizontal position, as a fraction of the width.
        apex_x: Fraction,
    },
    /// `parallelogram` — a parallelogram. `skew_offset` is the top edge's rightward shift, as a fraction
    /// of the shorter side (`adj`; `x2 = */ ss a 100000`).
    Parallelogram {
        /// Top-edge skew offset, as a fraction of the shorter side.
        skew_offset: Fraction,
    },
    /// `chevron` — a chevron/arrow block. `point_depth` is the point/notch depth, as a fraction of the
    /// shorter side (`adj`; `x1 = */ ss a 100000`).
    Chevron {
        /// Point depth, as a fraction of the shorter side.
        point_depth: Fraction,
    },
    /// `homePlate` — a pentagon arrow (home plate). `point_depth` is the point depth, as a fraction of
    /// the shorter side (`adj`; `dx1 = */ ss a 100000`).
    HomePlate {
        /// Point depth, as a fraction of the shorter side.
        point_depth: Fraction,
    },
    /// `plus` — a plus/cross. `arm_inset` is each edge's inset toward the center, as a fraction of the
    /// shorter side (`adj`; `x1 = */ ss a 100000`); larger values give thinner arms.
    Plus {
        /// Inset of each arm from the edges, as a fraction of the shorter side (larger ⇒ thinner arms).
        arm_inset: Fraction,
    },
    /// `donut` — a ring/annulus. `ring_thickness` is the band width, as a fraction of the shorter side
    /// (`adj`, radius handle; `dr = */ ss a 100000`).
    Donut {
        /// Ring band width, as a fraction of the shorter side.
        ring_thickness: Fraction,
    },
    /// `noSmoking` — a prohibition ("no") symbol. `band_thickness` is the ring/bar width, as a fraction
    /// of the shorter side (`adj`, radius handle; `dr = */ ss a 100000`).
    NoSmoking {
        /// Ring/bar width, as a fraction of the shorter side.
        band_thickness: Fraction,
    },
    /// `horizontalScroll` — a horizontal scroll. `curl_size` is the rolled-curl size, as a fraction of
    /// the shorter side (`adj`; `ch = */ ss a 100000`).
    HorizontalScroll {
        /// Rolled-curl size, as a fraction of the shorter side.
        curl_size: Fraction,
    },
    /// `verticalScroll` — a vertical scroll. `curl_size` is the rolled-curl size, as a fraction of the
    /// shorter side (`adj`; `ch = */ ss a 100000`).
    VerticalScroll {
        /// Rolled-curl size, as a fraction of the shorter side.
        curl_size: Fraction,
    },
    /// `bevel` — a beveled (raised) rectangle. `bevel_width` is the bevel edge width, as a fraction of
    /// the shorter side (`adj`; `x1 = */ ss a 100000`).
    Bevel {
        /// Bevel edge width, as a fraction of the shorter side.
        bevel_width: Fraction,
    },
    /// `can` — a cylinder. `top_ellipse_height` is the top ellipse's height, as a fraction of the
    /// shorter side (`adj`; `y2 = ss·a/100000`).
    Can {
        /// Top ellipse height, as a fraction of the shorter side.
        top_ellipse_height: Fraction,
    },
    /// `cube` — an isometric cube. `depth` is the top/side face depth, as a fraction of the shorter side
    /// (`adj`; `y1 = */ ss a 100000`).
    Cube {
        /// Isometric face depth, as a fraction of the shorter side.
        depth: Fraction,
    },
    /// `moon` — a crescent moon. `crescent_width` is the crescent's width, as a fraction of the shorter
    /// side (`adj`; `g0 = */ ss a 100000`).
    Moon {
        /// Crescent width, as a fraction of the shorter side.
        crescent_width: Fraction,
    },
    /// `smileyFace` — a smiley face. `mouth_curve` is the mouth's curvature, as a **signed** fraction of
    /// the height (`adj`; `dy2 = */ h a 100000`): negative frowns, positive smiles.
    SmileyFace {
        /// Mouth curvature, as a signed fraction of the height (negative = frown, positive = smile).
        mouth_curve: Fraction,
    },
    /// `diagStripe` — a diagonal stripe. `stripe_width` sets the band's width/position, as a fraction of
    /// the width (on X) and height (on Y) (`adj`; `x2 = */ w a 100000`, `y2 = */ h a 100000`).
    DiagonalStripe {
        /// Diagonal band width/position, as a fraction of the width (X) and height (Y).
        stripe_width: Fraction,
    },
    /// `bentConnector3` — a 3-segment bent connector. `bend_position` is the vertical jog's horizontal
    /// position, as a fraction of the width (`adj1`; `x1 = */ w adj1 100000`); unbounded (may leave 0..1).
    BentConnector3 {
        /// Bend column position, as a fraction of the width (may be outside `0..1`).
        bend_position: Fraction,
    },
    /// `curvedConnector3` — a 3-segment curved connector. `bend_position` is the S-curve's control
    /// column, as a fraction of the width (`adj1`; `x2 = */ w adj1 100000`); unbounded.
    CurvedConnector3 {
        /// Bend column position, as a fraction of the width (may be outside `0..1`).
        bend_position: Fraction,
    },
    /// A known preset this tier does not yet model. Its adjustments (if any) remain available by wire
    /// name through [`PresetGeometry::adjustment`] / [`set_adjustment`](PresetGeometry::set_adjustment).
    Unmodeled(PresetShapeType),
}

impl PresetGeometry {
    /// The shape as a typed [`ShapeGeometry`] with its adjustments in friendly units, or `None` if the
    /// `prst` is absent or names a token this build does not know.
    ///
    /// Adjustment values are the `avLst` override if present, else the shape's spec default.
    #[must_use]
    pub fn shape(&self, interner: &Interner) -> Option<ShapeGeometry> {
        let preset = self.preset(interner)?;
        Some(match preset {
            PresetShapeType::RoundedRectangle => ShapeGeometry::RoundedRectangle {
                corner_radius: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::RoundSingleCornerRectangle => {
                ShapeGeometry::RoundSingleCornerRectangle {
                    corner_radius: self.fraction(interner, "adj", FRACTION_DENOM),
                }
            }
            PresetShapeType::SnipSingleCornerRectangle => {
                ShapeGeometry::SnipSingleCornerRectangle {
                    snip_size: self.fraction(interner, "adj", FRACTION_DENOM),
                }
            }
            PresetShapeType::Octagon => ShapeGeometry::Octagon {
                corner_cut: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Plaque => ShapeGeometry::Plaque {
                corner_size: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::FoldedCorner => ShapeGeometry::FoldedCorner {
                fold_size: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Frame => ShapeGeometry::Frame {
                border_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
            },
            PresetShapeType::FourPointStar => ShapeGeometry::FourPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::FivePointStar => ShapeGeometry::FivePointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::SixPointStar => ShapeGeometry::SixPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::SevenPointStar => ShapeGeometry::SevenPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::EightPointStar => ShapeGeometry::EightPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::TenPointStar => ShapeGeometry::TenPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::TwelvePointStar => ShapeGeometry::TwelvePointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::SixteenPointStar => ShapeGeometry::SixteenPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::TwentyFourPointStar => ShapeGeometry::TwentyFourPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::ThirtyTwoPointStar => ShapeGeometry::ThirtyTwoPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::BracketPair => ShapeGeometry::BracketPair {
                corner_radius: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::BracePair => ShapeGeometry::BracePair {
                curl_radius: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::LeftBracket => ShapeGeometry::LeftBracket {
                corner_radius: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::RightBracket => ShapeGeometry::RightBracket {
                corner_radius: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::MathMinus => ShapeGeometry::MathMinus {
                bar_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
            },
            PresetShapeType::MathPlus => ShapeGeometry::MathPlus {
                arm_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
            },
            PresetShapeType::MathMultiply => ShapeGeometry::MathMultiply {
                stroke_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
            },
            PresetShapeType::Hexagon => ShapeGeometry::Hexagon {
                point_inset: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Trapezoid => ShapeGeometry::Trapezoid {
                top_inset: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Triangle => ShapeGeometry::Triangle {
                apex_x: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Parallelogram => ShapeGeometry::Parallelogram {
                skew_offset: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Chevron => ShapeGeometry::Chevron {
                point_depth: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::HomePlate => ShapeGeometry::HomePlate {
                point_depth: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Plus => ShapeGeometry::Plus {
                arm_inset: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Donut => ShapeGeometry::Donut {
                ring_thickness: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::NoSmoking => ShapeGeometry::NoSmoking {
                band_thickness: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::HorizontalScroll => ShapeGeometry::HorizontalScroll {
                curl_size: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::VerticalScroll => ShapeGeometry::VerticalScroll {
                curl_size: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Bevel => ShapeGeometry::Bevel {
                bevel_width: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Can => ShapeGeometry::Can {
                top_ellipse_height: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Cube => ShapeGeometry::Cube {
                depth: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::Moon => ShapeGeometry::Moon {
                crescent_width: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::SmileyFace => ShapeGeometry::SmileyFace {
                mouth_curve: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::DiagonalStripe => ShapeGeometry::DiagonalStripe {
                stripe_width: self.fraction(interner, "adj", FRACTION_DENOM),
            },
            PresetShapeType::BentConnector3 => ShapeGeometry::BentConnector3 {
                bend_position: self.fraction(interner, "adj1", FRACTION_DENOM),
            },
            PresetShapeType::CurvedConnector3 => ShapeGeometry::CurvedConnector3 {
                bend_position: self.fraction(interner, "adj1", FRACTION_DENOM),
            },
            other => ShapeGeometry::Unmodeled(other),
        })
    }

    /// Sets the shape from a typed [`ShapeGeometry`]: rewrites `prst` and each named adjustment (in the
    /// `avLst`, creating it if needed), converting friendly units back to native. `Unmodeled` sets only
    /// the preset.
    pub fn set_shape(&mut self, interner: &mut Interner, shape: ShapeGeometry) {
        match shape {
            ShapeGeometry::RoundedRectangle { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::RoundedRectangle,
                    "adj",
                    corner_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::RoundSingleCornerRectangle { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::RoundSingleCornerRectangle,
                    "adj",
                    corner_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::SnipSingleCornerRectangle { snip_size } => {
                self.apply(
                    interner,
                    PresetShapeType::SnipSingleCornerRectangle,
                    "adj",
                    snip_size,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Octagon { corner_cut } => {
                self.apply(
                    interner,
                    PresetShapeType::Octagon,
                    "adj",
                    corner_cut,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Plaque { corner_size } => {
                self.apply(
                    interner,
                    PresetShapeType::Plaque,
                    "adj",
                    corner_size,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::FoldedCorner { fold_size } => {
                self.apply(
                    interner,
                    PresetShapeType::FoldedCorner,
                    "adj",
                    fold_size,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Frame { border_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::Frame,
                    "adj1",
                    border_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::FourPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::FourPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::FivePointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::FivePointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::SixPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::SixPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::SevenPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::SevenPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::EightPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::EightPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::TenPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::TenPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::TwelvePointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::TwelvePointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::SixteenPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::SixteenPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::TwentyFourPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::TwentyFourPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::ThirtyTwoPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::ThirtyTwoPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::BracketPair { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::BracketPair,
                    "adj",
                    corner_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BracePair { curl_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::BracePair,
                    "adj",
                    curl_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftBracket { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftBracket,
                    "adj",
                    corner_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::RightBracket { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::RightBracket,
                    "adj",
                    corner_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::MathMinus { bar_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::MathMinus,
                    "adj1",
                    bar_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::MathPlus { arm_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::MathPlus,
                    "adj1",
                    arm_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::MathMultiply { stroke_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::MathMultiply,
                    "adj1",
                    stroke_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Hexagon { point_inset } => {
                self.apply(
                    interner,
                    PresetShapeType::Hexagon,
                    "adj",
                    point_inset,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Trapezoid { top_inset } => {
                self.apply(
                    interner,
                    PresetShapeType::Trapezoid,
                    "adj",
                    top_inset,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Triangle { apex_x } => {
                self.apply(
                    interner,
                    PresetShapeType::Triangle,
                    "adj",
                    apex_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Parallelogram { skew_offset } => {
                self.apply(
                    interner,
                    PresetShapeType::Parallelogram,
                    "adj",
                    skew_offset,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Chevron { point_depth } => {
                self.apply(
                    interner,
                    PresetShapeType::Chevron,
                    "adj",
                    point_depth,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::HomePlate { point_depth } => {
                self.apply(
                    interner,
                    PresetShapeType::HomePlate,
                    "adj",
                    point_depth,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Plus { arm_inset } => {
                self.apply(
                    interner,
                    PresetShapeType::Plus,
                    "adj",
                    arm_inset,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Donut { ring_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::Donut,
                    "adj",
                    ring_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::NoSmoking { band_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::NoSmoking,
                    "adj",
                    band_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::HorizontalScroll { curl_size } => {
                self.apply(
                    interner,
                    PresetShapeType::HorizontalScroll,
                    "adj",
                    curl_size,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::VerticalScroll { curl_size } => {
                self.apply(
                    interner,
                    PresetShapeType::VerticalScroll,
                    "adj",
                    curl_size,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Bevel { bevel_width } => {
                self.apply(
                    interner,
                    PresetShapeType::Bevel,
                    "adj",
                    bevel_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Can { top_ellipse_height } => {
                self.apply(
                    interner,
                    PresetShapeType::Can,
                    "adj",
                    top_ellipse_height,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Cube { depth } => {
                self.apply(
                    interner,
                    PresetShapeType::Cube,
                    "adj",
                    depth,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Moon { crescent_width } => {
                self.apply(
                    interner,
                    PresetShapeType::Moon,
                    "adj",
                    crescent_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::SmileyFace { mouth_curve } => {
                self.apply(
                    interner,
                    PresetShapeType::SmileyFace,
                    "adj",
                    mouth_curve,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::DiagonalStripe { stripe_width } => {
                self.apply(
                    interner,
                    PresetShapeType::DiagonalStripe,
                    "adj",
                    stripe_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BentConnector3 { bend_position } => {
                self.apply(
                    interner,
                    PresetShapeType::BentConnector3,
                    "adj1",
                    bend_position,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedConnector3 { bend_position } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedConnector3,
                    "adj1",
                    bend_position,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Unmodeled(preset) => self.set_preset(interner, preset),
        }
    }

    /// Reads adjustment `wire` (override or default) and converts it to a [`Fraction`] over `denom`.
    fn fraction(&self, interner: &Interner, wire: &str, denom: i32) -> Fraction {
        let native = self.adjustment(interner, wire).unwrap_or(0);
        Fraction::from_ratio(f64::from(native) / f64::from(denom))
    }

    /// Sets `preset` and writes `value` (a fraction over `denom`) to adjustment `wire` in native units.
    fn apply(
        &mut self,
        interner: &mut Interner,
        preset: PresetShapeType,
        wire: &str,
        value: Fraction,
        denom: i32,
    ) {
        self.set_preset(interner, preset);
        let native = (value.ratio() * f64::from(denom)).round() as i32;
        self.set_adjustment(interner, wire, native);
    }
}
