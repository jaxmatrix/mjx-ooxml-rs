//! The box model contract: one vocabulary for saying where everything went, and the seam above
//! which nothing has heard of OOXML.
//!
//! # What this crate is
//!
//! It defines four things and implements none of them for any document format:
//!
//! * [`BoxModel`] — the trait a layout engine implements. Lay out one page, say how big the document
//!   is, say what an edit invalidated, say who you are.
//! * [`FragmentTree`] — what a box model produces. Boxes, lines, glyph runs, images, shapes and
//!   tables, each carrying a [`SourceRef`] back to the document address that made it.
//! * [`Checkpoint`] — the small continuation token that makes page 300 of a flowing document
//!   reachable without laying out the 299 before it.
//! * [`SpatialIndex`] — where everything on a page is, so a hit test is a query rather than a walk.
//!
//! # Why the seam is here and not somewhere else
//!
//! `docs/UI_PLATFORM_PLAN.md` §2 draws two horizontal cuts through the stack, and this is the lower
//! one. **Above a `FragmentTree`, nothing has heard of OOXML**: scene building, tessellation,
//! painting, hit-testing, caret placement, selection, comment anchoring, accessibility and every
//! exporter are written against these fragments. **Below it, a box model may do anything at all.**
//!
//! That is what makes the box model swappable, which was the requirement this whole architecture is
//! organised around: adopt a different box model — a CSS one, a Markdown one, a domain-specific one
//! — and every surface above keeps working, because none of them can tell what produced the
//! fragments they are drawing.
//!
//! The seam is checkable rather than asserted. This crate depends on `mjx-ooxml-core` and `mjx-text`
//! and on **no format crate, ever**; `xtask/tests/layering.rs` fails on the edge, and
//! `tests/the_seam_holds.rs` fails on the identifier. `mjx-ooxml-core` is here only for
//! [`Emu`](mjx_ooxml_core::measure::Emu) — a length, not a markup — and `mjx-text` is here because a
//! box model must measure text and this crate never re-implements measurement.
//!
//! # An abstraction with one implementation is a guess
//!
//! A contract validated only by the implementation that shaped it proves nothing: every corner it
//! got wrong is a corner that implementation happens not to use, and the obvious gate — *the trait
//! compiles and the tree round-trips* — is satisfied by an interface that is secretly OOXML-shaped.
//!
//! So this crate's own tests carry a **second box model** that shares no ancestry with the three
//! OOXML ones: it reflows plain text into a fixed-width column, produces a real `FragmentTree` with
//! real `SourceRef`s, paginates, resumes from a checkpoint and hit-tests. `tests/support/mod.rs`
//! records what writing it forced to change here, which is the only measurement of the contract's
//! shape that means anything.
//!
//! # Nothing here panics
//!
//! A pathological document produces a bad-looking page, never a crash. Every index is a `get`, every
//! arithmetic on a length saturates ([`Emu`](mjx_ooxml_core::measure::Emu)'s operators do it for
//! free), every division checks its divisor, and every failure a caller can provoke is a
//! [`LayoutError`] or a [`FontError`](mjx_text::FontError). There is no `unwrap`, `expect`, `panic!`
//! or slice index on any layout path, and `tests/no_panic_on_a_layout_path.rs` holds that true by
//! scanning the source recursively rather than by assertion.
//!
//! # Where the pieces meet
//!
//! ```text
//! document  →  BoxModel::layout_page  →  PageFragments
//!                                          ├── FragmentTree   (what and where)
//!                                          ├── SpatialIndex   (found quickly)
//!                                          └── Checkpoint     (how to lay out the next page)
//! ```

pub mod checkpoint;
pub mod error;
pub mod fragment;
pub mod index;
pub mod measure;
pub mod model;
pub mod source;
pub mod text;

pub use checkpoint::{Checkpoint, ModelSignature, MAXIMUM_CHECKPOINT_BYTES};
pub use error::LayoutError;
pub use fragment::{
    BoxFragment, ClipId, DecorationRef, Fragment, FragmentId, FragmentNode, FragmentTree,
    FragmentTreeBuilder, GeometryRef, GlyphRunFragment, ImageFragment, ImageRef, LineFragment,
    ShapeFragment, TableCell, TableFragment, TransformId, UnitRect,
};
pub use index::{SpatialIndex, MAXIMUM_CELL_SPAN, MAXIMUM_GRID_SIDE};
pub use measure::{LayoutPoint, LayoutRect, LayoutSize, Transform};
pub use model::{
    BoxModel, ChangeKind, ChangeSet, Constraints, ContentChange, DirtyPages, Extent,
    ExtentPrecision, PageFragments, PageIndex, WritingMode,
};
pub use source::{PartId, SourcePath, SourceRef, INLINE_PATH_DEPTH};
pub use text::{slice_width, ComposedLine, ComposedSegment, LineComposer, TextRun};
