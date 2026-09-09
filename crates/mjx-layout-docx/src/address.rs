//! How a fragment says where in the document it came from.
//!
//! A [`SourcePath`] is a path of child indices and it orders **lexicographically**, which
//! `mjx-layout` documents as being document order for any box model that numbers children in the
//! order they appear. A flowing document is the case where that property earns its keep: sorting a
//! page's fragments by their address is reading order, and *which page is this paragraph on* is a
//! binary search over the checkpoints, because a checkpoint's own position is a [`SourceRef`] in the
//! same space.
//!
//! ```text
//! [paragraph]                    a paragraph's own box
//! [paragraph, line]              one line of it
//! [paragraph, line, segment]     one shaped run on that line
//! ```
//!
//! There is exactly one part: `word/document.xml`. Headers, footers and footnotes are separate
//! content streams and are MJXOFF-175 (R20)'s, and they will take [`PartId`]s of their own — which
//! is what a `PartId` is for and why the body's is named rather than assumed.

use mjx_layout::{PartId, SourcePath, SourceRef};

/// `word/document.xml` — the body, and in this child the only content stream there is.
pub const BODY: PartId = PartId::PRIMARY;

/// The address of the document itself.
#[must_use]
pub fn root() -> SourceRef {
    SourceRef::node(BODY, SourcePath::root())
}

/// The address of a paragraph.
#[must_use]
pub fn paragraph(index: usize) -> SourceRef {
    SourceRef::node(BODY, SourcePath::new(&[clamp(index)]))
}

/// The address of one line of a paragraph, covering `characters` of that paragraph's text.
#[must_use]
pub fn line(index: usize, line: usize, characters: std::ops::Range<usize>) -> SourceRef {
    SourceRef::new(
        BODY,
        SourcePath::new(&[clamp(index), clamp(line)]),
        clamp(characters.start)..clamp(characters.end),
    )
}

/// The address of one shaped run on one line.
#[must_use]
pub fn segment(
    index: usize,
    line: usize,
    segment: usize,
    characters: std::ops::Range<usize>,
) -> SourceRef {
    SourceRef::new(
        BODY,
        SourcePath::new(&[clamp(index), clamp(line), clamp(segment)]),
        clamp(characters.start)..clamp(characters.end),
    )
}

/// A `usize` as the `u32` a path segment is.
///
/// Saturating rather than truncating: a document with more than four billion paragraphs is not a
/// document, and a truncating cast would give two different paragraphs the same address, which is
/// worse than giving them both the last one.
fn clamp(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
