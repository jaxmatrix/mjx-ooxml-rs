//! Paragraph and run reading and editing (`w:p`/`w:r`/`w:t`) — the facade's own `u32` addressing
//! over [`mjx_docx::Document`]'s already-flattened paragraph/run methods.

use crate::error::Error;

use super::{BlockPath, RunPath};

impl super::Document {
    /// How many paragraphs the document body holds, or `0` if it declares no body.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the main document
    /// part cannot be read.
    pub fn paragraph_count(&mut self) -> Result<u32, Error> {
        Ok(crate::index::count(self.document.paragraph_count()?))
    }

    /// How many run-or-hyperlink slots the paragraph at `paragraph` holds at its own top level.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph`
    /// does not address one.
    pub fn run_count(&mut self, paragraph: impl Into<BlockPath>) -> Result<u32, Error> {
        Ok(crate::index::count(
            self.document.run_count(paragraph.into().to_model())?,
        ))
    }

    /// The whole text of the paragraph at `paragraph` — every run reachable from it, including runs
    /// nested inside a hyperlink, concatenated in document order.
    ///
    /// # Errors
    /// As [`run_count`](Self::run_count).
    pub fn paragraph_text(&mut self, paragraph: impl Into<BlockPath>) -> Result<String, Error> {
        Ok(self.document.paragraph_text(paragraph.into().to_model())?)
    }

    /// The text of the run at `run` within the paragraph at `paragraph` — the concatenation of every
    /// `w:t` the run holds.
    ///
    /// # Errors
    /// As [`run_count`](Self::run_count), plus the same for `run`.
    pub fn run_text(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
    ) -> Result<String, Error> {
        Ok(self
            .document
            .run_text(paragraph.into().to_model(), run.into().to_model())?)
    }

    /// Sets the text of the run at `run` within the paragraph at `paragraph`. Only `word/document.xml`
    /// is dirtied, and only the edited run's own byte range re-serializes.
    ///
    /// # Errors
    /// As [`run_text`](Self::run_text).
    pub fn set_run_text(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
        text: &str,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_run_text(paragraph.into().to_model(), run.into().to_model(), text)?)
    }

    /// Inserts a new, empty paragraph so it becomes the paragraph at `at`, shifting every paragraph
    /// at or after that position one place later. `at` must address an existing paragraph or the one
    /// past the last (`0..=paragraph_count()`).
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `at` is out of
    /// range.
    pub fn insert_paragraph(&mut self, at: impl Into<BlockPath>) -> Result<(), Error> {
        Ok(self.document.insert_paragraph(at.into().to_model())?)
    }

    /// Appends a new, empty paragraph as the body's new last paragraph.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body.
    pub fn append_paragraph(&mut self) -> Result<(), Error> {
        Ok(self.document.append_paragraph()?)
    }

    /// Removes the paragraph at `at`.
    ///
    /// # Errors
    /// As [`insert_paragraph`](Self::insert_paragraph).
    pub fn remove_paragraph(&mut self, at: impl Into<BlockPath>) -> Result<(), Error> {
        Ok(self.document.remove_paragraph(at.into().to_model())?)
    }

    /// Inserts a new run holding `text` so it becomes the top-level run-or-hyperlink slot `at` within
    /// the paragraph at `paragraph`, shifting every slot at or after that position one place later.
    ///
    /// # Errors
    /// As [`run_text`](Self::run_text).
    pub fn insert_run(
        &mut self,
        paragraph: impl Into<BlockPath>,
        at: impl Into<RunPath>,
        text: &str,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .insert_run(paragraph.into().to_model(), at.into().to_model(), text)?)
    }

    /// Appends a new run holding `text` as the paragraph's new last top-level run.
    ///
    /// # Errors
    /// As [`run_count`](Self::run_count).
    pub fn append_run(&mut self, paragraph: impl Into<BlockPath>, text: &str) -> Result<(), Error> {
        Ok(self
            .document
            .append_run(paragraph.into().to_model(), text)?)
    }

    /// Removes the run at `run` within the paragraph at `paragraph`.
    ///
    /// # Errors
    /// As [`run_text`](Self::run_text).
    pub fn remove_run(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .remove_run(paragraph.into().to_model(), run.into().to_model())?)
    }
}
