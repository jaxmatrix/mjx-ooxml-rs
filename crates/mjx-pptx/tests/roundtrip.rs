//! PR 2c integration tests: open a real `.pptx`, resolve slides, read shape text, edit a run, and
//! save so the edit round-trips while every other part stays byte-identical.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// A name → decompressed-bytes map of every entry that currently has materialized bytes.
fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

#[test]
fn resolves_single_slide_partname() {
    let pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(pres.slide_count(), 1);
    assert_eq!(
        pres.slide_part(0).expect("slide 0").as_str(),
        "/ppt/slides/slide1.xml"
    );
    assert_eq!(pres.presentation_part().as_str(), "/ppt/presentation.xml");
}

#[test]
fn enumerates_shapes_skipping_group_props() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    // spTree has 3 element children (nvGrpSpPr, grpSpPr, sp); only the one p:sp is a shape.
    assert_eq!(pres.shape_count(0).expect("shape count"), 1);
}

#[test]
fn reads_shape_text() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(pres.shape_text(0, 0).expect("shape text"), "Hello OOXML");
}

#[test]
fn reading_does_not_dirty_parts() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("open baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = pres.shape_text(0, 0).expect("read"); // read only
    let reopened = Package::open(&pres.save().expect("save")).expect("reopen");

    let reopened_map = byte_map(&reopened);
    for (name, original) in &snapshot {
        assert_eq!(
            reopened_map.get(name),
            Some(original),
            "reading dirtied part {name}"
        );
    }
}

#[test]
fn edit_round_trips_and_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("open baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(pres.shape_text(0, 0).expect("read"), "Hello OOXML"); // precondition

    pres.set_shape_text(0, 0, 0, "Goodbye OOXML").expect("edit");
    let saved = pres.save().expect("save");

    // The edit landed.
    let mut reread = Presentation::open(&saved).expect("reopen presentation");
    assert_eq!(reread.shape_text(0, 0).expect("reread"), "Goodbye OOXML");

    let reopened = Package::open(&saved).expect("reopen package");
    let reopened_map = byte_map(&reopened);

    // Same set of parts.
    let before: Vec<&String> = snapshot.keys().collect();
    let after: Vec<&String> = reopened_map.keys().collect();
    assert_eq!(before, after, "part set changed");

    // The slide changed; every other part is byte-identical.
    const SLIDE: &str = "ppt/slides/slide1.xml";
    assert_ne!(
        reopened_map.get(SLIDE),
        snapshot.get(SLIDE),
        "the edited slide should differ"
    );
    for (name, original) in &snapshot {
        if name == SLIDE {
            continue;
        }
        assert_eq!(
            reopened_map.get(name),
            Some(original),
            "part {name} must be byte-identical"
        );
    }
}

/// The decompressed bytes of `ppt/slides/slide1.xml` from a saved container, as a UTF-8 string.
fn saved_slide1(saved: &[u8]) -> String {
    let pkg = Package::open(saved).expect("reopen package");
    let bytes = byte_map(&pkg)
        .remove("ppt/slides/slide1.xml")
        .expect("slide1 present");
    String::from_utf8(bytes).expect("slide is utf-8")
}

const CANARY_BOUNDS: ShapeBounds = ShapeBounds {
    offset_x_emu: 914_400,
    offset_y_emu: 914_400,
    width_emu: 3_657_600,
    height_emu: 1_828_800,
};

#[test]
fn add_text_box_appends_shape() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(pres.shape_count(0).expect("count before"), 1); // precondition
    let new_idx = pres
        .add_text_box(0, "Canary\nLine two", CANARY_BOUNDS)
        .expect("add");
    assert_eq!(new_idx, 1, "new shape is appended after the existing one");
    assert_eq!(pres.shape_count(0).expect("count after"), 2);
}

#[test]
fn added_shape_reads_back_and_is_editable() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let new_idx = pres
        .add_text_box(0, "Canary\nLine two", CANARY_BOUNDS)
        .expect("add");

    // Reads back through the ordinary read path (i.e. the built subtree parses like a real one).
    let saved = pres.save().expect("save");
    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reread.shape_text(0, new_idx).expect("read new shape"),
        "Canary\nLine two"
    );

    // The added shape's first run is editable like any other.
    reread
        .set_shape_text(0, new_idx, 0, "Replaced")
        .expect("edit new shape");
    let saved2 = reread.save().expect("save again");
    let mut reread2 = Presentation::open(&saved2).expect("reopen again");
    assert_eq!(
        reread2.shape_text(0, new_idx).expect("reread new shape"),
        // First run only was replaced; the second paragraph's run is untouched.
        "Replaced\nLine two"
    );
}

#[test]
fn added_shape_gets_next_unique_id() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_text_box(0, "x", CANARY_BOUNDS).expect("add");
    let slide = saved_slide1(&pres.save().expect("save"));
    // Fixture ids are 1 (group) and 2 (title); the new box must be 3, exactly once.
    assert_eq!(
        slide.matches(r#"id="3""#).count(),
        1,
        "new id 3 appears once"
    );
    assert!(
        slide.contains(r#"name="TextBox 3""#),
        "new shape is named for its id: {slide}"
    );
    // The rectangle geometry and text-box flag are present.
    assert!(slide.contains(r#"<a:prstGeom prst="rect">"#), "{slide}");
    assert!(slide.contains(r#"txBox="1""#), "{slide}");
}

#[test]
fn add_text_box_leaves_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("open baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.add_text_box(0, "Canary", CANARY_BOUNDS).expect("add");
    let saved = pres.save().expect("save");

    let reopened_map = byte_map(&Package::open(&saved).expect("reopen"));
    // Construction adds no parts and removes none.
    assert_eq!(
        snapshot.keys().collect::<Vec<_>>(),
        reopened_map.keys().collect::<Vec<_>>(),
        "part set changed"
    );
    const SLIDE: &str = "ppt/slides/slide1.xml";
    assert_ne!(
        reopened_map.get(SLIDE),
        snapshot.get(SLIDE),
        "the edited slide should differ"
    );
    for (name, original) in &snapshot {
        if name == SLIDE {
            continue;
        }
        assert_eq!(
            reopened_map.get(name),
            Some(original),
            "part {name} must be byte-identical"
        );
    }
}

#[test]
fn add_text_box_escapes_markup() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_text_box(0, "a<b&c", CANARY_BOUNDS).expect("add");
    let saved = pres.save().expect("save");

    // Round-trips to the original characters...
    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(reread.shape_text(0, 1).expect("read"), "a<b&c");
    // ...and is stored escaped on the wire (not raw `<`/`&`).
    let slide = saved_slide1(&saved);
    assert!(
        slide.contains("a&lt;b&amp;c"),
        "text must be escaped: {slide}"
    );
}

#[test]
fn add_text_box_slide_out_of_range() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let err = pres.add_text_box(9, "x", CANARY_BOUNDS).unwrap_err();
    assert!(
        matches!(err, PptxError::SlideIndexOutOfRange { .. }),
        "{err:?}"
    );
}

#[test]
fn slide_index_out_of_range() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let err = pres.shape_text(9, 0).unwrap_err();
    assert!(
        matches!(err, PptxError::SlideIndexOutOfRange { .. }),
        "{err:?}"
    );
}

#[test]
fn shape_index_out_of_range() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let err = pres.shape_text(0, 5).unwrap_err();
    assert!(
        matches!(err, PptxError::ShapeIndexOutOfRange { .. }),
        "{err:?}"
    );
}

#[test]
fn run_index_out_of_range() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let err = pres.set_shape_text(0, 0, 9, "x").unwrap_err();
    assert!(
        matches!(err, PptxError::RunIndexOutOfRange { .. }),
        "{err:?}"
    );
}
