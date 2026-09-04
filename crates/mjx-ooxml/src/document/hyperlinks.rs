//! Hyperlinks (`w:hyperlink`) — scoped to the main document body's own top-level run-or-hyperlink
//! slots, mirroring [`mjx_docx::Document`]'s own scoping decision (a header/footer hyperlink stays
//! reachable through [`super::Document::document_mut`]).

use mjx_docx::HyperlinkTarget;

use crate::error::Error;

use super::{BlockPath, RunPath};

impl super::Document {
    /// The click target of the hyperlink at top-level run-or-hyperlink slot `at` within the
    /// paragraph at `paragraph`, resolved against the main document part's own relationships.
    /// `None` if `at` does not land on a hyperlink, or the hyperlink resolves to neither a
    /// relationship nor an anchor.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph`
    /// does not address one.
    pub fn hyperlink_target(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
    ) -> Result<Option<HyperlinkTarget>, Error> {
        Ok(self
            .document
            .hyperlink_target(paragraph.into().to_model(), at.into().to_model())?)
    }

    /// Inserts a new hyperlink wrapping one run of `text` at top-level run-or-hyperlink slot `at`
    /// within the paragraph at `paragraph`, shifting every slot at or after that position one place
    /// later.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if either address
    /// is out of range.
    pub fn insert_hyperlink(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
        text: &str,
        target: &HyperlinkTarget,
    ) -> Result<(), Error> {
        Ok(self.document.insert_hyperlink(
            paragraph.into().to_model(),
            at.into().to_model(),
            text,
            target,
        )?)
    }

    /// Removes the hyperlink at top-level run-or-hyperlink slot `at` within the paragraph at
    /// `paragraph` — together with every run it wraps — and the relationship it named, unless some
    /// other hyperlink still names the same relationship.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if either address
    /// does not resolve to a hyperlink.
    pub fn remove_hyperlink(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .remove_hyperlink(paragraph.into().to_model(), at.into().to_model())?)
    }
}
