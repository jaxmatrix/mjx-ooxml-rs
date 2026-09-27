//! [`SourceRef`] — the document address a fragment came from, and the one field that makes
//! everything above this layer possible.
//!
//! # What it is for
//!
//! A hit test answers *which pixel is this?* with a fragment. Every question a reader or an editor
//! actually asks — where does the caret go, what did I select, which paragraph is this comment
//! anchored to, what should a screen reader say next, which run does this edit change — is a
//! question about the **document**, not about the fragment. `SourceRef` is the link back, and it is
//! the reason the interaction layer, the exporters and the accessibility tree can be written once
//! against fragments instead of three times against `.pptx`, `.docx` and `.xlsx`.
//!
//! # Why it is opaque, and what that costs
//!
//! It is a part number, a path of small integers and a character range. **No OOXML type appears in
//! it**, and that is the seam: a box model that is not a `.docx` box model — an HTML one, a Markdown
//! one, the plain-text one this crate's own tests carry — assigns whatever numbering it likes and
//! every layer above works unchanged.
//!
//! The cost is that a `SourceRef` means nothing without the box model that issued it. That is the
//! correct division: only the box model knows that path `[3, 1, 0]` is the fourth slide's second
//! shape's first paragraph, and only it can turn a `SourceRef` back into a document node. The
//! contract is that the numbering is **stable for a given content revision**, so a checkpoint taken
//! on one page and a hit test on another agree about what path `[3, 1, 0]` is.
//!
//! # Why the path is inline up to six segments
//!
//! There is one `SourceRef` per fragment and hundreds of thousands of fragments in a long document,
//! so a `Vec<u32>` per fragment would be one allocation per glyph run. Six segments covers the deep
//! case in every format: a PowerPoint run is *slide → group → group → shape → paragraph → run*, and
//! a Word run is shallower. Deeper than six — a table nested inside a table inside a text box —
//! spills to a shared [`Arc`], so nothing is ever lost or truncated; it is a
//! representation choice, not a limit.

use std::sync::Arc;

/// How many path segments a [`SourcePath`] holds without allocating.
pub const INLINE_PATH_DEPTH: usize = 6;

/// Which part of the document a fragment came from.
///
/// A number the box model assigns, not a part name: this crate has never heard of a package. A box
/// model with one content stream uses [`PartId::PRIMARY`] for everything.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct PartId(u32);

impl PartId {
    /// The part a single-stream box model puts everything in.
    pub const PRIMARY: Self = Self(0);

    /// The part numbered `number`.
    #[must_use]
    pub const fn new(number: u32) -> Self {
        Self(number)
    }

    /// The number, for a diagnostic message or a lookup table.
    #[must_use]
    pub const fn number(self) -> u32 {
        self.0
    }
}

/// Where inside a part, as a path of child indices from the part's root.
///
/// Ordering is lexicographic on the segments, which is **document order** for any box model that
/// numbers children in the order they appear — so sorting fragments by their path sorts them the way
/// a reader reads them, and a range of paths is a range of the document.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SourcePath(PathStorage);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum PathStorage {
    /// Up to [`INLINE_PATH_DEPTH`] segments, held in the value itself.
    Inline {
        depth: u8,
        segments: [u32; INLINE_PATH_DEPTH],
    },
    /// Deeper than that. Shared, so cloning a fragment does not copy it.
    Deep(Arc<[u32]>),
}

impl SourcePath {
    /// The part's root — the empty path.
    #[must_use]
    pub const fn root() -> Self {
        Self(PathStorage::Inline {
            depth: 0,
            segments: [0; INLINE_PATH_DEPTH],
        })
    }

    /// The path made of `segments`.
    #[must_use]
    pub fn new(segments: &[u32]) -> Self {
        if segments.len() <= INLINE_PATH_DEPTH {
            let mut inline = [0_u32; INLINE_PATH_DEPTH];
            inline[..segments.len()].copy_from_slice(segments);
            // The length was just bounded by `INLINE_PATH_DEPTH`, which is far below `u8::MAX`.
            let depth = segments.len() as u8;
            return Self(PathStorage::Inline {
                depth,
                segments: inline,
            });
        }
        Self(PathStorage::Deep(Arc::from(segments)))
    }

    /// The segments, from the root outward.
    #[must_use]
    pub fn segments(&self) -> &[u32] {
        match &self.0 {
            PathStorage::Inline { depth, segments } => {
                let depth = usize::from(*depth).min(INLINE_PATH_DEPTH);
                &segments[..depth]
            }
            PathStorage::Deep(segments) => segments,
        }
    }

    /// How deep the path is.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.segments().len()
    }

    /// Whether this path is the root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments().is_empty()
    }

    /// This path with `segment` appended — the `segment`th child of whatever this addresses.
    #[must_use]
    pub fn child(&self, segment: u32) -> Self {
        let existing = self.segments();
        if existing.len() < INLINE_PATH_DEPTH {
            let mut inline = [0_u32; INLINE_PATH_DEPTH];
            inline[..existing.len()].copy_from_slice(existing);
            inline[existing.len()] = segment;
            // `existing.len()` is below `INLINE_PATH_DEPTH`, so the sum still fits a `u8`.
            let depth = (existing.len() + 1) as u8;
            return Self(PathStorage::Inline {
                depth,
                segments: inline,
            });
        }
        let mut deep = Vec::with_capacity(existing.len() + 1);
        deep.extend_from_slice(existing);
        deep.push(segment);
        Self(PathStorage::Deep(Arc::from(deep)))
    }

    /// The path this one is a child of, or `None` for the root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let segments = self.segments();
        let (_, head) = segments.split_last()?;
        Some(Self::new(head))
    }

    /// Whether `other` is this path or lies inside it — which is the test "is this fragment part of
    /// that paragraph?".
    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        let ours = self.segments();
        let theirs = other.segments();
        theirs.len() >= ours.len() && theirs.starts_with(ours)
    }
}

impl Default for SourcePath {
    fn default() -> Self {
        Self::root()
    }
}

impl PartialOrd for SourcePath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SourcePath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.segments().cmp(other.segments())
    }
}

/// The document address a fragment came from: a part, a path inside it, and the characters covered.
///
/// The character range is in **bytes**, in the text of whatever the path addresses, and is
/// `start..start` — an empty range — for a fragment that covers no text at all, such as a page
/// background or an image. It is a `u32` because it addresses one node's text, not a document's:
/// four gigabytes of characters in a single paragraph is not a document any box model will meet.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SourceRef {
    part: PartId,
    path: SourcePath,
    start: u32,
    end: u32,
}

impl SourceRef {
    /// An address covering `characters` of the node at `path` in `part`.
    ///
    /// A reversed range is normalised rather than refused; a range is a *description* of what a
    /// fragment covers, and refusing to build a fragment because two offsets arrived the wrong way
    /// round would lose text a reader can see.
    #[must_use]
    pub fn new(part: PartId, path: SourcePath, characters: std::ops::Range<u32>) -> Self {
        Self {
            part,
            path,
            start: characters.start.min(characters.end),
            end: characters.start.max(characters.end),
        }
    }

    /// An address covering no characters — a page, a shape, an image.
    #[must_use]
    pub fn node(part: PartId, path: SourcePath) -> Self {
        Self {
            part,
            path,
            start: 0,
            end: 0,
        }
    }

    /// Which part.
    #[must_use]
    pub const fn part(&self) -> PartId {
        self.part
    }

    /// Where inside it.
    #[must_use]
    pub const fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Which bytes of that node's text.
    #[must_use]
    pub const fn characters(&self) -> std::ops::Range<u32> {
        self.start..self.end
    }

    /// How many bytes it covers.
    #[must_use]
    pub const fn character_count(&self) -> u32 {
        self.end - self.start
    }

    /// Whether it covers no characters.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The same address with a different character range — how a caret at one offset is derived from
    /// the run that contains it.
    #[must_use]
    pub fn with_characters(&self, characters: std::ops::Range<u32>) -> Self {
        Self::new(self.part, self.path.clone(), characters)
    }

    /// Whether `other` addresses the same part and a path inside this one's, and — when both cover
    /// characters — an overlapping range.
    ///
    /// This is the question an invalidation asks: *does this change touch that fragment?*
    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        if self.part != other.part || !self.path.contains(&other.path) {
            return false;
        }
        if self.is_empty() || other.is_empty() || self.path != other.path {
            // A node-level address covers everything under it, whatever the character ranges say.
            return true;
        }
        other.start < self.end && self.start < other.end
    }
}

impl PartialOrd for SourceRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SourceRef {
    /// Document order: by part, then by path, then by where in the text.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.part
            .cmp(&other.part)
            .then_with(|| self.path.cmp(&other.path))
            .then_with(|| self.start.cmp(&other.start))
            .then_with(|| self.end.cmp(&other.end))
    }
}
