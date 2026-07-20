//! Integration tests for the layout/master inventory, read over `tests/fixtures/layouts.pptx`.
//!
//! That fixture is hand-authored so the inventory has something to enumerate: one master
//! (`Office Theme`) listing three layouts —
//!
//! | index | `p:sldLayout@type` | `p:cSld@name`       | placeholders                                          |
//! |-------|--------------------|---------------------|-------------------------------------------------------|
//! | 0     | `title`            | `Title Slide`       | `ctrTitle`, `subTitle`                                |
//! | 1     | `obj`              | `Title and Content` | `title`, body (`idx=1`), `dt`, `ftr`, `sldNum`        |
//! | 2     | `blank`            | `Blank`             | none                                                  |
//!
//! Layout 1 carries the date / footer / slide-number trio a real PowerPoint layout has — the slots a
//! slide inherits rather than owns (see `tests/slide_creation.rs`).
//!
//! — and two slides deliberately on *different* layouts: slide 0 on layout 1, slide 1 on layout 0.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_ooxml_types::presentationml::{SlideLayoutKind, SlideSizeKind};
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation};

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
    Presentation::open(&fixture("layouts.pptx")).expect("open")
}

#[test]
fn layouts_are_enumerated_in_master_then_list_order() {
    let mut pres = deck();
    assert_eq!(pres.master_count(), 1);
    assert_eq!(pres.layout_count(), 3);

    let named: Vec<(Option<String>, SlideLayoutKind)> = (0..pres.layout_count())
        .map(|idx| {
            (
                pres.layout_name(idx).expect("layout name"),
                pres.layout_kind(idx).expect("layout kind"),
            )
        })
        .collect();
    assert_eq!(
        named,
        vec![
            (Some("Title Slide".to_owned()), SlideLayoutKind::Title),
            (
                Some("Title and Content".to_owned()),
                SlideLayoutKind::TitleAndObject
            ),
            (Some("Blank".to_owned()), SlideLayoutKind::Blank),
        ],
        "layouts must follow p:sldLayoutIdLst order, with the names PowerPoint shows"
    );
}

#[test]
fn every_layout_belongs_to_its_master() {
    let mut pres = deck();
    for idx in 0..pres.layout_count() {
        assert_eq!(pres.layout_master(idx), Some(0));
    }
    assert_eq!(
        pres.master_name(0).expect("master name").as_deref(),
        Some("Office Theme")
    );
    assert!(pres
        .master_part(0)
        .expect("master part")
        .as_str()
        .ends_with("slideMaster1.xml"));
}

#[test]
fn each_slide_reports_the_layout_it_is_built_on() {
    let pres = deck();
    assert_eq!(pres.slide_count(), 2);
    // Slide 0 is on "Title and Content" (layout 1), slide 1 on "Title Slide" (layout 0) — so a
    // mapping that merely returned 0, or followed slide order, would fail here.
    assert_eq!(pres.slide_layout(0).expect("slide 0 layout"), Some(1));
    assert_eq!(pres.slide_layout(1).expect("slide 1 layout"), Some(0));
}

#[test]
fn layout_parts_are_addressable_and_distinct() {
    let pres = deck();
    let parts: Vec<&str> = (0..pres.layout_count())
        .map(|idx| pres.layout_part(idx).expect("layout part").as_str())
        .collect();
    assert_eq!(
        parts,
        vec![
            "/ppt/slideLayouts/slideLayout1.xml",
            "/ppt/slideLayouts/slideLayout2.xml",
            "/ppt/slideLayouts/slideLayout3.xml",
        ]
    );
}

#[test]
fn slide_size_reads_the_presentation_extent() {
    let mut pres = deck();
    let size = pres.slide_size().expect("slide size");
    assert_eq!(size.width_emu, 9_144_000); // 10 in
    assert_eq!(size.height_emu, 6_858_000); // 7.5 in
    assert_eq!(size.kind, SlideSizeKind::Screen4X3);
}

#[test]
fn reading_the_inventory_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    for idx in 0..pres.layout_count() {
        let _ = pres.layout_name(idx).expect("layout name");
        let _ = pres.layout_kind(idx).expect("layout kind");
    }
    let _ = pres.master_name(0).expect("master name");
    let _ = pres.slide_size().expect("slide size");
    let _ = pres.slide_layout(0).expect("slide layout");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original,
        "reading must leave every part byte-identical"
    );
}

#[test]
fn out_of_range_indices_are_typed_errors() {
    let mut pres = deck();
    assert!(matches!(
        pres.layout_name(9).expect_err("no such layout"),
        PptxError::LayoutIndexOutOfRange { index: 9, count: 3 }
    ));
    assert!(matches!(
        pres.layout_kind(9).expect_err("no such layout"),
        PptxError::LayoutIndexOutOfRange { .. }
    ));
    assert!(matches!(
        pres.master_name(9).expect_err("no such master"),
        PptxError::MasterIndexOutOfRange { index: 9, count: 1 }
    ));
    assert!(matches!(
        pres.slide_layout(9).expect_err("no such slide"),
        PptxError::SlideIndexOutOfRange { .. }
    ));
    assert_eq!(pres.layout_part(9), None);
    assert_eq!(pres.master_part(9), None);
    assert_eq!(pres.layout_master(9), None);
}

#[test]
fn a_single_layout_deck_still_reads() {
    // sample.pptx has one master, one (empty, `blank`) layout, and one slide on it.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!((pres.master_count(), pres.layout_count()), (1, 1));
    assert_eq!(pres.layout_kind(0).expect("kind"), SlideLayoutKind::Blank);
    assert_eq!(pres.layout_name(0).expect("name").as_deref(), Some("Blank"));
    assert_eq!(pres.slide_layout(0).expect("slide layout"), Some(0));
    // That deck's master carries no p:cSld@name.
    assert_eq!(pres.master_name(0).expect("master name"), None);
}
