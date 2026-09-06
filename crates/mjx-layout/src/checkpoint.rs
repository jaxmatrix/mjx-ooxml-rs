//! [`Checkpoint`] — the small continuation token that makes a three-hundred-page document scrollable.
//!
//! # The problem it solves
//!
//! Flow layout is sequentially dependent. Where page 300 starts depends on where page 299 ended,
//! which depends on 298, all the way back. So the naive answer to *"the reader dragged the scrollbar
//! to page 300"* is to lay out 299 pages and throw them away, and that is unaffordable at any
//! document size a reader would notice.
//!
//! The observation that fixes it: the *state* at a page boundary is tiny — where in the content the
//! next page starts, plus whatever the box model was carrying — while the *fragments* of a page are
//! not. So a box model emits a checkpoint at every page boundary, the caller keeps all of them, and
//! page *N* is laid out from *N−1*'s checkpoint in the time one page takes. Page fragments live in a
//! byte-budgeted cache and are re-derived; checkpoints are kept.
//!
//! # Why the state is bytes and not a type parameter
//!
//! A box model's continuation state is its own — a Word one carries an open list's counters, an
//! Excel one a scroll of frozen rows, a plain-text one a byte offset — and the seam cannot name it.
//! Three shapes were possible:
//!
//! * an associated `type Checkpoint` on [`BoxModel`](crate::BoxModel), which is the most Rust-like
//!   and costs nothing at runtime, but makes every caller generic over the box model and makes a
//!   checkpoint impossible to write to disk or hand across the bridge to the shell;
//! * a `Box<dyn Any>`, which is neither `Clone` nor comparable nor serialisable, so the equivalence
//!   proof this crate's tests rest on could not be written;
//! * an opaque byte string with a signature, which is what this is.
//!
//! The bytes are **private to the box model that wrote them** and are not interpreted here. What is
//! shared, and inspectable, is the part every box model has: which page the checkpoint ends, and
//! where in the content the next one starts. A caller can therefore ask *"where am I?"* without the
//! box model, which is what a scrollbar and a page-number display need.
//!
//! # Why the signature exists
//!
//! Feeding one box model's bytes to another would be reading arbitrary data as structure, and the
//! consequence is a plausible-looking page of the wrong content rather than a visible failure. So
//! every box model declares a [`ModelSignature`] and every checkpoint carries the one that wrote it;
//! a mismatch is [`LayoutError::ForeignCheckpoint`], which is
//! a refusal a caller can act on.

use std::fmt;

use crate::error::LayoutError;
use crate::model::PageIndex;
use crate::source::SourceRef;

/// The most continuation state one checkpoint may carry.
///
/// A checkpoint is kept for **every page**, so a three-hundred-page document holds three hundred of
/// them; at this ceiling that is at most 300 KiB of state, which is affordable, and at ten times it
/// would not be. The ceiling is a design statement rather than a tuning knob: a box model that needs
/// more than this is keeping page *content* in its continuation, and the answer is to re-derive that
/// content on resume rather than to store it.
pub const MAXIMUM_CHECKPOINT_BYTES: usize = 1024;

/// Which box model wrote a checkpoint.
///
/// A constant the box model picks for itself and never changes — a version of it is part of the
/// value, so that a box model whose continuation state changes shape can pick a new signature and
/// have old checkpoints refused rather than misread.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ModelSignature(u64);

impl ModelSignature {
    /// The signature numbered `value`.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The number, for a diagnostic message.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ModelSignature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#018x}", self.0)
    }
}

/// The layout state at the end of one page, from which the next one is laid out.
///
/// Cheap to clone and cheap to keep: a page's worth of continuation, never a page's worth of
/// fragments.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Checkpoint {
    model: ModelSignature,
    ends_page: PageIndex,
    position: SourceRef,
    state: Box<[u8]>,
}

impl Checkpoint {
    /// A checkpoint ending `ends_page`, resuming at `position`, carrying `state`.
    ///
    /// `position` is a caret rather than a span: it addresses the first character the **next** page
    /// will lay out. Expressing it as a [`SourceRef`] rather than as a second address type is what
    /// lets a caller compare a checkpoint against a fragment's own address without a translation —
    /// "which page is this paragraph on" is a binary search over checkpoints.
    ///
    /// # Errors
    ///
    /// [`LayoutError::CheckpointTooLarge`] when `state` is longer than
    /// [`MAXIMUM_CHECKPOINT_BYTES`].
    pub fn new(
        model: ModelSignature,
        ends_page: PageIndex,
        position: SourceRef,
        state: impl Into<Box<[u8]>>,
    ) -> Result<Self, LayoutError> {
        let state = state.into();
        if state.len() > MAXIMUM_CHECKPOINT_BYTES {
            return Err(LayoutError::CheckpointTooLarge {
                bytes: state.len(),
                ceiling: MAXIMUM_CHECKPOINT_BYTES,
            });
        }
        Ok(Self {
            model,
            ends_page,
            position,
            state,
        })
    }

    /// Which box model wrote it.
    #[must_use]
    pub const fn model(&self) -> ModelSignature {
        self.model
    }

    /// The page it ends. The next page laid out from it is this one plus one.
    #[must_use]
    pub const fn ends_page(&self) -> PageIndex {
        self.ends_page
    }

    /// The page it resumes — the one a caller may pass it to.
    #[must_use]
    pub fn resumes_page(&self) -> PageIndex {
        self.ends_page.next()
    }

    /// Where in the content the next page starts.
    #[must_use]
    pub const fn position(&self) -> &SourceRef {
        &self.position
    }

    /// The box model's own state. Meaningless to anything but the box model that wrote it.
    #[must_use]
    pub fn state(&self) -> &[u8] {
        &self.state
    }

    /// How many bytes the checkpoint holds, for the caller that keeps one per page.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.state.len()
    }

    /// Check that this checkpoint is `model`'s and resumes `page`, and hand back its state.
    ///
    /// The first thing a box model's `layout_page` does with a `resume` argument. Doing it in one
    /// place is what stops each box model from inventing its own half of the check — and the half
    /// usually forgotten is the page, because a checkpoint from the *right model* and the *wrong
    /// page* produces a page that looks entirely plausible and holds the wrong content.
    ///
    /// # Errors
    ///
    /// [`LayoutError::ForeignCheckpoint`] when another box model wrote it, and
    /// [`LayoutError::MisplacedCheckpoint`] when it ends a page other than `page - 1`.
    pub fn state_for(&self, model: ModelSignature, page: PageIndex) -> Result<&[u8], LayoutError> {
        if self.model != model {
            return Err(LayoutError::ForeignCheckpoint {
                written_by: self.model,
                given_to: model,
            });
        }
        if self.resumes_page() != page {
            return Err(LayoutError::MisplacedCheckpoint {
                requested: page,
                ends_page: self.ends_page,
                expected: page.previous().unwrap_or(page),
            });
        }
        Ok(&self.state)
    }
}
