//! The seven-tier text ladder, tier by tier, against a **file** rather than a runtime-built deck.
//!
//! `text_inheritance.rs` is the runtime counterpart: it loads `layouts.pptx` and then mutates it
//! through the builder API to reach the interesting cases. That leaves the tiers awkward to reach
//! from a builder — a partial layout override, a level a style does not define — covered only
//! indirectly, or by hand-injecting XML into a raw tree. (A shape's own `a:lstStyle` used to be on
//! that list; `shape_list_style.rs` now drives it through its public setters.)
//!
//! `text_levels.pptx` states all of it in the file. Every tier owns a facet no other tier touches, so
//! a failure here names the rung that broke rather than "inheritance is wrong somewhere".
//!
//! The values asserted are cited to ECMA-376 Part 1 where the spec, rather than the fixture, is what
//! makes them right.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Bullet, ColorSpec, FillSpec, FontSlot, ParagraphPropertiesSpec, TextAlignment};
use mjx_opc::Package;
use mjx_pptx::{Presentation, Surface};

/// Slide 0 of `text_levels.pptx`, in shape-tree order.
const TITLE: usize = 0;
const BODY: usize = 1;
const FOOTER: usize = 2;
const TEXT_BOX: usize = 3;
const AUTOSHAPE: usize = 4;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn levels() -> Presentation {
    Presentation::open(&fixture("text_levels.pptx")).expect("open")
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn bullet_character(spec: &ParagraphPropertiesSpec) -> Option<&str> {
    match spec.bullet()? {
        Bullet::Character(character) => Some(character.character.as_str()),
        _ => None,
    }
}

fn solid_hex(fill: Option<&FillSpec>) -> Option<&str> {
    match fill? {
        FillSpec::Solid(ColorSpec::Srgb(hex)) => Some(hex.as_str()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// Tiers 1 and 2 — the run and the paragraph, both stated in the file
// ---------------------------------------------------------------------------------------------

#[test]
fn a_paragraph_default_beats_every_tier_below_it() {
    let mut pres = levels();

    // Paragraph 0 sits at level 0, where the master's `p:bodyStyle` says 32pt. Its own
    // `a:pPr > a:defRPr` says 14pt, and the run declares no size of its own.
    let effective = pres
        .effective_run_properties(0, BODY, 0, 0)
        .expect("effective run");
    assert_eq!(effective.size_points(), Some(14.0));
}

#[test]
fn an_explicit_off_on_a_run_beats_the_layouts_bold() {
    let mut pres = levels();

    // Paragraph 1 is at level 1, where the layout placeholder's `a:lvl2pPr` declares `b="1"`.
    // The run answers `b="0"`, and an explicit false is a statement, not an absence.
    let effective = pres
        .effective_run_properties(0, BODY, 1, 0)
        .expect("effective run");
    assert_eq!(effective.is_bold(), Some(false));

    // The size still comes from the master at that level — the run overrode one facet, not the tier.
    assert_eq!(effective.size_points(), Some(28.0));
}

// ---------------------------------------------------------------------------------------------
// Tier 3 — the shape's own `a:lstStyle`, as the file states it
// ---------------------------------------------------------------------------------------------

#[test]
fn the_shapes_own_list_style_beats_the_layouts_and_the_masters() {
    let mut pres = levels();

    // Level 2 is stated by all three: the shape says 26pt, the layout italic and right-aligned,
    // the master 24pt with a bullet, an indent and a scheme colour. One read separates them.
    let run = pres
        .effective_run_properties(0, BODY, 2, 0)
        .expect("effective run");
    assert_eq!(run.size_points(), Some(26.0), "tier 3 beats tier 5");
    assert_eq!(run.is_italic(), Some(true), "tier 4 supplies the italic");

    let paragraph = pres
        .effective_paragraph_properties(0, BODY, 2)
        .expect("effective paragraph");
    assert_eq!(
        paragraph.alignment(),
        Some(TextAlignment::Right),
        "tier 4 supplies the alignment"
    );
    assert_eq!(
        bullet_character(&paragraph),
        Some("»"),
        "tier 5 still supplies the bullet"
    );
}

// ---------------------------------------------------------------------------------------------
// Tier 4 — the layout placeholder, overriding only what it declares
// ---------------------------------------------------------------------------------------------

#[test]
fn the_layout_placeholder_overrides_only_the_levels_it_declares() {
    let mut pres = levels();

    // The layout declares bold at level 1 and italic at level 2, and nothing at level 3.
    let level_three = pres
        .effective_run_properties(0, BODY, 3, 0)
        .expect("effective run");
    assert_eq!(level_three.is_bold(), None);
    assert_eq!(level_three.is_italic(), None);
    assert_eq!(
        level_three.size_points(),
        Some(20.0),
        "the master alone answers level 3"
    );
}

// ---------------------------------------------------------------------------------------------
// Tier 5 — the master's `p:txStyles`, level by level and slot by slot
// ---------------------------------------------------------------------------------------------

#[test]
fn each_level_reads_the_master_level_that_matches_it() {
    let mut pres = levels();

    // `a:pPr@lvl` selects the `a:lvlNpPr` every tier below the run contributes — the off-by-one
    // included: level 0 reads `a:lvl1pPr`. Sizes here come from the master except at levels 0 and 2,
    // which the paragraph and the shape override; the bullets and indents are the master's alone.
    let expected: [(usize, &str, f64); 4] = [
        (0, "•", 342_900.0),
        (1, "–", 742_950.0),
        (2, "»", 1_143_000.0),
        (3, "›", 1_600_200.0),
    ];

    for (para_idx, bullet, margin_emu) in expected {
        let paragraph = pres
            .effective_paragraph_properties(0, BODY, para_idx)
            .unwrap_or_else(|e| panic!("paragraph {para_idx}: {e}"));
        assert_eq!(
            bullet_character(&paragraph),
            Some(bullet),
            "bullet at level {para_idx}"
        );
        // 12700 EMU to the point.
        assert_eq!(
            paragraph.left_margin_points(),
            Some(margin_emu / 12700.0),
            "left margin at level {para_idx}"
        );
    }
}

#[test]
fn a_title_takes_the_title_style_not_the_body_style() {
    let mut pres = levels();
    let run = pres
        .effective_run_properties(0, TITLE, 0, 0)
        .expect("effective run");
    let paragraph = pres
        .effective_paragraph_properties(0, TITLE, 0)
        .expect("effective paragraph");

    assert_eq!(run.size_points(), Some(40.0));
    assert_eq!(paragraph.alignment(), Some(TextAlignment::Center));
    // `p:bodyStyle`'s level-0 bullet must not reach a title.
    assert_eq!(bullet_character(&paragraph), None);
}

#[test]
fn a_footer_placeholder_takes_the_other_style() {
    let mut pres = levels();
    let run = pres
        .effective_run_properties(0, FOOTER, 0, 0)
        .expect("effective run");

    // `p:otherStyle` says 12pt; `p:bodyStyle` at the same level says 32pt.
    assert_eq!(run.size_points(), Some(12.0));
}

#[test]
fn resolving_from_a_layout_surface_still_reaches_the_masters_text_styles() {
    let mut pres = levels();

    // ECMA-376 Part 1 §19.3.1.52: `p:txStyles` "is only for use within the Slide Master", so tier 5
    // reads the last part of the chain rather than walking it. A layout's own chain is
    // [layout, master], so resolving from the layout surface must still land on the master.
    let run = pres
        .effective_run_properties(Surface::Layout(0), 1, 0, 0)
        .expect("effective run");
    assert_eq!(run.size_points(), Some(32.0));
}

// ---------------------------------------------------------------------------------------------
// The `a:defPPr` rung — ECMA-376 Part 1 §21.1.2.2.2 / §21.1.2.2.6
// ---------------------------------------------------------------------------------------------

#[test]
fn a_level_no_tier_defines_falls_to_the_styles_def_ppr() {
    let mut pres = levels();

    // Paragraph 4 is at level 4. No tier declares an `a:lvl5pPr`, so nothing answers at that level
    // and the body style's own `a:defPPr` — "the paragraph properties that are to be applied when no
    // other paragraph properties have been specified" — is what remains.
    let run = pres
        .effective_run_properties(0, BODY, 4, 0)
        .expect("effective run");
    assert_eq!(run.size_points(), Some(10.0));
    assert!(run.underline().is_some(), "the defPPr's u=\"sng\" applies");

    let paragraph = pres
        .effective_paragraph_properties(0, BODY, 4)
        .expect("effective paragraph");
    assert_eq!(paragraph.alignment(), Some(TextAlignment::Justified));

    // There is no fallback to `a:lvl1pPr`: §21.1.2.4.13 keys the level elements strictly to
    // `a:pPr@lvl`, so level 0's bullet and indent must not leak down here.
    assert_eq!(bullet_character(&paragraph), None);
    assert_eq!(paragraph.left_margin_points(), None);
}

// ---------------------------------------------------------------------------------------------
// Shapes that are not placeholders — ECMA-376 Part 1 §19.3.1.35
// ---------------------------------------------------------------------------------------------

#[test]
fn a_text_box_takes_the_masters_body_style() {
    let mut pres = levels();

    // "Text box styling is handled from within the bodyStyle element" — 32pt at level 0.
    let run = pres
        .effective_run_properties(0, TEXT_BOX, 0, 0)
        .expect("effective run");
    assert_eq!(run.size_points(), Some(32.0));

    // It is not a placeholder, so tier 4 has no slot to match: the layout's bold and italic, and
    // the shape-level `a:lstStyle` on the body placeholder, cannot reach it.
    assert_eq!(run.is_bold(), None);
}

#[test]
fn a_shape_that_is_not_a_text_box_takes_the_masters_other_style() {
    let mut pres = levels();

    // "…for specifying the text formatting of text within a slide shape but not within a text box."
    let run = pres
        .effective_run_properties(0, AUTOSHAPE, 0, 0)
        .expect("effective run");
    assert_eq!(run.size_points(), Some(12.0));
}

// ---------------------------------------------------------------------------------------------
// Tier 6 — `p:defaultTextStyle`
// ---------------------------------------------------------------------------------------------

#[test]
fn the_presentation_default_supplies_what_no_master_style_states() {
    let mut pres = levels();

    // `p:defaultTextStyle` is the only tier in the deck that says `i="1"` at level 0. The master's
    // body style, which sits above it, states a size but no slant — so both survive on a text box.
    let run = pres
        .effective_run_properties(0, TEXT_BOX, 0, 0)
        .expect("effective run");
    assert_eq!(run.is_italic(), Some(true), "tier 6 supplies the italic");
    assert_eq!(
        run.size_points(),
        Some(32.0),
        "tier 5 still supplies the size"
    );
}

// ---------------------------------------------------------------------------------------------
// Tier 7 — the theme font scheme, and colour baking
// ---------------------------------------------------------------------------------------------

#[test]
fn a_theme_font_reference_in_the_master_resolves_to_the_themes_font() {
    let mut pres = levels();

    // The master's `a:lvl2pPr` names `+mn-lt`; nothing between it and the run states a typeface.
    let run = pres
        .effective_run_properties(0, BODY, 1, 0)
        .expect("effective run");
    assert_eq!(
        run.font(FontSlot::Latin).map(|f| f.typeface.as_str()),
        Some("Verdana")
    );
}

#[test]
fn a_scheme_colour_bakes_to_concrete_rgb_through_the_colour_map() {
    let mut pres = levels();

    // The master's `a:lvl3pPr` fills its text with `a:schemeClr val="tx2"`. The master's `p:clrMap`
    // maps `tx2` onto the theme's `dk2`, which this theme defines as `1F3864`. An effective answer
    // resolves both hops rather than handing back the slot name.
    let run = pres
        .effective_run_properties(0, BODY, 2, 0)
        .expect("effective run");
    assert_eq!(solid_hex(run.fill()), Some("1F3864"));
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn resolving_the_whole_ladder_keeps_all_parts_byte_identical() {
    let bytes = fixture("text_levels.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    for shape in [TITLE, FOOTER, TEXT_BOX, AUTOSHAPE] {
        pres.effective_run_properties(0, shape, 0, 0)
            .unwrap_or_else(|e| panic!("shape {shape}: {e}"));
    }
    for para_idx in 0..5 {
        pres.effective_run_properties(0, BODY, para_idx, 0)
            .unwrap_or_else(|e| panic!("body run {para_idx}: {e}"));
        pres.effective_paragraph_properties(0, BODY, para_idx)
            .unwrap_or_else(|e| panic!("body paragraph {para_idx}: {e}"));
    }
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &snapshot {
        assert_eq!(
            reopened.get(name),
            Some(original),
            "resolving the ladder dirtied part {name}"
        );
    }
}
