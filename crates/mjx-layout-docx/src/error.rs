//! What laying out a document can fail with.

use mjx_layout::LayoutError;
use mjx_text::FontError;

/// Everything Word's box model can refuse.
///
/// Deliberately short. A document is untrusted input, so almost everything a bad one can do is
/// *degraded* rather than refused — a paragraph wider than the page overflows, an unsatisfiable
/// `w:keepNext` chain is broken, a zero-width column falls back to one glyph's width — and the
/// variants here are the cases where continuing would mean showing a reader the wrong content
/// rather than a bad-looking page.
#[derive(Debug, thiserror::Error)]
pub enum DocumentLayoutError {
    /// The document could not be read at all.
    #[error("reading the document: {0}")]
    Document(#[from] mjx_docx::DocxError),

    /// A face would not shape.
    #[error("shaping: {0}")]
    Font(#[from] FontError),

    /// The shared half: a checkpoint from another box model, a checkpoint for another page, a page
    /// past the end of the document.
    #[error(transparent)]
    Layout(#[from] LayoutError),

    /// A checkpoint this box model wrote, whose bytes are not the shape it writes.
    ///
    /// Distinct from [`LayoutError::ForeignCheckpoint`]: the signature matched, so another *version*
    /// of this box model wrote it, or the bytes were corrupted in transit. Either way the state
    /// cannot be read and laying out from it would produce a plausible page of the wrong content.
    #[error("this box model's continuation state is {found} bytes, and it writes {expected}")]
    MalformedContinuation {
        /// How many bytes arrived.
        found: usize,
        /// How many this box model writes.
        expected: usize,
    },

    /// A checkpoint made from a different document.
    ///
    /// The continuation carries the document's paragraph count, and a mismatch means the content
    /// changed under the checkpoint. Resuming would address a paragraph that is not the one the
    /// checkpoint meant.
    #[error(
        "the continuation was made from a document of {recorded} paragraphs and this one has \
         {found}"
    )]
    StaleContinuation {
        /// What the checkpoint recorded.
        recorded: u32,
        /// What the content holds now.
        found: u32,
    },

    /// The content area has no width or no height to lay anything into.
    #[error("the content area is {width} x {height} EMU, which cannot hold a line")]
    EmptyContentArea {
        /// Its width.
        width: i64,
        /// Its height.
        height: i64,
    },
}
