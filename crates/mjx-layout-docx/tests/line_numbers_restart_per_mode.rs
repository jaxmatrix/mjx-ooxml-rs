//! **`w:lnNumType`: the three restart modes, the count-by interval, and the marks themselves.**
//!
//! # Why these are drawn and a page number is not
//!
//! A page number reaches a page through a `PAGE` field, which lives in the run stream and is
//! MJXOFF-177 (R22)'s to render. A footnote's mark reaches it through `w:footnoteRef`, likewise. A
//! **line** number is in no run stream at all: Word generates it from `w:lnNumType` and draws it in
//! the margin, so if this crate computed it and drew nothing, no renderer downstream could ever
//! complete the feature. So it is drawn, and this suite asserts both halves — the numbers, and the
//! glyphs.
//!
//! # The identity-value trap here
//!
//! `w:restart` defaults to `newPage`, so a fixture that states nothing exercises exactly the value a
//! build that ignored the attribute would produce. All three modes are asserted, against each other,
//! on a document long enough for them to disagree.

mod support;

use mjx_layout::{BoxModel, Fragment, PageIndex};
use mjx_layout_docx::PageReport;
use support::{constraints, flow, model};

/// A sheet that holds a handful of lines, so several pages exist to restart across.
fn small_page() -> String {
    support::page_geometry(6.5, 1.5, 0.25)
}

/// Sixteen one-line paragraphs.
fn body() -> Vec<String> {
    (0..16)
        .map(|index| support::paragraph("", &format!("Paragraph {index}.")))
        .collect()
}

/// Every page's report for a document whose body section states `numbering`.
fn reports(paragraphs: &[String], numbering: &str, limit: usize) -> Vec<PageReport> {
    let section = format!("{}{numbering}", small_page());
    let mut document = support::document_with(paragraphs, &section);
    let flow = flow(&mut document);
    let mut model = model();
    let area = constraints(6.5, 1.5);
    let mut out = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let Ok(page) = model.layout_page(
            &flow,
            PageIndex::new(u32::try_from(number).expect("a page number")),
            &area,
            resume.as_ref(),
        ) else {
            break;
        };
        resume = page.continuation().cloned();
        out.push(model.last_page().clone());
        if resume.is_none() {
            break;
        }
    }
    out
}

/// The numbers each page printed, flattened.
fn numbers(paragraphs: &[String], numbering: &str) -> Vec<Vec<i64>> {
    reports(paragraphs, numbering, 30)
        .into_iter()
        .map(|page| page.line_numbers)
        .collect()
}

/// A section that states no `w:lnNumType` numbers nothing at all.
#[test]
fn a_section_without_line_numbering_prints_none() {
    let pages = numbers(&body(), "");
    assert!(
        pages.iter().all(Vec::is_empty),
        "no `w:lnNumType`, no numbers: {pages:?}"
    );
}

/// `newPage` starts over on every page; `continuous` does not. The pair is the assertion.
#[test]
fn new_page_restarts_and_continuous_does_not() {
    let per_page = numbers(
        &body(),
        r#"<w:lnNumType w:countBy="1" w:restart="newPage"/>"#,
    );
    let running = numbers(
        &body(),
        r#"<w:lnNumType w:countBy="1" w:restart="continuous"/>"#,
    );
    assert!(per_page.len() > 2, "several pages: {}", per_page.len());
    assert_eq!(per_page.len(), running.len());

    assert!(
        per_page.iter().all(|page| page.first() == Some(&1)),
        "every page starts at one under `newPage`: {per_page:?}"
    );
    let flat: Vec<i64> = running.iter().flatten().copied().collect();
    assert_eq!(
        flat,
        (1..=i64::try_from(flat.len()).expect("a count")).collect::<Vec<_>>(),
        "and `continuous` counts straight through: {running:?}"
    );
    assert_ne!(
        per_page, running,
        "the two modes must disagree, or the fixture is too short to test either"
    );
}

/// `newSection` restarts at a section boundary and not at a page one.
#[test]
fn new_section_restarts_only_where_a_section_does() {
    let numbering = r#"<w:lnNumType w:countBy="1" w:restart="newSection"/>"#;
    let geometry = small_page();
    let mut paragraphs = body();
    // A section break half way down, carrying the same numbering, so the restart is the only
    // difference between the two halves.
    paragraphs[7] = support::paragraph(
        &format!("<w:sectPr>{geometry}{numbering}</w:sectPr>"),
        "Paragraph 7.",
    );
    let pages = numbers(&paragraphs, numbering);
    let flat: Vec<i64> = pages.iter().flatten().copied().collect();
    assert!(
        flat.iter().filter(|number| **number == 1).count() >= 2,
        "the count starts over once per section: {pages:?}"
    );
    assert!(
        pages.iter().filter(|page| page.first() == Some(&1)).count() < pages.len(),
        "but not once per page, which is what `newPage` would do: {pages:?}"
    );
}

/// `w:countBy` prints every *n*th line and leaves the others unnumbered.
#[test]
fn count_by_prints_every_nth_line() {
    let every = numbers(
        &body(),
        r#"<w:lnNumType w:countBy="1" w:restart="continuous"/>"#,
    );
    let third = numbers(
        &body(),
        r#"<w:lnNumType w:countBy="3" w:restart="continuous"/>"#,
    );
    let all: Vec<i64> = every.iter().flatten().copied().collect();
    let some: Vec<i64> = third.iter().flatten().copied().collect();
    assert!(
        some.len() < all.len(),
        "counting by three prints fewer: {} against {}",
        some.len(),
        all.len()
    );
    assert!(
        some.iter().all(|number| number % 3 == 0),
        "and every one it prints is a multiple of three: {some:?}"
    );
}

/// `w:suppressLineNumbers` skips a paragraph **and does not advance the count**, which is the
/// difference between suppressing a number and hiding one.
#[test]
fn suppress_line_numbers_skips_a_paragraph_without_advancing_the_count() {
    let numbering = r#"<w:lnNumType w:countBy="1" w:restart="continuous"/>"#;
    let plain = body();
    let mut suppressed = plain.clone();
    suppressed[2] = support::paragraph(r#"<w:suppressLineNumbers/>"#, "Paragraph 2.");

    let all: Vec<i64> = numbers(&plain, numbering).into_iter().flatten().collect();
    let some: Vec<i64> = numbers(&suppressed, numbering)
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(
        some.len(),
        all.len() - 1,
        "one paragraph, one line, one fewer number: {some:?}"
    );
    assert_eq!(
        some,
        (1..=i64::try_from(some.len()).expect("a count")).collect::<Vec<_>>(),
        "and the numbers after it carry the ones the suppressed line would have skipped past: \
         {some:?}"
    );
}

/// The number is actually **drawn**, in the margin to the left of the column.
#[test]
fn a_line_number_is_drawn_in_the_margin() {
    let geometry = small_page();
    let numbering = r#"<w:lnNumType w:countBy="1" w:restart="continuous"/>"#;
    let mut without = support::document_with(&body(), &geometry);
    let mut with = support::document_with(&body(), &format!("{geometry}{numbering}"));
    let area = constraints(6.5, 1.5);

    let count = |document: &mut mjx_docx::Document| -> (usize, mjx_ooxml_core::measure::Emu) {
        let flow = flow(document);
        let mut model = model();
        let page = model
            .layout_page(&flow, PageIndex::FIRST, &area, None)
            .expect("page one lays out");
        let runs: Vec<_> = page
            .fragments()
            .nodes()
            .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
            .collect();
        let leftmost = runs
            .iter()
            .map(|(_, node)| node.rect().left)
            .min()
            .unwrap_or(mjx_ooxml_core::measure::Emu::ZERO);
        (runs.len(), leftmost)
    };
    let (plain_runs, plain_left) = count(&mut without);
    let (numbered_runs, numbered_left) = count(&mut with);
    assert!(
        numbered_runs > plain_runs,
        "the numbers are glyphs on the page: {numbered_runs} against {plain_runs}"
    );
    assert!(
        numbered_left < plain_left,
        "and they sit to the left of the text column, in the margin: {numbered_left:?} against \
         {plain_left:?}"
    );
}
