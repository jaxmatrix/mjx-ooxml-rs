//! Phase 1 proof: every XML part of the real fixtures round-trips through the `mjx-xml` fidelity
//! tree **byte-for-byte**. This exercises the reader + hand-written writer against genuine
//! LibreOffice output (docx/xlsx) and our synthetic pptx.
//!
//! There are no exceptions, and there is no list of them. Until subtree copy-on-write (MJX-248) the
//! VML drawing in `vml.pptx` reflowed, because the tree recorded each attribute's name, value and
//! quote but not the whitespace separating it from the previous one, and Office wraps VML start tags
//! across lines. An element now carries the byte range it was parsed from and is copied out of it
//! whole, so there is nothing left for such a list to hold.
//!
//! **The corpus is the directory** (`mjx_fixtures::package_fixtures`), not a list in this file: six
//! of the fifteen committed fixtures used to be missing from the list that stood here.

use mjx_fixtures::{fixture, package_fixtures};
use mjx_opc::Package;
use mjx_xml::fidelity;

fn is_xml_part(name: &str) -> bool {
    name.ends_with(".xml") || name.ends_with(".rels") || name.ends_with(".vml")
}

#[test]
fn every_xml_part_round_trips_byte_identical() {
    let mut mismatches = Vec::new();
    let mut checked = 0;

    let fixtures = package_fixtures();
    assert!(
        fixtures.len() >= 15,
        "the committed corpus is {} fixture(s); this suite over an empty corpus passes vacuously",
        fixtures.len()
    );
    for fname in &fixtures {
        let pkg = Package::open(&fixture(fname)).unwrap_or_else(|e| panic!("{fname}: open: {e}"));
        for entry in pkg.entries() {
            if !is_xml_part(&entry.name) {
                continue;
            }
            checked += 1;
            let original = entry
                .bytes()
                .expect("fixture entries are raw (unedited) right after open");
            let doc = fidelity::parse(original)
                .unwrap_or_else(|e| panic!("{fname}:{} parse: {e}", entry.name));
            let reserialized = fidelity::serialize_to_vec(&doc);
            if reserialized != original {
                mismatches.push(format!("{fname}:{}", entry.name));
            }
        }
    }

    assert!(
        checked >= 20,
        "expected many XML parts, only checked {checked}"
    );
    assert!(
        mismatches.is_empty(),
        "these XML parts were not byte-identical through the tree: {mismatches:#?}"
    );
}
