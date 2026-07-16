//! Friendly measures for the typed shape-geometry tier.
//!
//! The mechanical tier stores adjustments in native spec units (fractions in 1000ths of a percent,
//! angles in 60000ths of a degree). The typed tier converts those into these self-explanatory
//! measures. [`Fraction`] covers the fraction-valued adjustments; [`Angle`] the angular ones (`arc`,
//! `chord`, `pie`). `Points` (length) arrives with the batches that use it.

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
