//! Integration tests for ActiveX form controls (MJX-135, tier MJX-137): recognizing the `p:control`s a
//! slide carries (`p:cSld > p:controls`), resolving `p:control@r:id` to the ActiveX control part, its
//! binary blob across the two-hop chain, and its fallback snapshot image — without modeling the control,
//! and with fidelity (the deck round-trips byte-identically and editing a slide leaves the ActiveX parts
//! untouched).
//!
//! The fixture `activex.pptx` is `sample.pptx` plus, on slide 1, a `p:controls > p:control` (`rId2`,
//! type `control`) referencing `ppt/activeX/activeX1.xml`, which relates to `ppt/activeX/activeX1.bin`
//! (`activeXControlBinary`), with a fallback snapshot `ppt/media/image1.png` (`rId3`). Hand-crafted
//! (see MJX-140 for producer-authentic validation).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::{Package, PartName};
use mjx_pptx::Presentation;

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

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

/// The ActiveX control is on slide 1 (surface 0), at control index 0.
const ACTIVEX_SURFACE: usize = 0;
const ACTIVEX_CONTROL: usize = 0;

/// Every part tied to the ActiveX control that must survive an edit made elsewhere byte-for-byte.
const ACTIVEX_PARTS: &[&str] = &[
    "ppt/activeX/activeX1.xml",
    "ppt/activeX/activeX1.bin",
    "ppt/activeX/_rels/activeX1.xml.rels",
    "ppt/media/image1.png",
    "ppt/slides/_rels/slide1.xml.rels",
];

#[test]
fn a_slide_reports_its_activex_controls() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    assert_eq!(
        pres.activex_control_count(ACTIVEX_SURFACE).expect("count"),
        1,
        "slide 1 carries one ActiveX control"
    );
    // A deck with no controls reports zero.
    let mut plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(plain.activex_control_count(0).expect("count"), 0);
}

#[test]
fn activex_control_rel_id_and_name_resolve() {
    let mut pres = Presentation::open(&fixture("activex.pptx")).expect("open");
    assert_eq!(
        pres.activex_control_rel_id(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("rel id"),
        Some("rId2".to_owned()),
        "the control names its control-part relationship"
    );
    assert_eq!(
        pres.activex_control_name(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("name"),
        Some("CommandButton1".to_owned()),
    );
    // An out-of-range control index answers None, not an error.
    assert_eq!(
        pres.activex_control_rel_id(ACTIVEX_SURFACE, 1)
            .expect("read"),
        None
    );
    assert_eq!(
        pres.activex_control_name(ACTIVEX_SURFACE, 1).expect("read"),
        None
    );
}

#[test]
fn activex_part_bytes_resolves_to_the_verbatim_ocx() {
    let bytes = fixture("activex.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let ocx = baseline
        .part_bytes(&part("/ppt/activeX/activeX1.xml"))
        .expect("fixture has a control part")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.activex_part_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("read"),
        Some(ocx.as_slice()),
        "the resolved bytes are exactly the package's ActiveX control part"
    );
    // An out-of-range control index answers None.
    assert_eq!(
        pres.activex_part_bytes(ACTIVEX_SURFACE, 1).expect("read"),
        None
    );
}

#[test]
fn activex_binary_bytes_resolves_across_the_two_hop_chain() {
    let bytes = fixture("activex.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let blob = baseline
        .part_bytes(&part("/ppt/activeX/activeX1.bin"))
        .expect("fixture has a control binary")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.activex_binary_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("read"),
        Some(blob.as_slice()),
        "the resolved bytes are exactly the package's ActiveX binary blob"
    );
}

#[test]
fn activex_snapshot_image_bytes_resolves_to_the_verbatim_snapshot() {
    let bytes = fixture("activex.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let snapshot = baseline
        .part_bytes(&part("/ppt/media/image1.png"))
        .expect("fixture has a snapshot image")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.activex_snapshot_rel_id(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("rel id"),
        Some("rId3".to_owned()),
    );
    assert_eq!(
        pres.activex_snapshot_image_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
            .expect("read"),
        Some(snapshot.as_slice()),
        "the resolved bytes are exactly the package's snapshot image"
    );
}

#[test]
fn reading_activex_leaves_every_part_byte_identical() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    pres.activex_control_count(ACTIVEX_SURFACE).expect("count");
    pres.activex_control_rel_id(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("rel id");
    pres.activex_control_name(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("name");
    pres.activex_part_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("ocx bytes");
    pres.activex_binary_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("binary bytes");
    pres.activex_snapshot_image_bytes(ACTIVEX_SURFACE, ACTIVEX_CONTROL)
        .expect("snapshot bytes");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading ActiveX must dirty nothing");
}

#[test]
fn editing_a_slide_leaves_the_activex_parts_byte_identical() {
    let bytes = fixture("activex.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the title text — the slide XML changes, but the separate ActiveX parts must not.
    pres.set_shape_text(ACTIVEX_SURFACE, 0, 0, "Edited")
        .expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in ACTIVEX_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "ActiveX part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}
