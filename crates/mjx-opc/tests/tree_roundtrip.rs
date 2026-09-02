//! Phase 1 proof: every XML part of the real fixtures round-trips through the `mjx-xml` fidelity
//! tree **byte-for-byte**. This exercises the reader + hand-written writer against genuine
//! LibreOffice output (docx/xlsx) and our synthetic pptx.

use std::path::PathBuf;

use mjx_opc::Package;
use mjx_xml::fidelity;

const FIXTURES: &[&str] = &[
    "sample.pptx",
    "sample.docx",
    "sample.xlsx",
    "text_levels.pptx",
    // The legacy-content fixtures (MJX-140): their slides, relationship streams and content types
    // are ordinary XML and must round-trip like any other. Their one VML part is the exception
    // below.
    "vml.pptx",
    "ole.pptx",
    "activex.pptx",
    "ink.pptx",
    // A table carrying `a:extLst` on its properties and on a cell (MJX-43).
    "table_extensions.pptx",
];

/// Parts this suite knows do **not** round-trip byte-identically through the fidelity tree, with the
/// reason. Deliberately tiny: an entry is an admission, and a part that starts round-tripping fails
/// here rather than silently gaining coverage nobody asked for.
///
/// This never weakens the round-trip *contract*: a part nobody edits keeps its original bytes through
/// the `mjx-opc` copy-on-write layer and is never re-serialized at all. What it records is a
/// limitation of the writer, reached only by a part a caller does edit.
const KNOWN_REFLOWS: &[(&str, &str, &str)] = &[(
    "vml.pptx",
    "ppt/drawings/vmlDrawing1.vml",
    "the fidelity reader records each attribute's name, value and quote but not the whitespace that \
     separated it from the previous one, so a start tag whose attributes were wrapped across lines \
     re-emits on one line. Office wraps VML start tags far more often than it wraps a slide's, which \
     is why this shows here first",
)];

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn is_xml_part(name: &str) -> bool {
    name.ends_with(".xml") || name.ends_with(".rels") || name.ends_with(".vml")
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
            let known = KNOWN_REFLOWS
                .iter()
                .find(|(fixture, part, _)| *fixture == fname && *part == entry.name);
            match (reserialized == original, known) {
                (true, None) | (false, Some(_)) => {}
                (false, None) => mismatches.push(format!("{fname}:{}", entry.name)),
                (true, Some((_, part, reason))) => mismatches.push(format!(
                    "{fname}:{part} now round-trips — delete its KNOWN_REFLOWS entry ({reason})"
                )),
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
