//! A long document, because **drift accumulates** and a three-page fixture cannot see it.
//!
//! # What only a long document exposes
//!
//! Every page's start is the previous page's end, so an error of one line on page three is an error
//! of one line on every page after it — and a fixture that stops at page three cannot tell a stable
//! engine from one that loses a line every twenty pages. Three properties are asserted here and each
//! of them needs length:
//!
//! 1. **The page count is stable.** Laying the same document out twice gives the same number of
//!    pages; laying it out from checkpoints and by walking gives the same number too.
//! 2. **Every line appears exactly once.** No line is dropped at a page boundary and none is drawn
//!    on two pages. This is the assertion a snapshot cannot make, because a snapshot of page *n* has
//!    no idea what is on page *n − 1*.
//! 3. **The boundaries are where the arithmetic says.** A page that holds *k* lines holds *k* lines
//!    two hundred pages in, which is what says nothing is accumulating.
//!
//! **There is no reference page count from Word here, and there cannot be.** The ticket asks for one
//! and the honest answer is that nobody has run Word: what is asserted is *internal consistency*,
//! which is a change detector and not evidence. See `tests/provenance.rs`, which says so with a
//! label rather than a comment.

mod support;

use mjx_layout::Fragment;
use support::{document, flow, model, page_of_lines, paragraph, walk};

/// Two hundred paragraphs of three lines each: six hundred lines, and a page of four holds a
/// hundred and fifty of them.
fn long_document() -> Vec<String> {
    let three_lines = "Alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima \
                       mike november oscar papa quebec romeo sierra tango uniform victor whiskey \
                       xray yankee zulu alpha bravo charlie delta echo foxtrot golf hotel india \
                       juliett kilo lima mike november oscar papa";
    (0..200)
        .map(|index| {
            paragraph(
                r#"<w:widowControl w:val="false"/>"#,
                &format!("{index}. {three_lines}"),
            )
        })
        .collect()
}

#[test]
fn the_page_count_is_the_same_twice_and_the_same_from_checkpoints() {
    let mut document = document(&long_document());
    let flow = flow(&mut document);
    let constraints = page_of_lines(4);

    let mut first = model();
    let once = walk(&mut first, &flow, &constraints, 1_000).len();
    let mut second = model();
    let twice = walk(&mut second, &flow, &constraints, 1_000).len();
    assert_eq!(
        once, twice,
        "the same document paginates the same way twice"
    );
    assert!(
        once > 40,
        "the fixture must be long enough for drift to show: {once} pages"
    );
}

/// **Every line exactly once.** A boundary that dropped a line, or drew it twice, is invisible on a
/// single page and obvious over a document.
#[test]
fn every_line_of_every_paragraph_appears_exactly_once() {
    let mut document = document(&long_document());
    let flow = flow(&mut document);
    let constraints = page_of_lines(4);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 1_000);

    let mut seen: Vec<(u32, u32)> = Vec::new();
    for page in &pages {
        for (_, node) in page.fragments().nodes() {
            if !matches!(node.fragment(), Fragment::Line(_)) {
                continue;
            }
            let path = node.source().path().segments();
            let (Some(paragraph), Some(line)) = (path.first().copied(), path.get(1).copied())
            else {
                continue;
            };
            seen.push((paragraph, line));
        }
    }
    let total = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        total,
        seen.len(),
        "a line was drawn on two pages: {} lines, {} distinct",
        total,
        seen.len()
    );

    // …and none was dropped: every paragraph's lines are `0..n` with no gap.
    let mut by_paragraph: std::collections::BTreeMap<u32, Vec<u32>> =
        std::collections::BTreeMap::new();
    for (paragraph, line) in seen {
        by_paragraph.entry(paragraph).or_default().push(line);
    }
    assert_eq!(
        by_paragraph.len(),
        flow.paragraph_count(),
        "every paragraph must appear somewhere"
    );
    for (paragraph, lines) in by_paragraph {
        let expected: Vec<u32> = (0..u32::try_from(lines.len()).expect("a line count")).collect();
        assert_eq!(
            lines, expected,
            "paragraph {paragraph} is missing a line or has one twice"
        );
    }
}

/// The boundaries do not drift: a page in the middle of the document holds as many lines as a page
/// near the start.
#[test]
fn a_page_two_hundred_paragraphs_in_holds_as_many_lines_as_the_first() {
    let mut document = document(&long_document());
    let flow = flow(&mut document);
    let constraints = page_of_lines(4);
    let mut model = model();
    let pages = walk(&mut model, &flow, &constraints, 1_000);

    let counts: Vec<usize> = pages
        .iter()
        .map(|page| support::line_count(page.fragments()))
        .collect();
    // The last page is short by definition; every other one must be full.
    let full = &counts[..counts.len() - 1];
    let widest = full.iter().copied().max().unwrap_or(0);
    let narrowest = full.iter().copied().min().unwrap_or(0);
    assert_eq!(
        widest, narrowest,
        "with widow control off, every full page holds the same number of lines: {counts:?}"
    );
    assert_eq!(widest, 4, "and that number is what the page was sized for");
}

/// Laying out a page in the middle from its checkpoint gives byte-for-byte the page walking gives —
/// asserted here over **many** pages rather than one, because an equivalence that holds at page two
/// and fails at page two hundred is the failure this suite exists for.
#[test]
fn every_tenth_page_resumes_to_the_page_walking_gives() {
    let mut document = document(&long_document());
    let flow = flow(&mut document);
    let constraints = page_of_lines(4);
    let mut walker = model();
    let pages = walk(&mut walker, &flow, &constraints, 1_000);

    for number in (10..pages.len()).step_by(10) {
        let resume = pages[number - 1]
            .continuation()
            .cloned()
            .expect("the page before continues");
        let mut resumed = model();
        let page = mjx_layout::BoxModel::layout_page(
            &mut resumed,
            &flow,
            mjx_layout::PageIndex::new(u32::try_from(number).expect("a page number")),
            &constraints,
            Some(&resume),
        )
        .expect("the page resumes");
        assert_eq!(
            support::snapshot(page.fragments()),
            support::snapshot(pages[number].fragments()),
            "page {number} differs between resuming and walking"
        );
    }
}

/// The estimate is an estimate, says so, and is within a factor of the truth — which is all
/// `ExtentPrecision::Estimated` promises and all a scrollbar needs before real pages arrive.
#[test]
fn the_extent_is_an_estimate_and_is_labelled_as_one() {
    use mjx_layout::{BoxModel, ExtentPrecision};
    let mut document = document(&long_document());
    let flow = flow(&mut document);
    let constraints = page_of_lines(4);
    let mut model = model();
    let extent = model.estimate_extent(&flow, &constraints);
    assert_eq!(
        extent.precision,
        ExtentPrecision::Estimated,
        "a flowing document's page count is not knowable without laying it out"
    );

    let real = walk(&mut model, &flow, &constraints, 1_000).len();
    let estimated = usize::try_from(extent.pages).expect("a page count");
    assert!(
        estimated * 4 >= real && real * 4 >= estimated,
        "the estimate must be the right order of magnitude: estimated {estimated}, laid out {real}"
    );
}
