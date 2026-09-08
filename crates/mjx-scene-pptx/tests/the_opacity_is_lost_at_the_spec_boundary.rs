//! ⚠ **This suite asserts a defect, on purpose.** When it goes red, the defect is fixed and this
//! file is deleted.
//!
//! # What is lost, and where
//!
//! `mjx-dml`'s `resolve_fill`, `resolve_line` and `resolve_effects` bake a DrawingML colour down to
//! a `ColorSpec::Srgb` hex **triplet**. A triplet has three channels. An `a:alpha` colour transform
//! is a fourth, `resolve_color` computes it into a `ResolvedColor` that carries it — and then each
//! of the three `resolve_*` functions throws it away, saying so in its own doc comment:
//!
//! > Note: `FillSpec` colors are RGB-only, so a resolved alpha (from an `a:alpha` transform) is not
//! > represented in the result.
//!
//! # Why that is not a footnote
//!
//! The **standard Office theme's third effect style** is
//! `<a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:schemeClr
//! val="phClr"><a:alpha val="63000"/></a:schemeClr></a:outerShdw>` — so every shape that takes its
//! effects from the theme, which is every shape a person styles with PowerPoint's gallery, has a
//! **63 %** shadow. Rendered from the spec, it is a **100 %** shadow: a solid slab of colour under
//! the shape instead of a soft one.
//!
//! It is not exotic and it is not this crate's to fix. `crates/mjx-scene-pptx` receives a
//! `ColorSpec` and cannot invent the channel that is not in it.
//!
//! # What fixing it costs, so that the next reader does not have to measure it again
//!
//! Two shapes of fix, both larger than the crate that suffers from the loss:
//!
//! 1. **Widen `ColorSpec`.** It is constructed at 263 sites across the workspace, matched in both
//!    bindings' `paint.rs`, projected into `mjx_ooxml`'s facade, and exercised by four walkthrough
//!    parity suites in three languages. A new variant would be missed silently by every existing
//!    `ColorSpec::Srgb` match; changing `Srgb(String)` to a struct variant touches all 263.
//! 2. **Grow an opacity vocabulary beside the spec**, and thread it through the three effective
//!    ladders in `crates/mjx-pptx/src/presentation/effective.rs` — each of which walks placeholder
//!    candidates across three parts and resolves a theme style — so that one ladder walk answers
//!    twice.
//!
//! Either is a work item of its own. MJXOFF-170 chose to state the loss and prove it rather than to
//! start one halfway.

use mjx_dml::ColorSpec;
use mjx_pptx::{Presentation, Surface};
use mjx_scene_pptx::paint::color_of;

/// The fixture whose second shape takes its effects from the theme — the case the module
/// documentation describes, in a real package rather than in prose.
fn effects_theme() -> Presentation {
    let bytes = mjx_fixtures::fixture("effects_theme.pptx");
    Presentation::open(&bytes).expect("a well-formed package")
}

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
fn the_resolved_shadow_colour_arrives_opaque() {
    let mut deck = effects_theme();
    let effects = deck
        .effective_shape_effects(Surface::Slide(0), vec![1])
        .expect("the shape resolves its effects")
        .expect("the shape's `a:effectRef` names theme effect style 3");

    let shadow = effects
        .outer_shadow
        .as_ref()
        .expect("theme effect style 3 is an outer shadow");

    // The colour did resolve — `phClr` became the reference's `accent1`, which is the ladder
    // working. What it did not keep is the alpha.
    assert!(
        matches!(&shadow.color, ColorSpec::Srgb(hex) if hex.len() == 6),
        "the shadow's colour resolved to something other than a six-digit hex triplet: {:?}. If it \
         is now a four-channel value, the seam has been fixed — delete this suite.",
        shadow.color
    );

    let colour = color_of(&shadow.color).expect("a six-digit triplet reads as a colour");
    assert_eq!(
        colour.alpha, 0xff,
        "the shadow's colour arrived with an alpha of {:#04x}. The document says 63 % — about \
         {:#04x} — so if this is no longer 0xff the loss described at the top of this file has been \
         repaired, and this whole suite should be deleted rather than adjusted.",
        colour.alpha,
        (0.63_f32 * 255.0).round() as u8
    );
}

#[test]
fn a_fill_loses_it_the_same_way() {
    // The same loss on the other resolver, so that a fix to one and not the other is caught. A
    // theme fill style's colours carry `a:alpha` and `a:lumMod` alike, and only the second survives.
    let mut deck = effects_theme();
    let fill = deck
        .effective_shape_fill(Surface::Slide(0), vec![1])
        .expect("the shape resolves its fill");

    if let Some(mjx_dml::FillSpec::Solid(colour)) = &fill {
        let resolved = color_of(colour);
        assert!(
            resolved.is_none_or(|resolved| resolved.alpha == 0xff),
            "a resolved fill colour arrived with an alpha channel. If `resolve_fill` now carries \
             one, this suite has done its job — delete it."
        );
    }
}
