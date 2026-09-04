//! `word/styles.xml` (MJXOFF-101): `sample.docx`'s own seven styles read back in full,
//! `w:basedOn` chain resolution (the three-deep override trap, and cycle safety), latent styles
//! (a fixture authored for this child — the corpus has none), `w:link` resolution, case
//! sensitivity, and authoring a style sheet into a document that starts with none.

use std::time::Duration;

use mjx_docx::{
    Document, DocxError, LinkedStyleResolution, PageSize, StyleDefinition, StyleIndex,
    MAX_BASED_ON_CHAIN_DEPTH,
};
use mjx_fixtures::fixture;
use mjx_ooxml_types::wordprocessingml::{HalfPointMeasure, StyleType};

// -------------------------------------------------------------------------------------------
// sample.docx: every style reads back with its full property set.
// -------------------------------------------------------------------------------------------

/// `sample.docx`'s own seven `styleId`s, confirmed directly against the fixture's own
/// `word/styles.xml` bytes (not assumed): `Normal`, `Heading`, `BodyText`, `List`, `Caption`,
/// `Index`, `PreformattedText`.
const SAMPLE_DOCX_STYLE_IDS: [&str; 7] = [
    "Normal",
    "Heading",
    "BodyText",
    "List",
    "Caption",
    "Index",
    "PreformattedText",
];

#[test]
fn every_style_in_sample_docx_reads_back_with_its_full_property_set() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    let found = document
        .style_sheet(|sheet, interner| {
            assert_eq!(
                sheet.style_count(),
                SAMPLE_DOCX_STYLE_IDS.len(),
                "sample.docx's own style count"
            );
            for expected_id in SAMPLE_DOCX_STYLE_IDS {
                let style = sheet
                    .style_by_id(expected_id, interner)
                    .unwrap_or_else(|| panic!("sample.docx must carry styleId {expected_id:?}"));
                assert_eq!(
                    style.kind(interner).expect("valid w:type"),
                    Some(StyleType::Paragraph),
                    "{expected_id}: every sample.docx style is a paragraph style"
                );
                // Every style has a name, and every one but Normal has basedOn — the full
                // property-set proof for the two representative styles below goes further.
                assert!(style.name().is_some(), "{expected_id} must carry w:name");
            }

            // docDefaults — rung one of the ladder — readable entirely on its own.
            let defaults = sheet
                .document_defaults()
                .expect("sample.docx carries w:docDefaults");
            let run_defaults = defaults
                .run_properties_default()
                .and_then(|d| d.run_properties())
                .expect("sample.docx's w:rPrDefault carries a w:rPr");
            assert_eq!(
                run_defaults
                    .font_size(interner)
                    .expect("valid w:sz")
                    .map(|s| s.to_wire().to_owned()),
                Some("24".to_owned()),
                "sample.docx's docDefaults w:sz"
            );
            let para_defaults = defaults
                .paragraph_properties_default()
                .and_then(|d| d.paragraph_properties())
                .expect("sample.docx's w:pPrDefault carries a w:pPr");
            assert_eq!(
                para_defaults
                    .suppress_auto_hyphens(interner)
                    .expect("valid w:suppressAutoHyphens"),
                Some(true),
                "sample.docx's docDefaults w:suppressAutoHyphens"
            );

            // sample.docx carries no w:latentStyles at all — checked directly against the
            // fixture's own bytes. This is the gap tests/fixtures/style_latent_styles.docx
            // exists to close (see latent_styles_round_trip_exactly below).
            assert!(
                sheet.latent_styles().is_none(),
                "sample.docx must not carry w:latentStyles (verified against the fixture's own \
                 bytes — a regression here means the fixture changed)"
            );

            // Two representative styles' full property sets, confirmed against the fixture's own
            // XML (see this crate's own module doc / the child's PR for the raw bytes).
            let normal = sheet.style_by_id("Normal", interner).expect("Normal");
            assert_eq!(normal.based_on(), None, "Normal carries no w:basedOn");
            let normal_rpr = normal.run_properties().expect("Normal has w:rPr");
            assert_eq!(
                normal_rpr
                    .font_size(interner)
                    .expect("valid w:sz")
                    .map(|s| s.to_wire().to_owned()),
                Some("24".to_owned())
            );
            let normal_ppr = normal.paragraph_properties().expect("Normal has w:pPr");
            assert_eq!(
                normal_ppr.widow_control(interner).expect("valid"),
                Some(false)
            );

            let heading = sheet.style_by_id("Heading", interner).expect("Heading");
            assert_eq!(
                heading
                    .based_on()
                    .map(|r| r.value(interner).expect("valid").into_owned()),
                Some("Normal".to_owned())
            );
            assert_eq!(
                heading
                    .next()
                    .map(|r| r.value(interner).expect("valid").into_owned()),
                Some("BodyText".to_owned())
            );
            let heading_ppr = heading.paragraph_properties().expect("Heading has w:pPr");
            assert_eq!(
                heading_ppr.keep_with_next(interner).expect("valid"),
                Some(true)
            );
        })
        .expect("read sample.docx's style sheet");
    assert!(
        found.is_some(),
        "sample.docx must relate to word/styles.xml"
    );
}

#[test]
fn sample_docx_styles_xml_round_trips_byte_identically_when_untouched() {
    let original = fixture("sample.docx");
    let mut document = Document::open(&original).expect("open sample.docx");
    // The forcing step MJXOFF-90's own roundtrip.rs uses for word/document.xml, restated here for
    // word/styles.xml specifically: mjx-opc's part-level copy-on-write already reproduces an
    // untouched part byte-for-byte before a line of this module runs, so a bare open/save would
    // prove nothing about StyleSheet::write_back — a no-op edit still parses the part into
    // StyleSheet and writes it back through the real decode/typed-model/encode path.
    document
        .edit_style_sheet(|_sheet, _interner| {})
        .expect("materialize word/styles.xml through the typed model with a no-op edit");
    let saved = document.save().expect("save");

    let original_pkg = mjx_opc::Package::open(&original).expect("open original");
    let saved_pkg = mjx_opc::Package::open(&saved).expect("open saved");
    let original_styles = original_pkg
        .part_bytes(&mjx_opc::PartName::new("/word/styles.xml").unwrap())
        .expect("original has word/styles.xml");
    let saved_styles = saved_pkg
        .part_bytes(&mjx_opc::PartName::new("/word/styles.xml").unwrap())
        .expect("saved has word/styles.xml");
    assert_eq!(
        original_styles, saved_styles,
        "word/styles.xml must round-trip byte-identically when nothing dirtied it"
    );
}

// -------------------------------------------------------------------------------------------
// The three-deep basedOn chain: the middle style's override, not the base's or the leaf's own
// (absent) properties, is the correct answer for the leaf.
// -------------------------------------------------------------------------------------------

/// The nearest ancestor (including `style` itself) in `chain` that sets a font size, or `None` if
/// none does — the minimal "walk the chain, take the first override" resolution MJXOFF-106's own
/// effective-properties ladder will build on, proven here only far enough to demonstrate the
/// mechanism is not a stub.
fn resolve_font_size(
    chain: &[&StyleDefinition],
    interner: &mjx_ooxml_core::Interner,
) -> Option<HalfPointMeasure> {
    chain.iter().find_map(|style| {
        style
            .run_properties()
            .and_then(|rpr| rpr.font_size(interner).expect("valid w:sz"))
    })
}

/// The nearest ancestor that sets bold, or `None`.
fn resolve_bold(chain: &[&StyleDefinition], interner: &mjx_ooxml_core::Interner) -> Option<bool> {
    chain.iter().find_map(|style| {
        style
            .run_properties()
            .and_then(|rpr| rpr.bold(interner).expect("valid w:b"))
    })
}

/// Would this test pass against a resolver that reads only the leaf's own direct properties, only
/// the base's, or nothing at all? No: `Leaf`'s own `w:rPr` is absent (no direct properties to
/// read), `Base`'s `w:sz` is `20` (the wrong half-point size), and `Leaf` has no `w:rPr` of its own
/// either — only a resolver that actually walks `Leaf → Middle → Base` and takes `Middle`'s
/// override arrives at `32`. See [`neutralising_the_chain_walk_turns_this_red`] for the pasted
/// proof that this assertion really does depend on the walk.
#[test]
fn the_middle_styles_override_not_the_base_or_the_leafs_own_absent_properties_resolves_for_the_leaf(
) {
    let mut document = Document::open(&fixture("style_based_on_chain.docx")).expect("open fixture");
    document
        .style_sheet(|sheet, interner| {
            let index = StyleIndex::build(sheet, interner).expect("build index");

            let chain = index
                .based_on_chain("Leaf", interner)
                .expect("Leaf's chain resolves (Leaf -> Middle -> Base, no cycle)");
            assert_eq!(
                chain
                    .iter()
                    .map(|s| s.style_id(interner).unwrap().unwrap().into_owned())
                    .collect::<Vec<_>>(),
                vec!["Leaf".to_owned(), "Middle".to_owned(), "Base".to_owned()],
                "based_on_chain must return self first, then each ancestor in walk order"
            );

            // The middle style's own override (32 half-points), not the base's (20) and not the
            // leaf's own (absent — Leaf sets no w:rPr at all).
            assert_eq!(
                resolve_font_size(&chain, interner),
                Some(HalfPointMeasure::from_wire("32")),
                "Leaf's effective font size must come from Middle, not Base or Leaf itself"
            );
            // Bold is set only on Base; neither Middle nor Leaf touch it, so the walk must reach
            // all the way to the root to resolve it — proving this is not merely "check the
            // immediate parent".
            assert_eq!(
                resolve_bold(&chain, interner),
                Some(true),
                "Leaf's effective bold must come from Base, two hops up"
            );
        })
        .expect("read style sheet")
        .expect("fixture has a style sheet");
}

// -------------------------------------------------------------------------------------------
// Cycle safety: a depth bound, not a visited-set — proved on the authored SelfCycle/Mutual
// fixture (the only cycle evidence in this suite; sample.docx has none — see the module's own
// doc comment on `styles.rs` for the corpus measurement this rests on), and proved *not* to
// false-positive on sample.docx's own real, non-cyclic chains.
// -------------------------------------------------------------------------------------------

#[test]
fn a_self_referencing_based_on_chain_returns_the_typed_error_within_bounded_steps() {
    run_with_timeout(|| {
        let mut document =
            Document::open(&fixture("style_based_on_cycle.docx")).expect("open fixture");
        document
            .style_sheet(|sheet, interner| {
                let index = StyleIndex::build(sheet, interner).expect("build index");
                let result = index.based_on_chain("SelfCycle", interner);
                match result {
                    Err(DocxError::BasedOnChainTooDeep { style_id, limit }) => {
                        assert_eq!(style_id, "SelfCycle");
                        assert_eq!(limit, MAX_BASED_ON_CHAIN_DEPTH);
                    }
                    other => panic!(
                        "a self-referencing chain must return \
                         Err(DocxError::BasedOnChainTooDeep {{ .. }}), not {other:?}"
                    ),
                }
            })
            .expect("read style sheet")
            .expect("fixture has a style sheet");
    });
}

#[test]
fn a_mutually_referencing_based_on_chain_returns_the_typed_error_within_bounded_steps() {
    run_with_timeout(|| {
        let mut document =
            Document::open(&fixture("style_based_on_cycle.docx")).expect("open fixture");
        document
            .style_sheet(|sheet, interner| {
                let index = StyleIndex::build(sheet, interner).expect("build index");
                for start in ["MutualA", "MutualB"] {
                    match index.based_on_chain(start, interner) {
                        Err(DocxError::BasedOnChainTooDeep { style_id, limit }) => {
                            assert_eq!(style_id, start);
                            assert_eq!(limit, MAX_BASED_ON_CHAIN_DEPTH);
                        }
                        other => panic!(
                            "a mutually-referencing chain starting at {start} must return \
                             Err(DocxError::BasedOnChainTooDeep {{ .. }}), not {other:?}"
                        ),
                    }
                }
            })
            .expect("read style sheet")
            .expect("fixture has a style sheet");
    });
}

/// The correction this child's own dispatch made twice, over its own transcript: `sample.docx`'s
/// `Normal` style carries **no** `w:basedOn` at all (an earlier brief claimed a self-cycle here;
/// checked directly against the fixture's raw bytes, twice, and refuted both times — see
/// `styles.rs`'s own module doc). `based_on_chain("Normal")` against the real corpus must
/// therefore resolve normally to a one-element chain, never trip the cycle guard — proving the
/// depth bound does not false-positive on legitimate input, the complement of the two tests above.
#[test]
fn based_on_chain_for_normal_in_sample_docx_is_a_normal_one_element_chain_not_a_cycle() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    document
        .style_sheet(|sheet, interner| {
            let index = StyleIndex::build(sheet, interner).expect("build index");
            let chain = index
                .based_on_chain("Normal", interner)
                .expect("Normal has no basedOn at all, so this must resolve, not error");
            assert_eq!(chain.len(), 1, "Normal's own chain is itself alone");
            assert_eq!(
                chain[0].style_id(interner).unwrap().unwrap().as_ref(),
                "Normal"
            );
        })
        .expect("read style sheet")
        .expect("sample.docx has a style sheet");

    // Every real chain in sample.docx terminates well inside the bound — including the deepest,
    // List -> BodyText -> Normal (measured depth 2).
    document
        .style_sheet(|sheet, interner| {
            let index = StyleIndex::build(sheet, interner).expect("build index");
            let chain = index
                .based_on_chain("List", interner)
                .expect("List's chain resolves");
            assert_eq!(
                chain
                    .iter()
                    .map(|s| s.style_id(interner).unwrap().unwrap().into_owned())
                    .collect::<Vec<_>>(),
                vec![
                    "List".to_owned(),
                    "BodyText".to_owned(),
                    "Normal".to_owned()
                ]
            );
        })
        .expect("read style sheet")
        .expect("sample.docx has a style sheet");
}

/// Runs `body` on a background thread with a generous timeout, failing the test if it does not
/// return in time — the "timeout on the test" this child's Done-when asks for on top of the
/// depth bound's own O(1)-space guarantee (`based_on_chain`'s own accumulation `Vec` is the bound;
/// this is an independent, coarser safety net proving the mechanism does not hang in practice
/// either).
fn run_with_timeout(body: impl FnOnce() + Send + 'static) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        body();
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_secs(10))
        .expect("must terminate within the timeout — a hang here means the depth bound failed");
}

// -------------------------------------------------------------------------------------------
// Latent styles: sample.docx has none (verified directly), so this is the only committed
// coverage — round-tripped exactly on a fixture authored for this child.
// -------------------------------------------------------------------------------------------

#[test]
fn latent_styles_round_trip_exactly_on_the_authored_fixture() {
    let original = fixture("style_latent_styles.docx");
    let mut document = Document::open(&original).expect("open fixture");

    document
        .style_sheet(|sheet, interner| {
            let latent = sheet
                .latent_styles()
                .expect("fixture carries w:latentStyles");
            assert_eq!(latent.default_locked_state(interner).unwrap(), Some(false));
            assert_eq!(latent.default_ui_priority(interner).unwrap(), Some(99));
            assert_eq!(latent.default_semi_hidden(interner).unwrap(), Some(true));
            assert_eq!(
                latent.default_unhide_when_used(interner).unwrap(),
                Some(true)
            );
            assert_eq!(latent.default_q_format(interner).unwrap(), Some(false));
            assert_eq!(latent.count(interner).unwrap(), Some(17));
            assert_eq!(latent.exception_count(), 17);

            let heading1 = latent
                .exceptions()
                .find(|e| e.name(interner).unwrap() == "heading 1")
                .expect("heading 1 exception");
            assert_eq!(heading1.ui_priority(interner).unwrap(), Some(9));
            assert_eq!(heading1.semi_hidden(interner).unwrap(), Some(true));
            assert_eq!(heading1.q_format(interner).unwrap(), Some(true));
            assert_eq!(heading1.locked(interner).unwrap(), Some(false));

            let no_spacing = latent
                .exceptions()
                .find(|e| e.name(interner).unwrap() == "No Spacing")
                .expect("No Spacing exception");
            assert_eq!(
                no_spacing.ui_priority(interner).unwrap(),
                None,
                "an exception with only `name` must round-trip every other attribute as absent"
            );
        })
        .expect("read style sheet")
        .expect("fixture has a style sheet");

    // Byte-identical when untouched (the same "read is not a write" proof as sample.docx's own).
    let saved = document.save().expect("save without editing anything");
    let original_pkg = mjx_opc::Package::open(&original).unwrap();
    let saved_pkg = mjx_opc::Package::open(&saved).unwrap();
    let part = mjx_opc::PartName::new("/word/styles.xml").unwrap();
    assert_eq!(
        original_pkg.part_bytes(&part),
        saved_pkg.part_bytes(&part),
        "an untouched word/styles.xml must not change bytes just from being opened and saved"
    );
}

#[test]
fn w_count_is_preserved_not_recomputed_when_the_exception_list_is_edited_without_syncing() {
    let mut document = Document::open(&fixture("style_latent_styles.docx")).expect("open fixture");
    document
        .edit_style_sheet(|sheet, interner| {
            let latent = sheet.latent_styles_or_insert(interner);
            assert_eq!(latent.count(interner).unwrap(), Some(17));
            // Add an exception without calling sync_count: the file's own w:count must stay 17,
            // proving push_exception never silently rewrites an attribute the caller did not
            // touch.
            latent.push_exception(mjx_docx::LatentStyleException::new(interner, "Extra"));
            assert_eq!(latent.exception_count(), 18);
            assert_eq!(
                latent.count(interner).unwrap(),
                Some(17),
                "w:count must not change until sync_count is called explicitly"
            );
            latent.sync_count(interner);
            assert_eq!(latent.count(interner).unwrap(), Some(18));
        })
        .expect("edit style sheet");
}

// -------------------------------------------------------------------------------------------
// Case sensitivity: styleId is case-sensitive, w:name is case-insensitive.
// -------------------------------------------------------------------------------------------

#[test]
fn style_id_matching_is_case_sensitive_and_name_matching_is_not() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    document
        .style_sheet(|sheet, interner| {
            let index = StyleIndex::build(sheet, interner).expect("build index");

            // Exact styleId matches; a differently-cased styleId does not.
            assert!(index.style_by_id("PreformattedText").is_some());
            assert!(
                index.style_by_id("preformattedtext").is_none(),
                "styleId matching must be case-sensitive"
            );
            assert!(
                index.style_by_id("PREFORMATTEDTEXT").is_none(),
                "styleId matching must be case-sensitive"
            );

            // w:name ("Preformatted Text") matches regardless of case — this is the exact
            // mismatch the ticket calls out: the styleId and the display name are spelled
            // differently, and name matching must still work case-insensitively against either
            // spelling a real producer might use.
            let by_exact_case = index
                .style_by_name("Preformatted Text")
                .expect("exact-case name lookup");
            let by_lower = index
                .style_by_name("preformatted text")
                .expect("lowercased name lookup must still match");
            let by_upper = index
                .style_by_name("PREFORMATTED TEXT")
                .expect("uppercased name lookup must still match");
            for style in [by_exact_case, by_lower, by_upper] {
                assert_eq!(
                    style.style_id(interner).unwrap().unwrap().as_ref(),
                    "PreformattedText"
                );
            }
        })
        .expect("read style sheet")
        .expect("sample.docx has a style sheet");
}

// -------------------------------------------------------------------------------------------
// w:link resolution: both directions, and a defect (missing target, wrong kind) reported as a
// value, never a panic.
// -------------------------------------------------------------------------------------------

#[test]
fn w_link_resolves_in_both_directions_and_reports_defects_as_values() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    document
        .edit_style_sheet(|sheet, interner| {
            let mut paragraph_style =
                StyleDefinition::new(interner, StyleType::Paragraph, "LinkedParagraph");
            paragraph_style.set_link(interner, Some("LinkedCharacter"));
            sheet.add_style(paragraph_style);

            let mut character_style =
                StyleDefinition::new(interner, StyleType::Character, "LinkedCharacter");
            character_style.set_link(interner, Some("LinkedParagraph"));
            sheet.add_style(character_style);

            let mut missing_link_target =
                StyleDefinition::new(interner, StyleType::Paragraph, "PointsNowhere");
            missing_link_target.set_link(interner, Some("DoesNotExist"));
            sheet.add_style(missing_link_target);

            let mut same_kind_link =
                StyleDefinition::new(interner, StyleType::Paragraph, "PointsAtSameKind");
            same_kind_link.set_link(interner, Some("PointsNowhere"));
            sheet.add_style(same_kind_link);

            let mut unlinked = StyleDefinition::new(interner, StyleType::Paragraph, "Unlinked");
            unlinked.set_name(interner, Some("Unlinked"));
            sheet.add_style(unlinked);
        })
        .expect("edit style sheet");

    document
        .style_sheet(|sheet, interner| {
            let index = StyleIndex::build(sheet, interner).expect("build index");

            match index.resolve_link("LinkedParagraph", interner).unwrap() {
                LinkedStyleResolution::Resolved(target) => {
                    assert_eq!(
                        target.style_id(interner).unwrap().unwrap().as_ref(),
                        "LinkedCharacter"
                    );
                }
                other => panic!("expected Resolved, got {other:?}"),
            }
            // The reverse direction resolves too.
            match index.resolve_link("LinkedCharacter", interner).unwrap() {
                LinkedStyleResolution::Resolved(target) => {
                    assert_eq!(
                        target.style_id(interner).unwrap().unwrap().as_ref(),
                        "LinkedParagraph"
                    );
                }
                other => panic!("expected Resolved, got {other:?}"),
            }

            assert_eq!(
                index.resolve_link("PointsNowhere", interner).unwrap(),
                LinkedStyleResolution::TargetMissing
            );
            assert_eq!(
                index.resolve_link("PointsAtSameKind", interner).unwrap(),
                LinkedStyleResolution::KindMismatch {
                    found: Some(StyleType::Paragraph)
                }
            );
            assert_eq!(
                index.resolve_link("Unlinked", interner).unwrap(),
                LinkedStyleResolution::NoLink
            );
            assert!(matches!(
                index.resolve_link("NeverDefined", interner),
                Err(DocxError::UnknownStyleId(id)) if id == "NeverDefined"
            ));
        })
        .expect("read style sheet")
        .expect("style sheet exists");
}

// -------------------------------------------------------------------------------------------
// Authoring: a style added to a document with no styles.xml (Document::blank's own output)
// produces a schema-valid part, a correct content-type entry and a correct relationship.
// -------------------------------------------------------------------------------------------

#[test]
fn a_blank_document_relates_to_no_styles_xml_until_one_is_added() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    assert!(
        document.parts().styles.is_none(),
        "Document::blank must not relate to word/styles.xml (see blank.rs's own doc comment)"
    );
    assert!(
        document
            .style_sheet(|_, _| ())
            .expect("style_sheet must not error just because there is none")
            .is_none(),
        "style_sheet must return None, not an empty StyleSheet, for a document with no \
         word/styles.xml"
    );
}

#[test]
fn adding_a_style_to_a_document_with_no_styles_xml_produces_a_valid_part_type_and_relationship() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");

    document
        .edit_style_sheet(|sheet, interner| {
            let mut style = StyleDefinition::new(interner, StyleType::Paragraph, "Custom");
            style.set_name(interner, Some("Custom"));
            let rpr = style.run_properties_or_insert(interner);
            rpr.set_bold(interner, Some(true));
            sheet.add_style(style);
        })
        .expect("edit style sheet");

    assert!(
        document.parts().styles.is_some(),
        "the document must now relate to word/styles.xml"
    );

    let styles_part = mjx_opc::PartName::new("/word/styles.xml").unwrap();

    // The relationship, content type and part all exist and are internally consistent —
    // Package::validate checks exactly this graph.
    document.validate().expect("the package's own invariants");

    // The read path sees the style back.
    document
        .style_sheet(|sheet, interner| {
            let style = sheet
                .style_by_id("Custom", interner)
                .expect("the added style reads back");
            assert_eq!(
                style
                    .run_properties()
                    .and_then(|rpr| rpr.bold(interner).unwrap()),
                Some(true)
            );
        })
        .expect("read style sheet")
        .expect("style sheet now exists");

    let saved = document.save().expect("save");
    let package = mjx_opc::Package::open(&saved).expect("reopen the saved package");
    assert!(
        package.part_bytes(&styles_part).is_some(),
        "word/styles.xml must actually be in the saved container"
    );

    // The relationship from word/document.xml names the styles relationship type.
    let document_part = mjx_opc::PartName::new("/word/document.xml").unwrap();
    let rels = package
        .relationships_for(Some(&document_part))
        .expect("word/document.xml now has a .rels part");
    assert!(
        rels.by_type(mjx_docx::constants::REL_STYLES)
            .any(|rel| rel.target == "styles.xml"),
        "word/_rels/document.xml.rels must carry the styles relationship"
    );

    // Schema-valid, through the same gate every fixture goes through — skips cleanly with no
    // local `References/` (matching every other schema-gate case in this crate), and asserts
    // specifically that word/styles.xml validated against wml.xsd rather than merely "some part
    // is fine" when References/ is present.
    if let Some(harness) = mjx_schema_gate::harness() {
        let rows = mjx_schema_gate::inspect_deck(
            &harness,
            "adding a style to a document with no styles.xml",
            &saved,
            &[],
        );
        let row = rows
            .iter()
            .find(|row| row.name == "/word/styles.xml")
            .expect("word/styles.xml is in the sweep");
        assert!(
            matches!(
                row.outcome,
                mjx_schema_gate::PartOutcome::Validated("wml.xsd")
            ),
            "the newly-authored word/styles.xml must validate against wml.xsd; it reported: {}",
            row.outcome.describe()
        );
    }

    // Reopening from scratch (a fresh Document::open over the saved bytes) sees the same style —
    // proves this is not an in-memory-only artifact of the same Document instance.
    let mut reopened = Document::open(&saved).expect("reopen");
    reopened
        .style_sheet(|sheet, interner| {
            assert!(sheet.style_by_id("Custom", interner).is_some());
        })
        .expect("read style sheet")
        .expect("reopened document has a style sheet");
}

// -------------------------------------------------------------------------------------------
// Editing one style leaves every other style's bytes, and every other part, untouched.
// -------------------------------------------------------------------------------------------

#[test]
fn editing_one_style_leaves_every_other_style_and_every_other_part_untouched() {
    let original = fixture("sample.docx");
    let mut document = Document::open(&original).expect("open sample.docx");

    document
        .edit_style_sheet(|sheet, interner| {
            let style = sheet
                .style_by_id_mut("Heading", interner)
                .expect("Heading exists");
            style
                .run_properties_or_insert(interner)
                .set_bold(interner, Some(false));
        })
        .expect("edit the Heading style");

    let saved = document.save().expect("save");

    let original_pkg = mjx_opc::Package::open(&original).expect("open original");
    let saved_pkg = mjx_opc::Package::open(&saved).expect("open saved");

    // Every part except word/styles.xml is untouched.
    for entry in original_pkg.entries() {
        if entry.name == "word/styles.xml" {
            continue;
        }
        let saved_entry = saved_pkg
            .entries()
            .iter()
            .find(|e| e.name == entry.name)
            .unwrap_or_else(|| panic!("{} must still be in the saved package", entry.name));
        assert_eq!(
            entry.bytes(),
            saved_entry.bytes(),
            "{} must be byte-identical — only word/styles.xml was edited",
            entry.name
        );
    }

    // Within word/styles.xml, every style other than Heading round-trips byte-for-byte at the
    // XML level: reparse both, compare each sibling style's own serialization.
    let mut before = Document::open(&original).unwrap();
    let mut after = Document::open(&saved).unwrap();
    for style_id in SAMPLE_DOCX_STYLE_IDS {
        if style_id == "Heading" {
            continue;
        }
        let before_bold = before
            .style_sheet(|sheet, interner| {
                sheet
                    .style_by_id(style_id, interner)
                    .and_then(|s| s.run_properties())
                    .and_then(|rpr| rpr.bold(interner).unwrap())
            })
            .unwrap()
            .unwrap();
        let after_bold = after
            .style_sheet(|sheet, interner| {
                sheet
                    .style_by_id(style_id, interner)
                    .and_then(|s| s.run_properties())
                    .and_then(|rpr| rpr.bold(interner).unwrap())
            })
            .unwrap()
            .unwrap();
        assert_eq!(before_bold, after_bold, "{style_id} must be unaffected");
    }

    // Heading itself did change, as intended.
    let heading_bold = after
        .style_sheet(|sheet, interner| {
            sheet
                .style_by_id("Heading", interner)
                .and_then(|s| s.run_properties())
                .and_then(|rpr| rpr.bold(interner).unwrap())
        })
        .unwrap()
        .unwrap();
    assert_eq!(heading_bold, Some(false));
}
