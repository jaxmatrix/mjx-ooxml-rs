//! A synthetic document, a box model over it and a scene source — **none of which has heard of
//! OOXML**, which is the point.
//!
//! # Why the corpus is here and not in `mjx-fixtures`
//!
//! `mjx-fixtures` holds committed `.pptx`, `.docx` and `.xlsx` files, and reaching for one would
//! mean this crate's suite depended on a format crate to prove that this crate does not. The
//! windowing, the budgets and the frame schedule are properties of the *viewport*, and the honest
//! way to test them is a document the viewport has never seen before — generated, so that four
//! hundred pages cost nothing to keep in git and so that the size of a page is a parameter rather
//! than a fixture's accident.
//!
//! # What the model is faithful about, and what it is not
//!
//! It is faithful about the four things a viewport actually depends on:
//!
//! * **the resume contract** — [`FlowModel::layout_page`] refuses a checkpoint from another model
//!   or another page, exactly as `Checkpoint::state_for` requires, so a viewport that resumed from
//!   the wrong page would fail here rather than quietly produce different fragments;
//! * **pages of different heights** — a page marked *tall* lays its content out past the nominal
//!   page bottom, which is what gives `measured_height` something to measure and makes the
//!   scroll-stability gate more than an identity;
//! * **the two invalidation shapes** — a reformat dirties one page and an insertion dirties every
//!   page after it, which is the difference between PowerPoint's box model and Word's and is what
//!   the minimality gate widens across;
//! * **cost** — every page of layout advances a clock the test owns, so a frame budget is
//!   measurable without a sleep and without a real timer.
//!
//! It is not faithful about typography, and does not try to be: a fragment tree of boxes has the
//! same shape, the same heap cost and the same spatial index as one of glyph runs, and building the
//! glyph runs would drag a font database into a crate that has no business owning one.
//!
//! Nothing here is production code. The two allowances below are what a `#[path]`-included helper
//! module always needs: each test binary compiles the whole file and uses part of it, so every
//! unused helper would warn, and `pub` here reaches nothing outside the binary, so every
//! declaration would warn again. `clippy --all-targets -- -D warnings` turns both into failures.
#![allow(dead_code, unreachable_pub)]

use std::cell::Cell;
use std::rc::Rc;

use mjx_layout::{
    BoxFragment, ChangeKind, ChangeSet, Checkpoint, Constraints, DirtyPages, Extent,
    ExtentPrecision, Fragment, FragmentTreeBuilder, LayoutError, LayoutRect, LayoutSize,
    ModelSignature, PageFragments, PageIndex, PartId, SourcePath, SourceRef,
};
use mjx_ooxml_core::measure::Emu;
use mjx_scene::{
    Color, Command, DeviceScale, DisplayList, Geometry, Paint, SceneBuilder, SceneRect,
};
use mjx_view::{ManualFrameClock, SceneSource};

/// The signature every [`FlowModel`] reports.
///
/// A constant rather than a per-instance value, so a checkpoint written by one view is legitimately
/// resumable by another over the same content — which is what a test that opens a second view onto
/// the same document needs.
pub const MODEL: ModelSignature = ModelSignature::new(0x1968_0001);

/// How far a *tall* page's content runs past the bottom of its **content box**.
///
/// Two inches, and the figure is not arbitrary: [`letter`] leaves a one-inch margin, so a page's
/// measured height is `max(page height, content bottom)` and an overrun of less than the margin
/// would not change the measurement at all. A first draft of this used half an inch and the
/// scroll-stability gate went green on a document whose pages were all exactly the estimated
/// height — a correction that corrected nothing.
pub const OVERRUN: Emu = Emu::from_emu(914_400 * 2);

/// How much taller a *tall* page therefore is than an ordinary one: [`OVERRUN`] less the margin the
/// content box already left below it.
pub const TALL_PAGE_GROWTH: Emu = Emu::from_emu(914_400);

/// The synthetic document: how many pages, how much on each, and which of them are tall.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Paragraphs {
    /// How many pages there really are.
    pub pages: u32,
    /// How many boxes each page holds. The knob that decides what a page of fragments costs.
    pub blocks_per_page: u32,
    /// Every `nth` page is laid out taller than the page box. Zero means none are.
    pub every_nth_page_is_tall: u32,
    /// What [`estimate_extent`](FlowModel::estimate_extent) guesses, which is deliberately allowed
    /// to differ from [`pages`](Self::pages) — that difference is the whole reason a scrollbar has
    /// to be able to move.
    pub estimated_pages: u32,
}

impl Paragraphs {
    /// A document of `pages` pages, forty blocks each, estimated exactly, none of them tall.
    #[must_use]
    pub fn of(pages: u32) -> Self {
        Self {
            pages,
            blocks_per_page: 40,
            every_nth_page_is_tall: 0,
            estimated_pages: pages,
        }
    }

    /// The same document with `blocks` boxes on every page.
    #[must_use]
    pub fn with_blocks(mut self, blocks: u32) -> Self {
        self.blocks_per_page = blocks;
        self
    }

    /// The same document, guessed at `pages` pages rather than measured.
    #[must_use]
    pub fn estimated_at(mut self, pages: u32) -> Self {
        self.estimated_pages = pages;
        self
    }

    /// The same document with every `nth` page laid out past the page box.
    #[must_use]
    pub fn every_nth_page_tall(mut self, nth: u32) -> Self {
        self.every_nth_page_is_tall = nth;
        self
    }

    /// Whether `page` overruns its page box.
    #[must_use]
    pub fn is_tall(&self, page: PageIndex) -> bool {
        self.every_nth_page_is_tall != 0
            && page.number().is_multiple_of(self.every_nth_page_is_tall)
    }
}

/// How a change to the content invalidates pages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reflow {
    /// A reformat dirties its own page; an insertion dirties every page after it. **A flow model** —
    /// Word's shape.
    Flowing,
    /// Every change dirties only its own page. **An absolutely-placing model** — PowerPoint's shape.
    Absolute,
}

/// What laying out this document can go wrong with.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FlowError {
    /// The shared half.
    Layout(LayoutError),
    /// A page past the end of the content was asked for.
    NoSuchPage(u32),
}

impl std::fmt::Display for FlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Layout(error) => write!(f, "{error}"),
            Self::NoSuchPage(page) => write!(f, "page {page} is past the end of the content"),
        }
    }
}

impl std::error::Error for FlowError {}

impl From<LayoutError> for FlowError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

/// A box model over [`Paragraphs`].
#[derive(Debug)]
pub struct FlowModel {
    reflow: Reflow,
    clock: Option<Rc<ManualFrameClock>>,
    nanos_per_page: u64,
    /// How many times `layout_page` was called, which the tests read as the model's own opinion of
    /// how much work the viewport asked for. `DocumentView::stats` counts the same thing from the
    /// other side, and a disagreement between the two would mean one of them is wrong.
    calls: Cell<u64>,
}

impl FlowModel {
    /// A flow model that costs nothing and charges no clock.
    #[must_use]
    pub fn flowing() -> Self {
        Self {
            reflow: Reflow::Flowing,
            clock: None,
            nanos_per_page: 0,
            calls: Cell::new(0),
        }
    }

    /// A model that places absolutely, so a change dirties one page.
    #[must_use]
    pub fn absolute() -> Self {
        Self {
            reflow: Reflow::Absolute,
            ..Self::flowing()
        }
    }

    /// The same model, charging `micros` microseconds to `clock` for every page it lays out.
    #[must_use]
    pub fn costing(mut self, clock: &Rc<ManualFrameClock>, micros: u64) -> Self {
        self.clock = Some(Rc::clone(clock));
        self.nanos_per_page = micros.saturating_mul(1_000);
        self
    }

    /// How many pages this model has laid out.
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.calls.get()
    }
}

impl mjx_layout::BoxModel for FlowModel {
    type Content = Paragraphs;
    type Error = FlowError;

    fn signature(&self) -> ModelSignature {
        MODEL
    }

    fn layout_page(
        &mut self,
        content: &Paragraphs,
        page: PageIndex,
        constraints: &Constraints,
        resume: Option<&Checkpoint>,
    ) -> Result<PageFragments, FlowError> {
        if page.number() >= content.pages {
            return Err(FlowError::NoSuchPage(page.number()));
        }
        // **The resume contract, enforced rather than assumed.** A viewport that handed over the
        // checkpoint for the wrong page would otherwise get plausible fragments back and nobody
        // would learn.
        match (page == PageIndex::FIRST, resume) {
            (true, _) => {}
            (false, None) => {
                return Err(FlowError::Layout(LayoutError::MisplacedCheckpoint {
                    requested: page,
                    ends_page: page,
                    expected: PageIndex::new(page.number().saturating_sub(1)),
                }))
            }
            (false, Some(checkpoint)) => {
                checkpoint.state_for(MODEL, page)?;
            }
        }
        self.calls.set(self.calls.get() + 1);
        if let Some(clock) = &self.clock {
            clock.advance_nanos(self.nanos_per_page);
        }

        let bottom = if content.is_tall(page) {
            constraints.content.bottom + OVERRUN
        } else {
            constraints.content.bottom
        };
        let mut builder = FragmentTreeBuilder::with_capacity(content.blocks_per_page as usize + 1);
        let root = builder.push_simple(
            None,
            address(page, 0),
            LayoutRect::from_edges(
                constraints.content.left,
                constraints.content.top,
                constraints.content.right,
                bottom,
            ),
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );
        let height = (bottom - constraints.content.top)
            .divided_by(i64::from(content.blocks_per_page.max(1)));
        for block in 0..content.blocks_per_page {
            let top = constraints.content.top + height.times(i64::from(block));
            builder.push_simple(
                root,
                address(page, block + 1),
                LayoutRect::from_edges(
                    constraints.content.left,
                    top,
                    constraints.content.right,
                    top + height,
                ),
                Fragment::Box(BoxFragment {
                    decoration: None,
                    cell: None,
                }),
            );
        }

        let continuation = if page.number() + 1 >= content.pages {
            None
        } else {
            Some(Checkpoint::new(
                MODEL,
                page,
                address(page.next(), 0),
                // A state large enough to be worth budgeting and small enough to be legal, so the
                // checkpoint stage's byte figure is a real number rather than a handful of bytes.
                vec![0u8; 128],
            )?)
        };
        Ok(PageFragments::new(page, builder.finish(), continuation))
    }

    fn estimate_extent(&self, content: &Paragraphs, constraints: &Constraints) -> Extent {
        Extent {
            pages: content.estimated_pages,
            page_size: constraints.page,
            precision: ExtentPrecision::Estimated,
        }
    }

    fn invalidate(&mut self, change: &ChangeSet) -> DirtyPages {
        let mut pages: Vec<PageIndex> = Vec::new();
        let mut earliest_reflow: Option<PageIndex> = None;
        for one in change.changes() {
            let page = page_of(&one.source);
            match (self.reflow, one.kind) {
                (Reflow::Absolute, _) | (Reflow::Flowing, ChangeKind::Reformatted) => {
                    if !pages.contains(&page) {
                        pages.push(page);
                    }
                }
                (Reflow::Flowing, ChangeKind::Inserted | ChangeKind::Removed) => {
                    earliest_reflow =
                        Some(earliest_reflow.map_or(page, |already| already.min(page)));
                }
            }
        }
        match earliest_reflow {
            Some(first) => DirtyPages::From(first),
            None if pages.is_empty() => DirtyPages::None,
            None => DirtyPages::Pages(pages),
        }
    }
}

/// The address of one block on one page.
///
/// The scheme is the model's own and is opaque to everything above it, which is exactly what a
/// `SourceRef` promises — path segment 0 is the page, segment 1 the block.
#[must_use]
pub fn address(page: PageIndex, block: u32) -> SourceRef {
    SourceRef::node(PartId::PRIMARY, SourcePath::new(&[page.number(), block]))
}

/// Which page an address is on.
#[must_use]
pub fn page_of(source: &SourceRef) -> PageIndex {
    PageIndex::new(source.path().segments().first().copied().unwrap_or(0))
}

/// A scene source that draws one filled rectangle per fragment.
///
/// Enough to make a display list whose byte size scales with the page's content, which is all the
/// byte budget cares about, and nothing else. See `crates/mjx-view/src/scene.rs` for why the real
/// one cannot live in the viewport.
#[derive(Debug, Default)]
pub struct PlainScenes {
    calls: Cell<u64>,
}

impl PlainScenes {
    /// How many display lists it has built.
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.calls.get()
    }
}

impl SceneSource for PlainScenes {
    type Error = mjx_scene::SceneError;

    fn build(
        &mut self,
        _page: PageIndex,
        fragments: &PageFragments,
    ) -> Result<DisplayList, Self::Error> {
        self.calls.set(self.calls.get() + 1);
        let tree = fragments.fragments();
        let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 1_000.0, 1_000.0);
        let paint = builder.add_paint(Paint::Solid(Color {
            red: 0x20,
            green: 0x20,
            blue: 0x20,
            alpha: 0xFF,
        }))?;
        for (id, _node) in tree.nodes() {
            let Some(bounds) = tree.page_bounds(id) else {
                continue;
            };
            let geometry = builder.add_geometry(&Geometry::Rectangle(SceneRect {
                left: points(bounds.left),
                top: points(bounds.top),
                right: points(bounds.right),
                bottom: points(bounds.bottom),
            }))?;
            builder.push(Command::FillPath { geometry, paint })?;
        }
        builder.finish()
    }
}

/// EMU as points, for a scene rectangle.
fn points(length: Emu) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let value = length.points() as f32;
    value
}

/// A page of US Letter with a one-inch margin.
#[must_use]
pub fn letter() -> Constraints {
    Constraints::single_column(
        LayoutSize {
            width: Emu::from_inches(8.5),
            height: Emu::from_inches(11.0),
        },
        Emu::from_inches(1.0),
    )
}
