//! Axis scaling and tick selection — the arithmetic that has correct answers.
//!
//! # Why this is its own module, and why it is asserted as numbers
//!
//! Everything else in this crate is placement: given a scale, a bar is a rectangle and a slice is an
//! arc, and getting one wrong is visible. **Tick selection is not visible.** A value axis running
//! from 0 to 100 in steps of 10 looks right under any implementation at all, including one that
//! divided the data range linearly, so a chart with a convenient data range is a test that cannot
//! fail. That is why `tests/ticks_are_arithmetic.rs` uses ranges no naive division survives —
//! `0.003 … 0.017` and `−45 … 1203` — and asserts the tick values as literals.
//!
//! # Where the algorithm comes from
//!
//! The nice-number selection is **Paul Heckbert's**, *"Nice Numbers for Graph Labels"*, Graphics
//! Gems I (Academic Press, 1990), pp. 61–63: pick the step from the mantissa set {1, 2, 5, 10}
//! scaled by a power of ten, then extend the range outward to whole multiples of it. This is
//! published, it is what nearly every plotting library implements, and it is why the provenance of
//! [`nice_number`] and [`Scale::automatic`] is *DocumentedBehaviour* rather than a guess.
//!
//! **One deliberate deviation, and it is stated because it changes the answer.** Heckbert's
//! `loose_label` rounds *twice*: it first makes the data range nice, then divides that by the
//! interval count and makes the result nice again. The double rounding systematically overshoots —
//! for `−45 … 1203` over five intervals it gives a step of 500 and a lower bound of **−500**, which
//! puts a tenth of the plot below data that starts at −45. This engine derives the step from the
//! **raw** range instead (`nice_number(span / intervals, Rounding::Nearest)`), which gives 200 and
//! −200. That is *EngineDerived*, and it is the one place where the published algorithm and what
//! Office draws disagree in a way a reader would notice.
//!
//! # Floating point, and why a tick is not `min + k * step`
//!
//! A step of `0.002` is not representable, so eight accumulations of it are not `0.016`. Every tick
//! here is computed as `(index * numerator) / 10^decimals` from the step's **integer** mantissa, so
//! the ninth tick of a 0.002 step is `18 / 1000` — the double nearest `0.018`, which is the double a
//! test writes as a literal. Accumulating instead would put a `0.018000000000000002` on the axis and
//! in every label.

use std::fmt;

/// A tick spacing, kept as an integer mantissa and a power of ten rather than as a `f64`.
///
/// `numerator × 10^exponent` — so `0.002` is `2 × 10⁻³` and `500` is `5 × 10²`. Keeping the pair is
/// what lets [`Scale::tick`] compute the *k*-th tick by one multiplication and one division rather
/// than by *k* additions, which is the difference between a label reading `0.018` and one reading
/// `0.018000000000000002`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    numerator: i32,
    exponent: i32,
}

impl Step {
    /// The step `numerator × 10^exponent`. A non-positive numerator is raised to one, because a step
    /// of zero would make every tick the same tick and a negative one would run the axis backwards
    /// by arithmetic rather than by the `c:orientation` that means it.
    #[must_use]
    pub const fn new(numerator: i32, exponent: i32) -> Self {
        Self {
            numerator: if numerator > 0 { numerator } else { 1 },
            exponent,
        }
    }

    /// The step as a number.
    #[must_use]
    pub fn value(self) -> f64 {
        self.scaled(1)
    }

    /// `multiple × self`, computed from the integer mantissa so that whole multiples land on the
    /// doubles a reader would write down.
    #[must_use]
    pub fn scaled(self, multiple: i64) -> f64 {
        let mantissa = multiple.saturating_mul(i64::from(self.numerator)) as f64;
        if self.exponent >= 0 {
            mantissa * ten_to(self.exponent)
        } else {
            mantissa / ten_to(-self.exponent)
        }
    }

    /// How many digits after the decimal point a tick of this step needs, so a label is written
    /// `0.002` rather than `0.0020000000000000005`.
    #[must_use]
    pub fn decimals(self) -> usize {
        if self.exponent >= 0 {
            0
        } else {
            usize::try_from(-self.exponent).unwrap_or(0).min(15)
        }
    }

    /// The next nice step up — 1 → 2 → 5 → 10. Used when a step chosen from the raw range would put
    /// more ticks on an axis than it has room for.
    #[must_use]
    pub fn next_up(self) -> Self {
        match self.numerator {
            1 => Self::new(2, self.exponent),
            2 => Self::new(5, self.exponent),
            _ => Self::new(1, self.exponent.saturating_add(1)),
        }
    }

    /// The step divided into five, which is the minor unit Office uses when a file states none.
    ///
    /// `GUESS:` five is what a 1/2/5 major sequence divides into evenly and what Office appears to
    /// draw, but the fifth is not a number any schema states.
    #[must_use]
    pub fn fifth(self) -> Self {
        match self.numerator {
            1 => Self::new(2, self.exponent.saturating_sub(1)),
            2 => Self::new(4, self.exponent.saturating_sub(1)),
            5 => Self::new(1, self.exponent),
            other => Self::new(other.max(1), self.exponent),
        }
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.*}", self.decimals(), self.value())
    }
}

/// `10^power`, saturating rather than overflowing to infinity for a power no chart can produce.
fn ten_to(power: i32) -> f64 {
    10f64.powi(power.clamp(-300, 300))
}

/// Which way [`nice_number`] rounds its mantissa.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rounding {
    /// Up to the next member of the set — Heckbert's `round = false`. Used for a range.
    Up,
    /// To the nearest member — Heckbert's `round = true`. Used for a step.
    Nearest,
}

/// Heckbert's `nicenum`: the member of {1, 2, 5, 10} × 10ⁿ nearest (or next above) `rough`.
///
/// A non-finite or non-positive `rough` answers `1 × 10⁰`, because an axis has to have a step and
/// the honest one for a range that is not a range is one.
#[must_use]
pub fn nice_number(rough: f64, rounding: Rounding) -> Step {
    if !rough.is_finite() || rough <= 0.0 {
        return Step::new(1, 0);
    }
    let exponent = rough.log10().floor();
    let exponent = if exponent.is_finite() {
        exponent as i32
    } else {
        0
    };
    let fraction = rough / ten_to(exponent);
    let mantissa = match rounding {
        Rounding::Nearest => {
            if fraction < 1.5 {
                1
            } else if fraction < 3.0 {
                2
            } else if fraction < 7.0 {
                5
            } else {
                10
            }
        }
        Rounding::Up => {
            if fraction <= 1.0 {
                1
            } else if fraction <= 2.0 {
                2
            } else if fraction <= 5.0 {
                5
            } else {
                10
            }
        }
    };
    if mantissa == 10 {
        Step::new(1, exponent.saturating_add(1))
    } else {
        Step::new(mantissa, exponent)
    }
}

/// Whether a value axis must include zero.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Baseline {
    /// A bar, column or area chart: the mark's *length* is what a reader compares, so the axis is
    /// anchored at zero whenever the data does not cross it. This is not a preference — a bar chart
    /// whose axis starts at 90 misrepresents every bar on it, and Office never draws one.
    Anchored,
    /// A line, scatter, radar or bubble chart: the mark's *position* is what a reader compares, so
    /// the axis may float. Office still snaps to zero when the data sits close to it, which is the
    /// five-sixths rule in [`Scale::automatic`].
    Floating,
}

/// An axis' resolved scale: where it starts, where it ends, and how far apart its ticks are.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Scale {
    /// The value at the axis' origin end.
    pub minimum: f64,
    /// The value at its far end.
    pub maximum: f64,
    /// The major tick spacing.
    pub major: Step,
    /// The minor tick spacing.
    pub minor: Step,
    /// The base of a logarithmic scale, or `None` for a linear one.
    pub logarithm_base: Option<f64>,
    /// The index of the first major tick, as a multiple of [`major`](Self::major).
    first_tick: i64,
    /// How many major ticks there are, first and last included.
    tick_count: usize,
}

impl Scale {
    /// The scale Office picks for a linear axis over `low ..= high`, given how many intervals the
    /// plot has room for and whether the marks are anchored to zero.
    ///
    /// `intervals` is clamped to `1 ..= 30`: an axis has to have at least one interval, and a plot
    /// area a hundred EMU tall must not be handed a thousand ticks to draw.
    #[must_use]
    pub fn automatic(low: f64, high: f64, intervals: usize, baseline: Baseline) -> Self {
        let (low, high) = usable_bounds(low, high);
        let intervals = intervals.clamp(1, 30);

        // Zero-basing, before the step is chosen: it changes the span the step is derived from.
        //
        // `Anchored` is spec-derived in effect — a bar's length *is* its value, so the axis it is
        // measured against contains zero. `Floating`'s five-sixths test is `GUESS:`; Office snaps a
        // line chart's axis to zero when the data sits near it and the exact threshold is not
        // published, so this engine uses the ratio most widely reported for it.
        let anchored_low = match baseline {
            Baseline::Anchored if low >= 0.0 => 0.0,
            Baseline::Floating if low >= 0.0 && high > 0.0 && low <= high * (5.0 / 6.0) => 0.0,
            _ => low,
        };
        let anchored_high = match baseline {
            Baseline::Anchored if high <= 0.0 => 0.0,
            Baseline::Floating if high <= 0.0 && low < 0.0 && high >= low * (5.0 / 6.0) => 0.0,
            _ => high,
        };

        let span = anchored_high - anchored_low;
        let mut major = nice_number(span / intervals as f64, Rounding::Nearest);
        // The step comes from the raw span, so an awkward range can need more ticks than the plot
        // asked for. Half again is the tolerance; past it the step goes up one nice number. Two
        // steps are enough to cross a whole decade, and the bound is what keeps this a loop that
        // terminates rather than one that trusts arithmetic.
        let allowance = intervals.saturating_add(intervals / 2).max(2);
        for _ in 0..4 {
            if span_in_steps(anchored_low, anchored_high, major) <= allowance {
                break;
            }
            major = major.next_up();
        }

        Self::from_step(anchored_low, anchored_high, major, major.fifth(), None)
    }

    /// The scale for an axis whose file states some or all of its bounds, falling back to
    /// [`automatic`](Self::automatic) for each part the file leaves out.
    ///
    /// **The document wins.** A `c:min`, a `c:max` or a `c:majorUnit` is used exactly as written,
    /// including one that makes an ugly axis: a reader who set the maximum to 97 meant 97.
    #[must_use]
    pub fn resolved(
        low: f64,
        high: f64,
        intervals: usize,
        baseline: Baseline,
        stated: StatedScale,
    ) -> Self {
        let automatic = Self::automatic(low, high, intervals, baseline);
        let minimum = stated.minimum.unwrap_or(automatic.minimum);
        let maximum = stated.maximum.unwrap_or(automatic.maximum);
        let (minimum, maximum) = if maximum > minimum {
            (minimum, maximum)
        } else {
            // A file that states a maximum at or below its minimum describes no axis. Widening by
            // the automatic span keeps every later division non-zero without inventing a bound the
            // file did not state on the side it did state.
            (
                minimum,
                minimum + (automatic.maximum - automatic.minimum).abs().max(1.0),
            )
        };
        let major = match stated.major_unit {
            Some(unit) if unit.is_finite() && unit > 0.0 => {
                nice_number(unit, Rounding::Nearest).exactly_or(unit)
            }
            _ => automatic.major,
        };
        let minor = match stated.minor_unit {
            Some(unit) if unit.is_finite() && unit > 0.0 => {
                nice_number(unit, Rounding::Nearest).exactly_or(unit)
            }
            _ => major.fifth(),
        };
        let mut scale = Self::from_step(minimum, maximum, major, minor, stated.logarithm_base);
        // A stated bound is honoured exactly rather than snapped outward to the step, which is what
        // `from_step` would otherwise do.
        if stated.minimum.is_some() {
            scale.minimum = minimum;
        }
        if stated.maximum.is_some() {
            scale.maximum = maximum;
        }
        scale.retick();
        scale
    }

    /// The fixed 0 … 1 scale of a hundred-percent-stacked plot, whose axis is a proportion and never
    /// depends on the data at all.
    #[must_use]
    pub fn proportional() -> Self {
        Self::from_step(0.0, 1.0, Step::new(2, -1), Step::new(4, -2), None)
    }

    /// A logarithmic scale over `low ..= high` to `base`, with a major tick at every whole power.
    ///
    /// Non-positive data has no logarithm. Office plots nothing for such a point and this engine
    /// says so the same way: the bounds are taken from the positive values alone, and a series with
    /// none at all gets `1 … base`, which is an axis rather than a division by zero.
    #[must_use]
    pub fn logarithmic(low: f64, high: f64, base: f64) -> Self {
        let base = if base.is_finite() && base > 1.0 {
            base
        } else {
            10.0
        };
        let low = if low.is_finite() && low > 0.0 {
            low
        } else {
            1.0
        };
        let high = if high.is_finite() && high > low {
            high
        } else {
            low * base
        };
        let ln_base = base.ln();
        let first = (low.ln() / ln_base).floor();
        let last = (high.ln() / ln_base).ceil();
        let first = clamp_index(first);
        let last = clamp_index(last).max(first.saturating_add(1));
        Self {
            minimum: base.powf(first as f64),
            maximum: base.powf(last as f64),
            major: Step::new(1, 0),
            minor: Step::new(1, 0),
            logarithm_base: Some(base),
            first_tick: first,
            tick_count: usize::try_from(last - first + 1).unwrap_or(2).min(64),
        }
    }

    /// Builds a scale by extending `low ..= high` outward to whole multiples of `major`.
    fn from_step(low: f64, high: f64, major: Step, minor: Step, log: Option<f64>) -> Self {
        let unit = major.value();
        let first = clamp_index(floor_multiple(low, unit));
        let last = clamp_index(ceil_multiple(high, unit)).max(first.saturating_add(1));
        let mut scale = Self {
            minimum: major.scaled(first),
            maximum: major.scaled(last),
            major,
            minor,
            logarithm_base: log,
            first_tick: first,
            tick_count: usize::try_from(last - first + 1).unwrap_or(2),
        };
        scale.tick_count = scale.tick_count.clamp(2, 1024);
        scale
    }

    /// Recomputes the tick run after a bound was replaced by a stated one.
    fn retick(&mut self) {
        if self.logarithm_base.is_some() {
            return;
        }
        let unit = self.major.value();
        let first = clamp_index(ceil_multiple(self.minimum, unit));
        let last = clamp_index(floor_multiple(self.maximum, unit));
        self.first_tick = first;
        self.tick_count = usize::try_from(last - first + 1)
            .unwrap_or(1)
            .clamp(1, 1024);
    }

    /// How many major ticks the axis carries.
    #[must_use]
    pub fn tick_count(&self) -> usize {
        self.tick_count
    }

    /// The value of the `index`-th major tick, counted from the axis' minimum end.
    #[must_use]
    pub fn tick(&self, index: usize) -> f64 {
        let index = i64::try_from(index).unwrap_or(0);
        match self.logarithm_base {
            Some(base) => base.powf((self.first_tick.saturating_add(index)) as f64),
            None => self.major.scaled(self.first_tick.saturating_add(index)),
        }
    }

    /// Every major tick value, in order.
    pub fn ticks(&self) -> impl Iterator<Item = f64> + '_ {
        (0..self.tick_count).map(|index| self.tick(index))
    }

    /// How many minor ticks fall between two major ones.
    #[must_use]
    pub fn minor_divisions(&self) -> usize {
        if self.logarithm_base.is_some() {
            return 0;
        }
        let major = self.major.value();
        let minor = self.minor.value();
        if minor <= 0.0 || !minor.is_finite() || minor >= major {
            return 0;
        }
        let divisions = (major / minor).round();
        if divisions.is_finite() && divisions > 1.0 {
            (divisions as usize).min(20).saturating_sub(1)
        } else {
            0
        }
    }

    /// Where `value` sits along the axis, as a fraction from its minimum (`0.0`) to its maximum
    /// (`1.0`). Outside that range for a value the axis does not cover, which is what lets a bar
    /// taller than its axis be clipped rather than mis-drawn.
    ///
    /// `None` for a value a logarithmic axis cannot place — zero and everything below it — and for a
    /// scale whose ends coincide, which the constructors do not produce but a caller could reach by
    /// building one field at a time.
    #[must_use]
    pub fn fraction(&self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self.logarithm_base {
            Some(base) => {
                if value <= 0.0 || self.minimum <= 0.0 || self.maximum <= 0.0 {
                    return None;
                }
                let ln_base = base.ln();
                if ln_base == 0.0 {
                    return None;
                }
                let low = self.minimum.ln() / ln_base;
                let high = self.maximum.ln() / ln_base;
                let span = high - low;
                (span != 0.0).then(|| ((value.ln() / ln_base) - low) / span)
            }
            None => {
                let span = self.maximum - self.minimum;
                (span != 0.0).then(|| (value - self.minimum) / span)
            }
        }
    }

    /// The fraction the marks are measured from: zero when the axis crosses it, otherwise whichever
    /// end is nearer. A bar grows from here and an area is filled to here.
    #[must_use]
    pub fn baseline_fraction(&self) -> f64 {
        self.fraction(0.0).unwrap_or(0.0).clamp(0.0, 1.0)
    }
}

impl Step {
    /// This step, unless `exact` is a number the nice-number set cannot express, in which case the
    /// exact value is kept as a mantissa over a power of ten.
    ///
    /// A file that states `c:majorUnit val="7"` means seven, not five.
    fn exactly_or(self, exact: f64) -> Self {
        if (self.value() - exact).abs() < f64::EPSILON * exact.abs().max(1.0) {
            return self;
        }
        // Three decimal places is enough for every major unit a real file states and keeps the
        // mantissa inside an `i32`.
        for decimals in 0..=3 {
            let scaled = exact * ten_to(decimals);
            if (scaled - scaled.round()).abs() < 1e-6 && scaled.round().abs() < f64::from(i32::MAX)
            {
                return Step::new(scaled.round() as i32, -decimals);
            }
        }
        self
    }
}

/// The bounds a file's stated scaling puts on an axis.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct StatedScale {
    /// `c:scaling > c:min`.
    pub minimum: Option<f64>,
    /// `c:scaling > c:max`.
    pub maximum: Option<f64>,
    /// `c:majorUnit`.
    pub major_unit: Option<f64>,
    /// `c:minorUnit`.
    pub minor_unit: Option<f64>,
    /// `c:scaling > c:logBase`.
    pub logarithm_base: Option<f64>,
}

/// Turns whatever the data gave into a pair a scale can be built from.
///
/// Four degenerate shapes reach here from real files, and each has one defined answer:
/// no finite values at all (`0 … 1`), every value the same non-zero number (`0 … v` widened to the
/// value's own magnitude), every value zero (`0 … 1`), and bounds the wrong way round (swapped).
fn usable_bounds(low: f64, high: f64) -> (f64, f64) {
    let (low, high) = match (low.is_finite(), high.is_finite()) {
        (true, true) => (low.min(high), low.max(high)),
        _ => (0.0, 1.0),
    };
    if low < high {
        return (low, high);
    }
    // A single distinct value. Office plots it against an axis that reaches zero, which is what
    // makes a one-point bar chart draw a bar rather than a line.
    let value = low;
    if value == 0.0 {
        (0.0, 1.0)
    } else if value > 0.0 {
        (0.0, value)
    } else {
        (value, 0.0)
    }
}

/// How many whole steps it takes to cover `low ..= high` once both ends are snapped outward.
fn span_in_steps(low: f64, high: f64, step: Step) -> usize {
    let unit = step.value();
    let first = floor_multiple(low, unit);
    let last = ceil_multiple(high, unit);
    let count = last - first;
    if count.is_finite() && count > 0.0 {
        (count as usize).min(4096)
    } else {
        1
    }
}

/// `floor(value / unit)`, with a relative tolerance so that a value that *is* a whole multiple does
/// not land one step low because the division rounded down.
fn floor_multiple(value: f64, unit: f64) -> f64 {
    if !unit.is_finite() || unit <= 0.0 {
        return 0.0;
    }
    let quotient = value / unit;
    let snapped = quotient.round();
    if (quotient - snapped).abs() < 1e-9 * snapped.abs().max(1.0) {
        snapped
    } else {
        quotient.floor()
    }
}

/// `ceil(value / unit)`, with the same tolerance in the other direction.
fn ceil_multiple(value: f64, unit: f64) -> f64 {
    if !unit.is_finite() || unit <= 0.0 {
        return 0.0;
    }
    let quotient = value / unit;
    let snapped = quotient.round();
    if (quotient - snapped).abs() < 1e-9 * snapped.abs().max(1.0) {
        snapped
    } else {
        quotient.ceil()
    }
}

/// A tick index that cannot overflow the arithmetic it is about to be used in.
fn clamp_index(value: f64) -> i64 {
    if value.is_nan() {
        0
    } else {
        value.clamp(-1.0e15, 1.0e15) as i64
    }
}
