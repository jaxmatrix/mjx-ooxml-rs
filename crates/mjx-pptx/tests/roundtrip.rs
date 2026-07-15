//! PR 2c integration tests: open a real `.pptx`, resolve slides, read shape text, edit a run, and
//! save so the edit round-trips while every other part stays byte-identical.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation};

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
