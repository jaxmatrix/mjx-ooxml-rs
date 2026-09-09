//! [`RangeStatistics`] — what a rule has to know about its **whole** range before it can answer for
//! one cell, and the cache that stops it being computed per cell.
//!
//! # Eight of the eighteen rule kinds are not local
//!
//! `cellIs greaterThan 5` is a question about one cell. `top10`, `aboveAverage`, `duplicateValues`,
//! `uniqueValues` and the three graded kinds are not: each needs the minimum, the maximum, the mean,
//! the standard deviation, a percentile or a multiset of every value in the rule's `@sqref` before
//! it can say anything about a single position. Computing that per cell would turn a band of two
//! hundred cells into two hundred scans of a column.
//!
//! So it is computed **once per rule** and cached, and the cache lives on the box model beside
//! [`FormatCache`](crate::numfmt::FormatCache) for the same reason that one does: a scroll is
//! another `layout_page`, and a statistic that had to be recomputed on every band would be paid for
//! at sixty hertz.
//!
//! # The scan is bounded by the populated cells, not by the range
//!
//! `@sqref="A:A"` names 1,048,576 positions and a sheet holds forty of them. The scan therefore
//! walks `mjx-sml`'s packed store — rows in document order, cells within a row in column order —
//! and tests each against the range list, rather than walking the coordinate range and probing.
//! That is the same discipline [`crate::geometry`] holds for the grid itself, and it is what makes a
//! whole-column colour scale cost what the data costs.
//!
//! It is bounded a second time, at [`SCAN_CEILING`], because a genuinely enormous sheet must not
//! make the first frame unbounded. [`RangeStatistics::is_complete`] says which answer you got, and
//! a caller that wants to know why a scale looks wrong on a 400,000-row sheet has somewhere to ask.

use std::collections::HashMap;

use mjx_sml::GridBounds;

use crate::condfmt::value::RuleValue;
use crate::sheet::SheetGrid;

/// How many populated cells one rule's scan will visit before it stops and says so.
///
/// GUESS: the number itself. It is far above any sheet a person looks at (a 256K-cell rule is a
/// whole-sheet colour scale over a quarter of a million populated positions) and far below the
/// point at which a frame stops arriving. Nothing in ECMA-376 bounds it, because ECMA-376 does not
/// describe a renderer.
pub const SCAN_CEILING: usize = 262_144;

/// Everything the eight non-local rule kinds need, computed once over one rule's range.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct RangeStatistics {
    /// Every numeric value in the range, ascending. Text, booleans, errors and blanks are absent.
    sorted: Vec<f64>,
    /// The sum of `sorted`, kept rather than re-derived so the mean is one division.
    sum: f64,
    /// The sum of the squares, for the standard deviation.
    sum_of_squares: f64,
    /// How many times each distinct value occurs, for `duplicateValues` and `uniqueValues`.
    counts: HashMap<String, u32>,
    /// How many populated cells the scan saw, numeric or not.
    populated: usize,
    /// Whether the scan finished, or stopped at [`SCAN_CEILING`].
    complete: bool,
}

impl RangeStatistics {
    /// Walks `grid`'s populated cells and summarises the ones inside `ranges`.
    #[must_use]
    pub fn scan(grid: &SheetGrid, ranges: &[GridBounds]) -> Self {
        let mut stats = Self {
            complete: true,
            ..Self::default()
        };
        let Some(data) = grid.worksheet().sheet_data() else {
            return stats;
        };
        let (first_row, last_row) = row_span(ranges);
        'rows: for row in data.rows() {
            let Some(number) = row.number() else { continue };
            let Some(index) = number.checked_sub(1) else {
                continue;
            };
            if index < first_row {
                continue;
            }
            if index > last_row {
                break;
            }
            for cell in row.cells() {
                let column = cell.reference().column();
                if !covers(ranges, index, column) {
                    continue;
                }
                if stats.populated >= SCAN_CEILING {
                    stats.complete = false;
                    break 'rows;
                }
                let text = grid.cell_text(&cell).unwrap_or_default();
                let value = RuleValue::read(cell.cell_type(), &text);
                stats.absorb(&value);
            }
        }
        stats.sorted.sort_by(f64::total_cmp);
        stats
    }

    /// Records one cell's value.
    fn absorb(&mut self, value: &RuleValue) {
        if value.is_blank() {
            return;
        }
        self.populated = self.populated.saturating_add(1);
        if let Some(number) = value.as_number() {
            self.sorted.push(number);
            self.sum += number;
            self.sum_of_squares += number * number;
        }
        if let Some(key) = value.duplicate_key() {
            *self.counts.entry(key).or_insert(0) += 1;
        }
    }

    /// How many numeric values the range holds.
    #[must_use]
    pub fn count(&self) -> usize {
        self.sorted.len()
    }

    /// How many populated cells it holds, numeric or not.
    #[must_use]
    pub fn populated(&self) -> usize {
        self.populated
    }

    /// Whether the scan reached the end of the range rather than [`SCAN_CEILING`].
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// The smallest number in the range, or `None` when it holds none.
    #[must_use]
    pub fn minimum(&self) -> Option<f64> {
        self.sorted.first().copied()
    }

    /// The largest number in the range, or `None` when it holds none.
    #[must_use]
    pub fn maximum(&self) -> Option<f64> {
        self.sorted.last().copied()
    }

    /// The arithmetic mean of the numbers, or `None` when there are none.
    #[must_use]
    pub fn mean(&self) -> Option<f64> {
        if self.sorted.is_empty() {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        Some(self.sum / self.sorted.len() as f64)
    }

    /// The **population** standard deviation of the numbers, or `None` when there are fewer than
    /// two.
    ///
    /// GUESS: that it is the population deviation (`STDEVP`) rather than the sample one (`STDEV`).
    /// ECMA-376 §18.3.1.10 says only that `@stdDev` is *"the number of standard deviations to
    /// include above or below the average"* and names neither; the two answers differ by
    /// `sqrt(n/(n-1))`, which is 6% at ten values and under 1% at a hundred, so a fixture of ten
    /// values is where a Windows sitting would tell them apart. One is here and the other is a
    /// one-line change.
    #[must_use]
    pub fn standard_deviation(&self) -> Option<f64> {
        let count = self.sorted.len();
        if count < 2 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        let count = count as f64;
        let mean = self.sum / count;
        let variance = (self.sum_of_squares / count) - (mean * mean);
        Some(variance.max(0.0).sqrt())
    }

    /// The value `fraction` of the way from the minimum to the maximum — what a `cfvo` of type
    /// `percent` means.
    ///
    /// `fraction` is `0.0..=1.0`; the wire spells it `0`..`100`.
    #[must_use]
    pub fn percent_of_range(&self, fraction: f64) -> Option<f64> {
        let (minimum, maximum) = (self.minimum()?, self.maximum()?);
        Some(minimum + fraction * (maximum - minimum))
    }

    /// The `fraction` percentile, interpolated the way `PERCENTILE.INC` does — what a `cfvo` of type
    /// `percentile` means.
    ///
    /// DocumentedBehaviour: the rank is `fraction * (n - 1)` and the answer is linearly interpolated
    /// between the two values it falls between, which is `PERCENTILE.INC`'s published definition and
    /// what §18.3.1.11's `percentile` refers to.
    #[must_use]
    pub fn percentile(&self, fraction: f64) -> Option<f64> {
        let count = self.sorted.len();
        if count == 0 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        let rank = fraction.clamp(0.0, 1.0) * (count.saturating_sub(1)) as f64;
        let low = rank.floor();
        let high = rank.ceil();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (low_index, high_index) = (low as usize, high as usize);
        let below = *self.sorted.get(low_index.min(count - 1))?;
        let above = *self.sorted.get(high_index.min(count - 1))?;
        Some(below + (rank - low) * (above - below))
    }

    /// The cut-off a `top10` rule compares against: the `count`-th largest value, or the `count`-th
    /// smallest when `from_bottom`.
    ///
    /// A cell fires on `>=` (or `<=`), which is why ties widen the highlight past `count` cells —
    /// DocumentedBehaviour, and the behaviour §18.3.1.10's *"top N"* wording produces in Excel.
    #[must_use]
    pub fn rank_threshold(&self, count: usize, from_bottom: bool) -> Option<f64> {
        let total = self.sorted.len();
        if total == 0 || count == 0 {
            return None;
        }
        let taken = count.min(total);
        if from_bottom {
            self.sorted.get(taken - 1).copied()
        } else {
            self.sorted.get(total - taken).copied()
        }
    }

    /// How many cells a `top10` whose `@percent` is set actually takes.
    ///
    /// GUESS: `floor(n * percent / 100)`, with a floor of one. Excel highlights one cell for *Top
    /// 10%* of a nine-value range, which rules out rounding up, and nothing states what it does at
    /// zero.
    #[must_use]
    pub fn rank_count_for_percent(&self, percent: u32) -> usize {
        #[allow(clippy::cast_precision_loss)]
        let total = self.sorted.len() as f64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let taken = (total * f64::from(percent) / 100.0).floor() as usize;
        taken.max(1)
    }

    /// How many cells in the range hold this value.
    #[must_use]
    pub fn occurrences(&self, value: &RuleValue) -> u32 {
        value
            .duplicate_key()
            .and_then(|key| self.counts.get(&key).copied())
            .unwrap_or(0)
    }

    /// Where `value` sits between the minimum and the maximum, clamped to `0.0..=1.0`.
    ///
    /// `None` when the range holds no numbers; `0.0` when every number in it is the same, which is
    /// the degenerate case a data bar and a colour scale both have to answer for.
    #[must_use]
    pub fn position_of(&self, value: f64) -> Option<f64> {
        let (minimum, maximum) = (self.minimum()?, self.maximum()?);
        Some(interpolate(value, minimum, maximum))
    }
}

/// Where `value` sits between `low` and `high`, clamped, with a zero-width span answering `0.0`.
#[must_use]
pub fn interpolate(value: f64, low: f64, high: f64) -> f64 {
    let span = high - low;
    if !span.is_finite() || span.abs() <= f64::EPSILON {
        return 0.0;
    }
    ((value - low) / span).clamp(0.0, 1.0)
}

/// Whether any range in `ranges` covers `(row, column)`.
#[must_use]
pub fn covers(ranges: &[GridBounds], row: u32, column: u16) -> bool {
    ranges.iter().any(|bounds| {
        row >= bounds.first_row()
            && row <= bounds.last_row()
            && column >= bounds.first_column()
            && column <= bounds.last_column()
    })
}

/// The smallest and largest row any of `ranges` reaches, so the scan can stop early.
fn row_span(ranges: &[GridBounds]) -> (u32, u32) {
    let first = ranges
        .iter()
        .map(|bounds| bounds.first_row())
        .min()
        .unwrap_or(u32::MAX);
    let last = ranges
        .iter()
        .map(|bounds| bounds.last_row())
        .max()
        .unwrap_or(0);
    (first, last)
}

/// The statistics already computed for this sheet's rules, keyed by the rule's position.
///
/// A `Vec` rather than a map: a sheet has a handful of rules, and the position is dense because
/// [`ConditionalIndex`](crate::condfmt::ConditionalIndex) numbers them itself.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct StatisticsCache {
    /// Which sheet the entries belong to. A different tab invalidates the whole cache, because a
    /// rule's position means nothing on another sheet.
    sheet: Option<usize>,
    entries: Vec<Option<RangeStatistics>>,
    /// The answer for a position the cache could not reach, which nothing can produce today.
    ///
    /// It exists so [`StatisticsCache::statistics`] can hand back a reference on **every** path
    /// without an `unwrap`, an `expect` or an `unreachable!`. `CLAUDE.md` forbids all three on a
    /// library path, and a structurally-impossible branch is exactly where one gets written.
    empty: RangeStatistics,
}

impl StatisticsCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drops everything. Called when the content changes, because every statistic here is a fact
    /// about cell *values*.
    pub fn clear(&mut self) {
        self.sheet = None;
        self.entries.clear();
    }

    /// How many rules have had their statistics computed.
    #[must_use]
    pub fn computed(&self) -> usize {
        self.entries.iter().filter(|entry| entry.is_some()).count()
    }

    /// The statistics for rule `position` of `grid`'s sheet, computing them on first ask.
    pub fn statistics(
        &mut self,
        grid: &SheetGrid,
        position: usize,
        ranges: &[GridBounds],
    ) -> &RangeStatistics {
        if self.sheet != Some(grid.index()) {
            self.sheet = Some(grid.index());
            self.entries.clear();
        }
        if self.entries.len() <= position {
            self.entries.resize(position.saturating_add(1), None);
        }
        if let Some(slot) = self.entries.get_mut(position) {
            if slot.is_none() {
                *slot = Some(RangeStatistics::scan(grid, ranges));
            }
        }
        match self.entries.get(position).and_then(Option::as_ref) {
            Some(statistics) => statistics,
            None => &self.empty,
        }
    }
}
