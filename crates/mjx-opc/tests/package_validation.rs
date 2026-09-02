//! Every package invariant `save` enforces, proved by a package that breaks exactly one of them.
//!
//! A validator that never fires is indistinguishable from no validator, so each test here builds a
//! package that is correct in every respect *but one*, and asserts both that `save` refuses it and
//! that it refuses it for the right reason. Breaking one thing at a time is the point: a package
//! broken twice would pass for the wrong reason.
//!
//! The counterpart tests — that a correct package still saves, that a package which arrived broken
//! can still be written back, and that validating changes nothing — are here too, because the cost of
//! a validator that is too eager is a library that cannot round-trip a real file.

use std::path::PathBuf;

use mjx_opc::{OpcError, Package, PackageDefect, PartName, Relationship, TargetMode};

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// The defect a save was refused for, or a panic naming what happened instead.
fn defect(package: &Package) -> PackageDefect {
    match package.save() {
        Ok(_) => panic!("the package saved, but it violates an invariant"),
        Err(OpcError::Invalid(defect)) => defect,
        Err(other) => panic!("expected an invariant failure, got {other:?}"),
    }
}

/// An internal relationship between two parts that both exist.
fn relate(package: &mut Package, source: Option<&PartName>, id: &str, target: &str) {
    package
        .add_relationship(
            source,
            Relationship {
                id: id.to_owned(),
                rel_type: "http://example.com/mjx/thing".to_owned(),
                target: target.to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("add relationship");
}

// ---------------------------------------------------------------------------------------------
// One invariant per test
// ---------------------------------------------------------------------------------------------

/// A part no content-type rule covers. Nothing else about this package is wrong: the part is
/// unreferenced, which is legal, and every relationship resolves.
#[test]
fn a_part_with_no_content_type_is_refused() {
    let mut package = Package::empty();
    let orphan = part("/data/payload.bin");
    package
        .insert_part(&orphan, "application/octet-stream", vec![0, 1, 2])
        .expect("insert");
    // Drop the Override that `insert_part` registered; `bin` has no Default either.
    package
        .remove_content_type_override(&orphan)
        .expect("remove override");
    assert!(package.content_type_of(&orphan).is_none());

    match defect(&package) {
        PackageDefect::PartWithoutContentType { part } => {
            assert_eq!(part, "/data/payload.bin");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// An internal relationship naming a part the package does not hold.
#[test]
fn a_relationship_targeting_an_absent_part_is_refused() {
    let mut package = Package::empty();
    relate(&mut package, None, "rId1", "ppt/presentation.xml");

    match defect(&package) {
        PackageDefect::RelationshipTargetMissing {
            relationships_part,
            relationship_id,
            target,
            resolved_part,
        } => {
            assert_eq!(relationships_part, "_rels/.rels");
            assert_eq!(relationship_id, "rId1");
            assert_eq!(target, "ppt/presentation.xml");
            assert_eq!(resolved_part, "/ppt/presentation.xml");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// An internal relationship whose target is not a part name at all — this one climbs above the
/// package root, which no part can be above.
#[test]
fn a_relationship_target_that_names_no_part_is_refused() {
    let mut package = Package::empty();
    relate(&mut package, None, "rId1", "../outside/thing.xml");

    match defect(&package) {
        PackageDefect::UnresolvableRelationshipTarget {
            relationships_part,
            relationship_id,
            target,
        } => {
            assert_eq!(relationships_part, "_rels/.rels");
            assert_eq!(relationship_id, "rId1");
            assert_eq!(target, "../outside/thing.xml");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// Two relationships in one `.rels` sharing an `Id` (ECMA-376 Part 2 §6.5.3). Both targets exist and
/// are typed, so the duplicate id is the only thing wrong.
#[test]
fn a_duplicate_relationship_id_is_refused() {
    let mut package = Package::empty();
    for name in ["/one.xml", "/two.xml"] {
        package
            .insert_part(&part(name), "application/xml", b"<a/>".to_vec())
            .expect("insert");
    }
    relate(&mut package, None, "rId1", "one.xml");
    relate(&mut package, None, "rId1", "two.xml");

    match defect(&package) {
        PackageDefect::DuplicateRelationshipId {
            relationships_part,
            relationship_id,
        } => {
            assert_eq!(relationships_part, "_rels/.rels");
            assert_eq!(relationship_id, "rId1");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// Markup naming a relationship id its own `.rels` never declares — the dangling `r:id`. The part is
/// typed and unreferenced (both legal); only the reference is broken.
#[test]
fn markup_naming_an_undeclared_relationship_is_refused() {
    let mut package = Package::empty();
    package
        .insert_part(
            &part("/doc.xml"),
            "application/xml",
            br#"<x xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><y r:id="rId7"/></x>"#.to_vec(),
        )
        .expect("insert");

    match defect(&package) {
        PackageDefect::UndeclaredRelationshipReference {
            part,
            element,
            attribute,
            relationship_id,
        } => {
            assert_eq!(part, "/doc.xml");
            assert_eq!(element, "y");
            assert_eq!(attribute, "r:id");
            assert_eq!(relationship_id, "rId7");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

/// …and the same markup saves once the relationship it names exists. Without this, the test above
/// would pass for a validator that rejected every reference.
#[test]
fn markup_naming_a_declared_relationship_saves() {
    let mut package = Package::empty();
    let doc = part("/doc.xml");
    package
        .insert_part(
            &doc,
            "application/xml",
            br#"<x xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><y r:id="rId7"/></x>"#.to_vec(),
        )
        .expect("insert");
    package
        .insert_part(&part("/target.xml"), "application/xml", b"<a/>".to_vec())
        .expect("insert");
    relate(&mut package, Some(&doc), "rId7", "target.xml");

    package.save().expect("a declared reference is fine");
}

/// A part this library wrote, typed as XML, whose bytes are not well-formed.
#[test]
fn authored_bytes_that_are_not_well_formed_xml_are_refused() {
    let mut package = Package::empty();
    package
        .insert_part(&part("/doc.xml"), "application/xml", b"<x><y></x>".to_vec())
        .expect("insert");

    match defect(&package) {
        PackageDefect::PartIsNotWellFormedXml { part, error } => {
            assert_eq!(part, "/doc.xml");
            assert!(!error.is_empty(), "the parse failure must be reported");
        }
        other => panic!("wrong defect: {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// The other half: what validation must *not* do
// ---------------------------------------------------------------------------------------------

/// Every fixture in the corpus opens, validates and saves unchanged. A validator that fires on real
/// Office files is worse than none.
#[test]
fn every_fixture_saves_clean() {
    for name in [
        "sample.pptx",
        "sample.docx",
        "sample.xlsx",
        "vml.pptx",
        "ole.pptx",
        "activex.pptx",
        "ink.pptx",
        "text_levels.pptx",
        "table_extensions.pptx",
        "charts.pptx",
        "tables.pptx",
        "layouts.pptx",
        "notes.pptx",
        "hyperlinks.pptx",
        "effects_theme.pptx",
    ] {
        let package = Package::open(&fixture(name)).unwrap_or_else(|e| panic!("{name}: open: {e}"));
        package
            .validate()
            .unwrap_or_else(|e| panic!("{name}: validate: {e}"));
        package
            .save()
            .unwrap_or_else(|e| panic!("{name}: save: {e}"));
    }
}

/// Markup that arrived broken is written back, not refused. The library's promise is that a part it
/// did not touch re-emits verbatim — refusing to write such a file would lose it, so the reference
/// check is scoped to markup this library produced.
#[test]
fn a_package_is_not_faulted_for_reference_markup_it_arrived_with() {
    let mut authored = Package::empty();
    authored
        .insert_part(
            &part("/doc.xml"),
            "application/xml",
            br#"<x xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId7"/>"#.to_vec(),
        )
        .expect("insert");
    // Only `save_unchecked` can produce the broken container in the first place.
    let broken = authored.save_unchecked().expect("write the broken package");

    let reopened = Package::open(&broken).expect("reopen");
    reopened
        .save()
        .expect("a part re-emitting its container bytes is not ours to fault");
}

/// …and the moment this library takes responsibility for that part's bytes, it does fault it.
#[test]
fn the_same_markup_is_faulted_once_this_library_writes_it() {
    let mut authored = Package::empty();
    authored
        .insert_part(
            &part("/doc.xml"),
            "application/xml",
            br#"<x xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId7"/>"#.to_vec(),
        )
        .expect("insert");
    let broken = authored.save_unchecked().expect("write the broken package");

    let mut reopened = Package::open(&broken).expect("reopen");
    // Any edit to the part moves it out of "container bytes we re-emit verbatim".
    reopened
        .part_tree_mut(&part("/doc.xml"))
        .expect("edit the part");
    assert!(matches!(
        defect(&reopened),
        PackageDefect::UndeclaredRelationshipReference { .. }
    ));
}

/// Reading a part must not change whether the package saves: validation is scoped by where the bytes
/// came from, not by what the caller happened to look at.
#[test]
fn reading_a_part_does_not_change_the_verdict() {
    let mut authored = Package::empty();
    authored
        .insert_part(
            &part("/doc.xml"),
            "application/xml",
            br#"<x xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId7"/>"#.to_vec(),
        )
        .expect("insert");
    let broken = authored.save_unchecked().expect("write the broken package");

    let mut reopened = Package::open(&broken).expect("reopen");
    reopened.part_tree(&part("/doc.xml")).expect("read it");
    reopened.save().expect("reading is not editing");
}

/// `save_unchecked` is the escape hatch, and it really does write the package validation refused.
#[test]
fn save_unchecked_writes_what_save_refuses() {
    let mut package = Package::empty();
    relate(&mut package, None, "rId1", "ppt/gone.xml");

    assert!(matches!(
        defect(&package),
        PackageDefect::RelationshipTargetMissing { .. }
    ));
    let bytes = package
        .save_unchecked()
        .expect("the escape hatch writes it");
    let reopened = Package::open(&bytes).expect("and the container is well formed");
    assert!(reopened
        .relationships_for(None)
        .expect("root rels")
        .by_id("rId1")
        .is_some());
}

/// Validation is a read-only pass: it must not parse a part that is still raw bytes, because a
/// materialized tree is exactly what part-level laziness exists to avoid.
#[test]
fn validation_leaves_every_part_in_the_state_it_found_it() {
    let mut package = Package::open(&fixture("sample.pptx")).expect("open");
    // One part read (Parsed), one edited (Edited), the rest untouched (Raw).
    package
        .part_tree(&part("/ppt/slides/slide1.xml"))
        .expect("read");
    package
        .part_tree_mut(&part("/ppt/presentation.xml"))
        .expect("edit");

    let before: Vec<(String, bool)> = package
        .entries()
        .iter()
        .map(|entry| (entry.name.clone(), entry.tree().is_some()))
        .collect();
    package.validate().expect("valid");
    let after: Vec<(String, bool)> = package
        .entries()
        .iter()
        .map(|entry| (entry.name.clone(), entry.tree().is_some()))
        .collect();

    assert_eq!(before, after, "validation materialized a tree");
    // And the untouched theme really was raw before and after — otherwise the assertion above is
    // comparing two sets of trees that were all materialized already.
    let theme = package
        .entries()
        .iter()
        .find(|entry| entry.name == "ppt/theme/theme1.xml")
        .expect("theme entry");
    assert!(
        theme.tree().is_none(),
        "an untouched part must still be raw bytes after validation"
    );
}
