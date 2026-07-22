//! [`ShapePath`] — the address of a shape on a surface, top-level or nested inside a group.
//!
//! A slide's shapes share one index space (see [`Presentation::shape_count`](crate::Presentation::shape_count)),
//! but a `p:grpSp` is one entry on it whose members were, until now, unreachable. A `ShapePath` turns
//! that index space into a *path*: a top-level index, optionally followed by the indices to descend
//! through nested groups.
//!
//! It mirrors [`Surface`](crate::Surface): a bare `usize` converts to a top-level address, so
//! `deck.shape_fill(0, 2)` addresses the third top-level shape exactly as before, while
//! `deck.shape_fill(0, [2, 1])` addresses member `1` of the group at top-level index `2`, and a
//! longer array descends through nested groups. The common case reads as if the type were not there.

use std::fmt;

/// The address of a shape within a surface's shape tree: a top-level index, then the indices to
/// descend through nested `p:grpSp` groups.
///
/// Construct one from a bare index for a top-level shape, or from an array / slice / `Vec` of indices
/// for a group member:
///
/// ```
/// use mjx_pptx::ShapePath;
/// let top: ShapePath = 2.into(); // the third top-level shape
/// let member: ShapePath = [2, 1].into(); // member 1 of the group at index 2
/// assert_eq!(top.indices(), [2]);
/// assert_eq!(member.indices(), [2, 1]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapePath(Repr);

/// The storage behind a [`ShapePath`]. A top-level shape — the overwhelmingly common case — is stored
/// inline so it never allocates; anything else spills to a `Vec`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Repr {
    /// A single top-level index.
    Top(usize),
    /// Any other address: a group member (two or more indices), or the degenerate empty path.
    Nested(Vec<usize>),
}

impl ShapePath {
    /// The address as a slice of indices, outermost first — `[2]` for a top-level shape, `[2, 1]` for
    /// member `1` of the group at index `2`.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        match &self.0 {
            Repr::Top(index) => std::slice::from_ref(index),
            Repr::Nested(indices) => indices,
        }
    }

    /// How deep the address reaches: `1` for a top-level shape, `2` for a member of a top-level group,
    /// and so on. An empty (degenerate) path reports `0`.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.indices().len()
    }

    /// Whether this addresses a top-level shape — a single index, no group descent.
    #[must_use]
    pub fn is_top_level(&self) -> bool {
        self.depth() == 1
    }
}

impl From<usize> for ShapePath {
    /// A bare index is a top-level shape — the common case, allocation-free.
    fn from(index: usize) -> Self {
        Self(Repr::Top(index))
    }
}

impl From<&ShapePath> for ShapePath {
    /// A borrowed path forwards as an owned clone, so a method holding a `&ShapePath` can pass it to
    /// any `impl Into<ShapePath>` parameter (a shape is addressed more than once when, say, its old
    /// hyperlink must be read before the new one is written).
    fn from(path: &ShapePath) -> Self {
        path.clone()
    }
}

impl From<Vec<usize>> for ShapePath {
    fn from(indices: Vec<usize>) -> Self {
        match indices.as_slice() {
            [only] => Self(Repr::Top(*only)),
            _ => Self(Repr::Nested(indices)),
        }
    }
}

impl From<&[usize]> for ShapePath {
    fn from(indices: &[usize]) -> Self {
        match indices {
            [only] => Self(Repr::Top(*only)),
            _ => Self(Repr::Nested(indices.to_vec())),
        }
    }
}

impl<const N: usize> From<[usize; N]> for ShapePath {
    fn from(indices: [usize; N]) -> Self {
        match indices.as_slice() {
            [only] => Self(Repr::Top(*only)),
            _ => Self(Repr::Nested(indices.to_vec())),
        }
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
                for (position, index) in indices.iter().enumerate() {
                    if position > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{index}")?;
                }
                f.write_str("]")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_index_is_a_top_level_shape() {
        let path: ShapePath = 3.into();
        assert_eq!(path.indices(), [3]);
        assert_eq!(path.depth(), 1);
        assert!(path.is_top_level());
    }

    #[test]
    fn an_array_addresses_a_group_member() {
        let path: ShapePath = [2, 1].into();
        assert_eq!(path.indices(), [2, 1]);
        assert_eq!(path.depth(), 2);
        assert!(!path.is_top_level());
    }

    #[test]
    fn every_constructor_normalizes_a_single_index_to_top_level() {
        // A one-element array, slice, or Vec is the same address as a bare index — and stored inline.
        let from_array: ShapePath = [7].into();
        let from_slice: ShapePath = [7_usize].as_slice().into();
        let from_vec: ShapePath = vec![7].into();
        let from_index: ShapePath = 7.into();
        assert_eq!(from_array, from_index);
        assert_eq!(from_slice, from_index);
        assert_eq!(from_vec, from_index);
        assert!(matches!(from_array.0, Repr::Top(7)));
        assert!(matches!(from_vec.0, Repr::Top(7)));
    }

    #[test]
    fn nested_paths_keep_their_full_depth() {
        let deep: ShapePath = [1, 0, 4].into();
        assert_eq!(deep.indices(), [1, 0, 4]);
        assert_eq!(deep.depth(), 3);
    }

    #[test]
    fn an_empty_path_is_degenerate() {
        let empty: ShapePath = [].into();
        assert_eq!(empty.indices(), [] as [usize; 0]);
        assert_eq!(empty.depth(), 0);
        assert!(!empty.is_top_level());
    }

    #[test]
    fn display_names_top_level_bare_and_nested_bracketed() {
        assert_eq!(ShapePath::from(2).to_string(), "2");
        assert_eq!(ShapePath::from([2, 1]).to_string(), "[2, 1]");
        assert_eq!(ShapePath::from([1, 0, 4]).to_string(), "[1, 0, 4]");
    }
}
