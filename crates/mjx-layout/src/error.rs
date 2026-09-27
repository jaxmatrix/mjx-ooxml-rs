//! What the shared layout machinery can fail with.
//!
//! A box model has an `Error` of its own — [`BoxModel::Error`](crate::BoxModel::Error) — because
//! only it knows what its content can be wrong about. [`LayoutError`] is the other half: the
//! failures that belong to *this* crate's own machinery and that every box model can meet, so that
//! none of them has to invent its own spelling for "that checkpoint is not mine".
//!
//! # Nothing here panics
//!
//! Every one of these is a condition a document or a caller can produce, and the crate's contract is
//! that a pathological document produces a bad-looking page rather than a crash. A rectangle with
//! `i64::MIN` for a width, a paragraph with four billion characters, a checkpoint from another box
//! model — each is an error or a saturation, never an `unwrap`.

use thiserror::Error;

use crate::checkpoint::ModelSignature;
use crate::model::PageIndex;

/// A failure in the shared layout machinery.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum LayoutError {
    /// A checkpoint written by one box model was handed to another.
    ///
    /// Continuation state is private to the box model that wrote it, so reading another's would be
    /// reading arbitrary bytes as structure. The signature is what makes that a typed refusal
    /// instead of a wrong page.
    #[error(
        "this checkpoint was written by box model {written_by} and handed to {given_to}; a \
         continuation token is private to the model that produced it"
    )]
    ForeignCheckpoint {
        /// The signature the checkpoint carries.
        written_by: ModelSignature,
        /// The signature of the box model it was given to.
        given_to: ModelSignature,
    },

    /// A checkpoint was handed to the wrong page.
    ///
    /// Page *N* resumes from the checkpoint that ended page *N−1*, and from no other. Resuming page
    /// 40 from page 12's checkpoint would produce a plausible-looking page of the wrong content,
    /// which is worse than a refusal.
    #[error(
        "page {requested} was asked to resume from the checkpoint that ended page {ends_page}; it \
         resumes only from the checkpoint that ended page {expected}"
    )]
    MisplacedCheckpoint {
        /// The page that was asked for.
        requested: PageIndex,
        /// The page the checkpoint actually ends.
        ends_page: PageIndex,
        /// The page whose checkpoint that request needed.
        expected: PageIndex,
    },

    /// A box model tried to write more continuation state than a checkpoint may carry.
    ///
    /// Checkpoints are kept for **every page** of a document that may be three hundred pages long,
    /// which is only affordable while each is small. A box model that needs more than
    /// [`MAXIMUM_CHECKPOINT_BYTES`](crate::checkpoint::MAXIMUM_CHECKPOINT_BYTES) is keeping page
    /// *content* in its continuation, and the answer is to re-derive it rather than to raise the
    /// ceiling.
    #[error(
        "a checkpoint carried {bytes} bytes of continuation state, past the {ceiling}-byte ceiling; \
         a continuation token is the layout state at a page boundary, not the page"
    )]
    CheckpointTooLarge {
        /// How many bytes the box model tried to store.
        bytes: usize,
        /// The ceiling it passed.
        ceiling: usize,
    },

    /// A page was asked for beyond the last one the content produces.
    #[error("page {requested} was asked for, and the content ended at page {last}")]
    PageBeyondContent {
        /// The page that was asked for.
        requested: PageIndex,
        /// The last page there is.
        last: PageIndex,
    },

    /// The area content was asked to flow into encloses no space at all.
    ///
    /// A zero-width column cannot hold a line and a zero-height one cannot hold a page, and looping
    /// while trying is how a layout engine hangs on a malformed document.
    #[error(
        "content was given an empty area to flow into ({width} by {height} EMU); no line fits, so \
         no page can be produced"
    )]
    EmptyContentArea {
        /// How wide the area was.
        width: i64,
        /// How tall it was.
        height: i64,
    },
}
