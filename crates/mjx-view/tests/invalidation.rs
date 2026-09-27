//! **Invalidation is minimal, not merely correct.**
//!
//! Correct is easy: drop everything on every edit and no stale page is ever shown. That
//! implementation passes every *correctness* test in this crate and makes a keystroke re-lay-out a
//! document, which is the defect §12's 30 ms keystroke-to-repaint budget exists to prevent.
//!
//! So the gate is a **count**, and it is asserted from both ends:
//!
//! * one keystroke into one paragraph of a twenty-page document drops **one** page;
//! * the identical edit, widened from a reformat to an insertion, drops **eighteen** — and the same
//!   edit against a box model that places absolutely drops one again.
//!
//! The second half is what makes the first mean something. A viewport that ignored the box model's
//! answer and dropped one page always would pass the first assertion and fail the second; one that
//! dropped everything always would fail the first.
//!
//! # What the session actually hands over
//!
//! `mjx-session` emits an [`Invalidation`] at **apply** time — the frame the edit happened in — and
//! it carries a [`ChangeKind`], which MJXOFF-168 added for exactly this reason: an address alone
//! says *where*, and the three kinds invalidate differently. Nothing here diffs a document. Two of
//! the cases below feed a real `Session`'s `drain_invalidations` straight into the viewport, so the
//! two crates are joined by a test rather than by a comment.

use mjx_layout::{ChangeKind, ExtentPrecision, LayoutSize, PageIndex};
use mjx_ooxml_core::measure::Emu;
use mjx_session::{
    Applied, Committed, Invalidation, ManualClock, MemoryDocument, MemoryJournal, Operation,
    ResidentDocument, Session, SessionError, Value,
};
use mjx_view::{CacheBudget, DocumentView, ManualFrameClock, RefinementTier, Stage, Viewport};

#[path = "support/mod.rs"]
mod support;

use support::{address, letter, FlowModel, Paragraphs, PlainScenes};

type View = DocumentView<FlowModel, PlainScenes>;

/// Twenty pages, every one of them materialised, so an over-wide invalidation has something to
/// destroy and a count can tell the difference.
fn warm(model: FlowModel) -> (Paragraphs, View) {
    let content = Paragraphs::of(20).with_blocks(8);
    let constraints = letter();
    let mut view = DocumentView::new(
        model,
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height: constraints.page.height,
        }),
        // Large enough that nothing is evicted, so every page dropped below was dropped by the
        // invalidation and not by the budget.
        CacheBudget::uniform(8 * 1024 * 1024),
    );
    let clock = ManualFrameClock::new();
    for page in 0..20 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&content, &clock, RefinementTier::Full)
            .expect("a frame");
    }
    let held = view
        .cache_report()
        .stage(Stage::Fragments)
        .expect("the stage")
        .entries;
    assert_eq!(held, 20, "the arrangement did not materialise every page");
    (content, view)
}

#[test]
fn one_keystroke_dirties_one_page_and_widening_it_dirties_the_rest() {
    let edited = PageIndex::new(2);

    // A reformat, in a flow model: one page.
    let (_content, mut view) = warm(FlowModel::flowing());
    let narrow = view.invalidate(&[Invalidation::at(&address(edited, 1))]);
    assert_eq!(
        narrow.fragments_suppressed, 1,
        "a reformat dirtied more than its own page"
    );
    assert_eq!(narrow.display_lists_suppressed, 1);
    assert_eq!(
        narrow.checkpoints_suppressed, 0,
        "a reformat cannot move the content after it, so no resume point is stale",
    );

    // **The same edit, widened.** A run whose text changed length is an insertion in flow terms, so
    // every page after it may move. Eighteen, not one.
    let (_content, mut view) = warm(FlowModel::flowing());
    let wide = view.invalidate(&[Invalidation::reflowing(&address(edited, 1))]);
    assert_eq!(
        wide.fragments_suppressed, 18,
        "widening the change did not widen what it dirtied, so the count above measures nothing",
    );
    assert!(
        wide.checkpoints_suppressed > 0,
        "the resume points after a reflow are derived from content that moved and are stale",
    );

    // And the third reading: the same widened edit against a box model that places absolutely —
    // PowerPoint's shape — dirties one page again, because nothing in a deck flows.
    let (_content, mut view) = warm(FlowModel::absolute());
    let absolute = view.invalidate(&[Invalidation::reflowing(&address(edited, 1))]);
    assert_eq!(
        absolute.fragments_suppressed, 1,
        "an absolutely-placing model reflowed, which it cannot do",
    );
}

#[test]
fn a_reflow_keeps_the_resume_point_that_leads_into_it() {
    // The off-by-one that would cost a whole document. A checkpoint *ending* page 4 is the state
    // page 5 resumes from; if the edit is on page 5, that checkpoint describes content that has not
    // changed and must survive. Dropping it would make the next jump re-lay-out from page 1.
    let (_content, mut view) = warm(FlowModel::flowing());
    let before = view
        .cache_report()
        .stage(Stage::Checkpoints)
        .expect("the stage")
        .entries;
    assert_eq!(before, 19, "nineteen resume points for twenty pages");

    let outcome = view.invalidate(&[Invalidation::reflowing(&address(PageIndex::new(5), 0))]);
    let after = view
        .cache_report()
        .stage(Stage::Checkpoints)
        .expect("the stage")
        .entries;
    assert_eq!(
        after, 5,
        "the checkpoints ending pages 1..=5 must survive an edit on page 6",
    );
    assert_eq!(outcome.checkpoints_suppressed, before - after);
}

#[test]
fn a_reflow_makes_the_documents_length_a_guess_again() {
    // The failure this closes is the one a scrollbar cannot recover from. A page that reported no
    // continuation ended the document, and from that moment its length is a *fact*. An insertion
    // undoes that — the document may now run past the page that used to end it — and a model that
    // kept the old figure would clamp the scrollbar short of content that exists.
    let content = Paragraphs::of(40).with_blocks(4).estimated_at(90);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height: constraints.page.height,
        }),
        CacheBudget::uniform(4 * 1024 * 1024),
    );
    let clock = ManualFrameClock::new();
    for page in 0..40 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&content, &clock, RefinementTier::Preview)
            .expect("a frame");
    }
    assert_eq!(view.scroll().pages(), 40);
    assert_eq!(view.scroll().count_precision(), ExtentPrecision::Exact);
    assert_eq!(view.stats().re_estimates, 0, "nothing had reflowed yet");

    // One insertion, on page 2 of what is now a forty-page document.
    view.invalidate(&[Invalidation::reflowing(&address(PageIndex::new(2), 0))]);
    // …and the **next frame** asks the box model how long the document is now, because that is
    // where the content is.
    view.scroll_to_page(PageIndex::new(2));
    view.frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert_eq!(view.stats().re_estimates, 1);
    assert_eq!(
        view.scroll().pages(),
        90,
        "the length is still the old measured figure, so an insertion past page 40 would be \
         unreachable",
    );
    assert_eq!(view.scroll().count_precision(), ExtentPrecision::Estimated);

    // And walking forward finds the end again, without failing on the pages the estimate invented.
    for page in 2..90 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&content, &clock, RefinementTier::Preview)
            .expect("a frame over an over-long estimate must not fail");
    }
    assert_eq!(view.scroll().pages(), 40);
    assert_eq!(view.scroll().count_precision(), ExtentPrecision::Exact);
    assert_eq!(
        view.stats().re_estimates,
        1,
        "the re-estimate ran on every frame rather than once per reflow",
    );
}

#[test]
fn an_empty_invalidation_drops_nothing_and_asks_the_box_model_nothing() {
    let (_content, mut view) = warm(FlowModel::flowing());
    let outcome = view.invalidate(&[]);
    assert_eq!(outcome.fragments_suppressed, 0);
    assert_eq!(outcome.pages, mjx_layout::DirtyPages::None);
    assert_eq!(
        view.cache_report()
            .stage(Stage::Fragments)
            .expect("the stage")
            .entries,
        20,
    );
}

#[test]
fn a_dropped_page_is_laid_out_again_and_the_others_are_not() {
    // Correctness beside minimality: the page that was dropped must come back, and the pages that
    // were not dropped must not be re-laid-out to bring it back.
    let (content, mut view) = warm(FlowModel::flowing());
    let before = view.stats().pages_laid_out;
    view.invalidate(&[Invalidation::at(&address(PageIndex::new(9), 3))]);
    assert!(view.fragments_for(PageIndex::new(9)).is_none());

    let clock = ManualFrameClock::new();
    view.scroll_to_page(PageIndex::new(9));
    view.frame(&content, &clock, RefinementTier::Full)
        .expect("a frame");
    assert!(view.fragments_for(PageIndex::new(9)).is_some());
    assert_eq!(
        view.stats().pages_laid_out - before,
        1,
        "bringing one page back cost more than one page of layout",
    );
}

// -------------------------------------------------------------------------------------------------
// The session's own stream, end to end.
// -------------------------------------------------------------------------------------------------

/// The smallest residency that can be edited: a string per address, and it reports the change kind
/// its caller asked for.
///
/// It exists so that the two crates are joined by a **test** rather than by a comment: the
/// invalidations below come out of a real [`Session::drain_invalidations`] and go straight into a
/// real [`DocumentView::invalidate`], and nothing in between translates them.
struct Typing {
    kind: ChangeKind,
    text: String,
}

impl ResidentDocument for Typing {
    fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError> {
        let was = Value::text(self.text.as_str());
        if let mjx_session::OperationKind::SetValue(Value::Text(text)) = operation.kind() {
            self.text = text.to_string();
        }
        Ok(Applied {
            inverse: Operation::set_value(operation.address().clone(), was),
            invalidation: match self.kind {
                ChangeKind::Reformatted => Invalidation::at(operation.address()),
                _ => Invalidation::reflowing(operation.address()),
            },
        })
    }

    fn dirty_bytes(&self) -> usize {
        self.text.len()
    }

    fn commit(&mut self) -> Result<Committed, SessionError> {
        Ok(Committed {
            parts_serialised: 1,
            bytes: self.text.clone().into_bytes(),
        })
    }
}

fn session(kind: ChangeKind) -> Session<Typing, ManualClock> {
    Session::new(
        Typing {
            kind,
            text: String::new(),
        },
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
}

#[test]
fn the_stream_a_session_emits_is_the_stream_the_viewport_consumes() {
    let edited = PageIndex::new(4);

    let mut typing = session(ChangeKind::Reformatted);
    typing
        .edit(Operation::set_value(address(edited, 2), Value::text("a")))
        .expect("an edit");
    let stream = typing.drain_invalidations();
    assert_eq!(stream.len(), 1, "one keystroke, one invalidation");
    assert_eq!(stream[0].kind(), ChangeKind::Reformatted);

    let (_content, mut view) = warm(FlowModel::flowing());
    assert_eq!(view.invalidate(&stream).fragments_suppressed, 1);

    // The same session shape, reporting a reflow, and the same viewport: fifteen pages.
    let mut typing = session(ChangeKind::Inserted);
    typing
        .edit(Operation::set_value(address(edited, 2), Value::text("ab")))
        .expect("an edit");
    let stream = typing.drain_invalidations();
    assert_eq!(stream[0].kind(), ChangeKind::Inserted);

    let (_content, mut view) = warm(FlowModel::flowing());
    assert_eq!(view.invalidate(&stream).fragments_suppressed, 16);
}

#[test]
fn a_burst_of_keystrokes_into_one_paragraph_still_dirties_one_page() {
    // Coalescing, from the viewport's side. Twenty keystrokes produce twenty invalidations, and a
    // viewport that dropped a page per invalidation would drop the same page twenty times — which
    // costs nothing in memory and would show up here as a count of twenty.
    let mut typing = session(ChangeKind::Reformatted);
    let where_it_is = address(PageIndex::new(7), 1);
    for index in 0..20 {
        typing
            .edit(Operation::set_value(
                where_it_is.clone(),
                Value::text("x".repeat(index + 1)),
            ))
            .expect("an edit");
    }
    let stream = typing.drain_invalidations();
    assert_eq!(stream.len(), 20);

    let (_content, mut view) = warm(FlowModel::flowing());
    let outcome = view.invalidate(&stream);
    assert_eq!(
        outcome.fragments_suppressed, 1,
        "twenty keystrokes into one paragraph dropped {} pages",
        outcome.fragments_suppressed,
    );
    assert_eq!(view.stats().fragments_suppressed, 1);
}

/// A compile-time reminder that the address vocabulary crossing the seam is `mjx-layout`'s and
/// carries no unit of its own: an `Emu` never appears in an invalidation.
const _: Option<Emu> = None;
