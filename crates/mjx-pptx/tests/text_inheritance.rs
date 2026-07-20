//! Integration tests for **effective** text formatting — what a run and a paragraph actually render
//! as, once every tier of the inheritance ladder is resolved.
//!
//! The fixture is deliberately spare: `layouts.pptx`'s title and body runs declare *nothing*. Every
//! number asserted here therefore exists nowhere in the slide — it comes from the layout placeholder's
//! `a:lstStyle` or the master's `p:txStyles`, which is the whole point. Each test names the tier it is
//! about, so a failure says which rung broke.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    Bullet, CharacterPropertiesSpec, FontSlot, IndentLevel, ParagraphPropertiesSpec, TextAlignment,
    TextFont,
};
use mjx_opc::Package;
use mjx_pptx::Presentation;

/// Slide 0 of `layouts.pptx`: shape 0 is a `title` placeholder, shape 1 the `idx="1"` content slot.
const TITLE: usize = 0;
const BODY: usize = 1;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn layouts() -> Presentation {
    Presentation::open(&fixture("layouts.pptx")).expect("open")
}

/// The bullet character a paragraph resolves to, for the level tests.
fn bullet_character(spec: &ParagraphPropertiesSpec) -> Option<&str> {
    match spec.bullet()? {
        Bullet::Character(character) => Some(character.character.as_str()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// Tier 5 — the master's `p:txStyles`
// ---------------------------------------------------------------------------------------------

#[test]
fn a_title_that_declares_nothing_renders_at_the_masters_size() {
    let mut pres = layouts();

    // The run's own `a:rPr` says only `lang` and `dirty`.
    assert_eq!(
        pres.run_properties(0, TITLE, 0, 0)
            .expect("declared")
            .and_then(|spec| spec.size_points()),
        None
    );

    // 44pt centred lives only in the master's `p:titleStyle > a:lvl1pPr`.
    let effective = pres
        .effective_run_properties(0, TITLE, 0, 0)
        .expect("effective run");
    assert_eq!(effective.size_points(), Some(44.0));

    let paragraph = pres
        .effective_paragraph_properties(0, TITLE, 0)
        .expect("effective paragraph");
    assert_eq!(paragraph.alignment(), Some(TextAlignment::Center));
}

#[test]
fn a_body_placeholder_takes_the_body_style_not_the_title_style() {
    let mut pres = layouts();
    let effective = pres
        .effective_run_properties(0, BODY, 0, 0)
        .expect("effective run");

    // `p:bodyStyle > a:lvl1pPr` is 32pt — the title's 44pt must not reach a body slot.
    assert_eq!(effective.size_points(), Some(32.0));
}

// ---------------------------------------------------------------------------------------------
// The level axis — one paragraph, three answers
// ---------------------------------------------------------------------------------------------

#[test]
fn demoting_a_paragraph_changes_what_it_renders_as() {
    // Each level reads its own `a:lvlNpPr` of the master's `bodyStyle`, and nothing else about the
    // paragraph changes. This is what a user sees when they press Tab.
    let expected = [
        (0, 32.0, "•", 27.0),
        (1, 28.0, "–", 58.5),
        (2, 24.0, "»", 90.0),
    ];

    for (level, size, bullet, margin) in expected {
        let mut pres = layouts();
        pres.set_paragraph_properties(
            0,
            BODY,
            0,
            &ParagraphPropertiesSpec::new().with_level(IndentLevel::of(level)),
        )
        .expect("set level");

        let paragraph = pres
            .effective_paragraph_properties(0, BODY, 0)
            .expect("effective paragraph");
        assert_eq!(bullet_character(&paragraph), Some(bullet), "level {level}");
        assert_eq!(
            paragraph.left_margin_points(),
            Some(margin),
            "level {level}"
        );

        let run = pres
            .effective_run_properties(0, BODY, 0, 0)
            .expect("effective run");
        assert_eq!(run.size_points(), Some(size), "level {level}");
    }
}

#[test]
fn a_level_the_master_does_not_define_inherits_nothing_from_it() {
    // `bodyStyle` defines three levels; a paragraph at level 4 finds no `a:lvl5pPr` and the tier
    // simply contributes nothing — it does not fall back to level 1.
    let mut pres = layouts();
    pres.set_paragraph_properties(
        0,
        BODY,
        0,
        &ParagraphPropertiesSpec::new().with_level(IndentLevel::of(4)),
    )
    .expect("set level");

    let paragraph = pres
        .effective_paragraph_properties(0, BODY, 0)
        .expect("effective paragraph");
    assert_eq!(paragraph.bullet(), None);
    assert_eq!(paragraph.left_margin_points(), None);
    assert_eq!(paragraph.level(), Some(IndentLevel::of(4)));
}

// ---------------------------------------------------------------------------------------------
// Tier order — who beats whom
// ---------------------------------------------------------------------------------------------

#[test]
fn a_layout_placeholder_overrides_the_master_and_inherits_the_rest() {
    let mut pres = layouts();
    let effective = pres
        .effective_run_properties(0, BODY, 0, 0)
        .expect("effective run");

    // slideLayout2's `idx="1"` placeholder declares only `b="1"` in its `a:lstStyle`; the size still
    // comes from the master below it.
    assert_eq!(effective.is_bold(), Some(true), "from the layout");
    assert_eq!(effective.size_points(), Some(32.0), "from the master");
}

#[test]
fn a_run_beats_every_tier_below_it() {
    let mut pres = layouts();
    pres.set_run_properties(
        0,
        BODY,
        0,
        0,
        &CharacterPropertiesSpec::new()
            .with_size_points(11.0)
            .with_bold(false),
    )
    .expect("set run properties");

    let effective = pres
        .effective_run_properties(0, BODY, 0, 0)
        .expect("effective run");
    assert_eq!(
        effective.size_points(),
        Some(11.0),
        "the run's own size wins"
    );
    assert_eq!(
        effective.is_bold(),
        Some(false),
        "`b=\"0\"` is a decision, and it beats the layout's bold"
    );
}

// (Tier 3 — a shape's own `a:lstStyle` — has no public setter to build it with, so it is covered by
// a unit test in `presentation.rs` that injects one into the tree directly.)

// ---------------------------------------------------------------------------------------------
// The shapes that inherit nothing
// ---------------------------------------------------------------------------------------------

#[test]
fn a_plain_text_box_takes_no_master_text_style() {
    let mut pres = layouts();
    let idx = pres
        .add_text_box(
            0,
            "free-standing",
            mjx_pptx::ShapeBounds::from_inches(1.0, 1.0, 3.0, 1.0),
        )
        .expect("add text box");

    // A text box is not a placeholder, so neither `p:bodyStyle` nor any placeholder's list style
    // applies to it. With no `p:defaultTextStyle` in this deck either, nothing is inherited at all.
    let effective = pres
        .effective_run_properties(0, idx, 0, 0)
        .expect("effective run");
    assert_eq!(effective.size_points(), None);
    assert_eq!(effective.is_bold(), None);
}

#[test]
fn a_deck_without_master_text_styles_still_answers() {
    // `sample.pptx`'s master has no `p:txStyles` and the deck no `p:defaultTextStyle`: resolution
    // must degrade to "the tiers that exist", not fail.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let effective = pres
        .effective_run_properties(0, 0, 0, 0)
        .expect("effective run");
    let declared = pres.run_properties(0, 0, 0, 0).expect("declared");

    assert_eq!(
        effective.size_points(),
        declared.and_then(|d| d.size_points())
    );
    pres.effective_paragraph_properties(0, 0, 0)
        .expect("effective paragraph");
}

// ---------------------------------------------------------------------------------------------
// Tier 7 — the theme font scheme
// ---------------------------------------------------------------------------------------------

#[test]
fn a_theme_font_reference_resolves_to_the_themes_font() {
    let mut pres = layouts();
    pres.set_run_properties(
        0,
        BODY,
        0,
        0,
        &CharacterPropertiesSpec::new()
            .with_font_for(FontSlot::Latin, TextFont::named("+mn-lt"))
            .with_font_for(FontSlot::EastAsian, TextFont::named("+mj-ea")),
    )
    .expect("set run properties");

    // Declared: the reference. Effective: the font the theme names.
    assert_eq!(
        pres.run_properties(0, BODY, 0, 0)
            .expect("declared")
            .and_then(|spec| spec.font(FontSlot::Latin).map(|f| f.typeface.clone())),
        Some("+mn-lt".to_owned())
    );

    let effective = pres
        .effective_run_properties(0, BODY, 0, 0)
        .expect("effective run");
    assert_eq!(
        effective.font(FontSlot::Latin).map(|f| f.typeface.as_str()),
        Some("Calibri")
    );
    // The theme's `majorFont` declares an empty `a:ea`, so that is the honest answer for `+mj-ea`.
    assert_eq!(
        effective
            .font(FontSlot::EastAsian)
            .map(|f| f.typeface.as_str()),
        Some("")
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn resolving_effective_formatting_keeps_all_parts_byte_identical() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Every tier is touched: the slide, its layout, the master, the theme and presentation.xml.
    pres.effective_run_properties(0, TITLE, 0, 0)
        .expect("title run");
    pres.effective_run_properties(0, BODY, 0, 0)
        .expect("body run");
    pres.effective_paragraph_properties(0, BODY, 0)
        .expect("body paragraph");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &snapshot {
        assert_eq!(
            reopened.get(name),
            Some(original),
            "resolving effective formatting dirtied part {name}"
        );
    }
}
