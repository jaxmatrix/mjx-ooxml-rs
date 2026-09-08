//! The viewport, the direction it is moving, and which pages that makes worth having.
//!
//! # Windowing is the whole point
//!
//! A four-hundred-page document at 150 dpi is roughly four gigabytes of pixels. Nobody holds that
//! anywhere, client or server, so what a viewport materialises is *the pages on screen plus a small
//! ring around them* — and the ring is **biased in the direction of travel**, because a reader
//! scrolling down will want page 12 long before they want page 7.
//!
//! A symmetric ring would be half wasted at every moment of every scroll, and — worse for a gate — a
//! symmetric ring is what a windowing test that never scrolls cannot tell from a biased one. So
//! [`WindowPolicy`] takes two figures and `crates/mjx-view/tests/windowing.rs` asserts the window
//! differs by direction.

use mjx_layout::PageIndex;
use mjx_ooxml_core::measure::Emu;

use crate::scroll::{ScrollAnchor, ScrollModel};

/// Which way the reader is going.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum ScrollDirection {
    /// Not moving, or moving so little that neither side deserves the bias. The window is
    /// symmetric, which is the right answer for a document being read rather than travelled
    /// through.
    #[default]
    Still,
    /// Towards the end of the document.
    Forward,
    /// Towards the beginning.
    Backward,
}

/// What is on screen, and where.
///
/// # Units, and where zoom lives
///
/// [`size`](Self::size) is in **document** EMU — how much document fits on screen — not in pixels.
/// That is what makes zoom expressible without this crate owning a rendering scale: at 50 % zoom
/// twice as much document fits in the same window, so the shell divides its pixel size by the zoom
/// and hands the result here. [`Viewport::with_zoom`] does that division, so the relationship is
/// written down once rather than rediscovered by every caller.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Viewport {
    /// How much document is visible, in EMU.
    pub size: mjx_layout::LayoutSize,
    /// Where the top of it is. **An anchor, never an offset** — see [`crate::scroll`].
    pub anchor: ScrollAnchor,
    /// Which way it last moved.
    pub direction: ScrollDirection,
}

impl Viewport {
    /// A viewport of `size` at the top of the document, not moving.
    #[must_use]
    pub const fn new(size: mjx_layout::LayoutSize) -> Self {
        Self {
            size,
            anchor: ScrollAnchor::ORIGIN,
            direction: ScrollDirection::Still,
        }
    }

    /// A viewport showing `physical` EMU of screen at `zoom`.
    ///
    /// `zoom` is a multiplier: 1.0 is actual size, 0.5 shows twice as much document, 2.0 half as
    /// much. A zoom that is not finite and positive is read as 1.0, because a viewport is on the
    /// frame path and a bad number from a pinch gesture must not become a division by zero.
    #[must_use]
    pub fn with_zoom(physical: mjx_layout::LayoutSize, zoom: f64) -> Self {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        Self::new(mjx_layout::LayoutSize {
            width: scaled(physical.width, zoom),
            height: scaled(physical.height, zoom),
        })
    }
}

/// `length / zoom`, saturating, in EMU.
fn scaled(length: Emu, zoom: f64) -> Emu {
    #[allow(clippy::cast_precision_loss)]
    let emu = length.emu() as f64 / zoom;
    Emu::from_emu_rounded(emu)
}

/// How far around the visible pages to materialise.
///
/// # Why two numbers and not one
///
/// The ring is biased in the direction of travel: [`ahead`](Self::ahead) is the side the reader is
/// going, [`behind`](Self::behind) the side they came from. Scrolling down, `ahead` is the higher
/// page numbers; scrolling up it is the lower ones. Standing still, the larger of the two is used
/// on both sides, so a document being read rather than travelled through keeps its neighbours in
/// either direction.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WindowPolicy {
    /// Pages to materialise past the visible range, in the direction of travel.
    pub ahead: u32,
    /// Pages to keep on the side already passed.
    pub behind: u32,
}

impl Default for WindowPolicy {
    /// Two ahead and one behind.
    ///
    /// Two, because one page of lead time at a fling's speed is a blank page; one behind, because a
    /// reader who scrolls back one page must not wait for a re-layout — which
    /// `crates/mjx-view/tests/windowing.rs` asserts on the layout counter rather than assuming.
    fn default() -> Self {
        Self {
            ahead: 2,
            behind: 1,
        }
    }
}

impl WindowPolicy {
    /// A policy with no prefetch at all: the visible pages and nothing else.
    ///
    /// The identity value, and it is here so a gate can prove the prefetch is doing something. A
    /// windowing test that only ever ran the default would be green for an implementation that
    /// ignored the numbers.
    pub const NONE: Self = Self {
        ahead: 0,
        behind: 0,
    };
}

/// Which pages a frame should have, and which of those are actually on screen.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PageWindow {
    first_visible: PageIndex,
    last_visible: PageIndex,
    first: PageIndex,
    last: PageIndex,
}

impl PageWindow {
    /// The window a viewport implies, under `policy`, in a document `model` describes.
    ///
    /// The visible range is every page the viewport's rectangle touches — **not one page**, which is
    /// the mistake a viewport tested at a single size never catches: at 25 % zoom a screen holds
    /// four pages, and a window of one would leave three of them blank.
    #[must_use]
    pub fn of(viewport: &Viewport, model: &mut ScrollModel, policy: WindowPolicy) -> Self {
        let pages = model.pages();
        let last_page = PageIndex::new(pages.saturating_sub(1));
        let top = model.offset_of_anchor(viewport.anchor);
        let bottom = top + viewport.size.height;
        let first_visible = viewport.anchor.page.min(last_page);
        // Walk forward from the first visible page until a page starts at or past the bottom edge.
        // A walk rather than a second binary search because the count is the number of pages on
        // screen, which is small by construction — if it were not, the window would not be a window.
        let mut last_visible = first_visible;
        while last_visible < last_page {
            let next = last_visible.next();
            if model.offset_of(next) >= bottom {
                break;
            }
            last_visible = next;
        }
        let (ahead, behind) = match viewport.direction {
            ScrollDirection::Forward => (policy.ahead, policy.behind),
            ScrollDirection::Backward => (policy.behind, policy.ahead),
            ScrollDirection::Still => {
                let both = policy.ahead.max(policy.behind);
                (both, both)
            }
        };
        let first = PageIndex::new(first_visible.number().saturating_sub(behind));
        let last = PageIndex::new(last_visible.number().saturating_add(ahead)).min(last_page);
        Self {
            first_visible,
            last_visible,
            first,
            last,
        }
    }

    /// The first page on screen.
    #[must_use]
    pub const fn first_visible(&self) -> PageIndex {
        self.first_visible
    }

    /// The last page on screen.
    #[must_use]
    pub const fn last_visible(&self) -> PageIndex {
        self.last_visible
    }

    /// The first page worth having, prefetch included.
    #[must_use]
    pub const fn first(&self) -> PageIndex {
        self.first
    }

    /// The last page worth having, prefetch included.
    #[must_use]
    pub const fn last(&self) -> PageIndex {
        self.last
    }

    /// Whether `page` is on screen.
    #[must_use]
    pub fn is_visible(&self, page: PageIndex) -> bool {
        page >= self.first_visible && page <= self.last_visible
    }

    /// Whether `page` is worth having at all.
    #[must_use]
    pub fn contains(&self, page: PageIndex) -> bool {
        page >= self.first && page <= self.last
    }

    /// Every page on screen, in order.
    pub fn visible(&self) -> impl Iterator<Item = PageIndex> {
        (self.first_visible.number()..=self.last_visible.number()).map(PageIndex::new)
    }

    /// Every page worth having, **visible pages first and prefetch after**.
    ///
    /// The order is the priority order a frame executes in, and it is the order rather than the set
    /// that makes a frame budget useful: a frame that runs out of time has drawn the pages the
    /// reader can see and deferred the ones they cannot.
    pub fn in_priority_order(&self) -> impl Iterator<Item = PageIndex> + '_ {
        let visible = self.visible();
        let prefetch = (self.first.number()..=self.last.number())
            .map(PageIndex::new)
            .filter(move |page| !self.is_visible(*page));
        visible.chain(prefetch)
    }

    /// How many pages are in it, prefetch included.
    #[must_use]
    pub fn len(&self) -> usize {
        (self.last.number() - self.first.number()) as usize + 1
    }

    /// Whether it holds no pages. Never true — a window always holds the page the reader is on —
    /// and present because `clippy::len_without_is_empty` is right to ask.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }
}
