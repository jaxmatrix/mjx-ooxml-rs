//! What a tick label and a data label say.
//!
//! # The number format is not applied, and that is a layering finding rather than an omission
//!
//! `c:numFmt@formatCode` on an axis or a data label is a **SpreadsheetML number format** — the same
//! `0.00%;[Red]-0.00%` language a cell carries — and this workspace has exactly one evaluator for
//! it, in `mjx-layout-xlsx`'s `numfmt/` module. That crate is rank 3.6 and this one is 3.55, so the
//! edge would be upward and the layering gate refuses it by name.
//!
//! Writing a second evaluator here is the wrong answer for the reason this workspace has deleted
//! fourteen duplications for. The right one is to move the format language **down**, the way
//! MJXOFF-166 moved the contrast rule down into `mjx-tokens` and MJXOFF-165 moved the authority
//! vocabulary down into the oracle, so that a cell and a chart label format a number through one
//! implementation. That is a ticket of its own; until it exists, this module formats a value by its
//! **axis step**, which gets `0.002` and `1 400` right and gets `[Red]$#,##0.00` wrong by ignoring
//! it, and the stated format is carried on the model so nothing is lost.

use crate::model::{PlotModel, SeriesModel};
use crate::scale::{Scale, Step};

/// Formats an axis value for a label.
///
/// The number of decimal places comes from the **step**, not from the value: an axis stepping by
/// `0.002` labels its second tick `0.004` and not `0.004000000000000001`, and one stepping by 500
/// labels its third `1,500` and not `1500.0`. `percent` writes it as a percentage, which is what a
/// hundred-percent-stacked plot's axis is.
#[must_use]
pub fn format_value(value: f64, step: Step, percent: bool) -> String {
    if !value.is_finite() {
        return String::new();
    }
    if percent {
        // A proportional axis steps by 0.2 and reads 0%, 20%, …; the step's own decimals are two
        // more than the percentage's.
        let scaled = value * 100.0;
        let decimals = step.decimals().saturating_sub(2);
        return format!("{scaled:.decimals$}%");
    }
    let decimals = step.decimals();
    let text = format!("{value:.decimals$}");
    group_thousands(&text)
}

/// Inserts a thin separator every three digits left of the decimal point.
///
/// Office groups an axis' thousands whenever the workbook's own format does, which is nearly always;
/// a five-digit axis label with no grouping is the single clearest sign of a chart drawn by something
/// that is not Office. `GUESS:` the separator is a comma, because this engine has no locale — a
/// locale-aware separator needs the number-format language this module's docs describe.
fn group_thousands(text: &str) -> String {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text),
    };
    let (whole, fraction) = match rest.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (rest, None),
    };
    if whole.len() <= 4 {
        return text.to_owned();
    }
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3 + 2);
    for (position, digit) in whole.chars().enumerate() {
        if position > 0 && (whole.len() - position) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    match fraction {
        Some(fraction) => format!("{sign}{grouped}.{fraction}"),
        None => format!("{sign}{grouped}"),
    }
}

/// The text of one point's data label, or `None` when the label tiers say to draw none.
///
/// The parts are assembled in the order Office writes them — series name, category name, value,
/// percentage — joined by `c:separator`, which defaults to a comma and a space.
#[must_use]
pub fn point_label(
    series: &SeriesModel,
    plot: &PlotModel,
    point: usize,
    categories: &[String],
    scale: Option<&Scale>,
) -> Option<String> {
    let index = u32::try_from(point).ok();
    let settings = index
        .and_then(|index| {
            series
                .point_labels
                .iter()
                .find(|(at, _)| *at == index)
                .map(|(_, settings)| settings.clone())
        })
        .unwrap_or_else(|| series.labels.clone());
    if settings.suppressed == Some(true) {
        return None;
    }
    let separator = settings
        .separator
        .clone()
        .unwrap_or_else(|| ", ".to_owned());
    let mut parts: Vec<String> = Vec::new();
    if settings.shows_series_name == Some(true) {
        if let Some(name) = &series.name {
            parts.push(name.clone());
        }
    }
    if settings.shows_category_name == Some(true) {
        if let Some(label) = categories.get(point) {
            parts.push(label.clone());
        }
    }
    if settings.shows_value == Some(true) {
        if let Some(value) = series.value(point) {
            let step = scale.map(|scale| scale.major).unwrap_or(Step::new(1, 0));
            parts.push(format_value(value, step, false));
        }
    }
    if settings.shows_percentage == Some(true) {
        if let Some(value) = series.value(point) {
            let total: f64 = plot
                .series
                .iter()
                .filter_map(|other| other.value(point))
                .map(f64::abs)
                .sum();
            // A pie's percentage is of its own series' total, not of the category's — the two are
            // the same only when there is one series, which is the case a pie always is.
            let total = if plot.series.len() == 1 {
                (0..series.point_count())
                    .filter_map(|at| series.value(at))
                    .map(f64::abs)
                    .sum()
            } else {
                total
            };
            if total > 0.0 {
                parts.push(format!("{:.0}%", value / total * 100.0));
            }
        }
    }
    if settings.shows_bubble_size == Some(true) {
        if let Some(size) = series.bubble_sizes.get(point).copied().flatten() {
            parts.push(format_value(size, Step::new(1, 0), false));
        }
    }
    (!parts.is_empty()).then(|| parts.join(&separator))
}
