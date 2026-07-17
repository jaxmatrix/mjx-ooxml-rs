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
pub mod effect;
pub mod fill;
pub mod geometry;
pub mod line;
pub mod resolve;
pub mod style;
pub mod text;
pub mod theme;

pub use color::{Color, ColorKind, ColorSpec, SchemeColor};
pub use effect::{
    BlendMode, BlurEffect, EffectList, EffectListSpec, FillOverlayEffect, GlowEffect,
    InnerShadowEffect, OuterShadowEffect, PresetShadow, PresetShadowEffect, RectangleAlignment,
    ReflectionEffect, SoftEdgeEffect,
};
pub use fill::{
    BlipFill, BlipFillMode, Fill, FillSpec, GradientFill, GradientStop, GradientStopSpec,
    GroupFill, NoFill, PatternFill, PatternType, SolidFill, SolidFillContent,
};
pub use geometry::{
    Angle, Emu, Fraction, GeometryGuide, GeometryGuideList, GeometryGuideListContent, LineWidth,
    PresetGeometry, PresetGeometryContent, ResolvedAdjustment, ShapeGeometry,
};
pub use line::{
    CompoundLine, LineCap, LineDash, LineEnd, LineEndLength, LineEndType, LineEndWidth, LineJoin,
    LineProperties, LineSpec, PenAlignment, PresetLineDash,
};
pub use resolve::{resolve_color, resolve_fill, resolve_line, ResolvedColor, SchemeColors};
pub use style::{ColorMap, StyleMatrixReference};
pub use text::{Paragraph, ParagraphContent, RunContent, Text, TextBody, TextBodyContent, TextRun};
pub use theme::{ColorScheme, ColorSchemeSlot, Theme, ThemeInfo};
