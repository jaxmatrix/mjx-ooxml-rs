//! The four pagination constraints, each asserted by **page assignment** and each shown to fail
//! when it is switched off.
//!
//! # The gate this file refuses to be
//!
//! *"The document paginates"* is green for an implementation that breaks every page at a fixed line
//! count and honours nothing at all, and a short fixture cannot tell the two apart. Neither can a
//! gate on a page's *content*: the same paragraphs are in the same order either way, only on
//! different pages.
//!
//! So every test here is a **pair**. The same document is laid out twice, differing only in the one
//! attribute under test, and the assertion is that a *named paragraph lands on a different page*. If
//! the constraint were not implemented, both halves of the pair would give the same answer and the
//! test would fail — which is the property `assert_ne!` states directly.

mod support;

use support::{document, flow, model, page_of_each_paragraph, page_of_lines, page_of_paragraph};

/// Three words per line at 6.5 inches, so a paragraph of this text is three lines exactly.
const THREE_LINES: &str = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo \
                           lima mike november oscar papa quebec romeo sierra tango uniform \
                           victor whiskey xray yankee zulu alpha bravo charlie delta echo \
                           foxtrot golf hotel india juliett kilo lima mike november oscar papa";

// -------------------------------------------------------------------------------------------
// 1. `w:pageBreakBefore`
// -------------------------------------------------------------------------------------------

#[test]
fn page_break_before_moves_its_paragraph_to_the_next_page() {
    let constraints = page_of_lines(6);
    let with = [
        support::paragraph("", "First."),
        support::paragraph("<w:pageBreakBefore/>", "Second."),
        support::paragraph("", "Third."),
    ];
    let without = [
        support::paragraph("", "First."),
        support::paragraph("", "Second."),
        support::paragraph("", "Third."),
    ];

    let honoured = page_of_paragraph(&with, &constraints, 1);
    let ignored = page_of_paragraph(&without, &constraints, 1);

    assert_eq!(
        ignored,
        Some(0),
        "all three fit on one page without the break"
    );
    assert_eq!(
        honoured,
        Some(1),
        "the break puts the second paragraph on page two"
    );
    assert_ne!(
        honoured, ignored,
        "a `w:pageBreakBefore` that changed no page assignment was not implemented"
    );
}

/// The one place `w:pageBreakBefore` is deliberately not honoured: the very first paragraph, where
/// it would open the document with a blank page. **GUESS**, marked at the site.
#[test]
fn page_break_before_on_the_first_paragraph_opens_no_blank_page() {
    let constraints = page_of_lines(6);
    let paragraphs = [
        support::paragraph("<w:pageBreakBefore/>", "First."),
        support::paragraph("", "Second."),
    ];
    assert_eq!(page_of_paragraph(&paragraphs, &constraints, 0), Some(0));
}

// -------------------------------------------------------------------------------------------
// 2. `w:keepLines`
// -------------------------------------------------------------------------------------------

#[test]
fn keep_lines_together_moves_a_paragraph_that_would_have_split() {
    // Four lines fit on a page. Paragraph 0 takes two, leaving room for two of paragraph 1's three
    // — so without `w:keepLines` paragraph 1 starts on page one and continues on page two, and with
    // it the whole paragraph moves.
    let constraints = page_of_lines(4);
    let two_lines = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                     mike november oscar papa quebec romeo sierra tango uniform victor";
    // Widow control is **off** on the paragraph under test, and deliberately so: it defaults to on
    // and would move the same paragraph for its own reason, which would make this pair green for an
    // implementation that had never heard of `w:keepLines`. One constraint per fixture.
    let with = [
        support::paragraph("", two_lines),
        support::paragraph(
            r#"<w:keepLines/><w:widowControl w:val="false"/>"#,
            THREE_LINES,
        ),
    ];
    let without = [
        support::paragraph("", two_lines),
        support::paragraph(r#"<w:widowControl w:val="false"/>"#, THREE_LINES),
    ];

    assert_eq!(
        page_of_paragraph(&without, &constraints, 1),
        Some(0),
        "without the constraint the paragraph starts on page one and splits"
    );
    assert_eq!(
        page_of_paragraph(&with, &constraints, 1),
        Some(1),
        "`w:keepLines` moves the whole paragraph rather than splitting it"
    );
}

/// The termination case: a `w:keepLines` paragraph taller than a whole page must be **placed**,
/// overflowing, rather than pushed forward for ever.
#[test]
fn a_keep_lines_paragraph_taller_than_the_page_is_placed_anyway() {
    let constraints = page_of_lines(2);
    let paragraphs = [support::paragraph(
        r#"<w:keepLines/><w:widowControl w:val="false"/>"#,
        THREE_LINES,
    )];
    assert_eq!(page_of_paragraph(&paragraphs, &constraints, 0), Some(0));
}

// -------------------------------------------------------------------------------------------
// 3. `w:widowControl`
// -------------------------------------------------------------------------------------------

/// An **orphan**: one line of a three-line paragraph would be left alone at the foot of a page.
///
/// `w:widowControl` defaults to **on** (§17.3.1.44), so the fixture that switches it off is the one
/// that has to say so — which is why the pair below is `w:val="false"` against nothing at all rather
/// than the other way round.
#[test]
fn widow_control_refuses_to_leave_one_line_behind() {
    // Three lines fit. Paragraph 0 takes two, so one line of paragraph 1 would sit alone at the
    // foot of page one.
    let constraints = page_of_lines(3);
    let two_lines = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                     mike november oscar papa quebec romeo sierra tango uniform victor";
    let on = [
        support::paragraph("", two_lines),
        support::paragraph("", THREE_LINES),
    ];
    let off = [
        support::paragraph("", two_lines),
        support::paragraph(r#"<w:widowControl w:val="false"/>"#, THREE_LINES),
    ];

    assert_eq!(
        page_of_paragraph(&off, &constraints, 1),
        Some(0),
        "with widow control off the orphan line stays at the foot of page one"
    );
    assert_eq!(
        page_of_paragraph(&on, &constraints, 1),
        Some(1),
        "with widow control on the whole paragraph moves rather than orphan one line"
    );
}

/// A **widow**: one line of a paragraph would be carried alone to the top of the next page, so a
/// second is sent with it — which changes how many lines page one holds.
#[test]
fn widow_control_sends_a_second_line_over_with_a_lone_one() {
    // Four lines fit; paragraph 0 is one line and paragraph 1 is three, so without the rule page one
    // would hold all four and nothing would carry over at all. Five lines of paragraph text against
    // a four-line page is what produces a widow.
    let constraints = page_of_lines(4);
    let four_lines = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                      mike november oscar papa quebec romeo sierra tango uniform victor whiskey \
                      xray yankee zulu alpha bravo charlie delta echo foxtrot golf hotel india \
                      juliett kilo lima mike november oscar papa quebec romeo sierra tango \
                      uniform victor whiskey xray yankee zulu alpha bravo charlie delta";
    let on = [support::paragraph("", four_lines)];
    let off = [support::paragraph(
        r#"<w:widowControl w:val="false"/>"#,
        four_lines,
    )];

    let mut on_document = document(&on);
    let on_flow = flow(&mut on_document);
    let mut on_model = model();
    let on_pages = support::walk(&mut on_model, &on_flow, &constraints, 4);

    let mut off_document = document(&off);
    let off_flow = flow(&mut off_document);
    let mut off_model = model();
    let off_pages = support::walk(&mut off_model, &off_flow, &constraints, 4);

    let on_first = support::line_count(on_pages[0].fragments());
    let off_first = support::line_count(off_pages[0].fragments());
    assert!(
        off_pages.len() < 2 || on_first < off_first,
        "widow control must move a line off page one when the next page would hold a lone one: \
         on {on_first}, off {off_first}"
    );
}

// -------------------------------------------------------------------------------------------
// 4. `w:keepNext`
// -------------------------------------------------------------------------------------------

#[test]
fn keep_with_next_moves_a_heading_on_to_its_own_paragraph() {
    // Three lines fit. Paragraph 0 takes two, paragraph 1 is a one-line "heading" that fits at the
    // foot of page one, and paragraph 2 is three lines that do not. With `w:keepNext` on the
    // heading, the heading must move to page two so that it shares a page with what follows it.
    let constraints = page_of_lines(3);
    let two_lines = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                     mike november oscar papa quebec romeo sierra tango uniform victor";
    let with = [
        support::paragraph("", two_lines),
        support::paragraph("<w:keepNext/>", "A heading."),
        support::paragraph("", THREE_LINES),
    ];
    let without = [
        support::paragraph("", two_lines),
        support::paragraph("", "A heading."),
        support::paragraph("", THREE_LINES),
    ];

    assert_eq!(
        page_of_paragraph(&without, &constraints, 1),
        Some(0),
        "without the constraint the heading sits at the foot of page one"
    );
    assert_eq!(
        page_of_paragraph(&with, &constraints, 1),
        Some(1),
        "`w:keepNext` moves the heading on to the page its paragraph starts"
    );
}

/// A chain of them all move together, and the paragraph they are kept with is on the same page as
/// the last of them.
#[test]
fn a_keep_with_next_chain_moves_as_one() {
    // Five lines to a page: the chain is two one-line paragraphs and what they are kept with is
    // three lines, so the chain *is* satisfiable — five lines exactly. A four-line page would make
    // it unsatisfiable, and this test would then be asserting the termination rule instead, which is
    // `tests/termination.rs`'s subject.
    let constraints = page_of_lines(5);
    let three_lines = THREE_LINES;
    let paragraphs = [
        support::paragraph("", three_lines),
        support::paragraph("<w:keepNext/>", "Chain one."),
        support::paragraph("<w:keepNext/>", "Chain two."),
        support::paragraph("", three_lines),
    ];
    let pages = page_of_each_paragraph_for(&paragraphs, &constraints);
    assert_eq!(pages[0], Some(0), "the first paragraph stays on page one");
    assert_eq!(
        pages[1], pages[2],
        "both links of the chain must be on one page"
    );
    assert_eq!(
        pages[2], pages[3],
        "the chain must be on the same page as what it is kept with"
    );
    assert_eq!(pages[1], Some(1), "and that page is page two");
}

fn page_of_each_paragraph_for(
    paragraphs: &[String],
    constraints: &mjx_layout::Constraints,
) -> Vec<Option<usize>> {
    let mut document = document(paragraphs);
    let flow = flow(&mut document);
    let mut model = model();
    page_of_each_paragraph(&mut model, &flow, constraints, 20)
}
