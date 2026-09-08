//! `mjx-view` — viewport windowing, byte-budgeted caches and frame scheduling (MJXOFF-168).
//!
//! # What this crate is for
//!
//! **This — not hosting — is what "high-throughput page view memory management" actually means, and
//! it is a windowing problem.** A four-hundred-page document at 150 dpi is roughly four gigabytes
//! of pixels; nobody holds that anywhere, client or server, and the discipline that makes it
//! affordable is identical in every topology. It is three rules:
//!
//! 1. **Materialise a window, not a document.** The pages on screen, plus a small prefetch ring
//!    biased in the direction of travel. [`PageWindow`].
//! 2. **Put a byte ceiling on every stage that holds something expensive, and evict least recently
//!    used.** [`CacheBudget`], and [`mjx_ooxml_core::ByteBudgetCache`] underneath it.
//! 3. **Give a frame a time budget and defer what does not fit**, rather than dropping the frame.
//!    [`FrameBudget`], [`FrameReport`].
//!
//! Around those sits the thing a reader actually notices: [`ScrollModel`], which is why the
//! scrollbar does not jump under their thumb as estimates are replaced by measurements.
//!
//! # Where it sits, and what the rank buys
//!
//! Rank **3.8** — above `mjx-session` (3.5), below the facade. That is what lets it name an
//! [`Invalidation`](mjx_session::Invalidation) at all, and what stops anything at or below the
//! format tier from reaching a viewport: a `.pptx` reader with a scroll position in it would be a
//! batch library with a window manager inside it.
//!
//! **What the rank does not buy is the other direction, and this is worth reading before relying on
//! it.** `xtask/tests/layering.rs` refuses only an edge that points up or sideways, so at 3.8
//! `mjx-view → mjx-pptx`, `mjx-view → mjx-dml` and `mjx-view → mjx-geometry` are all legal
//! *downward* edges and always will be. The property this crate actually has to hold — **a viewport
//! has never heard of OOXML** — is therefore held by construction and by an explicit gate, exactly
//! as `mjx-session`'s is and `mjx-paint`'s is:
//!
//! * it is generic over [`BoxModel`](mjx_layout::BoxModel) and over [`SceneSource`], and names no
//!   implementation of either;
//! * its one dependency above the shared-markup tier is `mjx-session` **with default features
//!   off**, so `cargo test -p mjx-view` is a build in which `mjx-pptx`, `mjx-docx` and `mjx-xlsx`
//!   are not present at all — a line that reached one would not compile;
//! * `crates/mjx-view/tests/the_seam_holds.rs` asserts the manifest and scans the source.
//!
//! A reader who assumes 3.8 is protecting this crate's own edges has it backwards.
//!
//! # The four traps this crate's gates are written against
//!
//! Every one of them produces a green suite over an implementation that does nothing, so each is
//! named here and closed by a test rather than by care.
//!
//! 1. **A memory ceiling is trivially satisfied by a cache that holds nothing.** So every budget
//!    assertion is two-sided: the ceiling held *and* the working set was retained — scrolling back
//!    one page must not re-lay it out, asserted on [`ViewStats::pages_laid_out`].
//! 2. **A budget so large it never evicts is indistinguishable from no budget.**
//!    [`CacheBudget::unbounded`] exists so `tests/resident_memory.rs` can run the identical walk
//!    twice and show that the bounded figure came from the bound.
//! 3. **A windowing test on a small fixture is green for an implementation that materialises
//!    everything.** The corpus is four hundred pages and full materialisation breaches the budget by
//!    a measured multiple.
//! 4. **A viewport exercised at one size, one scroll position and one zoom proves nothing.** The
//!    windowing suite runs three viewport heights, both scroll directions and a standing still, and
//!    asserts the window differs.
//!
//! # Single-threaded, and no global state
//!
//! Nothing here spawns a thread and nothing here is global. `wasm32` without cross-origin isolation
//! has no threads, so correctness may not depend on having them; and several documents must be able
//! to be open at once, which is also what makes the tests deterministic. The frame clock is
//! injected for the same reason `mjx-session`'s is — `std::time::Instant::now()` panics on
//! `wasm32-unknown-unknown` — see [`schedule`].
//!
//! # Not in scope
//!
//! Hit-testing wired to selection, and interaction of any kind. This crate schedules and caches; it
//! does not respond to input. The spatial index a hit test would query is built during layout and is
//! held here ([`DocumentView::fragments_for`] reaches it), which is the whole of what this unit
//! owes that one. The transports — where bytes come from, and how the chrome talks to the engine —
//! are a later unit too.
//!
//! # Example
//!
//! The windowing arithmetic and the scroll anchor, which are this crate's own contribution and need
//! no document to show. The whole [`DocumentView`] flow — layout, scene building, invalidation and
//! the frame budget — is driven from `crates/mjx-view/tests/`, against a synthetic box model that
//! lives there rather than here: a crate whose whole claim is *"a viewport has never heard of a
//! document"* should not ship one.
//!
//! ```
//! use mjx_layout::{Extent, ExtentPrecision, LayoutSize, PageIndex};
//! use mjx_ooxml_core::measure::Emu;
//! use mjx_view::{PageWindow, ScrollDirection, ScrollModel, Viewport, WindowPolicy};
//!
//! // Four hundred pages, guessed at, the way a document looks the instant it opens.
//! let page = LayoutSize { width: Emu::from_inches(8.5), height: Emu::from_inches(11.0) };
//! let mut scroll = ScrollModel::from_extent(Extent {
//!     pages: 400,
//!     page_size: page,
//!     precision: ExtentPrecision::Estimated,
//! });
//! assert_eq!(scroll.total_height(), page.height.times(400));
//!
//! // A window one page tall, scrolled to page 200 and moving forward.
//! let mut viewport = Viewport::new(LayoutSize { width: page.width, height: page.height });
//! let top_of_page_200 = scroll.offset_of(PageIndex::new(200));
//! viewport.anchor = scroll.anchor_at(top_of_page_200);
//! viewport.direction = ScrollDirection::Forward;
//!
//! let window = PageWindow::of(&viewport, &mut scroll, WindowPolicy::default());
//! assert_eq!(window.first_visible(), PageIndex::new(200));
//! // Two pages ahead, one behind — the ring is biased the way the reader is going.
//! assert_eq!(window.first(), PageIndex::new(199));
//! assert_eq!(window.last(), PageIndex::new(202));
//!
//! // Page 200 turns out to be twice as tall as the estimate. The document grows…
//! let was_tall = scroll.total_height();
//! let was_here = scroll.offset_of_anchor(viewport.anchor);
//! scroll.record_measured_height(PageIndex::new(200), page.height.times(2));
//! assert!(scroll.total_height() > was_tall);
//!
//! // …and the reader has not moved, because the anchor is a page and not an offset.
//! assert_eq!(scroll.offset_of_anchor(viewport.anchor), was_here);
//! let round_trip = scroll.offset_of_anchor(viewport.anchor);
//! assert_eq!(scroll.anchor_at(round_trip), viewport.anchor);
//! ```

pub mod budget;
pub mod scene;
pub mod schedule;
pub mod scroll;
pub mod view;
pub mod viewport;

pub use budget::{CacheBudget, CacheReport, Stage, StageReport};
pub use scene::SceneSource;
pub use schedule::{
    FrameBudget, FrameClock, FrameReport, FrameTask, ManualFrameClock, RefinementTier, TaskKind,
};
pub use scroll::{PageMetric, ScrollAnchor, ScrollModel};
pub use view::{DocumentView, Invalidated, ViewFailure, ViewStats};
pub use viewport::{PageWindow, ScrollDirection, Viewport, WindowPolicy};
