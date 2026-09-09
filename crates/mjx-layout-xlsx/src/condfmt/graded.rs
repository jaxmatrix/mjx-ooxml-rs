//! The three rule kinds that **interpolate**: colour scales, data bars and icon sets.
//!
//! # None of the three has a condition
//!
//! A `cellIs` rule asks a question and a cell either satisfies it or does not. A colour scale asks
//! nothing: every populated cell in its range takes a colour, and *which* colour is a position
//! between two thresholds. That is why these three never reach [`crate::condfmt::predicate`] and why
//! `@stopIfTrue` on one of them stops everything below it unconditionally.
//!
//! # A threshold is not a number until the range has been scanned
//!
//! `<cfvo type="min"/>` states no value at all, `type="percentile" val="50"` states a rank rather
//! than a value, and `type="percent" val="50"` states a *third* thing — halfway between the
//! smallest and largest, which is not the median. [`resolve`] turns each into a number using
//! [`RangeStatistics`], and answers `None` for the one kind it cannot: `type="formula"`, whose
//! `@val` is an expression and which is reported unevaluated rather than guessed at.
//!
//! # ⚠ Nothing here resolves a colour, and that is the point
//!
//! A colour scale's stops are `CT_Color`: an `@rgb`, or an `@indexed` row, or a `@theme` *position*
//! with a `@tint`. Blending two of them needs `xl/theme/theme1.xml` and the workbook's
//! `indexedColors`, neither of which a box model holds — resolving them is `mjx-scene-xlsx`'s job,
//! for exactly the reason `mjx_sml::Color`'s theme index stays a theme index everywhere else in this
//! crate.
//!
//! So the answer travels as [`ScaleBlend`]: **the two stops the value fell between, and how far
//! along it is.** The interpolation itself happens once, above, where the endpoints are real
//! colours. A box model that mixed two hex triplets would have been resolving a theme it cannot see
//! — and would have been silently wrong for every workbook whose scale is themed, which is most of
//! them.
//!
//! # ⚠ What the base schema does not carry
//!
//! `CT_DataBar` has three attributes: `@minLength`, `@maxLength` and `@showValue`. The **axis
//! position, the negative-value fill, the bar border and the gradient** are all `x14:dataBar` — the
//! `extLst` extension `mjx-sml` preserves verbatim and deliberately does not model. So a bar here is
//! a solid rectangle growing rightward from the left edge of the cell, a negative value clamps to
//! `@minLength`, and there is no axis. That is Excel 2007's data bar, faithfully; it is **not**
//! Excel 2010's, which is the one most people have seen. Reported rather than approximated.

use mjx_ooxml_types::spreadsheetml::{ConditionalFormatValueObjectType, IconSetType};
use mjx_sml::Color;

use crate::condfmt::rules::{Graded, Threshold};
use crate::condfmt::stats::{interpolate, RangeStatistics};

/// A colour a scale interpolated, **unresolved**: the two stops and the position between them.
///
/// See this module's own documentation for why the blend is not done here.
#[derive(Clone, PartialEq, Debug)]
pub struct ScaleBlend {
    /// The stop below the value.
    pub low: Color,
    /// The stop above it.
    pub high: Color,
    /// How far from `low` to `high` the value sits, `0.0..=1.0`.
    pub fraction: f64,
}

/// A data bar, as a fraction of the cell rather than as a rectangle.
///
/// The rectangle is the box model's, because only it knows how wide the cell is. This is the part
/// that is a property of the *rule and the value*, and it is the number a gate asserts.
#[derive(Clone, PartialEq, Debug)]
pub struct DataBarGeometry {
    /// How much of the cell's width the bar occupies, `0.0..=1.0`.
    ///
    /// Already includes `@minLength` and `@maxLength`: a value at the shortest threshold gives
    /// `minimum_length / 100`, not zero. §18.3.1.28 calls both *"a percentage of the cell width"*.
    pub fraction: f64,
    /// Where the value sat between the two thresholds, before the lengths were applied. Carried so
    /// a report can say *why* a bar is the width it is.
    pub position: f64,
    /// The bar's colour, unresolved.
    pub colour: Option<Color>,
    /// `@showValue` — whether the cell's own text is drawn as well.
    pub shows_value: bool,
}

/// Which icon of which set a cell takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IconChoice {
    /// Which set.
    pub set: IconSetType,
    /// Which icon of it, zero-based, **after** `@reverse` has been applied.
    pub index: u8,
    /// How many icons the set has, as the file's own threshold count states it.
    pub count: u8,
    /// `@showValue`.
    pub shows_value: bool,
}

/// What a graded rule made of one cell.
#[derive(Clone, PartialEq, Debug)]
pub enum GradedResult {
    /// A colour scale placed it between two stops.
    Scale(ScaleBlend),
    /// A data bar sized it.
    Bar(DataBarGeometry),
    /// An icon set chose an icon for it.
    Icon(IconChoice),
}

/// Turns one `cfvo` into a number, or `None` when it cannot be turned into one.
///
/// `formula` is the only kind that answers `None` on a well-formed file: its `@val` is an expression
/// and there is no calculation engine. A `formula` whose `@val` happens to be a bare number is read
/// as that number, because refusing it would be pedantry rather than honesty.
#[must_use]
pub fn resolve(threshold: &Threshold, statistics: &RangeStatistics) -> Option<f64> {
    match threshold.kind {
        ConditionalFormatValueObjectType::Minimum => statistics.minimum(),
        ConditionalFormatValueObjectType::Maximum => statistics.maximum(),
        ConditionalFormatValueObjectType::Number => threshold.number(),
        ConditionalFormatValueObjectType::Percent => {
            statistics.percent_of_range(threshold.number()? / 100.0)
        }
        ConditionalFormatValueObjectType::Percentile => {
            statistics.percentile(threshold.number()? / 100.0)
        }
        ConditionalFormatValueObjectType::Formula => threshold.number(),
    }
}

/// Whether every threshold of `graded` could be turned into a number.
///
/// A graded rule whose thresholds cannot all be resolved is reported **unevaluated** rather than
/// drawn at a guessed position: a data bar at the wrong length is a claim about the data, and a
/// colour scale whose midpoint is a formula is not a colour scale this build can draw.
#[must_use]
pub fn is_resolvable(graded: &Graded, statistics: &RangeStatistics) -> bool {
    match graded {
        Graded::ColorScale { stops } => {
            stops.len() >= 2
                && stops
                    .iter()
                    .all(|(threshold, _)| resolve(threshold, statistics).is_some())
        }
        Graded::DataBar {
            shortest, longest, ..
        } => resolve(shortest, statistics).is_some() && resolve(longest, statistics).is_some(),
        Graded::IconSet { thresholds, .. } => {
            thresholds.len() >= 2
                && thresholds
                    .iter()
                    .all(|threshold| resolve(threshold, statistics).is_some())
        }
    }
}

/// Applies a graded rule to one value.
///
/// `None` when the rule cannot be applied to *this* value — a text cell under a colour scale, or a
/// rule whose thresholds do not resolve.
#[must_use]
pub fn apply(graded: &Graded, value: f64, statistics: &RangeStatistics) -> Option<GradedResult> {
    match graded {
        Graded::ColorScale { stops } => {
            colour_scale(stops, value, statistics).map(GradedResult::Scale)
        }
        Graded::DataBar {
            shortest,
            longest,
            colour,
            minimum_length,
            maximum_length,
            shows_value,
        } => {
            let low = resolve(shortest, statistics)?;
            let high = resolve(longest, statistics)?;
            let position = interpolate(value, low, high);
            // GUESS: that `@minLength` and `@maxLength` are clamped to `0..=100` and read in the
            // order the file wrote them, so a file stating `minLength="90" maxLength="10"` draws a
            // bar that *shrinks* as the value grows. §18.3.1.28 gives both as percentages and
            // states no ordering constraint; refusing to invert them would be repairing the file.
            let shortest_fraction = f64::from(*minimum_length).clamp(0.0, 100.0) / 100.0;
            let longest_fraction = f64::from(*maximum_length).clamp(0.0, 100.0) / 100.0;
            Some(GradedResult::Bar(DataBarGeometry {
                fraction: shortest_fraction + position * (longest_fraction - shortest_fraction),
                position,
                colour: colour.clone(),
                shows_value: *shows_value,
            }))
        }
        Graded::IconSet {
            icons,
            thresholds,
            reversed,
            shows_value,
            thresholds_are_percentages: _,
        } => icon(
            *icons,
            thresholds,
            *reversed,
            *shows_value,
            value,
            statistics,
        )
        .map(GradedResult::Icon),
    }
}

/// Places a value between two of a scale's stops.
fn colour_scale(
    stops: &[(Threshold, Color)],
    value: f64,
    statistics: &RangeStatistics,
) -> Option<ScaleBlend> {
    if stops.len() < 2 {
        return None;
    }
    let mut points: Vec<(f64, &Color)> = Vec::with_capacity(stops.len());
    for (threshold, colour) in stops {
        points.push((resolve(threshold, statistics)?, colour));
    }
    // §18.3.1.11 pairs `cfvo` with `color` **by position** and says nothing about the numbers being
    // ascending. Sorting them is what makes a three-stop scale whose midpoint resolves below its
    // minimum — which a `percentile` on a skewed range genuinely produces — draw a ramp rather than
    // an inversion. GUESS: that Excel does the same.
    points.sort_by(|left, right| left.0.total_cmp(&right.0));
    let (first_value, first_colour) = *points.first()?;
    if value <= first_value {
        return Some(ScaleBlend {
            low: first_colour.clone(),
            high: first_colour.clone(),
            fraction: 0.0,
        });
    }
    for window in points.windows(2) {
        let [(low_value, low_colour), (high_value, high_colour)] = window else {
            continue;
        };
        if value <= *high_value {
            return Some(ScaleBlend {
                low: (*low_colour).clone(),
                high: (*high_colour).clone(),
                fraction: interpolate(value, *low_value, *high_value),
            });
        }
    }
    let (_, last_colour) = *points.last()?;
    Some(ScaleBlend {
        low: last_colour.clone(),
        high: last_colour.clone(),
        fraction: 1.0,
    })
}

/// Chooses an icon.
///
/// The first `cfvo` is the floor of the whole set and is never a boundary: a three-icon set writes
/// three `cfvo` and has **two** boundaries. So the icon is the number of *later* thresholds the
/// value meets, which is `0` for a value below all of them.
fn icon(
    set: IconSetType,
    thresholds: &[Threshold],
    reversed: bool,
    shows_value: bool,
    value: f64,
    statistics: &RangeStatistics,
) -> Option<IconChoice> {
    if thresholds.len() < 2 {
        return None;
    }
    let count = u8::try_from(thresholds.len()).unwrap_or(u8::MAX);
    let mut index = 0_u8;
    for (position, threshold) in thresholds.iter().enumerate().skip(1) {
        let boundary = resolve(threshold, statistics)?;
        // `@gte` defaults to `true`: the band includes its own lower boundary.
        let meets = if threshold.inclusive {
            value >= boundary
        } else {
            value > boundary
        };
        if meets {
            index = u8::try_from(position).unwrap_or(u8::MAX);
        }
    }
    if reversed {
        index = count.saturating_sub(1).saturating_sub(index);
    }
    Some(IconChoice {
        set,
        index,
        count,
        shows_value,
    })
}
