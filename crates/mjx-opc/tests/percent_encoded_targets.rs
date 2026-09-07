//! Percent-encoded part references (MJXOFF-209) — the package half.
//!
//! ECMA-376 Part 2 §9.1.1 makes a part name an IRI: a character outside the `pchar` set is written
//! percent-encoded in a relationship `Target` and in a content-type `Override`'s `PartName`, and the
//! part it names is the **decoded** form. Real producers write these — LibreOffice 25.8 writes
//! `Target=".../my%20image.png"` for an image whose file name holds a space — and until MJXOFF-209
//! there was no percent-decoding anywhere in `mjx-opc`. `Package::validate` therefore reported such
//! a relationship as `RelationshipTargetMissing`, so `Package::save` **refused a legal file**, and
//! `remove_unreferenced_parts` could not reach the part, so `save_unchecked` deleted it.
//!
//! # What must not happen while fixing it
//!
//! Decoding and re-encoding is **not** the identity: `%2520` and `%20`, `%5f` and `%5F`, `%75` and
//! `u` are pairs that decode alike and re-encode differently. A library that normalised on write
//! would change the bytes of `.rels` and `[Content_Types].xml` parts in files nobody asked it to
//! touch — a far wider fidelity regression than the bug. So decoding happens on the way into a
//! `PartName` and nowhere else, and `percent_encoded_targets.docx` exists to hold that line: every
//! one of those four spellings is in it, and the assertions below are on the **bytes** that come
//! back out.
//!
//! The fixture is **hand-built**, not producer-written, and that is worth stating plainly: it proves
//! the code path, not that Office writes this exact package. It was authored by this library
//! (`Document::blank` + `add_inline_picture` + `create_header`, so its WordprocessingML is real and
//! the schema gate checks it) and then post-processed to rename five parts and spell their
//! references the way a conforming producer would have to. LibreOffice would not serve: it renames
//! every embedded picture to `media/imageN.png`, so it never writes an *internal* encoded target,
//! though it does percent-encode the external ones.

use mjx_opc::{Package, PartName, Relationship, TargetMode};

const FIXTURE: &str = "percent_encoded_targets.docx";

/// The five parts the fixture addresses through an escape, with the `Target` and the `PartName`
/// spelling that names each. The two control streams deliberately disagree in spelling for
/// `lower_case.png` (`%5f` vs `%5F`) and for `upper.png` (`%75pper` vs plain), because a consumer
/// must decode both and neither may be rewritten to match the other.
const ENCODED: &[(&str, &str, &str)] = &[
    (
        "/word/media/logo one.png",
        r#"Target="media/logo%20one.png""#,
        r#"PartName="/word/media/logo%20one.png""#,
    ),
    (
        // A part whose name really contains `%20`, referenced with one more level of encoding.
        "/word/media/plain%20name.png",
        r#"Target="media/plain%2520name.png""#,
        r#"PartName="/word/media/plain%2520name.png""#,
    ),
    (
        "/word/media/lower_case.png",
        r#"Target="media/lower%5fcase.png""#,
        r#"PartName="/word/media/lower%5Fcase.png""#,
    ),
    (
        // An escape that was never required, next to a rule that does not use one.
        "/word/media/upper.png",
        r#"Target="media/%75pper.png""#,
        r#"PartName="/word/media/upper.png""#,
    ),
    (
        "/word/my header.xml",
        r#"Target="my%20header.xml""#,
        r#"PartName="/word/my%20header.xml""#,
    ),
];

fn part(name: &str) -> PartName {
    PartName::new(name).expect("a valid part name")
}

fn text_of(package: &Package, part_name: &str) -> String {
    String::from_utf8(
        package
            .part_bytes(&part(part_name))
            .unwrap_or_else(|| panic!("{part_name} is in the package"))
            .to_vec(),
    )
    .expect("utf-8")
}

/// `[Content_Types].xml` is not a part, so it is reached through the entry list.
fn content_types_text(package: &Package) -> String {
    let entry = package
        .entries()
        .iter()
        .find(|entry| entry.name == mjx_opc::CONTENT_TYPES_ZIP_NAME)
        .expect("every package has one");
    String::from_utf8(entry.bytes().expect("raw bytes").to_vec()).expect("utf-8")
}

#[test]
fn every_encoded_reference_resolves_to_the_part_the_container_holds() {
    let package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens");
    let document = part("/word/document.xml");
    let rels = package
        .relationships_for(Some(&document))
        .expect("the document part has relationships");

    let resolved: Vec<String> = rels
        .iter()
        .filter(|rel| rel.mode == TargetMode::Internal)
        .map(|rel| {
            document
                .resolve(&rel.target)
                .unwrap_or_else(|e| panic!("{}: {e}", rel.target))
                .as_str()
                .to_owned()
        })
        .collect();
    let expected: Vec<&str> = ENCODED.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(resolved, expected);

    // And each resolves to a part that is really there, with the content type its `Override` gives
    // it — which is the `[Content_Types].xml` half of the same defect.
    for (name, _, _) in ENCODED {
        let part = part(name);
        assert!(
            package.part_bytes(&part).is_some(),
            "{name} must be an entry in the container"
        );
        assert!(
            package.content_type_of(&part).is_some(),
            "{name} must resolve to a content type"
        );
    }
}

#[test]
fn a_legal_package_with_encoded_targets_can_be_saved() {
    // The first consequence of the defect: `Package::validate` reported every encoded edge as
    // `RelationshipTargetMissing`, so `save` refused a file this library could open.
    let package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens");
    package.validate().expect("the fixture is a valid package");
    package.save().expect("and it saves");
}

#[test]
fn the_package_wide_sweep_reaches_every_encoded_part() {
    // The second consequence, and the destructive one: an unresolvable target is not followed, so
    // the sweep never marked these parts reachable and deleted all five as orphans.
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens");
    let swept = package.remove_unreferenced_parts().expect("the sweep runs");
    assert!(
        swept.is_empty(),
        "nothing in this package is unreachable, yet the sweep removed {swept:?}"
    );
}

#[test]
fn an_edit_leaves_every_producer_spelling_byte_identical() {
    // The trap this ticket is really about. Decoding for resolution must never become re-encoding on
    // write: the producer's own spelling of each reference has to survive an edit that rewrites the
    // very parts holding it.
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens");
    let document = part("/word/document.xml");

    // Touch both control streams: a new relationship rewrites `word/_rels/document.xml.rels`, and a
    // new part with its own `Override` rewrites `[Content_Types].xml`.
    package
        .insert_part(
            &part("/word/settings.xml"),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml",
            br#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#.to_vec(),
        )
        .expect("the part is inserted");
    package
        .add_relationship(
            Some(&document),
            Relationship {
                id: "rId6".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings"
                        .to_owned(),
                target: "settings.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the relationship is added");

    let saved = package.save().expect("it saves");
    let reopened = Package::open(&saved).expect("it reopens");

    let rels = text_of(&reopened, "/word/_rels/document.xml.rels");
    let types = content_types_text(&reopened);
    for (name, target, part_name) in ENCODED {
        assert!(
            rels.contains(target),
            "{name}: the producer wrote {target} and the saved `.rels` no longer says so:\n{rels}"
        );
        assert!(
            types.contains(part_name),
            "{name}: the producer wrote {part_name} and the saved content types no longer say \
             so:\n{types}"
        );
    }
    // And the edit itself really landed, so the assertions above are not about an untouched part.
    assert!(rels.contains(r#"Target="settings.xml""#), "{rels}");
    assert!(
        types.contains(r#"PartName="/word/settings.xml""#),
        "{types}"
    );
}

#[test]
fn removing_the_content_type_of_an_encoded_part_finds_the_rule_that_names_it() {
    // `remove_override_element` matches the raw attribute, so a rule spelled `/word/my%20header.xml`
    // is only found for the part `/word/my header.xml` if the comparison decodes. Missing it would
    // leave the element in the stream while the parsed view dropped it — the exact drift the tandem
    // edit exists to prevent, and it would surface as a package that validates but writes a rule for
    // a part that is gone.
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens");
    let header = part("/word/my header.xml");
    assert_eq!(
        package.content_type_of(&header),
        Some("application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml")
    );

    package
        .remove_content_type_override(&header)
        .expect("the override is removed");

    assert!(
        !package
            .content_types()
            .overrides()
            .iter()
            .any(|rule| rule.part_name == header),
        "the parsed view dropped the rule"
    );
    let saved = package.save_unchecked().expect("it saves");
    let reopened = Package::open(&saved).expect("it reopens");
    assert!(
        !content_types_text(&reopened).contains("my%20header"),
        "and so did the stream itself:\n{}",
        content_types_text(&reopened)
    );
}

#[test]
fn a_part_this_library_authors_with_an_awkward_name_reads_back_as_itself() {
    // The write direction. `relative_target` and the `Override` writer encode, `resolve` and
    // `ContentTypes::parse` decode, and the pair has to be exact for a name the caller chose.
    let mut package = Package::empty();
    let source = part("/word/document.xml");
    package
        .insert_part(
            &source,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
            b"<w:document/>".to_vec(),
        )
        .expect("the document part");

    for (index, name) in [
        "/word/media/a picture.png",
        "/word/media/100% margin.png",
        "/word/media/caf\u{e9}.png",
    ]
    .into_iter()
    .enumerate()
    {
        let media = part(name);
        package
            .insert_part(&media, "image/png", b"not really a png".to_vec())
            .expect("the media part");
        let target = source.relative_target(&media);
        assert!(
            !target.contains(' '),
            "an authored target must be a conforming IRI, not {target:?}"
        );
        package
            .add_relationship(
                Some(&source),
                Relationship {
                    id: format!("rId{}", index + 1),
                    rel_type:
                        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                            .to_owned(),
                    target,
                    mode: TargetMode::Internal,
                },
            )
            .expect("the relationship");
    }

    let saved = package
        .save()
        .expect("a package we authored must validate and save");
    let reopened = Package::open(&saved).expect("it reopens");
    let resolved: Vec<String> = reopened
        .relationships_for(Some(&source))
        .expect("relationships")
        .iter()
        .map(|rel| {
            source
                .resolve(&rel.target)
                .unwrap_or_else(|e| panic!("{}: {e}", rel.target))
                .as_str()
                .to_owned()
        })
        .collect();
    assert_eq!(
        resolved,
        [
            "/word/media/a picture.png",
            "/word/media/100% margin.png",
            "/word/media/caf\u{e9}.png",
        ]
    );
    for name in &resolved {
        assert_eq!(
            reopened.content_type_of(&part(name)),
            Some("image/png"),
            "{name}: its `Override` must be found again through the encoding"
        );
    }
}
