//! Integration tests for ink (InkML) content parts (MJX-135, tier MJX-138): recognizing the InkML
//! parts a package carries and reading their bytes — preserve-first, without modeling the ink XML, and
//! with fidelity (the deck round-trips byte-identically and editing a slide leaves the ink part
//! untouched).
//!
//! Ink is referenced from the shape tree by a `p14:contentPart` wrapped in `mc:AlternateContent`, out
//! of reach of the shape index space, so it is recognized by content type (`application/inkml+xml`)
//! rather than shape navigation. The fixture `ink.pptx` is `sample.pptx` plus one `ppt/ink/ink1.xml`
//! part, related from slide 1 (`rId2`, type `customXml`) and registered via an `inkml` content-type
//! Override. Hand-crafted (see MJX-140 for producer-authentic validation).

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

/// Every part tied to the ink object that must survive an edit made elsewhere byte-for-byte.
const INK_PARTS: &[&str] = &["ppt/ink/ink1.xml", "ppt/slides/_rels/slide1.xml.rels"];

#[test]
fn ink_part_names_lists_the_ink_part() {
    let pres = Presentation::open(&fixture("ink.pptx")).expect("open");
    assert_eq!(
        pres.ink_part_names(),
        vec![part("/ppt/ink/ink1.xml")],
        "the sole InkML part is recognized by its content type"
    );

    // A deck with no ink answers with an empty list, not an error.
    let plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(
        plain.ink_part_names().is_empty(),
        "a deck without ink has no ink parts"
    );
}

#[test]
fn ink_part_bytes_resolves_to_the_verbatim_part() {
    let bytes = fixture("ink.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let ink_xml = baseline
        .part_bytes(&part("/ppt/ink/ink1.xml"))
        .expect("fixture has an ink part")
        .to_vec();

    let pres = Presentation::open(&bytes).expect("open");
    let names = pres.ink_part_names();
    let name = names.first().expect("one ink part");
    assert_eq!(
        pres.ink_part_bytes(name),
        Some(ink_xml.as_slice()),
        "the resolved bytes are exactly the package's ink part"
    );

    // An absent part answers None.
    assert_eq!(pres.ink_part_bytes(&part("/ppt/ink/nope.xml")), None);
}

#[test]
fn reading_ink_leaves_every_part_byte_identical() {
    let bytes = fixture("ink.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    for name in pres.ink_part_names() {
        pres.ink_part_bytes(&name).expect("bytes");
    }

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading ink must dirty nothing");
}

#[test]
fn editing_a_slide_leaves_the_ink_parts_byte_identical() {
    let bytes = fixture("ink.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the title text — the slide XML changes, but the separate ink part must not.
    pres.set_shape_text(0, 0, 0, "Edited").expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in INK_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "ink part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}
