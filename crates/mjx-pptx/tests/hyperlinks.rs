//! Hyperlinks on runs and shapes, over `tests/fixtures/hyperlinks.pptx`.
//!
//! A hyperlink names a relationship by `r:id` — external for a URL, internal for a slide jump — so a
//! caller reads and writes a resolved [`Hyperlink`] while the relationship wiring stays inside the
//! deck. These tests read the two links the fixture ships, then set / replace / clear links on runs,
//! ranges and shapes, asserting the exact relationship and part deltas each time.
//!
//! Fixture shape: slide 0 has a text box (shape 0) whose one run carries an **external URL**
//! `hlinkClick` (→ `https://example.com/`), and a rectangle (shape 1) whose `cNvPr` carries a
//! **slide-jump** `hlinkClick` (→ slide 1). Slide 1 is a plain slide (shape 0 is a text box with no
//! link), used to test creating links on demand.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::Package;
use mjx_pptx::{Hyperlink, Presentation};

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

fn names(map: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    map.keys().cloned().collect()
}

fn text_of(map: &BTreeMap<String, Vec<u8>>, part: &str) -> String {
    String::from_utf8(map[part].clone()).unwrap()
}

fn deck() -> Presentation {
    Presentation::open(&fixture("hyperlinks.pptx")).expect("open")
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

#[test]
fn a_url_link_on_a_run_reads_back_as_a_url() {
    let mut pres = deck();
    assert_eq!(
        pres.run_hyperlink(0, 0, 0, 0).expect("link"),
        Some(Hyperlink::Url("https://example.com/".to_owned()))
    );
    // A run with no link, and a shape whose text has no run link, both read `None`.
    assert_eq!(pres.run_hyperlink(1, 0, 0, 0).expect("link"), None);
}

#[test]
fn a_slide_jump_on_a_shape_reads_back_as_a_slide_index() {
    let mut pres = deck();
    assert_eq!(
        pres.shape_hyperlink(0, 1).expect("link"),
        Some(Hyperlink::Slide(1)),
        "the rectangle jumps to slide 1"
    );
    // The text box carries no shape-level link.
    assert_eq!(pres.shape_hyperlink(0, 0).expect("link"), None);
}

#[test]
fn reading_hyperlinks_leaves_every_part_byte_identical() {
    let bytes = fixture("hyperlinks.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.run_hyperlink(0, 0, 0, 0).expect("run link");
    pres.shape_hyperlink(0, 1).expect("shape link");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(snapshot, reopened, "reading must not change any part");
}

// ---------------------------------------------------------------------------------------------
// Setting on a run
// ---------------------------------------------------------------------------------------------

#[test]
fn set_a_url_on_a_run_adds_one_external_relationship() {
    let bytes = fixture("hyperlinks.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Slide 1's run has no rPr at all — the link must create one.
    pres.set_run_hyperlink(
        1,
        0,
        0,
        0,
        &Hyperlink::Url("https://rust-lang.org/".to_owned()),
    )
    .expect("set");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    // No part added or removed; only slide 2 and its rels changed.
    assert_eq!(names(&snapshot), names(&reopened), "part set changed");
    const SLIDE: &str = "ppt/slides/slide2.xml";
    const SLIDE_RELS: &str = "ppt/slides/_rels/slide2.xml.rels";
    assert_ne!(reopened.get(SLIDE), snapshot.get(SLIDE));
    let rels = text_of(&reopened, SLIDE_RELS);
    assert!(
        rels.contains("https://rust-lang.org/"),
        "external target present: {rels}"
    );
    assert!(
        rels.contains(r#"TargetMode="External""#),
        "external mode: {rels}"
    );
    for (name, original) in &snapshot {
        if name != SLIDE && name != SLIDE_RELS {
            assert_eq!(
                reopened.get(name),
                Some(original),
                "part {name} must be byte-identical"
            );
        }
    }

    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(
        reread.run_hyperlink(1, 0, 0, 0).expect("link"),
        Some(Hyperlink::Url("https://rust-lang.org/".to_owned()))
    );
}

#[test]
fn replacing_a_run_link_swaps_the_relationship() {
    let bytes = fixture("hyperlinks.pptx");
    let before = text_of(
        &byte_map(&Package::open(&bytes).expect("baseline")),
        "ppt/slides/_rels/slide1.xml.rels",
    );
    assert!(before.contains("https://example.com/"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_run_hyperlink(
        0,
        0,
        0,
        0,
        &Hyperlink::Url("https://example.org/new".to_owned()),
    )
    .expect("set");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    let rels = text_of(&reopened, "ppt/slides/_rels/slide1.xml.rels");
    assert!(
        rels.contains("https://example.org/new"),
        "new target present"
    );
    assert!(
        !rels.contains("https://example.com/"),
        "old target removed: {rels}"
    );

    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(
        reread.run_hyperlink(0, 0, 0, 0).expect("link"),
        Some(Hyperlink::Url("https://example.org/new".to_owned()))
    );
}

#[test]
fn clear_a_run_link_removes_the_child_and_the_relationship() {
    let bytes = fixture("hyperlinks.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.clear_run_hyperlink(0, 0, 0, 0).expect("clear");
    let saved = pres.save().expect("save");
    let reopened = byte_map(&Package::open(&saved).expect("reopen"));

    assert_eq!(names(&snapshot), names(&reopened), "part set changed");
    let rels = text_of(&reopened, "ppt/slides/_rels/slide1.xml.rels");
    assert!(
        !rels.contains("https://example.com/"),
        "url rel removed: {rels}"
    );
    // The shape's slide-jump relationship survives — only the run link went.
    assert!(
        rels.contains(r#"relationships/slide""#),
        "slide-jump rel intact"
    );

    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(reread.run_hyperlink(0, 0, 0, 0).expect("link"), None);
    assert_eq!(
        reread.shape_hyperlink(0, 1).expect("link"),
        Some(Hyperlink::Slide(1))
    );
}

// ---------------------------------------------------------------------------------------------
// Setting on a range and on a shape
// ---------------------------------------------------------------------------------------------

#[test]
fn set_a_link_on_a_text_range_splits_runs() {
    let bytes = fixture("hyperlinks.pptx");
    let mut pres = Presentation::open(&bytes).expect("open");
    // "Second slide" — link just the word "slide" (offsets 7..12).
    assert_eq!(pres.run_count(1, 0, 0).expect("runs"), 1);
    pres.set_text_range_hyperlink(
        1,
        0,
        0,
        7..12,
        &Hyperlink::Url("https://docs.rs/".to_owned()),
    )
    .expect("set range");
    let saved = pres.save().expect("save");

    let mut reread = Presentation::open(&saved).expect("open");
    // The one run split into "Second " and "slide"; the tail carries the link, the head does not.
    assert_eq!(reread.run_count(1, 0, 0).expect("runs"), 2);
    assert_eq!(reread.run_text(1, 0, 0, 1).expect("text"), "slide");
    assert_eq!(
        reread.run_hyperlink(1, 0, 0, 1).expect("link"),
        Some(Hyperlink::Url("https://docs.rs/".to_owned()))
    );
    assert_eq!(reread.run_hyperlink(1, 0, 0, 0).expect("link"), None);
}

#[test]
fn set_and_clear_a_slide_jump_on_a_shape() {
    let bytes = fixture("hyperlinks.pptx");
    let mut pres = Presentation::open(&bytes).expect("open");
    // Slide 1's text box has no shape link; make it jump to slide 0.
    pres.set_shape_hyperlink(1, 0, &Hyperlink::Slide(0))
        .expect("set");
    let saved = pres.save().expect("save");

    let mut reread = Presentation::open(&saved).expect("open");
    assert_eq!(
        reread.shape_hyperlink(1, 0).expect("link"),
        Some(Hyperlink::Slide(0))
    );
    let jump_rels = text_of(
        &byte_map(&Package::open(&saved).expect("reopen")),
        "ppt/slides/_rels/slide2.xml.rels",
    );
    assert!(
        jump_rels.contains(r#"relationships/slide""#),
        "internal slide rel added: {jump_rels}"
    );

    // Clearing removes it and the internal relationship, reading back `None`.
    reread.clear_shape_hyperlink(1, 0).expect("clear");
    let saved2 = reread.save().expect("save");
    let mut third = Presentation::open(&saved2).expect("open");
    assert_eq!(third.shape_hyperlink(1, 0).expect("link"), None);
    let final_rels = text_of(
        &byte_map(&Package::open(&saved2).expect("reopen")),
        "ppt/slides/_rels/slide2.xml.rels",
    );
    assert!(
        !final_rels.contains(r#"relationships/slide""#),
        "slide-jump rel removed: {final_rels}"
    );
}
