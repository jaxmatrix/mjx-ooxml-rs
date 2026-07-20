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
//! pictures (`p:pic`), groups, graphic frames, connectors — in document order, so `shape_idx` means
//! the same thing to every API. A group counts as one shape; its members are not separately
//! addressable. Ask [`Presentation::shape_kind`] what a given index is: the `p:spPr` surface (fill,
//! outline, effects, geometry) applies to shapes, pictures and connectors alike, while text APIs
//! return [`PptxError::ShapeHasNoTextBody`] for a kind that has none.
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

mod build;
pub mod constants;
mod error;
mod geometry;
mod nav;
mod presentation;
mod slide;
mod surface;

pub use error::PptxError;
pub use geometry::{ShapeBounds, SlideSize};
pub use presentation::Presentation;
pub use slide::ShapeKind;
pub use surface::Surface;
