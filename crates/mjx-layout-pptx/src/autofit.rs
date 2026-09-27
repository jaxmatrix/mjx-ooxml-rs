//! Autofit — what happens when text does not fit its shape.
//!
//! # ⚠ Two halves, and only one of them is a fact
//!
//! `a:normAutofit@fontScale` and `@lnSpcReduction` are **PowerPoint's own computed values**, written
//! into the file the last time PowerPoint laid the shape out. There are therefore two entirely
//! different things a renderer can do with autofit, and conflating them is how a renderer ends up
//! claiming a fidelity it does not have:
//!
//! 1. **Honour what is stored.** Multiply every font size by the stored `fontScale` and reduce every
//!    line's spacing by the stored `lnSpcReduction`. This reproduces what the author saw, it is
//!    exact, and it needs no search. [`AutofitPolicy::HonourOnly`] does this and nothing else.
//!
//! 2. **Compute a scale.** Search for a scale at which the text fits. This is *running PowerPoint's
//!    own search*, and **what PowerPoint's search actually is has never been specified**. The ladder
//!    below is derived from the values PowerPoint is observed to write — it never writes a
//!    `fontScale` of, say, `81000` — and the order in which the two factors are stepped is a guess.
//!    [`AutofitPolicy::HonourAndRecompute`] does this, and it is the default, because text that
//!    overflows its box is the most obvious rendering defect there is.
//!
//! **This is a question for the Windows sitting** (`docs/validation/07-the-reference-pack.md`), not
//! something LibreOffice can settle: the whole point of the stored values is that they came from
//! PowerPoint. Until then, nothing in this crate describes a recomputed scale as parity — the
//! computed value is reported as a value ([`AutofitOutcome::recomputed`]) so a caller can tell the
//! two halves apart.
//!
//! # `a:spAutoFit` is the other direction
//!
//! `normAutofit` shrinks the text to fit the shape; `spAutoFit` grows the *shape* to fit the text.
//! PowerPoint recomputes the shape's `a:ext@cy` and **writes it into the file**, so a deck opened
//! from disk already has bounds that fit — honouring the stored geometry is exact, and recomputing
//! only matters once this stack has edited the text. [`AutofitOutcome::grown_height`] carries the
//! height the text needs, and the caller decides whether the shape may grow.

use mjx_dml::{Fraction, TextAutofit};

/// What a box model does about autofit.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum AutofitPolicy {
    /// Honour a stored `normAutofit` scale, and search for one when the text still overflows.
    ///
    /// The default, and the only one that keeps overflowing text inside its box.
    #[default]
    HonourAndRecompute,
    /// Honour a stored scale and never search for one.
    ///
    /// The honest setting for anything that must not be a guess about PowerPoint — a fidelity
    /// baseline, an export whose numbers are being compared against Office.
    HonourOnly,
    /// Ignore autofit entirely: neither honour a stored scale nor search for one.
    ///
    /// Here so that a gate can *prove the search is what makes the difference*. A shape whose text
    /// happens to fit lays out identically under a correct autofit implementation and under none at
    /// all, so the only way to show the search runs is to switch it off and watch the layout change.
    Disabled,
}

/// What autofit did to one text body.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct AutofitOutcome {
    /// The factor every font size was multiplied by. `1.0` when nothing scaled it.
    pub font_scale: f64,
    /// The proportion every line's spacing was reduced by. `0.0` when nothing reduced it.
    pub line_space_reduction: f64,
    /// Whether the two above were **computed here** rather than read from the file.
    ///
    /// The flag that keeps the two halves apart: a `false` here means the numbers are PowerPoint's
    /// own, and a `true` means they are this crate's search's, which no one has checked against
    /// PowerPoint yet.
    pub recomputed: bool,
    /// The height the text needs, for a body whose `a:spAutoFit` says the shape grows to it.
    pub grown_height: Option<mjx_ooxml_core::measure::Emu>,
}

impl AutofitOutcome {
    /// Nothing scaled and nothing reduced.
    #[must_use]
    pub fn unscaled() -> Self {
        Self {
            font_scale: 1.0,
            line_space_reduction: 0.0,
            recomputed: false,
            grown_height: None,
        }
    }

    /// Whether any font size was changed.
    #[must_use]
    pub fn scales_text(&self) -> bool {
        (self.font_scale - 1.0).abs() > f64::EPSILON || self.line_space_reduction > f64::EPSILON
    }
}

/// The font scales PowerPoint is observed to write, largest first.
///
/// **This ladder is evidence, not specification.** It is the set of `a:normAutofit@fontScale` values
/// that appear in files PowerPoint authored; PowerPoint never writes an arbitrary percentage, which
/// is why a search over a continuous range would produce values no PowerPoint file contains and
/// would differ from Office on every shape it touched. Whether the ladder is *complete* is one of
/// the things the Windows sitting settles.
pub const FONT_SCALES: &[f64] = &[
    1.0, 0.925, 0.85, 0.775, 0.70, 0.65, 0.60, 0.55, 0.50, 0.45, 0.40, 0.35, 0.30, 0.25,
];

/// The line-space reductions PowerPoint is observed to write, smallest first.
///
/// **The order the two ladders are combined in is the guess.** PowerPoint plainly reduces the font
/// before it reduces the line spacing at first and then combines the two, but the exact interleaving
/// is not written down anywhere. The search takes the whole font ladder at no reduction, then the
/// whole of it at 10 %, then at 20 % — which reaches every pair PowerPoint can write and prefers a
/// larger font over tighter lines. A shape that needs a different pair than Office chose will differ
/// from Office, and these two tables are what the sitting changes.
pub const LINE_SPACE_REDUCTIONS: &[f64] = &[0.0, 0.10, 0.20];

/// The first rung of `from..=last` at which `fits` is true, found by bisection — or `last` when none
/// is.
///
/// # Why a bisection is safe here, and why it matters
///
/// Every probe of `fits` is a **whole composition of the text body**: every paragraph itemised,
/// shaped and broken. A linear walk of the fourteen scales at three reductions costs up to
/// forty-two of those for one shape, and a viewport re-lays a page on every zoom step — so the
/// difference between a walk and a bisection is the difference between an autofitting title being
/// free and being the dominant cost of scrolling.
///
/// The bisection is correct because **the fit is monotone in the rung**: a smaller font puts more
/// characters on each line, so a body needs no more lines and no taller a line than it did at a
/// larger one. A body that fits at rung *n* therefore fits at every rung after it, which is exactly
/// the predicate a binary search needs.
///
/// It is written as a free function over indices, rather than inline in the layout, so that it can
/// be checked against a linear scan over **every** monotone predicate the ladder admits — which is
/// the only way to know a bisection agrees with the walk it replaced.
///
/// # Errors
/// Whatever `fits` fails with — composing a body can fail when a face will not shape.
pub fn largest_fitting_rung<E>(
    from: usize,
    last: usize,
    fits: &mut impl FnMut(usize) -> Result<bool, E>,
) -> Result<usize, E> {
    let mut low = from.min(last);
    let mut high = last;
    while low < high {
        let middle = low + (high - low) / 2;
        if fits(middle)? {
            high = middle;
        } else {
            low = middle.saturating_add(1);
        }
    }
    Ok(low.min(last))
}

/// The rung a stored font scale starts the search at: the first one no larger than it.
///
/// A stored scale is PowerPoint's own last answer, and the search only runs when the text overflowed
/// *at* it — so every rung above it overflows too, and starting there is a fact rather than an
/// optimisation. A stored scale below the whole ladder starts at its foot.
#[must_use]
pub fn rung_of(font_scale: f64) -> usize {
    let last = FONT_SCALES.len().saturating_sub(1);
    FONT_SCALES
        .iter()
        .position(|&scale| scale <= font_scale)
        .unwrap_or(last)
}

/// The stored scale a body's autofit states, or `None` when it states no scaling autofit.
///
/// A `a:normAutofit` with neither attribute is a body PowerPoint has not yet had to scale, which is
/// `1.0` and `0.0` rather than "no autofit": the body *is* autofitting, it simply fits at full size.
#[must_use]
pub fn stored_scale(autofit: Option<TextAutofit>) -> Option<AutofitOutcome> {
    match autofit {
        Some(TextAutofit::Normal {
            font_scale,
            line_space_reduction,
        }) => Some(AutofitOutcome {
            font_scale: ratio(font_scale, 1.0),
            line_space_reduction: ratio(line_space_reduction, 0.0),
            recomputed: false,
            grown_height: None,
        }),
        _ => None,
    }
}

/// Whether a body's autofit grows the shape rather than shrinking the text.
#[must_use]
pub fn grows_the_shape(autofit: Option<TextAutofit>) -> bool {
    matches!(autofit, Some(TextAutofit::Shape))
}

/// Whether a body's autofit shrinks the text.
#[must_use]
pub fn shrinks_the_text(autofit: Option<TextAutofit>) -> bool {
    matches!(autofit, Some(TextAutofit::Normal { .. }))
}

/// A fraction as a ratio, clamped to the range a scale can sensibly take.
///
/// A file may state `fontScale="-50000"` or `"9999999"`, and a negative or unbounded scale would
/// produce a font size of zero or of four million points. The clamp is at the point of use rather
/// than in the model, because the model's job is to report what the file says.
fn ratio(value: Option<Fraction>, absent: f64) -> f64 {
    match value {
        None => absent,
        Some(fraction) => {
            let ratio = fraction.ratio();
            if !ratio.is_finite() {
                return absent;
            }
            ratio.clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladders_run_from_full_size_down_and_from_no_reduction_up() {
        // The direction is what the search depends on: it walks the reductions forward and bisects
        // the scales, so a table written the other way round would find the *smallest* font that
        // fits rather than the largest.
        assert_eq!(FONT_SCALES.first().copied(), Some(1.0));
        assert_eq!(FONT_SCALES.last().copied(), Some(0.25));
        for pair in FONT_SCALES.windows(2) {
            assert!(pair[1] < pair[0], "the scales descend: {pair:?}");
        }
        assert_eq!(LINE_SPACE_REDUCTIONS.first().copied(), Some(0.0));
        assert_eq!(LINE_SPACE_REDUCTIONS.last().copied(), Some(0.20));
        for pair in LINE_SPACE_REDUCTIONS.windows(2) {
            assert!(pair[1] > pair[0], "the reductions ascend: {pair:?}");
        }
    }

    #[test]
    fn the_bisection_agrees_with_the_linear_walk_it_replaced_on_every_monotone_ladder() {
        // The only honest test of a search that replaced a walk: not "it finds a rung" but "it finds
        // the *same* rung", over **every** predicate the ladder admits. A monotone predicate on
        // `n` rungs is exactly "false below some threshold and true from it on", so there are `n + 1`
        // of them and they can all be enumerated rather than sampled.
        let last = FONT_SCALES.len() - 1;
        for threshold in 0..=FONT_SCALES.len() {
            for from in 0..=last {
                let mut probes = 0_usize;
                let mut fits = |rung: usize| -> Result<bool, ()> {
                    probes += 1;
                    Ok(rung >= threshold)
                };
                let found = largest_fitting_rung(from, last, &mut fits).expect("infallible");

                // What the walk this replaced would have answered: the first rung at or after
                // `from` that fits, and the foot of the ladder when none does.
                let walked = (from..=last)
                    .find(|&rung| rung >= threshold)
                    .unwrap_or(last);
                assert_eq!(
                    found, walked,
                    "threshold {threshold}, starting at {from}: bisection said {found} and the \
                     walk said {walked}"
                );
                assert!(
                    probes <= 5,
                    "a fourteen-rung ladder needs four probes, not {probes}"
                );
            }
        }
    }

    #[test]
    fn a_stored_scale_starts_the_search_at_its_own_rung() {
        assert_eq!(rung_of(1.0), 0);
        assert_eq!(FONT_SCALES[rung_of(0.85)], 0.85);
        // A stored scale between two rungs starts at the first one no larger than it, because every
        // rung above it is a size the text has already overflowed at.
        assert_eq!(FONT_SCALES[rung_of(0.80)], 0.775);
        // And one below the whole ladder starts at its foot rather than at its head.
        assert_eq!(rung_of(0.01), FONT_SCALES.len() - 1);
    }

    #[test]
    fn every_rung_is_a_value_powerpoint_writes_as_a_whole_number_of_thousandths() {
        // `a:normAutofit@fontScale` is on the wire as 1000ths of a percent, so a rung that is not a
        // whole number of them could not be written back — which is how a value that is *nearly* one
        // of PowerPoint's would slip in.
        for &scale in FONT_SCALES.iter().chain(LINE_SPACE_REDUCTIONS) {
            let thousandths = scale * 100_000.0;
            assert!(
                (thousandths - thousandths.round()).abs() < 1e-6,
                "{scale} is {thousandths} thousandths of a percent, which is not a whole number"
            );
            assert!((0.0..=1.0).contains(&scale), "{scale} is not a proportion");
        }
    }

    #[test]
    fn a_stored_scale_is_read_and_never_recomputed() {
        let stored = stored_scale(Some(TextAutofit::Normal {
            font_scale: Some(Fraction::from_ratio(0.625)),
            line_space_reduction: Some(Fraction::from_ratio(0.1)),
        }))
        .expect("a normAutofit states one");
        assert!((stored.font_scale - 0.625).abs() < 1e-12);
        assert!((stored.line_space_reduction - 0.1).abs() < 1e-12);
        assert!(!stored.recomputed, "a stored value is not a computed one");
    }

    #[test]
    fn a_normautofit_with_no_attributes_fits_at_full_size() {
        let stored = stored_scale(Some(TextAutofit::Normal {
            font_scale: None,
            line_space_reduction: None,
        }))
        .expect("it is still a scaling autofit");
        assert_eq!(stored.font_scale, 1.0);
        assert_eq!(stored.line_space_reduction, 0.0);
    }

    #[test]
    fn the_other_two_choices_state_no_scale() {
        assert_eq!(stored_scale(Some(TextAutofit::None)), None);
        assert_eq!(stored_scale(Some(TextAutofit::Shape)), None);
        assert_eq!(stored_scale(None), None);
        assert!(grows_the_shape(Some(TextAutofit::Shape)));
        assert!(!grows_the_shape(Some(TextAutofit::None)));
        assert!(shrinks_the_text(Some(TextAutofit::Normal {
            font_scale: None,
            line_space_reduction: None
        })));
    }

    #[test]
    fn an_impossible_stored_scale_is_clamped_rather_than_applied() {
        let stored = stored_scale(Some(TextAutofit::Normal {
            font_scale: Some(Fraction::from_ratio(-4.0)),
            line_space_reduction: Some(Fraction::from_ratio(12.0)),
        }))
        .expect("a normAutofit states one");
        assert_eq!(stored.font_scale, 0.0);
        assert_eq!(stored.line_space_reduction, 1.0);
    }
}
