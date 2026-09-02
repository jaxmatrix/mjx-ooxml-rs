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

    // The target has to be a part the package actually holds: `save` validates the relationship
    // graph, and an internal relationship naming an absent part is one of the defects it refuses.
    pkg.insert_part(&part("/ppt/media/image1.png"), "image/png", b"x".to_vec())
        .expect("insert target");

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

/// Wires `source` → `target` with a fresh relationship id, resolving the target relative to `source`.
fn relate(pkg: &mut Package, source: &PartName, target: &PartName, id: &str) {
    pkg.add_relationship(
        Some(source),
        Relationship {
            id: id.to_owned(),
            rel_type: "http://example.com/test".to_owned(),
            target: source.relative_target(target),
            mode: TargetMode::Internal,
        },
    )
    .expect("relate");
}

#[test]
fn cascading_removal_takes_exclusive_targets_and_spares_shared_ones() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");

    let slide = part("/ppt/slides/slide1.xml");
    let notes = part("/ppt/notesSlides/notesSlide1.xml");
    let exclusive = part("/ppt/media/only-here.png");
    let shared = part("/ppt/media/everywhere.png");

    for (new, ct) in [
        (
            &notes,
            "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml",
        ),
        (&exclusive, "image/png"),
        (&shared, "image/png"),
    ] {
        pkg.insert_part(new, ct, b"x".to_vec()).expect("insert");
    }
    // The slide reaches all three; the notes slide points back at the slide (the dangling-reference
    // case); the presentation also shows the shared image.
    relate(&mut pkg, &slide, &notes, "rId90");
    relate(&mut pkg, &slide, &exclusive, "rId91");
    relate(&mut pkg, &slide, &shared, "rId92");
    relate(&mut pkg, &notes, &slide, "rId1");
    relate(&mut pkg, &part("/ppt/presentation.xml"), &shared, "rId90");

    let removed = pkg.remove_part_cascading(&slide).expect("cascade");

    assert!(removed.contains(&slide) && removed.contains(&notes) && removed.contains(&exclusive));
    assert_eq!(removed[0], slide, "the requested part goes first");
    assert!(!removed.contains(&shared));
    assert!(pkg.part_bytes(&notes).is_none(), "orphan notes slide kept");
    assert!(pkg.part_bytes(&exclusive).is_none(), "orphan image kept");
    assert!(
        pkg.part_bytes(&shared).is_some(),
        "an image the presentation still shows must survive"
    );
    // Nothing dangles: the removed parts' own .rels went with them.
    assert!(pkg
        .relationships()
        .iter()
        .all(|r| r.source.as_ref() != Some(&slide) && r.source.as_ref() != Some(&notes)));
    // The cascade walks downward only, so the presentation's own relationship to the slide is the
    // caller's to drop — and until it is, `save` refuses the package rather than writing a deck that
    // names a part that is not there.
    let inbound = pkg
        .relationships_for(Some(&part("/ppt/presentation.xml")))
        .expect("presentation rels")
        .iter()
        .find(|rel| rel.target.ends_with("slide1.xml"))
        .map(|rel| rel.id.clone())
        .expect("the presentation relates to slide1");
    assert!(matches!(
        pkg.save(),
        Err(mjx_opc::OpcError::Invalid(
            mjx_opc::PackageDefect::RelationshipTargetMissing { .. }
        ))
    ));
    pkg.remove_relationship(Some(&part("/ppt/presentation.xml")), &inbound)
        .expect("drop the inbound relationship");
    Package::open(&pkg.save().expect("save")).expect("reopen");
}

#[test]
fn cascading_removal_terminates_on_a_reference_cycle() {
    let bytes = fixture("sample.pptx");
    let mut pkg = Package::open(&bytes).expect("open");

    let a = part("/ppt/cycle/a.xml");
    let b = part("/ppt/cycle/b.xml");
    for new in [&a, &b] {
        pkg.insert_part(new, "application/xml", b"<x/>".to_vec())
            .expect("insert");
    }
    relate(&mut pkg, &a, &b, "rId1");
    relate(&mut pkg, &b, &a, "rId1");

    let removed = pkg.remove_part_cascading(&a).expect("cascade");
    assert_eq!(removed.len(), 2, "each part is removed exactly once");
    assert!(pkg.part_bytes(&a).is_none() && pkg.part_bytes(&b).is_none());
}

#[test]
fn cascading_removal_of_an_unknown_part_is_rejected() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let err = pkg
        .remove_part_cascading(&part("/ppt/slides/slide9.xml"))
        .expect_err("no such part");
    assert!(matches!(err, mjx_opc::OpcError::UnknownPart(_)), "{err:?}");
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

    // `remove_part` unwires the part's own outgoing edges, never the inbound ones, so the
    // presentation still names the slide: the caller drops that relationship before saving.
    let inbound = pkg
        .relationships_for(Some(&part("/ppt/presentation.xml")))
        .expect("presentation rels")
        .iter()
        .find(|rel| rel.target.ends_with("slide1.xml"))
        .map(|rel| rel.id.clone())
        .expect("the presentation relates to slide1");
    pkg.remove_relationship(Some(&part("/ppt/presentation.xml")), &inbound)
        .expect("drop the inbound relationship");

    // Saves + reopens cleanly, and the specific content type is gone.
    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    assert!(reopened.part_bytes(&slide).is_none());
    assert_ne!(
        reopened.content_type_of(&slide),
        Some(slide_ct),
        "the removed part's Override survived"
    );
}

/// Every part of a well-formed deck is reachable from the package root, so a sweep removes nothing and
/// leaves every part decompressed-byte identical.
#[test]
fn sweep_is_a_no_op_on_a_clean_deck() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pkg = Package::open(&bytes).expect("open");
    let removed = pkg.remove_unreferenced_parts().expect("sweep");

    assert!(
        removed.is_empty(),
        "a clean deck has no orphans: {removed:?}"
    );
    assert_eq!(byte_map(&pkg), original, "the sweep dirtied a part");
}

/// A media part nothing points at is swept, while every part the deck still reaches — including one a
/// live slide references — survives byte-identically.
#[test]
fn sweep_removes_a_lone_orphan_and_spares_the_live_deck() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let slide = part("/ppt/slides/slide1.xml");
    let shown = part("/ppt/media/shown.png");
    let orphan = part("/ppt/media/orphan.png");
    pkg.insert_part(&shown, "image/png", b"shown".to_vec())
        .expect("insert shown");
    pkg.insert_part(&orphan, "image/png", b"orphan".to_vec())
        .expect("insert orphan");
    // Only `shown` is wired to the (reachable) slide; `orphan` is left dangling.
    relate(&mut pkg, &slide, &shown, "rId90");

    // Snapshot after the edits, so the comparison isolates what the *sweep* changed.
    let before = byte_map(&pkg);
    let removed = pkg.remove_unreferenced_parts().expect("sweep");

    assert_eq!(removed, vec![orphan.clone()], "exactly the orphan is swept");
    assert!(pkg.part_bytes(&orphan).is_none(), "orphan not removed");
    assert!(
        pkg.part_bytes(&shown).is_some(),
        "a media part a live slide references must survive"
    );
    // The sweep disturbed nothing else: every surviving materialized part is byte-identical to its
    // pre-sweep bytes ([Content_Types].xml aside — removing the orphan drops its Override).
    for (name, bytes) in &byte_map(&pkg) {
        if name == "[Content_Types].xml" {
            continue;
        }
        assert_eq!(before.get(name), Some(bytes), "{name} was disturbed");
    }
    // Control parts are never candidates for removal.
    assert!(
        pkg.entries().iter().any(|e| e.name == "_rels/.rels")
            && pkg
                .entries()
                .iter()
                .any(|e| e.name == "ppt/_rels/presentation.xml.rels"),
        "a .rels control part was swept"
    );
    Package::open(&pkg.save().expect("save")).expect("reopen");
}

/// An orphan that itself references a second orphan: neither is reachable from the root, so the whole
/// chain is swept. Proves the walk is transitive-from-root, not a one-hop "is anything pointing at it".
#[test]
fn sweep_removes_an_orphan_chain() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let head = part("/ppt/media/orphan-head.xml");
    let tail = part("/ppt/media/orphan-tail.png");
    pkg.insert_part(&head, "application/xml", b"<x/>".to_vec())
        .expect("insert head");
    pkg.insert_part(&tail, "image/png", b"tail".to_vec())
        .expect("insert tail");
    // `head` points at `tail`, but nothing reachable points at `head`.
    relate(&mut pkg, &head, &tail, "rId1");

    let removed = pkg.remove_unreferenced_parts().expect("sweep");

    assert!(
        removed.contains(&head) && removed.contains(&tail),
        "the whole orphan chain must go: {removed:?}"
    );
    assert!(pkg.part_bytes(&head).is_none() && pkg.part_bytes(&tail).is_none());
}

/// Wires `source` → an external URI with a fresh relationship id.
fn relate_external(pkg: &mut Package, source: &PartName, target: &str, id: &str) {
    pkg.add_relationship(
        Some(source),
        Relationship {
            id: id.to_owned(),
            rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                .to_owned(),
            target: target.to_owned(),
            mode: TargetMode::External,
        },
    )
    .expect("relate external");
}

#[test]
fn external_relationships_lists_only_the_external_ones() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let slide = part("/ppt/slides/slide1.xml");
    let internal = part("/ppt/media/local.png");
    pkg.insert_part(&internal, "image/png", b"x".to_vec())
        .expect("insert");
    relate(&mut pkg, &slide, &internal, "rId50"); // Internal — must be ignored
    relate_external(&mut pkg, &slide, "https://example.com/linked.png", "rId51");

    let external = pkg.external_relationships();
    let ours: Vec<_> = external
        .iter()
        .filter(|r| r.source.as_ref() == Some(&slide))
        .collect();
    assert_eq!(ours.len(), 1, "only the external rel is reported: {ours:?}");
    let only = ours[0];
    assert_eq!(only.id, "rId51");
    assert_eq!(only.target, "https://example.com/linked.png");
    assert!(only.rel_type.ends_with("/image"));
}

#[test]
fn retarget_relationship_redirects_an_external_rel_to_a_placeholder_in_place() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let slide = part("/ppt/slides/slide1.xml");
    // Two rels so we can prove order is preserved: the external one sits before a later internal one.
    relate_external(&mut pkg, &slide, "https://example.com/linked.png", "rId60");
    let sentinel = part("/ppt/media/sentinel.png");
    pkg.insert_part(&sentinel, "image/png", b"s".to_vec())
        .expect("insert sentinel");
    relate(&mut pkg, &slide, &sentinel, "rId61");
    let order_before: Vec<String> = pkg
        .relationships_for(Some(&slide))
        .expect("rels")
        .iter()
        .map(|r| r.id.clone())
        .collect();

    // The redirect recipe: insert a placeholder part, then point the external rel at it internally.
    let placeholder = part("/ppt/media/placeholder.png");
    pkg.insert_part(&placeholder, "image/png", b"placeholder".to_vec())
        .expect("insert placeholder");
    let target = slide.relative_target(&placeholder);
    let found = pkg
        .retarget_relationship(Some(&slide), "rId60", &target, TargetMode::Internal)
        .expect("retarget");
    assert!(found, "the relationship existed");

    // View updated: now Internal and resolving to the placeholder part.
    let rels = pkg.relationships_for(Some(&slide)).expect("rels");
    let rel = rels.by_id("rId60").expect("rel");
    assert_eq!(rel.mode, TargetMode::Internal);
    assert_eq!(slide.resolve(&rel.target).expect("resolve"), placeholder);
    // Order preserved (in-place edit, not remove+append).
    let order_after: Vec<String> = rels.iter().map(|r| r.id.clone()).collect();
    assert_eq!(order_after, order_before, "the .rels order changed");

    // Survives a round-trip: no External rels remain on the slide, the redirect stuck.
    let reopened = Package::open(&pkg.save().expect("save")).expect("reopen");
    assert!(
        reopened
            .external_relationships()
            .iter()
            .all(|r| r.source.as_ref() != Some(&slide)),
        "an external rel survived the redirect"
    );
    assert_eq!(
        reopened
            .relationships_for(Some(&slide))
            .expect("rels")
            .by_id("rId60")
            .expect("rel")
            .target,
        target
    );
}

#[test]
fn retarget_relationship_of_an_unknown_id_is_a_no_op() {
    let mut pkg = Package::open(&fixture("sample.pptx")).expect("open");
    let slide = part("/ppt/slides/slide1.xml");
    assert!(!pkg
        .retarget_relationship(
            Some(&slide),
            "rId999",
            "../media/x.png",
            TargetMode::Internal
        )
        .expect("retarget"));
    // Unknown source .rels is likewise a no-op.
    assert!(!pkg
        .retarget_relationship(
            Some(&part("/ppt/nope.xml")),
            "rId1",
            "x",
            TargetMode::Internal
        )
        .expect("retarget"));
}

/// Subtree copy-on-write (MJX-248): editing one attribute of one element must leave every *other
/// subtree of the same part* byte-identical, not merely every other part.
///
/// `vml.pptx`'s drawing is the fixture that proves it because Office wrote it with its start tags
/// wrapped across lines. Reconstructing any of those elements collapses them onto one line, so a
/// sibling that survives with its wrapping intact can only have come from the original bytes.
#[test]
fn editing_one_attribute_leaves_the_other_subtrees_of_the_same_part_byte_identical() {
    let bytes = fixture("vml.pptx");
    let drawing = part("/ppt/drawings/vmlDrawing1.vml");
    let original = Package::open(&bytes)
        .expect("open baseline")
        .part_bytes(&drawing)
        .expect("the VML drawing is present")
        .to_vec();

    let mut pkg = Package::open(&bytes).expect("open");
    {
        let tree = pkg.part_tree_mut(&drawing).expect("editable");
        // `<o:shapelayout><o:idmap … data="1"/></o:shapelayout>` — one attribute, three levels down.
        let RawNode::Element(shapelayout) = &mut tree.root.children[1] else {
            panic!("expected <o:shapelayout> as the second child");
        };
        let RawNode::Element(idmap) = &mut shapelayout.children[1] else {
            panic!("expected <o:idmap> inside <o:shapelayout>");
        };
        idmap.attributes[1].value = Box::from(&b"7"[..]);
    }
    let saved = pkg.save().expect("save");
    let edited = Package::open(&saved)
        .expect("reopen")
        .part_bytes(&drawing)
        .expect("still present")
        .to_vec();

    // The edit landed.
    assert!(
        contains(&edited, br#"data="7""#),
        "the edit is missing:\n{}",
        String::from_utf8_lossy(&edited)
    );

    // Every sibling subtree came through with its original wrapping — byte for byte.
    for untouched in [
        &b"<v:shapetype id=\"_x0000_t202\" coordsize=\"21600,21600\" o:spt=\"202\"\r\n  path=\"m,l,21600r21600,l21600,xe\">"[..],
        &b"<v:shape id=\"_x0000_s1026\" type=\"#_x0000_t202\"\r\n  style=\"position:absolute;margin-left:10pt;margin-top:10pt;width:100pt;height:50pt\"\r\n  filled=\"f\" stroked=\"f\">"[..],
        &b"<v:path gradientshapeok=\"t\" o:connecttype=\"rect\"/>"[..],
    ] {
        assert!(
            contains(&edited, untouched),
            "an untouched subtree reflowed:\n{}",
            String::from_utf8_lossy(&edited)
        );
    }

    // The wrapping Office wrote is CRLF; reconstruction would have collapsed it to one space, so
    // these needles cannot match anything the writer produced from the model.
    // The rewritten root re-emitted every namespace declaration its verbatim descendants depend on.
    for declaration in [
        &br#"xmlns:v="urn:schemas-microsoft-com:vml""#[..],
        &br#"xmlns:o="urn:schemas-microsoft-com:office:office""#[..],
        &br#"xmlns:p="urn:schemas-microsoft-com:office:powerpoint""#[..],
    ] {
        assert!(
            contains(&edited, declaration),
            "the rewritten root dropped a namespace declaration:\n{}",
            String::from_utf8_lossy(&edited)
        );
    }

    // Only the path from the root down to the edited element was rewritten: everything else is
    // still literally the original bytes.
    assert_ne!(edited, original, "the edit should have changed the part");
    assert!(
        !contains(&edited, br#"data="1""#),
        "the stale original value survived:\n{}",
        String::from_utf8_lossy(&edited)
    );
}

/// Reading a part as a tree must leave its saved bytes alone even now that the tree can write from
/// the part's own buffer — the copy-on-write state, not the buffer, is what decides.
#[test]
fn reading_a_vml_part_does_not_reflow_it() {
    let bytes = fixture("vml.pptx");
    let drawing = part("/ppt/drawings/vmlDrawing1.vml");
    let mut pkg = Package::open(&bytes).expect("open");
    let original = pkg.part_bytes(&drawing).expect("present").to_vec();
    let _ = pkg.part_tree(&drawing).expect("readable");
    let saved = pkg.save().expect("save");
    assert_eq!(
        Package::open(&saved)
            .expect("reopen")
            .part_bytes(&drawing)
            .expect("present"),
        original.as_slice()
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// A part still holding a tree that can be written from its source buffer keeps the buffer; a part
/// whose every element has been rewritten gives it back.
#[test]
fn release_unused_part_sources_reclaims_only_a_fully_rewritten_part() {
    let bytes = fixture("vml.pptx");
    let drawing = part("/ppt/drawings/vmlDrawing1.vml");
    let mut pkg = Package::open(&bytes).expect("open");
    {
        let tree = pkg.part_tree_mut(&drawing).expect("editable");
        tree.root.attributes.clear();
    }
    assert_eq!(
        pkg.release_unused_part_sources(),
        0,
        "the children can still be copied from the buffer"
    );

    // Replacing the root outright leaves nothing that references the buffer.
    {
        let tree = pkg.part_tree_mut(&drawing).expect("editable");
        tree.root = tree.root.clone();
    }
    assert_eq!(pkg.release_unused_part_sources(), 1);
    assert_eq!(
        pkg.release_unused_part_sources(),
        0,
        "there is nothing left to release"
    );
    // ...and the part still saves, reconstructed from the model.
    let saved = pkg.save().expect("save");
    assert!(Package::open(&saved)
        .expect("reopen")
        .part_bytes(&drawing)
        .is_some());
}
