//! The continuation state Word's box model writes, and what it is for.
//!
//! # This is the first checkpoint that carries anything
//!
//! PowerPoint's box model writes a checkpoint whose state it never reads: a slide's index *is* its
//! page number, so page *N* is reachable without page *N−1*. Excel's writes one it validates and
//! does not use, for the same reason — a band's first row comes from the page number and the row
//! geometry. Both are correct and neither exercises the design.
//!
//! **Word's is the case `mjx-layout`'s [`Checkpoint`] was written for.** Where page 200 starts
//! depends on everything on the 199 pages before it: a font substitution, a changed margin or a
//! single edited word moves every boundary after it. There is no arithmetic that answers *which
//! paragraph does page 200 start at*, and the only alternatives are to lay out 199 pages and throw
//! them away, or to remember the boundary. This is remembering it.
//!
//! # Thirteen bytes
//!
//! ```text
//! [0]      version
//! [1..5]   paragraph index, little-endian u32
//! [5..9]   line index within that paragraph, little-endian u32
//! [9..13]  how many paragraphs the document held, little-endian u32
//! ```
//!
//! The last field is not padding. A checkpoint is *bytes* and a caller may keep one across an edit,
//! and resuming a 900-paragraph checkpoint against a 40-paragraph document would address a
//! paragraph that is not the one the checkpoint meant — producing a page that looks entirely
//! plausible and holds the wrong content, which is the failure mode
//! [`Checkpoint::state_for`](mjx_layout::Checkpoint::state_for)'s own documentation names. It is a
//! guard rather than a proof: two documents of the same paragraph count still pass, and nothing
//! cheaper than a digest of the whole document would not.

use mjx_layout::{Checkpoint, ModelSignature, PageIndex};

use crate::address;
use crate::error::DocumentLayoutError;
use crate::paginate::FlowPosition;

/// This state's own version. A box model whose continuation changes shape raises it, and old
/// checkpoints are then refused rather than misread.
pub const VERSION: u8 = 1;

/// How many bytes one continuation is.
pub const STATE_BYTES: usize = 13;

/// The continuation state, decoded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Continuation {
    /// Where the next page starts.
    pub position: FlowPosition,
    /// How many paragraphs the document held when the checkpoint was written.
    pub paragraphs: u32,
}

impl Continuation {
    /// The bytes a [`Checkpoint`] carries.
    #[must_use]
    pub fn encode(self) -> [u8; STATE_BYTES] {
        let mut state = [0_u8; STATE_BYTES];
        state[0] = VERSION;
        state[1..5].copy_from_slice(&self.position.paragraph.to_le_bytes());
        state[5..9].copy_from_slice(&self.position.line.to_le_bytes());
        state[9..13].copy_from_slice(&self.paragraphs.to_le_bytes());
        state
    }

    /// The checkpoint that ends `page` and resumes here, for a document of `paragraphs` paragraphs.
    ///
    /// # Errors
    /// [`DocumentLayoutError::Layout`] if the state is somehow longer than
    /// [`MAXIMUM_CHECKPOINT_BYTES`](mjx_layout::MAXIMUM_CHECKPOINT_BYTES), which thirteen bytes
    /// cannot be — the call is fallible because the constructor is, and swallowing it would be the
    /// one `expect` on a layout path.
    pub fn into_checkpoint(
        self,
        model: ModelSignature,
        page: PageIndex,
    ) -> Result<Checkpoint, DocumentLayoutError> {
        Ok(Checkpoint::new(
            model,
            page,
            address::paragraph(self.position.paragraph as usize),
            self.encode().to_vec(),
        )?)
    }

    /// Reads the state a caller handed back, checking that it is this box model's shape and that it
    /// was made from a document of `paragraphs` paragraphs.
    ///
    /// # Errors
    /// [`DocumentLayoutError::MalformedContinuation`] when the bytes are not this shape, and
    /// [`DocumentLayoutError::StaleContinuation`] when the document has changed size under it.
    pub fn decode(state: &[u8], paragraphs: u32) -> Result<Self, DocumentLayoutError> {
        if state.len() != STATE_BYTES || state.first() != Some(&VERSION) {
            return Err(DocumentLayoutError::MalformedContinuation {
                found: state.len(),
                expected: STATE_BYTES,
            });
        }
        let read = |at: usize| -> u32 {
            state
                .get(at..at + 4)
                .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
                .map_or(0, u32::from_le_bytes)
        };
        let recorded = read(9);
        if recorded != paragraphs {
            return Err(DocumentLayoutError::StaleContinuation {
                recorded,
                found: paragraphs,
            });
        }
        Ok(Self {
            position: FlowPosition {
                paragraph: read(1),
                line: read(5),
            },
            paragraphs: recorded,
        })
    }
}
