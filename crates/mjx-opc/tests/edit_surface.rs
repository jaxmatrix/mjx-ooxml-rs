//! PR 2a exit proof: the copy-on-write edit surface.
//!
//! Editing one part's fidelity tree and saving must reflect that edit on reopen while leaving every
//! other part decompressed-byte identical; merely reading a part must not change its saved bytes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_ooxml_core::RawNode;
use mjx_opc::{Package, PartName, Relationship, TargetMode};

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

#[test]
fn set_content_type_override_roundtrips_and_leaves_others_identical() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("open baseline"));
    let mut pkg = Package::open(&bytes).expect("open");

    // slide1 already has an Override → this exercises the replace path.
    let slide = part("/ppt/slides/slide1.xml");
    let custom = "application/vnd.mjx.custom+xml";
    pkg.set_content_type_override(&slide, custom)
        .expect("set override");
    assert_eq!(
        pkg.content_type_of(&slide),
        Some(custom),
        "view not updated in tandem"
    );

    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened.content_type_of(&slide),
        Some(custom),
        "override lost on reopen"
    );

    // Only [Content_Types].xml changed.
    let reopened_map = byte_map(&reopened);
    for (name, orig) in &original {
        if name == "[Content_Types].xml" {
            continue;
        }
        assert_eq!(reopened_map.get(name), Some(orig), "part {name} changed");
    }
    assert_ne!(
        reopened_map.get("[Content_Types].xml"),
        original.get("[Content_Types].xml"),
        "content-types should have changed"
    );
}

#[test]
fn set_content_type_default_roundtrips_and_leaves_others_identical() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("open baseline"));
    let mut pkg = Package::open(&bytes).expect("open");

    pkg.set_content_type_default("PNG", "image/png")
        .expect("set default");
    // The extension is stored lowercased, and now resolves for any part with it.
    let media = part("/ppt/media/image1.png");
    assert_eq!(pkg.content_type_of(&media), Some("image/png"));

    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened.content_type_of(&media),
        Some("image/png"),
        "default lost on reopen"
    );
    assert!(reopened
        .content_types()
        .defaults()
        .iter()
        .any(|d| d.extension == "png" && d.content_type == "image/png"));

    // Only [Content_Types].xml changed.
    let reopened_map = byte_map(&reopened);
    for (name, orig) in &original {
        if name == "[Content_Types].xml" {
            continue;
        }
        assert_eq!(reopened_map.get(name), Some(orig), "part {name} changed");
    }
}

#[test]
fn set_content_type_default_places_the_rule_before_the_overrides() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");
    pkg.set_content_type_default("png", "image/png")
        .expect("set default");

    let saved = String::from_utf8(
        byte_map(&Package::open(&pkg.save().expect("save")).expect("reopen"))
            .remove("[Content_Types].xml")
            .expect("content types"),
    )
    .expect("utf-8");
    let png = saved.find(r#"Extension="png""#).expect("Default emitted");
    let first_override = saved.find("<Override").expect("fixture has overrides");
    assert!(png < first_override, "Default must precede the Overrides");
}

#[test]
fn set_content_type_default_is_idempotent_and_rejects_conflicts() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");
    pkg.set_content_type_default("png", "image/png")
        .expect("set default");
    let after_first = byte_map(&pkg);
    let defaults = pkg.content_types().defaults().len();

    // Same rule again: no second element, no change to the control part.
    pkg.set_content_type_default("png", "image/png")
        .expect("idempotent");
    assert_eq!(pkg.content_types().defaults().len(), defaults);
    assert_eq!(byte_map(&pkg), after_first);

    // A conflicting type would silently retype every .png part — rejected.
    assert!(pkg.set_content_type_default("png", "image/jpeg").is_err());
    assert_eq!(
        pkg.content_type_of(&part("/ppt/media/i.png")),
        Some("image/png")
    );
}

#[test]
fn a_part_inserted_after_its_default_gets_no_override() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");
    let overrides_before = pkg.content_types().overrides().len();

    pkg.set_content_type_default("png", "image/png")
        .expect("set default");
    let media = part("/ppt/media/image1.png");
    pkg.insert_part(&media, "image/png", vec![0x89, b'P', b'N', b'G'])
        .expect("insert");

    assert_eq!(
        pkg.content_types().overrides().len(),
        overrides_before,
        "the Default should have covered the part"
    );
    assert_eq!(pkg.content_type_of(&media), Some("image/png"));
}

#[test]
fn add_relationship_to_existing_rels_roundtrips() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("open baseline"));
    let mut pkg = Package::open(&bytes).expect("open");

    let source = part("/ppt/presentation.xml");
    let rel = Relationship {
        id: "rId4".to_owned(),
        rel_type: "http://example.com/mjx/rel".to_owned(),
        target: "slides/slide1.xml".to_owned(),
        mode: TargetMode::Internal,
    };
    pkg.add_relationship(Some(&source), rel).expect("add rel");
    assert!(
        pkg.relationships_for(Some(&source))
            .expect("rels view")
            .by_id("rId4")
            .is_some(),
        "view not updated"
    );

    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    let rp = reopened
        .relationships_for(Some(&source))
        .expect("rels view");
    assert_eq!(
        rp.by_id("rId4").map(|r| r.target.as_str()),
        Some("slides/slide1.xml")
    );
    assert!(
        rp.by_id("rId1").is_some() && rp.by_id("rId3").is_some(),
        "existing relationships dropped"
    );

    // Only presentation's .rels changed.
    let reopened_map = byte_map(&reopened);
    for (name, orig) in &original {
        if name == "ppt/_rels/presentation.xml.rels" {
            continue;
        }
        assert_eq!(reopened_map.get(name), Some(orig), "part {name} changed");
    }
}

#[test]
fn add_relationship_synthesizes_new_rels_part() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");

    // theme1.xml has no .rels of its own.
    let source = part("/ppt/theme/theme1.xml");
    assert!(
        pkg.relationships_for(Some(&source)).is_none(),
        "theme unexpectedly has relationships"
    );

    let rel = Relationship {
        id: "rId1".to_owned(),
        rel_type: "http://example.com/mjx/image".to_owned(),
        target: "../media/image1.png".to_owned(),
        mode: TargetMode::Internal,
    };
    pkg.add_relationship(Some(&source), rel).expect("add rel");

    let rels_name = "ppt/theme/_rels/theme1.xml.rels";
    assert!(
        pkg.entries().iter().any(|e| e.name == rels_name),
        "synthesized .rels missing"
    );

    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    let rp = reopened
        .relationships_for(Some(&source))
        .expect("rels present after reopen");
    assert_eq!(
        rp.by_id("rId1").map(|r| r.target.as_str()),
        Some("../media/image1.png")
    );
}

#[test]
fn remove_relationship_roundtrips() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");
    let source = part("/ppt/presentation.xml");

    assert!(
        pkg.remove_relationship(Some(&source), "rId3")
            .expect("remove"),
        "rId3 should have existed"
    );
    assert!(pkg
        .relationships_for(Some(&source))
        .expect("rels view")
        .by_id("rId3")
        .is_none());

    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    let rp = reopened
        .relationships_for(Some(&source))
        .expect("rels view");
    assert!(rp.by_id("rId3").is_none(), "rId3 present after reopen");
    assert!(
        rp.by_id("rId1").is_some() && rp.by_id("rId2").is_some(),
        "other relationships dropped"
    );

    // Removing a missing id is a no-op.
    assert!(!pkg
        .remove_relationship(Some(&source), "rId999")
        .expect("no-op remove"));
}

#[test]
fn insert_part_registers_content_type_and_roundtrips() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");

    let new = part("/ppt/slides/slide2.xml");
    let ct = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
    let content = br#"<?xml version="1.0"?><p:sld xmlns:p="urn:p"/>"#.to_vec();
    pkg.insert_part(&new, ct, content.clone()).expect("insert");
    assert_eq!(pkg.content_type_of(&new), Some(ct));
    assert_eq!(pkg.part_bytes(&new), Some(content.as_slice()));

    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    assert_eq!(reopened.content_type_of(&new), Some(ct));
    assert_eq!(reopened.part_bytes(&new), Some(content.as_slice()));

    // Inserting the same part again is rejected.
    assert!(pkg.insert_part(&new, ct, b"<p:sld/>".to_vec()).is_err());
}

#[test]
fn insert_part_adds_no_override_when_a_default_covers_it() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");
    let overrides_before = pkg.content_types().overrides().len();

    // `.xml` is covered by a Default (application/xml); insert an .xml part with that exact type.
    let new = part("/customXml/item1.xml");
    pkg.insert_part(&new, "application/xml", b"<x/>".to_vec())
        .expect("insert");

    assert_eq!(
        pkg.content_types().overrides().len(),
        overrides_before,
        "an unnecessary Override was added"
    );
    assert_eq!(pkg.content_type_of(&new), Some("application/xml"));
}

#[test]
fn remove_part_drops_entry_override_and_rels() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");

    let slide = part("/ppt/slides/slide1.xml");
    let slide_ct = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
    // Preconditions: the slide has an Override and its own .rels.
    assert!(pkg
        .content_types()
        .overrides()
        .iter()
        .any(|o| o.part_name == slide));
    assert!(pkg
        .entries()
        .iter()
        .any(|e| e.name == "ppt/slides/_rels/slide1.xml.rels"));

    pkg.remove_part(&slide).expect("remove");

    assert!(pkg.part_bytes(&slide).is_none(), "entry not removed");
    assert!(
        !pkg.content_types()
            .overrides()
            .iter()
            .any(|o| o.part_name == slide),
        "override kept"
    );
    assert!(
        !pkg.entries()
            .iter()
            .any(|e| e.name == "ppt/slides/_rels/slide1.xml.rels"),
        "rels kept"
    );

    // Saves + reopens cleanly, and the specific content type is gone.
    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    assert!(reopened.part_bytes(&slide).is_none());
    assert_ne!(
        reopened.content_type_of(&slide),
        Some(slide_ct),
        "the removed part's Override survived"
    );
}
