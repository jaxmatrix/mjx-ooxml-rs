//! Tier-1 byte-identity proof (MJXOFF-91's whole deliverable): [`Workbook::open`] then
//! [`Workbook::save`] reproduces every part of the container with byte-identical decompressed
//! payloads, checked **part by part** rather than by a container hash.
//!
//! # Why a bare open/save assertion would prove nothing, and what is done about it
//!
//! `mjx-opc` already round-trips `sample.xlsx` byte-identically before a line of this crate runs:
//! part-level copy-on-write re-emits every stored part verbatim when nothing dirties it, and
//! `crates/mjx-opc/tests/roundtrip.rs` pins exactly that over the whole corpus. So
//! `Workbook::open(bytes).save()` passing is, on its own, a statement about machinery one layer
//! down — the same "green precisely when nothing happened" shape this project keeps finding.
//!
//! Two things close it here, and they are the reason this file is not one assertion:
//!
//! 1. **The graph is asserted, not just the bytes.** A workbook whose sheet list, part graph and
//!    part inventory came back wrong would still round-trip byte-perfectly, because none of that is
//!    written anywhere. [`every_part_of_every_xlsx_fixture_survives_open_and_save`] therefore
//!    requires this crate's own resolution to have produced the right answers *and* the bytes to be
//!    unchanged.
//! 2. **Reading is asserted not to dirty.** The real hazard in a copy-on-write design is a read that
//!    quietly promotes a part to `Edited`, after which its bytes come from this project's writer
//!    rather than from the container.
//!    [`opening_reading_and_inventorying_never_dirties_a_part`] pins every entry's provenance after
//!    the whole read surface has been exercised, so a later child that reached for `part_tree_mut`
//!    where `part_tree` would do fails here rather than in a user's file.
//!
//! And [`one_changed_byte_in_a_preserved_part_is_caught`] is the negative: the comparison this file
//! rests on is shown to fail when a single byte of one part changes, so a green run means the parts
//! really were compared.

use mjx_fixtures::{fixture, package_fixtures_with_extension};
use mjx_opc::{Package, PartName, PartProvenance};
use mjx_xlsx::{PartClassification, PartKind, Workbook};

/// The `.xlsx` corpus, read from the fixtures directory rather than from a list in this file.
///
/// A suite iterating an empty list passes, so the corpus is asserted non-empty before anything else
/// happens — the guard `mjx-opc`'s own byte-identity suite carries, for the same reason.
fn corpus() -> Vec<String> {
    let fixtures = package_fixtures_with_extension("xlsx");
    assert!(
        !fixtures.is_empty(),
        "no .xlsx fixture — a byte-identity suite over an empty corpus passes vacuously"
    );
    fixtures
}

/// Compares two containers part by part, returning the names of the parts whose decompressed bytes
/// differ, plus a structural comparison of the entry list.
///
/// Returns `(entry_names_before, entry_names_after, differing_part_names)`.
fn compare(before: &Package, after: &Package) -> (Vec<String>, Vec<String>, Vec<String>) {
    let names_before: Vec<String> = before.entries().iter().map(|e| e.name.clone()).collect();
    let names_after: Vec<String> = after.entries().iter().map(|e| e.name.clone()).collect();
    let mut differing = Vec::new();
    for (a, b) in before.entries().iter().zip(after.entries()) {
        if a.name == b.name && a.bytes() != b.bytes() {
            differing.push(a.name.clone());
        }
    }
    (names_before, names_after, differing)
}

#[test]
fn every_part_of_every_xlsx_fixture_survives_open_and_save() {
    for name in corpus() {
        let name = name.as_str();
        let original = fixture(name);

        let workbook = Workbook::open(&original).unwrap_or_else(|e| panic!("{name}: open: {e}"));

        // (1) This crate's own code really ran, and reached the right answers. A container-level
        // round trip would be green with every one of these wrong.
        assert_eq!(
            workbook.workbook_part().as_str(),
            "/xl/workbook.xml",
            "{name}: the officeDocument relationship must lead to the workbook part"
        );
        assert!(
            !workbook.sheets().is_empty(),
            "{name}: a workbook with no tabs is not a workbook anyone wrote"
        );
        for (index, sheet) in workbook.sheets().iter().enumerate() {
            let resolved = workbook
                .worksheet(index)
                .unwrap_or_else(|e| panic!("{name}: sheet {index}: {e}"))
                .unwrap_or_else(|| {
                    panic!("{name}: sheet {index} ({}) reaches no part", sheet.name)
                });
            assert_eq!(
                resolved.part(),
                sheet.part.as_ref().expect("a resolved part")
            );
            assert!(
                resolved.kind().is_some(),
                "{name}: sheet {index} ({}) has a content type that is not a sheet's",
                sheet.name
            );
        }
        assert!(
            workbook.part_inventory().iter().any(|row| row.classification
                == PartClassification::Classified(PartKind::Workbook)),
            "{name}: the inventory must classify the workbook part"
        );

        // (2) …and after all of that reading, every part still writes back exactly as it arrived.
        let saved = workbook
            .save()
            .unwrap_or_else(|e| panic!("{name}: save: {e}"));
        let before = Package::open(&original).unwrap_or_else(|e| panic!("{name}: reopen: {e}"));
        let after = Package::open(&saved).unwrap_or_else(|e| panic!("{name}: reopen saved: {e}"));

        let (names_before, names_after, differing) = compare(&before, &after);
        assert_eq!(
            names_before, names_after,
            "{name}: the container's entry set or order changed"
        );
        assert!(
            differing.is_empty(),
            "{name}: decompressed bytes changed for {differing:?}"
        );
        assert!(
            names_before.len() >= 8,
            "{name}: only {} entries — too few for this comparison to mean anything",
            names_before.len()
        );
    }
}

/// `sample.xlsx`'s ten parts, named, so that a fixture quietly losing one is a failure here rather
/// than a silently weaker suite everywhere.
#[test]
fn the_sample_workbook_carries_exactly_the_ten_parts_this_suite_is_written_against() {
    let package = Package::open(&fixture("sample.xlsx")).expect("open");
    let mut names: Vec<&str> = package.entries().iter().map(|e| e.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "docProps/app.xml",
            "docProps/core.xml",
            "xl/_rels/workbook.xml.rels",
            "xl/sharedStrings.xml",
            "xl/styles.xml",
            "xl/theme/theme1.xml",
            "xl/workbook.xml",
            "xl/worksheets/sheet1.xml",
        ]
    );
}

#[test]
fn opening_reading_and_inventorying_never_dirties_a_part() {
    // The whole read surface, exercised: open (which parses `xl/workbook.xml`), the sheet list, each
    // sheet's own part graph, the workbook's part graph, and the inventory.
    let workbook = Workbook::open(&fixture("sample.xlsx")).expect("open sample.xlsx");
    let _ = workbook.parts();
    let _ = workbook.sheets();
    for index in 0..workbook.sheets().len() {
        let _ = workbook.worksheet(index).expect("resolve the sheet");
    }
    let _ = workbook.part_inventory();
    workbook.validate().expect("validate");

    // `Workbook` holds its package privately, so the provenance is inspected on the container it
    // writes: a part this library had authored would come back re-serialized, and the only thing
    // that could have authored one is a read above.
    let saved = workbook.save().expect("save");
    let original = Package::open(&fixture("sample.xlsx")).expect("reopen the fixture");
    let written = Package::open(&saved).expect("reopen what was written");
    for (before, after) in original.entries().iter().zip(written.entries()) {
        assert_eq!(before.name, after.name);
        assert_eq!(
            before.bytes(),
            after.bytes(),
            "{} was rewritten, so something in the read path dirtied it",
            before.name
        );
        assert_eq!(
            after.provenance(),
            PartProvenance::FromContainer,
            "{} came back from the container, so its provenance must say so",
            after.name
        );
    }
}

#[test]
fn one_changed_byte_in_a_preserved_part_is_caught() {
    // The discriminating half of this file: the comparison above is shown to go red when exactly one
    // byte of one part changes, so a green run means the parts really were compared rather than the
    // loop never having run.
    let original = fixture("sample.xlsx");
    let before = Package::open(&original).expect("open");

    let styles = PartName::new("/xl/styles.xml").expect("a valid part name");
    let mut corrupted_bytes = before
        .part_bytes(&styles)
        .expect("the fixture's styles part")
        .to_vec();
    let position = corrupted_bytes
        .iter()
        .position(|byte| *byte == b'S')
        .expect("xl/styles.xml contains an ASCII 'S'");
    corrupted_bytes[position] = b's';

    let mut corrupted = Package::open(&original).expect("open a second copy");
    corrupted
        .replace_part_bytes(&styles, corrupted_bytes)
        .expect("replace the styles part with a one-byte change");
    let saved = corrupted.save_unchecked().expect("write it out");
    let after = Package::open(&saved).expect("reopen");

    let (names_before, names_after, differing) = compare(&before, &after);
    assert_eq!(
        names_before, names_after,
        "the structural comparison is unaffected by a payload change — which is exactly why the \
         per-part byte comparison has to exist beside it"
    );
    assert_eq!(
        differing,
        vec!["xl/styles.xml".to_owned()],
        "one changed byte must show up as exactly one differing part, named"
    );
}
