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

// ---------------------------------------------------------------------------------------------
// MJX-140 — tying the ink part back to the shape that references it, and authoring/editing ink
// ---------------------------------------------------------------------------------------------

/// A minimal but real InkML document: one trace of three points.
const INK_STROKES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0, 5 9, 11 3</inkml:trace></inkml:ink>"#;

#[test]
fn ink_references_tie_the_part_to_the_content_part_that_names_it() {
    let mut pres = Presentation::open(&fixture("ink.pptx")).expect("open");
    let references = pres.ink_references(0).expect("references");

    assert_eq!(references.len(), 1, "the fixture references one ink part");
    let reference = &references[0];
    assert_eq!(reference.rel_id, "rId2");
    assert_eq!(reference.part, Some(part("/ppt/ink/ink1.xml")));
    assert_eq!(
        reference.shape_index, None,
        "the fixture's content part is wrapped in mc:AlternateContent, which is not in the shape \
         index space"
    );

    // A deck with no ink reports none rather than erroring.
    let mut plain = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(plain.ink_references(0).expect("references").is_empty());
}

#[test]
fn a_content_part_that_names_something_other_than_ink_is_not_reported_as_ink() {
    // The `customXml` relationship type ink reuses is shared with genuine custom XML. Point the
    // fixture's content part at a part that is *not* InkML and it must stop being reported — this is
    // what stops `ink_references` from being a rename of "every content part".
    let mut pkg = Package::open(&fixture("ink.pptx")).expect("open");
    let custom = part("/ppt/customXml/item1.xml");
    pkg.insert_part(
        &custom,
        "application/xml",
        br#"<root xmlns="urn:example"/>"#.to_vec(),
    )
    .expect("insert");
    pkg.retarget_relationship(
        Some(&part("/ppt/slides/slide1.xml")),
        "rId2",
        "../customXml/item1.xml",
        mjx_opc::TargetMode::Internal,
    )
    .expect("retarget");

    let mut pres = Presentation::open(&pkg.save().expect("save")).expect("open");
    assert!(
        pres.ink_references(0).expect("references").is_empty(),
        "a content part naming non-ink markup is not an ink reference"
    );
}

#[test]
fn added_ink_is_a_shape_that_resolves_back_to_its_part() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let before = pres.shape_count(0).expect("count");

    let shape_idx = pres.add_ink(0, INK_STROKES).expect("add ink");
    assert_eq!(shape_idx, before, "the ink lands at the end of the tree");
    assert_eq!(pres.shape_count(0).expect("count"), before + 1);

    // Read the graph back both ways.
    let ink_part = pres
        .ink_part_for_shape(0, shape_idx)
        .expect("part")
        .expect("the added ink resolves to a part");
    assert_eq!(ink_part, part("/ppt/ink/ink1.xml"));
    assert_eq!(
        pres.shape_for_ink_part(0, &ink_part).expect("shape"),
        Some(shape_idx),
        "and the part resolves back to the shape"
    );
    assert_eq!(pres.ink_part_bytes(&ink_part), Some(INK_STROKES));

    // It survives a save/reopen with the same graph.
    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let references = reopened.ink_references(0).expect("references");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].shape_index, Some(shape_idx));
    assert_eq!(references[0].part.as_ref(), Some(&ink_part));
}

#[test]
fn a_second_ink_part_is_numbered_after_the_first() {
    let mut pres = Presentation::open(&fixture("ink.pptx")).expect("open");
    pres.add_ink(0, INK_STROKES).expect("add ink");
    let names = pres.ink_part_names();
    assert_eq!(
        names,
        vec![part("/ppt/ink/ink1.xml"), part("/ppt/ink/ink2.xml")],
        "a new ink part does not collide with the one the deck already had"
    );
}

#[test]
fn ink_that_is_not_inkml_is_refused_before_anything_is_written() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let original = byte_map(&Package::open(&fixture("sample.pptx")).expect("baseline"));

    assert!(matches!(
        pres.add_ink(0, b"not xml at all"),
        Err(PptxError::InvalidInkContent)
    ));
    assert!(
        matches!(
            pres.add_ink(0, br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#),
            Err(PptxError::InvalidInkContent)
        ),
        "well-formed XML in the wrong namespace is not ink either"
    );

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(
        reopened, original,
        "a refused add must leave the package exactly as it was"
    );
}

#[test]
fn editing_ink_content_changes_only_the_ink_part() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape_idx = pres.add_ink(0, INK_STROKES).expect("add ink");
    let baseline = byte_map(&Package::open(&pres.save().expect("save")).expect("baseline"));

    let edited = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>1 1, 2 2</inkml:trace></inkml:ink>"#;
    pres.set_ink_content(0, shape_idx, edited).expect("edit");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        after.get("ppt/ink/ink1.xml").map(Vec::as_slice),
        Some(edited.as_slice()),
        "the ink part carries the new strokes"
    );
    for (name, bytes) in &baseline {
        if name == "ppt/ink/ink1.xml" {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(bytes),
            "{name} must be byte-identical after an ink edit"
        );
    }
    assert_eq!(after.len(), baseline.len(), "no part was added or removed");
}

#[test]
fn editing_ink_on_a_shape_that_references_none_is_refused() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert!(matches!(
        pres.set_ink_content(0, 0, INK_STROKES),
        Err(PptxError::ShapeIsNotAContentPart)
    ));
}
