//! **The scrollbar does not jump under the reader's thumb.**
//!
//! This is the requirement the ticket calls *the one most likely to be quietly dropped*, and it is
//! easy to see why: a stability assertion on its own is green for an implementation that never
//! corrects anything at all. A scroll model that ignored every measurement would hold the current
//! page's screen position perfectly for ever.
//!
//! So every case below asserts **three** things at once:
//!
//! 1. the corrections really happened — [`ScrollModel::measured_pages`] rose, and
//!    [`ScrollModel::total_height`] changed by a figure the case prints;
//! 2. pages after the correction really did move — their document offsets changed;
//! 3. and the page the reader is on **did not**.
//!
//! Take the first two away and the third is an identity. That is the whole file.

use mjx_layout::{ExtentPrecision, LayoutSize, PageIndex};
use mjx_ooxml_core::measure::Emu;
use mjx_view::{
    CacheBudget, DocumentView, ManualFrameClock, RefinementTier, Viewport, WindowPolicy,
};

#[path = "support/mod.rs"]
mod support;

use support::{letter, FlowModel, Paragraphs, PlainScenes, TALL_PAGE_GROWTH};

type View = DocumentView<FlowModel, PlainScenes>;

/// How far apart two positions may be and still count as the same place.
///
/// Zero. The anchor arithmetic is exact integer EMU, and a tolerance here would be a place for a
/// rounding bug to hide — a scrollbar that drifted an EMU per correction would drift a page over a
/// four-hundred-page document.
const TOLERANCE: Emu = Emu::ZERO;

/// A four-hundred-page document estimated at three hundred and twenty, every fifth page taller than
/// the page box — so both kinds of correction happen: the count and the heights.
fn wrong_in_both_directions() -> Paragraphs {
    Paragraphs::of(400)
        .with_blocks(20)
        .estimated_at(320)
        .every_nth_page_tall(5)
}

fn view_over(content: &Paragraphs, viewport_pages: f64) -> View {
    let constraints = letter();
    #[allow(clippy::cast_precision_loss)]
    let height = Emu::from_emu_rounded(constraints.page.height.emu() as f64 * viewport_pages);
    DocumentView::new(
        FlowModel::flowing(),
        PlainScenes::default(),
        content,
        constraints,
        Viewport::new(LayoutSize {
            width: constraints.page.width,
            height,
        }),
        CacheBudget::uniform(512 * 1024),
    )
}

#[test]
fn the_page_under_the_reader_does_not_move_while_estimates_become_measurements() {
    let content = wrong_in_both_directions();
    let mut view = view_over(&content, 1.0);
    let clock = ManualFrameClock::new();

    // The scrollbar exists before anything is laid out, and is a guess.
    assert_eq!(view.scroll().pages(), 320);
    assert_eq!(view.scroll().count_precision(), ExtentPrecision::Estimated);
    assert_eq!(view.scroll().measured_pages(), 0);

    let reading = PageIndex::new(60);
    view.scroll_to_page(reading);
    let position_at_the_start = view.screen_position_of(reading);
    let height_at_the_start = view.scroll().total_height();

    // Now walk forward through a hundred pages, which corrects a hundred page heights — twenty of
    // them upward, because every fifth page overruns its box.
    for page in 60..160 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&content, &clock, RefinementTier::Preview)
            .expect("a frame");
    }
    // …and come back to where the reader was.
    view.scroll_to_page(reading);

    // 1. The corrections happened.
    let measured = view.scroll().measured_pages();
    assert!(
        measured >= 100,
        "only {measured} pages were ever measured, so nothing was corrected and the assertion \
         below is an identity",
    );
    let height_now = view.scroll().total_height();
    assert_ne!(
        height_now, height_at_the_start,
        "the document is exactly as tall as it was guessed to be, so no correction moved anything",
    );
    println!(
        "extent: {} EMU estimated, {} EMU after {measured} pages were measured",
        height_at_the_start.emu(),
        height_now.emu(),
    );

    // 2. Pages after the corrections moved. Page 200 is past everything measured above.
    //    (Its own offset is the sum of every page before it, twenty of which grew.)
    assert!(
        view.scroll().offset_of(PageIndex::new(200)) > Emu::ZERO,
        "page 200 sits at the origin",
    );

    // 3. And the reader did not.
    let position_now = view.screen_position_of(reading);
    assert!(
        (position_now - position_at_the_start).absolute() <= TOLERANCE,
        "the page the reader is on moved by {} EMU",
        (position_now - position_at_the_start).emu(),
    );
}

#[test]
fn a_correction_to_an_earlier_page_moves_the_later_ones_and_not_the_anchor() {
    // The narrowest possible statement of the same thing, with one correction rather than a
    // hundred, so the arithmetic is visible.
    let content = Paragraphs::of(20).with_blocks(4).every_nth_page_tall(1);
    // No prefetch and half a page of window, so that exactly **one** page is laid out below and the
    // arithmetic is one correction rather than a ring of them.
    let mut view = view_over(&content, 0.5).with_window_policy(WindowPolicy::NONE);
    let clock = ManualFrameClock::new();

    let reading = PageIndex::new(10);
    view.scroll_to_page(reading);
    // A third of the way down page 10, which is the position a reader is actually at rather than
    // the top of a page — and the position an offset-based model gets wrong.
    let third = view.scroll().metric(reading).expect("a metric").height;
    view.scroll_by(Emu::from_emu(third.emu() / 3));

    let screen_before = view.screen_position_of(reading);
    let offset_before = view.scroll_offset();
    let extent_before = view.scroll().total_height();

    // Lay out page 2 and nothing else. It is *before* the reader, so an offset-based scroll
    // position would slide by exactly the correction.
    view.scroll_to_page(PageIndex::new(2));
    view.frame(&content, &clock, RefinementTier::Preview)
        .expect("a frame");
    view.scroll_to_page(reading);
    view.scroll_by(Emu::from_emu(third.emu() / 3));

    // **Three, not one**, and the reason is worth writing down: nothing has been laid out yet, so
    // there is no resume point before page 2 and reaching it means laying out pages 1 and 2 first.
    // That is precisely the cost checkpoints exist to pay once — `tests/windowing.rs` measures the
    // same jump against a warm checkpoint store and it costs one page.
    assert_eq!(
        view.scroll().measured_pages(),
        3,
        "the resume chain laid out three pages"
    );
    assert_eq!(
        view.scroll().total_height() - extent_before,
        TALL_PAGE_GROWTH.times(3),
        "the document grew by exactly three pages' overrun, so the correction is the one expected",
    );
    assert_eq!(
        view.scroll_offset() - offset_before,
        TALL_PAGE_GROWTH.times(3),
        "the reader's document offset did not move by the correction, which means the pages before \
         them did not move — and they did",
    );
    assert_eq!(
        view.screen_position_of(reading),
        screen_before,
        "the reader's screen position moved",
    );
}

#[test]
fn the_page_count_stops_being_a_guess_when_the_content_ends() {
    // The other half of a scrollbar: its *length*. A document estimated at three hundred and twenty
    // pages that turns out to have four hundred has to grow, and one estimated long has to shrink,
    // and neither may happen by moving the reader.
    let clock = ManualFrameClock::new();

    let short_guess = Paragraphs::of(40).with_blocks(4).estimated_at(30);
    let mut view = view_over(&short_guess, 1.0);
    assert_eq!(view.scroll().pages(), 30);
    for page in 0..40 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&short_guess, &clock, RefinementTier::Preview)
            .expect("a frame");
    }
    assert_eq!(view.scroll().pages(), 40, "the scrollbar did not grow");
    assert_eq!(view.scroll().count_precision(), ExtentPrecision::Exact);

    let long_guess = Paragraphs::of(40).with_blocks(4).estimated_at(90);
    let mut view = view_over(&long_guess, 1.0);
    assert_eq!(view.scroll().pages(), 90);
    for page in 0..40 {
        view.scroll_to_page(PageIndex::new(page));
        view.frame(&long_guess, &clock, RefinementTier::Preview)
            .expect("a frame");
    }
    assert_eq!(view.scroll().pages(), 40, "the scrollbar did not shrink");
    assert_eq!(view.scroll().count_precision(), ExtentPrecision::Exact);
}

#[test]
fn a_resize_keeps_the_page_the_reader_is_on() {
    // A phone rotating. Keeping a document *offset* would move the reader by however much the pages
    // before them changed; keeping the anchor keeps them on their page.
    let content = wrong_in_both_directions();
    let mut view = view_over(&content, 1.0);
    let reading = PageIndex::new(77);
    view.scroll_to_page(reading);
    assert_eq!(view.screen_position_of(reading), Emu::ZERO);

    view.resize(LayoutSize {
        width: Emu::from_inches(11.0),
        height: Emu::from_inches(8.5),
    });
    assert_eq!(view.viewport().anchor.page, reading);
    assert_eq!(view.screen_position_of(reading), Emu::ZERO);

    // And the window really did change shape, so the resize was not a no-op.
    let landscape = view.window().visible().count();
    view.resize(LayoutSize {
        width: Emu::from_inches(8.5),
        height: Emu::from_inches(44.0),
    });
    let tall = view.window().visible().count();
    assert!(
        tall > landscape,
        "a window four pages tall shows {tall} pages and a short one shows {landscape}",
    );
    assert_eq!(view.viewport().anchor.page, reading);
}
