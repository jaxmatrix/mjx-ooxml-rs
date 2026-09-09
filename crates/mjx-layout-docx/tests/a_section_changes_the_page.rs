//! **A document's page shape changes half way through, and the pages after the change are laid out
//! on the new sheet.**
//!
//! # The shortcut this suite exists to catch
//!
//! `mjx-docx`'s own `sections.rs` names it: a `w:sectPr` inside a paragraph's `w:pPr` *ends* the
//! section that paragraph belongs to, and the body-level one governs whatever is left. An engine
//! that read only the body-level `w:sectPr` — by far the commonest shortcut, because it is the one
//! that is easy to find — would lay **every** page of every document out at the *last* section's
//! page size. That is invisible in a single-section fixture and wrong in every multi-section one, so
//! every fixture below changes the sheet **and asserts on page one**, which is the page a
//! last-`sectPr`-only implementation gets wrong.
//!
//! # And the one that is invisible to content
//!
//! An `evenPage` break inserts a **blank page** when the next page would have the wrong parity. No
//! assertion about which paragraph is on which page can see it — the paragraphs are all still in
//! order, one page later — so the page **count** is asserted directly.

mod support;

use mjx_layout::PageIndex;
use mjx_layout_docx::{DocumentBoxModel, DocumentFlow};
use mjx_ooxml_core::measure::Emu;
use support::{constraints, flow, model, page_of_lines};

/// A paragraph that ends its section, with `section` as its own `w:sectPr`.
fn section_break(section: &str, text: &str) -> String {
    support::paragraph(&format!("<w:sectPr>{section}</w:sectPr>"), text)
}

/// Every page of `document`, with the box model's own report for each.
fn pages(
    paragraphs: &[String],
    body_section: &str,
    constraints: &mjx_layout::Constraints,
    limit: usize,
) -> Vec<(usize, mjx_layout_docx::PageReport)> {
    let markup = support::document_markup_with(paragraphs, body_section);
    let mut document = support::document_from_bytes(markup);
    let flow = flow(&mut document);
    let mut model = model();
    let mut reports = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let Ok(page) = <DocumentBoxModel as mjx_layout::BoxModel>::layout_page(
            &mut model,
            &flow,
            PageIndex::new(u32::try_from(number).expect("a page number")),
            constraints,
            resume.as_ref(),
        ) else {
            break;
        };
        resume = page.continuation().cloned();
        reports.push((number, model.last_page().clone()));
        if resume.is_none() {
            break;
        }
    }
    reports
}

/// **The trap, stated as an assertion.** Section one is landscape and section two is portrait, and
/// the pages differ in width — which an implementation that read only the body-level `w:sectPr`
/// cannot produce, because it would lay both out portrait.
#[test]
fn a_mid_document_page_size_change_moves_the_second_section_on_to_a_different_sheet() {
    let landscape = support::page_geometry(11.0, 8.5, 1.0);
    let portrait = support::page_geometry(8.5, 11.0, 1.0);
    let paragraphs = vec![
        support::paragraph("", "Alpha in the first section."),
        section_break(&landscape, "Omega, the last of the landscape section."),
        support::paragraph("", "Beta, on a portrait sheet."),
    ];
    let reports = pages(&paragraphs, &portrait, &constraints(8.5, 11.0), 6);
    assert_eq!(reports.len(), 2, "one page per section: {reports:?}");

    let first = &reports[0].1;
    let second = &reports[1].1;
    assert_eq!(first.section, Some(0));
    assert_eq!(second.section, Some(1));
    // 11 inches wide less two one-inch margins.
    assert_eq!(
        first.body.width(),
        Emu::from_inches(9.0),
        "page one is the landscape section's own sheet, which is what a last-`sectPr`-only \
         implementation cannot produce"
    );
    assert_eq!(second.body.width(), Emu::from_inches(6.5));
    assert_ne!(
        first.body.width(),
        second.body.width(),
        "the two sections must not share a sheet, or this fixture asserts nothing"
    );
}

/// `nextPage` starts a page and `continuous` does not — the same document twice, differing in one
/// attribute, asserted on the page count.
///
/// **`w:type` is on the section it *starts*, which is the body-level `w:sectPr` here** — §17.6.22
/// gives the break kind of "the current section", and the current section of a `w:sectPr` is the one
/// it governs. Putting it on the paragraph-level one would state how section *one* begins, which is
/// a question about the page before the document.
#[test]
fn a_continuous_break_does_not_start_a_page_and_next_page_does() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let paragraphs = vec![
        support::paragraph("", "Alpha."),
        section_break(&geometry, "Omega of section one."),
        support::paragraph("", "Beta."),
    ];
    let area = constraints(8.5, 11.0);
    let body_of = |kind: &str| format!(r#"<w:type w:val="{kind}"/>{geometry}"#);
    let next_page = pages(&paragraphs, &body_of("nextPage"), &area, 6);
    let continuous = pages(&paragraphs, &body_of("continuous"), &area, 6);
    assert_eq!(next_page.len(), 2, "`nextPage` starts a page");
    assert_eq!(
        continuous.len(),
        1,
        "`continuous` carries on down the same sheet: {continuous:?}"
    );
    assert_eq!(
        continuous[0].1.column_groups, 2,
        "and the sheet then holds two sections' worth of content"
    );
}

/// An `oddPage` break opens a **blank** page when the next one would be even, and an `evenPage`
/// break on the same document opens none.
///
/// The paragraphs are in the same order either way, so nothing about *which* paragraph is where can
/// see this. The page count can — and the pair is what makes the assertion mean something: a build
/// that inserted a blank page for *every* parity break would pass a one-sided version of this test.
#[test]
fn an_odd_page_break_inserts_a_blank_page_that_content_cannot_see() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let paragraphs = vec![
        support::paragraph("", "Alpha, on page one."),
        section_break(&geometry, "Omega of section one."),
        support::paragraph("", "Beta, in section two."),
    ];
    let area = constraints(8.5, 11.0);
    let body_of = |kind: &str| format!(r#"<w:type w:val="{kind}"/>{geometry}"#);
    let plain = pages(&paragraphs, &body_of("nextPage"), &area, 8);
    let even = pages(&paragraphs, &body_of("evenPage"), &area, 8);
    let odd = pages(&paragraphs, &body_of("oddPage"), &area, 8);

    assert_eq!(plain.len(), 2);
    assert_eq!(
        even.len(),
        2,
        "section two already lands on page two, which is even, so `evenPage` costs nothing: \
         {even:?}"
    );
    assert_eq!(
        odd.len(),
        3,
        "page two must be blank so that section two starts on page three, the next odd one: \
         {odd:?}"
    );
    assert!(!odd[0].1.blank);
    assert!(odd[1].1.blank, "the inserted page carries no body content");
    assert!(!odd[2].1.blank);
    assert_eq!(odd[2].1.section, Some(1));
    assert_eq!(odd[2].1.page_number, 3);
    assert!(
        even.iter().all(|(_, page)| !page.blank),
        "and the even case inserts none at all"
    );
}

/// Three sections, each restarting its own page numbering, asserted on the number each page shows.
#[test]
fn page_numbering_restarts_per_section() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let paragraphs = vec![
        support::paragraph("", "Front matter."),
        section_break(
            &format!(r#"{geometry}<w:pgNumType w:fmt="lowerRoman" w:start="1"/>"#),
            "End of the front matter.",
        ),
        support::paragraph("", "Chapter one."),
        section_break(
            &format!(r#"{geometry}<w:pgNumType w:start="1"/>"#),
            "End of chapter one.",
        ),
        support::paragraph("", "The appendix."),
    ];
    let reports = pages(
        &paragraphs,
        &format!(r#"{geometry}<w:pgNumType w:start="100"/>"#),
        &constraints(8.5, 11.0),
        8,
    );
    let numbers: Vec<i64> = reports.iter().map(|(_, page)| page.page_number).collect();
    assert_eq!(
        numbers,
        vec![1, 1, 100],
        "each section's own `w:start` wins, and a section without one carries on from the page \
         before"
    );
    assert_eq!(
        mjx_layout_docx::format_number(
            1,
            mjx_ooxml_types::wordprocessingml::NumberFormat::LowercaseRomanNumerals
        ),
        "i",
        "and the first section asked for lower-case Roman numerals"
    );
}

/// A section that governs no paragraph at all — two `w:sectPr`s with nothing between them — is
/// skipped rather than allowed to swallow the next section's content.
#[test]
fn an_empty_section_does_not_swallow_the_next_one() {
    let narrow = support::page_geometry(4.0, 11.0, 0.5);
    let wide = support::page_geometry(8.5, 11.0, 0.5);
    let paragraphs = vec![
        section_break(&narrow, "Only paragraph of section one."),
        // A paragraph whose own `w:sectPr` ends a section that begins after the previous one's last
        // paragraph — so section two governs exactly this paragraph, and section three none.
        support::paragraph("", "Section two's paragraph."),
    ];
    let reports = pages(&paragraphs, &wide, &constraints(8.5, 11.0), 6);
    assert!(reports.len() >= 2, "two sections, two pages: {reports:?}");
    assert_eq!(reports[0].1.body.width(), Emu::from_inches(3.0));
    assert_eq!(reports[1].1.body.width(), Emu::from_inches(7.5));
}

/// A section's `w:vAlign` moves its content down a page it does not fill.
#[test]
fn vertical_alignment_moves_content_down_a_page_it_does_not_fill() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let top = support::document_with(
        &[support::paragraph("", "One line.")],
        &format!(r#"{geometry}<w:vAlign w:val="top"/>"#),
    );
    let bottom = support::document_with(
        &[support::paragraph("", "One line.")],
        &format!(r#"{geometry}<w:vAlign w:val="bottom"/>"#),
    );
    let area = constraints(8.5, 11.0);
    let first = support::first_line_top(top, &area);
    let last = support::first_line_top(bottom, &area);
    assert!(
        last > first,
        "`bottom` must push the only line down the page: {first:?} against {last:?}"
    );
}

/// The long-document page-count assertion MJXOFF-174 rested on still holds, section machinery and
/// all: a page of a uniform document holds a constant number of lines however far in it is.
#[test]
fn a_long_single_section_document_still_paginates_uniformly() {
    let paragraphs: Vec<String> = (0..120)
        .map(|index| support::paragraph("", &format!("Paragraph number {index}.")))
        .collect();
    let area = page_of_lines(4);
    let mut document = support::document(&paragraphs);
    let flow = DocumentFlow::read(&mut document).expect("the document resolves");
    let mut model = model();
    let walked = support::walk(&mut model, &flow, &area, 200);
    assert!(walked.len() > 20, "a long document: {} pages", walked.len());
    let counts: Vec<usize> = walked
        .iter()
        .map(|page| support::line_count(page.fragments()))
        .collect();
    let full = counts[0];
    assert!(
        counts[..counts.len() - 1]
            .iter()
            .all(|count| *count == full),
        "every page but the last holds the same number of lines: {counts:?}"
    );
}
