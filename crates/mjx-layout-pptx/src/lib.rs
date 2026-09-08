//! PowerPoint's box model: the first implementation of [`mjx_layout::BoxModel`], and the first code
//! in this workspace that turns a real `.pptx` into a [`FragmentTree`](mjx_layout::FragmentTree).
//!
//! # What it does
//!
//! One slide is one page. A slide's shape tree is walked in z-order, every shape is placed at the
//! bounds `mjx-pptx` resolves for it — through the layout and the master, for a placeholder that
//! states none — and every text body is laid out inside its shape: the four insets, the columns, the
//! nine indent levels with their bullets, the line spacing, the anchor, and the autofit that ties
//! them together.
//!
//! ```no_run
//! use mjx_layout::{BoxModel, PageIndex};
//! use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
//! use mjx_pptx::Presentation;
//! use mjx_text::FontResolver;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut presentation = Presentation::open(&std::fs::read("deck.pptx")?)?;
//! let deck = SlideDeck::read(&mut presentation)?;
//! let constraints = constraints_for(&deck);
//!
//! let mut model = SlideBoxModel::new(FontResolver::builder().with_platform_fonts().build());
//! let page = model.layout_page(&deck, PageIndex::FIRST, &constraints, None)?;
//! println!("{} fragments on slide 1", page.fragments().len());
//! # Ok(())
//! # }
//! ```
//!
//! # What it consumes and never re-derives
//!
//! Everything about *what a shape says* comes from `mjx-pptx`, already resolved:
//!
//! | Question | Answered by |
//! |---|---|
//! | Where is this shape? | [`effective_shape_bounds`](mjx_pptx::Presentation::effective_shape_bounds) |
//! | How is it rotated? | [`effective_shape_transform`](mjx_pptx::Presentation::effective_shape_transform) |
//! | How does its text sit in it? | [`effective_body_properties`](mjx_pptx::Presentation::effective_body_properties) |
//! | What is this paragraph's bullet, indent, spacing? | [`effective_paragraph_properties`](mjx_pptx::Presentation::effective_paragraph_properties) |
//! | What face and size is this run? | [`effective_run_properties`](mjx_pptx::Presentation::effective_run_properties) |
//! | What paints the shape? | [`effective_shape_fill`](mjx_pptx::Presentation::effective_shape_fill) / [`effective_shape_outline`](mjx_pptx::Presentation::effective_shape_outline) |
//!
//! The seven-tier ladder, the placeholder slot matching, the master's `p:txStyles` and the theme
//! font substitution have all already run by the time anything here reads a value. That is the whole
//! reason this crate sits *above* the format tier rather than inside it, and
//! `tests/the_ladder_is_consumed.rs` holds it by grepping this crate's own source for the
//! identifiers a re-derivation would need.
//!
//! Every **measurement** comes from `mjx-text`: shaping, bidirectional resolution, script
//! itemisation, face fallback and line breaking. Nothing here measures a glyph.
//!
//! # ⚠ Nothing in this crate is parity with PowerPoint, and it is not described as such
//!
//! ECMA-376 says what the attributes are and is nearly silent on what a renderer does with them, so
//! a number of behaviours here are readings rather than facts. Every one of them is marked `GUESS:`
//! at the site that makes the choice, and they are collected in
//! [`crate::body`] and [`crate::autofit`]. The sharpest is autofit: **honouring** a stored
//! `a:normAutofit` scale reproduces exactly what the author saw, and **computing** one is running
//! PowerPoint's own search, which has never been specified. The two are kept apart by
//! [`AutofitOutcome::recomputed`], so a caller can always tell which it got.
//!
//! Confirmation is a human sitting against real Microsoft Office on Windows
//! (`docs/validation/07-the-reference-pack.md`). LibreOffice is a change detector and not a
//! reference.
//!
//! # What is here since R15, and what is still not
//!
//! MJXOFF-170 added the rest of PowerPoint's visual vocabulary: **tables** (the grid, rows that grow
//! to fit their text, merged and spanned cells, cell insets and anchoring, effective fills and
//! borders through the table style's six conditional bands), **effects** (a shape's effective
//! `a:effectLst`, carried on its decoration for the scene builder to translate), **pictures**
//! (a `p:pic` becomes an [`ImageFragment`](mjx_layout::ImageFragment) whose handle is shared by
//! relationship id, so a page that repeats a logo decodes it once), and **speaker notes**
//! ([`SlideBoxModel::layout_notes`], the same walk under a different part).
//!
//! Charts and SmartArt are R23 and animations are not in this loop at all. A graphic frame holding
//! one is laid out as the box it occupies, so it is hit-testable and takes up its room, and nothing
//! moves when it grows into real fragments.
//!
//! **Two things a picture does not carry, and neither is this crate's to fix.** `a:srcRect` and the
//! image adjustments (`a:duotone`, `a:clrChange`, `a:alphaModFix`, `a:lum`) are not modelled
//! anywhere in this workspace — `mjx-dml`'s `PictureFill` preserves them as opaque `RawNode`s and
//! exposes the relationship id and the tile/stretch mode alone — so a picture is laid out whole and
//! unadjusted. Consuming them needs them modelled first.
//!
//! **Shape geometry is a handle, deliberately.** A [`ShapeFragment`](mjx_layout::ShapeFragment)
//! carries a [`GeometryRef`](mjx_layout::GeometryRef) and the provider that turns it into an outline
//! lives above this crate; `mjx-geometry` is not a dependency and
//! `tests/the_seam_holds.rs` refuses the edge by name.

#![forbid(unsafe_code)]

pub mod address;
pub mod autofit;
pub mod body;
pub mod bullet;
pub mod deck;
pub mod error;
pub mod model;
pub mod table;
pub mod text;

pub use address::{TextHit, LAYOUTS, MASTERS, NOTES, SLIDES};
pub use autofit::{AutofitOutcome, AutofitPolicy};
pub use body::{PlacedBody, PlacedColumn, PlacedLine, PlacedMarker, PlacedPiece, VerticalLayout};
pub use bullet::{AutoNumberCounters, Marker};
pub use deck::{
    Cell, CellInsets, Paragraph, PictureContent, Run, Shape, ShapeContent, ShapeDecoration, Slide,
    SlideDeck, TableContent, TextBody, CELL_EDGES,
};
pub use error::SlideLayoutError;
pub use model::{
    constraints_for, Decoration, ImageRequest, PageCatalogue, ShapeOutlineRequest, SlideBoxModel,
};
pub use table::{PlacedCell, PlacedTable};
pub use text::{RunStyle, TabStops, TextEngine};
