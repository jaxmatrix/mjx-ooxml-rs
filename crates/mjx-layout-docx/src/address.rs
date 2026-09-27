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
//! # Five parts, not one
//!
//! MJXOFF-174 had one content stream and used [`PartId::PRIMARY`] for all of it. A document has
//! five kinds, and they are separated by [`PartId`] rather than by a sixth path segment for a reason
//! a selection makes obvious: **a header's third paragraph and the body's third paragraph are not
//! the same place, and a path that gave them the same segments would sort them together.** A hit
//! test in a footnote would report a caret in the body, and dragging a selection across a page
//! break would sweep the header's text into it.
//!
//! `PartId` is a number a box model assigns — `mjx-layout` has never heard of a package — so the
//! numbering is this crate's, and it is stated here rather than left implicit:
//!
//! | Part | Id | Path |
//! |---|---|---|
//! | `word/document.xml` | [`BODY`] | `[paragraph, …]` |
//! | a header | [`HEADER`] | `[stream, paragraph, …]` |
//! | a footer | [`FOOTER`] | `[stream, paragraph, …]` |
//! | `word/footnotes.xml` | [`FOOTNOTES`] | `[note, paragraph, …]` |
//! | `word/endnotes.xml` | [`ENDNOTES`] | `[note, paragraph, …]` |
//!
//! Headers and footers carry the **stream index** as their first segment because a document has
//! several and a page shows one of each: two pages showing different headers must not produce
//! fragments that claim to be the same place.
//!
//! # A generated mark has an address too
//!
//! A footnote's number, a line number in the margin and a tab leader are glyphs the *document does
//! not contain*, and every one of them still needs an address — a hit test that could not name them
//! would report a caret inside a paragraph that has no such character. Each takes the address of the
//! line it belongs to, so it sorts into reading order with everything else on it.

use mjx_layout::{PartId, SourcePath, SourceRef};

/// `word/document.xml` — the body.
pub const BODY: PartId = PartId::PRIMARY;

/// A header part. The stream index is the path's first segment.
pub const HEADER: PartId = PartId::new(1);

/// A footer part.
pub const FOOTER: PartId = PartId::new(2);

/// `word/footnotes.xml`.
pub const FOOTNOTES: PartId = PartId::new(3);

/// `word/endnotes.xml`.
pub const ENDNOTES: PartId = PartId::new(4);

/// The address of the document itself.
#[must_use]
pub fn root() -> SourceRef {
    SourceRef::node(BODY, SourcePath::root())
}

/// The root of one page's header or footer band.
#[must_use]
pub fn furniture_root(part: PartId, stream: usize) -> SourceRef {
    SourceRef::node(part, SourcePath::new(&[clamp(stream)]))
}

/// The root of a page's note area.
#[must_use]
pub fn notes_root(part: PartId) -> SourceRef {
    SourceRef::node(part, SourcePath::root())
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

/// The address of a paragraph in a secondary stream: a header, a footer, a footnote or an endnote.
///
/// `container` is the stream index for a header or footer and the note index for a note, which is
/// the same shape for the same reason — a document has several of each and a page shows some of
/// them.
#[must_use]
pub fn stream_paragraph(part: PartId, container: usize, index: usize) -> SourceRef {
    SourceRef::node(part, SourcePath::new(&[clamp(container), clamp(index)]))
}

/// One line of such a paragraph.
#[must_use]
pub fn stream_line(
    part: PartId,
    container: usize,
    index: usize,
    line: usize,
    characters: std::ops::Range<usize>,
) -> SourceRef {
    SourceRef::new(
        part,
        SourcePath::new(&[clamp(container), clamp(index), clamp(line)]),
        clamp(characters.start)..clamp(characters.end),
    )
}

/// One shaped run on such a line.
#[must_use]
pub fn stream_segment(
    part: PartId,
    container: usize,
    index: usize,
    line: usize,
    segment: usize,
    characters: std::ops::Range<usize>,
) -> SourceRef {
    SourceRef::new(
        part,
        SourcePath::new(&[clamp(container), clamp(index), clamp(line), clamp(segment)]),
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
