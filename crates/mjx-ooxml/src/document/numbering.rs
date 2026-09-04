//! Numbering (`word/numbering.xml`) — attaching and detaching a paragraph's list reference
//! (`w:numPr`). Defining new numbering instances/abstract definitions is not part of this ticket's
//! curated surface (see [`super::Document`]'s own module doc) and stays reachable through
//! [`super::Document::document_mut`]'s [`mjx_docx::Document::edit_numbering`].

use crate::error::Error;

use super::BlockPath;

impl super::Document {
    /// Attaches the paragraph at `paragraph` to the numbering instance `numbering_id` at `level`
    /// (`w:numPr/w:numId` and `w:numPr/w:ilvl`), replacing any numbering reference it already
    /// carried.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph`
    /// does not address one.
    pub fn attach_paragraph_to_list(
        &mut self,
        paragraph: impl Into<BlockPath>,
        numbering_id: i64,
        level: i64,
    ) -> Result<(), Error> {
        Ok(self.document.attach_paragraph_to_list(
            paragraph.into().to_model(),
            numbering_id,
            level,
        )?)
    }

    /// Removes the paragraph at `paragraph`'s own `w:numPr`, if it carries one (a no-op otherwise).
    ///
    /// # Errors
    /// As [`attach_paragraph_to_list`](Self::attach_paragraph_to_list).
    pub fn detach_paragraph_from_list(
        &mut self,
        paragraph: impl Into<BlockPath>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .detach_paragraph_from_list(paragraph.into().to_model())?)
    }
}
