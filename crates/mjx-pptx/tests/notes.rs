//! Speaker notes, read over `tests/fixtures/notes.pptx`.
//!
//! The whole point of the workstream: a notes slide carries the same `p:cSld > p:spTree` a slide
//! does, so once it is addressable as a [`Surface`] every shape/text/effective-property method works
//! on it unchanged. These tests prove that for `Surface::Notes` and `Surface::NotesMaster`.
//!
//! Fixture shape: slide 0 owns `notesSlide1.xml` (a `sldImg` placeholder at index 0 and a `body`
//! placeholder holding "Remember to smile." at index 1) which follows `notesMaster1.xml` (its own
//! `body` placeholder plus a `p:notesStyle` whose `a:lvl1pPr` sets 12pt). Slide 1 owns **no** notes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeKind, Surface};

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

fn deck() -> Presentation {
    Presentation::open(&fixture("notes.pptx")).expect("open")
}

/// The body placeholder on the notes slide is the second shape (the `sldImg` is the first).
const NOTES_BODY: usize = 1;

#[test]
fn a_notes_slide_exposes_the_same_shape_surface() {
    let mut pres = deck();

    assert_eq!(
        pres.shape_count(Surface::Notes(0)).expect("count"),
        2,
        "the slide-image and the body placeholder"
    );
    assert_eq!(
        pres.shape_kind(Surface::Notes(0), NOTES_BODY).expect("kind"),
        ShapeKind::Shape
    );

    let img = pres
        .shape_placeholder(Surface::Notes(0), 0)
        .expect("placeholder")
        .expect("the slide image is a placeholder");
    assert_eq!(img.kind, PlaceholderType::SlideImage);

    let body = pres
        .shape_placeholder(Surface::Notes(0), NOTES_BODY)
        .expect("placeholder")
        .expect("the notes body is a placeholder");
    assert_eq!(body.kind, PlaceholderType::Body);
}

#[test]
fn notes_body_text_reads_through_the_shape_surface() {
    let mut pres = deck();
    assert_eq!(
        pres.shape_text(Surface::Notes(0), NOTES_BODY).expect("text"),
        "Remember to smile."
    );
}

#[test]
fn notes_body_inherits_its_size_from_the_notes_master() {
    let mut pres = deck();
    // The run's own `a:rPr` declares no size; 12pt lives only in the notes master's
    // `p:notesStyle > a:lvl1pPr`. Resolving it proves the notes surface inherits from the notes
    // master exactly as a slide inherits from its slide master's `p:txStyles`.
    let effective = pres
        .effective_run_properties(Surface::Notes(0), NOTES_BODY, 0, 0)
        .expect("effective run");
    assert_eq!(effective.size_points(), Some(12.0));
}

#[test]
fn the_notes_master_is_addressable_on_its_own() {
    let mut pres = deck();
    assert_eq!(
        pres.shape_count(Surface::NotesMaster).expect("count"),
        1,
        "the notes master's own body placeholder"
    );
    let body = pres
        .shape_placeholder(Surface::NotesMaster, 0)
        .expect("placeholder")
        .expect("the master body is a placeholder");
    assert_eq!(body.kind, PlaceholderType::Body);
}

#[test]
fn theme_and_colour_map_resolve_through_the_notes_master() {
    let mut pres = deck();
    // The notes master carries the `p:clrMap` and the `theme` relationship, so both resolve for a
    // notes surface with no extra wiring.
    assert!(pres.color_map(Surface::Notes(0)).expect("color map").is_some());
    assert!(pres.theme(Surface::Notes(0)).expect("theme").is_some());
    assert!(pres.theme(Surface::NotesMaster).expect("theme").is_some());
}

#[test]
fn a_slide_without_notes_reports_no_notes_surface() {
    let mut pres = deck();
    // Slide 1 owns no notes slide part: addressing it as a notes surface is a typed error, not a
    // panic or an empty read.
    let err = pres.shape_count(Surface::Notes(1)).expect_err("no notes");
    assert!(matches!(err, PptxError::SurfaceHasNoNotes { slide: 1 }));
}

#[test]
fn reading_notes_leaves_every_part_byte_identical() {
    let bytes = fixture("notes.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Exercise the whole notes read surface, then save.
    pres.shape_count(Surface::Notes(0)).expect("count");
    pres.shape_text(Surface::Notes(0), NOTES_BODY).expect("text");
    pres.effective_run_properties(Surface::Notes(0), NOTES_BODY, 0, 0)
        .expect("effective");
    pres.color_map(Surface::Notes(0)).expect("color map");
    pres.shape_count(Surface::NotesMaster).expect("count");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(snapshot, reopened, "reading notes must not change any part");
}
