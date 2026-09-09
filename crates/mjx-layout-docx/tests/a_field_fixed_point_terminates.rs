//! **The field/pagination fixed point terminates**, and its fallback is exercised by a document
//! that deliberately does not converge.
//!
//! # The cycle, in one sentence
//!
//! A `PAGE` field's value depends on where the page break fell; its *width* — one digit or two —
//! decides where the line breaks, therefore where the page breaks, therefore what the value is.
//!
//! # What is asserted here, and what is asserted elsewhere
//!
//! `crate::fields`'s own documentation carries the argument; this carries the two facts a reader of
//! a green run needs:
//!
//! 1. an ordinary document **converges**, and [`Convergence::Converged`] says in how many passes;
//! 2. a document that cannot converge **stops anyway**, at [`MAXIMUM_PASSES`], reporting
//!    [`Convergence::Exhausted`] — and a caller then chooses the cached results.
//!
//! The second is the one that matters. A fallback that is never exercised is a fallback that does
//! not work, and an oscillating document is unusual enough that no real fixture would reach it — so
//! the oscillator is written by hand, as a closure that returns a different environment every time.
//! Driving the loop rather than a document is deliberate there: it is the loop that has to
//! terminate, and a test that could only reach it through a `.docx` would be testing pagination.
//!
//! **And one test drives the loop with a real document**, because a loop that terminates over a
//! hand-written map and a loop that terminates over a pagination are two claims:
//! [`a_real_document_of_page_fields_settles`] paginates a multi-page document full of `PAGE` and
//! `NUMPAGES` fields, observes where every block landed, and asserts the second observation equals
//! the first.
//!
//! # And the two-assembly bound is asserted, not assumed
//!
//! [`a_page_of_fields_still_assembles_at_most_twice`] lays out a document whose header, footer and
//! body are all full of fields **and** which carries a footnote, and asserts
//! `PageReport::assemblies` is still at most two. That is the property MJXOFF-175 proved and
//! MJXOFF-176 preserved, and the one a length-changing field is the obvious way to break.

mod support;

use mjx_layout::{BoxModel, PageIndex};
use mjx_layout_docx::{resolve_fields, Convergence, FieldEnvironment, MAXIMUM_PASSES};
use support::generated::{complex_field, plain_run, raw_paragraph};
use support::{constraints, document_with_footnote, flow, model, page_geometry, walk};

#[test]
fn an_ordinary_document_reaches_the_fixed_point() {
    // The shape every real document has: pass zero knows nothing, pass one observes the pagination,
    // pass two observes the same pagination again and stops.
    let mut passes = 0_usize;
    let (environment, convergence) = resolve_fields(|_| -> Result<FieldEnvironment, ()> {
        passes += 1;
        let mut observed = FieldEnvironment::cached_results();
        observed.observe_total_pages(7);
        observed.observe_block(0, 1);
        Ok(observed)
    })
    .expect("the loop does not fail");
    assert_eq!(
        convergence,
        Convergence::Converged { passes: 2 },
        "pass one produced the environment, pass two agreed with it"
    );
    assert_eq!(passes, 2, "and it cost exactly two passes");
    assert_eq!(environment.total_pages(), Some(7));
}

#[test]
fn a_document_with_no_length_changing_field_converges_in_one_pass() {
    let (_, convergence) = resolve_fields(|_| -> Result<FieldEnvironment, ()> {
        Ok(FieldEnvironment::cached_results())
    })
    .expect("the loop does not fail");
    assert_eq!(
        convergence,
        Convergence::Converged { passes: 1 },
        "nothing was observed, so pass zero's own environment was already the answer"
    );
}

#[test]
fn a_document_that_oscillates_stops_and_says_so() {
    // The document `crate::fields` describes: a `NUMPAGES` field that is 9 when the document is ten
    // pages long and 10 when it is nine. It has no fixed point, and Word has the same problem — press
    // F9 twice and watch the number flip.
    let mut passes = 0_usize;
    let (environment, convergence) = resolve_fields(|_| -> Result<FieldEnvironment, ()> {
        passes += 1;
        let mut observed = FieldEnvironment::cached_results();
        observed.observe_total_pages(if passes.is_multiple_of(2) { 9 } else { 10 });
        Ok(observed)
    })
    .expect("the loop does not fail");
    assert_eq!(
        convergence,
        Convergence::Exhausted,
        "an oscillator is declared an oscillator rather than iterated for ever"
    );
    assert_eq!(
        passes, MAXIMUM_PASSES,
        "and it costs exactly the budget, not one pass more"
    );
    assert!(
        !convergence.converged(),
        "a caller reads this and falls back to the cached results"
    );
    assert!(
        environment.total_pages().is_some(),
        "the last environment is handed back, so a caller that would rather show it can"
    );
}

#[test]
fn the_fallback_is_the_cached_results_and_is_reachable() {
    // What a caller does with `Exhausted`: throw the last environment away and render what the file
    // says. This asserts the two are actually different objects, so the fallback is a real choice
    // rather than a name for the same thing.
    let (last, convergence) = resolve_fields(|environment| -> Result<FieldEnvironment, ()> {
        let mut observed = environment.clone();
        observed.observe_total_pages(i64::from(
            u8::try_from(observed.total_pages().unwrap_or(0) % 2 + 1).unwrap_or(1),
        ));
        Ok(observed)
    })
    .expect("the loop does not fail");
    assert_eq!(convergence, Convergence::Exhausted);
    assert_ne!(
        last,
        FieldEnvironment::cached_results(),
        "the fallback is a different environment from the last one, which is what makes it a choice"
    );
}

#[test]
fn a_real_document_of_page_fields_settles() {
    // The loop driven by a **pagination** rather than by a hand-written map. Every paragraph carries
    // a `PAGE` and a `NUMPAGES` field whose stored values are wrong, so pass one changes every one
    // of them and pass two has to agree with pass one for this to pass at all.
    //
    // It is the test that would catch a `PAGE` field reading *the page being assembled*: such a
    // field's value would depend on the assembly, so a paragraph laid out twice on one page would
    // observe two different values and the environment would never stop moving.
    let paragraphs: Vec<String> = (0..30)
        .map(|index| {
            raw_paragraph(&format!(
                "{}{}{}{}",
                plain_run(&format!("item {index}, page ")),
                complex_field(" PAGE ", "1"),
                plain_run(" of "),
                complex_field(" NUMPAGES ", "1")
            ))
        })
        .collect();
    let markup = support::document_markup(&paragraphs);
    let constraints = constraints(3.0, 1.0);

    // One pass: paginate under `environment` and report where every block landed.
    let observe = |environment: &FieldEnvironment| -> Result<FieldEnvironment, ()> {
        let mut document = support::document_from_bytes(markup.clone());
        let read = flow(&mut document).with_fields(environment.clone());
        let mut engine = model();
        let pages = walk(&mut engine, &read, &constraints, 200);
        let mut observed = FieldEnvironment::cached_results();
        observed.observe_total_pages(i64::try_from(pages.len()).unwrap_or(i64::MAX));
        for (number, page) in pages.iter().enumerate() {
            for block in support::paragraphs_on(page.fragments()) {
                observed.observe_block(
                    u32::try_from(block).unwrap_or(u32::MAX),
                    i64::try_from(number).unwrap_or(0) + 1,
                );
            }
        }
        Ok(observed)
    };

    let (environment, convergence) = resolve_fields(observe).expect("the loop does not fail");
    assert!(
        convergence.converged(),
        "a document of ordinary page fields settles: {convergence:?}"
    );
    assert!(
        environment.total_pages().is_some_and(|pages| pages > 1),
        "and the fixture is more than one page, so the fields had something to say"
    );
    assert_eq!(
        environment.page_of_block(0),
        Some(1),
        "the first block is on page one, which is the value its `PAGE` field renders"
    );
    let last = environment
        .page_of_block(29)
        .expect("the last block landed somewhere");
    assert!(
        last > 1,
        "and the last one is not, so the two fields render different values"
    );
}

#[test]
fn a_page_of_fields_still_assembles_at_most_twice() {
    // The property MJXOFF-175 proved and MJXOFF-176 preserved. A field is the obvious way to break
    // it, because its width changes with the page it is on — and it does not, because a
    // `FieldEnvironment` is constant while the document is laid out. See `crate::fields`.
    let paragraphs: Vec<String> = (0..40)
        .map(|index| {
            raw_paragraph(&format!(
                "{}{}{}",
                plain_run(&format!("paragraph {index} of page ")),
                complex_field(" PAGE ", "1"),
                plain_run(&format!(" of {index}"))
            ))
        })
        .chain(std::iter::once(support::referencing_note(
            true,
            2,
            "a paragraph that carries a footnote",
        )))
        .collect();
    let mut document = document_with_footnote(
        &paragraphs,
        &page_geometry(6.0, 3.0, 0.25),
        2,
        &[
            "a note long enough to take space",
            "and a second line of it",
        ],
    );
    let mut environment = FieldEnvironment::cached_results();
    environment.observe_total_pages(9);
    for block in 0..41_u32 {
        environment.observe_block(block, i64::from(block / 5 + 1));
    }
    let read = flow(&mut document).with_fields(environment);
    let mut model = model();
    let constraints = constraints(6.0, 3.0);
    let pages = walk(&mut model, &read, &constraints, 12);
    assert!(pages.len() > 2, "the fixture is more than one page long");

    let mut reached_two = false;
    let mut resume: Option<mjx_layout::Checkpoint> = None;
    for number in 0..pages.len() {
        let page = model
            .layout_page(
                &read,
                PageIndex::new(u32::try_from(number).expect("a page number")),
                &constraints,
                resume.as_ref(),
            )
            .expect("the page lays out");
        let report = model.last_page();
        assert!(
            report.assemblies <= 2,
            "page {number} needed {} assemblies; the bound is two and a field must not raise it",
            report.assemblies
        );
        reached_two |= report.assemblies == 2;
        resume = page.continuation().cloned();
        if resume.is_none() {
            break;
        }
    }
    assert!(
        reached_two,
        "and two is actually reached somewhere — a bound nothing reaches proves nothing"
    );
}
