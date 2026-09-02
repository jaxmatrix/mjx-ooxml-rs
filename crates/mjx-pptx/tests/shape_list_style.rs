//! Authoring the shape's own list style — tier 3 of the text ladder (MJX-43).
//!
//! The tier was readable and resolvable from the day the ladder was written, and unwritable: a caller
//! could ask what a shape offers at an indent level and never say it. These tests drive the setters
//! against `text_levels.pptx`, whose shapes state every other tier, so a value that arrives can only
//! have come from the one under test.
//!
//! The fixture's body placeholder already carries an `a:lstStyle` (level 2 at 26pt) and the master
//! states 32/28/24/20pt down the levels, which is what makes the assertions here discriminating: an
//! authored size has to displace a number the file already has an answer for.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{CharacterPropertiesSpec, IndentLevel, ParagraphPropertiesSpec, TextAlignment};
use mjx_opc::Package;
use mjx_pptx::Presentation;

/// Slide 0 of `text_levels.pptx`, in shape-tree order.
const BODY: usize = 1;
const TEXT_BOX: usize = 3;

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

/// A spec that states one thing a level can carry: the size its runs default to.
fn size(points: f64) -> ParagraphPropertiesSpec {
    ParagraphPropertiesSpec::new()
        .with_default_run_properties(CharacterPropertiesSpec::new().with_size_points(points))
}

// ---------------------------------------------------------------------------------------------
// Authoring
// ---------------------------------------------------------------------------------------------

#[test]
fn a_level_authored_on_a_shape_reaches_every_paragraph_at_that_level() {
    let mut pres = levels();

    // The text box states no list style of its own, so level 0 comes from the master's `p:bodyStyle`.
    assert_eq!(
        pres.effective_run_properties(0, TEXT_BOX, 0, 0)
            .expect("baseline")
            .size_points(),
        Some(32.0),
        "the master answers level 0 before anything is authored"
    );

    pres.set_shape_list_style_level(0, TEXT_BOX, IndentLevel::TOP, &size(44.0))
        .expect("author level 0");

    assert_eq!(
        pres.effective_run_properties(0, TEXT_BOX, 0, 0)
            .expect("effective")
            .size_points(),
        Some(44.0),
        "tier 3 now answers, and 44pt is a size no other tier in this deck states"
    );
}

#[test]
fn what_is_authored_reads_back_as_what_the_shape_declares() {
    let mut pres = levels();
    assert!(
        pres.shape_list_style_level(0, TEXT_BOX, IndentLevel::of(4))
            .expect("read")
            .is_none(),
        "the text box declares nothing at level 4"
    );

    pres.set_shape_list_style_level(
        0,
        TEXT_BOX,
        IndentLevel::of(4),
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Justified),
    )
    .expect("author");

    let declared = pres
        .shape_list_style_level(0, TEXT_BOX, IndentLevel::of(4))
        .expect("read")
        .expect("now declared");
    assert_eq!(declared.alignment(), Some(TextAlignment::Justified));
    assert!(
        pres.shape_list_style_level(0, TEXT_BOX, IndentLevel::of(3))
            .expect("read")
            .is_none(),
        "authoring level 4 did not invent a level 3"
    );
}

#[test]
fn authoring_a_level_merges_with_what_the_shape_already_states() {
    let mut pres = levels();

    // The body placeholder's own `a:lstStyle` states 26pt at level 2 and nothing else. Naming an
    // alignment must not cost it the size.
    pres.set_shape_list_style_level(
        0,
        BODY,
        IndentLevel::of(2),
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Center),
    )
    .expect("author");

    let declared = pres
        .shape_list_style_level(0, BODY, IndentLevel::of(2))
        .expect("read")
        .expect("declared");
    assert_eq!(declared.alignment(), Some(TextAlignment::Center));
    assert_eq!(
        declared
            .default_run_properties()
            .and_then(CharacterPropertiesSpec::size_points),
        Some(26.0),
        "the size the shape already stated survived the merge"
    );

    // The layout states `algn="r"` at level 2; tier 3 sits above it, so the authored centre wins.
    assert_eq!(
        pres.effective_paragraph_properties(0, BODY, 2)
            .expect("effective")
            .alignment(),
        Some(TextAlignment::Center)
    );
}

#[test]
fn a_paragraph_that_states_the_property_itself_still_wins() {
    let mut pres = levels();

    // Paragraph 0 of the body states its own 14pt default. Tier 3 is *beneath* the paragraph.
    pres.set_shape_list_style_level(0, BODY, IndentLevel::TOP, &size(40.0))
        .expect("author level 0");

    assert_eq!(
        pres.effective_run_properties(0, BODY, 0, 0)
            .expect("effective")
            .size_points(),
        Some(14.0),
        "the paragraph's own default still beats the shape's list style"
    );
    // …but a paragraph that states nothing at level 0 would take it, which the text box shows.
    assert_eq!(
        pres.shape_list_style_level(0, BODY, IndentLevel::TOP)
            .expect("read")
            .expect("declared")
            .default_run_properties()
            .and_then(CharacterPropertiesSpec::size_points),
        Some(40.0),
        "the level was written even though nothing renders differently"
    );
}

#[test]
fn the_default_properties_apply_where_no_level_does() {
    let mut pres = levels();

    pres.set_shape_list_style_default(0, TEXT_BOX, &size(9.0))
        .expect("author the default");
    assert_eq!(
        pres.effective_run_properties(0, TEXT_BOX, 0, 0)
            .expect("effective")
            .size_points(),
        Some(9.0),
        "`a:defPPr` answers where the style names no level"
    );

    // A level, once stated, is consulted before the default.
    pres.set_shape_list_style_level(0, TEXT_BOX, IndentLevel::TOP, &size(44.0))
        .expect("author level 0");
    assert_eq!(
        pres.effective_run_properties(0, TEXT_BOX, 0, 0)
            .expect("effective")
            .size_points(),
        Some(44.0)
    );
    assert_eq!(
        pres.shape_list_style_default(0, TEXT_BOX)
            .expect("read")
            .expect("declared")
            .default_run_properties()
            .and_then(CharacterPropertiesSpec::size_points),
        Some(9.0),
        "the default is still there, just outranked"
    );
}

// ---------------------------------------------------------------------------------------------
// Clearing
// ---------------------------------------------------------------------------------------------

#[test]
fn clearing_a_level_falls_back_to_the_tier_below() {
    let mut pres = levels();
    assert_eq!(
        pres.effective_run_properties(0, BODY, 2, 0)
            .expect("baseline")
            .size_points(),
        Some(26.0),
        "the shape's own list style answers level 2"
    );

    assert!(pres
        .clear_shape_list_style_level(0, BODY, IndentLevel::of(2))
        .expect("clear"));
    assert_eq!(
        pres.effective_run_properties(0, BODY, 2, 0)
            .expect("effective")
            .size_points(),
        Some(24.0),
        "the master's `p:bodyStyle` answers again"
    );
    assert!(
        !pres
            .clear_shape_list_style_level(0, BODY, IndentLevel::of(2))
            .expect("clear again"),
        "clearing what is no longer stated reports so"
    );
}

#[test]
fn clearing_a_level_the_shape_never_stated_leaves_the_file_as_it_was() {
    // The *dirtiness* half of this claim is not observable from the public API — re-serialising a
    // well-formed part is byte-identical, so a needless rebuild leaves no trace out here. It is
    // pinned by `a_clear_that_finds_nothing_leaves_the_part_clean` in `presentation.rs`, which can
    // see the package. What this test adds is the end-to-end file comparison.
    let bytes = fixture("text_levels.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    assert!(!pres
        .clear_shape_list_style_level(0, BODY, IndentLevel::of(5))
        .expect("clear"));
    assert!(!pres
        .clear_shape_list_style_default(0, BODY)
        .expect("clear default"));
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for (name, original) in &before {
        assert_eq!(after.get(name), Some(original), "part {name} was disturbed");
    }
}

#[test]
fn clearing_the_whole_list_style_drops_every_level_it_stated() {
    let mut pres = levels();
    pres.set_shape_list_style_level(0, BODY, IndentLevel::TOP, &size(40.0))
        .expect("author a second level");

    assert!(pres.clear_shape_list_style(0, BODY).expect("clear"));
    assert!(
        pres.shape_list_style_level(0, BODY, IndentLevel::of(2))
            .expect("read")
            .is_none(),
        "the level the file shipped with went with it"
    );
    assert_eq!(
        pres.effective_run_properties(0, BODY, 2, 0)
            .expect("effective")
            .size_points(),
        Some(24.0),
        "level 2 falls all the way through to the master"
    );
    assert!(
        !pres.clear_shape_list_style(0, BODY).expect("clear again"),
        "a shape with no list style reports nothing to clear"
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn an_authored_list_style_survives_a_save_and_reopen() {
    let mut pres = levels();
    pres.set_shape_list_style_level(0, TEXT_BOX, IndentLevel::of(1), &size(13.5))
        .expect("author");
    let saved = pres.save().expect("save");

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened
            .shape_list_style_level(0, TEXT_BOX, IndentLevel::of(1))
            .expect("read")
            .expect("declared")
            .default_run_properties()
            .and_then(CharacterPropertiesSpec::size_points),
        Some(13.5)
    );
}

#[test]
fn the_authored_element_sits_where_the_schema_puts_it() {
    let mut pres = levels();
    pres.set_shape_list_style_level(0, TEXT_BOX, IndentLevel::TOP, &size(44.0))
        .expect("author");
    let saved = pres.save().expect("save");

    let bytes = byte_map(&Package::open(&saved).expect("reopen"));
    let slide = String::from_utf8(bytes["ppt/slides/slide1.xml"].clone()).expect("utf-8");
    assert!(
        slide.contains(
            r#"<a:bodyPr/><a:lstStyle><a:lvl1pPr><a:defRPr sz="4400"/></a:lvl1pPr></a:lstStyle><a:p>"#
        ),
        "`a:lstStyle` sits between `a:bodyPr` and the first `a:p`: {slide}"
    );
}

#[test]
fn authoring_a_list_style_dirties_only_that_slide() {
    let bytes = fixture("text_levels.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_shape_list_style_level(0, BODY, IndentLevel::of(3), &size(21.0))
        .expect("author");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_ne!(
        before.get("ppt/slides/slide1.xml"),
        after.get("ppt/slides/slide1.xml"),
        "the slide was the part edited"
    );
    for (name, original) in &before {
        if name == "ppt/slides/slide1.xml" {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "part {name} was disturbed");
    }
}

#[test]
fn a_shape_with_no_text_body_says_so_rather_than_authoring_one() {
    let mut pres = Presentation::open(&fixture("tables.pptx")).expect("open");
    // Shape 1 of `tables.pptx` is the table's graphic frame — it frames no text body at all.
    assert!(pres
        .set_shape_list_style_level(0, 1, IndentLevel::TOP, &size(12.0))
        .is_err());
}
