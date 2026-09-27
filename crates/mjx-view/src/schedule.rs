//! The frame budget, the clock it is read against, and the two refinement tiers.
//!
//! # Layout is off the input path, and this is how
//!
//! `docs/UI_PLATFORM_PLAN.md` §12 gives a frame 16.6 ms at 60 Hz and 8.3 ms at 120 Hz. Laying out a
//! page can cost more than that on its own, so a viewport that laid out everything its window asked
//! for would drop frames exactly when the reader is moving fastest.
//!
//! So a frame **executes a priority-ordered list of tasks until its budget is spent and defers the
//! rest to the next frame**. Visible pages come first, prefetch after; nothing is dropped, only
//! deferred, and [`FrameReport`] says how much of each happened. A budget nothing measures is a
//! comment, so every figure below is reported and
//! `crates/mjx-view/tests/frame_schedule.rs` asserts both sides of it: a frame that fits, and a
//! frame that does not.
//!
//! # Why one task always runs
//!
//! A frame that checked the budget before its first task and found it already spent would make no
//! progress at all, for ever — a viewport that never draws is worse than one that drops a frame. So
//! the budget is checked **between** tasks and never before the first, and a single task that
//! overruns the whole budget is reported as an overrun rather than prevented.
//!
//! # Why this crate has a second clock
//!
//! `mjx-session` already ships a [`Clock`](mjx_session::Clock) and a `ManualClock`, and this is not
//! an oversight. That one answers *"has two seconds of idle passed"* and its `Timestamp` is a count
//! of **milliseconds**, which is the right resolution for a commit schedule and the wrong one for a
//! frame: a 16.6 ms budget measured in whole milliseconds is a 16-or-17 ms budget, and the
//! difference is a dropped frame every few seconds. A frame budget is nanoseconds or it is nothing.
//!
//! # And why no clock reading a real time source ships here
//!
//! `std::time::Instant::now()` panics on `wasm32-unknown-unknown`, and this crate is in the
//! cross-build matrix for that target. A host already has a better time source than this crate
//! could pick — a browser has the `requestAnimationFrame` timestamp, a desktop shell has the
//! swapchain's — so the crate ships the trait and the clock a caller drives, and the shell supplies
//! the rest. That is the same division `mjx-session` made and for the same reason.

use std::cell::Cell;
use std::rc::Rc;

use mjx_layout::PageIndex;

/// A monotonic time source, in nanoseconds.
///
/// `&self` rather than `&mut self`: reading the time is not a mutation of the view.
pub trait FrameClock {
    /// Nanoseconds since an origin only the clock knows.
    fn now_nanos(&self) -> u64;
}

impl<T: FrameClock + ?Sized> FrameClock for Rc<T> {
    fn now_nanos(&self) -> u64 {
        (**self).now_nanos()
    }
}

impl<T: FrameClock + ?Sized> FrameClock for &T {
    fn now_nanos(&self) -> u64 {
        (**self).now_nanos()
    }
}

/// A clock a caller drives.
///
/// It is the test clock, and it is also the honest shape for a host that already has a frame
/// timestamp. Interior mutability with a [`Cell`] and no lock, because the whole render path must
/// work single-threaded: `wasm32` without cross-origin isolation has no threads, and correctness may
/// not depend on having them.
#[derive(Debug, Default)]
pub struct ManualFrameClock {
    nanos: Cell<u64>,
}

impl ManualFrameClock {
    /// A clock at the origin.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves it forward.
    pub fn advance_nanos(&self, nanos: u64) {
        self.nanos.set(self.nanos.get().saturating_add(nanos));
    }

    /// Moves it forward by whole microseconds, which is the unit a page of layout is measured in.
    pub fn advance_micros(&self, micros: u64) {
        self.advance_nanos(micros.saturating_mul(1_000));
    }
}

impl FrameClock for ManualFrameClock {
    fn now_nanos(&self) -> u64 {
        self.nanos.get()
    }
}

/// How long a frame has.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FrameBudget {
    /// Nanoseconds.
    pub nanos: u64,
}

impl Default for FrameBudget {
    /// 60 Hz — §12's figure, and what a display gives unless it says otherwise.
    fn default() -> Self {
        Self::at_refresh_rate(60)
    }
}

impl FrameBudget {
    /// The budget a display of `hertz` gives one frame.
    ///
    /// A refresh rate of zero is read as 60, because a display that reports nonsense must not
    /// become a viewport that never finishes a frame.
    #[must_use]
    pub const fn at_refresh_rate(hertz: u32) -> Self {
        let hertz = if hertz == 0 { 60 } else { hertz };
        Self {
            nanos: 1_000_000_000 / hertz as u64,
        }
    }

    /// A budget of `micros` microseconds.
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros * 1_000,
        }
    }

    /// **No budget at all**: every task a frame is given runs, however long it takes.
    ///
    /// The identity value. A viewport built with this schedules exactly as one with no scheduler,
    /// which is what makes it worth having: a frame-budget gate that only ever ran the real budget
    /// could not show that the budget is what produced the deferral.
    pub const UNLIMITED: Self = Self { nanos: u64::MAX };
}

/// How much fidelity a frame is asking for.
///
/// # Progressive refinement, and why it is two values rather than a number
///
/// During a fling the reader sees each page for a few tens of milliseconds, and building a display
/// list for a page that is about to leave the screen is work thrown away. So a fling asks for
/// [`RefinementTier::Preview`] — pages are laid out, so the geometry and the scrollbar are right,
/// and no scene is built — and the settle asks for [`RefinementTier::Full`].
///
/// Two values rather than a continuum because the *stages* are discrete: there is no half a display
/// list. A third tier would need a third thing to skip, and there is not one yet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Debug)]
pub enum RefinementTier {
    /// Lay pages out; build no display lists. What a fling gets.
    Preview,
    /// Everything. What a settled viewport gets.
    #[default]
    Full,
}

impl RefinementTier {
    /// Whether this tier builds display lists.
    #[must_use]
    pub const fn builds_scenes(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// One unit of work a frame can do.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FrameTask {
    /// Which page.
    pub page: PageIndex,
    /// What has to happen to it.
    pub kind: TaskKind,
    /// Whether the page is on screen, as opposed to in the prefetch ring.
    pub visible: bool,
}

/// What a [`FrameTask`] does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum TaskKind {
    /// Lay the page out — the expensive one, and the one that produces a checkpoint.
    Layout,
    /// Turn its fragments into a display list.
    Scene,
}

/// What one frame did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FrameReport {
    /// The tier the frame was asked for.
    pub tier: RefinementTier,
    /// The budget it had, in nanoseconds.
    pub budget_nanos: u64,
    /// What it actually spent.
    pub spent_nanos: u64,
    /// Pages laid out this frame.
    pub pages_laid_out: usize,
    /// Display lists built this frame.
    pub scenes_built: usize,
    /// Tasks answered from a cache, costing nothing.
    pub cache_hits: usize,
    /// Layout tasks skipped because the document ended before the page they named.
    ///
    /// A window is built from the scroll model's page count, which is an *estimate* until a page
    /// reports no continuation — so a window can legitimately name a page that does not exist, and
    /// a frame skips it rather than failing. Reaching a non-zero figure here means the estimate was
    /// long, which is the normal case for a document nobody has laid out yet.
    pub pages_past_the_end: usize,
    /// Work the budget ran out before reaching. **Deferred, never dropped** — the next frame starts
    /// with it.
    pub deferred: Vec<FrameTask>,
    /// Whether every page on screen has a display list, at [`RefinementTier::Full`], or fragments,
    /// at [`RefinementTier::Preview`].
    ///
    /// The one figure a shell needs in order to decide whether to keep asking for frames.
    pub visible_complete: bool,
}

impl FrameReport {
    /// Whether the frame spent more than it had.
    ///
    /// Not a failure: a single page of layout can cost more than a whole frame, and a viewport that
    /// refused to run it would never draw that page at all. It is a *measurement*, and
    /// `tests/frame_schedule.rs` asserts it on both sides.
    #[must_use]
    pub const fn overran(&self) -> bool {
        self.spent_nanos > self.budget_nanos
    }

    /// Whether anything was left for the next frame.
    #[must_use]
    pub fn deferred_anything(&self) -> bool {
        !self.deferred.is_empty()
    }
}
