//! Friendly measures for the typed shape-geometry tier.
//!
//! The mechanical tier stores adjustments in native spec units (fractions in 1000ths of a percent,
//! angles in 60000ths of a degree). The typed tier converts those into these self-explanatory
//! measures. Only [`Fraction`] is needed so far — the single-adjustment shapes are all fraction-valued;
//! `Angle` (radians) and `Points` (length) arrive with the batches that use them.

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
