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

use super::measures::{Angle, Fraction};
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
    /// `arc` — an elliptical arc. Both angles are absolute (`adj1`/`adj2`, `ahPolar`); the drawn sweep
    /// is `end_angle − start_angle`.
    Arc {
        /// Start angle.
        start_angle: Angle,
        /// End angle.
        end_angle: Angle,
    },
    /// `chord` — a circle segment cut by a chord. `start_angle`/`end_angle` are absolute (`adj1`/`adj2`).
    Chord {
        /// Start angle.
        start_angle: Angle,
        /// End angle.
        end_angle: Angle,
    },
    /// `pie` — a pie slice. `start_angle`/`end_angle` are absolute (`adj1`/`adj2`).
    Pie {
        /// Start angle.
        start_angle: Angle,
        /// End angle.
        end_angle: Angle,
    },
    /// `downArrow` — a downward arrow. `shaft_thickness` is the shaft width as a fraction of the width
    /// (`adj1`); `head_length` is the arrowhead length as a fraction of the shorter side (`adj2`).
    DownArrow {
        /// Shaft thickness, as a fraction of the width.
        shaft_thickness: Fraction,
        /// Arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `leftArrow` — a leftward arrow. `shaft_thickness` is a fraction of the height (`adj1`);
    /// `head_length` is a fraction of the shorter side (`adj2`).
    LeftArrow {
        /// Shaft thickness, as a fraction of the height.
        shaft_thickness: Fraction,
        /// Arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `rightArrow` — a rightward arrow. `shaft_thickness` is a fraction of the height (`adj1`);
    /// `head_length` is a fraction of the shorter side (`adj2`).
    RightArrow {
        /// Shaft thickness, as a fraction of the height.
        shaft_thickness: Fraction,
        /// Arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `leftRightArrow` — a double-headed horizontal arrow. `shaft_thickness` is a fraction of the
    /// height (`adj1`); `head_length` is each head's length as a fraction of the shorter side (`adj2`).
    LeftRightArrow {
        /// Shaft thickness, as a fraction of the height.
        shaft_thickness: Fraction,
        /// Each arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `upDownArrow` — a double-headed vertical arrow. `shaft_thickness` is a fraction of the width
    /// (`adj1`); `head_length` is each head's length as a fraction of the shorter side (`adj2`).
    UpDownArrow {
        /// Shaft thickness, as a fraction of the width.
        shaft_thickness: Fraction,
        /// Each arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `notchedRightArrow` — a rightward arrow with a notched tail. `shaft_thickness` is a fraction of
    /// the height (`adj1`); `head_length` is a fraction of the shorter side (`adj2`).
    NotchedRightArrow {
        /// Shaft thickness, as a fraction of the height.
        shaft_thickness: Fraction,
        /// Arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `stripedRightArrow` — a rightward arrow with a striped tail. `shaft_thickness` is a fraction of
    /// the height (`adj1`); `head_length` is a fraction of the shorter side (`adj2`).
    StripedRightArrow {
        /// Shaft thickness, as a fraction of the height.
        shaft_thickness: Fraction,
        /// Arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `swooshArrow` — a curved (swoosh) arrow. `head_thickness` is the arrowhead's vertical span as a
    /// fraction of the height (`adj1`); `head_length` is a fraction of the shorter side (`adj2`).
    ///
    /// (Reviewer note: whether `adj1` is the head vs. tail thickness is not fully unambiguous from the
    /// geometry file — confirm against ECMA-376 prose.)
    SwooshArrow {
        /// Arrowhead thickness, as a fraction of the height.
        head_thickness: Fraction,
        /// Arrowhead length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `cloudCallout` — a cloud callout. `tail_x`/`tail_y` locate the tail tip relative to the center,
    /// as **signed** fractions of the width (`adj1`) and height (`adj2`).
    CloudCallout {
        /// Tail-tip horizontal offset from center, as a signed fraction of the width.
        tail_x: Fraction,
        /// Tail-tip vertical offset from center, as a signed fraction of the height.
        tail_y: Fraction,
    },
    /// `wedgeEllipseCallout` — an elliptical callout with a wedge tail. `tail_x`/`tail_y` are signed
    /// fractions of the width (`adj1`) and height (`adj2`).
    WedgeEllipseCallout {
        /// Tail-tip horizontal offset from center, as a signed fraction of the width.
        tail_x: Fraction,
        /// Tail-tip vertical offset from center, as a signed fraction of the height.
        tail_y: Fraction,
    },
    /// `wedgeRectCallout` — a rectangular callout with a wedge tail. `tail_x`/`tail_y` are signed
    /// fractions of the width (`adj1`) and height (`adj2`).
    WedgeRectangleCallout {
        /// Tail-tip horizontal offset from center, as a signed fraction of the width.
        tail_x: Fraction,
        /// Tail-tip vertical offset from center, as a signed fraction of the height.
        tail_y: Fraction,
    },
    /// `wedgeRoundRectCallout` — a rounded-rectangle callout with a wedge tail. `tail_x`/`tail_y` are
    /// signed fractions of the width (`adj1`) and height (`adj2`); the corner radius (`adj3`) has no
    /// handle and is not modeled here.
    WedgeRoundedRectangleCallout {
        /// Tail-tip horizontal offset from center, as a signed fraction of the width.
        tail_x: Fraction,
        /// Tail-tip vertical offset from center, as a signed fraction of the height.
        tail_y: Fraction,
    },
    /// `round2SameRect` — a rectangle with the two top and two bottom corners rounded independently.
    /// Both radii are fractions of the shorter side (`adj1` = top, `adj2` = bottom).
    RoundSameSideCornersRectangle {
        /// Top corners' radius, as a fraction of the shorter side.
        top_corner_radius: Fraction,
        /// Bottom corners' radius, as a fraction of the shorter side.
        bottom_corner_radius: Fraction,
    },
    /// `round2DiagRect` — a rectangle with the two diagonal corner-pairs rounded independently. Both
    /// radii are fractions of the shorter side (`adj1` = top-left & bottom-right, `adj2` = the other pair).
    RoundDiagonalCornersRectangle {
        /// Top-left & bottom-right corners' radius, as a fraction of the shorter side.
        top_left_bottom_right_radius: Fraction,
        /// Top-right & bottom-left corners' radius, as a fraction of the shorter side.
        top_right_bottom_left_radius: Fraction,
    },
    /// `snip2SameRect` — a rectangle with the two top and two bottom corners snipped independently.
    /// Both are fractions of the shorter side (`adj1` = top, `adj2` = bottom).
    SnipSameSideCornersRectangle {
        /// Top corners' snip size, as a fraction of the shorter side.
        top_corner_snip: Fraction,
        /// Bottom corners' snip size, as a fraction of the shorter side.
        bottom_corner_snip: Fraction,
    },
    /// `snip2DiagRect` — a rectangle with the two diagonal corner-pairs snipped independently. Both
    /// are fractions of the shorter side (`adj1` = top-left & bottom-right, `adj2` = the other pair).
    SnipDiagonalCornersRectangle {
        /// Top-left & bottom-right corners' snip size, as a fraction of the shorter side.
        top_left_bottom_right_snip: Fraction,
        /// Top-right & bottom-left corners' snip size, as a fraction of the shorter side.
        top_right_bottom_left_snip: Fraction,
    },
    /// `snipRoundRect` — a rectangle with one rounded and one snipped top corner. Both are fractions of
    /// the shorter side (`adj1` = top-left round radius, `adj2` = top-right snip size).
    SnipAndRoundSingleCornerRectangle {
        /// Top-left corner's round radius, as a fraction of the shorter side.
        round_corner_radius: Fraction,
        /// Top-right corner's snip size, as a fraction of the shorter side.
        snip_corner_size: Fraction,
    },
    /// `leftBrace` — a left curly brace. `curl_radius` is the curl arc radius as a fraction of the
    /// shorter side (`adj1`); `point_position` is the mid-point's vertical position as a fraction of
    /// the height (`adj2`).
    LeftBrace {
        /// Curl arc radius, as a fraction of the shorter side.
        curl_radius: Fraction,
        /// Mid-point vertical position, as a fraction of the height.
        point_position: Fraction,
    },
    /// `rightBrace` — a right curly brace. `curl_radius` is a fraction of the shorter side (`adj1`);
    /// `point_position` is a fraction of the height (`adj2`).
    RightBrace {
        /// Curl arc radius, as a fraction of the shorter side.
        curl_radius: Fraction,
        /// Mid-point vertical position, as a fraction of the height.
        point_position: Fraction,
    },
    /// `ribbon` — a downward ribbon banner. `band_height` is the fold band height as a fraction of the
    /// height (`adj1`); `panel_width` is the central panel's half-width as a fraction of the width (`adj2`).
    Ribbon {
        /// Fold band height, as a fraction of the height.
        band_height: Fraction,
        /// Central panel half-width, as a fraction of the width.
        panel_width: Fraction,
    },
    /// `ribbon2` — an upward ribbon banner. `band_height` is a fraction of the height (`adj1`);
    /// `panel_width` is a fraction of the width (`adj2`).
    Ribbon2 {
        /// Fold band height, as a fraction of the height.
        band_height: Fraction,
        /// Central panel half-width, as a fraction of the width.
        panel_width: Fraction,
    },
    /// `wave` — a wave/flag. `amplitude` is the wave height as a fraction of the height (`adj1`);
    /// `skew` is the horizontal lean as a **signed** fraction of the width (`adj2`).
    Wave {
        /// Wave amplitude, as a fraction of the height.
        amplitude: Fraction,
        /// Horizontal skew, as a signed fraction of the width.
        skew: Fraction,
    },
    /// `doubleWave` — a double wave. `amplitude` is a fraction of the height (`adj1`); `skew` is a
    /// signed fraction of the width (`adj2`).
    DoubleWave {
        /// Wave amplitude, as a fraction of the height.
        amplitude: Fraction,
        /// Horizontal skew, as a signed fraction of the width.
        skew: Fraction,
    },
    /// `gear6` — a 6-tooth gear. `tooth_depth` is the radial tooth/ring depth (`adj1`) and `tooth_width`
    /// the tooth land width (`adj2`), both fractions of the shorter side.
    Gear6 {
        /// Radial tooth depth, as a fraction of the shorter side.
        tooth_depth: Fraction,
        /// Tooth land width, as a fraction of the shorter side.
        tooth_width: Fraction,
    },
    /// `gear9` — a 9-tooth gear. `tooth_depth` (`adj1`) and `tooth_width` (`adj2`) are fractions of the
    /// shorter side.
    Gear9 {
        /// Radial tooth depth, as a fraction of the shorter side.
        tooth_depth: Fraction,
        /// Tooth land width, as a fraction of the shorter side.
        tooth_width: Fraction,
    },
    /// `bentConnector4` — a 4-segment bent connector. `bend_x` is the vertical jog's position as a
    /// fraction of the width (`adj1`); `bend_y` is the horizontal jog's position as a fraction of the
    /// height (`adj2`). Both may leave `0..1`.
    BentConnector4 {
        /// Vertical-jog position, as a fraction of the width.
        bend_x: Fraction,
        /// Horizontal-jog position, as a fraction of the height.
        bend_y: Fraction,
    },
    /// `curvedConnector4` — a 4-segment curved connector. `bend_x` (`adj1`, fraction of width) and
    /// `bend_y` (`adj2`, fraction of height) place the two curve bends. Both may leave `0..1`.
    CurvedConnector4 {
        /// First-bend position, as a fraction of the width.
        bend_x: Fraction,
        /// Second-bend position, as a fraction of the height.
        bend_y: Fraction,
    },
    /// `corner` — an L-shaped corner. `horizontal_arm_thickness` (`adj1`) and `vertical_arm_thickness`
    /// (`adj2`) are fractions of the shorter side.
    ///
    /// (Reviewer note: which arm each adjustment governs is not fully unambiguous from the geometry
    /// file — confirm against ECMA-376 prose.)
    Corner {
        /// Thickness of the horizontal arm, as a fraction of the shorter side.
        horizontal_arm_thickness: Fraction,
        /// Thickness of the vertical arm, as a fraction of the shorter side.
        vertical_arm_thickness: Fraction,
    },
    /// `halfFrame` — half of a picture frame (an L). `top_arm_thickness` (`adj1`) and
    /// `side_arm_thickness` (`adj2`) are fractions of the shorter side.
    ///
    /// (Reviewer note: which arm each adjustment governs is not fully unambiguous from the geometry
    /// file — confirm against ECMA-376 prose.)
    HalfFrame {
        /// Thickness of the top arm, as a fraction of the shorter side.
        top_arm_thickness: Fraction,
        /// Thickness of the side arm, as a fraction of the shorter side.
        side_arm_thickness: Fraction,
    },
    /// `mathEqual` — an equals sign. `bar_thickness` is each bar's thickness (`adj1`) and `bar_gap` is
    /// half the gap between the bars (`adj2`), both fractions of the height.
    MathEqual {
        /// Each bar's thickness, as a fraction of the height.
        bar_thickness: Fraction,
        /// Half the gap between the two bars, as a fraction of the height.
        bar_gap: Fraction,
    },
    /// `nonIsoscelesTrapezoid` — a trapezoid with independently inset top vertices. `left_top_inset`
    /// (`adj1`) and `right_top_inset` (`adj2`) are fractions of the shorter side.
    NonIsoscelesTrapezoid {
        /// Horizontal inset of the top-left vertex, as a fraction of the shorter side.
        left_top_inset: Fraction,
        /// Horizontal inset of the top-right vertex, as a fraction of the shorter side.
        right_top_inset: Fraction,
    },
    /// `callout1` — a line callout with 1 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex2` the pointer tip.
    Callout1 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (pointer tip), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (pointer tip), y as a signed fraction of the height.
        vertex2_y: Fraction,
    },
    /// `callout2` — a line callout with 2 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex3` the pointer tip.
    Callout2 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (pointer tip), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (pointer tip), y as a signed fraction of the height.
        vertex3_y: Fraction,
    },
    /// `callout3` — a line callout with 3 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex4` the pointer tip.
    Callout3 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (bend), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (bend), y as a signed fraction of the height.
        vertex3_y: Fraction,
        /// Leader vertex 4 (pointer tip), x as a signed fraction of the width.
        vertex4_x: Fraction,
        /// Leader vertex 4 (pointer tip), y as a signed fraction of the height.
        vertex4_y: Fraction,
    },
    /// `accentCallout1` — a line callout with an accent bar with 1 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex2` the pointer tip.
    AccentCallout1 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (pointer tip), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (pointer tip), y as a signed fraction of the height.
        vertex2_y: Fraction,
    },
    /// `accentCallout2` — a line callout with an accent bar with 2 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex3` the pointer tip.
    AccentCallout2 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (pointer tip), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (pointer tip), y as a signed fraction of the height.
        vertex3_y: Fraction,
    },
    /// `accentCallout3` — a line callout with an accent bar with 3 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex4` the pointer tip.
    AccentCallout3 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (bend), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (bend), y as a signed fraction of the height.
        vertex3_y: Fraction,
        /// Leader vertex 4 (pointer tip), x as a signed fraction of the width.
        vertex4_x: Fraction,
        /// Leader vertex 4 (pointer tip), y as a signed fraction of the height.
        vertex4_y: Fraction,
    },
    /// `borderCallout1` — a bordered line callout with 1 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex2` the pointer tip.
    BorderCallout1 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (pointer tip), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (pointer tip), y as a signed fraction of the height.
        vertex2_y: Fraction,
    },
    /// `borderCallout2` — a bordered line callout with 2 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex3` the pointer tip.
    BorderCallout2 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (pointer tip), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (pointer tip), y as a signed fraction of the height.
        vertex3_y: Fraction,
    },
    /// `borderCallout3` — a bordered line callout with 3 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex4` the pointer tip.
    BorderCallout3 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (bend), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (bend), y as a signed fraction of the height.
        vertex3_y: Fraction,
        /// Leader vertex 4 (pointer tip), x as a signed fraction of the width.
        vertex4_x: Fraction,
        /// Leader vertex 4 (pointer tip), y as a signed fraction of the height.
        vertex4_y: Fraction,
    },
    /// `accentBorderCallout1` — a bordered line callout with an accent bar with 1 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex2` the pointer tip.
    AccentBorderCallout1 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (pointer tip), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (pointer tip), y as a signed fraction of the height.
        vertex2_y: Fraction,
    },
    /// `accentBorderCallout2` — a bordered line callout with an accent bar with 2 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex3` the pointer tip.
    AccentBorderCallout2 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (pointer tip), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (pointer tip), y as a signed fraction of the height.
        vertex3_y: Fraction,
    },
    /// `accentBorderCallout3` — a bordered line callout with an accent bar with 3 segment(s). Adjustments are the leader-line
    /// vertices as signed fractions of width (x) and height (y); `vertex1` is the box-side
    /// anchor and `vertex4` the pointer tip.
    AccentBorderCallout3 {
        /// Leader vertex 1 (box anchor), x as a signed fraction of the width.
        vertex1_x: Fraction,
        /// Leader vertex 1 (box anchor), y as a signed fraction of the height.
        vertex1_y: Fraction,
        /// Leader vertex 2 (bend), x as a signed fraction of the width.
        vertex2_x: Fraction,
        /// Leader vertex 2 (bend), y as a signed fraction of the height.
        vertex2_y: Fraction,
        /// Leader vertex 3 (bend), x as a signed fraction of the width.
        vertex3_x: Fraction,
        /// Leader vertex 3 (bend), y as a signed fraction of the height.
        vertex3_y: Fraction,
        /// Leader vertex 4 (pointer tip), x as a signed fraction of the width.
        vertex4_x: Fraction,
        /// Leader vertex 4 (pointer tip), y as a signed fraction of the height.
        vertex4_y: Fraction,
    },
    /// `bentConnector5` — a 5-segment bent connector. The three bend positions of the connector route, as fractions of width/height/width (unbounded — may leave 0..1).
    BentConnector5 {
        /// Bend1 x, as a fraction of the width.
        bend1_x: Fraction,
        /// Bend2 y, as a fraction of the height.
        bend2_y: Fraction,
        /// Bend3 x, as a fraction of the width.
        bend3_x: Fraction,
    },
    /// `curvedConnector5` — a 5-segment curved connector. The three curve control positions, as fractions of width/height/width (unbounded — may leave 0..1).
    CurvedConnector5 {
        /// Bend1 x, as a fraction of the width.
        bend1_x: Fraction,
        /// Bend2 y, as a fraction of the height.
        bend2_y: Fraction,
        /// Bend3 x, as a fraction of the width.
        bend3_x: Fraction,
    },
    /// `curvedDownArrow` — a downward curved arrow. Curved-band body thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    CurvedDownArrow {
        /// Body thickness, as a fraction of the shorter side.
        body_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `curvedUpArrow` — an upward curved arrow. Curved-band body thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    CurvedUpArrow {
        /// Body thickness, as a fraction of the shorter side.
        body_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `curvedLeftArrow` — a leftward curved arrow. Curved-band body thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    CurvedLeftArrow {
        /// Body thickness, as a fraction of the shorter side.
        body_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `curvedRightArrow` — a rightward curved arrow. Curved-band body thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    CurvedRightArrow {
        /// Body thickness, as a fraction of the shorter side.
        body_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `ellipseRibbon` — a downward-arched ribbon banner. Ribbon arch height, central panel width, and folded-strip thickness (fractions of height/width/height).
    EllipseRibbon {
        /// Arch height, as a fraction of the height.
        arch_height: Fraction,
        /// Center width, as a fraction of the width.
        center_width: Fraction,
        /// Fold thickness, as a fraction of the height.
        fold_thickness: Fraction,
    },
    /// `ellipseRibbon2` — an upward-arched ribbon banner. Ribbon arch height, central panel width, and folded-strip thickness (fractions of height/width/height).
    EllipseRibbon2 {
        /// Arch height, as a fraction of the height.
        arch_height: Fraction,
        /// Center width, as a fraction of the width.
        center_width: Fraction,
        /// Fold thickness, as a fraction of the height.
        fold_thickness: Fraction,
    },
    /// `leftRightRibbon` — a left-right ribbon banner. Ribbon band height, tapered end width, and the center crossing fold (fractions of height/shorter-side/height).
    LeftRightRibbon {
        /// Band height, as a fraction of the height.
        band_height: Fraction,
        /// End width, as a fraction of the shorter side.
        end_width: Fraction,
        /// Center fold, as a fraction of the height.
        center_fold: Fraction,
    },
    /// `bentUpArrow` — a bent arrow turning upward. Arrow shaft thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    BentUpArrow {
        /// Shaft thickness, as a fraction of the shorter side.
        shaft_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `leftUpArrow` — an arrow pointing left and up. Arrow shaft thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    LeftUpArrow {
        /// Shaft thickness, as a fraction of the shorter side.
        shaft_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `leftRightUpArrow` — a three-way (left/right/up) arrow. Arrow shaft thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    LeftRightUpArrow {
        /// Shaft thickness, as a fraction of the shorter side.
        shaft_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
    },
    /// `quadArrow` — a four-way arrow. Arrow shaft thickness, arrowhead width, and arrowhead length, each a fraction of the shorter side.
    QuadArrow {
        /// Shaft thickness, as a fraction of the shorter side.
        shaft_thickness: Fraction,
        /// Head width, as a fraction of the shorter side.
        head_width: Fraction,
        /// Head length, as a fraction of the shorter side.
        head_length: Fraction,
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
            PresetShapeType::Arc => ShapeGeometry::Arc {
                start_angle: self.angle(interner, "adj1"),
                end_angle: self.angle(interner, "adj2"),
            },
            PresetShapeType::Chord => ShapeGeometry::Chord {
                start_angle: self.angle(interner, "adj1"),
                end_angle: self.angle(interner, "adj2"),
            },
            PresetShapeType::Pie => ShapeGeometry::Pie {
                start_angle: self.angle(interner, "adj1"),
                end_angle: self.angle(interner, "adj2"),
            },
            PresetShapeType::DownArrow => ShapeGeometry::DownArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::LeftArrow => ShapeGeometry::LeftArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::RightArrow => ShapeGeometry::RightArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::LeftRightArrow => ShapeGeometry::LeftRightArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::UpDownArrow => ShapeGeometry::UpDownArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::NotchedRightArrow => ShapeGeometry::NotchedRightArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::StripedRightArrow => ShapeGeometry::StripedRightArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::SwooshArrow => ShapeGeometry::SwooshArrow {
                head_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::CloudCallout => ShapeGeometry::CloudCallout {
                tail_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                tail_y: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::WedgeEllipseCallout => ShapeGeometry::WedgeEllipseCallout {
                tail_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                tail_y: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::WedgeRectangleCallout => ShapeGeometry::WedgeRectangleCallout {
                tail_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                tail_y: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::WedgeRoundedRectangleCallout => {
                ShapeGeometry::WedgeRoundedRectangleCallout {
                    tail_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                    tail_y: self.fraction(interner, "adj2", FRACTION_DENOM),
                }
            }
            PresetShapeType::RoundSameSideCornersRectangle => {
                ShapeGeometry::RoundSameSideCornersRectangle {
                    top_corner_radius: self.fraction(interner, "adj1", FRACTION_DENOM),
                    bottom_corner_radius: self.fraction(interner, "adj2", FRACTION_DENOM),
                }
            }
            PresetShapeType::RoundDiagonalCornersRectangle => {
                ShapeGeometry::RoundDiagonalCornersRectangle {
                    top_left_bottom_right_radius: self.fraction(interner, "adj1", FRACTION_DENOM),
                    top_right_bottom_left_radius: self.fraction(interner, "adj2", FRACTION_DENOM),
                }
            }
            PresetShapeType::SnipSameSideCornersRectangle => {
                ShapeGeometry::SnipSameSideCornersRectangle {
                    top_corner_snip: self.fraction(interner, "adj1", FRACTION_DENOM),
                    bottom_corner_snip: self.fraction(interner, "adj2", FRACTION_DENOM),
                }
            }
            PresetShapeType::SnipDiagonalCornersRectangle => {
                ShapeGeometry::SnipDiagonalCornersRectangle {
                    top_left_bottom_right_snip: self.fraction(interner, "adj1", FRACTION_DENOM),
                    top_right_bottom_left_snip: self.fraction(interner, "adj2", FRACTION_DENOM),
                }
            }
            PresetShapeType::SnipAndRoundSingleCornerRectangle => {
                ShapeGeometry::SnipAndRoundSingleCornerRectangle {
                    round_corner_radius: self.fraction(interner, "adj1", FRACTION_DENOM),
                    snip_corner_size: self.fraction(interner, "adj2", FRACTION_DENOM),
                }
            }
            PresetShapeType::LeftBrace => ShapeGeometry::LeftBrace {
                curl_radius: self.fraction(interner, "adj1", FRACTION_DENOM),
                point_position: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::RightBrace => ShapeGeometry::RightBrace {
                curl_radius: self.fraction(interner, "adj1", FRACTION_DENOM),
                point_position: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Ribbon => ShapeGeometry::Ribbon {
                band_height: self.fraction(interner, "adj1", FRACTION_DENOM),
                panel_width: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Ribbon2 => ShapeGeometry::Ribbon2 {
                band_height: self.fraction(interner, "adj1", FRACTION_DENOM),
                panel_width: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Wave => ShapeGeometry::Wave {
                amplitude: self.fraction(interner, "adj1", FRACTION_DENOM),
                skew: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::DoubleWave => ShapeGeometry::DoubleWave {
                amplitude: self.fraction(interner, "adj1", FRACTION_DENOM),
                skew: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Gear6 => ShapeGeometry::Gear6 {
                tooth_depth: self.fraction(interner, "adj1", FRACTION_DENOM),
                tooth_width: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Gear9 => ShapeGeometry::Gear9 {
                tooth_depth: self.fraction(interner, "adj1", FRACTION_DENOM),
                tooth_width: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::BentConnector4 => ShapeGeometry::BentConnector4 {
                bend_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                bend_y: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::CurvedConnector4 => ShapeGeometry::CurvedConnector4 {
                bend_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                bend_y: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Corner => ShapeGeometry::Corner {
                horizontal_arm_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertical_arm_thickness: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::HalfFrame => ShapeGeometry::HalfFrame {
                top_arm_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                side_arm_thickness: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::MathEqual => ShapeGeometry::MathEqual {
                bar_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                bar_gap: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::NonIsoscelesTrapezoid => ShapeGeometry::NonIsoscelesTrapezoid {
                left_top_inset: self.fraction(interner, "adj1", FRACTION_DENOM),
                right_top_inset: self.fraction(interner, "adj2", FRACTION_DENOM),
            },
            PresetShapeType::Callout1 => ShapeGeometry::Callout1 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::Callout2 => ShapeGeometry::Callout2 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
            },
            PresetShapeType::Callout3 => ShapeGeometry::Callout3 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
                vertex4_x: self.fraction(interner, "adj8", FRACTION_DENOM),
                vertex4_y: self.fraction(interner, "adj7", FRACTION_DENOM),
            },
            PresetShapeType::AccentCallout1 => ShapeGeometry::AccentCallout1 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::AccentCallout2 => ShapeGeometry::AccentCallout2 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
            },
            PresetShapeType::AccentCallout3 => ShapeGeometry::AccentCallout3 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
                vertex4_x: self.fraction(interner, "adj8", FRACTION_DENOM),
                vertex4_y: self.fraction(interner, "adj7", FRACTION_DENOM),
            },
            PresetShapeType::BorderCallout1 => ShapeGeometry::BorderCallout1 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::BorderCallout2 => ShapeGeometry::BorderCallout2 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
            },
            PresetShapeType::BorderCallout3 => ShapeGeometry::BorderCallout3 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
                vertex4_x: self.fraction(interner, "adj8", FRACTION_DENOM),
                vertex4_y: self.fraction(interner, "adj7", FRACTION_DENOM),
            },
            PresetShapeType::AccentBorderCallout1 => ShapeGeometry::AccentBorderCallout1 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::AccentBorderCallout2 => ShapeGeometry::AccentBorderCallout2 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
            },
            PresetShapeType::AccentBorderCallout3 => ShapeGeometry::AccentBorderCallout3 {
                vertex1_x: self.fraction(interner, "adj2", FRACTION_DENOM),
                vertex1_y: self.fraction(interner, "adj1", FRACTION_DENOM),
                vertex2_x: self.fraction(interner, "adj4", FRACTION_DENOM),
                vertex2_y: self.fraction(interner, "adj3", FRACTION_DENOM),
                vertex3_x: self.fraction(interner, "adj6", FRACTION_DENOM),
                vertex3_y: self.fraction(interner, "adj5", FRACTION_DENOM),
                vertex4_x: self.fraction(interner, "adj8", FRACTION_DENOM),
                vertex4_y: self.fraction(interner, "adj7", FRACTION_DENOM),
            },
            PresetShapeType::BentConnector5 => ShapeGeometry::BentConnector5 {
                bend1_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                bend2_y: self.fraction(interner, "adj2", FRACTION_DENOM),
                bend3_x: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::CurvedConnector5 => ShapeGeometry::CurvedConnector5 {
                bend1_x: self.fraction(interner, "adj1", FRACTION_DENOM),
                bend2_y: self.fraction(interner, "adj2", FRACTION_DENOM),
                bend3_x: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::CurvedDownArrow => ShapeGeometry::CurvedDownArrow {
                body_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::CurvedUpArrow => ShapeGeometry::CurvedUpArrow {
                body_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::CurvedLeftArrow => ShapeGeometry::CurvedLeftArrow {
                body_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::CurvedRightArrow => ShapeGeometry::CurvedRightArrow {
                body_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::EllipseRibbon => ShapeGeometry::EllipseRibbon {
                arch_height: self.fraction(interner, "adj1", FRACTION_DENOM),
                center_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                fold_thickness: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::EllipseRibbon2 => ShapeGeometry::EllipseRibbon2 {
                arch_height: self.fraction(interner, "adj1", FRACTION_DENOM),
                center_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                fold_thickness: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::LeftRightRibbon => ShapeGeometry::LeftRightRibbon {
                band_height: self.fraction(interner, "adj1", FRACTION_DENOM),
                end_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                center_fold: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::BentUpArrow => ShapeGeometry::BentUpArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::LeftUpArrow => ShapeGeometry::LeftUpArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::LeftRightUpArrow => ShapeGeometry::LeftRightUpArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
            },
            PresetShapeType::QuadArrow => ShapeGeometry::QuadArrow {
                shaft_thickness: self.fraction(interner, "adj1", FRACTION_DENOM),
                head_width: self.fraction(interner, "adj2", FRACTION_DENOM),
                head_length: self.fraction(interner, "adj3", FRACTION_DENOM),
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
            ShapeGeometry::Arc {
                start_angle,
                end_angle,
            } => {
                self.apply_angle(interner, PresetShapeType::Arc, "adj1", start_angle);
                self.apply_angle(interner, PresetShapeType::Arc, "adj2", end_angle);
            }
            ShapeGeometry::Chord {
                start_angle,
                end_angle,
            } => {
                self.apply_angle(interner, PresetShapeType::Chord, "adj1", start_angle);
                self.apply_angle(interner, PresetShapeType::Chord, "adj2", end_angle);
            }
            ShapeGeometry::Pie {
                start_angle,
                end_angle,
            } => {
                self.apply_angle(interner, PresetShapeType::Pie, "adj1", start_angle);
                self.apply_angle(interner, PresetShapeType::Pie, "adj2", end_angle);
            }
            ShapeGeometry::DownArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::DownArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::DownArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::RightArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::RightArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::RightArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftRightArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftRightArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftRightArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::UpDownArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::UpDownArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::UpDownArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::NotchedRightArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::NotchedRightArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::NotchedRightArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::StripedRightArrow {
                shaft_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::StripedRightArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::StripedRightArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::SwooshArrow {
                head_thickness,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::SwooshArrow,
                    "adj1",
                    head_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::SwooshArrow,
                    "adj2",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CloudCallout { tail_x, tail_y } => {
                self.apply(
                    interner,
                    PresetShapeType::CloudCallout,
                    "adj1",
                    tail_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CloudCallout,
                    "adj2",
                    tail_y,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::WedgeEllipseCallout { tail_x, tail_y } => {
                self.apply(
                    interner,
                    PresetShapeType::WedgeEllipseCallout,
                    "adj1",
                    tail_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::WedgeEllipseCallout,
                    "adj2",
                    tail_y,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::WedgeRectangleCallout { tail_x, tail_y } => {
                self.apply(
                    interner,
                    PresetShapeType::WedgeRectangleCallout,
                    "adj1",
                    tail_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::WedgeRectangleCallout,
                    "adj2",
                    tail_y,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::WedgeRoundedRectangleCallout { tail_x, tail_y } => {
                self.apply(
                    interner,
                    PresetShapeType::WedgeRoundedRectangleCallout,
                    "adj1",
                    tail_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::WedgeRoundedRectangleCallout,
                    "adj2",
                    tail_y,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::RoundSameSideCornersRectangle {
                top_corner_radius,
                bottom_corner_radius,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::RoundSameSideCornersRectangle,
                    "adj1",
                    top_corner_radius,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::RoundSameSideCornersRectangle,
                    "adj2",
                    bottom_corner_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::RoundDiagonalCornersRectangle {
                top_left_bottom_right_radius,
                top_right_bottom_left_radius,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::RoundDiagonalCornersRectangle,
                    "adj1",
                    top_left_bottom_right_radius,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::RoundDiagonalCornersRectangle,
                    "adj2",
                    top_right_bottom_left_radius,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::SnipSameSideCornersRectangle {
                top_corner_snip,
                bottom_corner_snip,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::SnipSameSideCornersRectangle,
                    "adj1",
                    top_corner_snip,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::SnipSameSideCornersRectangle,
                    "adj2",
                    bottom_corner_snip,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::SnipDiagonalCornersRectangle {
                top_left_bottom_right_snip,
                top_right_bottom_left_snip,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::SnipDiagonalCornersRectangle,
                    "adj1",
                    top_left_bottom_right_snip,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::SnipDiagonalCornersRectangle,
                    "adj2",
                    top_right_bottom_left_snip,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::SnipAndRoundSingleCornerRectangle {
                round_corner_radius,
                snip_corner_size,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::SnipAndRoundSingleCornerRectangle,
                    "adj1",
                    round_corner_radius,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::SnipAndRoundSingleCornerRectangle,
                    "adj2",
                    snip_corner_size,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftBrace {
                curl_radius,
                point_position,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftBrace,
                    "adj1",
                    curl_radius,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftBrace,
                    "adj2",
                    point_position,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::RightBrace {
                curl_radius,
                point_position,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::RightBrace,
                    "adj1",
                    curl_radius,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::RightBrace,
                    "adj2",
                    point_position,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Ribbon {
                band_height,
                panel_width,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Ribbon,
                    "adj1",
                    band_height,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Ribbon,
                    "adj2",
                    panel_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Ribbon2 {
                band_height,
                panel_width,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Ribbon2,
                    "adj1",
                    band_height,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Ribbon2,
                    "adj2",
                    panel_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Wave { amplitude, skew } => {
                self.apply(
                    interner,
                    PresetShapeType::Wave,
                    "adj1",
                    amplitude,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Wave,
                    "adj2",
                    skew,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::DoubleWave { amplitude, skew } => {
                self.apply(
                    interner,
                    PresetShapeType::DoubleWave,
                    "adj1",
                    amplitude,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::DoubleWave,
                    "adj2",
                    skew,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Gear6 {
                tooth_depth,
                tooth_width,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Gear6,
                    "adj1",
                    tooth_depth,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Gear6,
                    "adj2",
                    tooth_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Gear9 {
                tooth_depth,
                tooth_width,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Gear9,
                    "adj1",
                    tooth_depth,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Gear9,
                    "adj2",
                    tooth_width,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BentConnector4 { bend_x, bend_y } => {
                self.apply(
                    interner,
                    PresetShapeType::BentConnector4,
                    "adj1",
                    bend_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BentConnector4,
                    "adj2",
                    bend_y,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedConnector4 { bend_x, bend_y } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedConnector4,
                    "adj1",
                    bend_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedConnector4,
                    "adj2",
                    bend_y,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Corner {
                horizontal_arm_thickness,
                vertical_arm_thickness,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Corner,
                    "adj1",
                    horizontal_arm_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Corner,
                    "adj2",
                    vertical_arm_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::HalfFrame {
                top_arm_thickness,
                side_arm_thickness,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::HalfFrame,
                    "adj1",
                    top_arm_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::HalfFrame,
                    "adj2",
                    side_arm_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::MathEqual {
                bar_thickness,
                bar_gap,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::MathEqual,
                    "adj1",
                    bar_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::MathEqual,
                    "adj2",
                    bar_gap,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::NonIsoscelesTrapezoid {
                left_top_inset,
                right_top_inset,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::NonIsoscelesTrapezoid,
                    "adj1",
                    left_top_inset,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::NonIsoscelesTrapezoid,
                    "adj2",
                    right_top_inset,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Callout1 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Callout1,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout1,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout1,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout1,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Callout2 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Callout2,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout2,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout2,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout2,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout2,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout2,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::Callout3 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
                vertex4_x,
                vertex4_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj7",
                    vertex4_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::Callout3,
                    "adj8",
                    vertex4_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::AccentCallout1 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout1,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout1,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout1,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout1,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::AccentCallout2 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout2,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout2,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout2,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout2,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout2,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout2,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::AccentCallout3 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
                vertex4_x,
                vertex4_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj7",
                    vertex4_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentCallout3,
                    "adj8",
                    vertex4_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BorderCallout1 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout1,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout1,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout1,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout1,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BorderCallout2 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout2,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout2,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout2,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout2,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout2,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout2,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BorderCallout3 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
                vertex4_x,
                vertex4_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj7",
                    vertex4_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BorderCallout3,
                    "adj8",
                    vertex4_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::AccentBorderCallout1 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout1,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout1,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout1,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout1,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::AccentBorderCallout2 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout2,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout2,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout2,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout2,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout2,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout2,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::AccentBorderCallout3 {
                vertex1_x,
                vertex1_y,
                vertex2_x,
                vertex2_y,
                vertex3_x,
                vertex3_y,
                vertex4_x,
                vertex4_y,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj1",
                    vertex1_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj2",
                    vertex1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj3",
                    vertex2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj4",
                    vertex2_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj5",
                    vertex3_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj6",
                    vertex3_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj7",
                    vertex4_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::AccentBorderCallout3,
                    "adj8",
                    vertex4_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BentConnector5 {
                bend1_x,
                bend2_y,
                bend3_x,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::BentConnector5,
                    "adj1",
                    bend1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BentConnector5,
                    "adj2",
                    bend2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BentConnector5,
                    "adj3",
                    bend3_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedConnector5 {
                bend1_x,
                bend2_y,
                bend3_x,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedConnector5,
                    "adj1",
                    bend1_x,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedConnector5,
                    "adj2",
                    bend2_y,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedConnector5,
                    "adj3",
                    bend3_x,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedDownArrow {
                body_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedDownArrow,
                    "adj1",
                    body_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedDownArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedDownArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedUpArrow {
                body_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedUpArrow,
                    "adj1",
                    body_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedUpArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedUpArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedLeftArrow {
                body_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedLeftArrow,
                    "adj1",
                    body_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedLeftArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedLeftArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::CurvedRightArrow {
                body_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::CurvedRightArrow,
                    "adj1",
                    body_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedRightArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::CurvedRightArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::EllipseRibbon {
                arch_height,
                center_width,
                fold_thickness,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::EllipseRibbon,
                    "adj1",
                    arch_height,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::EllipseRibbon,
                    "adj2",
                    center_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::EllipseRibbon,
                    "adj3",
                    fold_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::EllipseRibbon2 {
                arch_height,
                center_width,
                fold_thickness,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::EllipseRibbon2,
                    "adj1",
                    arch_height,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::EllipseRibbon2,
                    "adj2",
                    center_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::EllipseRibbon2,
                    "adj3",
                    fold_thickness,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftRightRibbon {
                band_height,
                end_width,
                center_fold,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftRightRibbon,
                    "adj1",
                    band_height,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftRightRibbon,
                    "adj2",
                    end_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftRightRibbon,
                    "adj3",
                    center_fold,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::BentUpArrow {
                shaft_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::BentUpArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BentUpArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::BentUpArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftUpArrow {
                shaft_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftUpArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftUpArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftUpArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::LeftRightUpArrow {
                shaft_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::LeftRightUpArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftRightUpArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::LeftRightUpArrow,
                    "adj3",
                    head_length,
                    FRACTION_DENOM,
                );
            }
            ShapeGeometry::QuadArrow {
                shaft_thickness,
                head_width,
                head_length,
            } => {
                self.apply(
                    interner,
                    PresetShapeType::QuadArrow,
                    "adj1",
                    shaft_thickness,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::QuadArrow,
                    "adj2",
                    head_width,
                    FRACTION_DENOM,
                );
                self.apply(
                    interner,
                    PresetShapeType::QuadArrow,
                    "adj3",
                    head_length,
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

    /// Reads angle adjustment `wire` (override or default; native 60000ths of a degree) as an [`Angle`].
    fn angle(&self, interner: &Interner, wire: &str) -> Angle {
        let native = self.adjustment(interner, wire).unwrap_or(0);
        Angle::from_degrees(f64::from(native) / 60_000.0)
    }

    /// Sets `preset` and writes `value` to angle adjustment `wire` in native units (60000ths of a degree).
    fn apply_angle(
        &mut self,
        interner: &mut Interner,
        preset: PresetShapeType,
        wire: &str,
        value: Angle,
    ) {
        self.set_preset(interner, preset);
        let native = (value.degrees() * 60_000.0).round() as i32;
        self.set_adjustment(interner, wire, native);
    }
}
