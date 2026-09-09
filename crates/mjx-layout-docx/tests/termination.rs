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
