//! [`DocumentView`] — the windowing, the caches and the frame schedule, wired together.

use std::collections::BTreeMap;

use mjx_layout::{
    BoxModel, ChangeSet, Checkpoint, Constraints, DirtyPages, LayoutSize, PageFragments, PageIndex,
};
use mjx_ooxml_core::cache::{Admission, ByteBudgetCache, ENTRY_OVERHEAD_BYTES};
use mjx_ooxml_core::measure::Emu;
use mjx_scene::{diff_frames, DisplayList, FrameDiff};
use mjx_session::Invalidation;
use thiserror::Error;

use crate::budget::{CacheBudget, CacheReport, Stage, StageReport};
use crate::scene::SceneSource;
use crate::schedule::{FrameBudget, FrameClock, FrameReport, FrameTask, RefinementTier, TaskKind};
use crate::scroll::ScrollModel;
use crate::viewport::{PageWindow, ScrollDirection, Viewport, WindowPolicy};

/// Anything a frame can fail with.
///
/// # There is no third variant, and that is deliberate
///
/// A viewport has **no refusal of its own**. The obvious candidates — *no such page*, *past the end
/// of the content* — are questions only the box model can answer, and a viewport that answered them
/// would be a viewport that could disagree with the document it is showing. So the two errors here
/// are the box model's and the scene source's, carried **unchanged**.
///
/// The one case that looked like a viewport error is handled rather than reported: a document
/// estimated longer than it turns out to be will have a page past the end inside a window built
/// before the end was known, and a frame **skips** it and counts it in
/// [`FrameReport::pages_past_the_end`]. Failing there would mean a frame that errored because the
/// document was shorter than guessed, which is not an error at all.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum ViewFailure<L, S> {
    /// The box model refused to lay a page out.
    #[error("laying out a page: {0}")]
    Layout(L),
    /// The scene source refused to build a display list.
    #[error("building a display list: {0}")]
    Scene(S),
}

/// Everything a view has done, for a caller measuring it rather than trusting it.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct ViewStats {
    /// Frames run.
    pub frames: u64,
    /// Frames that spent more than their budget.
    pub frames_over_budget: u64,
    /// **Calls to [`BoxModel::layout_page`].** The figure every caching claim in this crate is
    /// measured by: scrolling back one page and re-laying it out looks exactly like scrolling back
    /// one page and not re-laying it out, unless something counts.
    pub pages_laid_out: u64,
    /// Calls to [`SceneSource::build`].
    pub scenes_built: u64,
    /// Tasks a frame skipped because the answer was already cached.
    pub cache_hits: u64,
    /// Tasks a frame ran out of time before reaching.
    pub tasks_deferred: u64,
    /// Layout tasks skipped because the document turned out to end before the page they named.
    ///
    /// Not an error and not a defect: a window is built from the *estimate*, and the estimate is
    /// allowed to be long. See [`ViewFailure`].
    pub pages_past_the_end: u64,
    /// Frames that had to ask the box model how long the document is, because an edit reflowed it.
    pub re_estimates: u64,
    /// Fragment-cache entries dropped because an edit made them stale.
    pub fragments_suppressed: u64,
    /// Display-list entries dropped because an edit made them stale.
    pub display_lists_suppressed: u64,
    /// Checkpoints dropped because an edit made the resume state stale.
    pub checkpoints_suppressed: u64,
}

/// What an [`invalidate`](DocumentView::invalidate) did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Invalidated {
    /// What the box model said had to be re-laid-out.
    pub pages: DirtyPages,
    /// How many cached pages of fragments were dropped.
    ///
    /// **The count the minimality gate asserts.** One keystroke into one paragraph of a
    /// twenty-page document must drop one page's fragments, and widening the change must make this
    /// number rise — which is how the gate proves it is measuring the invalidation and not the
    /// cache's size.
    pub fragments_suppressed: usize,
    /// How many cached display lists were dropped.
    pub display_lists_suppressed: usize,
    /// How many resume points were dropped.
    pub checkpoints_suppressed: usize,
}

/// A document, a window onto it, and the caches that make the window affordable.
///
/// See the crate documentation for the whole design; the two things worth knowing before reading
/// this type are that **checkpoints are kept for every page and fragments are not**, and that the
/// state a scroll position lives in is an *anchor* rather than an offset.
pub struct DocumentView<M: BoxModel, S: SceneSource> {
    model: M,
    scenes: S,
    constraints: Constraints,
    scroll: ScrollModel,
    viewport: Viewport,
    window_policy: WindowPolicy,
    budget: CacheBudget,
    frame_budget: FrameBudget,
    /// Resume points, keyed by the page each one **ends**. Kept for every page laid out and never
    /// evicted: one is at most `MAXIMUM_CHECKPOINT_BYTES`, so four hundred of them is four hundred
    /// kibibytes, and keeping them is what turns "scroll to page 300" into one page of layout.
    checkpoints: BTreeMap<u32, Checkpoint>,
    fragments: ByteBudgetCache<PageIndex, PageFragments>,
    display_lists: ByteBudgetCache<PageIndex, DisplayList>,
    /// The page that reported no continuation, once one has.
    ended_at: Option<PageIndex>,
    /// Set when an edit reflowed the document, so the next frame asks the box model how long it is
    /// now. See [`DocumentView::invalidate`].
    extent_stale: bool,
    stats: ViewStats,
}

impl<M: BoxModel, S: SceneSource> std::fmt::Debug for DocumentView<M, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentView")
            .field("viewport", &self.viewport)
            .field("window_policy", &self.window_policy)
            .field("frame_budget", &self.frame_budget)
            .field("checkpoints", &self.checkpoints.len())
            .field("fragments", &self.fragments)
            .field("display_lists", &self.display_lists)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl<M: BoxModel, S: SceneSource> DocumentView<M, S> {
    /// Opens a view onto `content`, drawing its scrollbar from
    /// [`BoxModel::estimate_extent`] rather than from a layout pass.
    ///
    /// This is the open-to-first-page path §12 budgets at 500 ms for fifty pages, and the reason it
    /// can be fast is that **nothing is laid out here**: the estimate gives a page count and a page
    /// size, the scroll model is built from those, and the first frame lays out only what the
    /// viewport shows.
    pub fn new(
        model: M,
        scenes: S,
        content: &M::Content,
        constraints: Constraints,
        viewport: Viewport,
        budget: CacheBudget,
    ) -> Self {
        let extent = model.estimate_extent(content, &constraints);
        Self {
            model,
            scenes,
            constraints,
            scroll: ScrollModel::from_extent(extent),
            viewport,
            window_policy: WindowPolicy::default(),
            fragments: ByteBudgetCache::new(budget.fragments),
            display_lists: ByteBudgetCache::new(budget.display_lists),
            budget,
            frame_budget: FrameBudget::default(),
            checkpoints: BTreeMap::new(),
            ended_at: None,
            extent_stale: false,
            stats: ViewStats::default(),
        }
    }

    /// The same view under a different prefetch policy.
    #[must_use]
    pub const fn with_window_policy(mut self, policy: WindowPolicy) -> Self {
        self.window_policy = policy;
        self
    }

    /// The same view under a different frame budget.
    #[must_use]
    pub const fn with_frame_budget(mut self, budget: FrameBudget) -> Self {
        self.frame_budget = budget;
        self
    }

    /// The box model, for reading.
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// The scene source, for reading.
    pub const fn scene_source(&self) -> &S {
        &self.scenes
    }

    /// The scroll model. `&mut` because every position it answers is derived from prefix sums it
    /// rebuilds lazily, and rebuilding on a shared reference would mean a lock.
    pub const fn scroll(&mut self) -> &mut ScrollModel {
        &mut self.scroll
    }

    /// Where the reader is.
    pub const fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// What the view has done.
    pub const fn stats(&self) -> ViewStats {
        self.stats
    }

    /// The ceilings it was built with.
    pub const fn budget(&self) -> &CacheBudget {
        &self.budget
    }

    /// Resizes the window — a resize, an orientation change, or a zoom the shell has already
    /// divided out ([`Viewport::with_zoom`]).
    ///
    /// The **anchor is kept**, so the page the reader was on stays the page they are on. Keeping a
    /// document *offset* instead would move them by however much the pages before them changed
    /// height, which on a phone rotating to landscape is most of the document.
    pub fn resize(&mut self, size: LayoutSize) {
        self.viewport.size = size;
    }

    /// Moves to a document offset, recording which way that was.
    ///
    /// This is the **one** place an offset becomes an anchor. See [`crate::scroll`]: everything
    /// afterwards is anchor arithmetic, which is what stops a correction to a page height from
    /// moving the reader.
    pub fn scroll_to(&mut self, offset: Emu) {
        let was = self.scroll.offset_of_anchor(self.viewport.anchor);
        self.viewport.anchor = self.scroll.anchor_at(offset);
        self.viewport.direction = if offset > was {
            ScrollDirection::Forward
        } else if offset < was {
            ScrollDirection::Backward
        } else {
            ScrollDirection::Still
        };
    }

    /// Moves by `delta`, positive towards the end of the document.
    pub fn scroll_by(&mut self, delta: Emu) {
        let offset = self.scroll.offset_of_anchor(self.viewport.anchor) + delta;
        self.scroll_to(offset);
    }

    /// Puts the top of `page` at the top of the window.
    pub fn scroll_to_page(&mut self, page: PageIndex) {
        let offset = self.scroll.offset_of(page);
        self.scroll_to(offset);
    }

    /// Where the reader is, as a document offset.
    pub fn scroll_offset(&mut self) -> Emu {
        self.scroll.offset_of_anchor(self.viewport.anchor)
    }

    /// Where the top of `page` sits **relative to the top of the window**, positive downward.
    ///
    /// **This is the scroll-stability measurement.** A page whose screen position changes while the
    /// reader is not scrolling is a page that jumped, and
    /// `crates/mjx-view/tests/scroll_stability.rs` asserts this figure across a sequence of height
    /// corrections that demonstrably move [`ScrollModel::total_height`].
    pub fn screen_position_of(&mut self, page: PageIndex) -> Emu {
        let top = self.scroll.offset_of(page);
        top - self.scroll_offset()
    }

    /// Which pages this frame would want.
    pub fn window(&mut self) -> PageWindow {
        PageWindow::of(&self.viewport, &mut self.scroll, self.window_policy)
    }

    /// The fragments held for `page`, without touching the eviction order.
    #[must_use]
    pub fn fragments_for(&self, page: PageIndex) -> Option<&PageFragments> {
        self.fragments.peek(&page)
    }

    /// The display list held for `page`, without touching the eviction order.
    #[must_use]
    pub fn display_list_for(&self, page: PageIndex) -> Option<&DisplayList> {
        self.display_lists.peek(&page)
    }

    /// What changed between two frames' display lists for `page`.
    ///
    /// `mjx-scene`'s frame diff, reached from here rather than re-derived, so a repaint uploads the
    /// records that moved instead of the page. `None` when either list is not held — a caller with
    /// nothing to compare against uploads the whole page, which is the correct answer and not a
    /// failure.
    #[must_use]
    pub fn diff_against(&self, page: PageIndex, previous: &DisplayList) -> Option<FrameDiff> {
        self.display_lists
            .peek(&page)
            .map(|next| diff_frames(previous, next))
    }

    /// How many bytes of checkpoint are held.
    #[must_use]
    pub fn checkpoint_bytes(&self) -> usize {
        self.checkpoints.values().fold(0usize, |sum, checkpoint| {
            sum.saturating_add(checkpoint.byte_size())
                .saturating_add(ENTRY_OVERHEAD_BYTES)
        })
    }

    /// What every stage the view holds is doing, against its declared ceiling.
    #[must_use]
    pub fn cache_report(&self) -> CacheReport {
        let checkpoints = StageReport {
            stage: Stage::Checkpoints,
            ceiling: self.budget.checkpoints,
            bytes: self.checkpoint_bytes(),
            entries: self.checkpoints.len(),
            // Never evicted, by design: see `crate::budget`.
            evictions: 0,
            suppressions: self.stats.checkpoints_suppressed,
            hits: 0,
            misses: 0,
        };
        let fragments = self.fragments.stats();
        let display_lists = self.display_lists.stats();
        CacheReport::new(vec![
            checkpoints,
            StageReport {
                stage: Stage::Fragments,
                ceiling: self.budget.fragments,
                bytes: self.fragments.bytes(),
                entries: self.fragments.len(),
                evictions: fragments.evictions,
                suppressions: fragments.suppressions,
                hits: fragments.hits,
                misses: fragments.misses,
            },
            StageReport {
                stage: Stage::DisplayLists,
                ceiling: self.budget.display_lists,
                bytes: self.display_lists.bytes(),
                entries: self.display_lists.len(),
                evictions: display_lists.evictions,
                suppressions: display_lists.suppressions,
                hits: display_lists.hits,
                misses: display_lists.misses,
            },
        ])
    }

    // ---------------------------------------------------------------------------------------------
    // Invalidation.
    // ---------------------------------------------------------------------------------------------

    /// Consumes the session's invalidation stream and drops exactly what it made stale.
    ///
    /// **The session says what changed; nothing here diffs a document.** `mjx-session` emits an
    /// [`Invalidation`] at apply time, which is the frame the edit happened in, and re-deriving the
    /// same fact by comparing documents would be both slower and wrong — a diff cannot tell a
    /// reformat from a replacement, and the box model's answer differs for the two.
    ///
    /// # What is dropped, and what is deliberately not
    ///
    /// The box model decides, through [`BoxModel::invalidate`]. A model that places absolutely
    /// answers [`DirtyPages::Pages`] and one paragraph's edit costs one page; a flow model answers
    /// [`DirtyPages::From`] and everything after the edit is stale, **including the resume points**,
    /// because a checkpoint is a state derived from the content before it. A checkpoint *ending* the
    /// page before the first dirty one is still good — it is the resume point *into* the change —
    /// which is why the retention below is strict rather than inclusive.
    ///
    /// # And why a reflow makes the document's *length* a guess again
    ///
    /// A page that reported no continuation ended the document, and from that moment the scrollbar's
    /// length is a fact rather than an estimate. An insertion undoes that: the document may now run
    /// past the page that used to end it. So a reflow clears both the end marker and the exact page
    /// count, and the **next frame** asks [`BoxModel::estimate_extent`] how long the document is now
    /// — on the frame rather than here, because that is where the content is, and R05 wrote
    /// `estimate_extent` to be *"cheap enough to run over a whole document on every edit"* for
    /// exactly this.
    pub fn invalidate(&mut self, invalidations: &[Invalidation]) -> Invalidated {
        if invalidations.is_empty() {
            return Invalidated {
                pages: DirtyPages::None,
                fragments_suppressed: 0,
                display_lists_suppressed: 0,
                checkpoints_suppressed: 0,
            };
        }
        let mut changes = ChangeSet::new();
        for invalidation in invalidations {
            changes.record(invalidation.content_change());
        }
        let pages = self.model.invalidate(&changes);
        let fragments_suppressed = self
            .fragments
            .suppress_matching(|page| pages.contains(*page));
        let display_lists_suppressed = self
            .display_lists
            .suppress_matching(|page| pages.contains(*page));
        let checkpoints_suppressed = match &pages {
            DirtyPages::None | DirtyPages::Pages(_) => 0,
            DirtyPages::From(first) => {
                let before = self.checkpoints.len();
                self.checkpoints.retain(|ends, _| *ends < first.number());
                self.ended_at = None;
                self.extent_stale = true;
                before - self.checkpoints.len()
            }
            DirtyPages::All => {
                let before = self.checkpoints.len();
                self.checkpoints.clear();
                self.ended_at = None;
                self.extent_stale = true;
                before
            }
        };
        self.stats.fragments_suppressed += fragments_suppressed as u64;
        self.stats.display_lists_suppressed += display_lists_suppressed as u64;
        self.stats.checkpoints_suppressed += checkpoints_suppressed as u64;
        Invalidated {
            pages,
            fragments_suppressed,
            display_lists_suppressed,
            checkpoints_suppressed,
        }
    }

    // ---------------------------------------------------------------------------------------------
    // The frame.
    // ---------------------------------------------------------------------------------------------

    /// Runs one frame: materialise what the window wants, in priority order, until the budget is
    /// spent.
    ///
    /// # The order, and why nothing is remembered between frames
    ///
    /// Tasks are derived from the **current** window every frame rather than carried over from the
    /// last one. A page that has left the window is not deferred work, it is work that is no longer
    /// wanted, and a queue that carried it would spend a fling's frames laying out pages behind the
    /// reader. [`FrameReport::deferred`] is therefore the tail of *this* frame's list, and the next
    /// frame finds the same pages still missing and tries again.
    ///
    /// # Errors
    /// [`ViewFailure`] — the viewport's own refusal, the box model's, or the scene source's.
    pub fn frame(
        &mut self,
        content: &M::Content,
        clock: &impl FrameClock,
        tier: RefinementTier,
    ) -> Result<FrameReport, ViewFailure<M::Error, S::Error>> {
        let started = clock.now_nanos();
        self.stats.frames += 1;
        if self.extent_stale {
            // A reflow happened since the last frame, so how long the document is is a guess again.
            // Asked here rather than in `invalidate` because this is where the content is.
            let extent = self.model.estimate_extent(content, &self.constraints);
            self.scroll.re_estimate(extent);
            self.extent_stale = false;
            self.stats.re_estimates += 1;
        }
        let window = self.window();

        // A pin set is replaced, never accumulated: an accumulating one is how a byte budget becomes
        // decoration one frame at a time.
        self.fragments.unpin_all();
        self.display_lists.unpin_all();
        for page in window.visible() {
            self.fragments.pin(&page);
            self.display_lists.pin(&page);
        }

        let mut cache_hits = 0usize;
        let mut tasks: Vec<FrameTask> = Vec::new();
        for page in window.in_priority_order() {
            let visible = window.is_visible(page);
            // `get` rather than `contains_key`, and deliberately: this is the moment a page is
            // *used*, so it is the moment its place in the eviction order has to move. A frame that
            // asked `contains_key` would leave every cached page ageing at the clock reading it was
            // inserted at, and the least recently used entry would be the least recently
            // **inserted** one — an order that looks right on a walk and is wrong the moment a
            // reader goes back to a page they have been reading. It is also what makes
            // `CacheStats::hits` a real figure rather than a field nothing writes.
            if self.fragments.get(&page).is_some() {
                cache_hits += 1;
            } else {
                tasks.push(FrameTask {
                    page,
                    kind: TaskKind::Layout,
                    visible,
                });
            }
            if tier.builds_scenes() {
                if self.display_lists.get(&page).is_some() {
                    cache_hits += 1;
                } else {
                    tasks.push(FrameTask {
                        page,
                        kind: TaskKind::Scene,
                        visible,
                    });
                }
            }
        }

        let mut pages_laid_out = 0usize;
        let mut scenes_built = 0usize;
        let mut past_the_end = 0usize;
        let mut deferred = Vec::new();
        for (position, task) in tasks.iter().enumerate() {
            // Checked **between** tasks and never before the first: a frame that made no progress
            // at all would never make any, and a viewport that never draws is worse than one that
            // drops a frame.
            if position > 0 && clock.now_nanos().saturating_sub(started) >= self.frame_budget.nanos
            {
                deferred.extend_from_slice(&tasks[position..]);
                break;
            }
            match task.kind {
                TaskKind::Layout => {
                    // The window was built from the scroll model, and the scroll model's page count
                    // is an estimate until a page reports no continuation. A page **after** the one
                    // that ended the content can therefore be in a window that was correct when it
                    // was built — including one built earlier in this very frame, since the page
                    // that ends the document may be laid out two tasks ago. Skipping it is the
                    // answer; failing would make a frame error because the document was shorter
                    // than guessed.
                    if self.is_past_the_end(task.page) {
                        past_the_end += 1;
                        continue;
                    }
                    let laid = self.lay_out(content, task.page, task.visible)?;
                    pages_laid_out += laid;
                }
                TaskKind::Scene => {
                    if self.build_scene(task.page, task.visible)? {
                        scenes_built += 1;
                    }
                }
            }
        }

        // A page unpinned by this frame may have left the caches over their ceilings; trimming is
        // what gives those bytes back, rather than waiting for the next insertion to notice.
        self.fragments.trim();
        self.display_lists.trim();

        let spent = clock.now_nanos().saturating_sub(started);
        // A page past the end is not incomplete; it does not exist. Counting it against completeness
        // would make a shell ask for frames for ever at the end of a document whose estimate was
        // long.
        let visible_complete = window.visible().all(|page| {
            self.is_past_the_end(page)
                || (self.fragments.contains_key(&page)
                    && (!tier.builds_scenes() || self.display_lists.contains_key(&page)))
        });
        self.stats.pages_laid_out += pages_laid_out as u64;
        self.stats.scenes_built += scenes_built as u64;
        self.stats.cache_hits += cache_hits as u64;
        self.stats.tasks_deferred += deferred.len() as u64;
        self.stats.pages_past_the_end += past_the_end as u64;
        if spent > self.frame_budget.nanos {
            self.stats.frames_over_budget += 1;
        }
        Ok(FrameReport {
            tier,
            budget_nanos: self.frame_budget.nanos,
            spent_nanos: spent,
            pages_laid_out,
            scenes_built,
            cache_hits,
            pages_past_the_end: past_the_end,
            deferred,
            visible_complete,
        })
    }

    /// Whether the content is already known to end before `page`.
    ///
    /// `false` until some page reports no continuation, because until then the page count is a
    /// guess and a guess must not refuse anything.
    fn is_past_the_end(&self, page: PageIndex) -> bool {
        self.ended_at.is_some_and(|ended| page > ended)
    }

    /// Lays out `page`, resuming from the nearest checkpoint and laying out whatever lies between.
    ///
    /// Answers how many pages were actually laid out, which is at most one when a checkpoint for the
    /// preceding page is held — and it always is, once that page has been laid out even once, which
    /// is the whole reason checkpoints are kept for every page.
    fn lay_out(
        &mut self,
        content: &M::Content,
        page: PageIndex,
        visible: bool,
    ) -> Result<usize, ViewFailure<M::Error, S::Error>> {
        let start = self.resume_start_for(page);
        let mut laid = 0usize;
        for number in start..=page.number() {
            let current = PageIndex::new(number);
            let resume = current
                .previous()
                .and_then(|previous| self.checkpoints.get(&previous.number()));
            let fragments = self
                .model
                .layout_page(content, current, &self.constraints, resume)
                .map_err(ViewFailure::Layout)?;
            laid += 1;
            // Only the page that was asked for is pinned. The pages between the resume point and it
            // are collateral — worth keeping if they fit, and not worth holding the budget open
            // for.
            self.absorb(current, fragments, visible && current == page);
        }
        Ok(laid)
    }

    /// The first page that has to be laid out in order to reach `page`.
    ///
    /// The page after the deepest checkpoint before it, or the first page when there is none.
    fn resume_start_for(&self, page: PageIndex) -> u32 {
        self.checkpoints
            .range(..page.number())
            .next_back()
            .map_or(0, |(ends, _)| ends + 1)
    }

    /// Files a freshly laid-out page: its checkpoint, its measured height, its fragments.
    fn absorb(&mut self, page: PageIndex, fragments: PageFragments, pin: bool) {
        let height = measured_height(&fragments, &self.constraints);
        self.scroll.record_measured_height(page, height);
        match fragments.continuation() {
            Some(checkpoint) => {
                self.checkpoints.insert(page.number(), checkpoint.clone());
                // The estimate said the document was shorter than it is. Extending keeps
                // `offset_of` meaningful for the page that was just laid out, and keeps the
                // scrollbar from stopping short of content that exists.
                self.scroll
                    .extend_to_at_least(page.number().saturating_add(2));
            }
            None => {
                // The content ended here, so the page count is no longer a guess.
                self.ended_at = Some(page);
                self.scroll
                    .record_exact_page_count(page.number().saturating_add(1));
            }
        }
        let cost = fragment_cost(&fragments);
        // A visible page is **retained**: it is what this frame paints, and refusing it would leave
        // the reader a blank page to save a byte. Nothing else about the budget changes — room is
        // made first, so the page just laid out is never the one evicted to fit it.
        self.fragments.insert_retained(page, fragments, cost);
        // **And pinned, if it is on screen.** Without this the prefetch tasks that follow in the
        // same frame can evict the page the frame exists to draw: the pin set was fixed at the top
        // of the frame from what was *already* cached, and a page laid out during the frame was
        // not in it. `tests/windowing.rs` caught exactly that — a jump back to page 4 cost six
        // pages of layout instead of four, because the visible page was evicted by its own
        // prefetch ring and re-laid-out on the following frame.
        if pin {
            self.fragments.pin(&page);
        }
    }

    /// Builds and files the display list for `page`. `false` when the fragments are not held, which
    /// happens when the layout task for the same page was deferred by the frame budget.
    fn build_scene(
        &mut self,
        page: PageIndex,
        visible: bool,
    ) -> Result<bool, ViewFailure<M::Error, S::Error>> {
        let Some(fragments) = self.fragments.peek(&page) else {
            return Ok(false);
        };
        let list = self
            .scenes
            .build(page, fragments)
            .map_err(ViewFailure::Scene)?;
        let cost = list.byte_len();
        if visible {
            self.display_lists.insert_retained(page, list, cost);
            self.display_lists.pin(&page);
        } else if let Admission::Rejected { .. } = self.display_lists.insert(page, list, cost) {
            // A prefetched page's display list that does not fit is simply not kept. The rejection
            // is counted by the cache and shows up in `cache_report`.
        }
        Ok(true)
    }
}

/// What holding one page of fragments is charged against the fragment budget.
///
/// The tree and the spatial index together, because [`PageFragments`] holds both and dropping one
/// without the other is not something a caller can do.
fn fragment_cost(fragments: &PageFragments) -> usize {
    fragments
        .fragments()
        .heap_bytes()
        .saturating_add(fragments.index().heap_bytes())
}

/// How tall a page turned out to be.
///
/// The nominal page height, or the bottom edge of its content when the content overruns it — which
/// is what a continuous box model produces, and what makes the height a *measurement* rather than
/// the constant it was estimated at. A paginated model returns the nominal height and the scroll
/// model records it as exact anyway, so "this page is exactly as tall as we guessed" is a different
/// state from "this page has not been laid out".
fn measured_height(fragments: &PageFragments, constraints: &Constraints) -> Emu {
    let tree = fragments.fragments();
    let mut bottom = constraints.page.height;
    for root in tree.roots() {
        if let Some(bounds) = tree.page_bounds(*root) {
            if bounds.bottom > bottom {
                bottom = bounds.bottom;
            }
        }
    }
    bottom
}
