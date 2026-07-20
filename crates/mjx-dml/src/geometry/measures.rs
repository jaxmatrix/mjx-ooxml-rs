//! Friendly measures for the typed DrawingML tiers.
//!
//! The wire forms store measures in native spec units (fractions in 1000ths of a percent, angles in
//! 60000ths of a degree, lengths in EMU). These self-explanatory newtypes convert those into intent.
//! [`Fraction`] covers the fraction-valued shape adjustments; [`Angle`] the angular ones (`arc`,
//! `chord`, `pie`); [`LineWidth`] the outline width (`a:ln@w`). `Points` (length) arrives with the
//! batches that use it.

/// A fraction of some geometric reference named by the field that holds it (e.g. a corner radius as a
/// fraction of the shorter side). `1.0` is 100%. A value may exceed `1.0` (e.g. a connector's bend
/// position) or be **negative** (e.g. a smiley's mouth curve, where the sign flips smile ↔ frown)
/// where the shape allows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fraction(f64);

impl Fraction {
    /// Wraps a ratio (`1.0` = 100%).
    #[must_use]
    pub const fn from_ratio(ratio: f64) -> Self {
        Self(ratio)
    }

    /// The ratio (`1.0` = 100%).
    #[must_use]
    pub const fn ratio(self) -> f64 {
        self.0
    }
}

/// An angle, stored in **radians**. A shape's angular adjustments (a pie/arc/chord's start and end)
/// are read and written through this; construct from and read as radians or degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(f64);

impl Angle {
    /// Wraps an angle given in radians.
    #[must_use]
    pub const fn from_radians(radians: f64) -> Self {
        Self(radians)
    }

    /// Wraps an angle given in degrees.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Self {
        Self(degrees.to_radians())
    }

    /// The angle in radians.
    #[must_use]
    pub const fn radians(self) -> f64 {
        self.0
    }

    /// The angle in degrees.
    #[must_use]
    pub fn degrees(self) -> f64 {
        self.0.to_degrees()
    }
}

/// English Metric Units per point (`72` points per inch, `914400` EMU per inch → `12700`).
const EMU_PER_POINT: i64 = 12_700;

/// An outline width, stored in **English Metric Units** (`a:ln@w`, `ST_LineWidth`; EMU 0..=20116800).
/// Construct from and read as EMU or points — PowerPoint's line-weight UI is in points, and one point
/// is `12700` EMU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineWidth(i64);

impl LineWidth {
    /// Wraps a width given in EMU.
    #[must_use]
    pub const fn from_emu(emu: i64) -> Self {
        Self(emu)
    }

    /// The width in EMU.
    #[must_use]
    pub const fn emu(self) -> i64 {
        self.0
    }

    /// Wraps a width given in points (one point = `12700` EMU), rounded to the nearest EMU.
    #[must_use]
    pub fn from_points(points: f64) -> Self {
        Self((points * EMU_PER_POINT as f64).round() as i64)
    }

    /// The width in points (one point = `12700` EMU).
    #[must_use]
    pub fn points(self) -> f64 {
        self.0 as f64 / EMU_PER_POINT as f64
    }
}

/// Hundredths of a point — the unit DrawingML writes text measurements in (`sz="4400"` is 44 pt).
const HUNDREDTHS_PER_POINT: f64 = 100.0;

/// A font size **in points** — `FontSize::from_points(10.5)` is what a reader would call "10½ point".
///
/// Points are the only unit this type talks about, because points are the unit every font-size
/// control in every application is in. The file itself stores hundredths of a point (`a:rPr@sz`,
/// `ST_TextFontSize`), which is why the value is *stored* as an integer and why sizes are exact to
/// half a point and finer — but that is the wire's business, reachable through [`from_wire`] /
/// [`to_wire`] where a serializer needs it, and nowhere else.
///
/// The schema's range is `100..=400000` (1 pt to 4000 pt). It is documented rather than enforced, as
/// with every other measure here: a file may carry an out-of-range value, and reading one must not
/// fail.
///
/// [`from_wire`]: FontSize::from_wire
/// [`to_wire`]: FontSize::to_wire
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontSize(i32);

impl FontSize {
    /// A size given in points — `18.0` is eighteen point, `10.5` is ten and a half.
    ///
    /// Rounded to the nearest hundredth of a point, the finest distinction the format records.
    #[must_use]
    pub fn from_points(points: f64) -> Self {
        Self((points * HUNDREDTHS_PER_POINT).round() as i32)
    }

    /// The size in points.
    #[must_use]
    pub fn points(self) -> f64 {
        f64::from(self.0) / HUNDREDTHS_PER_POINT
    }

    /// Wraps the value exactly as written in the file (hundredths of a point) — for de/serialization.
    /// Callers reasoning about type size want [`from_points`](Self::from_points).
    #[must_use]
    pub const fn from_wire(hundredths_of_a_point: i32) -> Self {
        Self(hundredths_of_a_point)
    }

    /// The value exactly as written in the file (hundredths of a point) — for de/serialization.
    /// Callers reasoning about type size want [`points`](Self::points).
    #[must_use]
    pub const fn to_wire(self) -> i32 {
        self.0
    }
}

/// A text measurement **in points** that is not a font size: character spacing (`a:rPr@spc` — a
/// negative value tightens) and the kerning threshold (`a:rPr@kern` — the size from which kerning
/// applies).
///
/// Points on the surface, hundredths of a point on the wire (`ST_TextPoint` /
/// `ST_TextNonNegativePoint`, both bounded to ±4000 pt), for the same reason as [`FontSize`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPoint(i32);

impl TextPoint {
    /// A measurement given in points — `-0.5` tightens spacing by half a point.
    ///
    /// Rounded to the nearest hundredth of a point, the finest distinction the format records.
    #[must_use]
    pub fn from_points(points: f64) -> Self {
        Self((points * HUNDREDTHS_PER_POINT).round() as i32)
    }

    /// The measurement in points.
    #[must_use]
    pub fn points(self) -> f64 {
        f64::from(self.0) / HUNDREDTHS_PER_POINT
    }

    /// Wraps the value exactly as written in the file (hundredths of a point) — for de/serialization.
    /// Callers reasoning about spacing want [`from_points`](Self::from_points).
    #[must_use]
    pub const fn from_wire(hundredths_of_a_point: i32) -> Self {
        Self(hundredths_of_a_point)
    }

    /// The value exactly as written in the file (hundredths of a point) — for de/serialization.
    /// Callers reasoning about spacing want [`points`](Self::points).
    #[must_use]
    pub const fn to_wire(self) -> i32 {
        self.0
    }
}

/// A general length in **English Metric Units** (`914400` EMU per inch, `12700` per point) — the
/// spec's `ST_Coordinate`/`ST_PositiveCoordinate` family. Used by the effect measures (a blur/shadow
/// radius, a shadow distance, a soft-edge radius) that carry a raw EMU length with no dedicated
/// newtype of their own. Construct from and read as EMU or points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Emu(i64);

impl Emu {
    /// Wraps a length given in EMU.
    #[must_use]
    pub const fn from_emu(emu: i64) -> Self {
        Self(emu)
    }

    /// The length in EMU.
    #[must_use]
    pub const fn emu(self) -> i64 {
        self.0
    }

    /// Wraps a length given in points (one point = `12700` EMU), rounded to the nearest EMU.
    #[must_use]
    pub fn from_points(points: f64) -> Self {
        Self((points * EMU_PER_POINT as f64).round() as i64)
    }

    /// The length in points (one point = `12700` EMU).
    #[must_use]
    pub fn points(self) -> f64 {
        self.0 as f64 / EMU_PER_POINT as f64
    }
}
