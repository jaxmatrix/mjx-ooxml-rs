//! **The opacity survives the spec boundary** (MJXOFF-243, RC04).
//!
// The name is kept from MJXOFF-170, which wrote this file to assert the *loss*; RC04 inverted it.
//!
//! # What this file used to say
//!
//! Between MJXOFF-170 and RC04 this suite asserted a defect on purpose: `mjx-dml`'s `resolve_fill`,
//! `resolve_line` and `resolve_effects` baked a DrawingML colour down to a `ColorSpec::Srgb` hex
//! **triplet**, and an `a:alpha` transform is a fourth channel with nowhere to go. The file is kept
//! rather than deleted because the fixture, the reasoning and the measurement are the same — only
//! the expected answer changed — and because a reader who finds the old name in a ledger row, a
//! commit message or `crates/mjx-scene-pptx/src/paint.rs` should land somewhere that says what
//! happened.
//!
//! # Why the case is the standard Office theme's shadow
//!
//! The **standard Office theme's third effect style** is
//! `<a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:schemeClr
//! val="phClr"><a:alpha val="63000"/></a:schemeClr></a:outerShdw>` — so every shape that takes its
//! effects from the theme, which is every shape a person styles with PowerPoint's gallery, has a
//! **63 %** shadow. Rendered without the channel it was a **100 %** shadow: a solid slab of colour
//! under the shape instead of a soft one. That is what makes this the case worth pinning: it is not
//! exotic, it is on every deck anybody sends.
//!
//! # The two halves, and why both are here
//!
//! The first test reads the alpha out of the theme part's **own bytes**, before resolution. Without
//! it the rest would be asserting that an opaque colour is opaque, which is true of a file that
//! never stated an alpha and proves nothing at all. The second and third read the same value out of
//! the resolved spec and out of the `mjx_scene::Color` a painter receives — the two forms RC04
//! decided on.

use mjx_dml::{ColorSpec, ColorTransform, Fraction};
use mjx_pptx::{Presentation, Surface};
use mjx_scene_pptx::paint::color_of;

/// The fixture whose second shape takes its effects from the theme — the case the module
/// documentation describes, in a real package rather than in prose.
fn effects_theme() -> Presentation {
    let bytes = mjx_fixtures::fixture("effects_theme.pptx");
    Presentation::open(&bytes).expect("a well-formed package")
}

/// 63 % as the byte a `mjx_scene::Color` carries: `round(0.63 * 255)` is `160.65`, so `0xA1`.
const SIXTY_THREE_PERCENT: u8 = 0xA1;

#[test]
fn the_theme_shadow_this_suite_is_about_really_is_alpha_in_the_file() {
    // Read *before* resolution, out of the theme part's own bytes. Without this the rest of the
    // suite would be asserting that an opaque colour is opaque, which is true of a file that never
    // stated an alpha and proves nothing at all.
    let bytes = mjx_fixtures::fixture("effects_theme.pptx");
    let package = mjx_opc::Package::open(&bytes).expect("a well-formed package");
    let theme = package
        .part_names()
        .find(|part| part.as_str().contains("theme"))
        .expect("the fixture carries a theme part");
    let raw = package.part_bytes(&theme).expect("the theme's bytes");
    let text = String::from_utf8_lossy(raw);
    assert!(
        text.contains("<a:alpha val=\"63000\"/>"),
        "the fixture's theme no longer states the 63 % shadow alpha this suite is about, so \
         everything below would pass vacuously"
    );
}

#[test]
fn the_resolved_shadow_colour_keeps_its_opacity() {
    let mut deck = effects_theme();
    let effects = deck
        .effective_shape_effects(Surface::Slide(0), vec![1])
        .expect("the shape resolves its effects")
        .expect("the shape's `a:effectRef` names theme effect style 3");

    let shadow = effects
        .outer_shadow
        .as_ref()
        .expect("theme effect style 3 is an outer shadow");

    // The colour resolved — `phClr` became the reference's `accent1`, which is the ladder working —
    // and the opacity travelled with it as the one transform a resolved colour keeps.
    let ColorSpec::Transformed { base, transforms } = &shadow.color else {
        panic!(
            "the shadow's colour resolved to {:?}. A 63 % shadow is a triplet under one \
             `a:alpha`; a bare `Srgb` here is the colour at 100 %, which paints a solid slab under \
             every theme-styled shape.",
            shadow.color
        );
    };
    assert!(
        matches!(base.as_ref(), ColorSpec::Srgb(hex) if hex.len() == 6),
        "the shadow's base colour is {base:?} rather than a six-digit triplet"
    );
    assert_eq!(
        transforms,
        &vec![ColorTransform::Alpha(Fraction::from_ratio(0.63))],
        "the document says 63 %, and every other transform is baked into the triplet"
    );
}

#[test]
fn the_shadow_reaches_the_scene_at_the_opacity_the_theme_states() {
    let mut deck = effects_theme();
    let shadow = deck
        .effective_shape_effects(Surface::Slide(0), vec![1])
        .expect("the shape resolves its effects")
        .expect("the shape's `a:effectRef` names theme effect style 3")
        .outer_shadow
        .expect("theme effect style 3 is an outer shadow");

    let colour = color_of(&shadow.color).expect("a resolved colour reads as a colour");
    assert_eq!(
        colour.alpha, SIXTY_THREE_PERCENT,
        "the shadow arrived at an alpha of {:#04x}; the document says 63 %, which is {:#04x}",
        colour.alpha, SIXTY_THREE_PERCENT
    );
}

#[test]
fn an_opaque_fill_on_the_same_deck_is_still_opaque() {
    // The control: the same deck's theme fill style states no `a:alpha`, so its colour must still
    // resolve to a bare triplet. Without this, wrapping every colour in an alpha would pass above.
    let mut deck = effects_theme();
    let fill = deck
        .effective_shape_fill(Surface::Slide(0), vec![1])
        .expect("the shape resolves its fill");

    if let Some(mjx_dml::FillSpec::Solid(colour)) = &fill {
        let resolved = color_of(colour).expect("a resolved colour reads as a colour");
        assert_eq!(
            resolved.alpha, 0xff,
            "the shape's fill states no opacity and arrived at {:#04x}",
            resolved.alpha
        );
    }
}
