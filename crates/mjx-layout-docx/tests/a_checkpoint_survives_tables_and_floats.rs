//! The resumption property, re-measured over the content MJXOFF-176 added.
//!
//! # Why this is a separate suite rather than a fixture in the old one
//!
//! `a_checkpoint_is_work_not_output.rs` proves the checkpoint earns its keep on a document made of
//! paragraphs. Tables and floats both change *what a block layout costs*: a table lays out every
//! paragraph in every cell, and a paragraph that anchors a float is laid out **twice** — once to
//! settle its own top and once against the exclusion that top produced. Either could have turned a
//! constant-cost resumption into a linear one without any assertion about the *fragments* noticing,
//! which is exactly the failure MJXOFF-174 built [`mjx_layout_docx::DocumentBoxModel::paragraphs_visited`]
//! to catch.
//!
//! So the three assertions are R19's three, over a document with both:
//!
//! 1. the resumed page and the walked page are **the same page, EMU for EMU**;
//! 2. the resumed one visits a bounded number of blocks;
//! 3. and — the one that makes the other two mean something — **withholding the checkpoint makes
//!    the cost scale with the page number while supplying it does not.** A constant is not evidence
//!    of anything; a line through the origin is.

mod support;

use mjx_layout::{BoxModel, PageIndex};
use support::{
    constraints, document, floating_paragraph, flow, model, offset_position, paragraph, row,
    snapshot, square_wrap, table, text_cell,
};

/// Six hundred blocks: every tenth a four-row table, every fifth of the rest a paragraph with a
/// wrapped picture beside it, and the remainder ordinary text.
fn mixed_body() -> Vec<String> {
    let mut body: Vec<String> = Vec::new();
    for index in 0..600 {
        if index % 10 == 0 {
            let rows: Vec<String> = (0..4)
                .map(|r| {
                    row(
                        "",
                        &[
                            text_cell("", &format!("table {index} row {r} left")),
                            text_cell("", "right"),
                        ],
                    )
                })
                .collect();
            body.push(table("", &[3000, 3000], &rows));
        } else if index % 5 == 0 {
            body.push(floating_paragraph(
                "",
                &format!("paragraph {index} with a picture beside it and some words after"),
                457_200,
                457_200,
                &offset_position(0, 0),
                &square_wrap("bothSides"),
            ));
        } else {
            body.push(paragraph(
                "",
                &format!("paragraph {index} with enough words to make a line or two"),
            ));
        }
    }
    body
}

#[test]
fn a_page_of_tables_and_floats_is_the_same_page_resumed_or_walked() {
    let body = mixed_body();
    let mut document = document(&body);
    let flow = flow(&mut document);
    let constraints = constraints(6.0, 4.0);
    let target = PageIndex::new(20);

    let mut walked_model = model();
    let walked = walked_model
        .layout_page(&flow, target, &constraints, None)
        .expect("the page lays out from the beginning");
    let walked_cost = walked_model.paragraphs_visited();

    let mut resumed_model = model();
    let mut resume = None;
    for page in 0..=20_u32 {
        let laid = resumed_model
            .layout_page(&flow, PageIndex::new(page), &constraints, resume.as_ref())
            .expect("the page lays out from a checkpoint");
        resume = laid.continuation().cloned();
        if page == 20 {
            assert_eq!(
                snapshot(laid.fragments()),
                snapshot(walked.fragments()),
                "the resumed page and the walked page are the same page"
            );
        }
    }
    let resumed_cost = resumed_model.paragraphs_visited();

    assert!(
        resumed_cost * 4 < walked_cost,
        "resuming must be far cheaper than walking: {resumed_cost} against {walked_cost}"
    );
}

#[test]
fn withholding_the_checkpoint_makes_the_cost_scale_with_the_page_number() {
    let body = mixed_body();
    let mut document = document(&body);
    let flow = flow(&mut document);
    let constraints = constraints(6.0, 4.0);

    // Walked: the cost of page N includes every page before it, so it grows.
    let mut walked: Vec<u32> = Vec::new();
    for page in [10_u32, 20, 40] {
        let mut engine = model();
        engine
            .layout_page(&flow, PageIndex::new(page), &constraints, None)
            .expect("it lays out");
        walked.push(engine.paragraphs_visited());
    }
    assert!(
        walked[0] < walked[1] && walked[1] < walked[2],
        "without a checkpoint the cost is a line through the origin: {walked:?}"
    );

    // Resumed: each page from the last, and the cost does not grow with the page number.
    let mut engine = model();
    let mut resume = None;
    let mut resumed: Vec<u32> = Vec::new();
    for page in 0..=40_u32 {
        let laid = engine
            .layout_page(&flow, PageIndex::new(page), &constraints, resume.as_ref())
            .expect("it lays out");
        resume = laid.continuation().cloned();
        resumed.push(engine.paragraphs_visited());
        if resume.is_none() {
            break;
        }
    }
    let sampled: Vec<u32> = [10_usize, 20, 40]
        .iter()
        .filter_map(|page| resumed.get(*page).copied())
        .collect();
    assert_eq!(sampled.len(), 3, "the fixture reaches page forty");
    let spread = sampled.iter().max().unwrap_or(&0) - sampled.iter().min().unwrap_or(&0);
    assert!(
        spread <= 8,
        "with a checkpoint the cost is flat, not a line: {sampled:?}"
    );
    assert!(
        *sampled.iter().max().unwrap_or(&0) < walked[0],
        "and every resumed page is cheaper than the cheapest walked one: {sampled:?} against \
         {walked:?}"
    );
}

#[test]
fn a_block_that_anchors_a_float_is_laid_out_at_most_twice() {
    // The float path lays a block out a second time on purpose — once to settle its own top, once
    // against the exclusion that top produced — and this is the assertion that the second pass does
    // not become a third. One page of nothing but floated paragraphs: if each cost three layouts
    // rather than two, this number would be half as large again.
    let body: Vec<String> = (0..40)
        .map(|index| {
            floating_paragraph(
                "",
                &format!("paragraph {index} with a picture beside it and words after it"),
                457_200,
                457_200,
                &offset_position(0, 0),
                &square_wrap("bothSides"),
            )
        })
        .collect();
    let mut document = document(&body);
    let flow = flow(&mut document);
    let constraints = constraints(6.0, 4.0);

    let mut engine = model();
    let page = engine
        .layout_page(&flow, PageIndex::FIRST, &constraints, None)
        .expect("page one lays out");
    let placed = support::paragraphs_on(page.fragments()).len();
    let visited = engine.paragraphs_visited();
    assert!(placed > 0, "the page holds some of them");
    assert!(
        visited <= u32::try_from(placed + 2).unwrap_or(u32::MAX) * 2,
        "each block on the page costs at most two layouts: {visited} for {placed} blocks"
    );
}
