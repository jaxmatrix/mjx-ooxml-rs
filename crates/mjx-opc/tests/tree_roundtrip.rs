//! Phase 1 proof: every XML part of the real fixtures round-trips through the `mjx-xml` fidelity
//! tree **byte-for-byte**. This exercises the reader + hand-written writer against genuine
//! LibreOffice output (docx/xlsx) and our synthetic pptx.

use std::path::PathBuf;

use mjx_opc::Package;
use mjx_xml::fidelity;

const FIXTURES: &[&str] = &["sample.pptx", "sample.docx", "sample.xlsx"];

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn is_xml_part(name: &str) -> bool {
    name.ends_with(".xml") || name.ends_with(".rels")
}

#[test]
fn every_xml_part_round_trips_byte_identical() {
    let mut mismatches = Vec::new();
    let mut checked = 0;

    for &fname in FIXTURES {
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
