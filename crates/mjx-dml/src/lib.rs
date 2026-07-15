//! `mjx-dml` — DrawingML: shapes, text bodies, color model, effects, preset geometry, theme
//! (shared by all formats).
//!
//! # Status
//!
//! The first typed models are the DrawingML **text** types in [`text`] — `a:txBody` / `a:p` / `a:r`
//! / `a:t` — implementing the [`mjx_ooxml_core::FromXml`] / [`mjx_ooxml_core::ToXml`] traits via
//! `#[derive(FromXml, ToXml)]` (the `mjx-derive` proc-macro). They read a real text body out of a
//! slide, expose its text, and rebuild it byte-identically. Preset-shape geometry and the rest of
//! DrawingML follow in later phases.
//!
//! # Fidelity
//!
//! Each modeled type keeps everything it does not itself model — its element name (with prefix), all
//! attributes, the self-closing flag, and any unmodeled children (`a:bodyPr`, `a:rPr`, whitespace,
//! foreign elements) — so a parsed value re-serializes exactly. See [`text`] for the mechanism.

pub mod text;

pub use text::{Paragraph, ParagraphContent, RunContent, Text, TextBody, TextBodyContent, TextRun};
