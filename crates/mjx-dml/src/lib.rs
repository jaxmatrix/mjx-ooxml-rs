//! `mjx-dml` — DrawingML: shapes, text bodies, color model, effects, preset geometry, theme
//! (shared by all formats).
//!
//! # Status
//!
//! The first typed models are the DrawingML **text** types in [`text`] — `a:txBody` / `a:p` / `a:r`
//! / `a:t` — implementing the [`mjx_ooxml_core::FromXml`] / [`mjx_ooxml_core::ToXml`] traits via
//! `#[derive(FromXml, ToXml)]` (the `mjx-derive` proc-macro). They read a real text body out of a
//! slide, expose its text, and rebuild it byte-identically. [`geometry`] adds the preset-shape
//! geometry fidelity model (`a:prstGeom` / `a:avLst` / `a:gd`). The rest of DrawingML follows in
//! later phases.
//!
//! # Fidelity
//!
//! Each modeled type keeps everything it does not itself model — its element name (with prefix), all
//! attributes, the self-closing flag, and any unmodeled children (`a:bodyPr`, `a:rPr`, whitespace,
//! foreign elements) — so a parsed value re-serializes exactly. See [`text`] for the mechanism.

pub(crate) mod build;
pub mod color;
pub mod fill;
pub mod geometry;
pub mod text;

pub use color::{Color, ColorKind, SchemeColor};
pub use fill::{
    BlipFill, BlipFillMode, Fill, GradientFill, GradientStop, GroupFill, NoFill, PatternFill,
    PatternType, SolidFill, SolidFillContent,
};
pub use geometry::{
    Angle, Fraction, GeometryGuide, GeometryGuideList, GeometryGuideListContent, PresetGeometry,
    PresetGeometryContent, ResolvedAdjustment, ShapeGeometry,
};
pub use text::{Paragraph, ParagraphContent, RunContent, Text, TextBody, TextBodyContent, TextRun};
