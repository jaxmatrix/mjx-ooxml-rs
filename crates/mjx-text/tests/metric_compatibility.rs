//! Metric compatibility, measured against metrics published outside this repository.
//!
//! # The trap this file is written against
//!
//! **A face always agrees with itself.** A test that loads Carlito and asserts its advances match a
//! table this crate derived from Carlito is green whatever either of them says, and would stay green
//! if the substitution table shipped nonsense. So not one expected value below is read out of a
//! font: every number is a literal, transcribed from a published source, and the source is named
//! next to it.
//!
//! The sources are the ones `mjx_text::reference` documents in full:
//!
//! * the **(URW)++ base-35 AFM** files, which carry the published Helvetica, Times-Roman and
//!   Courier widths that Arial, Times New Roman and Courier New were drawn to;
//! * **`@capsizecss/metrics` 4.2.0**, whose vertical metrics for Arial, Times New Roman and Courier
//!   New are measured from Microsoft's own faces rather than from a clone;
//! * **ECMA-376 Part 1 §18.3.1.13**, whose Maximum Digit Width pins Calibri's digit advance at
//!   1038 units on a 2048-unit em.
//!
//! # And the second trap
//!
//! A comparison that can only come back green proves nothing either. So
//! [`a_face_that_is_not_metric_compatible_is_reported_as_divergent`] runs the *same* machinery over
//! a pair that is deliberately wrong — Carlito measured against Arial — and requires it to fail.

mod support;

use mjx_text::{
    reference_for_family, verify_metric_compatibility, MetricCompatibility, ReferenceAuthority,
    UnverifiedReason, ADVANCE_TOLERANCE_PER_MILLE,
};
use support::bundled_face;

/// One substitution, and the file that carries the substitute.
struct Pair {
    original: &'static str,
    substitute_file: &'static str,
    /// How many characters the published table for `original` covers. Written out so that a
    /// reference table quietly shrinking to one entry cannot leave this suite green.
    expected_characters_compared: u32,
}

/// The five pairs `docs/UI_PLATFORM_PLAN.md` §10 names.
const PAIRS: &[Pair] = &[
    Pair {
        original: "Calibri",
        substitute_file: "Carlito-Regular.ttf",
        expected_characters_compared: 10,
    },
    Pair {
        original: "Arial",
        substitute_file: "LiberationSans-Regular.ttf",
        expected_characters_compared: 92,
    },
    Pair {
        original: "Times New Roman",
        substitute_file: "LiberationSerif-Regular.ttf",
        expected_characters_compared: 92,
    },
    Pair {
        original: "Courier New",
        substitute_file: "LiberationMono-Regular.ttf",
        expected_characters_compared: 92,
    },
];

#[test]
fn every_pair_with_published_metrics_matches_them_character_by_character() {
    for pair in PAIRS {
        let face = bundled_face(pair.substitute_file);
        let verdict =
            verify_metric_compatibility(pair.original, &face).expect("a committed face re-opens");
        match verdict {
            MetricCompatibility::Verified {
                characters_compared,
                worst_deviation_per_mille,
            } => {
                assert_eq!(
                    characters_compared, pair.expected_characters_compared,
                    "`{}` was compared on {characters_compared} characters, and the published \
                     table covers {}. A comparison that silently narrowed is a comparison that \
                     stopped proving anything.",
                    pair.original, pair.expected_characters_compared
                );
                assert!(
                    worst_deviation_per_mille <= ADVANCE_TOLERANCE_PER_MILLE,
                    "`{}`: worst deviation {worst_deviation_per_mille}",
                    pair.original
                );
            }
            other => panic!(
                "`{}` should be metric-compatible with `{}`, and the verdict was {other:?}",
                pair.substitute_file, pair.original
            ),
        }
    }
}

/// The ticket's own shape: a *fixed string*, and a total transcribed from published metrics.
///
/// The tolerance is the per-character tolerance accumulated over the string, which is the honest
/// bound: each character may round by up to half a unit of the substitute's own em, and 56 of them
/// may round the same way.
#[test]
fn a_fixed_string_measures_what_the_published_metrics_say_it_should() {
    /// 56 characters, covering every lowercase letter, four capitals, the ten digits, a comma, a
    /// full stop and nine spaces.
    const PANGRAM: &str = "The quick brown fox jumps over the lazy dog, 0123456789.";
    /// Ten characters: the only ones any published source pins for Calibri.
    const DIGITS: &str = "0123456789";

    struct Expectation {
        substitute_file: &'static str,
        text: &'static str,
        /// The published total, in thousandths of an em.
        expected_per_mille: f64,
        source: &'static str,
    }

    let expectations = [
        Expectation {
            substitute_file: "LiberationSans-Regular.ttf",
            text: PANGRAM,
            // Sum of the Helvetica AFM widths (1000/em) for the 56 characters above:
            // 4 capitals + 35 lowercase + 10 digits + comma + period + 9 spaces = 26180.
            expected_per_mille: 26_180.0,
            source:
                "(URW)++ base-35 NimbusSans-Regular.afm — the published Helvetica widths Arial \
                     was drawn to",
        },
        Expectation {
            substitute_file: "LiberationSerif-Regular.ttf",
            text: PANGRAM,
            expected_per_mille: 24_025.0,
            source: "(URW)++ base-35 NimbusRoman-Regular.afm — the published Times-Roman widths",
        },
        Expectation {
            substitute_file: "LiberationMono-Regular.ttf",
            text: PANGRAM,
            // Courier is monospaced at 600/1000: 56 × 600.
            expected_per_mille: 33_600.0,
            source: "(URW)++ base-35 NimbusMonoPS-Regular.afm — the published Courier widths, 600 \
                     for every glyph",
        },
        Expectation {
            substitute_file: "Carlito-Regular.ttf",
            text: DIGITS,
            // ECMA-376 Part 1 §18.3.1.13: Calibri's Maximum Digit Width is 1038 units on a
            // 2048-unit em, and Calibri's figures are tabular, so all ten share it.
            // 10 × 1038 × 1000 / 2048 = 5068.359375.
            expected_per_mille: 5_068.359_375,
            source: "ECMA-376 Part 1 §18.3.1.13, Calibri's Maximum Digit Width of 1038/2048",
        },
    ];

    for expectation in expectations {
        let face = bundled_face(expectation.substitute_file);
        let reader = face.reader().expect("a committed face re-opens");
        let measured = reader
            .unshaped_advance(expectation.text)
            .expect("the face covers plain ASCII");
        let character_count = expectation.text.chars().count();
        #[allow(clippy::cast_precision_loss)]
        let tolerance = character_count as f64 * ADVANCE_TOLERANCE_PER_MILLE;
        let deviation = (measured.per_mille() - expectation.expected_per_mille).abs();
        assert!(
            deviation <= tolerance,
            "`{}` set in {} measures {:.6} thousandths of an em, and {} says {:.6} — a difference \
             of {deviation:.6}, over a tolerance of {tolerance:.6}. Every line of this document \
             would break somewhere else.",
            expectation.text,
            expectation.substitute_file,
            measured.per_mille(),
            expectation.source,
            expectation.expected_per_mille,
        );
    }
}

/// Line height is the other half of pagination: equal advances put the same words on a line, and
/// equal vertical metrics put the same number of lines on a page.
///
/// # What is asserted, and what deliberately is not
///
/// The `hhea` triple and the em square decide the baseline-to-baseline distance, so those must
/// agree exactly. **Cap height and x-height must not**, and this was measured rather than assumed:
/// Arial's published `OS/2.sCapHeight` is 1467 and Liberation Sans's is 1409; Arial's x-height is
/// 1062 and Liberation Sans's 1082. They are *design* metrics — they place small capitals and
/// decide how a drop cap is scaled, and they move no line break — and the Liberation family
/// deliberately does not match them. They stay in `mjx_text::reference` as published facts R03 and
/// R04 will want, and out of this assertion, because asserting them would be asserting a promise
/// nobody made.
#[test]
fn vertical_metrics_match_the_published_metrics_of_the_original() {
    struct Expectation {
        original: &'static str,
        substitute_file: &'static str,
    }

    // Every number compared here is the literal in `mjx_text::reference`, transcribed from
    // `@capsizecss/metrics` 4.2.0, which measured Microsoft's own faces.
    let expectations = [
        Expectation {
            original: "Arial",
            substitute_file: "LiberationSans-Regular.ttf",
        },
        Expectation {
            original: "Times New Roman",
            substitute_file: "LiberationSerif-Regular.ttf",
        },
        Expectation {
            original: "Courier New",
            substitute_file: "LiberationMono-Regular.ttf",
        },
    ];

    for expectation in expectations {
        let reference = reference_for_family(expectation.original)
            .unwrap_or_else(|| panic!("no reference for `{}`", expectation.original));
        let vertical = reference.vertical.unwrap_or_else(|| {
            panic!(
                "`{}` has no published vertical metrics",
                expectation.original
            )
        });
        assert_eq!(
            vertical.authority,
            ReferenceAuthority::Published,
            "`{}`'s vertical metrics are not published data",
            expectation.original
        );

        let face = bundled_face(expectation.substitute_file);
        let metrics = face.metrics();
        assert_eq!(
            metrics.units_per_em,
            vertical.units_per_em,
            "`{}` is drawn on a {}-unit em and `{}` on a {}-unit one",
            expectation.substitute_file,
            metrics.units_per_em,
            expectation.original,
            vertical.units_per_em
        );
        assert_eq!(
            (metrics.ascender, metrics.descender, metrics.line_gap),
            (vertical.ascender, vertical.descender, vertical.line_gap),
            "`{}`'s hhea metrics differ from the published `{}` ones, so the two put a different \
             number of lines on a page",
            expectation.substitute_file,
            expectation.original
        );

        let expected_line_height = i32::from(vertical.ascender) - i32::from(vertical.descender)
            + i32::from(vertical.line_gap);
        assert_eq!(
            metrics.line_height(),
            expected_line_height,
            "`{}` sets {} font units between baselines and the published `{}` sets \
             {expected_line_height}",
            expectation.substitute_file,
            metrics.line_height(),
            expectation.original
        );

        // The design metrics are read, because a face that carries none of them is a face whose
        // `OS/2` table this crate has misread — but they are not compared. See the test's
        // documentation for why.
        assert!(
            metrics.cap_height.is_some() && metrics.x_height.is_some(),
            "`{}` reports no cap height or x-height at all",
            expectation.substitute_file
        );
    }
}

/// The negative case, and the reason the four assertions above are evidence rather than a
/// tautology: the same machinery, run over a pair that is *not* metric-compatible, must say so.
///
/// Carlito is a Calibri clone; Arial is a Helvetica clone. Their advances are nowhere near each
/// other — Carlito's `A` is 578.6 thousandths of an em and Arial's is 667 — so a comparison that
/// could not fail would be caught here.
#[test]
fn a_face_that_is_not_metric_compatible_is_reported_as_divergent() {
    let carlito = bundled_face("Carlito-Regular.ttf");
    let verdict =
        verify_metric_compatibility("Arial", &carlito).expect("a committed face re-opens");
    match verdict {
        MetricCompatibility::Divergent {
            characters_compared,
            worst_deviation_per_mille,
            worst_character,
        } => {
            assert_eq!(characters_compared, 92);
            assert!(
                worst_deviation_per_mille > ADVANCE_TOLERANCE_PER_MILLE,
                "a divergent verdict must exceed the tolerance"
            );
            assert!(
                !worst_character.is_control(),
                "the worst character should be a real one, and it was {worst_character:?}"
            );
        }
        other => panic!(
            "Carlito is a Calibri clone, not an Arial one, and comparing it against Arial's \
             published widths should be `Divergent`; it was {other:?}"
        ),
    }
}

/// The fifth pair `docs/UI_PLATFORM_PLAN.md` §10 names — Cambria → Caladea — has **no published
/// reference**, and this suite says so out loud rather than leaving the gap to be discovered.
///
/// Microsoft publishes no width table for Cambria, and it is absent from the metric collections
/// that carry Arial, Times New Roman and Courier New. Filling it in needs one measurement of the
/// real face on a licensed Windows machine, which is the offline pass MJXOFF-155 §9 already plans;
/// **advance widths are facts about a font rather than the font program**, so what that pass would
/// commit is numbers.
///
/// Until then the pair resolves and is recorded — see `tests/substitution_manifest.rs` — with the
/// verdict [`MetricCompatibility::Unverified`], which is the truth about it.
#[test]
fn the_cambria_pair_is_recorded_as_unverified_rather_than_silently_skipped() {
    let reference =
        reference_for_family("Cambria").expect("Cambria has an entry, even with no numbers in it");
    assert_eq!(
        reference.advance_authority,
        ReferenceAuthority::Unverified,
        "Cambria's entry must not claim to be published data"
    );
    assert_eq!(reference.covered_character_count(), 0);
    assert!(
        reference
            .advance_provenance
            .contains("no published metric table"),
        "the entry must say why it is empty"
    );

    let caladea = bundled_face("Caladea-Regular.ttf");
    assert_eq!(
        verify_metric_compatibility("Cambria", &caladea).expect("a committed face re-opens"),
        MetricCompatibility::Unverified {
            reason: UnverifiedReason::ReferenceCarriesNoPublishedNumbers
        },
        "with no published Cambria metrics, the verdict must be `Unverified` — never `Verified`"
    );
}

/// A family nobody published anything about is a different answer from a family whose entry is
/// empty, and the two must not be conflated.
#[test]
fn an_unknown_original_is_distinguished_from_one_with_an_empty_entry() {
    let liberation = bundled_face("LiberationSans-Regular.ttf");
    assert_eq!(
        verify_metric_compatibility("Wingdings", &liberation).expect("a committed face re-opens"),
        MetricCompatibility::Unverified {
            reason: UnverifiedReason::NoReferenceForTheOriginal
        }
    );
}
