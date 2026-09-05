//! [`BlockPath`] and [`RunPath`] — the facade's own `u32` addressing for a paragraph and a run,
//! mirroring [`crate::ShapePath`]'s own manners exactly (see `crate::address`'s module doc) but over
//! [`mjx_docx::BlockPath`]/[`mjx_docx::RunPath`]'s `usize` nesting instead of a shape group's.
//!
//! # Why these are not `mjx-docx`'s
//!
//! Same reasoning as [`crate::ShapePath`] versus `mjx_pptx::ShapePath`: the model's own path types
//! carry `usize` and convert from `usize`, which is right for a Rust API and wrong for a binding —
//! `u32` is the one width every target (32- and 64-bit desktop, `wasm32`) agrees on. The facade owns
//! its own addressing vocabulary and converts at the boundary; a bare index is the common case and
//! costs nothing (stored inline), and only a path descending into a container (a run inside a
//! `w:hyperlink`, a paragraph inside a table cell once one exists) allocates, once, on its way down.
//!
//! ```
//! use mjx_ooxml::{BlockPath, RunPath};
//!
//! let paragraph: BlockPath = 1.into(); // the second top-level paragraph
//! let run: RunPath = 0.into(); // its first run
//! assert_eq!(paragraph.indices(), [1]);
//! assert_eq!(run.indices(), [0]);
//! ```

use std::fmt;

use crate::index::{count, index};

/// The address of a paragraph within a block container's content: a top-level index, then the
/// indices to descend through nested block containers — see [`mjx_docx::BlockPath`]'s own doc
/// comment for what those are today.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockPath(Repr);

/// The address of a run within one paragraph's content: a top-level index, then the indices to
/// descend through nested run containers (a `w:hyperlink`, today) — see [`mjx_docx::RunPath`]'s own
/// doc comment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunPath(Repr);

/// The storage shared by both path kinds. A top-level address — the overwhelmingly common case — is
/// kept inline so it never allocates; anything else spills to a `Vec`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Repr {
    /// A single top-level index.
    Top(u32),
    /// Any other address: a nested container's member (two or more indices), or the degenerate
    /// empty path.
    Nested(Vec<u32>),
}

macro_rules! path_type {
    ($name:ident, $model:path) => {
        impl $name {
            /// The address as a slice of indices, outermost first.
            #[must_use]
            pub fn indices(&self) -> &[u32] {
                match &self.0 {
                    Repr::Top(idx) => std::slice::from_ref(idx),
                    Repr::Nested(indices) => indices,
                }
            }

            /// How deep the address reaches: `1` for a top-level address, `2` for a member of one
            /// container, and so on. An empty (degenerate) path reports `0`.
            #[must_use]
            pub fn depth(&self) -> u32 {
                count(self.indices().len())
            }

            /// Whether this addresses a top-level item — a single index, no container descent.
            #[must_use]
            pub fn is_top_level(&self) -> bool {
                self.depth() == 1
            }

            /// The address of member `index` of the container this addresses — one step deeper.
            #[must_use]
            pub fn child(&self, index: u32) -> Self {
                let mut indices = self.indices().to_vec();
                indices.push(index);
                Self::from(indices)
            }

            /// The address of the container this item is a member of, or `None` for a top-level
            /// item.
            #[must_use]
            pub fn parent(&self) -> Option<Self> {
                let indices = self.indices();
                match indices.len() {
                    0 | 1 => None,
                    len => Some(Self::from(&indices[..len - 1])),
                }
            }

            /// The model's path, for the delegated call. Allocation-free for a top-level address.
            pub(crate) fn to_model(&self) -> $model {
                match &self.0 {
                    Repr::Top(idx) => <$model>::from(index(*idx)),
                    Repr::Nested(indices) => {
                        <$model>::from(indices.iter().copied().map(index).collect::<Vec<_>>())
                    }
                }
            }
        }

        impl From<u32> for $name {
            /// A bare index is a top-level address — the common case, allocation-free.
            fn from(value: u32) -> Self {
                Self(Repr::Top(value))
            }
        }

        impl From<&$name> for $name {
            /// A borrowed path forwards as an owned clone, so a caller holding one can pass it to a
            /// method that takes it by value more than once.
            fn from(path: &$name) -> Self {
                path.clone()
            }
        }

        impl From<Vec<u32>> for $name {
            fn from(indices: Vec<u32>) -> Self {
                match indices.as_slice() {
                    [only] => Self(Repr::Top(*only)),
                    _ => Self(Repr::Nested(indices)),
                }
            }
        }

        impl From<&[u32]> for $name {
            fn from(indices: &[u32]) -> Self {
                match indices {
                    [only] => Self(Repr::Top(*only)),
                    _ => Self(Repr::Nested(indices.to_vec())),
                }
            }
        }

        impl<const N: usize> From<[u32; N]> for $name {
            fn from(indices: [u32; N]) -> Self {
                Self::from(indices.as_slice())
            }
        }

        impl fmt::Display for $name {
            /// A top-level address shows as its bare index (`2`); a nested one as a bracketed path
            /// (`[2, 1]`), matching [`crate::ShapePath`]'s own `Display`.
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
    };
}

path_type!(BlockPath, mjx_docx::BlockPath);
path_type!(RunPath, mjx_docx::RunPath);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_index_is_top_level_and_allocation_free() {
        let block = BlockPath::from(3);
        assert_eq!(block.indices(), [3]);
        assert!(block.is_top_level());
        let run = RunPath::from(0);
        assert_eq!(run.indices(), [0]);
        assert!(run.is_top_level());
    }

    #[test]
    fn child_and_parent_are_inverses() {
        let top = BlockPath::from(2);
        let member = top.child(1);
        assert_eq!(member.indices(), [2, 1]);
        assert_eq!(member.parent(), Some(top));
        assert_eq!(member.depth(), 2);
        assert!(!member.is_top_level());
    }

    #[test]
    fn paths_round_trip_through_the_model() {
        for path in [BlockPath::from(2), BlockPath::from([2, 1])] {
            let model = path.to_model();
            assert_eq!(model.indices().len(), path.indices().len());
        }
    }

    #[test]
    fn display_names_top_level_bare_and_nested_bracketed() {
        assert_eq!(BlockPath::from(2).to_string(), "2");
        assert_eq!(BlockPath::from([2, 1]).to_string(), "[2, 1]");
        assert_eq!(RunPath::from(0).to_string(), "0");
        assert_eq!(RunPath::from([2, 0]).to_string(), "[2, 0]");
    }
}
