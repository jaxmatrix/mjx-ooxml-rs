//! `word/numbering.xml` (MJXOFF-104): the two-hop resolution from a paragraph's `w:numPr` to the
//! `w:lvl` it actually uses, the shared-abstract-definition/`w:startOverride` trap, style-linked
//! numbering through `word/styles.xml`, `numId = 0` versus a genuinely dangling `numId`, and
//! authoring/round-tripping `word/numbering.xml` itself.
//!
//! No fixture in the corpus carried `word/numbering.xml` before this child — see `numbering.rs`'s
//! own module doc. `tests/fixtures/numbering_definitions.docx` is authored for it: two `w:num`
//! instances (`numId` 2 and 5, deliberately non-contiguous and non-ascending against the also-present
//! `numId` 9) sharing one `w:abstractNum`, where only `numId` 2 carries a `w:startOverride`; a third
//! `w:abstractNum` that delegates through `w:numStyleLink` to a numbering-type style in
//! `word/styles.xml`; and one paragraph per scenario, including an explicit `numId = 0`.
//! `tests/fixtures/paragraph_properties.docx` (MJXOFF-96, already committed) supplies the dangling-
//! `numId` evidence: it carries a real `w:numPr` (`numId = 5`) while relating to no
//! `word/numbering.xml` at all.

use mjx_docx::{
    AbstractNumbering, Document, DocxError, LevelTextSegment, LevelTextTemplate, MainDocument,
    NumberingIndex, NumberingInstance, NumberingLevel, NumberingLevelOverride, NumberingLookup,
    NumberingProperties, Package, PageSize, PartName, StyleDefinition, StyleString,
    MAX_NUM_STYLE_LINK_DEPTH,
};
use mjx_fixtures::fixture;
use mjx_ooxml_core::{FromXml, Interner};
use mjx_ooxml_types::wordprocessingml::{NumberFormat, StyleType};
use mjx_schema_gate::assert_authored_deck_is_schema_valid;

// -------------------------------------------------------------------------------------------
// The two-hop resolution, on the authored fixture: numId -> w:num -> abstractNumId -> w:abstractNum
// -> w:lvl.
// -------------------------------------------------------------------------------------------

/// Would this pass if the work were not done? No: a resolver that indexes `w:abstractNum` by its
/// position in the part (rather than by `abstractNumId`) or `w:num` by its position (rather than by
/// `numId`) reads the wrong abstract definition or instance the moment the file's own document order
/// disagrees with numeric order — which `numbering_definitions.docx` is built to do (`numId` 5 is
/// written before `numId` 2, and `numId` 9 is present but unrelated).
#[test]
fn the_two_hop_resolution_reads_the_correct_level_through_a_shared_abstract_definition() {
    let mut document = Document::open(&fixture("numbering_definitions.docx")).expect("open");
    let found = document
        .numbering(|numbering, interner| {
            let index = NumberingIndex::build(numbering, interner).expect("build index");

            let NumberingLookup::Resolved(resolution) = index
                .resolve(5, 0, interner)
                .expect("resolve numId 5, ilvl 0")
            else {
                panic!("numId 5 is not 0 and must resolve");
            };
            assert_eq!(resolution.instance().numbering_id(interner), Ok(5));
            assert_eq!(
                resolution
                    .abstract_definition()
                    .abstract_numbering_id(interner),
                Ok(0)
            );
            let level = resolution.level().expect("abstractNum 0 defines ilvl 0");
            assert_eq!(
                level
                    .text_template()
                    .and_then(|t| t.raw(interner).ok().flatten())
                    .as_deref(),
                Some("%1.")
            );

            let NumberingLookup::Resolved(resolution) = index
                .resolve(5, 1, interner)
                .expect("resolve numId 5, ilvl 1")
            else {
                panic!("numId 5 is not 0 and must resolve");
            };
            let level = resolution.level().expect("abstractNum 0 defines ilvl 1");
            assert_eq!(
                level
                    .text_template()
                    .and_then(|t| t.raw(interner).ok().flatten())
                    .as_deref(),
                Some("%2)")
            );
        })
        .expect("read word/numbering.xml");
    assert!(
        found.is_some(),
        "the fixture must relate to word/numbering.xml"
    );
}

/// The Done-when's own trap, proved and mutation-checked. `numId` 2 and `numId` 5 share
/// `abstractNumId` 0; only `numId` 2 carries a `w:startOverride` at `ilvl` 0. See this test's own
/// mutation proof recorded in the child's PR/comment: neutralising
/// `NumberingIndex::resolve`'s override handling (reading `abstract_definition.level(...)`'s own
/// `w:start` unconditionally, ignoring `w:lvlOverride`) turns this test red — both instances would
/// then read `effective_start() == Some(1)`.
#[test]
fn a_start_override_changes_only_the_overriding_instance_not_its_sibling() {
    let mut document = Document::open(&fixture("numbering_definitions.docx")).expect("open");
    document
        .numbering(|numbering, interner| {
            let index = NumberingIndex::build(numbering, interner).expect("build index");

            let NumberingLookup::Resolved(overridden) = index
                .resolve(2, 0, interner)
                .expect("resolve numId 2, ilvl 0")
            else {
                panic!("numId 2 is not 0 and must resolve");
            };
            assert_eq!(
                overridden.effective_start(),
                Some(5),
                "numId 2's own w:lvlOverride/w:startOverride must win"
            );

            let NumberingLookup::Resolved(sibling) = index
                .resolve(5, 0, interner)
                .expect("resolve numId 5, ilvl 0")
            else {
                panic!("numId 5 is not 0 and must resolve");
            };
            assert_eq!(
                sibling.effective_start(),
                Some(1),
                "numId 5 shares the same abstractNum but carries no override of its own — it must \
                 read the abstract definition's own w:start, untouched by numId 2's override"
            );

            // Both share the exact same abstract definition and, absent the override, the exact
            // same level object.
            assert_eq!(
                overridden
                    .abstract_definition()
                    .abstract_numbering_id(interner),
                sibling
                    .abstract_definition()
                    .abstract_numbering_id(interner)
            );
        })
        .expect("read word/numbering.xml");
}

// -------------------------------------------------------------------------------------------
// numId = 0 versus a genuinely dangling numId.
// -------------------------------------------------------------------------------------------

/// Would this pass if the work were not done? No: a resolver that treats every non-matching `numId`
/// (including `0`) as [`DocxError::UnknownNumberingId`] fails this test — `0` must be
/// [`NumberingLookup::None`], not an error.
#[test]
fn numid_zero_resolves_to_no_numbering_never_a_lookup_failure() {
    let mut document = Document::open(&fixture("numbering_definitions.docx")).expect("open");
    document
        .numbering(|numbering, interner| {
            let index = NumberingIndex::build(numbering, interner).expect("build index");
            assert_eq!(
                index.resolve(0, 0, interner).expect("numId 0 never errors"),
                NumberingLookup::None
            );
        })
        .expect("read word/numbering.xml");

    // The same distinction at the `Document` level, which does not even require
    // `word/numbering.xml` to be related — proved against a document with no numbering.xml at all.
    // Assertion happens inside the closure: `NumberingLookup<'_>` borrows from data parsed inside
    // `resolve_numbering`'s own call, so it cannot be returned out through `R` (the same reason
    // `Document::numbering`/`edit_style_sheet` are callback-shaped rather than reference-returning).
    let mut blank = Document::blank(PageSize::a4()).expect("blank");
    blank
        .resolve_numbering(0, 0, |lookup, _interner| {
            assert_eq!(*lookup, NumberingLookup::None);
        })
        .expect("numId 0 never errors, even with no word/numbering.xml related");
}

/// A synthetic, in-memory proof that a `numId` naming no `w:num` is
/// [`DocxError::UnknownNumberingId`] — never a panic — independent of any file.
///
/// Would this pass if the work were not done? No: an unwrap/expect on the lookup (rather than a typed
/// `Option`/`Result` chain) panics here instead of returning `Err`.
#[test]
fn a_numid_naming_no_num_is_a_typed_error_synthetic() {
    let mut interner = Interner::default();
    let mut numbering = mjx_docx::Numbering::new(&mut interner);
    numbering.push_abstract_numbering(AbstractNumbering::new(&mut interner, 0));
    numbering.push_instance(NumberingInstance::new(&mut interner, 1, 0));

    let index = NumberingIndex::build(&numbering, &interner).expect("build index");
    let error = index
        .resolve(42, 0, &interner)
        .expect_err("numId 42 is not defined");
    assert!(matches!(error, DocxError::UnknownNumberingId(42)));
}

/// The real-corpus half of the same proof: `paragraph_properties.docx` (MJXOFF-96) carries a genuine
/// `w:numPr` (`ilvl = 1`, `numId = 5` — confirmed directly against its own bytes by
/// `tests/paragraph_properties.rs`'s own `every_ct_pprbase_member_reads_back_from_the_seeded_fixture`)
/// while relating to **no** `word/numbering.xml` at all. Resolving that real, already-committed
/// reference is a typed error end to end, never a panic.
///
/// Would this pass if the work were not done? No: a `resolve_numbering` that `.expect()`s a numbering
/// part to exist, or that treats "no numbering.xml" the same as `numId = 0` ("no numbering"), either
/// panics or silently returns [`NumberingLookup::None`] for a paragraph that plainly claims a list —
/// this test fails either way.
#[test]
fn paragraph_properties_docx_dangling_numid_is_a_typed_error_never_a_panic() {
    let bytes = fixture("paragraph_properties.docx");

    // First, confirm the real defect directly against the fixture's own bytes (no numbering.xml
    // relationship at all), the same low-level path `tests/paragraph_properties.rs` uses.
    let mut package = Package::open(&bytes).expect("open paragraph_properties.docx");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let main = MainDocument::from_xml(&doc.root, &doc.interner).expect("parse w:document");
    let body = main.body().expect("the fixture has a body");
    let paragraph = body.paragraph(0).expect("the first paragraph");
    let numbering_ref = paragraph
        .properties()
        .expect("carries w:pPr")
        .numbering()
        .expect("carries w:numPr");
    let numbering_id = numbering_ref
        .numbering_id(&doc.interner)
        .expect("valid w:numId")
        .expect("w:numId is present");
    let level = numbering_ref
        .level(&doc.interner)
        .expect("valid w:ilvl")
        .expect("w:ilvl is present");
    assert_eq!(numbering_id, 5, "the fixture's own dangling numId");

    let mut document = Document::open(&bytes).expect("open");
    assert!(
        document.parts().numbering.is_none(),
        "paragraph_properties.docx must relate to no word/numbering.xml — the defect this test \
         exists to prove is real"
    );
    assert!(document
        .numbering(|_, _| ())
        .expect("no numbering.xml is not itself an error")
        .is_none());

    let error = document
        .resolve_numbering(numbering_id, level, |_lookup, _interner| ())
        .expect_err(
            "a numId naming no w:num (because there is no numbering.xml at all) is a \
                     typed error",
        );
    assert!(matches!(error, DocxError::UnknownNumberingId(5)));
}

// -------------------------------------------------------------------------------------------
// Style-linked numbering: w:numStyleLink resolves through StyleIndex.
// -------------------------------------------------------------------------------------------

/// Would this pass if the work were not done? No: a resolver that reads `numId` 9's own abstract
/// definition's (empty) `w:lvl` list directly — skipping the `w:numStyleLink` redirect through
/// `word/styles.xml`'s `"ListStyleLink"` style — returns `level: None` instead of the real level 0
/// text `"%1."` that lives on `numId` 5.
#[test]
fn num_style_link_resolves_through_the_style_index_to_the_real_levels() {
    let mut document = Document::open(&fixture("numbering_definitions.docx")).expect("open");
    let outcome = document
        .resolve_numbering(9, 0, |lookup, interner| match lookup {
            NumberingLookup::Resolved(resolution) => {
                let level = resolution
                    .level()
                    .expect("redirected to numId 5's own level 0");
                (
                    resolution.instance().numbering_id(interner),
                    level
                        .text_template()
                        .and_then(|t| t.raw(interner).ok().flatten())
                        .map(std::borrow::Cow::into_owned),
                )
            }
            NumberingLookup::None => panic!("numId 9 is not 0 and must resolve"),
        })
        .expect("resolve numId 9 through its w:numStyleLink redirect");
    assert_eq!(outcome, (Ok(5), Some("%1.".to_owned())));
}

/// The bounded-depth guard on a genuine `w:numStyleLink` cycle: a numbering style whose own
/// `w:numId` points back at the very instance whose abstract definition links to that style.
///
/// Would this pass if the work were not done? No: an unbounded redirect loop hangs instead of
/// returning [`DocxError::NumberingStyleLinkTooDeep`].
#[test]
fn a_num_style_link_cycle_is_bounded_and_reported_as_a_typed_error_not_a_hang() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document
        .edit_numbering(|numbering, interner| {
            let mut cyclic = AbstractNumbering::new(interner, 0);
            cyclic.set_numbering_style_link(Some(StyleString::new(
                interner,
                "numStyleLink",
                "CycleStyle",
            )));
            numbering.push_abstract_numbering(cyclic);
            numbering.push_instance(NumberingInstance::new(interner, 20, 0));
        })
        .expect("edit numbering");
    document
        .edit_style_sheet(|sheet, interner| {
            let mut style = StyleDefinition::new(interner, StyleType::Numbering, "CycleStyle");
            let mut redirect = NumberingProperties::new(interner);
            redirect.set_numbering_id(interner, Some(20));
            style
                .paragraph_properties_or_insert(interner)
                .set_numbering(Some(redirect));
            sheet.add_style(style);
        })
        .expect("edit styles");

    let error = document
        .resolve_numbering(20, 0, |_lookup, _| ())
        .expect_err("a self-referencing w:numStyleLink chain must not resolve");
    assert!(matches!(
        error,
        DocxError::NumberingStyleLinkTooDeep {
            numbering_id: 20,
            limit,
        } if limit == MAX_NUM_STYLE_LINK_DEPTH
    ));
}

/// `w:numStyleLink` naming a `styleId` the style sheet does not define is a typed defect, not a
/// panic — the same class [`mjx_docx::LinkedStyleResolution::TargetMissing`] reports for `w:link`.
#[test]
fn a_num_style_link_naming_no_style_is_a_typed_error() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document
        .edit_numbering(|numbering, interner| {
            let mut orphaned = AbstractNumbering::new(interner, 0);
            orphaned.set_numbering_style_link(Some(StyleString::new(
                interner,
                "numStyleLink",
                "NoSuchStyle",
            )));
            numbering.push_abstract_numbering(orphaned);
            numbering.push_instance(NumberingInstance::new(interner, 1, 0));
        })
        .expect("edit numbering");

    let error = document
        .resolve_numbering(1, 0, |_lookup, _| ())
        .expect_err(
            "NoSuchStyle is not defined anywhere, including in a document with no \
                     word/styles.xml at all",
        );
    assert!(matches!(
        error,
        DocxError::NumberingStyleLinkTargetMissing { style_id } if style_id == "NoSuchStyle"
    ));
}

// -------------------------------------------------------------------------------------------
// LevelTextTemplate's %1-%9 placeholder grammar.
// -------------------------------------------------------------------------------------------

#[test]
fn level_text_placeholder_grammar_distinguishes_multi_level_labels_from_bullet_glyphs() {
    let mut interner = Interner::default();

    let multi_level = LevelTextTemplate::new(&mut interner, "%1.%2.%3.");
    assert_eq!(
        multi_level.segments(&interner).expect("valid"),
        Some(vec![
            LevelTextSegment::Level(1),
            LevelTextSegment::Literal(".".to_owned()),
            LevelTextSegment::Level(2),
            LevelTextSegment::Literal(".".to_owned()),
            LevelTextSegment::Level(3),
            LevelTextSegment::Literal(".".to_owned()),
        ])
    );

    let bullet = LevelTextTemplate::new(&mut interner, "\u{f0b7}");
    assert_eq!(
        bullet.segments(&interner).expect("valid"),
        Some(vec![LevelTextSegment::Literal("\u{f0b7}".to_owned())]),
        "a bullet glyph carries no %-placeholder at all"
    );

    // A trailing lone '%' and a '%0' (0 is not 1-9) are both copied through literally, never
    // rejected — untrusted input is never a panic here.
    let edge_cases = LevelTextTemplate::new(&mut interner, "%0 trailing %");
    assert_eq!(
        edge_cases.segments(&interner).expect("valid"),
        Some(vec![LevelTextSegment::Literal("%0 trailing %".to_owned())])
    );

    let mut absent = LevelTextTemplate::new(&mut interner, "placeholder");
    absent.set_raw(&mut interner, None);
    assert_eq!(
        absent.segments(&interner).expect("valid"),
        None,
        "an absent w:val must parse to None, never Some(vec![])"
    );
}

// -------------------------------------------------------------------------------------------
// Round-trip and authoring.
// -------------------------------------------------------------------------------------------

/// Would this pass if the work were not done? No: a `Numbering::write_back` that rebuilds every
/// element from the model instead of preserving unedited spans would still often *look* identical
/// for simple content, but this is the same discriminating shape `styles.rs`'s own equivalent test
/// uses — a no-op edit forces the real decode/typed-model/encode path, not the OPC copy-on-write
/// layer's own untouched-part fast path.
#[test]
fn numbering_xml_round_trips_byte_identically_through_a_no_op_edit() {
    let original = fixture("numbering_definitions.docx");
    let mut document = Document::open(&original).expect("open");
    document
        .edit_numbering(|_numbering, _interner| {})
        .expect("materialize word/numbering.xml through the typed model with a no-op edit");
    let saved = document.save().expect("save");

    let original_pkg = Package::open(&original).expect("open original");
    let saved_pkg = Package::open(&saved).expect("open saved");
    let part = PartName::new("/word/numbering.xml").expect("valid part name");
    let original_numbering = original_pkg
        .part_bytes(&part)
        .expect("original has numbering.xml");
    let saved_numbering = saved_pkg
        .part_bytes(&part)
        .expect("saved has numbering.xml");
    assert_eq!(
        original_numbering, saved_numbering,
        "word/numbering.xml must round-trip byte-identically when nothing dirtied it"
    );
}

/// Editing `word/numbering.xml` must not disturb any other part.
#[test]
fn editing_numbering_leaves_every_other_part_untouched() {
    let original = fixture("numbering_definitions.docx");
    let mut document = Document::open(&original).expect("open");
    document
        .edit_numbering(|numbering, interner| {
            let mut extra = NumberingInstance::new(interner, 100, 0);
            let mut override_level = NumberingLevelOverride::new(interner, 0);
            override_level.set_start_override(interner, Some(42));
            extra.push_level_override(override_level);
            numbering.push_instance(extra);
        })
        .expect("add a num instance");
    let saved = document.save().expect("save");

    let original_pkg = Package::open(&original).expect("open original");
    let saved_pkg = Package::open(&saved).expect("open saved");
    for part in [
        "/word/document.xml",
        "/word/styles.xml",
        "/docProps/core.xml",
    ] {
        let part_name = PartName::new(part).expect("valid part name");
        assert_eq!(
            original_pkg.part_bytes(&part_name),
            saved_pkg.part_bytes(&part_name),
            "{part} must be untouched by an edit scoped to word/numbering.xml"
        );
    }

    // And the new instance really did land, at its resolvable id.
    let mut reopened = Document::open(&saved).expect("reopen");
    let found = reopened
        .numbering(|numbering, interner| {
            let index = NumberingIndex::build(numbering, interner).expect("build index");
            let NumberingLookup::Resolved(resolution) = index
                .resolve(100, 0, interner)
                .expect("resolve the new instance")
            else {
                panic!("numId 100 is not 0 and must resolve");
            };
            assert_eq!(resolution.effective_start(), Some(42));
        })
        .expect("read word/numbering.xml");
    assert!(found.is_some());
}

/// Authoring `word/numbering.xml` into a document that starts with none: the relationship, the
/// content type, and the resulting document are all schema-valid under the MJXOFF-90 gate.
#[test]
fn adding_numbering_to_a_document_with_none_produces_a_valid_part_type_and_relationship() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    assert!(
        document.parts().numbering.is_none(),
        "a blank document starts with none"
    );

    document
        .edit_numbering(|numbering, interner| {
            let mut abstract_def = AbstractNumbering::new(interner, 0);
            let mut level = NumberingLevel::new(interner, 0);
            level.set_start(interner, Some(1));
            level.set_format(Some(mjx_docx::LevelNumberFormat::new(
                interner,
                NumberFormat::Decimal,
            )));
            level.set_text_template(Some(LevelTextTemplate::new(interner, "%1.")));
            abstract_def.push_level(level);
            numbering.push_abstract_numbering(abstract_def);
            numbering.push_instance(NumberingInstance::new(interner, 1, 0));
        })
        .expect("author a fresh numbering definition");
    assert!(document.parts().numbering.is_some());

    document
        .attach_paragraph_to_list(0, 1, 0)
        .expect("attach the blank paragraph");

    let saved = document.save().expect("save");
    assert_authored_deck_is_schema_valid("blank document with an authored numbering.xml", &saved);

    let package = Package::open(&saved).expect("reopen");
    let numbering_part = PartName::new("/word/numbering.xml").expect("valid part name");
    let content_type = package
        .content_type_of(&numbering_part)
        .expect("word/numbering.xml has a registered content type");
    assert_eq!(
        content_type,
        mjx_docx::constants::CONTENT_TYPE_NUMBERING,
        "the numbering part must carry the numbering content type, not e.g. styles'"
    );
    let document_part = PartName::new("/word/document.xml").expect("valid part name");
    let rels = package
        .relationships_for(Some(&document_part))
        .expect("word/document.xml has relationships");
    assert!(
        rels.by_type(mjx_docx::constants::REL_NUMBERING)
            .next()
            .is_some(),
        "word/document.xml must relate to word/numbering.xml via the numbering relationship type"
    );
}

/// `Document::attach_paragraph_to_list`/`detach_paragraph_from_list` round-trip through the real
/// `w:numPr` model, and detaching removes it entirely rather than leaving an empty shell.
#[test]
fn attach_and_detach_paragraph_from_list_round_trip() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document.attach_paragraph_to_list(0, 7, 2).expect("attach");

    let saved = document.save().expect("save");
    let mut package = Package::open(&saved).expect("reopen");
    let part = PartName::new("/word/document.xml").expect("valid part name");
    let doc = package.part_tree(&part).expect("read");
    let main = MainDocument::from_xml(&doc.root, &doc.interner).expect("parse");
    let body = main.body().expect("body");
    let paragraph = body.paragraph(0).expect("paragraph 0");
    let numbering_ref = paragraph
        .properties()
        .expect("carries w:pPr")
        .numbering()
        .expect("carries w:numPr");
    assert_eq!(numbering_ref.numbering_id(&doc.interner), Ok(Some(7)));
    assert_eq!(numbering_ref.level(&doc.interner), Ok(Some(2)));

    let mut document = Document::open(&saved).expect("reopen as Document");
    document.detach_paragraph_from_list(0).expect("detach");
    let saved_again = document.save().expect("save");
    let mut package = Package::open(&saved_again).expect("reopen");
    let doc = package.part_tree(&part).expect("read");
    let main = MainDocument::from_xml(&doc.root, &doc.interner).expect("parse");
    let body = main.body().expect("body");
    let paragraph = body.paragraph(0).expect("paragraph 0");
    assert!(
        paragraph
            .properties()
            .is_none_or(|p| p.numbering().is_none()),
        "detach must remove w:numPr entirely"
    );
}
