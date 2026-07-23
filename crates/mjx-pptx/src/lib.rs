//! `mjx-pptx` — PresentationML: presentation, slides, shape trees.
//!
//! The entry point is [`Presentation`]: open a `.pptx`'s container bytes, read the slides and the text
//! of their shapes, edit a run's text, and save. It owns an [`mjx_opc::Package`] and reuses the
//! DrawingML text model ([`mjx_dml::TextBody`]) for shape text; everything it does not model is
//! preserved verbatim by the OPC copy-on-write layer, so editing one run leaves every other part
//! byte-identical.
//!
//! # Addressing shapes
//!
//! A slide's shapes live in **one index space covering every [`ShapeKind`]** — autoshapes (`p:sp`),
//! pictures (`p:pic`), groups, graphic frames, connectors — in document order. Every shape API takes
//! an address as [`impl Into<ShapePath>`](ShapePath): a bare index for a top-level shape, so
//! `deck.shape_fill(0, 2)` reads the third shape, and an array `[2, 1]` to descend into a group —
//! member `1` of the group at index `2`, nesting as deep as the groups do. A group counts as one
//! shape on the top-level space; its members are reached by descending into it. Ask
//! [`Presentation::shape_kind`] what a given address is: the `p:spPr` surface (fill, outline,
//! effects, geometry) applies to shapes, pictures and connectors alike, while text APIs return
//! [`PptxError::ShapeHasNoTextBody`] for a kind that has none.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = std::fs::read("deck.pptx")?;
//! let mut deck = mjx_pptx::Presentation::open(&bytes)?;
//! println!("{}", deck.shape_text(0, 0)?);      // read the first shape's text
//! deck.set_shape_text(0, 0, 0, "New title")?;  // edit the first run
//! let saved = deck.save()?;
//! # let _ = saved;
//! # Ok(())
//! # }
//! ```
//!
//! # Editing one shape several ways
//!
//! Each `set_shape_*` method states the address again, which reads badly once a caller means more
//! than one thing. [`Presentation::shape`] opens a [`ShapeCursor`] instead: the address once, the
//! edits after it, applied together in one pass over the part — with `.member(i)` / `.sibling(i)` /
//! `.parent()` to move around a group while doing it.
//!
//! ```no_run
//! # use mjx_pptx::{Presentation, PptxError};
//! # use mjx_dml::{FillSpec, LineSpec};
//! # fn f(deck: &mut Presentation, navy: FillSpec, gold: FillSpec) -> Result<(), PptxError> {
//! deck.shape(0, 2)?                    // the group at top-level index 2
//!     .member(0)?.fill(navy)
//!     .sibling(1)?.fill(gold).text("Q3")
//!     .apply()?;
//! # Ok(())
//! # }
//! ```

mod address;
mod build;
pub mod constants;
mod cursor;
mod error;
mod geometry;
mod hyperlink;
mod nav;
mod presentation;
mod slide;
mod surface;
mod table;

pub use address::ShapePath;
pub use cursor::ShapeCursor;
pub use error::PptxError;
pub use geometry::{CellMargins, ShapeBounds, SlideSize};
pub use hyperlink::Hyperlink;
pub use presentation::Presentation;
pub use slide::{GraphicFrameKind, PlaceholderInfo, ShapeKind};
pub use surface::Surface;
pub use table::{CellFormat, Cells, TableStyleDefinition, TableStyleFormat};
