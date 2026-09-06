//! [`BoxModel`] — the contract itself, and the vocabulary its four methods speak.
//!
//! # What a box model is
//!
//! Anything that can turn content into positioned fragments. PowerPoint's is absolute placement,
//! Word's is reflow with floats and page splitting, Excel's is a grid, and this crate's own tests
//! carry a fourth that reflows plain text into a fixed-width column and has never heard of OOXML.
//! All four answer the same four questions:
//!
//! * **Lay out one page** — [`BoxModel::layout_page`], resuming from a
//!   [`Checkpoint`] rather than from the beginning.
//! * **How big is the whole thing** — [`BoxModel::estimate_extent`], cheaply, so a scrollbar exists
//!   before page two has been laid out.
//! * **What did this edit break** — [`BoxModel::invalidate`], so a keystroke reflows a paragraph
//!   rather than a document.
//! * **Who am I** — [`BoxModel::signature`], so a checkpoint cannot be handed to the wrong model.
//!
//! # Why an abstraction with one implementation would be a guess
//!
//! A contract validated only by the implementation that shaped it proves nothing: every corner it
//! got wrong is a corner the one implementation happens not to use. That is why this crate's tests
//! carry a second box model that shares no ancestry with the OOXML ones, and why the first thing to
//! report about it is what writing it *forced to change here*.

use crate::checkpoint::{Checkpoint, ModelSignature};
use crate::fragment::FragmentTree;
use crate::index::SpatialIndex;
use crate::measure::{LayoutRect, LayoutSize};
use crate::source::SourceRef;
use mjx_ooxml_core::measure::Emu;
use mjx_text::TextDirection;

/// Which page, counted from zero.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct PageIndex(u32);

impl PageIndex {
    /// The first page.
    pub const FIRST: Self = Self(0);

    /// The page numbered `number`, counting from zero.
    #[must_use]
    pub const fn new(number: u32) -> Self {
        Self(number)
    }

    /// The number, counting from zero. A reader's "page 1" is `PageIndex::new(0)`.
    #[must_use]
    pub const fn number(self) -> u32 {
        self.0
    }

    /// The page after this one. The last page is its own successor rather than wrapping, because a
    /// wrap would send a scroll back to the beginning.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// The page before this one, or `None` for the first.
    #[must_use]
    pub const fn previous(self) -> Option<Self> {
        match self.0 {
            0 => None,
            number => Some(Self(number - 1)),
        }
    }
}

impl std::fmt::Display for PageIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Displayed one-based, because every message this appears in is read by a person and every
        // person's first page is page 1.
        write!(formatter, "{}", self.0.saturating_add(1))
    }
}

/// Which way lines stack and which way they run.
///
/// `mjx-dml` already models vertical text (`a:bodyPr@vert`), Word has `w:textDirection` and Excel
/// has rotated cell text, so the seam carries it from the start: a box model that could only ever
/// stack lines downward would have to be replaced rather than extended when the first vertical
/// Japanese slide arrived.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum WritingMode {
    /// Lines run horizontally and stack downward — Latin, Cyrillic, Arabic, and most Han.
    #[default]
    HorizontalTopToBottom,
    /// Lines run vertically and stack **right to left** — traditional Japanese and Chinese.
    VerticalRightToLeft,
    /// Lines run vertically and stack left to right — Mongolian.
    VerticalLeftToRight,
}

impl WritingMode {
    /// Whether lines run down the page rather than across it.
    #[must_use]
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::VerticalRightToLeft | Self::VerticalLeftToRight)
    }
}

/// The page a box model is laying content into.
///
/// Deliberately small. Everything else a real box model needs — indents, spacing, floats, autofit,
/// frozen panes — is in the *content*, because it comes from the document; what is here is what the
/// **caller** decides, which is the page and how the content area sits on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Constraints {
    /// The whole page, from its top-left corner. A slide, a sheet of paper, a printed worksheet.
    pub page: LayoutSize,
    /// The area content flows into — the page less its margins.
    ///
    /// Given as a rectangle rather than four margins so that a box model does not have to subtract,
    /// and so that a caller can ask for a region of a page (a header, a footnote area, a preview
    /// pane) without inventing margins that produce it.
    pub content: LayoutRect,
    /// How many columns the content area is divided into. Zero is read as one.
    pub columns: u16,
    /// The gap between two columns.
    pub column_gap: Emu,
    /// The base direction of the content area, which decides which side a line starts at.
    pub base_direction: TextDirection,
    /// Which way lines run and stack.
    pub writing_mode: WritingMode,
}

impl Constraints {
    /// A single left-to-right column filling `page` inside `margin` on every side.
    #[must_use]
    pub fn single_column(page: LayoutSize, margin: Emu) -> Self {
        Self {
            page,
            content: LayoutRect::from_edges(
                margin,
                margin,
                page.width - margin,
                page.height - margin,
            ),
            columns: 1,
            column_gap: Emu::ZERO,
            base_direction: TextDirection::LeftToRight,
            writing_mode: WritingMode::HorizontalTopToBottom,
        }
    }

    /// How many columns there really are — never zero, so that dividing by it is safe.
    #[must_use]
    pub fn column_count(self) -> u16 {
        self.columns.max(1)
    }

    /// The area of column `index`, or `None` when there is no such column or no room for one.
    #[must_use]
    pub fn column(self, index: u16) -> Option<LayoutRect> {
        let count = self.column_count();
        if index >= count {
            return None;
        }
        let gaps = self.column_gap.times(i64::from(count) - 1);
        let usable = self.content.width() - gaps;
        if usable <= Emu::ZERO {
            return None;
        }
        let width = usable.divided_by(i64::from(count));
        if width <= Emu::ZERO {
            return None;
        }
        let left = self.content.left + (width + self.column_gap).times(i64::from(index));
        Some(LayoutRect::from_edges(
            left,
            self.content.top,
            left + width,
            self.content.bottom,
        ))
    }
}

/// How much of a document there is — the number a scrollbar is drawn from.
///
/// Carries how it was arrived at, because a scrollbar drawn from a guess and a scrollbar drawn from
/// a measurement behave differently: the first must be allowed to move under the reader as real
/// pages are laid out, and the second must not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Extent {
    /// How many pages, at the constraints the estimate was made under.
    pub pages: u32,
    /// How big one page is.
    pub page_size: LayoutSize,
    /// Whether the page count is measured or guessed.
    pub precision: ExtentPrecision,
}

/// Whether an [`Extent`] is a guess or an answer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExtentPrecision {
    /// Derived without laying anything out — a character count divided by a characters-per-page
    /// figure, a row count divided by rows per page. Cheap, and wrong by a few percent.
    Estimated,
    /// Every page has been laid out and this is how many there were.
    Exact,
}

/// What changed in the content.
///
/// A box model is told *what* changed rather than being asked to diff, because the layer that made
/// the change knows and a diff of a document is more expensive than the layout it would avoid. This
/// is the receiving end of `docs/UI_PLATFORM_PLAN.md` §4 L-1's edit journal: every command reports
/// the addresses it dirtied, and they arrive here.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct ChangeSet {
    changes: Vec<ContentChange>,
}

impl ChangeSet {
    /// Nothing changed.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one change.
    pub fn record(&mut self, change: ContentChange) {
        self.changes.push(change);
    }

    /// Every change recorded, in the order they happened.
    #[must_use]
    pub fn changes(&self) -> &[ContentChange] {
        &self.changes
    }

    /// Whether nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// The earliest address any change touches, in document order, or `None` when nothing changed.
    ///
    /// This is the whole of what a *flow* box model needs: everything from the first change to the
    /// end of the document may move, because that is what flow means. A box model that places
    /// absolutely — PowerPoint's — reads the individual changes instead and dirties one page.
    #[must_use]
    pub fn earliest(&self) -> Option<&SourceRef> {
        self.changes.iter().map(|change| &change.source).min()
    }
}

/// One change to the content.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ContentChange {
    /// What was touched.
    pub source: SourceRef,
    /// How.
    pub kind: ChangeKind,
}

/// What kind of change was made.
///
/// Three kinds rather than one, because they invalidate differently: reformatting a run cannot move
/// anything before it, while inserting or removing content moves everything after it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChangeKind {
    /// Content appeared. Everything after it moves.
    Inserted,
    /// Content went away. Everything after it moves.
    Removed,
    /// The same content, differently formatted — a bold, a colour, a font size. It may still change
    /// how the content breaks, so it is not free, but nothing *before* it can move.
    Reformatted,
}

/// Which pages a change invalidated.
///
/// [`DirtyPages::From`] is the answer flow layout almost always gives, and saying so explicitly is
/// what stops a caller from re-laying out a whole document because it could not tell "pages 7
/// onward" from "everything".
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DirtyPages {
    /// Nothing needs re-laying out.
    None,
    /// These pages, and no others. A box model that places absolutely gives this.
    Pages(Vec<PageIndex>),
    /// This page and every page after it — what an edit to flowing content produces.
    From(PageIndex),
    /// Everything. A page-size change, a margin change, a font substitution.
    All,
}

impl DirtyPages {
    /// Whether `page` needs re-laying out.
    #[must_use]
    pub fn contains(&self, page: PageIndex) -> bool {
        match self {
            Self::None => false,
            Self::Pages(pages) => pages.contains(&page),
            Self::From(first) => page >= *first,
            Self::All => true,
        }
    }

    /// Whether nothing at all needs re-laying out.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Pages(pages) => pages.is_empty(),
            Self::From(_) | Self::All => false,
        }
    }
}

/// One page, laid out: the fragments, where they are, and how to lay out the next one.
#[derive(Clone, PartialEq, Debug)]
pub struct PageFragments {
    page: PageIndex,
    fragments: FragmentTree,
    index: SpatialIndex,
    continuation: Option<Checkpoint>,
}

impl PageFragments {
    /// A laid-out page. The spatial index is built here, from the finished tree, so that a hit test
    /// is a query rather than a walk and no caller can forget to build one.
    #[must_use]
    pub fn new(page: PageIndex, fragments: FragmentTree, continuation: Option<Checkpoint>) -> Self {
        let index = SpatialIndex::build(&fragments);
        Self {
            page,
            fragments,
            index,
            continuation,
        }
    }

    /// Which page this is.
    #[must_use]
    pub const fn page(&self) -> PageIndex {
        self.page
    }

    /// What is on it.
    #[must_use]
    pub const fn fragments(&self) -> &FragmentTree {
        &self.fragments
    }

    /// Where everything is.
    #[must_use]
    pub const fn index(&self) -> &SpatialIndex {
        &self.index
    }

    /// The checkpoint the next page is laid out from, or `None` when the content ended here.
    #[must_use]
    pub const fn continuation(&self) -> Option<&Checkpoint> {
        self.continuation.as_ref()
    }

    /// Whether this is the last page.
    #[must_use]
    pub const fn is_last(&self) -> bool {
        self.continuation.is_none()
    }

    /// The tree and the checkpoint, for a caller that keeps the checkpoint and drops the fragments —
    /// which is what a byte-budgeted page cache does.
    #[must_use]
    pub fn into_parts(self) -> (FragmentTree, Option<Checkpoint>) {
        (self.fragments, self.continuation)
    }
}

/// A box model: anything that can turn content into positioned fragments.
///
/// # `&mut self`, and why
///
/// Three of the four methods that could be `&self` are, and [`BoxModel::layout_page`] is not. Laying
/// out a page is where a box model's own caches live — a shaped-run cache, a resolved-style cache, a
/// measured-column cache — and a `&self` signature would force every one of them behind a lock that
/// a single-threaded layout pass pays for and does not need.
pub trait BoxModel {
    /// What this box model lays out. A parsed document, a string, a spreadsheet — whatever it is,
    /// this crate never looks inside it.
    type Content: ?Sized;

    /// What laying it out can fail with.
    ///
    /// The box model's own, not this crate's: only it knows what its content can be wrong about.
    /// [`LayoutError`](crate::LayoutError) is the shared half, and a box model that meets one wraps
    /// it — which is why `From<LayoutError>` is required rather than suggested.
    type Error: std::error::Error + From<crate::LayoutError>;

    /// Which box model this is. A constant; see [`ModelSignature`].
    fn signature(&self) -> ModelSignature;

    /// Lay out one page, resuming from `resume` rather than from the beginning.
    ///
    /// `resume` is the checkpoint that ended page `page - 1`, and is `None` exactly when `page` is
    /// [`PageIndex::FIRST`]. An implementation validates it with
    /// [`Checkpoint::state_for`], which checks both halves — the right
    /// model *and* the right page.
    ///
    /// # The equivalence this method promises
    ///
    /// Laying out pages 1..=*N* in order and laying out page *N* alone from *N−1*'s checkpoint must
    /// produce **the same fragments**. That is not a quality of implementation; it is what makes a
    /// scrollbar honest, and a box model that does not hold it will show a reader a different page
    /// depending on how they arrived at it.
    ///
    /// # Errors
    ///
    /// The box model's own, plus the shared [`LayoutError`](crate::LayoutError)s — a checkpoint from
    /// another model, a checkpoint for another page, or a page past the end of the content.
    fn layout_page(
        &mut self,
        content: &Self::Content,
        page: PageIndex,
        constraints: &Constraints,
        resume: Option<&Checkpoint>,
    ) -> Result<PageFragments, Self::Error>;

    /// How big the whole document is, without laying it out.
    ///
    /// Cheap enough to run over a whole document on every edit. An [`ExtentPrecision::Estimated`]
    /// answer is expected and is what a scrollbar is drawn from until real pages replace it.
    fn estimate_extent(&self, content: &Self::Content, constraints: &Constraints) -> Extent;

    /// Which pages `change` invalidated.
    fn invalidate(&mut self, change: &ChangeSet) -> DirtyPages;
}
