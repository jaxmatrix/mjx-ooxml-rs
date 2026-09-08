//! Every public item in this crate is reached from a test, and this is the file that makes that
//! true.
//!
//! # Why
//!
//! A public surface nothing calls is a surface nobody has checked — it compiles, its documentation
//! reads well, and the first caller finds out what it actually does. This programme's audit found a
//! 591-line HTTP router no test reached, and `mjx-scene` already keeps a file of this shape
//! (`tests/public_surface_is_reachable.rs`) for the same reason.
//!
//! Two of the items below are the ones this file was written for, and neither would have been
//! reached by the behavioural suites:
//!
//! * [`DocumentView::diff_against`], which is how MJXOFF-161's frame-to-frame display-list diff is
//!   consumed. The ticket says *"consume, do not re-create"* about it, and a method nothing calls
//!   consumes nothing.
//! * [`ScrollModel::metric`] and [`PageMetric`]'s two fields, which are how a shell draws a page
//!   ruler and how a caller tells a measurement from a guess.
//!
//! This is a *reachability* gate, not a behavioural one — the assertions here are deliberately
//! shallow, and the real ones live in the four suites beside it.

use std::rc::Rc;

use mjx_layout::{ExtentPrecision, LayoutSize, PageIndex};
use mjx_ooxml_core::cache::{Admission, ByteBudgetCache, Rejection, ENTRY_OVERHEAD_BYTES};
use mjx_ooxml_core::measure::Emu;
use mjx_view::{
    CacheBudget, DocumentView, FrameBudget, FrameClock, ManualFrameClock, RefinementTier,
    ScrollAnchor, ScrollDirection, Stage, TaskKind, Viewport, WindowPolicy,
};

#[path = "support/mod.rs"]
mod support;

use support::{letter, FlowModel, Paragraphs, PlainScenes};

#[test]
fn the_frame_diff_is_reachable_and_says_what_changed() {
    // MJXOFF-161's `diff_frames`, consumed rather than re-created. A repaint uploads the records
    // that moved instead of the page, and this is the one call that makes that possible.
    // Every other page is laid out taller than the page box, so page 0 and page 1 are genuinely
    // different geometry. Without that the two display lists would be byte-identical — the synthetic
    // model puts the same boxes at the same coordinates on every page — and the assertion below
    // would be measuring the corpus rather than the diff.
    let content = Paragraphs::of(6).with_blocks(6).every_nth_page_tall(2);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(constraints.page),
        CacheBudget::uniform(2 * 1024 * 1024),
    );
    let clock = ManualFrameClock::new();
    view.frame(&content, &clock, RefinementTier::Full)
        .expect("a frame");

    let page = PageIndex::FIRST;
    let first = view
        .display_list_for(page)
        .expect("the page has a display list")
        .clone();

    // The same page against itself: nothing moved.
    let unchanged = view
        .diff_against(page, &first)
        .expect("the page is still held");
    assert!(
        !unchanged.header_changed() && unchanged.changes().is_empty(),
        "a display list differs from itself",
    );

    // A different page's list against it: something did.
    let other = view
        .display_list_for(PageIndex::new(1))
        .expect("the prefetched page has one");
    let changed = view.diff_against(page, other);
    assert!(
        changed.is_some_and(|diff| !diff.changes().is_empty()),
        "two different pages produced no difference at all",
    );

    // And a page that is not held answers `None` rather than pretending.
    assert!(view.diff_against(PageIndex::new(5), &first).is_none());
}

#[test]
fn the_scroll_models_accessors_are_reachable() {
    let content = Paragraphs::of(8).with_blocks(4).every_nth_page_tall(2);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(constraints.page),
        CacheBudget::uniform(1024 * 1024),
    );
    let clock = ManualFrameClock::new();

    let guessed = view.scroll().metric(PageIndex::FIRST).expect("a metric");
    assert_eq!(guessed.precision, ExtentPrecision::Estimated);
    assert!(guessed.height > Emu::ZERO);
    assert!(view.scroll().metric(PageIndex::new(99)).is_none());

    view.frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    let measured = view.scroll().metric(PageIndex::FIRST).expect("a metric");
    assert_eq!(measured.precision, ExtentPrecision::Exact);

    // The anchor's two constructors, and the round trip between an anchor and an offset.
    assert_eq!(ScrollAnchor::top_of(PageIndex::FIRST), ScrollAnchor::ORIGIN);
    let anchor = ScrollAnchor::top_of(PageIndex::new(3));
    let offset = view.scroll().offset_of_anchor(anchor);
    assert_eq!(view.scroll().anchor_at(offset), anchor);
    assert_eq!(
        view.scroll().anchor_at(Emu::from_emu(-1)),
        ScrollAnchor::ORIGIN
    );

    // `scroll_by` and `scroll_offset`, which `scroll_to_page` does not reach.
    let before = view.scroll_offset();
    view.scroll_by(Emu::from_inches(3.0));
    assert!(view.scroll_offset() > before);
    assert_eq!(view.viewport().direction, ScrollDirection::Forward);
    view.scroll_by(Emu::ZERO);
    assert_eq!(view.viewport().direction, ScrollDirection::Still);
}

#[test]
fn the_window_and_the_budget_accessors_are_reachable() {
    let content = Paragraphs::of(20).with_blocks(4);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(constraints.page),
        CacheBudget::mobile(),
    )
    .with_window_policy(WindowPolicy {
        ahead: 3,
        behind: 2,
    })
    .with_frame_budget(FrameBudget::from_micros(8_000));

    view.scroll_to_page(PageIndex::new(10));
    let window = view.window();
    assert!(window.contains(window.first()) && window.contains(window.last()));
    assert!(!window.contains(PageIndex::new(19)));
    assert!(!window.is_empty());
    assert_eq!(window.len(), 6);
    assert!(window.is_visible(window.first_visible()));

    assert_eq!(view.budget().checkpoints, CacheBudget::mobile().checkpoints);
    assert!(CacheBudget::desktop().total() > CacheBudget::mobile().total());
    assert!(CacheBudget::desktop().viewport_total() < CacheBudget::desktop().total());
    assert_eq!(Stage::Checkpoints.label(), "checkpoints");
    assert!(!Stage::GlyphAtlas.is_held_by_the_viewport());

    // The report's own accessors.
    let report = view.cache_report();
    assert!(report.first_overrun().is_none());
    assert_eq!(report.bytes(), 0);
    assert!(report
        .stage(Stage::Fragments)
        .is_some_and(mjx_view::StageReport::within_ceiling));
    assert!(report.stage(Stage::Tessellations).is_none());
}

#[test]
fn the_clock_and_the_task_vocabulary_are_reachable() {
    let clock = Rc::new(ManualFrameClock::new());
    clock.advance_micros(5);
    assert_eq!(clock.now_nanos(), 5_000);
    clock.advance_nanos(7);
    assert_eq!(clock.now_nanos(), 5_007);
    // The blanket implementations, both of which the view relies on: a shared clock and a borrowed
    // one read the same time.
    let borrowed: &ManualFrameClock = &clock;
    assert_eq!(FrameClock::now_nanos(&borrowed), clock.now_nanos());

    assert!(RefinementTier::Full.builds_scenes());
    assert!(!RefinementTier::Preview.builds_scenes());
    assert!(RefinementTier::Preview < RefinementTier::Full);
    assert!(TaskKind::Layout < TaskKind::Scene);
    assert_eq!(FrameBudget::default(), FrameBudget::at_refresh_rate(60));
    assert_eq!(FrameBudget::from_micros(1_000).nanos, 1_000_000);
    assert!(FrameBudget::UNLIMITED.nanos > FrameBudget::at_refresh_rate(120).nanos);

    // A frame report's own predicates, on a real report.
    let content = Paragraphs::of(3).with_blocks(2);
    let constraints = letter();
    let mut view = DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        &content,
        constraints,
        Viewport::new(constraints.page),
        CacheBudget::uniform(512 * 1024),
    );
    let report = view
        .frame(&content, &clock, RefinementTier::Full)
        .expect("a frame");
    assert_eq!(report.tier, RefinementTier::Full);
    assert!(!report.overran());
    assert!(!report.deferred_anything());
    assert_eq!(report.pages_past_the_end, 0);
    assert!(view.fragments_for(PageIndex::FIRST).is_some());
    assert!(view.model().calls() > 0);
    assert_eq!(view.scene_source().calls(), view.stats().scenes_built);

    // `resize` keeps the anchor — the behavioural half is in `scroll_stability.rs`; this is the
    // call itself.
    view.resize(LayoutSize {
        width: Emu::from_inches(4.0),
        height: Emu::from_inches(4.0),
    });
    assert_eq!(view.viewport().size.width, Emu::from_inches(4.0));
    assert!(format!("{view:?}").contains("DocumentView"));
}

#[test]
fn the_shared_caches_own_surface_is_reachable_from_here_too() {
    // `ByteBudgetCache` is `mjx-ooxml-core`'s and has its own unit tests; what this reaches is the
    // half a *consumer* uses, so that the re-exports at this crate's rank are known to work.
    let mut cache: ByteBudgetCache<u32, String> = ByteBudgetCache::new(4 * ENTRY_OVERHEAD_BYTES);
    assert!(cache.insert(1, "one".to_owned(), 0).is_admitted());
    cache.insert_retained(2, "two".to_owned(), 0);
    assert_eq!(cache.get(&1).map(String::as_str), Some("one"));
    if let Some(value) = cache.get_mut(&2) {
        value.push('!');
    }
    assert_eq!(cache.peek(&2).map(String::as_str), Some("two!"));
    assert_eq!(cache.len(), 2);
    assert!(cache.bytes() > 0 && cache.bytes() <= cache.budget());
    assert_eq!(cache.pinned_bytes(), 0);
    let mut keys: Vec<u32> = cache.keys().copied().collect();
    keys.sort_unstable();
    assert_eq!(keys, vec![1, 2]);
    assert!(cache.pin(&1));
    assert!(cache.unpin(&1));
    cache.unpin_all();
    cache.trim();
    assert_eq!(cache.suppress(&1).as_deref(), Some("one"));
    assert_eq!(cache.suppress_matching(|key| *key == 2), 1);
    assert!(cache.is_empty());
    assert_eq!(cache.least_recently_used(), None);
    assert!(!cache.contains_key(&1));
    let refused = cache.insert(3, "three".to_owned(), usize::MAX / 2);
    assert_eq!(refused.rejection(), Some(Rejection::LargerThanBudget));
    assert!(matches!(refused, Admission::Rejected { value, .. } if value == "three"));
    cache.clear();
    assert_eq!(cache.stats().rejections, 1);
    assert!(format!("{cache:?}").contains("ByteBudgetCache"));
}
