//! [`SessionError`] — everything a session can refuse, as a typed value.
//!
//! This crate holds the user's unsaved work. A panic here loses it, so nothing in a library path
//! unwraps, and every refusal below names what was asked for rather than aborting.

use std::error::Error;
use std::fmt;

/// What went wrong.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The document refused the operation. Carries whatever error the residency's own crate
    /// produced — a `PptxError`, a `DocxError`, a `XlsxError`, or a box model's own — boxed rather
    /// than enumerated, because the whole point of [`ResidentDocument`](crate::ResidentDocument) is
    /// that this crate does not know which documents exist.
    #[error("the document refused the operation: {0}")]
    Document(#[source] Box<dyn Error + Send + Sync>),

    /// The address named a node the document does not have, or one it cannot edit this way.
    #[error("no editable node at {what}")]
    NoSuchNode {
        /// The address, spelled the way the residency spells it.
        what: String,
    },

    /// The residency cannot express this operation for this node — a spreadsheet asked to place a
    /// box, a slide asked for a pooled string.
    #[error("this document cannot {what}")]
    Unsupported {
        /// What was asked.
        what: String,
    },

    /// The node's current value cannot be expressed as a [`Value`](crate::Value), so no exact
    /// inverse can be recorded and the edit is refused rather than made un-undoable.
    ///
    /// Refusing is the conservative reading of the project's standing rule that the user's own
    /// document wins: an undo that restored *approximately* what was there would be this library
    /// quietly authoring over markup it did not understand.
    #[error("the current value at {what} has no exact inverse, so the edit was refused")]
    NoExactInverse {
        /// The address, spelled the way the residency spells it.
        what: String,
    },

    /// A sink refused a write. The journal or the document did not reach durable storage, and the
    /// session has **not** truncated anything.
    #[error("a sink refused a write: {0}")]
    Sink(#[source] SinkError),

    /// A journal being recovered is not one this version wrote.
    #[error("the journal is not one this version wrote: {reason}")]
    UnreadableJournal {
        /// What about it was wrong.
        reason: &'static str,
    },
}

impl SessionError {
    /// Wraps a residency's own error.
    pub fn document<E: Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Document(Box::new(error))
    }

    /// The address `what` names nothing editable.
    pub fn no_such_node(what: impl fmt::Display) -> Self {
        Self::NoSuchNode {
            what: what.to_string(),
        }
    }

    /// This residency cannot do `what`.
    pub fn unsupported(what: impl fmt::Display) -> Self {
        Self::Unsupported {
            what: what.to_string(),
        }
    }

    /// The value at `what` cannot be inverted exactly.
    pub fn no_exact_inverse(what: impl fmt::Display) -> Self {
        Self::NoExactInverse {
            what: what.to_string(),
        }
    }
}

impl From<SinkError> for SessionError {
    fn from(error: SinkError) -> Self {
        Self::Sink(error)
    }
}

/// What a sink can refuse.
///
/// A sink is injected, so this crate has no idea whether it is a file, a browser's origin-private
/// filesystem, a network endpoint or a `Vec<u8>` in a test. It gets a message and nothing else.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SinkError(String);

impl SinkError {
    /// A refusal, described.
    pub fn new(message: impl fmt::Display) -> Self {
        Self(message.to_string())
    }
}
