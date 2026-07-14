//! Phase 0 exit proof: open real OOXML packages, understand their structure, and reconstruct the
//! container losslessly (per-part decompressed-byte identity + structural identity).
//!
//! Fixtures live at the workspace root under `tests/fixtures/` (shared across crates) and are
//! committed independently of this crate's code — never taken from the git-ignored `References/`.

use std::path::PathBuf;

use mjx_opc::Package;

const FIXTURES: &[&str] = &["sample.pptx", "sample.docx", "sample.xlsx"];

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

#[test]
fn opens_and_enumerates_every_fixture() {
    for &name in FIXTURES {
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
    for &name in FIXTURES {
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
