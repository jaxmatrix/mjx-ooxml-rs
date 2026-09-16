//! **`w:themeTint` and `w:themeShade` are applied to the theme colour they modify** (MJXOFF-243, RC04).
//!
//! # What is true today, and why it is not a footnote
//!
//! `crates/mjx-docx/docs/effective_properties.md` states the gap outright: the two attributes are
//! *"read back on [`EffectiveColor`]'s own theme-colour siblings but **not baked into the resolved
//! `RRGGBB`**"*, because baking them would mean either reimplementing DrawingML's transform outside
//! `mjx-dml` or building synthetic markup to carry it. So a run that says *accent1, half as light*
//! resolves today to accent1 exactly, and a reader painting from `EffectiveColor::Hex` paints the
//! wrong colour with nothing to tell it so.
//!
//! # The rule, sourced rather than guessed
//!
//! ECMA-376 Part 1 (the `themeShade` / `themeTint` attribute descriptions, which are written once
//! and referenced by every element carrying the pair) gives the algorithm in full:
//!
//! > Given an RGB color defined as three hex values in RRGGBB format, the shade is applied as
//! > follows: convert the color to the HSL color format (values from 0 to 1); modify the luminance
//! > factor as follows: `L' = L * Shade_percentage`; convert the resultant HSL color to RGB.
//!
//! and, for a tint, `L' = L * Tint_pct + (1 - Tint_pct)`. Both bytes are *"a hex encoding of the
//! value (from 0–255)"*, so `80` is `128/255` and `40` is `64/255`. The precedence is stated too,
//! and it decides the fixture below, which carries **both**:
//!
//! > If the themeTint is supplied, the value of this attribute shall be ignored.
//!
//! This is HSL luminance — the arithmetic `mjx-dml`'s own `a:lum`/`a:lumMod` transforms already do —
//! and **not** DrawingML's `a:tint`/`a:shade`, which work in linear light and would give a different
//! answer. Picking the wrong one of the two is the way this ticket goes subtly wrong, so the
//! expected values below are stated as hexes rather than as a formula.
//!
//! # The spec's own worked example rounds, and this suite does not
//!
//! The prose works `C0504D` at shade `BF` through to `943634`, having first rounded the HSL triple
//! to two decimal places. Applying the same steps at full precision gives `953735`. The algorithm is
//! the spec's; the rounding is the example's, and an implementation that reproduces the example's
//! two-decimal detour would be wrong for every other colour.

use mjx_docx::{Document, EffectiveColor};
use mjx_fixtures::fixture;

/// `tests/fixtures/run_properties.docx`, whose second paragraph's run states every `EG_RPrBase`
/// member — including the `w:color` this suite is about.
fn run_properties() -> Document {
    Document::open(&fixture("run_properties.docx")).expect("open run_properties.docx")
}

/// The theme colour the fixture's run names, before any tint or shade.
const ACCENT1: &str = "18A303";
/// `accent1` with the run's `w:themeTint="80"` applied: `L' = L * (128/255) + (1 - 128/255)`.
const ACCENT1_TINTED: &str = "6BFC55";
/// `accent1` with the run's `w:themeShade="40"` applied: `L' = L * (64/255)`. **Not** the answer —
/// the tint is supplied, so the shade is ignored — and named here so the test can say so.
const ACCENT1_SHADED: &str = "062901";
/// The literal `w:val` on the same element, which the theme reference overrides outright.
const IGNORED_LITERAL: &str = "FF0000";

/// The run's colour is the theme colour with its tint applied.
#[test]
fn a_theme_colours_tint_is_baked_into_the_resolved_value() {
    let mut document = run_properties();
    let effective = document
        .effective_run_properties(1, 0)
        .expect("paragraph 1, run 0");

    assert_eq!(
        effective.color,
        Some(EffectiveColor::Hex(ACCENT1_TINTED.to_owned())),
        "the run states `w:color w:val=\"{IGNORED_LITERAL}\" w:themeColor=\"accent1\" \
         w:themeTint=\"80\" w:themeShade=\"40\"`, and the fixture's theme defines accent1 as \
         {ACCENT1}. The tint is supplied, so the shade is ignored and the answer is \
         {ACCENT1_TINTED}"
    );
}

/// The three wrong answers, named so that a near-miss is not mistaken for the right one.
#[test]
fn the_resolved_value_is_none_of_the_three_colours_it_is_not() {
    let mut document = run_properties();
    let colour = document
        .effective_run_properties(1, 0)
        .expect("paragraph 1, run 0")
        .color
        .expect("the run states a colour");

    for (wrong, why) in [
        (
            ACCENT1,
            "the untransformed theme colour — the tint was read and not applied",
        ),
        (
            ACCENT1_SHADED,
            "the shade applied instead of the tint, which the specification forbids when both are \
             stated",
        ),
        (
            IGNORED_LITERAL,
            "the literal `w:val`, which a theme reference overrides outright",
        ),
    ] {
        assert_ne!(
            colour,
            EffectiveColor::Hex(wrong.to_owned()),
            "the run resolved to {wrong}: {why}"
        );
    }
}

/// A theme colour stating neither attribute is unchanged, so the bake is conditional.
///
/// Without this the suite could be satisfied by an implementation that transformed every theme
/// colour it resolved, which would move every colour in every Word document that names one.
#[test]
fn a_theme_colour_with_no_tint_or_shade_is_untouched() {
    let mut document =
        Document::open(&fixture("effective_properties.docx")).expect("open the fixture");
    let effective = document
        .effective_run_properties(6, 0)
        .expect("paragraph 6, run 0");
    assert_eq!(
        effective.color,
        Some(EffectiveColor::Hex("4F81BD".to_owned())),
        "paragraph 6's run names `accent1` and states no tint or shade, so it must still resolve to \
         the theme's own 4F81BD"
    );
}

/// The same rule applies to the other elements carrying the pair, read through `w:u`.
///
/// The fixture's underline names `accent2` with no tint or shade, so its resolved value is the
/// theme's own — which is what makes the tinted `w:color` above a *difference* rather than a
/// property of the fixture.
#[test]
fn an_underlines_theme_colour_resolves_by_the_same_rule() {
    let mut document = run_properties();
    let underline = document
        .effective_run_properties(1, 0)
        .expect("paragraph 1, run 0")
        .underline
        .expect("the run states an underline");
    assert_eq!(
        underline.color,
        EffectiveColor::Hex("0369A3".to_owned()),
        "the underline names `accent2`, which the fixture's theme defines as 0369A3, and states \
         neither tint nor shade"
    );
}
