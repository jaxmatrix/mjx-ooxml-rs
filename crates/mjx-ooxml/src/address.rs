//! [`Surface`] and [`ShapePath`] — *which part* and *which shape*, in the facade's own `u32`.
//!
//! # Why these are not `mjx-pptx`'s
//!
//! [`mjx_pptx::Surface`] and [`mjx_pptx::ShapePath`] carry `usize` payloads and convert from
//! `usize`, which is right for a Rust API and wrong for this one. The facade addresses everything in
//! `u32` so that a caller — and a binding generated over it — sees one width on every target. Adding
//! `From<u32>` alongside `From<usize>` down there is not an option: it would make every bare integer
//! literal in `deck.shape_fill(0, 2)` ambiguous across the whole workspace.
//!
//! So the facade owns its addressing vocabulary, and converts at the boundary. The conversion is
//! free for the common cases: a [`Surface`] is `Copy` and rebuilt by value, and a top-level
//! [`ShapePath`] — one index, no group descent — is stored inline and never allocates. Only a path
//! that descends into a group allocates, once, on its way down.
//!
//! # Reading them
//!
//! A bare index means a slide, and a bare index means a top-level shape, exactly as one layer down:
//!
//! ```
//! use mjx_ooxml::{ShapePath, Surface};
//!
//! let slide: Surface = 0.into();
//! assert_eq!(slide, Surface::Slide(0));
//!
//! let top: ShapePath = 2.into();          // the third top-level shape
//! let member: ShapePath = [2, 1].into();  // member 1 of the group at index 2
//! assert_eq!(top.indices(), [2]);
//! assert_eq!(member.indices(), [2, 1]);
//! assert_eq!(member.parent(), Some(top));
//! ```

use std::fmt;

use crate::index::{count, index};

/// The shape-bearing part a call is about: a slide, a slide layout, a slide master, a slide's notes
/// slide, or the single notes master.
///
/// All five carry the same `p:cSld > p:spTree`, so every shape method applies to each of them
/// equally. Editing a **layout or master** is how one change reaches many slides: a slide
/// placeholder that declares no property of its own inherits from the same-slot placeholder up its
/// chain, which is what the [`effective_*`](crate::Deck::effective_shape_fill) readers walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Surface {
    /// A slide, indexed as in [`Deck::slide_count`](crate::Deck::slide_count).
    Slide(u32),
    /// A slide layout, indexed as in [`Deck::layout_count`](crate::Deck::layout_count) — one flat
    /// space across every master.
    Layout(u32),
    /// A slide master, indexed as in [`Deck::master_count`](crate::Deck::master_count).
    Master(u32),
    /// The notes slide of the slide at this index (a notes slide belongs to exactly one slide).
    Notes(u32),
    /// The single notes master every notes slide inherits from.
    NotesMaster,
}

impl Surface {
    /// The index within this surface's own kind. The [`NotesMaster`](Surface::NotesMaster) is unique
    /// and reports `0`.
    #[must_use]
    pub fn index(self) -> u32 {
        match self {
            Self::Slide(idx) | Self::Layout(idx) | Self::Master(idx) | Self::Notes(idx) => idx,
            Self::NotesMaster => 0,
        }
    }

    /// The kind's name, as it appears in error messages: `slide`, `layout`, `master`, `notes`,
    /// `notes master`.
    #[must_use]
    pub fn kind_name(self) -> &'static str {
        self.to_model().kind_name()
    }

    /// Whether this surface stands at the head of its own inheritance chain — a slide master or the
    /// notes master, neither of which inherits from a further part.
    #[must_use]
    pub fn is_master_like(self) -> bool {
        matches!(self, Self::Master(_) | Self::NotesMaster)
    }

    /// The model's surface, for the delegated call.
    pub(crate) fn to_model(self) -> mjx_pptx::Surface {
        match self {
            Self::Slide(idx) => mjx_pptx::Surface::Slide(index(idx)),
            Self::Layout(idx) => mjx_pptx::Surface::Layout(index(idx)),
            Self::Master(idx) => mjx_pptx::Surface::Master(index(idx)),
            Self::Notes(idx) => mjx_pptx::Surface::Notes(index(idx)),
            Self::NotesMaster => mjx_pptx::Surface::NotesMaster,
        }
    }
}

impl From<u32> for Surface {
    /// A bare index means a slide — the common case.
    fn from(value: u32) -> Self {
        Self::Slide(value)
    }
}

impl From<mjx_pptx::Surface> for Surface {
    fn from(surface: mjx_pptx::Surface) -> Self {
        match surface {
            mjx_pptx::Surface::Slide(idx) => Self::Slide(count(idx)),
            mjx_pptx::Surface::Layout(idx) => Self::Layout(count(idx)),
            mjx_pptx::Surface::Master(idx) => Self::Master(count(idx)),
            mjx_pptx::Surface::Notes(idx) => Self::Notes(count(idx)),
            mjx_pptx::Surface::NotesMaster => Self::NotesMaster,
        }
    }
}

impl fmt::Display for Surface {
    /// `slide 0`, `layout 1`, `master 0`, `notes 2`, `notes master`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_model().fmt(f)
    }
}

/// The address of a shape within a surface's shape tree: a top-level index, then the indices to
/// descend through nested `p:grpSp` groups.
///
/// A surface's shapes share **one index space covering every kind** — autoshapes, pictures, groups,
/// graphic frames, connectors — in document order. A group is one entry on that space; its members
/// are reached by descending into it, as deep as the groups nest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapePath(Repr);

/// The storage behind a [`ShapePath`]. A top-level shape — the overwhelmingly common case — is kept
/// inline so it never allocates; anything else spills to a `Vec`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Repr {
    /// A single top-level index.
    Top(u32),
    /// Any other address: a group member (two or more indices), or the degenerate empty path.
    Nested(Vec<u32>),
}

impl ShapePath {
    /// The address as a slice of indices, outermost first — `[2]` for a top-level shape, `[2, 1]`
    /// for member `1` of the group at index `2`.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        match &self.0 {
            Repr::Top(idx) => std::slice::from_ref(idx),
            Repr::Nested(indices) => indices,
        }
    }

    /// How deep the address reaches: `1` for a top-level shape, `2` for a member of a top-level
    /// group, and so on. An empty (degenerate) path reports `0`.
    #[must_use]
    pub fn depth(&self) -> u32 {
        count(self.indices().len())
    }

    /// Whether this addresses a top-level shape — a single index, no group descent.
    #[must_use]
    pub fn is_top_level(&self) -> bool {
        self.depth() == 1
    }

    /// The address of member `index` of the group this addresses — one step deeper.
    #[must_use]
    pub fn child(&self, index: u32) -> Self {
        let mut indices = self.indices().to_vec();
        indices.push(index);
        Self::from(indices)
    }

    /// The address of the group this shape is a member of, or `None` for a top-level shape — the
    /// shape tree is not itself a shape.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let indices = self.indices();
        match indices.len() {
            0 | 1 => None,
            len => Some(Self::from(&indices[..len - 1])),
        }
    }

    /// The model's path, for the delegated call. Allocation-free for a top-level shape.
    pub(crate) fn to_model(&self) -> mjx_pptx::ShapePath {
        match &self.0 {
            Repr::Top(idx) => mjx_pptx::ShapePath::from(index(*idx)),
            Repr::Nested(indices) => {
                mjx_pptx::ShapePath::from(indices.iter().copied().map(index).collect::<Vec<_>>())
            }
        }
    }
}

impl From<u32> for ShapePath {
    /// A bare index is a top-level shape — the common case, allocation-free.
    fn from(value: u32) -> Self {
        Self(Repr::Top(value))
    }
}

impl From<&ShapePath> for ShapePath {
    /// A borrowed path forwards as an owned clone, so a caller holding one can pass it to a method
    /// that takes it by value more than once.
    fn from(path: &ShapePath) -> Self {
        path.clone()
    }
}

impl From<Vec<u32>> for ShapePath {
    fn from(indices: Vec<u32>) -> Self {
        match indices.as_slice() {
            [only] => Self(Repr::Top(*only)),
            _ => Self(Repr::Nested(indices)),
        }
    }
}

impl From<&[u32]> for ShapePath {
    fn from(indices: &[u32]) -> Self {
        match indices {
            [only] => Self(Repr::Top(*only)),
            _ => Self(Repr::Nested(indices.to_vec())),
        }
    }
}

impl<const N: usize> From<[u32; N]> for ShapePath {
    fn from(indices: [u32; N]) -> Self {
        Self::from(indices.as_slice())
    }
}

impl From<mjx_pptx::ShapePath> for ShapePath {
    fn from(path: mjx_pptx::ShapePath) -> Self {
        Self::from(
            path.indices()
                .iter()
                .copied()
                .map(count)
                .collect::<Vec<_>>(),
        )
    }
}

impl fmt::Display for ShapePath {
    /// A top-level shape shows as its bare index (`2`); a nested one as a bracketed path (`[2, 1]`),
    /// which is how an out-of-range error names what was asked for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.indices() {
            [only] => write!(f, "{only}"),
            indices => {
                f.write_str("[")?;
                for (position, idx) in indices.iter().enumerate() {
                    if position > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{idx}")?;
                }
                f.write_str("]")
            }
        }
    }
}

/// The model paths for a slice of facade paths — what the group methods forward.
pub(crate) fn to_model_paths(paths: &[ShapePath]) -> Vec<mjx_pptx::ShapePath> {
    paths.iter().map(ShapePath::to_model).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_index_is_a_slide_and_a_top_level_shape() {
        assert_eq!(Surface::from(3), Surface::Slide(3));
        assert_eq!(ShapePath::from(2).indices(), [2]);
        assert!(ShapePath::from(2).is_top_level());
    }

    /// Every kind must survive the trip through the model and back, or a delegate silently
    /// re-addresses a layout as a slide.
    #[test]
    fn surfaces_round_trip_through_the_model() {
        for surface in [
            Surface::Slide(2),
            Surface::Layout(1),
            Surface::Master(0),
            Surface::Notes(3),
            Surface::NotesMaster,
        ] {
            assert_eq!(Surface::from(surface.to_model()), surface);
            assert_eq!(surface.to_model().index(), surface.index() as usize);
        }
        assert_eq!(Surface::Layout(1).to_string(), "layout 1");
        assert_eq!(Surface::NotesMaster.to_string(), "notes master");
        assert!(Surface::Master(0).is_master_like());
        assert!(!Surface::Slide(0).is_master_like());
    }

    #[test]
    fn paths_round_trip_through_the_model() {
        for path in [
            ShapePath::from(2),
            ShapePath::from([2, 1]),
            ShapePath::from([4, 0, 3]),
        ] {
            assert_eq!(ShapePath::from(path.to_model()), path);
        }
        assert_eq!(ShapePath::from([2, 1]).to_string(), "[2, 1]");
        assert_eq!(ShapePath::from(2).to_string(), "2");
        assert_eq!(ShapePath::from(2).child(1), ShapePath::from([2, 1]));
        assert_eq!(ShapePath::from([2, 1]).parent(), Some(ShapePath::from(2)));
        assert_eq!(ShapePath::from(2).parent(), None);
        assert_eq!(ShapePath::from([4, 0, 3]).depth(), 3);
    }
}
