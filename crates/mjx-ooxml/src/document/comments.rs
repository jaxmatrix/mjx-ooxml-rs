//! Comments (`word/comments.xml`) — a flat, `Interner`-free summary of every comment
//! ([`CommentSummary`]), plus adding, removing and reading one comment's own resolved range.

use mjx_docx::DocxError;
use mjx_ooxml_core::FromXmlError;

use crate::error::Error;

use super::BlockPath;

/// One comment (`w:comment`), flattened to owned data — the facade's own mirror of
/// [`mjx_docx::Comment`], which needs the part's [`mjx_ooxml_core::Interner`] to answer `author`/
/// `id`/`text` and so cannot cross a binding boundary itself. See [`super::sections::SectionSummary`]
/// for the same pattern applied to a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentSummary {
    /// The comment's own id (`w:id`) — what [`super::Document::remove_comment`] and
    /// [`super::Document::comment_range_text`] take.
    pub id: i64,
    /// The comment's author (`w:author`).
    pub author: String,
    /// The comment author's initials (`w:initials`), if stated.
    pub initials: Option<String>,
    /// The comment's own text: every paragraph it holds, joined by a newline.
    pub text: String,
}

impl super::Document {
    /// Every comment this document's `word/comments.xml` holds, in document order — empty if the
    /// document relates to no `word/comments.xml` at all.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if `word/comments.xml`
    /// is related but cannot be read.
    pub fn comments(&mut self) -> Result<Vec<CommentSummary>, Error> {
        let read = self.document.comments(
            |comments, interner| -> Result<Vec<CommentSummary>, DocxError> {
                comments
                    .comments()
                    .map(|comment| {
                        Ok(CommentSummary {
                            id: comment.id(interner).map_err(FromXmlError::from)?,
                            author: comment.author(interner).unwrap_or_default(),
                            initials: comment.initials(interner),
                            text: comment.text(),
                        })
                    })
                    .collect()
            },
        )?;
        Ok(read.transpose()?.unwrap_or_default())
    }

    /// Adds a new comment on the **whole** paragraph at `paragraph`, creating `word/comments.xml`
    /// first if the document has none. Returns the comment's own freshly assigned id.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph`
    /// does not address one.
    pub fn add_comment(
        &mut self,
        paragraph: impl Into<BlockPath>,
        author: &str,
        initials: Option<&str>,
        text: &str,
    ) -> Result<i64, Error> {
        Ok(self
            .document
            .add_comment(paragraph.into().to_model(), author, initials, text)?)
    }

    /// Removes the comment with `id`: every `w:commentRangeStart`/`w:commentRangeEnd`/
    /// `w:commentReference` naming it, and the entry itself from `word/comments.xml`.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a related part
    /// cannot be read.
    pub fn remove_comment(&mut self, id: i64) -> Result<(), Error> {
        Ok(self.document.remove_comment(id)?)
    }

    /// [`comments`](Self::comments)'s own resolved range for the comment with `id`: the text between
    /// its `w:commentRangeStart`/`w:commentRangeEnd`. `None` if `id` names no comment whose range
    /// resolves (both markers found).
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body.
    pub fn comment_range_text(&mut self, id: i64) -> Result<Option<String>, Error> {
        Ok(self.document.comment_range_text(id)?)
    }
}
