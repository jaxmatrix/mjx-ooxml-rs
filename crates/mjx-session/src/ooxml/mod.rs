//! The three OOXML residencies — and the only place in this crate that names a format.
//!
//! Everything under `src/` outside this directory is generic over
//! [`ResidentDocument`](crate::ResidentDocument) and compiles with the format crates absent.
//! `crates/mjx-session/tests/the_seam_holds.rs` is what holds that true, by name and by file count.
//!
//! # How an address becomes a document node
//!
//! A [`SourceRef`](mjx_layout::SourceRef) is *opaque* by design: a part number, a path of small
//! integers and a character range, with no OOXML type in it. Its own documentation says the cost
//! plainly — *"a `SourceRef` means nothing without the box model that issued it"* — and names the
//! division: only the box model knows that path `[3, 1, 0]` is the fourth slide's second shape's
//! first paragraph, and only it can turn the number back into a document node.
//!
//! These three types are that box model's addressing half, written down now so the layout crates
//! (`mjx-layout-pptx` and its siblings) have a numbering to agree with rather than invent. Each
//! residency documents its own scheme on its own type, and the schemes do not have to match — that
//! is the whole point of the number being opaque.
//!
//! # What a residency owes, restated for these three
//!
//! * **Apply mutates the model only.** Every call below reaches an existing format-crate method;
//!   this crate wraps editing in residency and a journal and does not re-implement it.
//! * **Commit serialises each dirty part once**, through `mjx_opc::Package::settle_edited_parts`,
//!   and reports how many it actually wrote XML for.
//! * **The inverse is exact or the edit is refused.** A residency that could not put back exactly
//!   what was there returns [`SessionError::NoExactInverse`](crate::SessionError) rather than
//!   recording an undo that would author something. The one place that bites is a spreadsheet cell
//!   holding rich inline markup — see [`SpreadsheetSession`].

mod presentation;
mod spreadsheet;
mod word;

pub use presentation::PresentationSession;
pub use spreadsheet::{SpreadsheetSession, DEFAULT_RESIDENCY_BUDGET_BYTES};
pub use word::WordSession;

/// How much a residency counts an operation as having dirtied, beyond its payload.
///
/// The `dirty_bytes` trigger exists so that a bulk edit — pasting ten thousand rows — does not have
/// to wait for a clock. What it wants to know is *how much work is waiting*, and the honest cheap
/// answer is the size of the edits applied since the last commit: the payload, plus a constant for
/// the markup any edit costs whatever it carries. Asking the model how large it has become would
/// mean walking it, which is the expensive thing this whole design defers.
const OVERHEAD_PER_OPERATION: usize = 64;

/// The running estimate every residency keeps.
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct DirtyEstimate {
    bytes: usize,
}

impl DirtyEstimate {
    /// Adds one applied operation's payload.
    pub(crate) fn note(&mut self, payload_bytes: usize) {
        self.bytes = self
            .bytes
            .saturating_add(payload_bytes)
            .saturating_add(OVERHEAD_PER_OPERATION);
    }

    /// The estimate.
    pub(crate) const fn bytes(self) -> usize {
        self.bytes
    }

    /// Back to nothing, because a commit just wrote it all.
    pub(crate) fn clear(&mut self) {
        self.bytes = 0;
    }
}
