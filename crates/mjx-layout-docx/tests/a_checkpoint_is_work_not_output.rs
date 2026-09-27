//! **The gate this whole crate is organised around**: laying out page *N* from *N−1*'s checkpoint
//! must produce the same page as walking to it, and must do a different amount of work.
//!
//! # Why a gate on the fragments proves nothing
//!
//! A checkpoint that is never read still produces correct output. An implementation that walked from
//! page one on every request would satisfy every assertion about *what is on page 200*, would pass
//! every snapshot, and would be unusable — which is the exact failure `mjx-layout`'s own
//! `Checkpoint` documentation names and the reason
//! [`DocumentBoxModel::paragraphs_visited`](mjx_layout_docx::DocumentBoxModel::paragraphs_visited)
//! exists.
//!
//! So there are two assertions here and they are not the same assertion twice:
//!
//! 1. **The output is identical** — resumed and walked pages snapshot the same, EMU for EMU. This is
//!    the equivalence `BoxModel::layout_page` promises, and without it a scrollbar shows a reader a
//!    different page depending on how they arrived at it.
//! 2. **The work is not** — the resumed page visits a handful of paragraphs and the walked one
//!    visits every paragraph before it. This is what says the checkpoint is *used*.
//!
//! And there is a third, which is what makes the second one honest: **the count scales with the page
//! number when the checkpoint is withheld.** A constant is not evidence of anything; a line through
//! the origin is.

mod support;

use mjx_layout::{BoxModel, PageIndex};
use support::{constraints, document, flow, model, snapshot, walk};

/// A document of `count` short paragraphs, each identifiable by its own text.
fn many(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| support::paragraph("", &format!("Paragraph number {index} of the document.")))
        .collect()
}

#[test]
fn page_forty_from_a_checkpoint_is_the_same_page_as_page_forty_by_walking() {
    let mut document = document(&many(400));
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 1.2);

    let mut walker = model();
    let pages = walk(&mut walker, &flow, &constraints, 41);
    assert!(
        pages.len() > 40,
        "the fixture must reach page 41; it reached {}",
        pages.len()
    );
    let walked = snapshot(pages[40].fragments());

    // The same page, laid out alone, from page 40's checkpoint.
    let mut resumed_model = model();
    let resume = pages[39]
        .continuation()
        .cloned()
        .expect("page 40 continues");
    let resumed = resumed_model
        .layout_page(&flow, PageIndex::new(40), &constraints, Some(&resume))
        .expect("page 41 resumes");

    assert_eq!(
        snapshot(resumed.fragments()),
        walked,
        "a resumed page and a walked page must be the same page, EMU for EMU"
    );
}

#[test]
fn resuming_does_bounded_work_and_walking_does_not() {
    let mut document = document(&many(400));
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 1.2);

    let mut walker = model();
    let pages = walk(&mut walker, &flow, &constraints, 41);
    let resume = pages[39]
        .continuation()
        .cloned()
        .expect("page 40 continues");

    let mut resumed_model = model();
    resumed_model
        .layout_page(&flow, PageIndex::new(40), &constraints, Some(&resume))
        .expect("page 41 resumes");
    let resumed_work = resumed_model.paragraphs_visited();

    let mut walking_model = model();
    walking_model
        .layout_page(&flow, PageIndex::new(40), &constraints, None)
        .expect("page 41 without a checkpoint");
    let walking_work = walking_model.paragraphs_visited();

    assert!(
        resumed_work <= 8,
        "a resumed page should look at the paragraphs on it; it looked at {resumed_work}"
    );
    assert!(
        walking_work > resumed_work * 8,
        "walking to page 41 must cost far more than resuming to it: walked {walking_work}, \
         resumed {resumed_work}"
    );
}

/// The falsification. **A constant work figure is not evidence** — an implementation that visited
/// one paragraph per page and cached everything would also be constant. What says the checkpoint is
/// load-bearing is that withholding it makes the cost *scale with the page number* while supplying
/// it does not.
#[test]
fn without_a_checkpoint_the_cost_scales_with_the_page_number_and_with_one_it_does_not() {
    let mut document = document(&many(400));
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 1.2);

    let mut walker = model();
    let pages = walk(&mut walker, &flow, &constraints, 41);

    let mut walked_cost = Vec::new();
    let mut resumed_cost = Vec::new();
    for page in [10_usize, 20, 40] {
        let mut cold = model();
        cold.layout_page(
            &flow,
            PageIndex::new(u32::try_from(page).expect("a page number")),
            &constraints,
            None,
        )
        .expect("a page without a checkpoint");
        walked_cost.push(cold.paragraphs_visited());

        let resume = pages[page - 1]
            .continuation()
            .cloned()
            .expect("the page before continues");
        let mut warm = model();
        warm.layout_page(
            &flow,
            PageIndex::new(u32::try_from(page).expect("a page number")),
            &constraints,
            Some(&resume),
        )
        .expect("a page from its checkpoint");
        resumed_cost.push(warm.paragraphs_visited());
    }

    assert!(
        walked_cost[0] < walked_cost[1] && walked_cost[1] < walked_cost[2],
        "without a checkpoint the work must grow with the page number: {walked_cost:?}"
    );
    let widest = resumed_cost.iter().copied().max().unwrap_or(0);
    let narrowest = resumed_cost.iter().copied().min().unwrap_or(0);
    assert!(
        widest.saturating_sub(narrowest) <= 2,
        "with a checkpoint the work must not grow with the page number: {resumed_cost:?}"
    );
}

/// A checkpoint made from a different document is refused rather than misread — the failure that
/// otherwise produces a plausible page of the wrong content.
#[test]
fn a_checkpoint_from_another_document_is_refused() {
    let mut long = document(&many(400));
    let long_flow = flow(&mut long);
    let constraints = constraints(6.5, 1.2);
    let mut walker = model();
    let pages = walk(&mut walker, &long_flow, &constraints, 3);
    let resume = pages[0]
        .continuation()
        .cloned()
        .expect("page one continues");

    let mut short = document(&many(9));
    let short_flow = flow(&mut short);
    let mut other = model();
    let refused = other.layout_page(&short_flow, PageIndex::new(1), &constraints, Some(&resume));
    assert!(
        matches!(
            refused,
            Err(mjx_layout_docx::DocumentLayoutError::StaleContinuation { .. })
        ),
        "a checkpoint from a 400-paragraph document must not be read against a 9-paragraph one: \
         {refused:?}"
    );
}

/// A checkpoint offered for the wrong page is refused by `mjx-layout` itself, which is the half of
/// the check every box model would otherwise have to remember to write.
#[test]
fn a_checkpoint_for_another_page_is_refused() {
    let mut document = document(&many(60));
    let flow = flow(&mut document);
    let constraints = constraints(6.5, 1.2);
    let mut walker = model();
    let pages = walk(&mut walker, &flow, &constraints, 4);
    let resume = pages[0]
        .continuation()
        .cloned()
        .expect("page one continues");

    let mut other = model();
    let refused = other.layout_page(&flow, PageIndex::new(3), &constraints, Some(&resume));
    assert!(
        matches!(
            refused,
            Err(mjx_layout_docx::DocumentLayoutError::Layout(
                mjx_layout::LayoutError::MisplacedCheckpoint { .. }
            ))
        ),
        "page 2's checkpoint must not lay out page 4: {refused:?}"
    );
}
