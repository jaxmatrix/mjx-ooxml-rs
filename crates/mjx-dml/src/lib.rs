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
//! later phases, and [`geometry::formula`] evaluates the guide-formula language (`a:gd@fmla`) those
//! geometries express their coordinates in.
//!
//! # Fidelity
//!
//! Each modeled type keeps everything it does not itself model — its element name (with prefix), all
//! attributes, the self-closing flag, and any unmodeled children (`a:bodyPr`, `a:rPr`, whitespace,
//! foreign elements) — so a parsed value re-serializes exactly. See [`text`] for the mechanism.

pub(crate) mod build;
pub mod codec;
pub mod color;
pub mod diagram;
pub mod effect;
pub mod fill;
pub mod geometry;
pub mod graphic;
pub mod line;
pub mod nonvisual;
pub mod picture;
pub mod resolve;
pub mod shape3d;
pub mod shape_properties;
pub mod style;
pub mod table;
pub mod text;
pub mod theme;
pub mod wordprocessing_drawing;

pub use color::{Color, ColorKind, ColorSpec, SchemeColor};
pub use effect::{
    BlendMode, BlurEffect, EffectList, EffectListSpec, FillOverlayEffect, GlowEffect,
    InnerShadowEffect, OuterShadowEffect, PresetShadow, PresetShadowEffect, RectangleAlignment,
    ReflectionEffect, SoftEdgeEffect,
};
pub use fill::{
    Fill, FillSpec, GradientFill, GradientStop, GradientStopSpec, GroupFill, NoFill, PatternFill,
    PatternType, PictureFill, PictureFillMode, SolidFill, SolidFillContent,
};
pub use geometry::{
    AdjustAngle, AdjustCoordinate, AdjustHandle, AdjustPoint, Angle, BoundedAdjustment,
    ConnectionSite, CustomGeometry, CustomGeometrySpec, DrawCommand, Emu, FontSize, Fraction,
    GeometryGuide, GeometryGuideList, GeometryGuideListContent, GuideArgument, GuideContext,
    GuideError, GuideFormula, GuideFormulaError, GuideOperator, GuideSpec, IndentLevel, LineWidth,
    Path2D, Path2DList, Path2DSpec, PathFillMode, Point, Position, PresetGeometry,
    PresetGeometryContent, Rectangle, ResolvedAdjustHandle, ResolvedAdjustment,
    ResolvedConnectionSite, ResolvedCustomGeometry, ResolvedDrawCommand, ResolvedGuides,
    ResolvedPath, ResolvedPoint, ResolvedRectangle, ShapeGeometry, Size, TextPoint, Transform2D,
};
pub use graphic::{Graphic, GraphicData, GraphicDataContent, PICTURE_GRAPHIC_URI};
pub use line::{
    CompoundLine, LineCap, LineDash, LineEnd, LineEndLength, LineEndType, LineEndWidth, LineJoin,
    LineProperties, LineSpec, PenAlignment, PresetLineDash,
};
pub use nonvisual::{
    NonVisualConnectorProperties, NonVisualContentPartProperties, NonVisualDrawingProps,
    NonVisualDrawingShapeProperties, NonVisualGraphicFrameProperties,
    NonVisualGroupDrawingShapeProperties, NonVisualPictureProperties,
};
pub use picture::{new_picture, Picture, PictureNonVisual};
pub use resolve::{
    resolve_character_properties, resolve_color, resolve_effects, resolve_fill, resolve_line,
    ResolvedColor, SchemeColors,
};
pub use shape3d::{
    Backdrop, Bevel, BevelPreset, Camera, LightRig, LightRigDirection, LightRigType, Point3D,
    PresetCamera, PresetMaterial, Scene3D, Scene3DSpec, Shape3D, Shape3DSpec, SphereCoordinates,
    Vector3D,
};
pub use shape_properties::{ShapeGeometryChoice, ShapeProperties};
pub use style::{ColorMap, StyleMatrixReference};
pub use table::{
    applicable_parts, Cell3D, CellBorder, FontCollectionIndex, FontReference, OnOffStyle, Table,
    TableBackgroundStyle, TableCell, TableCellBorderStyle, TableCellContent, TableCellProperties,
    TableColumn, TableContent, TableGrid, TableGridContent, TablePart, TablePartStyle,
    TableProperties, TableRow, TableRowContent, TableStyle, TableStyleBorder, TableStyleCellStyle,
    TableStyleFlags, TableStyleList, TableStylePart, TableStyleTextStyle, TextAnchoring,
    TextDirection, TextHorizontalOverflow, ThemeableLineStyle,
};
pub use text::{
    AutoNumberBullet, AutonumberScheme, Bullet, BulletCharacter, BulletColor, BulletPicture,
    BulletSize, BulletTypeface, CharacterProperties, CharacterPropertiesSpec, FieldContent,
    FontAlignment, FontSlot, LineBreakContent, Paragraph, ParagraphContent, ParagraphProperties,
    ParagraphPropertiesSpec, RunContent, TabAlignment, TabStop, Text, TextAlignment, TextBody,
    TextBodyContent, TextCapitalization, TextField, TextFont, TextLineBreak, TextListStyle,
    TextRun, TextSpacing, TextStrike, TextUnderline, UnderlineFill, UnderlineLine,
};
pub use theme::{
    ColorScheme, ColorSchemeSlot, FontCollection, FontScheme, FontSchemeSlot, SupplementalFont,
    Theme, ThemeFontReference, ThemeInfo,
};
