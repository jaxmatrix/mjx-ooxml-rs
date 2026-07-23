//! Integration tests for the text-formatting surface: reading and writing run and paragraph
//! properties on a real deck, at every scope a user can select.
//!
//! The scopes are the point. A run boundary exists *because* formatting changes there, so formatting
//! one run, a paragraph, a whole shape and an arbitrary character range are four different edits —
//! and the last of them has to split runs, which is where the interesting failures live.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    CharacterPropertiesSpec, ColorSpec, IndentLevel, ParagraphPropertiesSpec, SchemeColor,
    TextAlignment,
};
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeBounds};

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

const BOUNDS: ShapeBounds = ShapeBounds {
    offset_x_emu: 914_400,
    offset_y_emu: 914_400,
    width_emu: 3_657_600,
    height_emu: 1_828_800,
};

/// `sample.pptx` with a three-line text box added as shape 1.
fn deck_with_lines() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_text_box(0, "first line\nsecond line\nthird line", BOUNDS)
        .expect("add text box");
    (pres, shape)
}

// ---------------------------------------------------------------------------------------------
// The paragraph axis
// ---------------------------------------------------------------------------------------------

#[test]
fn paragraphs_and_runs_are_addressable() {
    let (mut pres, shape) = deck_with_lines();
    assert_eq!(pres.paragraph_count(0, shape).expect("paragraphs"), 3);
    for para in 0..3 {
        assert_eq!(
            pres.run_count(0, shape, para).expect("runs"),
            1,
            "each line is one run"
        );
    }
    assert_eq!(
        pres.paragraph_text(0, shape, 1).expect("text"),
        "second line"
    );
    assert_eq!(pres.run_text(0, shape, 2, 0).expect("text"), "third line");
}

#[test]
fn reading_properties_does_not_dirty_the_part() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = pres.paragraph_count(0, 0).expect("count");
    let _ = pres.paragraph_text(0, 0, 0).expect("text");
    let _ = pres.paragraph_properties(0, 0, 0).expect("paragraph props");
    let _ = pres.run_properties(0, 0, 0, 0).expect("run props");
    let _ = pres.end_run_properties(0, 0, 0).expect("end props");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    for (name, original) in &snapshot {
        assert_eq!(reopened.get(name), Some(original), "reading dirtied {name}");
    }
}

#[test]
fn an_out_of_range_index_is_rejected_by_kind() {
    let (mut pres, shape) = deck_with_lines();
    let err = pres
        .paragraph_text(0, shape, 9)
        .expect_err("no paragraph 9");
    assert!(
        matches!(
            err,
            PptxError::ParagraphIndexOutOfRange { index: 9, count: 3 }
        ),
        "{err:?}"
    );
    let err = pres.run_text(0, shape, 0, 4).expect_err("no run 4");
    assert!(
        matches!(err, PptxError::RunIndexOutOfRange { index: 4, count: 1 }),
        "{err:?}"
    );
    let err = pres.paragraph_count(0, 99).expect_err("no shape 99");
    assert!(
        matches!(err, PptxError::ShapeIndexOutOfRange { .. }),
        "{err:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// The four scopes
// ---------------------------------------------------------------------------------------------

#[test]
fn one_run_can_be_formatted_alone() {
    let (mut pres, shape) = deck_with_lines();
    pres.set_run_properties(
        0,
        shape,
        1,
        0,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the second line");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened
            .run_properties(0, shape, 1, 0)
            .expect("props")
            .and_then(|p| p.is_bold()),
        Some(true)
    );
    for untouched in [0, 2] {
        assert_eq!(
            reopened
                .run_properties(0, shape, untouched, 0)
                .expect("props")
                .and_then(|p| p.is_bold()),
            None,
            "paragraph {untouched} should be untouched"
        );
    }
}

#[test]
fn a_whole_paragraph_can_be_formatted_at_once() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_text_box(0, "one two three", BOUNDS)
        .expect("add text box");
    // Split into three runs first, so "every run" means more than one.
    pres.set_text_range_properties(
        0,
        shape,
        0,
        4..7,
        &CharacterPropertiesSpec::new().with_italic(true),
    )
    .expect("italicize the middle word");
    assert_eq!(pres.run_count(0, shape, 0).expect("runs"), 3);

    pres.set_paragraph_run_properties(
        0,
        shape,
        0,
        &CharacterPropertiesSpec::new().with_size_points(20.0),
    )
    .expect("resize the paragraph");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    for run in 0..3 {
        assert_eq!(
            reopened
                .run_properties(0, shape, 0, run)
                .expect("props")
                .and_then(|p| p.size_points()),
            Some(20.0),
            "run {run} should have been resized"
        );
    }
    // The earlier italic survives — a paragraph-wide size does not erase what it did not name.
    assert_eq!(
        reopened
            .run_properties(0, shape, 0, 1)
            .expect("props")
            .and_then(|p| p.is_italic()),
        Some(true)
    );
}

#[test]
fn a_whole_shape_can_be_formatted_at_once() {
    let (mut pres, shape) = deck_with_lines();
    pres.set_shape_run_properties(
        0,
        shape,
        &CharacterPropertiesSpec::new().with_color(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("recolor the whole shape");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    for para in 0..3 {
        assert!(
            reopened
                .run_properties(0, shape, para, 0)
                .expect("props")
                .and_then(|p| p.fill().cloned())
                .is_some(),
            "paragraph {para} should have been recolored"
        );
    }
}

#[test]
fn formatting_a_paragraph_also_formats_what_gets_typed_next() {
    // A paragraph's `a:endParaRPr` is what text typed at the end takes on, so selecting a paragraph
    // and restyling it must reach the paragraph mark too.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_text_box(0, "text", BOUNDS).expect("add");
    pres.set_end_run_properties(
        0,
        shape,
        0,
        &CharacterPropertiesSpec::new().with_size_points(12.0),
    )
    .expect("give the paragraph a mark to find");

    pres.set_paragraph_run_properties(0, shape, 0, &CharacterPropertiesSpec::new().with_bold(true))
        .expect("bold the paragraph");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    let end = reopened
        .end_run_properties(0, shape, 0)
        .expect("end props")
        .expect("the paragraph mark exists");
    assert_eq!(end.is_bold(), Some(true), "the mark was not restyled");
    assert_eq!(end.size_points(), Some(12.0), "and kept what it had");
}

#[test]
fn an_empty_paragraph_can_be_formatted_before_anything_is_typed() {
    // The shape `add_slide_from_layout` hands back: placeholders holding one empty run.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_end_run_properties(
        slide,
        0,
        0,
        &CharacterPropertiesSpec::new().with_size_points(24.0),
    )
    .expect("format the empty placeholder");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened
            .end_run_properties(slide, 0, 0)
            .expect("end props")
            .and_then(|p| p.size_points()),
        Some(24.0)
    );
}

#[test]
fn paragraph_layout_is_set_separately_from_character_formatting() {
    let (mut pres, shape) = deck_with_lines();
    pres.set_paragraph_properties(
        0,
        shape,
        1,
        &ParagraphPropertiesSpec::new()
            .with_alignment(TextAlignment::Center)
            .with_level(IndentLevel::of(2))
            .with_left_margin_points(36.0),
    )
    .expect("lay out the paragraph");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    let props = reopened
        .paragraph_properties(0, shape, 1)
        .expect("props")
        .expect("the paragraph declares properties");
    assert_eq!(props.alignment(), Some(TextAlignment::Center));
    assert_eq!(props.level(), Some(IndentLevel::of(2)));
    assert_eq!(props.left_margin_points(), Some(36.0));
}

// ---------------------------------------------------------------------------------------------
// Ranges — the scope that has to split runs
// ---------------------------------------------------------------------------------------------

#[test]
fn formatting_part_of_a_run_splits_it() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_text_box(0, "Hello world", BOUNDS).expect("add");
    assert_eq!(pres.run_count(0, shape, 0).expect("runs"), 1);

    pres.set_text_range_properties(
        0,
        shape,
        0,
        6..11,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the second word");

    // The run split in two at the boundary; nothing follows, so there is no third run.
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.run_count(0, shape, 0).expect("runs"), 2);
    assert_eq!(reopened.run_text(0, shape, 0, 0).expect("text"), "Hello ");
    assert_eq!(reopened.run_text(0, shape, 0, 1).expect("text"), "world");
    assert_eq!(
        reopened
            .run_properties(0, shape, 0, 0)
            .expect("props")
            .and_then(|p| p.is_bold()),
        None,
        "the head must not be bold"
    );
    assert_eq!(
        reopened
            .run_properties(0, shape, 0, 1)
            .expect("props")
            .and_then(|p| p.is_bold()),
        Some(true)
    );
    // The text as a whole is unchanged — splitting divides, it does not rewrite.
    assert_eq!(
        reopened.paragraph_text(0, shape, 0).expect("text"),
        "Hello world"
    );
}

#[test]
fn a_range_in_the_middle_produces_three_runs() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_text_box(0, "one two three", BOUNDS).expect("add");
    pres.set_text_range_properties(
        0,
        shape,
        0,
        4..7,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the middle word");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    let texts: Vec<String> = (0..3)
        .map(|run| reopened.run_text(0, shape, 0, run).expect("text"))
        .collect();
    assert_eq!(texts, vec!["one ", "two", " three"]);
    let bold: Vec<Option<bool>> = (0..3)
        .map(|run| {
            reopened
                .run_properties(0, shape, 0, run)
                .expect("props")
                .and_then(|p| p.is_bold())
        })
        .collect();
    assert_eq!(bold, vec![None, Some(true), None]);
}

#[test]
fn a_range_already_on_run_boundaries_splits_nothing() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_text_box(0, "Hello world", BOUNDS).expect("add");
    pres.set_text_range_properties(
        0,
        shape,
        0,
        6..11,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("first edit splits");
    assert_eq!(pres.run_count(0, shape, 0).expect("runs"), 2);

    // The same range again now lines up with a run boundary, so it must not split further.
    pres.set_text_range_properties(
        0,
        shape,
        0,
        6..11,
        &CharacterPropertiesSpec::new().with_italic(true),
    )
    .expect("second edit");
    assert_eq!(
        pres.run_count(0, shape, 0).expect("runs"),
        2,
        "repeated edits must not accumulate runs"
    );
    let props = pres
        .run_properties(0, shape, 0, 1)
        .expect("props")
        .expect("the run has properties");
    assert_eq!(props.is_bold(), Some(true));
    assert_eq!(props.is_italic(), Some(true));
}

#[test]
fn a_range_that_runs_past_the_text_is_rejected() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_text_box(0, "short", BOUNDS).expect("add");
    let err = pres
        .set_text_range_properties(0, shape, 0, 2..99, &CharacterPropertiesSpec::new())
        .expect_err("out of bounds");
    assert!(
        matches!(
            err,
            PptxError::TextRangeOutOfBounds {
                start: 2,
                end: 99,
                length: 5
            }
        ),
        "{err:?}"
    );
    assert_eq!(
        pres.run_count(0, shape, 0).expect("runs"),
        1,
        "a rejected range must not have split anything"
    );
}

#[test]
fn graphemes_and_scalars_count_differently() {
    // "café 👍🏽" — 7 scalars (the emoji is two), 6 grapheme clusters.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_text_box(0, "café 👍🏽", BOUNDS).expect("add");
    let text = pres.paragraph_text(0, shape, 0).expect("text");
    assert_eq!(text.chars().count(), 7);

    // By grapheme, 5..6 is the whole emoji.
    pres.set_text_range_properties_by_grapheme(
        0,
        shape,
        0,
        5..6,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the emoji");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    let runs: Vec<String> = (0..reopened.run_count(0, shape, 0).expect("runs"))
        .map(|run| reopened.run_text(0, shape, 0, run).expect("text"))
        .collect();
    assert_eq!(
        runs,
        vec!["café ", "👍🏽"],
        "the emoji must stay whole: {runs:?}"
    );

    // By scalar, the same 5..6 would cover only the emoji's base and split the cluster.
    let mut scalar_pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let scalar_shape = scalar_pres.add_text_box(0, "café 👍🏽", BOUNDS).expect("add");
    scalar_pres
        .set_text_range_properties(
            0,
            scalar_shape,
            0,
            5..6,
            &CharacterPropertiesSpec::new().with_bold(true),
        )
        .expect("scalar range");
    let scalar_runs: Vec<String> = (0..scalar_pres.run_count(0, scalar_shape, 0).expect("runs"))
        .map(|run| scalar_pres.run_text(0, scalar_shape, 0, run).expect("text"))
        .collect();
    assert_eq!(
        scalar_runs.len(),
        3,
        "a scalar range splits the cluster: {scalar_runs:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn formatting_text_leaves_every_other_part_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_run_properties(0, 0, 0, 0, &CharacterPropertiesSpec::new().with_bold(true))
        .expect("bold the title");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        snapshot.keys().collect::<Vec<_>>(),
        reopened.keys().collect::<Vec<_>>(),
        "formatting text adds and removes no parts"
    );
    const SLIDE: &str = "ppt/slides/slide1.xml";
    assert_ne!(
        reopened.get(SLIDE),
        snapshot.get(SLIDE),
        "the slide changed"
    );
    for (name, original) in &snapshot {
        if name == SLIDE {
            continue;
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "part {name} must be byte-identical"
        );
    }
}

#[test]
fn a_layouts_text_can_be_formatted_like_a_slides() {
    // Text formatting is Surface-addressed like every other shape call, so editing a layout reaches
    // every slide built on it.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.set_paragraph_properties(
        mjx_pptx::Surface::Layout(1),
        0,
        0,
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Right),
    )
    .expect("lay out the layout's title");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened
            .paragraph_properties(mjx_pptx::Surface::Layout(1), 0, 0)
            .expect("props")
            .and_then(|p| p.alignment()),
        Some(TextAlignment::Right)
    );
}

// ---------------------------------------------------------------------------------------------
// Replacing a shape's text wholesale
// ---------------------------------------------------------------------------------------------

#[test]
fn setting_a_shapes_text_content_round_trips_line_for_line() {
    // One paragraph per line, one run per paragraph — so what `shape_text` reads back is exactly
    // what was written, and every line is addressable as run 0 of its paragraph.
    let (mut pres, shape) = deck_with_lines();
    pres.set_shape_text_content(0, shape, "alpha\nbeta")
        .expect("set text content");

    assert_eq!(pres.shape_text(0, shape).expect("text"), "alpha\nbeta");
    assert_eq!(pres.paragraph_count(0, shape).expect("paragraphs"), 2);
    assert_eq!(pres.run_count(0, shape, 1).expect("runs"), 1);
    assert_eq!(pres.run_text(0, shape, 1, 0).expect("run"), "beta");
}

#[test]
fn replacing_the_text_keeps_the_bodys_own_layout() {
    // Only the paragraphs are swapped: `a:bodyPr` (and `a:lstStyle`) survive, so restating a
    // placeholder's text does not disturb how that placeholder is laid out.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let before = String::from_utf8(
        Package::open(&fixture("sample.pptx"))
            .expect("package")
            .entries()
            .iter()
            .find(|e| e.name == "ppt/slides/slide1.xml")
            .and_then(|e| e.bytes())
            .expect("slide bytes")
            .to_vec(),
    )
    .expect("utf-8");
    let body_pr_before = before.matches("<a:bodyPr").count();

    pres.set_shape_text_content(0, 0, "replaced").expect("set");
    let saved = pres.save().expect("save");
    let after = String::from_utf8(
        Package::open(&saved)
            .expect("package")
            .entries()
            .iter()
            .find(|e| e.name == "ppt/slides/slide1.xml")
            .and_then(|e| e.bytes())
            .expect("slide bytes")
            .to_vec(),
    )
    .expect("utf-8");

    assert_eq!(
        after.matches("<a:bodyPr").count(),
        body_pr_before,
        "no body properties element is added or dropped"
    );
    assert!(after.contains("replaced"));
}

#[test]
fn only_a_shape_can_be_given_a_text_body() {
    // A picture, a group, a graphic frame and a connector have no `p:txBody` in their schema, so
    // there is nothing to replace and nothing may be created.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let group = mjx_pptx::Surface::Slide(1);
    assert!(matches!(
        pres.set_shape_text_content(group, 2, "nope"),
        Err(PptxError::ShapeHasNoTextBody)
    ));
}

#[test]
fn replacing_the_text_dirties_only_that_slide() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let original = byte_map(&Package::open(&fixture("sample.pptx")).expect("package"));
    pres.set_shape_text_content(0, 0, "one edit").expect("set");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, bytes) in &original {
        if name == "ppt/slides/slide1.xml" {
            continue;
        }
        assert_eq!(
            reopened.get(name),
            Some(bytes),
            "part {name} must be byte-identical"
        );
    }
}
