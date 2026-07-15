//! PR 2a exit proof: the copy-on-write edit surface.
//!
//! Editing one part's fidelity tree and saving must reflect that edit on reopen while leaving every
//! other part decompressed-byte identical; merely reading a part must not change its saved bytes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_ooxml_core::RawNode;
use mjx_opc::{Package, PartName};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

/// A name → decompressed-bytes map of every entry that currently has materialized bytes.
fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

#[test]
fn edit_one_part_every_other_byte_identical() {
    let bytes = fixture("sample.pptx");
    // Baseline snapshot from an independent, unedited package.
    let original = byte_map(&Package::open(&bytes).expect("open baseline"));

    let mut pkg = Package::open(&bytes).expect("open");
    let pres = part("/ppt/presentation.xml");
    {
        let tree = pkg
            .part_tree_mut(&pres)
            .expect("presentation is an editable part");
        tree.root.empty = false;
        tree.root
            .children
            .push(RawNode::Comment(Box::from(&b"mjx-edit"[..])));
    }
    let saved = pkg.save().expect("save");
    let reopened = Package::open(&saved).expect("reopen");

    // Structural identity: same entry names, same order.
    let before: Vec<&str> = pkg.entries().iter().map(|e| e.name.as_str()).collect();
    let after: Vec<&str> = reopened.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(before, after, "entry set/order changed across the edit");

    // The edited part reflects the mutation.
    let edited = reopened
        .part_bytes(&pres)
        .expect("presentation present after reopen");
    assert!(
        edited.windows(8).any(|w| w == b"mjx-edit"),
        "the injected comment is missing from the edited part"
    );

    // Every OTHER part is decompressed-byte identical to the original.
    let reopened_map = byte_map(&reopened);
    for (name, orig) in &original {
        if name == "ppt/presentation.xml" {
            continue;
        }
        assert_eq!(
            reopened_map.get(name),
            Some(orig),
            "part {name} changed but should be byte-identical"
        );
    }
    // And the edited part genuinely differs from the original.
    assert_ne!(
        reopened_map.get("ppt/presentation.xml"),
        original.get("ppt/presentation.xml"),
        "the edited part should differ from the original"
    );
}

#[test]
fn reading_a_part_does_not_change_its_saved_bytes() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");
    let pres = part("/ppt/presentation.xml");
    let original = pkg.part_bytes(&pres).expect("present").to_vec();

    // Reading parses + caches a tree but must NOT dirty the part.
    let _ = pkg.part_tree(&pres).expect("readable");
    let saved = pkg.save().expect("save");
    let reopened = Package::open(&saved).expect("reopen");

    assert_eq!(
        reopened.part_bytes(&pres).expect("present"),
        original.as_slice(),
        "reading a part changed its saved bytes"
    );
}

#[test]
fn part_tree_unknown_part_errors() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let missing = part("/ppt/slides/slide999.xml");
    assert!(pkg.part_tree(&missing).is_err());
    assert!(pkg.part_tree_mut(&missing).is_err());
}

#[test]
fn part_tree_rejects_control_parts() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    // `.rels` parts are control parts.
    let root_rels = part("/_rels/.rels");
    assert!(
        pkg.part_tree(&root_rels).is_err(),
        "root .rels must be rejected"
    );
    let pres_rels = part("/ppt/_rels/presentation.xml.rels");
    assert!(
        pkg.part_tree_mut(&pres_rels).is_err(),
        "part .rels must be rejected"
    );
    // `[Content_Types].xml` is the content-type control item.
    let ct = part("/[Content_Types].xml");
    assert!(
        pkg.part_tree_mut(&ct).is_err(),
        "content-types must be rejected"
    );
}
