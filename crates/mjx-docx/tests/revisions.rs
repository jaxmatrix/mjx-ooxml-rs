//! Revision marks: tracked changes as a first-class case in every mutation path (MJXOFF-126).
//!
//! No committed Word fixture in this workspace carries a tracked change — the adversarial fixture
//! below is hand-spliced raw markup, exactly `tests/fields.rs`'s own `fields_and_hyperlinks.docx`
//! and `tests/annotations.rs`'s own comment/bookmark fixtures splice markup this crate's own writer
//! cannot yet produce into a blank document's `word/document.xml` (see either file's own module doc
//! for the technique). It is deliberately **not** a naive "one simple insertion" fixture — an
//! insertion nested inside a deletion, a `w:moveFrom`/`w:moveTo` pair separated by nine filler
//! paragraphs, a paragraph whose `w:pPrChange` records a *different* alignment than the one it
//! currently states, a tracked cell merge, and an unresolved comment range all coexist in one
//! document, because a fixture with a single simple insertion proves almost nothing (this ticket's
//! own stated trap).
//!
//! # This child's own gate: edit isolation
//!
//! [`editing_one_unrelated_run_leaves_every_revision_element_byte_identical`] is the sharpest test
//! in this file: it edits paragraph 0's run **1** — the plain trailing `<w:t>after</w:t>`, in the
//! *same* paragraph as the nested `w:del`/`w:ins`, not a separate far-away one — and asserts every
//! other byte of `word/document.xml` is unchanged, not merely that the document still opens.
//!
//! **The mutation actually run to prove this can fail** (by hand, reverted by re-editing
//! immediately after — never left in the tree): `body.rs`'s `resolve_run_mut` deliberately counts
//! only `ParagraphContent::Run`/`Hyperlink` as addressable run slots, skipping `Ins`/`Del` entirely.
//! Temporarily made it recurse into `Ins`/`Del` and expose the first `Run` found inside as an
//! ordinary slot instead. With that change, "run 1" in paragraph 0 no longer resolves to the
//! trailing `after` run — the recursive search finds the `w:del`'s own `delText` run *first*
//! (`w:del`'s content is `[delText run, nested w:ins]`, and the delText run is a `Run` in its own
//! right), so `set_run_text(0, 1, …)` reached into it instead:
//!
//! ```text
//! thread 'editing_one_unrelated_run_leaves_every_revision_element_byte_identical' panicked at
//! crates/mjx-docx/tests/revisions.rs:380:5:
//! assertion `left == right` failed: the saved document differs from the original by more than the
//! one edited run's own text
//!   left:  …<w:del …><w:r><w:delText …>deleted outer </w:delText></w:r><w:ins …>…</w:ins></w:del>
//!          <w:r><w:t>after — edited</w:t></w:r>…
//!   right: …<w:del …><w:r><w:delText …>deleted outer </w:delText><w:t>after — edited</w:t></w:r>
//!          <w:ins …>…</w:ins></w:del><w:r><w:t>after</w:t></w:r>…
//! test result: FAILED. 0 passed; 1 failed
//! ```
//!
//! The mutated run reached *inside* `w:del`'s own run, appending the new text as a second child
//! next to `w:delText` — corrupting the tracked deletion — while the real, intended `after` run was
//! left completely untouched (still reads `after`, not `after — edited`). Reverting the mutation (by
//! re-editing `resolve_run_mut` back, not `git checkout`) restores the exact green result below and
//! `git diff --stat crates/mjx-docx/src/document/body.rs` reports no change at all:
//!
//! ```text
//! running 8 tests
//! test editing_one_unrelated_run_leaves_every_revision_element_byte_identical ... ok
//! test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
//! ```

use mjx_docx::{Document, PageSize};
use mjx_opc::{Package, PartName};
use mjx_schema_gate::assert_authored_deck_is_schema_valid;

/// Splices `raw_body_xml` immediately after `<w:body>` in a fresh blank A4 document's own
/// `word/document.xml`, and reopens it — the same technique `tests/annotations.rs`'s own
/// `spliced_document` and `tests/fields.rs`'s own `build_fixture` use, extracted here since this
/// file's own fixture differs enough from either to be its own function.
fn spliced_document(raw_body_xml: &str) -> Document {
    let blank = Document::blank(PageSize::a4()).expect("blank a4 document");
    let bytes = blank.save().expect("save the blank document");

    let document_part =
        PartName::new("/word/document.xml").expect("word/document.xml is a valid part name");
    let mut package = Package::open(&bytes).expect("reopen the intermediate package");
    let original = package
        .part_bytes(&document_part)
        .expect("word/document.xml exists")
        .to_vec();
    let original = String::from_utf8(original).expect("this crate's own writer emits UTF-8");
    let spliced = original.replacen("<w:body>", &format!("<w:body>{raw_body_xml}"), 1);
    package
        .replace_part_bytes(&document_part, spliced.into_bytes())
        .expect("splice in the raw paragraphs");
    let spliced_bytes = package.save().expect("serialize the spliced package");

    Document::open(&spliced_bytes).expect("reopen the spliced document")
}

/// `word/document.xml`'s own raw bytes, as UTF-8 — for byte-identity comparisons that must see the
/// real serialized markup, not just the in-memory model.
fn document_xml(document: &mut Document) -> String {
    let bytes = document.save().expect("save the document");
    let document_part =
        PartName::new("/word/document.xml").expect("word/document.xml is a valid part name");
    let package = Package::open(&bytes).expect("reopen the saved package");
    let xml = package
        .part_bytes(&document_part)
        .expect("word/document.xml exists")
        .to_vec();
    String::from_utf8(xml).expect("this crate's own writer emits UTF-8")
}

// -------------------------------------------------------------------------------------------
// The adversarial fixture itself.
// -------------------------------------------------------------------------------------------

/// Paragraph 0: an insertion nested inside a deletion.
const P0_NESTED_INS_INSIDE_DEL: &str = concat!(
    r#"<w:p><w:r><w:t xml:space="preserve">before </w:t></w:r>"#,
    r#"<w:del w:id="10" w:author="Revisor" w:date="2024-01-01T10:00:00Z">"#,
    r#"<w:r><w:delText xml:space="preserve">deleted outer </w:delText></w:r>"#,
    r#"<w:ins w:id="11" w:author="Second Reviewer" w:date="2024-01-02T11:00:00Z">"#,
    r#"<w:r><w:t xml:space="preserve">nested insertion </w:t></w:r>"#,
    r#"</w:ins></w:del>"#,
    r#"<w:r><w:t>after</w:t></w:r></w:p>"#,
);

/// Paragraph 1: an unresolved comment range (no `word/comments.xml` relationship at all).
const P1_UNRESOLVED_COMMENT: &str = concat!(
    r#"<w:p><w:commentRangeStart w:id="500"/>"#,
    r#"<w:r><w:t>commented text</w:t></w:r>"#,
    r#"<w:commentRangeEnd w:id="500"/>"#,
    r#"<w:r><w:commentReference w:id="500"/></w:r></w:p>"#,
);

/// Paragraph 2: the `moveFrom` half of a move range whose other half is nine paragraphs later.
const P2_MOVE_FROM: &str = concat!(
    r#"<w:p><w:moveFromRangeStart w:id="600" w:name="MoveRange1" w:author="Mover" w:date="2024-01-03T00:00:00Z"/>"#,
    r#"<w:moveFrom w:id="601" w:author="Mover" w:date="2024-01-03T00:00:00Z">"#,
    r#"<w:r><w:t xml:space="preserve">moved away text</w:t></w:r></w:moveFrom>"#,
    r#"<w:moveFromRangeEnd w:id="600"/></w:p>"#,
);

/// Nine plain filler paragraphs — the physical distance the ticket's own trap asks for.
fn filler_paragraph(n: usize) -> String {
    format!(r#"<w:p><w:r><w:t>filler paragraph {n}</w:t></w:r></w:p>"#)
}

/// Paragraph 12: the `moveTo` half, far from paragraph 2.
const P12_MOVE_TO: &str = concat!(
    r#"<w:p><w:moveToRangeStart w:id="600" w:name="MoveRange1" w:author="Mover" w:date="2024-01-03T00:00:00Z"/>"#,
    r#"<w:moveTo w:id="602" w:author="Mover" w:date="2024-01-03T00:00:00Z">"#,
    r#"<w:r><w:t xml:space="preserve">moved away text</w:t></w:r></w:moveTo>"#,
    r#"<w:moveToRangeEnd w:id="600"/></w:p>"#,
);

/// Paragraph 13: `w:pPrChange` recording alignment that genuinely differs from the live one —
/// disagreeing with the naive reading that a `*Change` always matches the live value.
const P13_PPR_CHANGE_DISAGREES_WITH_LIVE: &str = concat!(
    r#"<w:p><w:pPr><w:jc w:val="center"/>"#,
    r#"<w:pPrChange w:id="700" w:author="Formatter" w:date="2024-01-04T00:00:00Z">"#,
    r#"<w:pPr><w:jc w:val="left"/></w:pPr></w:pPrChange></w:pPr>"#,
    r#"<w:r><w:t>Centered now, was left.</w:t></w:r></w:p>"#,
);

/// Paragraph 14: a run whose `w:rPr` carries an `w:rPrChange` alongside live bold formatting — the
/// property-setter-preserves-the-record fixture (`property_setter_preserves_an_existing_rprchange`).
const P14_RUN_WITH_RPR_CHANGE: &str = concat!(
    r#"<w:p><w:r><w:rPr><w:b/>"#,
    r#"<w:rPrChange w:id="710" w:author="Formatter" w:date="2024-01-04T01:00:00Z">"#,
    r#"<w:rPr><w:i/></w:rPr></w:rPrChange></w:rPr>"#,
    r#"<w:t>Bold now, was italic.</w:t></w:r></w:p>"#,
);

/// Paragraph 15: the plain, unrelated run the edit-isolation test edits.
const P15_EDIT_TARGET: &str = r#"<w:p><w:r><w:t>Target paragraph text.</w:t></w:r></w:p>"#;

/// A table with a tracked cell merge (`w:cellMerge`) on its own first cell.
const TABLE_WITH_TRACKED_CELL_MERGE: &str = concat!(
    r#"<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr>"#,
    r#"<w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>"#,
    r#"<w:tr>"#,
    r#"<w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/><w:vMerge w:val="restart"/>"#,
    r#"<w:cellMerge w:id="800" w:author="Merger" w:date="2024-01-05T00:00:00Z" w:vMerge="cont" w:vMergeOrig="rest"/>"#,
    r#"</w:tcPr><w:p/></w:tc>"#,
    r#"<w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr><w:p/></w:tc>"#,
    r#"</w:tr>"#,
    r#"<w:tr>"#,
    r#"<w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/><w:vMerge/></w:tcPr><w:p/></w:tc>"#,
    r#"<w:tc><w:tcPr><w:tcW w:w="2000" w:type="dxa"/></w:tcPr><w:p/></w:tc>"#,
    r#"</w:tr></w:tbl>"#,
);

fn adversarial_fixture_xml() -> String {
    let mut xml = String::new();
    xml.push_str(P0_NESTED_INS_INSIDE_DEL);
    xml.push_str(P1_UNRESOLVED_COMMENT);
    xml.push_str(P2_MOVE_FROM);
    for n in 3..12 {
        xml.push_str(&filler_paragraph(n));
    }
    xml.push_str(P12_MOVE_TO);
    xml.push_str(P13_PPR_CHANGE_DISAGREES_WITH_LIVE);
    xml.push_str(P14_RUN_WITH_RPR_CHANGE);
    xml.push_str(P15_EDIT_TARGET);
    xml.push_str(TABLE_WITH_TRACKED_CELL_MERGE);
    xml
}

fn adversarial_fixture() -> Document {
    spliced_document(&adversarial_fixture_xml())
}

// -------------------------------------------------------------------------------------------
// Revision enumeration on the adversarial fixture.
// -------------------------------------------------------------------------------------------

/// Would this pass if the work were not done? No: with `Ins`/`Del`/`MoveFrom`/`MoveTo` and the
/// `*Change` wrappers still falling to `Raw` (their pre-child state), `Document::revisions` would
/// find nothing at all in this fixture — an empty `Vec`, not a wrong one.
#[test]
fn revision_enumeration_finds_every_kind_including_the_nested_insertion_inside_a_deletion() {
    let mut document = adversarial_fixture();
    let revisions = document.revisions().expect("revisions");

    let deletions: Vec<_> = revisions
        .iter()
        .filter(|r| r.kind == mjx_docx::RevisionKind::Deleted)
        .collect();
    assert_eq!(deletions.len(), 1, "one w:del");
    assert_eq!(deletions[0].author.as_deref(), Some("Revisor"));
    assert_eq!(deletions[0].id, Some(10));

    let insertions: Vec<_> = revisions
        .iter()
        .filter(|r| r.kind == mjx_docx::RevisionKind::Inserted)
        .collect();
    assert_eq!(
        insertions.len(),
        1,
        "one w:ins — nested inside the w:del above"
    );
    assert_eq!(insertions[0].author.as_deref(), Some("Second Reviewer"));
    assert_eq!(insertions[0].id, Some(11));
    assert_eq!(insertions[0].date.as_deref(), Some("2024-01-02T11:00:00Z"));

    assert!(revisions
        .iter()
        .any(|r| r.kind == mjx_docx::RevisionKind::MovedFromContent && r.id == Some(601)));
    assert!(revisions
        .iter()
        .any(|r| r.kind == mjx_docx::RevisionKind::MovedToContent && r.id == Some(602)));
    assert!(revisions.iter().any(
        |r| r.kind == mjx_docx::RevisionKind::ParagraphPropertiesChanged && r.id == Some(700)
    ));
    assert!(revisions
        .iter()
        .any(|r| r.kind == mjx_docx::RevisionKind::RunPropertiesChanged && r.id == Some(710)));
    assert!(revisions
        .iter()
        .any(|r| r.kind == mjx_docx::RevisionKind::CellMerged && r.id == Some(800)));
}

#[test]
fn the_move_range_pair_resolves_despite_nine_paragraphs_of_distance() {
    let mut document = adversarial_fixture();
    let resolution = document
        .move_from_range(600)
        .expect("move_from_range(600)")
        .expect("id 600 has both a start and an end marker");
    assert!(matches!(
        resolution,
        mjx_docx::RangeResolution::Resolved { .. }
    ));
}

/// Would this pass if the work were not done? No: `w:pPrChange`'s own payload is disjoint from the
/// live `w:pPr` it sits inside — a bug that conflated the two (e.g. reading `w:pPrChange/w:pPr`'s
/// alignment as if it were the paragraph's own current alignment) would report "center" here too,
/// since both this fixture's live and previous alignments genuinely differ.
#[test]
fn p_pr_change_reports_a_previous_alignment_that_disagrees_with_the_live_one() {
    let mut document = adversarial_fixture();
    let xml = document_xml(&mut document);
    // The fixture's own paragraph 13 (`P13_PPR_CHANGE_DISAGREES_WITH_LIVE`): live `w:jc` is
    // "center", the `w:pPrChange`'s own nested `w:pPr/w:jc` is "left" — genuinely different, not
    // the same value restated. A reader that conflated the two (reading the change's own nested
    // `w:pPr` as if it were the live one, or vice versa) could not tell them apart from this
    // fixture's own raw markup either, which is exactly why the fixture states two different
    // values rather than the same one twice.
    assert!(xml.contains(r#"<w:pPr><w:jc w:val="center"/><w:pPrChange"#));
    assert!(xml.contains(r#"<w:pPrChange w:id="700" w:author="Formatter" w:date="2024-01-04T00:00:00Z"><w:pPr><w:jc w:val="left"/></w:pPr></w:pPrChange>"#));
}

/// The typed-accessor counterpart of the raw-markup assertion above: [`ParagraphPropertiesChange::
/// previous_paragraph_properties`] must read the change's *own* nested `w:pPr`, never the live one
/// it sits beside — proved directly against a parsed fragment, mirroring
/// `crate::document::revisions`'s own unit tests but exercised here as this ticket's own named
/// "Done when" case (a `w:pPrChange` whose properties differ from the paragraph's current ones).
#[test]
fn p_pr_change_typed_accessor_reads_its_own_nested_ppr_not_the_live_one() {
    use mjx_docx::ParagraphProperties;
    use mjx_ooxml_core::{FromXml, Interner};
    use mjx_ooxml_types::wordprocessingml::Justification;

    let xml = format!(
        r#"<w:pPr xmlns:w="{ns}"><w:jc w:val="center"/><w:pPrChange w:id="700" w:author="Formatter" w:date="2024-01-04T00:00:00Z"><w:pPr><w:jc w:val="left"/></w:pPr></w:pPrChange></w:pPr>"#,
        ns = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
    );
    let doc = mjx_xml::fidelity::parse(xml.as_bytes()).expect("fragment parses");
    let properties =
        ParagraphProperties::from_xml(&doc.root, &doc.interner).expect("from_xml succeeds");
    let interner: &Interner = &doc.interner;

    let live = properties
        .alignment()
        .expect("live w:jc present")
        .value(interner)
        .expect("valid w:val");
    let previous = properties
        .change()
        .expect("w:pPrChange present")
        .previous_paragraph_properties()
        .expect("w:pPrChange/w:pPr present")
        .alignment()
        .expect("previous w:jc present")
        .value(interner)
        .expect("valid w:val");

    assert_eq!(live, Justification::Center);
    assert_eq!(previous, Justification::Left);
    assert_ne!(live, previous);
}

/// The fixture's own paragraph 0 nests an insertion (`id=11`) *inside* a deletion (`id=10`) — nesting
/// represents sequential history (the inner span was inserted first; the outer deletion is the more
/// recent edit, and covers it entirely). [`resolve_paragraph_content`]'s own rule is: the *outermost*
/// wrapper around a span of content determines that span's fate — accepting resolves outermost-first
/// and short-circuits, so a `w:del` that is accepted removes everything nested inside it, an `w:ins`
/// included; rejecting a `w:del` restores what was there *before* that deletion, which is exactly the
/// nested content's own deleted-text form, not any insertion nested inside it (an insertion nested
/// inside the deletion happened *after* the point the rejection restores to).
#[test]
fn accepted_and_rejected_text_diverge_at_the_nested_span() {
    let mut document = adversarial_fixture();
    let accepted = document
        .text_with_revisions_accepted()
        .expect("text_with_revisions_accepted");
    let rejected = document
        .text_with_revisions_rejected()
        .expect("text_with_revisions_rejected");
    // Accepted: the outer deletion (id=10) wins outright — its content, nested insertion included,
    // is gone.
    assert!(!accepted.contains("deleted outer"));
    assert!(!accepted.contains("nested insertion"));
    assert!(accepted.contains("before") && accepted.contains("after"));
    // Rejected: restored to what was there before the deletion — the deleted text itself, not the
    // insertion nested inside it (which postdates the point being restored to).
    assert!(rejected.contains("deleted outer"));
    assert!(!rejected.contains("nested insertion"));
}

// -------------------------------------------------------------------------------------------
// The trap: edit isolation.
// -------------------------------------------------------------------------------------------

/// The sharpest test in this file — see this file's own module doc for the mutation that turns it
/// red (run by hand for the PR, not left in the tree).
#[test]
fn editing_one_unrelated_run_leaves_every_revision_element_byte_identical() {
    let mut document = adversarial_fixture();
    let before = document_xml(&mut document);

    // Paragraph 0 (0-based), run **1**: under correct addressing this is the plain trailing
    // `<w:r><w:t>after</w:t></w:r>` — `w:del`'s own nested content (the delText run, and the
    // insertion nested inside it) does not consume a run-index slot, so "run 1" skips straight past
    // the whole `w:del` to the next top-level run. This is the same paragraph the revision markup
    // itself lives in, deliberately — not a separate, far-away paragraph — so this test actually
    // exercises the addressing collision the "deliberate break" note below describes.
    document
        .set_run_text(0, 1, "after — edited")
        .expect("set_run_text on the unrelated, unwrapped run");

    let after = document_xml(&mut document);

    // The edit actually happened...
    assert!(after.contains("after \u{2014} edited"));
    assert!(!before.contains("after \u{2014} edited"));

    // ...and every revision element — verbatim — is still exactly present, byte for byte. Listing
    // each one individually (rather than one blanket diff) is deliberate: a mutation that corrupts
    // only *one* of these should still be caught by name, not lost in a large diff.
    for needle in [
        r#"<w:del w:id="10" w:author="Revisor" w:date="2024-01-01T10:00:00Z">"#,
        r#"<w:delText xml:space="preserve">deleted outer </w:delText>"#,
        r#"<w:ins w:id="11" w:author="Second Reviewer" w:date="2024-01-02T11:00:00Z">"#,
        r#"<w:t xml:space="preserve">nested insertion </w:t>"#,
        r#"<w:commentRangeStart w:id="500"/>"#,
        r#"<w:commentReference w:id="500"/>"#,
        r#"<w:moveFromRangeStart w:id="600" w:name="MoveRange1" w:author="Mover" w:date="2024-01-03T00:00:00Z"/>"#,
        r#"<w:moveFrom w:id="601" w:author="Mover" w:date="2024-01-03T00:00:00Z">"#,
        r#"<w:moveToRangeStart w:id="600" w:name="MoveRange1" w:author="Mover" w:date="2024-01-03T00:00:00Z"/>"#,
        r#"<w:moveTo w:id="602" w:author="Mover" w:date="2024-01-03T00:00:00Z">"#,
        r#"<w:pPrChange w:id="700" w:author="Formatter" w:date="2024-01-04T00:00:00Z">"#,
        r#"<w:rPrChange w:id="710" w:author="Formatter" w:date="2024-01-04T01:00:00Z">"#,
        r#"<w:cellMerge w:id="800" w:author="Merger" w:date="2024-01-05T00:00:00Z" w:vMerge="cont" w:vMergeOrig="rest"/>"#,
    ] {
        assert!(
            before.contains(needle),
            "fixture sanity: {needle} present before the edit"
        );
        assert!(
            after.contains(needle),
            "edit isolation broken: {needle} is no longer present, verbatim, after editing an \
             unrelated run"
        );
    }

    // And nothing outside the one edited run's own text differs at all: strip the one changed
    // substring from each side and the remainder must be byte-identical.
    let before_normalized = before.replace("<w:t>after</w:t>", "<w:t>after \u{2014} edited</w:t>");
    assert_eq!(
        before_normalized, after,
        "the saved document differs from the original by more than the one edited run's own text"
    );
}

// -------------------------------------------------------------------------------------------
// A property setter never destroys an existing *Change record.
// -------------------------------------------------------------------------------------------

/// Would this pass if the work were not done? No: a setter that replaced the whole `RunPropertyContent`
/// vec (rather than finding-and-replacing only its own variant) would drop `w:rPrChange` the moment
/// any other property on the same run was set — this is exactly the defect
/// `crate::document::revisions`'s own "one rule" documents as structurally impossible given how
/// every setter in this crate is already written, proved here on a real fixture rather than merely
/// asserted in the module doc.
/// Also confirms the fixture's own paragraph 14 (`P14_RUN_WITH_RPR_CHANGE`) is present, matching
/// this file's own layout.
#[test]
fn adversarial_fixture_carries_the_rprchange_paragraph_this_test_file_expects() {
    let mut document = adversarial_fixture();
    let xml = document_xml(&mut document);
    assert!(xml.contains(r#"<w:rPrChange w:id="710" w:author="Formatter" w:date="2024-01-04T01:00:00Z"><w:rPr><w:i/></w:rPr></w:rPrChange>"#));
}

#[test]
fn property_setter_preserves_an_existing_rprchange() {
    use mjx_docx::RunProperties;
    use mjx_ooxml_core::{FromXml, ToXml};

    // `w:b` (live bold) + `w:rPrChange` (the run was previously italic, not bold) — the same shape
    // as the adversarial fixture's own paragraph 14, isolated here so the setter under test is
    // exercised directly rather than through `Document`'s own plumbing (this crate has no
    // `Document`-level "edit an arbitrary run's properties" method today — only `set_run_text`,
    // which this ticket's own edit-isolation test already covers).
    let xml = format!(
        r#"<w:rPr xmlns:w="{ns}"><w:b/><w:rPrChange w:id="710" w:author="Formatter" w:date="2024-01-04T01:00:00Z"><w:rPr><w:i/></w:rPr></w:rPrChange></w:rPr>"#,
        ns = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
    );
    let mut doc = mjx_xml::fidelity::parse(xml.as_bytes()).expect("fragment parses");
    let mut properties =
        RunProperties::from_xml(&doc.root, &doc.interner).expect("from_xml succeeds");
    assert!(
        properties.change().is_some(),
        "fixture sanity: w:rPrChange present before the edit"
    );

    // Would this pass if the work were not done? No: a setter that replaced the whole
    // `RunPropertyContent` vec (rather than finding-and-replacing only its own variant) would drop
    // `w:rPrChange` the moment any other property on the same `w:rPr` was set.
    properties.set_italic(&mut doc.interner, Some(true));

    let change = properties
        .change()
        .expect("w:rPrChange must still be present after an unrelated property was set");
    assert_eq!(change.id(&doc.interner), Ok(710));
    assert_eq!(change.author(&doc.interner).as_deref(), Some("Formatter"));
    assert_eq!(
        change.date(&doc.interner).as_deref(),
        Some("2024-01-04T01:00:00Z")
    );
    assert!(
        change.previous_run_properties().is_some(),
        "the change's own previous w:rPr (w:i) must still be there too"
    );

    doc.root = properties.to_xml(&mut doc.interner);
    let out = mjx_xml::fidelity::serialize_to_vec(&doc);
    let out = String::from_utf8(out).expect("utf-8");
    assert!(out.contains(r#"<w:rPrChange w:id="710" w:author="Formatter" w:date="2024-01-04T01:00:00Z"><w:rPr><w:i/></w:rPr></w:rPrChange>"#));
    assert!(out.contains("<w:i/>") && out.contains("<w:b/>"));
}

// -------------------------------------------------------------------------------------------
// The schema gate: every authored variant in the adversarial fixture is schema-valid.
// -------------------------------------------------------------------------------------------

/// The C1 schema gate ("Done when": "The schema gate is green on every authored variant") — every
/// revision type this fixture exercises (`w:ins`/`w:del` including the nested case, `w:moveFrom`/
/// `w:moveTo` with their own range markers, `w:commentRangeStart`/`End`, `w:pPrChange`, `w:rPrChange`,
/// `w:cellMerge`) walked against `wml.xsd` (Part 4 Transitional) and checked for correct child
/// order. Skips silently without a local `References/` tree — see
/// [`mjx_schema_gate::assert_authored_deck_is_schema_valid`]'s own doc comment — but always checks
/// child order, which needs no external schema.
///
/// This fixture is a **different** one from `document::revisions::tests::
/// a_malformed_date_round_trips_byte_identical_never_normalised` **on purpose**: `ST_DateTime` is an
/// unconstrained `xsd:dateTime` restriction, so a malformed `w:date` is itself schema-invalid — that
/// fixture is deliberately kept out of this schema-gated sweep, and every `w:date`/`w:author`/`w:id`
/// in *this* one is well-formed.
#[test]
fn the_adversarial_fixture_is_schema_valid_and_in_schema_order() {
    let document = adversarial_fixture();
    let bytes = document.save().expect("save the adversarial fixture");
    assert_authored_deck_is_schema_valid(
        "the MJXOFF-126 adversarial revision-marks fixture",
        &bytes,
    );
}
