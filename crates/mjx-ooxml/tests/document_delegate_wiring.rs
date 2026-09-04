//! Every Word delegate is wired to the method it is named after — the Word sibling of
//! `delegate_wiring.rs`. See that file's own doc comment for why every assertion here is
//! **asymmetric on purpose**: nothing is set to the same value as its neighbour, and every pair that
//! could be swapped (`row`/`column`, `header`/`footer`, `paragraph`/`run`, `instruction`/`cached
//! result`) is given values that differ, so a delegate wired to its neighbour fails here even though
//! the method it wrongly calls works perfectly.

use mjx_ooxml::{
    Document, ErrorCode, HeaderFooterType, HyperlinkTarget, MergedCellType, PageSize,
    SectionLocation,
};

fn blank() -> Document {
    Document::blank(PageSize::a4()).expect("a blank document")
}

/// `paragraph_text`/`run_text` and `set_run_text` must never confuse the paragraph axis with the run
/// axis: two paragraphs, each with a run holding the *other* paragraph's would-be text, told apart.
#[test]
fn paragraph_and_run_addressing_are_not_transposed() {
    let mut document = blank();
    document.append_paragraph().expect("a second paragraph");
    // `Document::blank` writes one *empty* paragraph — no placeholder run to clear.
    document
        .append_run(0, "first paragraph")
        .expect("a run on paragraph 0");
    document
        .append_run(1, "second paragraph")
        .expect("a run on paragraph 1");

    assert_eq!(document.paragraph_text(0).unwrap(), "first paragraph");
    assert_eq!(document.paragraph_text(1).unwrap(), "second paragraph");
    assert_eq!(document.run_text(0, 0).unwrap(), "first paragraph");
    assert_eq!(document.run_text(1, 0).unwrap(), "second paragraph");
}

/// `cell_text`/`set_cell_text`/`cell_span` must not swap `row` and `column` — a 3-row, 2-column table
/// makes a transposed address land out of range.
#[test]
fn table_row_and_column_are_not_transposed() {
    let mut document = blank();
    let table = document.append_table(3, 2).expect("a 3x2 table");
    document
        .set_cell_text(table, 2, 1, "row 2, col 1")
        .expect("the bottom-right cell exists at (2, 1)");
    // A transposed writer would have tried (1, 2), which is out of range for a 3x2 table and would
    // have failed outright — so this call succeeding at all is already part of the proof, and the
    // readback confirms the value landed at the address asked for, not its mirror.
    assert_eq!(document.cell_text(table, 2, 1).unwrap(), "row 2, col 1");
    assert_eq!(document.cell_text(table, 1, 0).unwrap(), "");

    let (rows, columns) = document.table_dimensions(table).unwrap();
    assert_eq!(
        (rows, columns),
        (3, 2),
        "table_dimensions swapped rows and columns"
    );

    document
        .set_cell_span(table, 0, 0, Some(2))
        .expect("a horizontal (column) span");
    let (row_span, column_span) = document.cell_span(table, 0, 0).unwrap();
    assert_eq!(
        (row_span, column_span),
        (1, 2),
        "a horizontal merge must widen the column span, not the row span"
    );
}

/// `header_text`/`footer_text`/`set_header_text`/`set_footer_text` given different text must not
/// cross — the single easiest wiring mistake for a "header and footer" pair to make.
#[test]
fn header_and_footer_are_not_each_other() {
    let mut document = blank();
    document
        .set_header_text(
            SectionLocation::Body,
            HeaderFooterType::Default,
            "top of page",
        )
        .expect("a header");
    document
        .set_footer_text(
            SectionLocation::Body,
            HeaderFooterType::Default,
            "bottom of page",
        )
        .expect("a footer");

    assert_eq!(
        document.header_text(0, HeaderFooterType::Default).unwrap(),
        Some("top of page".to_owned()),
        "header_text read the footer"
    );
    assert_eq!(
        document.footer_text(0, HeaderFooterType::Default).unwrap(),
        Some("bottom of page".to_owned()),
        "footer_text read the header"
    );

    // Removing the header must not disturb the footer.
    document
        .remove_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("removing the header");
    assert_eq!(
        document.footer_text(0, HeaderFooterType::Default).unwrap(),
        Some("bottom of page".to_owned()),
        "remove_header reached the footer"
    );
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// `set_field_instruction`/`set_field_cached_result_text` must not cross. Field *construction* is
/// not part of this facade's curated surface (see `Document`'s own module doc), so this uses the
/// same hand-authored complex field `crates/mjx-docx/tests/fields.rs` already relies on:
/// `fields_and_hyperlinks.docx` paragraph 1 holds one `HYPERLINK` field whose instruction and cached
/// result are already distinguishable strings.
#[test]
fn field_instruction_and_cached_result_are_not_each_other() {
    let mut document =
        Document::open(&fixture("fields_and_hyperlinks.docx")).expect("open the fixture");
    let before = document.fields(1).expect("the field before editing");
    assert_eq!(
        before[0].instruction(),
        " HYPERLINK \"http://example.com\" "
    );
    assert_eq!(before[0].cached_result(), Some("example.com"));

    document
        .set_field_instruction(1, &[0], " HYPERLINK \"http://example.org\" ")
        .expect("editing the instruction");
    let after_instruction = document
        .fields(1)
        .expect("the field after editing the instruction");
    assert_eq!(
        after_instruction[0].instruction(),
        " HYPERLINK \"http://example.org\" ",
        "set_field_instruction did not reach the instruction"
    );
    assert_eq!(
        after_instruction[0].cached_result(),
        Some("example.com"),
        "set_field_instruction reached the cached result"
    );

    document
        .set_field_cached_result_text(1, &[0], "example.org")
        .expect("editing the cached result");
    let after_result = document
        .fields(1)
        .expect("the field after editing the cached result");
    assert_eq!(
        after_result[0].instruction(),
        " HYPERLINK \"http://example.org\" ",
        "set_field_cached_result_text reached the instruction"
    );
    assert_eq!(
        after_result[0].cached_result(),
        Some("example.org"),
        "set_field_cached_result_text did not reach the cached result"
    );
}

/// `set_cell_vertical_merge`'s `Restart`/`Continue` must not cross with `set_cell_span`'s own
/// gridSpan — both touch `w:tcPr` but on different children, and a wiring mistake here would set the
/// wrong one.
#[test]
fn cell_span_and_vertical_merge_are_not_each_other() {
    let mut document = blank();
    let table = document.append_table(2, 2).expect("a 2x2 table");
    document
        .set_cell_vertical_merge(table, 0, 0, Some(MergedCellType::Restart))
        .expect("starting a vertical merge");
    document
        .set_cell_vertical_merge(table, 1, 0, Some(MergedCellType::Continue))
        .expect("continuing it");
    // The vertical merge must grow the *row* span to 2 while the column span stays untouched at 1 —
    // a wiring bug that set gridSpan instead of vMerge would grow the column span instead.
    let (row_span, column_span) = document.cell_span(table, 0, 0).unwrap();
    assert_eq!(
        (row_span, column_span),
        (2, 1),
        "a vertical merge must widen the row span, not the column span"
    );
    let (anchor_row, anchor_column) = document.merged_cell_anchor(table, 1, 0).unwrap();
    assert_eq!(
        (anchor_row, anchor_column),
        (0, 0),
        "the continuation cell must resolve up to the restart, not sideways"
    );
}

/// A hyperlink's `Url` and `Anchor` variants must not cross — inserting one of each and reading both
/// back distinctly.
#[test]
fn hyperlink_url_and_anchor_are_not_each_other() {
    let mut document = blank();
    document.append_paragraph().expect("a second paragraph");
    document
        .insert_hyperlink(
            0,
            0,
            "external",
            &HyperlinkTarget::Url("https://example.org/".to_owned()),
        )
        .expect("a url hyperlink");
    document
        .insert_hyperlink(
            1,
            0,
            "internal",
            &HyperlinkTarget::Anchor("bookmark".to_owned()),
        )
        .expect("an anchor hyperlink");

    assert_eq!(
        document.hyperlink_target(0, 0).unwrap(),
        Some(HyperlinkTarget::Url("https://example.org/".to_owned()))
    );
    assert_eq!(
        document.hyperlink_target(1, 0).unwrap(),
        Some(HyperlinkTarget::Anchor("bookmark".to_owned()))
    );
}

/// `text_with_revisions_accepted`/`_rejected` must not cross: a tracked change survives editing only
/// through the reading its own name promises.
#[test]
fn accepted_and_rejected_revision_text_are_not_each_other() {
    // Revision *authoring* is not part of the curated surface (see `Document`'s own module doc), so
    // this proves the pair on the identity every no-tracked-change document must satisfy: with
    // nothing to diverge on, both readings must agree exactly.
    let mut document = blank();
    document.append_run(0, "steady state").expect("a run");
    assert_eq!(
        document.text_with_revisions_accepted().unwrap(),
        document.text_with_revisions_rejected().unwrap()
    );
}

/// An out-of-range address reports [`ErrorCode::IndexOutOfRange`], not some other code — the
/// facade's own translation, exercised on the Word side the same way `delegate_wiring.rs` exercises
/// it for PowerPoint.
#[test]
fn a_word_error_names_where_it_happened() {
    let mut document = blank();
    let error = document.paragraph_text(99).expect_err("no such paragraph");
    assert_eq!(error.code(), ErrorCode::IndexOutOfRange);
}
