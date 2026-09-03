//! Phase 0 exit proof: open real OOXML packages, understand their structure, and reconstruct the
//! container losslessly (per-part decompressed-byte identity + structural identity).
//!
//! Fixtures live at the workspace root under `tests/fixtures/` (shared across crates) and are
//! committed independently of this crate's code — never taken from the git-ignored `References/`.
//!
//! **The corpus is the directory.** This file used to carry a hand-maintained list of nine of the
//! fifteen committed fixtures, so six of them — `charts.pptx`, `tables.pptx`, `layouts.pptx`,
//! `notes.pptx`, `hyperlinks.pptx`, `effects_theme.pptx` — sat outside the byte-identity contract,
//! which is this project's core promise. `mjx_fixtures::package_fixtures` reads the directory, so a
//! fixture added in any later phase joins this suite the moment it lands.

use mjx_fixtures::{fixture, package_fixtures};
use mjx_opc::{Package, PartName, Relationship, TargetMode};

/// The committed corpus must not quietly shrink to nothing: a suite iterating an empty list passes.
fn corpus() -> Vec<String> {
    let fixtures = package_fixtures();
    assert!(
        fixtures.len() >= 15,
        "the committed corpus is {} fixture(s); a byte-identity suite over an empty corpus passes \
         vacuously",
        fixtures.len()
    );
    fixtures
}

#[test]
fn opens_and_enumerates_every_fixture() {
    for name in corpus() {
        let name = name.as_str();
        let pkg = Package::open(&fixture(name)).unwrap_or_else(|e| panic!("{name}: open: {e}"));

        assert!(!pkg.entries().is_empty(), "{name}: no entries");
        assert!(
            pkg.relationships_for(None).is_some(),
            "{name}: missing package-root relationships"
        );

        // Every addressable part must resolve to some content type (Override or Default).
        for part in pkg.part_names() {
            assert!(
                pkg.content_type_of(&part).is_some(),
                "{name}: no content type for {}",
                part.as_str()
            );
        }
    }
}

#[test]
fn round_trip_preserves_every_part_verbatim() {
    for name in corpus() {
        let name = name.as_str();
        let original = fixture(name);
        let pkg = Package::open(&original).unwrap_or_else(|e| panic!("{name}: open: {e}"));
        let saved = pkg.save().unwrap_or_else(|e| panic!("{name}: save: {e}"));
        let reopened = Package::open(&saved).unwrap_or_else(|e| panic!("{name}: reopen: {e}"));

        // Structural identity: same entry names, same order.
        let before: Vec<&str> = pkg.entries().iter().map(|e| e.name.as_str()).collect();
        let after: Vec<&str> = reopened.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            before, after,
            "{name}: entry set/order changed across round-trip"
        );

        // Per-part decompressed-payload byte identity.
        for (a, b) in pkg.entries().iter().zip(reopened.entries()) {
            assert_eq!(
                a.bytes(),
                b.bytes(),
                "{name}: decompressed bytes changed for entry {}",
                a.name
            );
        }
    }
}

#[test]
fn an_empty_package_is_a_valid_package() {
    // `Package::empty` is the constructor that needs no file. What it hands back must satisfy the
    // same invariants `opens_and_enumerates_every_fixture` demands of a real container: a root
    // relationship part, and a content type for every addressable part.
    let pkg = Package::empty();
    assert!(
        pkg.relationships_for(None).is_some(),
        "an empty package still has package-root relationships"
    );
    for part in pkg.part_names() {
        assert!(
            pkg.content_type_of(&part).is_some(),
            "no content type for {}",
            part.as_str()
        );
    }

    // The bytes it writes are bytes it can read back: the navigation views it built by construction
    // and the views a reopen parses out of the stream agree.
    let saved = pkg.save().expect("save");
    let reopened = Package::open(&saved).expect("reopen");
    let defaults: Vec<(&str, &str)> = reopened
        .content_types()
        .defaults()
        .iter()
        .map(|d| (d.extension.as_str(), d.content_type.as_str()))
        .collect();
    assert_eq!(
        defaults,
        [
            ("rels", mjx_opc::CONTENT_TYPE_RELATIONSHIPS),
            ("xml", mjx_opc::CONTENT_TYPE_XML),
        ]
    );
    assert_eq!(
        reopened
            .relationships_for(None)
            .expect("root relationships")
            .len(),
        0
    );
}

#[test]
fn a_package_built_from_empty_round_trips_verbatim() {
    // The same per-part byte-identity contract the fixtures are held to, applied to a package this
    // library assembled rather than read. A part inserted as raw bytes must come back byte for byte,
    // and the two control streams — which `insert_part` and `add_relationship` rewrite — must
    // re-serialize to themselves.
    let mut pkg = Package::empty();
    let part = PartName::new("/office/document.xml").expect("part name");
    let payload = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><doc a="1"/>"#;
    pkg.insert_part(&part, "application/x-mjx-test+xml", payload.to_vec())
        .expect("insert");
    pkg.add_relationship(
        None,
        Relationship {
            id: "rId1".to_owned(),
            rel_type: "urn:mjx:test:officeDocument".to_owned(),
            target: "office/document.xml".to_owned(),
            mode: TargetMode::Internal,
        },
    )
    .expect("relate");

    let saved = pkg.save().expect("save");
    let reopened = Package::open(&saved).expect("reopen");

    let before: Vec<&str> = pkg.entries().iter().map(|e| e.name.as_str()).collect();
    let after: Vec<&str> = reopened.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(before, after);

    assert_eq!(reopened.part_bytes(&part), Some(payload.as_slice()));
    assert_eq!(
        reopened.content_type_of(&part),
        Some("application/x-mjx-test+xml")
    );
    let root = reopened
        .relationships_for(None)
        .expect("root relationships")
        .by_id("rId1")
        .expect("rId1")
        .clone();
    assert_eq!(root.target, "office/document.xml");
    assert_eq!(root.mode, TargetMode::Internal);

    // Save → reopen → save is a fixed point.
    assert_eq!(saved, reopened.save().expect("re-save"));
}
