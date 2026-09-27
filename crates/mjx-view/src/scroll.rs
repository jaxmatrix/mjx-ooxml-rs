//! [`ScrollModel`] — where every page sits in the document, and why the scrollbar does not jump.
//!
//! # The problem, stated exactly
//!
//! A scrollbar has to exist the instant a document opens, and at that moment nothing has been laid
//! out. [`BoxModel::estimate_extent`](mjx_layout::BoxModel::estimate_extent) is what makes that
//! possible: a page count derived from a character count, marked
//! [`ExtentPrecision::Estimated`](mjx_layout::ExtentPrecision) and *wrong by a few percent*.
//!
//! Then pages get laid out, and the truth arrives one page at a time. If the model simply replaced
//! the estimate, every correction would move every offset after it — **including the offset the
//! reader is currently at** — and the page under their thumb would slide. A long document with a
//! scrollbar that jumps is unusable, and the ticket says so: this is a correctness requirement, not
//! polish.
//!
//! # The fix: the anchor is the state, and the offset is derived
//!
//! A [`ScrollAnchor`] is *a page and how far into it*, not a document offset. It is what a reader
//! actually means by "where I am": page 47, a third of the way down. A raw offset means the same
//! thing only while the pages before it keep the heights they were guessed at.
//!
//! So [`Viewport`](crate::Viewport) holds an anchor and derives the offset from this model. Correct
//! a page's height and:
//!
//! * [`ScrollModel::total_height`] changes — the scrollbar's *length* is now more nearly right;
//! * [`ScrollModel::offset_of`] for pages after the correction changes — they really did move;
//! * the anchor's own screen position **does not**, because the anchor was never an offset.
//!
//! `crates/mjx-view/tests/scroll_stability.rs` asserts all three at once, and the third alone would
//! be green for a model that never corrected anything — which is why the first two are asserted
//! beside it.

use mjx_layout::{Extent, ExtentPrecision, PageIndex};
use mjx_ooxml_core::measure::Emu;

/// Where a reader is, in terms that survive a correction.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ScrollAnchor {
    /// Which page the top of the viewport is inside.
    pub page: PageIndex,
    /// How far down that page, from its top edge. Never negative and never past the page's height,
    /// as long as it came from [`ScrollModel::anchor_at`].
    pub within: Emu,
}

impl ScrollAnchor {
    /// The very top of the document.
    pub const ORIGIN: Self = Self {
        page: PageIndex::FIRST,
        within: Emu::ZERO,
    };

    /// The top of `page`.
    #[must_use]
    pub const fn top_of(page: PageIndex) -> Self {
        Self {
            page,
            within: Emu::ZERO,
        }
    }
}

/// One page's height, and whether it is known or guessed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PageMetric {
    /// How tall.
    pub height: Emu,
    /// Whether that came from laying the page out or from the estimate.
    pub precision: ExtentPrecision,
}

/// Where every page sits, and how much document there is.
///
/// See the module documentation for why the anchor rather than the offset is the state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScrollModel {
    pages: Vec<PageMetric>,
    /// The height a page is guessed at until it is laid out.
    estimated_height: Emu,
    /// Whether the **page count** is known, as opposed to each page's height.
    count_precision: ExtentPrecision,
    /// Prefix sums, rebuilt lazily: `offsets[n]` is the top of page `n`, and the last entry is the
    /// whole document's height. Rebuilt rather than patched, because a patch that missed one page
    /// is a scrollbar that is wrong by exactly the amount nobody notices until they scroll to the
    /// end.
    offsets: Vec<Emu>,
    offsets_stale: bool,
}

impl ScrollModel {
    /// A model drawn from an estimate.
    ///
    /// Every page starts at the estimate's uniform height and marked
    /// [`ExtentPrecision::Estimated`]; the extent's own precision decides whether the **count** is
    /// already exact, which it is for a box model that places absolutely — a deck knows how many
    /// slides it has without laying one out.
    #[must_use]
    pub fn from_extent(extent: Extent) -> Self {
        let height = extent.page_size.height;
        let count = extent.pages.max(1) as usize;
        let mut model = Self {
            pages: vec![
                PageMetric {
                    height,
                    precision: ExtentPrecision::Estimated,
                };
                count
            ],
            estimated_height: height,
            count_precision: extent.precision,
            offsets: Vec::new(),
            offsets_stale: true,
        };
        model.rebuild_offsets();
        model
    }

    /// How many pages the model believes there are.
    #[must_use]
    pub fn pages(&self) -> u32 {
        // `u32` because `PageIndex` is one, and the vector was built from one.
        u32::try_from(self.pages.len()).unwrap_or(u32::MAX)
    }

    /// Whether the page **count** is known rather than guessed.
    #[must_use]
    pub const fn count_precision(&self) -> ExtentPrecision {
        self.count_precision
    }

    /// How many pages have had their height measured rather than guessed.
    ///
    /// The figure that says how much of the scrollbar is real. A test asserting that a correction
    /// happened reads this, because a stability assertion alone is green for a model that never
    /// corrects anything.
    #[must_use]
    pub fn measured_pages(&self) -> usize {
        self.pages
            .iter()
            .filter(|page| page.precision == ExtentPrecision::Exact)
            .count()
    }

    /// One page's metric, or `None` past the end.
    #[must_use]
    pub fn metric(&self, page: PageIndex) -> Option<PageMetric> {
        self.pages.get(page.number() as usize).copied()
    }

    /// How tall the whole document is.
    #[must_use]
    pub fn total_height(&mut self) -> Emu {
        self.rebuild_offsets();
        self.offsets.last().copied().unwrap_or(Emu::ZERO)
    }

    /// Where the top of `page` is, measured from the top of the document.
    ///
    /// Past the last page this answers the document's own height, so a caller that asks about a
    /// page that has just been removed is placed at the end rather than at zero.
    #[must_use]
    pub fn offset_of(&mut self, page: PageIndex) -> Emu {
        self.rebuild_offsets();
        let index = page.number() as usize;
        self.offsets
            .get(index)
            .copied()
            .unwrap_or_else(|| self.offsets.last().copied().unwrap_or(Emu::ZERO))
    }

    /// The anchor a document offset names.
    ///
    /// This is the **one** conversion from an offset into an anchor, and a caller does it once — at
    /// the moment the reader drags the scrollbar or the shell restores a saved position. Doing it
    /// every frame would put the offset back in charge and undo the whole design.
    #[must_use]
    pub fn anchor_at(&mut self, offset: Emu) -> ScrollAnchor {
        self.rebuild_offsets();
        if offset <= Emu::ZERO || self.pages.is_empty() {
            return ScrollAnchor::ORIGIN;
        }
        // `partition_point` over the prefix sums: the number of page tops at or below `offset`,
        // less one, is the page containing it. A binary search rather than a walk, because a
        // hundred-thousand-page document is a spreadsheet and this runs on the scroll path.
        let tops = &self.offsets[..self.pages.len()];
        let index = tops.partition_point(|top| *top <= offset).saturating_sub(1);
        let page = PageIndex::new(u32::try_from(index).unwrap_or(u32::MAX));
        let within = offset - tops[index];
        ScrollAnchor { page, within }
    }

    /// The document offset an anchor names.
    #[must_use]
    pub fn offset_of_anchor(&mut self, anchor: ScrollAnchor) -> Emu {
        self.offset_of(anchor.page) + anchor.within
    }

    /// Replaces `page`'s guessed height with the one it turned out to have.
    ///
    /// Idempotent, and cheap when the height has not changed: a correction that agrees with the
    /// guess still marks the page [`ExtentPrecision::Exact`] but leaves the prefix sums alone.
    pub fn record_measured_height(&mut self, page: PageIndex, height: Emu) {
        let Some(metric) = self.pages.get_mut(page.number() as usize) else {
            return;
        };
        let changed = metric.height != height;
        metric.height = height;
        metric.precision = ExtentPrecision::Exact;
        if changed {
            self.offsets_stale = true;
        }
    }

    /// The document turned out to have exactly `pages` pages.
    ///
    /// Called when a box model reports a page with no continuation. Shrinking drops the pages past
    /// the end; growing appends more at the estimated height, because a document that is longer
    /// than the estimate still needs a scrollbar for the part nobody has reached.
    pub fn record_exact_page_count(&mut self, pages: u32) {
        let wanted = pages.max(1) as usize;
        if self.pages.len() != wanted {
            self.pages.resize(
                wanted,
                PageMetric {
                    height: self.estimated_height,
                    precision: ExtentPrecision::Estimated,
                },
            );
            self.offsets_stale = true;
        }
        self.count_precision = ExtentPrecision::Exact;
    }

    /// The document turned out to be at least `pages` long.
    ///
    /// A box model that keeps producing continuations past the estimate is telling the model its
    /// guess was short; extending here is what keeps [`offset_of`](Self::offset_of) meaningful for
    /// the page that was just laid out.
    pub fn extend_to_at_least(&mut self, pages: u32) {
        let wanted = pages.max(1) as usize;
        if self.pages.len() < wanted {
            self.pages.resize(
                wanted,
                PageMetric {
                    height: self.estimated_height,
                    precision: ExtentPrecision::Estimated,
                },
            );
            self.offsets_stale = true;
        }
    }

    /// The document changed shape, and this is the box model's fresh guess at how long it is now.
    ///
    /// # Why an edit has to be able to make the count a *guess* again
    ///
    /// [`record_exact_page_count`](Self::record_exact_page_count) is called when a page reports no
    /// continuation, and from then on the scrollbar's length is a fact. An edit that inserts content
    /// makes it a guess again — the document may now be longer than the page that used to end it —
    /// and a model that kept the old figure would clamp the scrollbar short of content that exists,
    /// which is the one failure mode worse than a scrollbar that moves.
    ///
    /// Measured heights are **kept**, deliberately. A page whose fragments were suppressed will be
    /// laid out again and will report its height again; a page that was not suppressed did not move
    /// and its measurement is still right. Throwing them all away would send the scrollbar back to
    /// the estimate for a one-character edit.
    pub fn re_estimate(&mut self, extent: Extent) {
        let wanted = extent.pages.max(1) as usize;
        if self.pages.len() != wanted {
            self.pages.resize(
                wanted,
                PageMetric {
                    height: self.estimated_height,
                    precision: ExtentPrecision::Estimated,
                },
            );
            self.offsets_stale = true;
        }
        self.count_precision = extent.precision;
    }

    /// Rebuilds the prefix sums if anything moved.
    fn rebuild_offsets(&mut self) {
        if !self.offsets_stale {
            return;
        }
        self.offsets.clear();
        self.offsets.reserve(self.pages.len() + 1);
        let mut running = Emu::ZERO;
        for metric in &self.pages {
            self.offsets.push(running);
            running += metric.height;
        }
        self.offsets.push(running);
        self.offsets_stale = false;
    }
}
