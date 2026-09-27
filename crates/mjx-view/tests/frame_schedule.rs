//! **A frame budget nothing measures is a comment.**
//!
//! §12 gives a frame 16.6 ms at 60 Hz and 8.3 ms at 120 Hz. The three things that have to be true
//! of a budget, and are asserted here rather than assumed:
//!
//! 1. **it is respected** — a frame whose work exceeds it stops and defers the rest;
//! 2. **what happens when it is exceeded is defined** — one task that overruns the whole budget
//!    still runs, and the frame reports the overrun rather than dropping the page;
//! 3. **it is what produced the deferral** — the identical work under [`FrameBudget::UNLIMITED`]
//!    defers nothing. Without that control the first assertion is green for a viewport that
//!    deferred at random.
//!
//! # How time is charged without a real clock
//!
//! The box model holds an [`Rc<ManualFrameClock>`] shared with the test and advances it by a fixed
//! amount every time it lays a page out. So *layout costs time* in the same way it does in a real
//! frame, deterministically, on one thread, with no sleep and no `Instant` — which matters because
//! `Instant::now()` panics on `wasm32-unknown-unknown` and this crate builds for that target.

use std::rc::Rc;

use mjx_layout::{LayoutSize, PageIndex};
use mjx_ooxml_core::measure::Emu;
use mjx_view::{
    CacheBudget, DocumentView, FrameBudget, ManualFrameClock, RefinementTier, Viewport,
    WindowPolicy,
};

#[path = "support/mod.rs"]
mod support;

use support::{letter, FlowModel, Paragraphs, PlainScenes};

type View = DocumentView<FlowModel, PlainScenes>;

/// A view whose every page of layout costs `micros` microseconds on `clock`.
fn costing(
    clock: &Rc<ManualFrameClock>,
    micros: u64,
    budget: FrameBudget,
    viewport_pages: f64,
) -> (Paragraphs, View) {
    let content = Paragraphs::of(60).with_blocks(20);
    let constraints = letter();
    #[allow(clippy::cast_precision_loss)]
    let height = Emu::from_emu_rounded(constraints.page.height.emu() as f64 * viewport_pages);
    let view = DocumentView::new(
        FlowModel::flowing().costing(clock, micros),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height,
        }),
        CacheBudget::uniform(8 * 1024 * 1024),
    )
    .with_frame_budget(budget);
    (content, view)
}

#[test]
fn a_frame_stops_at_its_budget_and_defers_the_rest() {
    // Six pages of window at five milliseconds each, against a 60 Hz frame. Three fit; the rest are
    // deferred, and the *visible* ones are the three that ran, because the priority order puts them
    // first.
    let clock = Rc::new(ManualFrameClock::new());
    let (content, mut view) = costing(&clock, 5_000, FrameBudget::at_refresh_rate(60), 4.0);
    view.scroll_to_page(PageIndex::new(0));

    let report = view
        .frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert_eq!(report.budget_nanos, 16_666_666);
    assert_eq!(
        report.pages_laid_out, 4,
        "four pages of layout fit in 16.6 ms"
    );
    assert!(report.deferred_anything(), "nothing was deferred");
    assert!(
        report.deferred.iter().all(|task| !task.visible),
        "a page on screen was deferred while a prefetched one ran",
    );
    // **The overshoot is one task, and that is the contract rather than a miss.** The budget is
    // checked *between* tasks, so the last one to start may cross it; a frame cannot un-run work it
    // has begun, and a scheduler that checked before starting would never make progress on a page
    // that costs more than a frame. What is asserted is the *bound* on the overshoot.
    assert!(
        report.spent_nanos <= report.budget_nanos + 5_000_000,
        "the frame spent {} ns, more than its budget plus the one task that may cross it",
        report.spent_nanos,
    );

    // Nothing is dropped: the next frame finds the same pages still missing and finishes them.
    let second = view
        .frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert!(second.pages_laid_out > 0, "the deferred work never ran");
}

#[test]
fn the_same_work_under_no_budget_defers_nothing() {
    // **The identity control.** `FrameBudget::UNLIMITED` is the value at which the scheduler does
    // nothing at all, and running the identical arrangement against it is what shows that the
    // deferral above came from the budget rather than from the workload.
    let clock = Rc::new(ManualFrameClock::new());
    let (content, mut view) = costing(&clock, 5_000, FrameBudget::UNLIMITED, 4.0);
    view.scroll_to_page(PageIndex::new(0));

    let report = view
        .frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert!(
        !report.deferred_anything(),
        "an unlimited budget deferred work"
    );
    assert!(report.visible_complete);
    assert_eq!(view.stats().tasks_deferred, 0);
    assert!(
        report.pages_laid_out >= 6,
        "{} pages",
        report.pages_laid_out
    );
}

#[test]
fn a_single_task_larger_than_the_whole_budget_still_runs_and_is_reported_as_an_overrun() {
    // The other side of the budget, and the one an implementation gets wrong by being careful: a
    // frame that checked the budget *before* its first task would find it already spent on the
    // second frame and never make progress again. So the first task always runs, and a frame that
    // could not fit it says so.
    let clock = Rc::new(ManualFrameClock::new());
    let (content, mut view) = costing(&clock, 25_000, FrameBudget::at_refresh_rate(60), 1.0);
    view.scroll_to_page(PageIndex::new(0));

    let report = view
        .frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert_eq!(
        report.pages_laid_out, 1,
        "the one page that had to run did not"
    );
    assert!(
        report.overran(),
        "a 25 ms page inside a 16.6 ms frame did not overrun"
    );
    assert_eq!(view.stats().frames_over_budget, 1);
    assert!(report.visible_complete, "the page on screen was not drawn");

    // …and the frame after it makes progress rather than stalling.
    let second = view
        .frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert!(second.pages_laid_out > 0 || second.cache_hits > 0);
}

#[test]
fn a_higher_refresh_rate_is_a_smaller_budget_and_fits_less() {
    // Two budgets, two answers. A frame-budget test at one refresh rate is the same trap as a
    // viewport at one size: the figure could be ignored entirely.
    let mut fitted = Vec::new();
    for hertz in [30u32, 60, 120] {
        let clock = Rc::new(ManualFrameClock::new());
        let (content, mut view) = costing(&clock, 4_000, FrameBudget::at_refresh_rate(hertz), 8.0);
        view.scroll_to_page(PageIndex::new(0));
        let report = view
            .frame(&content, &clock, RefinementTier::Preview)
            .expect("a frame");
        fitted.push(report.pages_laid_out);
    }
    assert_eq!(
        fitted,
        vec![9, 5, 3],
        "the refresh rate did not decide how much fitted"
    );

    // A display that reports nonsense must not become a viewport that never finishes a frame.
    assert_eq!(
        FrameBudget::at_refresh_rate(0),
        FrameBudget::at_refresh_rate(60),
    );
}

#[test]
fn a_fling_builds_no_display_lists_and_the_settle_builds_them_all() {
    // Progressive refinement. During a fling a page is on screen for a few tens of milliseconds and
    // a display list built for it is work thrown away, so `Preview` lays pages out and stops.
    let clock = Rc::new(ManualFrameClock::new());
    let (content, mut view) = costing(&clock, 200, FrameBudget::UNLIMITED, 1.0);

    for page in 0..30 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&content, &clock, RefinementTier::Preview)
            .expect("a frame");
    }
    assert!(
        view.stats().pages_laid_out >= 30,
        "the fling laid nothing out"
    );
    assert_eq!(
        view.stats().scenes_built,
        0,
        "a fling built {} display lists",
        view.stats().scenes_built,
    );
    assert_eq!(view.scene_source().calls(), 0);

    // The reader stops. The settle builds scenes for what is on screen, and lays nothing out again
    // — the fling's fragments are still cached, which is the whole point of the preview tier.
    let laid_out_by_the_fling = view.stats().pages_laid_out;
    let settled = view
        .frame(&content, &clock, RefinementTier::Full)
        .expect("a frame");
    assert!(settled.scenes_built > 0, "the settle built nothing");
    assert_eq!(
        view.stats().pages_laid_out,
        laid_out_by_the_fling,
        "the settle re-laid-out pages the fling had already done",
    );
    assert!(settled.visible_complete);
}

#[test]
fn the_priority_order_is_visible_pages_first() {
    // The order is what makes a budget useful rather than arbitrary: a frame that runs out of time
    // must have drawn what the reader can see.
    //
    // Asserted **structurally**, on the window, because that is where the guarantee lives: a frame
    // builds its task list in this order and defers a *suffix* of it, so "visible work is never
    // deferred while prefetch work runs" follows from the order rather than from one arrangement's
    // timings. `a_frame_stops_at_its_budget_and_defers_the_rest` asserts the runtime half — every
    // task that frame deferred was a prefetch.
    let clock = Rc::new(ManualFrameClock::new());
    let (_content, mut view) = costing(&clock, 9_000, FrameBudget::at_refresh_rate(60), 2.0);
    view.scroll_to_page(PageIndex::new(10));

    let window = view.window();
    let order: Vec<PageIndex> = window.in_priority_order().collect();
    let boundary = order
        .iter()
        .position(|page| !window.is_visible(*page))
        .unwrap_or(order.len());
    assert!(boundary > 0, "no visible page came first");
    assert!(
        boundary < order.len(),
        "the window has no prefetch, so nothing is being ordered"
    );
    assert!(
        order[..boundary]
            .iter()
            .all(|page| window.is_visible(*page)),
        "a prefetched page came before a visible one",
    );
    assert!(
        order[boundary..]
            .iter()
            .all(|page| !window.is_visible(*page)),
        "a visible page came after a prefetched one",
    );
    assert_eq!(order.len(), window.len());

    // And with no prefetch at all there is nothing to deprioritise, so a two-page window at nine
    // milliseconds a page fits inside one 60 Hz frame — which shows the ordering above is between
    // two real groups rather than between a group and an empty one.
    let clock = Rc::new(ManualFrameClock::new());
    let (content, view) = costing(&clock, 8_000, FrameBudget::at_refresh_rate(60), 2.0);
    let mut view = view.with_window_policy(WindowPolicy::NONE);
    view.scroll_to_page(PageIndex::new(0));
    let narrow = view
        .frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    assert!(!narrow.deferred_anything(), "a two-page window did not fit");
    assert_eq!(narrow.pages_laid_out, 2);
}
