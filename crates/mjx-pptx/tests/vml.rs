//! Integration tests for legacy VML recognition (MJX-47, tier V1): finding the `vmlDrawingN.vml`
//! parts a package carries and reading their bytes — preserve-first, without modeling the VML XML,
//! and with fidelity (the deck round-trips byte-identically, and editing a slide leaves the VML part
//! untouched).
//!
//! Gated behind the `vml` feature. The fixture `vml.pptx` is `sample.pptx` plus a single
//! `ppt/drawings/vmlDrawing1.vml` part, related from slide 1 (`rId2`, type `vmlDrawing`) and
//! registered via a `vml` content-type Default. It is hand-crafted (see MJX-140 for producer-authentic
//! validation).
#![cfg(feature = "vml")]

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

/// Every part tied to the VML drawing that must survive an edit made elsewhere byte-for-byte.
const VML_PARTS: &[&str] = &[
    "ppt/drawings/vmlDrawing1.vml",
    "ppt/slides/_rels/slide1.xml.rels",
];

#[test]
fn vml_part_names_lists_the_drawing() {
    let pres = Presentation::open(&fixture("vml.pptx")).expect("open");
    assert_eq!(
        pres.vml_part_names(),
        vec![part("/ppt/drawings/vmlDrawing1.vml")],
        "the sole VML drawing is recognized by its content type"
    );

    // A deck with no VML answers with an empty list, not an error.
    let plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(
        plain.vml_part_names().is_empty(),
        "a deck without VML has no VML parts"
    );
}

#[test]
fn vml_part_bytes_resolves_to_the_verbatim_part() {
    let bytes = fixture("vml.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let vml_xml = baseline
        .part_bytes(&part("/ppt/drawings/vmlDrawing1.vml"))
        .expect("fixture has a VML part")
        .to_vec();

    let pres = Presentation::open(&bytes).expect("open");
    let names = pres.vml_part_names();
    let name = names.first().expect("one VML part");
    assert_eq!(
        pres.vml_part_bytes(name),
        Some(vml_xml.as_slice()),
        "the resolved bytes are exactly the package's VML part"
    );

    // An absent part answers None.
    assert_eq!(pres.vml_part_bytes(&part("/ppt/drawings/nope.vml")), None);
}

#[test]
fn reading_vml_leaves_every_part_byte_identical() {
    let bytes = fixture("vml.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    for name in pres.vml_part_names() {
        pres.vml_part_bytes(&name).expect("bytes");
    }

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading VML must dirty nothing");
}

#[test]
fn editing_a_slide_leaves_the_vml_part_byte_identical() {
    let bytes = fixture("vml.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the title text — the slide XML changes, but the separate VML part must not.
    pres.set_shape_text(0, 0, 0, "Edited").expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in VML_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "VML part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}
