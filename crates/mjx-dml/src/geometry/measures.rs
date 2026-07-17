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
