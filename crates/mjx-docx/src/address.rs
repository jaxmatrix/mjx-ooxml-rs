//! [`BlockPath`] and [`RunPath`] — the addresses of a paragraph and of a run.
//!
//! Follows `mjx-pptx`'s [`ShapePath`](https://docs.rs/mjx-pptx) vocabulary (see
//! `crates/mjx-pptx/src/address.rs`), but for a different kind of nesting. A `p:grpSp` nests
//! *shapes*: the group is itself a shape, and its members are addressed by descending one more
//! index. WordprocessingML has no such thing for paragraphs — what nests a paragraph one level
//! deeper is a **block container** (a table cell, once `w:tbl` is modeled), not another paragraph —
//! and what nests a run one level deeper is a **run container** ([`Body::hyperlink`]'s own wrapper,
//! `w:hyperlink`, today; `w:sdt`, `w:customXml`, `w:smartTag`, `w:dir` and `w:bdo` are already
//! recognized members of the same wire group, `EG_ContentRunContent`, but this child leaves their
//! own content opaque — see [`ParagraphContent`](crate::document::ParagraphContent)'s doc comment).
//!
//! Both types share [`ShapePath`]'s manners deliberately: a bare index is the common case and costs
//! nothing (stored inline, no allocation), an array/slice/`Vec` addresses a nested member, and
//! `child`/`parent` walk one level at a time. A caller who knows `mjx-pptx`'s addressing already
//! knows this one.

use std::fmt;

/// The address of a paragraph (or, once `w:tbl` is modeled, a table) within a block container's
/// content: a top-level index, then the indices to descend through nested block containers (a table
/// cell holds its own block-level content — `EG_ContentBlockContent` again, one level down).
///
/// No fixture in this workspace can construct a depth-2 `BlockPath` today, because `w:tbl`'s content
/// is `mjx_docx`'s own [`Unmodeled`](crate::document::Unmodeled) until MJXOFF-116 types the table
/// structure that would produce one — but the type is shaped to take that member without a breaking
/// change to any signature that already accepts `impl Into<BlockPath>`, exactly as `ShapePath` was
/// shaped for group nesting before `mjx-pptx` modeled `p:grpSp`. `depth`/`child`/`parent` are pure
/// index arithmetic, independent of any tree, and are tested as such below.
///
/// Construct one from a bare index for a top-level paragraph, or from an array / slice / `Vec` of
/// indices once a nested block container exists to address:
///
/// ```
/// use mjx_docx::BlockPath;
/// let top: BlockPath = 1.into(); // the second top-level paragraph
/// assert_eq!(top.indices(), [1]);
/// assert!(top.is_top_level());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPath(BlockRepr);

/// The storage behind a [`BlockPath`]. A top-level paragraph — the overwhelmingly common case — is
/// stored inline so it never allocates; anything else spills to a `Vec`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockRepr {
    /// A single top-level index.
    Top(usize),
    /// Any other address: a nested block container's member (two or more indices), or the
    /// degenerate empty path.
    Nested(Vec<usize>),
}

impl BlockPath {
    /// The address as a slice of indices, outermost first.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        match &self.0 {
            BlockRepr::Top(index) => std::slice::from_ref(index),
            BlockRepr::Nested(indices) => indices,
        }
    }

    /// How deep the address reaches: `1` for a top-level paragraph, `2` for a member of a top-level
    /// block container, and so on. An empty (degenerate) path reports `0`.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.indices().len()
    }

    /// Whether this addresses a top-level paragraph — a single index, no container descent.
    #[must_use]
    pub fn is_top_level(&self) -> bool {
        self.depth() == 1
    }

    /// The address of member `index` of the block container this addresses — one step deeper.
    #[must_use]
    pub fn child(&self, index: usize) -> Self {
        let mut indices = self.indices().to_vec();
        indices.push(index);
        Self::from(indices)
    }

    /// The address of the block container this paragraph is a member of, or `None` for a top-level
    /// paragraph — the body itself is not addressed by a `BlockPath`.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let indices = self.indices();
        match indices.len() {
            0 | 1 => None,
            len => Some(Self::from(&indices[..len - 1])),
        }
    }
}

impl From<usize> for BlockPath {
    /// A bare index is a top-level paragraph — the common case, allocation-free.
    fn from(index: usize) -> Self {
        Self(BlockRepr::Top(index))
    }
}

impl From<&BlockPath> for BlockPath {
    fn from(path: &BlockPath) -> Self {
        path.clone()
    }
}

impl From<Vec<usize>> for BlockPath {
    fn from(indices: Vec<usize>) -> Self {
        match indices.as_slice() {
            [only] => Self(BlockRepr::Top(*only)),
            _ => Self(BlockRepr::Nested(indices)),
        }
    }
}

impl From<&[usize]> for BlockPath {
    fn from(indices: &[usize]) -> Self {
        match indices {
            [only] => Self(BlockRepr::Top(*only)),
            _ => Self(BlockRepr::Nested(indices.to_vec())),
        }
    }
}

impl<const N: usize> From<[usize; N]> for BlockPath {
    fn from(indices: [usize; N]) -> Self {
        match indices.as_slice() {
            [only] => Self(BlockRepr::Top(*only)),
            _ => Self(BlockRepr::Nested(indices.to_vec())),
        }
    }
}

impl fmt::Display for BlockPath {
    /// A top-level paragraph shows as its bare index (`1`); a nested one as a bracketed path
    /// (`[1, 0]`), which is how an out-of-range error names what was asked for.
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

/// The address of a run within one paragraph's content: a top-level index, then the indices to
/// descend through nested run containers (today, only `w:hyperlink` recurses into
/// [`ParagraphContent`](crate::document::ParagraphContent) — see that type's doc comment for why
/// `w:sdt`/`w:customXml`/`w:smartTag`/`w:dir`/`w:bdo` do not, yet).
///
/// Unlike [`BlockPath`], nesting here is not speculative: a run inside a hyperlink is exactly the
/// "runs out of an order a sloppy walker would produce" case MJXOFF-92 was told to seed a fixture
/// for, and `Document::run_text`/`Document::set_run_text` genuinely resolve a depth-2 `RunPath` today.
///
/// ```
/// use mjx_docx::RunPath;
/// let top: RunPath = 0.into(); // the paragraph's first run
/// let nested: RunPath = [2, 0].into(); // the first run inside the third top-level item
/// assert_eq!(top.indices(), [0]);
/// assert_eq!(nested.indices(), [2, 0]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPath(RunRepr);

/// The storage behind a [`RunPath`] — see [`BlockRepr`], the same shape for a different kind of
/// container.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RunRepr {
    /// A single top-level index.
    Top(usize),
    /// Any other address: a run container's member (two or more indices), or the degenerate empty
    /// path.
    Nested(Vec<usize>),
}

impl RunPath {
    /// The address as a slice of indices, outermost first.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        match &self.0 {
            RunRepr::Top(index) => std::slice::from_ref(index),
            RunRepr::Nested(indices) => indices,
        }
    }

    /// How deep the address reaches: `1` for a run directly in the paragraph, `2` for a run inside
    /// one run container (a hyperlink), and so on. An empty (degenerate) path reports `0`.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.indices().len()
    }

    /// Whether this addresses a run directly in the paragraph — a single index, no container
    /// descent.
    #[must_use]
    pub fn is_top_level(&self) -> bool {
        self.depth() == 1
    }

    /// The address of member `index` of the run container this addresses — one step deeper.
    #[must_use]
    pub fn child(&self, index: usize) -> Self {
        let mut indices = self.indices().to_vec();
        indices.push(index);
        Self::from(indices)
    }

    /// The address of the run container this run is a member of, or `None` for a run directly in
    /// the paragraph.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let indices = self.indices();
        match indices.len() {
            0 | 1 => None,
            len => Some(Self::from(&indices[..len - 1])),
        }
    }
}

impl From<usize> for RunPath {
    /// A bare index is a run directly in the paragraph — the common case, allocation-free.
    fn from(index: usize) -> Self {
        Self(RunRepr::Top(index))
    }
}

impl From<&RunPath> for RunPath {
    fn from(path: &RunPath) -> Self {
        path.clone()
    }
}

impl From<Vec<usize>> for RunPath {
    fn from(indices: Vec<usize>) -> Self {
        match indices.as_slice() {
            [only] => Self(RunRepr::Top(*only)),
            _ => Self(RunRepr::Nested(indices)),
        }
    }
}

impl From<&[usize]> for RunPath {
    fn from(indices: &[usize]) -> Self {
        match indices {
            [only] => Self(RunRepr::Top(*only)),
            _ => Self(RunRepr::Nested(indices.to_vec())),
        }
    }
}

impl<const N: usize> From<[usize; N]> for RunPath {
    fn from(indices: [usize; N]) -> Self {
        match indices.as_slice() {
            [only] => Self(RunRepr::Top(*only)),
            _ => Self(RunRepr::Nested(indices.to_vec())),
        }
    }
}

impl fmt::Display for RunPath {
    /// A top-level run shows as its bare index (`0`); a nested one as a bracketed path (`[2, 0]`).
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
    fn a_bare_index_is_a_top_level_block() {
        let path: BlockPath = 3.into();
        assert_eq!(path.indices(), [3]);
        assert_eq!(path.depth(), 1);
        assert!(path.is_top_level());
    }

    #[test]
    fn an_array_addresses_a_nested_block_member() {
        let path: BlockPath = [2, 1].into();
        assert_eq!(path.indices(), [2, 1]);
        assert_eq!(path.depth(), 2);
        assert!(!path.is_top_level());
    }

    #[test]
    fn every_block_path_constructor_normalizes_a_single_index_to_top_level() {
        let from_array: BlockPath = [7].into();
        let from_slice: BlockPath = [7_usize].as_slice().into();
        let from_vec: BlockPath = vec![7].into();
        let from_index: BlockPath = 7.into();
        assert_eq!(from_array, from_index);
        assert_eq!(from_slice, from_index);
        assert_eq!(from_vec, from_index);
        assert!(matches!(from_array.0, BlockRepr::Top(7)));
    }

    #[test]
    fn block_path_child_and_parent_are_inverses() {
        let top: BlockPath = 2.into();
        let member = top.child(1);
        assert_eq!(member.indices(), [2, 1]);
        assert_eq!(member.parent(), Some(top));
    }

    #[test]
    fn a_top_level_block_path_has_no_parent() {
        let top: BlockPath = 4.into();
        assert_eq!(top.parent(), None);
    }

    #[test]
    fn block_path_display_names_top_level_bare_and_nested_bracketed() {
        assert_eq!(BlockPath::from(2).to_string(), "2");
        assert_eq!(BlockPath::from([2, 1]).to_string(), "[2, 1]");
    }

    #[test]
    fn a_bare_index_is_a_top_level_run() {
        let path: RunPath = 0.into();
        assert_eq!(path.indices(), [0]);
        assert!(path.is_top_level());
    }

    #[test]
    fn an_array_addresses_a_run_inside_a_run_container() {
        let path: RunPath = [2, 0].into();
        assert_eq!(path.indices(), [2, 0]);
        assert_eq!(path.depth(), 2);
        assert!(!path.is_top_level());
    }

    #[test]
    fn run_path_child_and_parent_are_inverses() {
        let top: RunPath = 3.into();
        let nested = top.child(0);
        assert_eq!(nested.indices(), [3, 0]);
        assert_eq!(nested.parent(), Some(top));
    }

    #[test]
    fn run_path_display_names_top_level_bare_and_nested_bracketed() {
        assert_eq!(RunPath::from(0).to_string(), "0");
        assert_eq!(RunPath::from([2, 0]).to_string(), "[2, 0]");
    }
}
