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
//! # Forty-five bytes, and the rule that decided which
//!
//! ```text
//! [0]       version
//! [1..5]    paragraph index, little-endian u32
//! [5..9]    line index within that paragraph, little-endian u32
//! [9..13]   how many paragraphs the flow held, little-endian u32
//! [13..21]  the number this page displays, little-endian i64
//! [21..29]  the line-number counter this page opens with, little-endian i64
//! [29..33]  the note carried into this page, or u32::MAX for none, little-endian u32
//! [33..41]  that note's own number, little-endian i64
//! [41..45]  the first of its lines not yet placed, little-endian u32
//! ```
//!
//! MJXOFF-174 needed thirteen and MJXOFF-175 needs forty-five, and the four fields it added are
//! exactly the four things that **cannot be recomputed from the position**. That is the rule, and it
//! is worth stating because three plausible candidates failed it:
//!
//! * **which section a page is in** — a function of the paragraph index, which the position holds;
//! * **a footnote's number** — the *n*th reference in document order is note *n*, and how many
//!   precede a position is a prefix sum [`crate::model::DocumentFlow`] computes once when it reads
//!   the document (see [`crate::numbering`]);
//! * **whether a page is blank because an `evenPage` break demanded a parity** — a function of the
//!   position and the page number, both of which are here.
//!
//! What is left needs the history: the **page number** (a section may restart it, so it is not the
//! page index), the **continuous line-number counter** (a count of lines, which is exactly the thing
//! that cannot be known without laying them out), and the **carried note** with its number and the
//! line it resumes at.
//!
//! # The guard field is not padding
//!
//! A checkpoint is *bytes* and a caller may keep one across an edit, and resuming a 900-paragraph
//! checkpoint against a 40-paragraph document would address a paragraph that is not the one the
//! checkpoint meant — producing a page that looks entirely plausible and holds the wrong content,
//! which is the failure mode
//! [`Checkpoint::state_for`](mjx_layout::Checkpoint::state_for)'s own documentation names. It is a
//! guard rather than a proof: two documents of the same paragraph count still pass, and nothing
//! cheaper than a digest of the whole document would not.

use mjx_layout::{Checkpoint, ModelSignature, PageIndex};

use crate::address;
use crate::error::DocumentLayoutError;
use crate::notes::NoteCarry;
use crate::paginate::FlowPosition;

/// This state's own version. A box model whose continuation changes shape raises it, and old
/// checkpoints are then refused rather than misread.
///
/// Two at MJXOFF-175: sections, page numbering, line numbering and carried footnotes all needed
/// history, and a thirteen-byte MJXOFF-174 checkpoint decoded as one of these would resume a
/// document at page zero with no notes — plausible and wrong, which is the whole reason the version
/// is the first byte.
pub const VERSION: u8 = 2;

/// How many bytes one continuation is.
pub const STATE_BYTES: usize = 45;

/// The value the carried-note slot holds when nothing is carried.
const NO_CARRY: u32 = u32::MAX;

/// The continuation state, decoded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Continuation {
    /// Where the next page starts.
    pub position: FlowPosition,
    /// How many paragraphs the flow held when the checkpoint was written.
    pub paragraphs: u32,
    /// The number the next page displays.
    pub page_number: i64,
    /// The line number the next page's first numbered line carries, under
    /// `w:lnNumType@restart="continuous"`.
    pub line_number: i64,
    /// The footnote whose remaining lines open the next page's note area.
    pub carry: Option<NoteCarry>,
}

impl Continuation {
    /// The bytes a [`Checkpoint`] carries.
    #[must_use]
    pub fn encode(self) -> [u8; STATE_BYTES] {
        let mut state = [0_u8; STATE_BYTES];
        state[0] = VERSION;
        state[1..5].copy_from_slice(&self.position.block.to_le_bytes());
        state[5..9].copy_from_slice(&self.position.unit.to_le_bytes());
        state[9..13].copy_from_slice(&self.paragraphs.to_le_bytes());
        state[13..21].copy_from_slice(&self.page_number.to_le_bytes());
        state[21..29].copy_from_slice(&self.line_number.to_le_bytes());
        let (note, number, line) = match self.carry {
            Some(carry) => (
                u32::try_from(carry.note).unwrap_or(NO_CARRY),
                carry.number,
                carry.line,
            ),
            None => (NO_CARRY, 0, 0),
        };
        state[29..33].copy_from_slice(&note.to_le_bytes());
        state[33..41].copy_from_slice(&number.to_le_bytes());
        state[41..45].copy_from_slice(&line.to_le_bytes());
        state
    }

    /// The checkpoint that ends `page` and resumes here, for a flow of `paragraphs` paragraphs.
    ///
    /// # Errors
    /// [`DocumentLayoutError::Layout`] if the state is somehow longer than
    /// [`MAXIMUM_CHECKPOINT_BYTES`](mjx_layout::MAXIMUM_CHECKPOINT_BYTES), which forty-five bytes
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
            address::paragraph(self.position.block as usize),
            self.encode().to_vec(),
        )?)
    }

    /// Reads the state a caller handed back, checking that it is this box model's shape and that it
    /// was made from a flow of `paragraphs` paragraphs.
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
        let word = |at: usize| -> u32 {
            state
                .get(at..at + 4)
                .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
                .map_or(0, u32::from_le_bytes)
        };
        let long = |at: usize| -> i64 {
            state
                .get(at..at + 8)
                .and_then(|slice| <[u8; 8]>::try_from(slice).ok())
                .map_or(0, i64::from_le_bytes)
        };
        let recorded = word(9);
        if recorded != paragraphs {
            return Err(DocumentLayoutError::StaleContinuation {
                recorded,
                found: paragraphs,
            });
        }
        let note = word(29);
        let carry = if note == NO_CARRY {
            None
        } else {
            Some(NoteCarry {
                note: note as usize,
                number: long(33),
                line: word(41),
            })
        };
        Ok(Self {
            position: FlowPosition {
                block: word(1),
                unit: word(5),
            },
            paragraphs: recorded,
            page_number: long(13),
            line_number: long(21),
            carry,
        })
    }
}
