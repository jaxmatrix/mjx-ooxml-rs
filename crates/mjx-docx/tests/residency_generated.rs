//! MJXOFF-177 (R22): the four things the residency used to skip — fields, tracked changes,
//! equations and a numbering definition's own levels.
//!
//! # Why these are in the residency at all
//!
//! Each is something a **reader sees** that a paragraph's own `w:t` runs do not contain. A box model
//! has to measure what a reader sees, so each has to be resolved once for the whole document rather
//! than looked up per paragraph — which is the same argument `residency.rs` makes for the ladder,
//! applied to four more subjects.
//!
//! # The one that was a bug rather than a gap
//!
//! `w:ins`, `w:del`, `w:moveFrom` and `w:moveTo` used to fall to `walk_paragraph_content`'s own
//! wildcard, so **content inside one contributed nothing at all** — a document with tracked
//! insertions was read with the inserted text missing, as though every change had been rejected, and
//! nothing anywhere reported it. [`an_insertion_contributes_its_text`] is the assertion that would
//! have caught it.

use mjx_docx::{Document, Package, PartName, RevisionKind};
use mjx_fixtures::fixture;

/// `word/document.xml`.
const DOCUMENT_PART: &str = "/word/document.xml";

/// A document whose `word/document.xml` is `body`, wrapped in the minimal envelope.
///
/// Authored as markup and spliced into a blank package, which is how every fixture in this crate
/// that needs an element `Document::blank` does not write reaches the reader.
fn document_of(body: &str) -> Document {
    let markup = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#,
            r#" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">"#,
            "<w:body>{body}<w:sectPr/></w:body>",
            "</w:document>",
        ),
        body = body
    );
    let blank = Document::blank(mjx_docx::PageSize::a4()).expect("a blank document");
    let bytes = blank.save_unchecked().expect("the blank saves");
    let mut package = Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new(DOCUMENT_PART).expect("a valid part name"),
            markup.into_bytes(),
        )
        .expect("the main part is replaceable");
    Document::from_package(package).expect("the authored document opens")
}

#[test]
fn a_complex_field_carries_its_instruction_and_the_bytes_of_its_cached_result() {
    let mut document = document_of(concat!(
        "<w:p>",
        r#"<w:r><w:t xml:space="preserve">page </w:t></w:r>"#,
        r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r>"#,
        r#"<w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r>"#,
        r#"<w:r><w:fldChar w:fldCharType="separate"/></w:r>"#,
        "<w:r><w:t>7</w:t></w:r>",
        r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let paragraph = &formatting.paragraphs()[0];
    assert_eq!(
        paragraph.text(),
        "page 7",
        "the cached result is ordinary text and the instruction is not"
    );
    let fields = paragraph.fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].instruction, " PAGE ");
    assert_eq!(
        paragraph.text().get(fields[0].result.clone()),
        Some("7"),
        "and the span names exactly the bytes a renderer replaces"
    );
    assert_eq!(fields[0].form, mjx_docx::FieldForm::Complex);
    assert!(!fields[0].dirty && !fields[0].locked);
}

#[test]
fn a_field_with_no_separator_has_an_empty_result() {
    // Legal markup, not malformed: a field that has never been computed. A renderer must not read
    // the bytes after the field as its cached result.
    let mut document = document_of(concat!(
        "<w:p>",
        r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r>"#,
        r#"<w:r><w:instrText xml:space="preserve"> DATE </w:instrText></w:r>"#,
        r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
        "<w:r><w:t>after</w:t></w:r>",
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let fields = formatting.paragraphs()[0].fields();
    assert_eq!(fields.len(), 1);
    assert!(
        fields[0].result.is_empty(),
        "an unseparated field has no result: {:?}",
        fields[0].result
    );
    assert_eq!(formatting.paragraphs()[0].text(), "after");
}

#[test]
fn the_committed_fields_fixture_resolves_its_nesting_and_its_split_instruction() {
    // `fields_and_hyperlinks.docx` is Word-shaped: a `TOC` holding two `PAGEREF`s, and a `HYPERLINK`
    // whose instruction is split across two `w:instrText` runs (`HYPER` then `LINK "…"`). A reader
    // that took the first `w:instrText` as the whole instruction would see a field named `HYPER`.
    let mut document =
        Document::open(&fixture("fields_and_hyperlinks.docx")).expect("the fixture opens");
    let formatting = document.formatting().expect("the fixture resolves");
    let first = formatting.paragraphs()[0].fields();
    assert_eq!(first.len(), 3, "a `TOC` and its two `PAGEREF`s");
    assert!(first[0].instruction.contains("TOC"));
    assert_eq!(first[0].parent, None);
    assert_eq!(first[1].parent, Some(0));
    assert_eq!(first[2].parent, Some(0));

    let second = formatting.paragraphs()[1].fields();
    assert_eq!(second.len(), 1);
    assert!(
        second[0].instruction.contains("HYPERLINK"),
        "the two `w:instrText` runs concatenate: {:?}",
        second[0].instruction
    );
}

#[test]
fn an_insertion_contributes_its_text() {
    // The bug this file exists for. Before MJXOFF-177 the whole `w:ins` fell to a wildcard and the
    // paragraph read as `kept `.
    let mut document = document_of(concat!(
        "<w:p>",
        r#"<w:r><w:t xml:space="preserve">kept </w:t></w:r>"#,
        r#"<w:ins w:id="1" w:author="Priya" w:date="2026-09-09T00:00:00Z">"#,
        "<w:r><w:t>and added</w:t></w:r>",
        "</w:ins>",
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let paragraph = &formatting.paragraphs()[0];
    assert_eq!(paragraph.text(), "kept and added");
    let spans = paragraph.revisions();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].kind, RevisionKind::Inserted);
    assert_eq!(spans[0].author.as_deref(), Some("Priya"));
    assert_eq!(
        paragraph.text().get(spans[0].range.clone()),
        Some("and added")
    );
}

#[test]
fn a_deletion_contributes_its_delete_text() {
    let mut document = document_of(concat!(
        "<w:p>",
        r#"<w:r><w:t xml:space="preserve">kept </w:t></w:r>"#,
        r#"<w:del w:id="2" w:author="Sam" w:date="2026-09-09T00:00:00Z">"#,
        "<w:r><w:delText>and removed</w:delText></w:r>",
        "</w:del>",
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let paragraph = &formatting.paragraphs()[0];
    assert_eq!(
        paragraph.text(),
        "kept and removed",
        "the text is the all-markup view; which bytes are deleted is the span's job"
    );
    assert_eq!(paragraph.revisions()[0].kind, RevisionKind::Deleted);
}

#[test]
fn a_runs_range_still_covers_the_text_when_a_revision_is_in_it() {
    // The invariant `residency.rs` promises about runs — no gaps, no overlaps — has to survive
    // content that used to contribute nothing, or a layout engine's itemisation silently drops it.
    let mut document = document_of(concat!(
        "<w:p>",
        r#"<w:r><w:t xml:space="preserve">one </w:t></w:r>"#,
        r#"<w:ins w:id="1" w:author="Priya"><w:r><w:t xml:space="preserve">two </w:t></w:r></w:ins>"#,
        r#"<w:del w:id="2" w:author="Sam"><w:r><w:delText>three</w:delText></w:r></w:del>"#,
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let paragraph = &formatting.paragraphs()[0];
    let mut at = 0_usize;
    for run in paragraph.runs() {
        assert_eq!(run.range.start, at, "runs cover the text without gaps");
        at = run.range.end;
    }
    assert_eq!(at, paragraph.text().len(), "and reach its end");
}

#[test]
fn an_equation_is_resolved_to_plain_values_and_contributes_no_character() {
    use mjx_docx::EquationNode;
    let mut document = document_of(concat!(
        "<w:p>",
        "<w:r><w:t>before</w:t></w:r>",
        "<m:oMath>",
        "<m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f>",
        "</m:oMath>",
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let paragraph = &formatting.paragraphs()[0];
    assert_eq!(
        paragraph.text(),
        "before",
        "an equation is a generated mark: it has a position and no character"
    );
    let equations = paragraph.equations();
    assert_eq!(equations.len(), 1);
    assert_eq!(equations[0].at, "before".len());
    assert!(!equations[0].display, "an `m:oMath` is inline");
    let EquationNode::Fraction {
        numerator,
        denominator,
        ..
    } = &equations[0].nodes[0]
    else {
        panic!("the one element is a fraction: {:?}", equations[0].nodes);
    };
    assert_eq!(numerator.len(), 1);
    assert_eq!(denominator.len(), 1);
}

#[test]
fn a_display_equation_carries_its_paragraph_justification() {
    let mut document = document_of(concat!(
        "<w:p>",
        "<m:oMathPara>",
        r#"<m:oMathParaPr><m:jc m:val="center"/></m:oMathParaPr>"#,
        "<m:oMath><m:r><m:t>x</m:t></m:r></m:oMath>",
        "</m:oMathPara>",
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    let equations = formatting.paragraphs()[0].equations();
    assert_eq!(equations.len(), 1);
    assert!(
        equations[0].display,
        "an `m:oMathPara` is a display equation"
    );
    assert_eq!(
        equations[0].justification,
        Some(mjx_ooxml_types::officemath::Justification::Center)
    );
}

#[test]
fn only_the_numbering_definitions_a_paragraph_reaches_are_resolved() {
    // `numbering_definitions.docx` defines more lists than its body uses; resolving all of them
    // would pay a `w:numStyleLink` walk each for a marker nobody will ever draw.
    let mut document =
        Document::open(&fixture("numbering_definitions.docx")).expect("the fixture opens");
    let formatting = document.formatting().expect("the fixture resolves");
    let reached: std::collections::BTreeSet<i64> = formatting
        .paragraphs()
        .iter()
        .filter_map(|paragraph| paragraph.properties().numbering)
        .map(|reference| reference.numbering_id)
        .filter(|id| *id != 0)
        .collect();
    let resolved: std::collections::BTreeSet<i64> = formatting
        .numbering_definitions()
        .iter()
        .map(|definition| definition.numbering_id)
        .collect();
    assert_eq!(resolved, reached, "exactly the ones the body names");
    for definition in formatting.numbering_definitions() {
        assert!(
            !definition.levels.is_empty(),
            "and each carries its levels rather than only the one a paragraph names"
        );
        assert!(
            definition
                .levels
                .iter()
                .all(|level| level.template.is_empty() || level.start >= 0),
            "with a start and a template each"
        );
    }
}

#[test]
fn a_numbering_reference_the_document_does_not_define_resolves_to_nothing() {
    // A defect in the document, not a reason to refuse to read it.
    let mut document = document_of(concat!(
        "<w:p>",
        r#"<w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="42"/></w:numPr></w:pPr>"#,
        "<w:r><w:t>an orphan item</w:t></w:r>",
        "</w:p>",
    ));
    let formatting = document.formatting().expect("the document resolves");
    assert_eq!(formatting.paragraphs()[0].text(), "an orphan item");
    assert!(formatting.numbering_definition(42).is_none());
}
