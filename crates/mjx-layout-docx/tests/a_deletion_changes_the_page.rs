//! **Revision marks change layout, not just colour** — asserted on the page count, which no amount
//! of tinting can change.
//!
//! # Why the page count and not the pixels
//!
//! A renderer that drew deletions struck through and insertions underlined, and measured the same
//! text in every display mode, would pass every assertion about colour, about strike-through, about
//! which author made a change, and about where a change bar goes. It would be **wrong**, and wrong
//! in the way that is hardest to see: the marks look right and every paragraph after the first
//! deletion is on a different page from the one Word puts it on.
//!
//! So the fixture carries a deletion long enough to change the page **count** between *All Markup*
//! and *No Markup*, and that count is the assertion. A tinting implementation produces the same
//! number in both and fails here.
//!
//! # The four modes, and the two that are not obvious
//!
//! *Simple Markup* measures what *No Markup* measures — Word's own simple view shows the document as
//! it would be with the changes accepted, and replaces the marks with a bar in the margin — so those
//! two paginate **identically** and differ only in [`RevisionView::shows_change_bars`]. *Original*
//! drops the insertions instead of the deletions, which is a third pagination again.
//!
//! # ⚠ And a document with no revisions paginates identically in all four
//!
//! Which is the trap the ticket names: a fixture without a tracked change cannot tell any of this
//! apart. [`a_document_without_revisions_is_the_same_in_every_view`] asserts that too, so a reader
//! knows the difference above came from the revisions rather than from the view.

mod support;

use mjx_layout_docx::RevisionView;
use support::generated::{deleted, inserted, plain_run, raw_paragraph};
use support::{constraints, document, flow, model, walk};

/// How many pages `paragraphs` takes in `view`.
fn pages_in(paragraphs: &[String], view: RevisionView) -> usize {
    let mut document = document(paragraphs);
    let read = flow(&mut document).with_revision_view(view);
    let mut model = model();
    walk(&mut model, &read, &constraints(3.0, 1.0), 40).len()
}

/// A body whose every paragraph is a short kept sentence followed by a **long** tracked deletion.
///
/// # Why the deletion is inside a paragraph rather than being one
///
/// A paragraph made entirely of deleted text still occupies a line when the deletion is hidden —
/// an empty paragraph is a line, because a reader must be able to put a caret in it — so a fixture
/// of deleted *paragraphs* paginates the same in both views and asserts nothing. The deletion has to
/// be **text on a line that also carries kept text**, so that hiding it removes lines rather than
/// emptying paragraphs. That is a real property of Word's own behaviour and it is exactly the kind
/// of thing a fixture gets wrong silently.
fn body_with_a_long_deletion() -> Vec<String> {
    let mut paragraphs = vec![raw_paragraph(&plain_run("the opening paragraph"))];
    for index in 0..8 {
        paragraphs.push(raw_paragraph(&format!(
            "{}{}",
            plain_run("keep "),
            deleted(
                100 + index,
                "Priya",
                &format!(
                    "a deleted sentence number {index} long enough to wrap over several lines of \
                     this narrow page and take a page of its own"
                ),
            )
        )));
    }
    paragraphs.push(raw_paragraph(&plain_run("the closing paragraph")));
    paragraphs
}

#[test]
fn the_same_document_paginates_differently_in_all_markup_and_no_markup() {
    let body = body_with_a_long_deletion();
    let with_marks = pages_in(&body, RevisionView::AllMarkup);
    let without = pages_in(&body, RevisionView::NoMarkup);
    assert!(
        with_marks > without,
        "showing the deletions must take more pages than hiding them: {with_marks} against \
         {without}"
    );
}

#[test]
fn simple_markup_paginates_exactly_as_no_markup_does() {
    // Word's *Simple Markup* shows the finished document and marks the changed lines with a bar. Its
    // **text** is No Markup's, and reading it as All Markup's is the mistake that would make a
    // reviewer's default view a different document from the one everyone else sees.
    let body = body_with_a_long_deletion();
    assert_eq!(
        pages_in(&body, RevisionView::SimpleMarkup),
        pages_in(&body, RevisionView::NoMarkup)
    );
    assert!(RevisionView::SimpleMarkup.shows_change_bars());
    assert!(!RevisionView::NoMarkup.shows_change_bars());
}

#[test]
fn the_original_view_drops_the_insertions_instead() {
    let mut paragraphs = vec![raw_paragraph(&plain_run("the opening paragraph"))];
    for index in 0..20 {
        paragraphs.push(raw_paragraph(&inserted(
            200 + index,
            "Priya",
            &format!("an inserted sentence number {index} of some length"),
        )));
    }
    paragraphs.push(raw_paragraph(&plain_run("the closing paragraph")));
    let final_view = pages_in(&paragraphs, RevisionView::NoMarkup);
    let original = pages_in(&paragraphs, RevisionView::Original);
    assert!(
        final_view > original,
        "the finished document holds the insertions and the original does not: {final_view} \
         against {original}"
    );
}

#[test]
fn an_insertion_is_measured_in_every_view_but_the_original() {
    // Before MJXOFF-177 `mjx-docx`'s residency skipped `w:ins` entirely, so a document with tracked
    // insertions laid out with the inserted text **missing** — every view rendered as though the
    // changes had been rejected. This is the assertion that would have caught it.
    let paragraphs = vec![raw_paragraph(&format!(
        "{}{}",
        plain_run("kept "),
        inserted(1, "Priya", "and added")
    ))];
    let mut document = document(&paragraphs);
    let read = flow(&mut document);
    let text = read.paragraphs()[0].text();
    assert!(
        text.contains("and added"),
        "an insertion is content, and its text has to be there: {text:?}"
    );
    assert_eq!(
        read.paragraphs()[0].revisions().len(),
        1,
        "and it is marked as an insertion rather than as ordinary text"
    );
}

#[test]
fn a_deletion_is_in_the_text_and_is_marked() {
    let paragraphs = vec![raw_paragraph(&format!(
        "{}{}",
        plain_run("kept "),
        deleted(1, "Priya", "and removed")
    ))];
    let mut document = document(&paragraphs);
    let read = flow(&mut document);
    let paragraph = &read.paragraphs()[0];
    assert!(
        paragraph.text().contains("and removed"),
        "the all-markup view is the whole string: {:?}",
        paragraph.text()
    );
    let span = &paragraph.revisions()[0];
    assert_eq!(span.kind, mjx_docx::RevisionKind::Deleted);
    assert_eq!(span.author.as_deref(), Some("Priya"));
    assert_eq!(
        paragraph.text().get(span.range.clone()),
        Some("and removed"),
        "and the span names exactly the deleted bytes"
    );
}

#[test]
fn a_document_without_revisions_is_the_same_in_every_view() {
    // The trap, asserted: a fixture with no tracked change cannot distinguish the four views, so a
    // suite built on one would be green for an implementation that ignored the setting entirely.
    let paragraphs: Vec<String> = (0..20)
        .map(|index| raw_paragraph(&plain_run(&format!("an ordinary paragraph {index}"))))
        .collect();
    let counts: Vec<usize> = [
        RevisionView::AllMarkup,
        RevisionView::SimpleMarkup,
        RevisionView::NoMarkup,
        RevisionView::Original,
    ]
    .into_iter()
    .map(|view| pages_in(&paragraphs, view))
    .collect();
    assert!(
        counts.windows(2).all(|pair| pair[0] == pair[1]),
        "with nothing tracked, every view is the same document: {counts:?}"
    );
}

#[test]
fn a_change_bar_is_reported_for_a_changed_paragraph_in_a_marking_view() {
    // The bar is *reported* and not drawn — this crate resolves no paint — so what is asserted is
    // the report, which is what `mjx-scene-docx` will draw from.
    let paragraphs = vec![raw_paragraph(&format!(
        "{}{}",
        plain_run("kept "),
        deleted(1, "Priya", "and removed")
    ))];
    for (view, expected) in [
        (RevisionView::AllMarkup, true),
        (RevisionView::SimpleMarkup, true),
        (RevisionView::NoMarkup, false),
        (RevisionView::Original, false),
    ] {
        let mut document = document(&paragraphs);
        let read = flow(&mut document).with_revision_view(view);
        let composed = mjx_layout_docx::compose(
            &read.paragraphs()[0],
            &mjx_layout_docx::Generated {
                view,
                ..mjx_layout_docx::Generated::default()
            },
        );
        assert_eq!(
            composed.change_bar(),
            expected,
            "{view:?} draws a change bar: {expected}"
        );
    }
}

#[test]
fn a_deletion_inside_an_insertion_is_reported_as_the_deletion() {
    // Word writes a `w:del` inside a `w:ins` for text one reviewer added and another removed, and
    // what a reader must see is that it was **deleted**. Reading the outer container would show it
    // in the finished document.
    let inner = deleted(2, "Sam", "second thoughts");
    let paragraphs = vec![raw_paragraph(&format!(
        r#"<w:ins w:id="1" w:author="Priya" w:date="2026-09-09T00:00:00Z">{inner}</w:ins>"#
    ))];
    let mut document = document(&paragraphs);
    let read = flow(&mut document);
    let paragraph = &read.paragraphs()[0];
    let covering = mjx_layout_docx::changes_anything(paragraph.revisions(), RevisionView::NoMarkup);
    assert!(
        covering,
        "the finished document does not hold text that was added and then removed"
    );
}
