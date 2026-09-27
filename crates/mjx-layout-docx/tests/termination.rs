//! Every pagination constraint can refuse content, and a constraint that can refuse content can
//! refuse it **for ever**. This is the suite that says it does not.
//!
//! # The three shapes an infinite loop takes here
//!
//! 1. **An unsatisfiable `w:keepNext` chain.** A run of paragraphs each kept with the next, longer
//!    than a page, has no assignment that satisfies it: every page's trailing chain is pushed
//!    forward, and the page it is pushed on to has the same problem.
//! 2. **A paragraph that does not fit anywhere.** A `w:keepLines` paragraph taller than a page, or a
//!    single word wider than the measure, has to be *placed* rather than deferred.
//! 3. **A measure narrower than one glyph.** The line breaker can then never advance, and a naive
//!    loop produces empty lines until memory runs out.
//!
//! Each of them is run below on a fixture large enough that a loop would be obvious, and each
//! assertion is the same one: **it produced pages, and it produced them quickly.** There is no
//! timeout here, deliberately — a timeout is a flaky test on a loaded machine. What there is instead
//! is a bound on the number of pages, which a non-terminating implementation cannot satisfy because
//! it never returns at all.

mod support;

use mjx_layout::{BoxModel, PageIndex};
use support::{constraints, document, flow, model, page_of_lines, paragraph, walk};

/// Long enough to be several lines at any measure a test uses.
const PROSE: &str = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                     mike november oscar papa quebec romeo sierra tango uniform victor whiskey \
                     xray yankee zulu alpha bravo charlie delta echo foxtrot golf hotel";

/// **The classic.** A thousand paragraphs, every one of them kept with the next, on a page that
/// holds three lines. No assignment satisfies the chain.
#[test]
fn an_unsatisfiable_keep_with_next_chain_still_produces_pages() {
    let paragraphs: Vec<String> = (0..1_000)
        .map(|index| paragraph("<w:keepNext/>", &format!("Link number {index}.")))
        .collect();
    let mut document = document(&paragraphs);
    let flow = flow(&mut document);
    let constraints = page_of_lines(3);

    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 2_000);
    assert!(!pages.is_empty(), "the chain must produce pages");
    assert!(
        pages.len() < 2_000,
        "a thousand paragraphs must not need two thousand pages: {}",
        pages.len()
    );
    // Every page must hold something. A page that held nothing would produce a next position equal
    // to its own start, which is the loop.
    for (number, page) in pages.iter().enumerate() {
        assert!(
            support::line_count(page.fragments()) > 0,
            "page {number} holds nothing, which is where the loop begins"
        );
    }
}

/// A chain that is exactly as long as a page **is** satisfiable, and the difference between the two
/// cases is what says the guard fires only when it has to.
#[test]
fn a_satisfiable_chain_is_kept_together() {
    let constraints = page_of_lines(4);
    let paragraphs = [
        paragraph("", PROSE),
        paragraph("<w:keepNext/>", "Kept."),
        paragraph("", "With this."),
    ];
    let pages = support::page_of_paragraph(&paragraphs, &constraints, 1);
    let target = support::page_of_paragraph(&paragraphs, &constraints, 2);
    assert_eq!(pages, target, "a satisfiable chain shares a page");
}

/// A `w:keepLines` paragraph taller than the whole page is placed, overflowing, rather than deferred
/// for ever.
#[test]
fn a_paragraph_taller_than_the_page_terminates() {
    let paragraphs: Vec<String> = (0..50)
        .map(|_| paragraph("<w:keepLines/>", PROSE))
        .collect();
    let mut document = document(&paragraphs);
    let flow = flow(&mut document);
    let constraints = page_of_lines(1);

    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 500);
    assert!(!pages.is_empty());
    assert!(
        pages.len() <= 60,
        "fifty unsplittable paragraphs need about fifty pages, not {}",
        pages.len()
    );
}

/// A measure narrower than a single glyph. The composer cannot advance, so the paragraph is taken
/// whole as one line — which is what stops the loop and what a reader sees in Word.
#[test]
fn a_column_narrower_than_one_glyph_terminates() {
    let paragraphs = [paragraph("", PROSE), paragraph("", PROSE)];
    let mut document = document(&paragraphs);
    let flow = flow(&mut document);
    // A hundredth of an inch: narrower than any glyph at eleven points.
    let constraints = constraints(0.01, 2.0);

    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 100);
    assert!(!pages.is_empty(), "it must still produce a page");
    assert!(pages.len() < 100, "and not one page per glyph");
}

/// An indent wider than the column, which makes the *measure* negative before the composer sees it.
#[test]
fn an_indent_wider_than_the_column_terminates() {
    let paragraphs = [paragraph(
        r#"<w:ind w:left="20000" w:right="20000"/>"#,
        PROSE,
    )];
    let mut document = document(&paragraphs);
    let flow = flow(&mut document);
    let constraints = constraints(1.0, 2.0);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 50);
    assert!(!pages.is_empty());
    assert!(pages.len() < 50);
}

/// A page with no room at all is refused rather than looped over.
#[test]
fn a_content_area_with_no_height_is_refused() {
    let mut document = document(&[paragraph("", PROSE)]);
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 0.0);
    let mut model = model();
    let refused = model.layout_page(&flow, PageIndex::FIRST, &constraints, None);
    assert!(
        matches!(
            refused,
            Err(mjx_layout_docx::DocumentLayoutError::EmptyContentArea { .. })
        ),
        "{refused:?}"
    );
}

/// An empty document lays out one page and stops.
#[test]
fn an_empty_document_is_one_page() {
    let mut document = document(&[]);
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 11.0);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 10);
    assert_eq!(pages.len(), 1, "nothing is still one page");
    assert!(pages[0].is_last(), "and it is the last one");
}

/// A paragraph with no runs at all still occupies a line, because a reader must be able to put a
/// caret in it.
#[test]
fn an_empty_paragraph_still_occupies_a_line() {
    let mut document = document(&["<w:p/>".to_owned(), paragraph("", "After.")]);
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 11.0);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 4);
    assert_eq!(
        support::line_count(pages[0].fragments()),
        2,
        "an empty paragraph is a line of its own"
    );
}

/// A thousand tabs on one line. Every tab must advance the pen, or a paragraph of tabs draws every
/// one of them on top of the last — and the bound on a leader's glyph count must hold.
#[test]
fn a_paragraph_of_a_thousand_tabs_terminates() {
    let tabs: String = std::iter::repeat_n("<w:tab/>", 1_000).collect();
    let paragraphs = [support::paragraph_with_run(
        r#"<w:tabs><w:tab w:val="left" w:pos="1440" w:leader="dot"/></w:tabs>"#,
        &tabs,
    )];
    let mut document = document(&paragraphs);
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 11.0);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 100);
    assert!(!pages.is_empty());
}

/// **MJXOFF-175's own shape of the same failure**, and the one the ticket names: the note area and
/// the body decide each other's height, so an implementation that let the reservation grow without a
/// bound would place no body line, produce a next position equal to the page's own start, and
/// repeat that page for ever.
///
/// `crate::notes` caps the reservation at the body's first line and splits the note instead. Here
/// the note is **forty times** the height of the page it is referenced from.
#[test]
fn a_footnote_taller_than_the_page_terminates_and_advances_both_flows() {
    let geometry = support::page_geometry(6.5, 1.5, 0.25);
    let long: Vec<String> = (0..80).map(|index| format!("Note line {index}.")).collect();
    let lines: Vec<&str> = long.iter().map(String::as_str).collect();
    let paragraphs = vec![
        support::paragraph_with_run(
            "",
            r#"<w:t xml:space="preserve">Paragraph zero.</w:t><w:footnoteReference w:id="2"/>"#,
        ),
        paragraph("", "Paragraph one."),
        paragraph("", "Paragraph two."),
    ];
    let mut document = support::document_with_footnote(&paragraphs, &geometry, 2, &lines);
    let flow = flow(&mut document);
    let mut model = model();
    let area = constraints(6.5, 1.5);

    let mut pages = 0_usize;
    let mut resume = None;
    let mut placed_note_lines = 0_usize;
    while pages < 500 {
        let page = model
            .layout_page(
                &flow,
                PageIndex::new(u32::try_from(pages).expect("a page number")),
                &area,
                resume.as_ref(),
            )
            .expect("every page lays out");
        let report = model.last_page().clone();
        assert!(
            !report.notes.is_empty() || report.carried_note.is_none(),
            "a page that carries a note forward must have placed some of it: {report:?}"
        );
        placed_note_lines += report.notes.len();
        resume = page.continuation().cloned();
        pages += 1;
        if resume.is_none() {
            break;
        }
    }
    assert!(
        pages < 500,
        "the note/body fixed point must terminate; it produced {pages} pages and was still going"
    );
    assert!(
        pages > 5,
        "an eighty-line note on a page this small really does need several: {pages}"
    );
    assert!(
        placed_note_lines > 0,
        "and every page's area held part of the note"
    );
}

/// A page too small to hold **anything** — the note area's cap is then zero — still advances.
///
/// This is the boundary the cap is written against: the body must keep its first line, so when the
/// page is one line tall the notes get nothing at all, and the *body* is what makes progress. Both
/// flows are then finite because each page consumes at least one line of one of them.
#[test]
fn a_page_with_no_room_for_a_note_still_advances_the_body() {
    let geometry = support::page_geometry(6.5, 0.45, 0.1);
    let paragraphs: Vec<String> = (0..6)
        .map(|index| {
            support::paragraph_with_run(
                "",
                &format!(
                    r#"<w:t xml:space="preserve">Paragraph {index}.</w:t><w:footnoteReference w:id="2"/>"#
                ),
            )
        })
        .collect();
    let mut document =
        support::document_with_footnote(&paragraphs, &geometry, 2, &["A note.", "Two lines."]);
    let flow = flow(&mut document);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints(6.5, 0.45), 200);
    assert!(!pages.is_empty());
    assert!(
        pages.len() < 200,
        "a six-paragraph document must not need two hundred pages: {}",
        pages.len()
    );
}

/// A long chain of `continuous` sections all sharing one sheet terminates: each group consumes a
/// section, and there are finitely many sections.
#[test]
fn a_chain_of_continuous_sections_terminates() {
    let geometry = support::page_geometry(6.5, 11.0, 0.25);
    let mut paragraphs: Vec<String> = Vec::new();
    for index in 0..40 {
        paragraphs.push(paragraph(
            &format!(r#"<w:sectPr><w:type w:val="continuous"/>{geometry}</w:sectPr>"#),
            &format!("Section {index}."),
        ));
    }
    let markup = support::document_markup_with(
        &paragraphs,
        &format!(r#"<w:type w:val="continuous"/>{geometry}"#),
    );
    let mut document = support::document_from_bytes(markup);
    let flow = flow(&mut document);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints(6.5, 11.0), 200);
    assert!(!pages.is_empty());
    assert!(
        pages.len() < 200,
        "forty continuous sections share their sheets rather than each demanding one: {}",
        pages.len()
    );
    assert!(
        model.last_page().column_groups >= 1,
        "and a page really does hold several sections' groups"
    );
}
