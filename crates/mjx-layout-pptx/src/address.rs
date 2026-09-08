//! The addressing scheme — how a fragment says which shape, which paragraph and which characters it
//! came from, and how a point on a page turns back into that.
//!
//! # This scheme is not new, and that matters more than its details
//!
//! A [`SourceRef`] means nothing without the box model that issued it, so a box model that invents
//! its own numbering has made every consumer above it depend on *this* crate. `mjx-session`
//! (MJXOFF-167) already wrote a presentation's addressing down — `PresentationSession`'s doc comment
//! is the contract — and it did so before any box model existed, because the *editing* side needed
//! it first. So this crate adopts that scheme exactly rather than defining a second one:
//!
//! | Piece | Meaning |
//! |---|---|
//! | [`PartId`] `0` | slides · `1` layouts · `2` masters · `3` notes slides |
//! | path segment `0` | which surface of that kind |
//! | path segments `1 ..` | the shape path — one index per top-level shape, more to descend into `p:grpSp` groups |
//! | the last **two** segments, for text | the paragraph index and the run index inside the shape |
//!
//! The consequence is the point: a `SetBounds` operation journalled by `mjx-session` and a fragment
//! laid out here name the same node, so [`BoxModel::invalidate`](mjx_layout::BoxModel::invalidate)
//! can be a comparison rather than a translation. The two crates never depend on each other — 3.5
//! and 3.6 are sideways — and they agree because the scheme is written down in both.
//!
//! `crates/mjx-layout-pptx/tests/the_addressing_agrees_with_the_session.rs` is what keeps that true:
//! it reconstructs a `mjx-session` address from a laid-out fragment and asserts the segments match.
//!
//! # Which characters a range counts
//!
//! [`SourceRef`]'s own rule is that the range is *"in the text of whatever the path addresses"*, and
//! this crate follows it exactly:
//!
//! * a **line** fragment's path names the paragraph, so its range is in the paragraph's own text —
//!   the concatenation of its runs, which is what [`Presentation::paragraph_text`] answers;
//! * a **glyph run** fragment's path names the run, so its range is in that run's own text.
//!
//! A hit test therefore lands on a run and an offset inside it, which is the pair
//! `mjx-pptx`'s text API takes.
//!
//! [`Presentation::paragraph_text`]: mjx_pptx::Presentation::paragraph_text

use mjx_layout::{PartId, SourcePath, SourceRef};
use mjx_pptx::Surface;

/// Slides — the surface kind a page index means.
pub const SLIDES: PartId = PartId::new(0);
/// Slide layouts.
pub const LAYOUTS: PartId = PartId::new(1);
/// Slide masters.
pub const MASTERS: PartId = PartId::new(2);
/// Notes slides.
pub const NOTES: PartId = PartId::new(3);

/// The part a surface's fragments are addressed under, or `None` for the notes master.
///
/// `Surface::NotesMaster` is the one surface with no number of its own — there is exactly one of it
/// — and rather than invent a fifth part for a surface this box model never lays out, it is refused.
#[must_use]
pub fn part_of(surface: Surface) -> Option<PartId> {
    match surface {
        Surface::Slide(_) => Some(SLIDES),
        Surface::Layout(_) => Some(LAYOUTS),
        Surface::Master(_) => Some(MASTERS),
        Surface::Notes(_) => Some(NOTES),
        Surface::NotesMaster => None,
    }
}

/// The surface a part number and a first path segment name, or `None` for a part this scheme does
/// not number.
#[must_use]
pub fn surface_of(part: PartId, index: u32) -> Option<Surface> {
    let index = index as usize;
    match part.number() {
        0 => Some(Surface::Slide(index)),
        1 => Some(Surface::Layout(index)),
        2 => Some(Surface::Master(index)),
        3 => Some(Surface::Notes(index)),
        _ => None,
    }
}

/// A shape's address: the surface index, then its path through the shape tree.
#[must_use]
pub fn shape_path(surface_index: u32, shape: &[u32]) -> SourcePath {
    let mut segments = Vec::with_capacity(shape.len() + 1);
    segments.push(surface_index);
    segments.extend_from_slice(shape);
    SourcePath::new(&segments)
}

/// A paragraph's address: the shape's, plus which paragraph.
#[must_use]
pub fn paragraph_path(surface_index: u32, shape: &[u32], paragraph: usize) -> SourcePath {
    shape_path(surface_index, shape).child(clamp(paragraph))
}

/// A run's address: the paragraph's, plus which run inside it.
#[must_use]
pub fn run_path(surface_index: u32, shape: &[u32], paragraph: usize, run: usize) -> SourcePath {
    paragraph_path(surface_index, shape, paragraph).child(clamp(run))
}

/// A whole node, covering no characters — a page, a shape, a paragraph box.
#[must_use]
pub fn node(part: PartId, path: SourcePath) -> SourceRef {
    SourceRef::node(part, path)
}

/// An address covering `bytes` of the node at `path`.
#[must_use]
pub fn span(part: PartId, path: SourcePath, bytes: std::ops::Range<usize>) -> SourceRef {
    SourceRef::new(part, path, clamp(bytes.start)..clamp(bytes.end))
}

/// A `usize` from a document as the `u32` a path segment is, saturating rather than wrapping.
///
/// A wrap would make two different shapes share an address, which is worse than a shape at
/// `u32::MAX` in a document that cannot exist.
fn clamp(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// What a hit test found: which shape, and — when the point landed on text — which paragraph, run
/// and byte offset inside that run.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TextHit {
    /// Which surface kind.
    pub part: PartId,
    /// Which surface of that kind.
    pub surface_index: u32,
    /// The shape's path through the shape tree, from the top level down.
    pub shape: Vec<u32>,
    /// Which paragraph of that shape's text body, when the point landed on text.
    pub paragraph: Option<usize>,
    /// Which run of that paragraph, when the point landed on a glyph run.
    pub run: Option<usize>,
    /// The byte offset inside that run's own text — where a caret would go.
    pub offset: Option<usize>,
}

impl TextHit {
    /// The address split out of a fragment's [`SourceRef`], given how deep the shape path is.
    ///
    /// `shape_depth` is how many segments after the surface index belong to the shape, which the
    /// caller knows because it laid the shape out. Reading it from the path alone is impossible: a
    /// three-segment path is *[slide, shape, paragraph]* for a top-level shape and
    /// *[slide, group, shape]* for a grouped one, and nothing in the path says which.
    #[must_use]
    pub fn from_source(source: &SourceRef, shape_depth: usize) -> Option<Self> {
        let segments = source.path().segments();
        let (&surface_index, rest) = segments.split_first()?;
        if rest.len() < shape_depth {
            return None;
        }
        let (shape, text) = rest.split_at(shape_depth);
        let paragraph = text.first().map(|&index| index as usize);
        let run = text.get(1).map(|&index| index as usize);
        Some(Self {
            part: source.part(),
            surface_index,
            shape: shape.to_vec(),
            paragraph,
            run,
            offset: run.map(|_| source.characters().start as usize),
        })
    }

    /// The shape path as `mjx-pptx`'s own [`ShapePath`](mjx_pptx::ShapePath) takes it.
    #[must_use]
    pub fn shape_indices(&self) -> Vec<usize> {
        self.shape.iter().map(|&index| index as usize).collect()
    }
}
