//! What laying a slide out can fail with.
//!
//! Three sources, and they are kept apart on purpose: the shared machinery's
//! ([`LayoutError`] — a foreign checkpoint, an empty content area), the
//! font engine's ([`FontError`] — a face that will not shape), and the deck's
//! ([`PptxError`] — a malformed package, an index out of range). A caller that
//! wants to know *which* can match; one that does not can print it.
//!
//! # Nothing here is a panic
//!
//! A slide comes from an untrusted file. A shape with no bounds, a paragraph with no runs, a font
//! size of zero, a column count of sixty thousand, a text body whose insets are wider than the shape
//! — every one of them produces a page that looks wrong rather than a crash, and
//! `tests/no_panic_on_a_layout_path.rs` holds that true by scanning the source rather than by
//! assertion.

use mjx_layout::LayoutError;
use mjx_pptx::PptxError;
use mjx_text::FontError;
use thiserror::Error;

/// A failure laying out a slide.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SlideLayoutError {
    /// The shared layout machinery refused something — a checkpoint from another box model, a
    /// checkpoint for another page, a page past the last slide, an empty content area.
    #[error(transparent)]
    Layout(#[from] LayoutError),

    /// A face would not shape.
    #[error(transparent)]
    Text(#[from] FontError),

    /// The deck would not answer — a malformed part, an index out of range, a relationship that
    /// points outside the package.
    #[error(transparent)]
    Deck(#[from] PptxError),

    /// A continuation carried something other than the four bytes this box model writes.
    ///
    /// Reachable only by handing this model a checkpoint whose signature matches and whose state
    /// does not, which is what the perturbation test does deliberately.
    #[error(
        "a continuation carried {0} bytes; this box model writes exactly four — the index of the \
         slide the next page lays out"
    )]
    MalformedContinuation(usize),
}
