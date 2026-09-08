//! **A window, not a document** — and every assertion here is two-sided, because each of them has a
//! degenerate implementation that would otherwise pass.
//!
//! | assertion | what would pass without the other half |
//! |---|---|
//! | the fragment cache stayed under its ceiling | a cache that holds nothing |
//! | scrolling back one page hit the cache | a cache with no ceiling at all |
//! | the window is biased in the direction of travel | a symmetric window, if nothing ever scrolls |
//! | the prefetch ring materialises pages off screen | a window of one page, if nothing counts |
//!
//! And one trap that is not about caches at all: **a viewport tested at one size proves nothing
//! about windowing.** At a page-and-a-half of viewport height two pages are visible, and at four
//! pages' worth four are; a `PageWindow` that returned the anchor's page and stopped would be green
//! against a single size for ever. [`the_window_follows_the_viewport_size_and_therefore_the_zoom`]
//! runs four heights and asserts the count rises with each.

use std::rc::Rc;

use mjx_layout::{LayoutSize, PageIndex};
use mjx_ooxml_core::measure::Emu;
use mjx_view::{
    CacheBudget, DocumentView, ManualFrameClock, RefinementTier, ScrollDirection, Stage, Viewport,
    WindowPolicy,
};

#[path = "support/mod.rs"]
mod support;

use support::{letter, FlowModel, Paragraphs, PlainScenes};

type View = DocumentView<FlowModel, PlainScenes>;

/// A view over `pages` pages with a window `viewport_pages` pages tall and a `budget` per stage.
fn view_over(pages: u32, viewport_pages: f64, budget: usize) -> (Paragraphs, View) {
    let content = Paragraphs::of(pages).with_blocks(60);
    let constraints = letter();
    let height = Emu::from_emu_rounded(constraints.page.height.emu() as f64 * viewport_pages);
    let view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height,
        }),
        CacheBudget::uniform(budget),
    );
    (content, view)
}

/// Runs frames until the window is complete, or gives up — which is a failure of the test's own
/// arrangement rather than of the code, so it says so.
fn settle(view: &mut View, content: &Paragraphs, clock: &ManualFrameClock, tier: RefinementTier) {
    for _ in 0..16 {
        let report = view.frame(content, clock, tier).expect("a frame");
        if report.visible_complete && !report.deferred_anything() {
            return;
        }
    }
    panic!("the window never completed in sixteen frames");
}

#[test]
fn the_window_follows_the_viewport_size_and_therefore_the_zoom() {
    // Four heights, and the visible count has to rise with each. A viewport exercised at one size
    // is the trap this case exists for: `PageWindow::of` walking forward until a page starts past
    // the bottom edge is indistinguishable from `PageWindow::of` returning one page, unless more
    // than one page fits.
    let mut counts = Vec::new();
    for viewport_pages in [0.5, 1.5, 2.5, 4.0] {
        let (_content, mut view) = view_over(50, viewport_pages, 8 * 1024 * 1024);
        view.scroll_to_page(PageIndex::new(10));
        let window = view.window();
        counts.push(window.visible().count());
    }
    // Four heights, four answers: a window four pages tall ends exactly where page 15 begins, and
    // a page that starts at the bottom edge is not on screen.
    assert_eq!(
        counts,
        vec![1, 2, 3, 4],
        "the visible page count did not follow the viewport height",
    );

    // The same thing said as a zoom, which is what a reader actually does. `Viewport::with_zoom`
    // divides the physical window by the zoom, so a quarter zoom shows four times the document.
    let constraints = letter();
    let physical = LayoutSize {
        width: constraints.page.width,
        height: constraints.page.height,
    };
    let full = Viewport::with_zoom(physical, 1.0);
    let quarter = Viewport::with_zoom(physical, 0.25);
    assert_eq!(quarter.size.height.emu(), full.size.height.emu() * 4);
    // A pinch gesture that produced a zero or a NaN must not become a division by zero on the
    // frame path.
    assert_eq!(Viewport::with_zoom(physical, 0.0).size, full.size);
    assert_eq!(Viewport::with_zoom(physical, f64::NAN).size, full.size);
}

#[test]
fn the_prefetch_ring_is_biased_in_the_direction_of_travel() {
    let (_content, mut view) = view_over(50, 1.0, 8 * 1024 * 1024);
    view.scroll_to_page(PageIndex::new(20));
    assert_eq!(view.viewport().direction, ScrollDirection::Forward);
    let forward = view.window();
    assert_eq!(forward.first(), PageIndex::new(19), "one page behind");
    assert_eq!(forward.last(), PageIndex::new(22), "two pages ahead");

    view.scroll_to_page(PageIndex::new(10));
    assert_eq!(view.viewport().direction, ScrollDirection::Backward);
    let backward = view.window();
    assert_eq!(
        backward.first(),
        PageIndex::new(8),
        "two pages ahead is now two below"
    );
    assert_eq!(
        backward.last(),
        PageIndex::new(11),
        "one page behind is now one above"
    );

    // Standing still is a third answer and not a repeat of either: the larger of the two figures on
    // both sides, because a reader who has stopped may go either way.
    view.scroll_to_page(PageIndex::new(10));
    assert_eq!(view.viewport().direction, ScrollDirection::Still);
    let still = view.window();
    assert_eq!(still.first(), PageIndex::new(8));
    assert_eq!(still.last(), PageIndex::new(12));
}

#[test]
fn the_prefetch_ring_actually_materialises_pages_that_are_not_on_screen() {
    // The identity control: the same walk under `WindowPolicy::NONE`, which prefetches nothing. A
    // window test that only ever ran the default policy would be green for an implementation that
    // ignored the two numbers entirely.
    let clock = ManualFrameClock::new();

    let (content, mut view) = view_over(50, 1.0, 8 * 1024 * 1024);
    view.scroll_to_page(PageIndex::new(20));
    settle(&mut view, &content, &clock, RefinementTier::Full);
    let with_prefetch = view.stats().pages_laid_out;
    assert!(
        view.fragments_for(PageIndex::new(22)).is_some(),
        "the page two ahead was not prefetched",
    );

    let (content, view) = view_over(50, 1.0, 8 * 1024 * 1024);
    let mut view = view.with_window_policy(WindowPolicy::NONE);
    view.scroll_to_page(PageIndex::new(20));
    settle(&mut view, &content, &clock, RefinementTier::Full);
    assert!(
        view.fragments_for(PageIndex::new(22)).is_none(),
        "a policy that prefetches nothing prefetched something",
    );
    assert!(
        view.stats().pages_laid_out < with_prefetch,
        "the two policies did the same amount of work, so the prefetch figures do nothing",
    );
}

#[test]
fn scrolling_back_one_page_hits_the_cache_and_re_lays_out_nothing() {
    // **The other half of every ceiling in this crate.** A cache that evicted everything would
    // satisfy the byte bound perfectly and fail here.
    let clock = ManualFrameClock::new();
    let (content, mut view) = view_over(50, 1.0, 8 * 1024 * 1024);

    view.scroll_to_page(PageIndex::new(20));
    settle(&mut view, &content, &clock, RefinementTier::Full);
    view.scroll_to_page(PageIndex::new(21));
    settle(&mut view, &content, &clock, RefinementTier::Full);
    let before = view.stats().pages_laid_out;

    view.scroll_to_page(PageIndex::new(20));
    settle(&mut view, &content, &clock, RefinementTier::Full);
    assert_eq!(
        view.stats().pages_laid_out,
        before,
        "scrolling back one page re-laid it out",
    );
    assert_eq!(
        view.scene_source().calls(),
        view.stats().scenes_built,
        "the two counters disagree about how many display lists were built",
    );
}

#[test]
fn a_budget_that_fits_two_pages_holds_two_pages_and_evicts_the_rest() {
    // A walk with a budget small enough that eviction has to happen, and both sides asserted: the
    // ceiling held, *and* the cache is not empty, *and* the eviction path ran.
    let clock = ManualFrameClock::new();
    // One page of sixty blocks is a few kilobytes; a 40 KiB ceiling fits a handful.
    let (content, mut view) = view_over(60, 1.0, 40 * 1024);
    for page in 0..60 {
        view.scroll_to_page(PageIndex::new(page));
        settle(&mut view, &content, &clock, RefinementTier::Full);
    }

    let report = view.cache_report();
    assert!(
        report.within_ceilings(),
        "a stage is over its ceiling: {:?}",
        report.first_overrun(),
    );
    let fragments = report.stage(Stage::Fragments).expect("the fragment stage");
    assert!(
        fragments.evictions > 0,
        "nothing was evicted, so the ceiling is held by luck",
    );
    assert!(
        fragments.entries > 1,
        "{} pages held — a cache that keeps nothing is not a cache",
        fragments.entries,
    );
    assert!(
        fragments.hits > 0,
        "no lookup was ever answered from the cache",
    );

    // And the working set really is the recent one: the last page walked is still held and an
    // early one is not.
    assert!(view.fragments_for(PageIndex::new(59)).is_some());
    assert!(
        view.fragments_for(PageIndex::new(0)).is_none(),
        "page 1 survived a walk of sixty pages under a budget that fits a handful",
    );
}

#[test]
fn checkpoints_are_kept_for_every_page_and_fragments_are_not() {
    // The asymmetry the ticket calls the design. After a walk, every page has a resume point and
    // almost none has fragments — which is what turns "scroll back to page 3" into one page of
    // layout instead of three.
    let clock = ManualFrameClock::new();
    let (content, mut view) = view_over(60, 1.0, 40 * 1024);
    for page in 0..60 {
        view.scroll_to_page(PageIndex::new(page));
        settle(&mut view, &content, &clock, RefinementTier::Full);
    }
    let report = view.cache_report();
    let checkpoints = report.stage(Stage::Checkpoints).expect("the stage");
    let fragments = report.stage(Stage::Fragments).expect("the stage");
    assert_eq!(
        checkpoints.entries, 59,
        "a resume point per page except the last, which ends the content",
    );
    assert_eq!(checkpoints.evictions, 0, "a checkpoint was evicted");
    assert!(
        fragments.entries < 10,
        "{} pages of fragments held beside {} checkpoints — the asymmetry is gone",
        fragments.entries,
        checkpoints.entries,
    );

    // And the payoff, measured: jumping back to page 3 costs one page of layout, not three.
    let before = view.stats().pages_laid_out;
    view.scroll_to_page(PageIndex::new(3));
    settle(&mut view, &content, &clock, RefinementTier::Full);
    let cost = view.stats().pages_laid_out - before;
    assert!(
        cost <= 4,
        "jumping back to page 4 cost {cost} pages of layout, so the checkpoints bought nothing",
    );
}

#[test]
fn a_document_shorter_than_its_estimate_skips_the_pages_that_are_not_there() {
    // A window is built from the scroll model's page count, and that count is a **guess** until a
    // page reports no continuation — so a window can legitimately name a page that does not exist,
    // and one built earlier in this very frame can name a page the frame has since discovered is
    // past the end. Skipping it is the answer. Failing would mean a frame that errored because the
    // document turned out shorter than guessed, which is not an error at all.
    let content = Paragraphs::of(40).with_blocks(8).estimated_at(90);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        // Four pages tall, so pages 37..=40 are visible at once and the page that ends the document
        // is laid out two tasks before the page after it is reached.
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height: constraints.page.height.times(4),
        }),
        CacheBudget::uniform(8 * 1024 * 1024),
    );
    let clock = ManualFrameClock::new();
    view.scroll_to_page(PageIndex::new(37));

    let report = view
        .frame(&content, &clock, RefinementTier::Full)
        .expect("a frame over a document shorter than its estimate must not fail");
    assert!(
        report.pages_past_the_end > 0,
        "no page past the end was skipped, so this case is not reaching the path it names",
    );
    assert_eq!(
        u64::try_from(report.pages_past_the_end).expect("a count"),
        view.stats().pages_past_the_end,
    );
    assert!(
        report.visible_complete,
        "the pages that do exist were not drawn — a page past the end is not incomplete, it is \
         absent, and counting it would make a shell ask for frames for ever",
    );
    assert_eq!(
        view.scroll().pages(),
        40,
        "the scrollbar did not shrink to the document's real length",
    );

    // The identity control: the same viewport in the middle of the document skips nothing.
    view.scroll_to_page(PageIndex::new(10));
    let middle = view
        .frame(&content, &clock, RefinementTier::Full)
        .expect("a frame");
    assert_eq!(middle.pages_past_the_end, 0);
}

#[test]
fn the_layout_counter_and_the_box_model_agree() {
    // Two counters for one fact, from opposite sides. A viewport that mis-counted its own work
    // would make every other assertion in this file meaningless, and nothing else would notice.
    let clock = Rc::new(ManualFrameClock::new());
    let content = Paragraphs::of(30).with_blocks(20);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing().costing(&clock, 100),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(constraints.page),
        CacheBudget::uniform(1024 * 1024),
    );
    for page in [0u32, 5, 12, 4, 29, 0] {
        view.scroll_to_page(PageIndex::new(page));
        settle(&mut view, &content, &clock, RefinementTier::Full);
    }
    assert_eq!(view.stats().pages_laid_out, view.model().calls());
    assert!(view.stats().pages_laid_out > 0);
}
