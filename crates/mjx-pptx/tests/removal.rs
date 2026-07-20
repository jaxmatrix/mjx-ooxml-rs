//! Integration tests for removal: taking a shape back off a slide, a layout or a master, and taking
//! a slide back out of a deck.
//!
//! Removal is the half of the API that construction has always been missing. What is tested here is
//! that it closes the gap in the one shape index space, that the part still parses afterwards, and
//! that it touches nothing else — and, for a slide, that the whole
//! `p:sldIdLst` → relationship → part chain is unwired consistently.

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

// ---------------------------------------------------------------------------------------------
// Slide removal. `layouts.pptx` has two slides on different layouts, so a removal can be seen to
// take the right one.
// ---------------------------------------------------------------------------------------------

#[test]
fn removing_a_slide_takes_the_right_one_and_shifts_the_rest() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    assert_eq!(pres.slide_count(), 2);
    let survivor_text = pres.shape_text(1, 0).expect("text of slide 1");
    let survivor_layout = pres.slide_layout(1).expect("layout of slide 1");

    pres.remove_slide(0).expect("remove slide 0");

    assert_eq!(pres.slide_count(), 1);
    assert_eq!(
        pres.shape_text(0, 0).expect("text"),
        survivor_text,
        "the surviving slide moved down to index 0, unchanged"
    );
    assert_eq!(pres.slide_layout(0).expect("layout"), survivor_layout);
}

#[test]
fn a_deck_with_a_slide_removed_reopens_consistently() {
    // The real proof: the p:sldIdLst, the relationships and the part set we rewrote must agree well
    // enough for a fresh `open` to resolve the deck.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let removed_part = pres.slide_part(0).expect("slide 0").clone();
    let survivor_text = pres.shape_text(1, 0).expect("text");

    pres.remove_slide(0).expect("remove");
    let saved = pres.save().expect("save");

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.slide_count(), 1);
    assert_eq!(reopened.shape_text(0, 0).expect("text"), survivor_text);

    let pkg = Package::open(&saved).expect("reopen package");
    assert!(
        !pkg.part_names().any(|p| p == removed_part),
        "the slide part survived"
    );
    assert!(
        !pkg.entries()
            .iter()
            .any(|e| e.name == "ppt/slides/_rels/slide1.xml.rels"),
        "the slide's own .rels survived"
    );
}

#[test]
fn removing_a_slide_takes_the_images_only_it_showed() {
    const PNG: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, b'I', b'H', b'D', b'R', 0,
        0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0,
    ];
    const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";

    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    // The shared image is on both slides; the exclusive one only on slide 0.
    pres.add_picture(0, PNG, BOUNDS).expect("shared on slide 0");
    pres.add_picture(1, PNG, BOUNDS).expect("shared on slide 1");
    pres.add_picture(0, GIF, BOUNDS).expect("exclusive");

    pres.remove_slide(0).expect("remove");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");

    let media: Vec<String> = pkg
        .part_names()
        .filter(|p| p.as_str().starts_with("/ppt/media/"))
        .map(|p| p.as_str().to_owned())
        .collect();
    assert!(
        media.iter().any(|m| m.ends_with(".png")),
        "the image the surviving slide still shows was deleted: {media:?}"
    );
    assert!(
        !media.iter().any(|m| m.ends_with(".gif")),
        "the image only the removed slide showed survived: {media:?}"
    );
}

#[test]
fn a_removed_slides_part_name_is_not_recycled() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.remove_slide(0).expect("remove slide1.xml");
    let added = pres.add_slide_from_layout(2).expect("add");

    assert_eq!(
        pres.slide_part(added).expect("new slide").as_str(),
        "/ppt/slides/slide3.xml",
        "a new slide is numbered past every part, never into a freed name"
    );
    let reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reopened.slide_count(), 2);
}

#[test]
fn an_out_of_range_slide_is_rejected() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let err = pres.remove_slide(9).expect_err("no such slide");
    assert!(
        matches!(err, PptxError::SlideIndexOutOfRange { index: 9, count: 2 }),
        "{err:?}"
    );
    assert_eq!(pres.slide_count(), 2, "a rejected removal changes nothing");
}

#[test]
fn removing_a_slide_touches_only_the_deck_wiring() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.remove_slide(0).expect("remove");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // Exactly the deck wiring changes: the slide list, the relationship to the slide, and the
    // content-type override. The slide and its .rels are gone; nothing else moves.
    let rewritten = [
        "ppt/presentation.xml",
        "ppt/_rels/presentation.xml.rels",
        "[Content_Types].xml",
    ];
    let deleted = ["ppt/slides/slide1.xml", "ppt/slides/_rels/slide1.xml.rels"];
    for (name, original) in &snapshot {
        if rewritten.contains(&name.as_str()) {
            continue;
        }
        if deleted.contains(&name.as_str()) {
            assert!(!reopened.contains_key(name), "part {name} should be gone");
            continue;
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "part {name} must be byte-identical"
        );
    }
    assert!(
        reopened.keys().all(|name| snapshot.contains_key(name)),
        "removing a slide added a part"
    );
}
