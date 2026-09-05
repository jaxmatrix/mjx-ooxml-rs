//! Footnotes, endnotes (`word/footnotes.xml`/`word/endnotes.xml`) and revisions (`w:ins`/`w:del`/…)
//! — user-visible notes flattened to owned data (the reserved `separator`/`continuationSeparator`/
//! `continuationNotice` entries every part carries are never surfaced here, matching
//! [`mjx_docx::Document::remove_footnote`]'s own "only ever means the last *user* footnote" rule),
//! plus the accepted/rejected revision-text readers, which are already concrete.

use mjx_docx::{DocxError, RevisionInfo};
use mjx_ooxml_core::FromXmlError;

use crate::error::Error;

use super::BlockPath;

/// One user-visible footnote or endnote, flattened to owned data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSummary {
    /// The note's own id (`w:id`) — what [`super::Document::remove_footnote`]/`remove_endnote` take.
    pub id: i64,
    /// The note's own text: every paragraph it holds, joined by a newline.
    pub text: String,
}

impl super::Document {
    /// Every **user-visible** footnote this document's `word/footnotes.xml` holds, in document
    /// order (the reserved separator entries every footnotes part carries are excluded) — empty if
    /// the document relates to no `word/footnotes.xml` at all.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if `word/footnotes.xml`
    /// is related but cannot be read.
    pub fn footnotes(&mut self) -> Result<Vec<NoteSummary>, Error> {
        let read = self.document.footnotes(
            |footnotes, interner| -> Result<Vec<NoteSummary>, DocxError> {
                footnotes
                    .user_footnotes(interner)
                    .map(|note| {
                        Ok(NoteSummary {
                            id: note.id(interner).map_err(FromXmlError::from)?,
                            text: note.text(),
                        })
                    })
                    .collect()
            },
        )?;
        Ok(read.transpose()?.unwrap_or_default())
    }

    /// Adds a new user footnote referenced from the end of the paragraph at `paragraph`, creating
    /// `word/footnotes.xml` first if the document has none. Returns the footnote's own freshly
    /// assigned id.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph`
    /// does not address one.
    pub fn add_footnote(
        &mut self,
        paragraph: impl Into<BlockPath>,
        text: &str,
    ) -> Result<i64, Error> {
        Ok(self
            .document
            .add_footnote(paragraph.into().to_model(), text)?)
    }

    /// Removes the user footnote with `id` — every `w:footnoteReference` naming it, and the entry
    /// itself. Never removes the part, and never a reserved entry (a no-op for either).
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a related part
    /// cannot be read.
    pub fn remove_footnote(&mut self, id: i64) -> Result<(), Error> {
        Ok(self.document.remove_footnote(id)?)
    }

    /// As [`footnotes`](Self::footnotes), for `word/endnotes.xml`.
    ///
    /// # Errors
    /// As [`footnotes`](Self::footnotes).
    pub fn endnotes(&mut self) -> Result<Vec<NoteSummary>, Error> {
        let read = self.document.endnotes(
            |endnotes, interner| -> Result<Vec<NoteSummary>, DocxError> {
                endnotes
                    .user_endnotes(interner)
                    .map(|note| {
                        Ok(NoteSummary {
                            id: note.id(interner).map_err(FromXmlError::from)?,
                            text: note.text(),
                        })
                    })
                    .collect()
            },
        )?;
        Ok(read.transpose()?.unwrap_or_default())
    }

    /// As [`add_footnote`](Self::add_footnote), for endnotes.
    ///
    /// # Errors
    /// As [`add_footnote`](Self::add_footnote).
    pub fn add_endnote(
        &mut self,
        paragraph: impl Into<BlockPath>,
        text: &str,
    ) -> Result<i64, Error> {
        Ok(self
            .document
            .add_endnote(paragraph.into().to_model(), text)?)
    }

    /// As [`remove_footnote`](Self::remove_footnote), for endnotes.
    ///
    /// # Errors
    /// As [`remove_footnote`](Self::remove_footnote).
    pub fn remove_endnote(&mut self, id: i64) -> Result<(), Error> {
        Ok(self.document.remove_endnote(id)?)
    }

    /// Every tracked-change marker the document body holds (`w:ins`/`w:del`/`w:moveFrom`/`w:moveTo`
    /// and the `*Change` wrappers), in document order.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body.
    pub fn revisions(&mut self) -> Result<Vec<RevisionInfo>, Error> {
        Ok(self.document.revisions()?)
    }

    /// The document body's own text with every tracked insertion kept and every tracked deletion
    /// dropped.
    ///
    /// # Errors
    /// As [`revisions`](Self::revisions).
    pub fn text_with_revisions_accepted(&mut self) -> Result<String, Error> {
        Ok(self.document.text_with_revisions_accepted()?)
    }

    /// [`text_with_revisions_accepted`](Self::text_with_revisions_accepted)'s own rejected-text
    /// counterpart: tracked deletions kept, tracked insertions dropped.
    ///
    /// # Errors
    /// As [`revisions`](Self::revisions).
    pub fn text_with_revisions_rejected(&mut self) -> Result<String, Error> {
        Ok(self.document.text_with_revisions_rejected()?)
    }
}
