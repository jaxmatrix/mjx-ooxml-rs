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

// ---------------------------------------------------------------------------------------------
// N2 — the ergonomic notes surface: notes_text / set_notes_text / clear_notes
// ---------------------------------------------------------------------------------------------

/// The sorted part names of a package — the "which parts exist" set a fidelity delta is asserted on.
fn names(map: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    map.keys().cloned().collect()
}

#[test]
fn notes_text_reads_the_body_and_is_none_without_notes() {
    let mut pres = deck();
    assert_eq!(
        pres.notes_text(0).expect("notes"),
        Some("Remember to smile.".to_owned())
    );
    assert_eq!(pres.notes_text(1).expect("notes"), None, "slide 1 has no notes");
}

#[test]
fn set_notes_text_on_a_slide_with_notes_touches_only_that_part() {
    let bytes = fixture("notes.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_notes_text(0, "Pause for the demo.").expect("set");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    // No part added or removed — the notes slide already existed.
    assert_eq!(names(&snapshot), names(&reopened), "part set changed");
    const NOTES: &str = "ppt/notesSlides/notesSlide1.xml";
    assert_ne!(reopened.get(NOTES), snapshot.get(NOTES), "the notes slide changed");
    for (name, original) in &snapshot {
        if name != NOTES {
            assert_eq!(reopened.get(name), Some(original), "part {name} must be byte-identical");
        }
    }

    // The text reads back.
    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(reread.notes_text(0).expect("notes"), Some("Pause for the demo.".to_owned()));
}

#[test]
fn set_notes_text_creates_the_notes_slide_on_demand() {
    let bytes = fixture("notes.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Slide 1 has no notes; the deck already has a notes master to follow.
    pres.set_notes_text(1, "Speaker note for slide two.").expect("set");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    // Exactly the new notes slide and its .rels appear — the notes master is reused, not recreated.
    let added: Vec<_> = reopened.keys().filter(|k| !snapshot.contains_key(*k)).cloned().collect();
    assert_eq!(
        added,
        vec![
            "ppt/notesSlides/_rels/notesSlide2.xml.rels".to_owned(),
            "ppt/notesSlides/notesSlide2.xml".to_owned(),
        ]
    );
    assert!(reopened.keys().all(|k| snapshot.contains_key(k) || added.contains(k)), "nothing else added");

    // The slide's own .rels gained the notesSlide relationship; the content types gained an override.
    const SLIDE_RELS: &str = "ppt/slides/_rels/slide2.xml.rels";
    assert_ne!(reopened.get(SLIDE_RELS), snapshot.get(SLIDE_RELS), "slide rels changed");
    let ct = String::from_utf8(reopened["[Content_Types].xml"].clone()).unwrap();
    assert!(ct.contains("/ppt/notesSlides/notesSlide2.xml"), "content type override added");

    // The notes master part and presentation were not touched.
    const MASTER: &str = "ppt/notesMasters/notesMaster1.xml";
    assert_eq!(reopened.get(MASTER), snapshot.get(MASTER), "notes master untouched");
    assert_eq!(
        reopened.get("ppt/presentation.xml"),
        snapshot.get("ppt/presentation.xml"),
        "presentation untouched — the notes master already existed"
    );

    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(reread.notes_text(1).expect("notes"), Some("Speaker note for slide two.".to_owned()));
}

#[test]
fn set_notes_text_synthesizes_the_notes_master_when_the_deck_has_none() {
    // `sample.pptx` has a single slide and no notes master at all.
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_notes_text(0, "First speaker note.").expect("set");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    // The notes master and the notes slide are both born, each with its .rels.
    let added: Vec<_> = reopened.keys().filter(|k| !snapshot.contains_key(*k)).cloned().collect();
    assert_eq!(
        added,
        vec![
            "ppt/notesMasters/_rels/notesMaster1.xml.rels".to_owned(),
            "ppt/notesMasters/notesMaster1.xml".to_owned(),
            "ppt/notesSlides/_rels/notesSlide1.xml.rels".to_owned(),
            "ppt/notesSlides/notesSlide1.xml".to_owned(),
        ]
    );

    // The presentation gained a `p:notesMasterIdLst`, in schema order right after `p:sldMasterIdLst`.
    let pres_xml = String::from_utf8(reopened["ppt/presentation.xml"].clone()).unwrap();
    let masters = pres_xml.find("sldMasterIdLst").expect("sldMasterIdLst");
    let notes = pres_xml.find("notesMasterIdLst").expect("notesMasterIdLst");
    let slides = pres_xml.find("sldIdLst").expect("sldIdLst");
    assert!(masters < notes && notes < slides, "notesMasterIdLst must sit between the two");

    // Both content-type overrides are present; the presentation rels gained the notesMaster rel.
    let ct = String::from_utf8(reopened["[Content_Types].xml"].clone()).unwrap();
    assert!(ct.contains("notesMaster1.xml"), "notes master override");
    assert!(ct.contains("notesSlide1.xml"), "notes slide override");
    let pres_rels = String::from_utf8(reopened["ppt/_rels/presentation.xml.rels"].clone()).unwrap();
    assert!(pres_rels.contains("relationships/notesMaster"), "presentation notesMaster rel");

    // The synthesized deck still opens and reads back.
    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(reread.notes_text(0).expect("notes"), Some("First speaker note.".to_owned()));
    assert_eq!(reread.shape_count(Surface::NotesMaster).expect("count"), 1);
}

#[test]
fn clear_notes_removes_the_part_and_returns_the_part_set() {
    let bytes = fixture("notes.pptx");
    let baseline = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Create notes on slide 1, then clear them: the part set must return to the baseline.
    pres.set_notes_text(1, "Transient note.").expect("set");
    pres.clear_notes(1).expect("clear");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    assert_eq!(names(&baseline), names(&reopened), "the notes slide and its .rels are gone");
    assert!(!reopened.contains_key("ppt/notesSlides/notesSlide2.xml"));

    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(reread.notes_text(1).expect("notes"), None, "slide 1 has no notes again");
    // The slide's own rels no longer names a notes slide, and the parts never touched are unchanged.
    let slide_rels = String::from_utf8(reopened["ppt/slides/_rels/slide2.xml.rels"].clone()).unwrap();
    assert!(!slide_rels.contains("notesSlide"), "the notes relationship is gone");
    const NOTES1: &str = "ppt/notesSlides/notesSlide1.xml";
    assert_eq!(reopened.get(NOTES1), baseline.get(NOTES1), "slide 0's notes untouched");
}

#[test]
fn clear_notes_is_a_no_op_without_notes() {
    let bytes = fixture("notes.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.clear_notes(1).expect("clear"); // slide 1 has no notes
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(snapshot, reopened, "clearing absent notes must change nothing");
}