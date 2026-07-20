//! Integration tests for removal: taking a shape back off a slide, a layout or a master.
//!
//! Removal is the half of the API that construction has always been missing. What is tested here is
//! that it closes the gap in the one shape index space, that the part still parses afterwards, and
//! that it touches nothing else.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeBounds, Surface};

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

/// A slide carrying the fixture's title plus three labelled text boxes.
fn deck_with_three_boxes() -> Presentation {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    for label in ["one", "two", "three"] {
        pres.add_text_box(0, label, BOUNDS).expect("add");
    }
    pres
}

#[test]
fn removing_a_shape_closes_the_gap_in_the_index_space() {
    let mut pres = deck_with_three_boxes();
    assert_eq!(pres.shape_count(0).expect("count"), 4); // the fixture title + three boxes

    pres.remove_shape(0, 1).expect("remove the first box");

    assert_eq!(pres.shape_count(0).expect("count"), 3);
    assert_eq!(pres.shape_text(0, 0).expect("text"), "Hello OOXML");
    assert_eq!(
        pres.shape_text(0, 1).expect("text"),
        "two",
        "the shapes after the removed one move down one index"
    );
    assert_eq!(pres.shape_text(0, 2).expect("text"), "three");
}

#[test]
fn the_slide_still_parses_after_a_removal() {
    let mut pres = deck_with_three_boxes();
    pres.remove_shape(0, 2).expect("remove the middle box");

    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.shape_count(0).expect("count"), 3);
    let texts: Vec<String> = (0..3)
        .map(|idx| reopened.shape_text(0, idx).expect("text"))
        .collect();
    assert_eq!(texts, vec!["Hello OOXML", "one", "three"]);
}

#[test]
fn every_shape_can_be_removed_leaving_an_empty_shape_tree() {
    let mut pres = deck_with_three_boxes();
    for _ in 0..4 {
        pres.remove_shape(0, 0).expect("remove");
    }
    assert_eq!(pres.shape_count(0).expect("count"), 0);

    // An empty shape tree still round-trips through the reader.
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.shape_count(0).expect("count"), 0);
}

#[test]
fn a_picture_is_removed_like_any_other_shape_but_keeps_its_image() {
    const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, b'I', b'H', b'D', b'R', 0,
        0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0,
    ];
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres.add_picture(0, PNG, BOUNDS).expect("add picture");
    pres.remove_shape(0, picture).expect("remove the picture");

    let saved = pres.save().expect("save");
    let pkg = Package::open(&saved).expect("reopen");
    assert!(
        pkg.part_names()
            .any(|p| p.as_str().starts_with("/ppt/media/")),
        "the image part stays: removing a shape is not a package garbage collection"
    );
    let mut reopened = Presentation::open(&saved).expect("reopen presentation");
    assert_eq!(reopened.shape_count(0).expect("count"), 1);
}

#[test]
fn a_layouts_shape_can_be_removed_too() {
    // Removal is Surface-addressed like every other shape call: this drops the layout's footer slot,
    // and with it the footer every slide on that layout was inheriting.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    assert_eq!(pres.shape_count(Surface::Layout(1)).expect("count"), 5);

    pres.remove_shape(Surface::Layout(1), 3).expect("remove");

    assert_eq!(pres.shape_count(Surface::Layout(1)).expect("count"), 4);
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened.shape_count(Surface::Layout(1)).expect("count"),
        4,
        "the layout part still parses"
    );
}

#[test]
fn an_out_of_range_shape_is_rejected_and_names_its_surface() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let err = pres
        .remove_shape(Surface::Layout(1), 9)
        .expect_err("no such shape");
    match err {
        PptxError::ShapeIndexOutOfRange {
            surface,
            index: 9,
            count: 5,
        } => assert_eq!(surface.to_string(), "layout 1"),
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(
        pres.shape_count(Surface::Layout(1)).expect("count"),
        5,
        "a rejected removal changes nothing"
    );
}

#[test]
fn removing_a_shape_leaves_every_other_part_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.remove_shape(0, 0).expect("remove the title");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        snapshot.keys().collect::<Vec<_>>(),
        reopened.keys().collect::<Vec<_>>(),
        "removing a shape adds and removes no parts"
    );
    const SLIDE: &str = "ppt/slides/slide1.xml";
    assert_ne!(
        reopened.get(SLIDE),
        snapshot.get(SLIDE),
        "the edited slide should differ"
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
