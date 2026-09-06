//! The two measures that are not anybody's markup: a length in English Metric Units, and an angle.
//!
//! # Why these live at rank 0.0 and not in `mjx-dml`
//!
//! Both types were written for DrawingML and lived in `mjx-dml::geometry::measures` until
//! MJXOFF-160. They are not DrawingML-specific: an EMU is the unit `a:off@x`, `w:pgSz@w` (through
//! twips, `635` EMU each), `xdr:colOff` and every drawing anchor in all three formats are finally
//! expressed in, and the client platform's box model — `mjx-layout`, rank 1.6 — positions every
//! fragment it produces in the same unit. `mjx-layout` may not depend on `mjx-dml` (2.0) and must
//! not define a second `Emu`, because two EMU types in one workspace is exactly the class of defect
//! the layering rule exists to prevent: they would be interconvertible only through a cast, and one
//! of them would eventually be wrong.
//!
//! So the single definition sits at the floor of the graph, where every tier can reach it, and
//! `mjx-dml::geometry::measures` re-exports it. There is exactly one `Emu` and one `Angle` in the
//! workspace, and this is where they are.
//!
//! # Why the arithmetic saturates
//!
//! Both a document and a layout can produce absurd numbers — a `.pptx` may say
//! `off x="9223372036854775807"`, and a table with ten thousand nested frames can accumulate one.
//! Rust's `+` panics on overflow in a debug build, and `mjx-layout`'s contract is that a
//! pathological document produces a bad-looking page and never a crash. So [`Emu`]'s operators
//! saturate, and the saturation point is not a practical limit: `i64::MAX` EMU is about a quarter of
//! a light-year.

use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// English Metric Units per point (`72` points per inch, `914400` EMU per inch → `12700`).
pub const EMU_PER_POINT: i64 = 12_700;

/// English Metric Units per inch, the definition the rest are derived from.
pub const EMU_PER_INCH: i64 = 914_400;

/// English Metric Units per twip (a twentieth of a point — `w:pgSz@w`, `w:ind@left`, `w:tab@pos`).
pub const EMU_PER_TWIP: i64 = EMU_PER_POINT / 20;

/// English Metric Units per centimetre.
pub const EMU_PER_CENTIMETRE: i64 = 360_000;

/// A general length in **English Metric Units** (`914400` EMU per inch, `12700` per point) — the
/// spec's `ST_Coordinate`/`ST_PositiveCoordinate` family, and the unit the box model positions every
/// fragment in.
///
/// EMU is an integer unit chosen so that the units every part of Office actually writes divide into
/// it exactly: a point is `12700`, a twip is `635`, a centimetre is `360000`, an inch is `914400`.
/// That is why layout is done in it rather than in floating-point points — a page of accumulated
/// `f64` additions drifts, and two boxes that should share an edge stop sharing it.
///
/// ```
/// use mjx_ooxml_core::measure::Emu;
///
/// assert_eq!(Emu::from_points(1.0).emu(), 12_700);
/// assert_eq!(Emu::from_twips(20).emu(), 12_700);
/// assert_eq!((Emu::from_emu(3) + Emu::from_emu(4)).emu(), 7);
/// // Saturating, not panicking: a document may carry anything.
/// assert_eq!((Emu::MAXIMUM + Emu::from_emu(1)), Emu::MAXIMUM);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Emu(i64);

impl Emu {
    /// Zero length.
    pub const ZERO: Self = Self(0);

    /// The largest length this type can hold — about a quarter of a light-year.
    pub const MAXIMUM: Self = Self(i64::MAX);

    /// The most negative length this type can hold.
    pub const MINIMUM: Self = Self(i64::MIN);

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

    /// Wraps a length already in EMU but carried as a `f64`, rounding and saturating.
    ///
    /// The affine arithmetic a box model does — a group's child coordinate space, a shape's
    /// rotation — is a ratio applied to a length, so it happens in `f64` and comes back needing
    /// exactly this conversion. Going through [`Emu::from_points`] instead would divide and
    /// multiply by `12700` for nothing and lose precision doing it.
    #[must_use]
    pub fn from_emu_rounded(emu: f64) -> Self {
        Self::from_f64(emu)
    }

    /// Wraps a length given in points (one point = `12700` EMU), rounded to the nearest EMU.
    ///
    /// A non-finite or out-of-range value saturates rather than becoming an unspecified integer: a
    /// point measurement reaches here from a shaped run's advance, and a malformed face can make one
    /// infinite.
    #[must_use]
    pub fn from_points(points: f64) -> Self {
        Self::from_f64(points * EMU_PER_POINT as f64)
    }

    /// The length in points (one point = `12700` EMU).
    #[must_use]
    pub fn points(self) -> f64 {
        self.0 as f64 / EMU_PER_POINT as f64
    }

    /// Wraps a length given in twips — a twentieth of a point, which is the unit WordprocessingML
    /// writes page sizes, margins, indents and tab stops in. Exact: one twip is `635` EMU.
    #[must_use]
    pub const fn from_twips(twips: i64) -> Self {
        Self(twips.saturating_mul(EMU_PER_TWIP))
    }

    /// The length in twips, truncated toward zero.
    #[must_use]
    pub const fn twips(self) -> i64 {
        self.0 / EMU_PER_TWIP
    }

    /// Wraps a length given in inches.
    #[must_use]
    pub fn from_inches(inches: f64) -> Self {
        Self::from_f64(inches * EMU_PER_INCH as f64)
    }

    /// The length in inches.
    #[must_use]
    pub fn inches(self) -> f64 {
        self.0 as f64 / EMU_PER_INCH as f64
    }

    /// The length with its sign removed, saturating at [`Emu::MAXIMUM`] rather than wrapping on
    /// [`Emu::MINIMUM`].
    #[must_use]
    pub const fn absolute(self) -> Self {
        Self(self.0.saturating_abs())
    }

    /// The smaller of two lengths.
    #[must_use]
    pub const fn minimum(self, other: Self) -> Self {
        if self.0 <= other.0 {
            self
        } else {
            other
        }
    }

    /// The larger of two lengths.
    #[must_use]
    pub const fn maximum(self, other: Self) -> Self {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }

    /// The length scaled by `factor`, saturating.
    #[must_use]
    pub fn scaled_by(self, factor: f64) -> Self {
        Self::from_f64(self.0 as f64 * factor)
    }

    /// The length multiplied by a whole number, saturating.
    #[must_use]
    pub const fn times(self, count: i64) -> Self {
        Self(self.0.saturating_mul(count))
    }

    /// The length divided by a whole number, or [`Emu::ZERO`] for a divisor of zero.
    ///
    /// Division is the one arithmetic operation on an `i64` that can fail two ways — a zero divisor
    /// and `i64::MIN / -1` — and neither may panic on a layout path.
    #[must_use]
    pub const fn divided_by(self, divisor: i64) -> Self {
        match divisor {
            0 => Self::ZERO,
            -1 => Self(self.0.saturating_neg()),
            divisor => Self(self.0 / divisor),
        }
    }

    /// A `f64` in EMU, clamped into range and rounded. `NaN` becomes zero.
    fn from_f64(value: f64) -> Self {
        if value.is_nan() {
            return Self::ZERO;
        }
        let rounded = value.round();
        if rounded >= i64::MAX as f64 {
            return Self::MAXIMUM;
        }
        if rounded <= i64::MIN as f64 {
            return Self::MINIMUM;
        }
        // Finite and inside the range checked above, so the cast is exact.
        Self(rounded as i64)
    }
}

impl Add for Emu {
    type Output = Self;

    /// Saturating — see the module documentation.
    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Emu {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for Emu {
    type Output = Self;

    /// Saturating — see the module documentation.
    fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl SubAssign for Emu {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Neg for Emu {
    type Output = Self;

    /// Saturating — see the module documentation.
    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

impl std::iter::Sum for Emu {
    fn sum<I: Iterator<Item = Self>>(iterator: I) -> Self {
        iterator.fold(Self::ZERO, |total, length| total + length)
    }
}

/// An angle, stored in **radians**. A shape's angular adjustments (a pie/arc/chord's start and end)
/// are read and written through this, and a box model's rotation is expressed in it; construct from
/// and read as radians or degrees.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Angle(f64);

impl Angle {
    /// No rotation.
    pub const ZERO: Self = Self(0.0);

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
