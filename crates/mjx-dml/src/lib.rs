//! `mjx-dml` — DrawingML: shapes, text bodies, color model, effects, preset geometry, theme
//! (shared by all formats).
//!
//! **Start at [`guide`]** — six narrative pages written for a caller who has a shape and wants it
//! filled, outlined, positioned or coloured, and for a format crate being wired onto this one. Every
//! item below carries its own doc comment; the guide is the story around them.
//!
//! # What is here
//!
//! [`color`], [`fill`], [`line`](mod@line), [`effect`] and [`shape3d`] are what a shape looks like;
//! [`geometry`] is what shape it is and where, over the named measures ([`Emu`], [`Angle`],
//! [`Fraction`]) all of it is expressed in, with [`geometry::formula`] evaluating the guide-formula
//! language (`a:gd@fmla`) a custom geometry's coordinates are written in. [`text`] is `a:txBody`
//! down to `a:t`; [`table`] is `a:tbl`; [`theme`] is the palette every scheme colour resolves
//! against, and [`resolve`] is the resolver that does it. [`shape_properties`], [`nonvisual`],
//! [`picture`] and [`graphic`] are the wrappers a host puts round all of that, and
//! [`wordprocessing_drawing`] and [`spreadsheet_drawing`] are the two satellite schemas whose
//! content is entirely DrawingML and which therefore live here rather than in `mjx-docx` and
//! `mjx-xlsx`.
//!
//! # Fidelity
//!
//! Each modeled type keeps everything it does not itself model — its element name (with prefix), all
//! attributes, the self-closing flag, and any unmodeled children (`a:bodyPr`, `a:rPr`, whitespace,
//! foreign elements) — so a parsed value re-serializes exactly. [`text`] states the mechanism and
//! [`guide::fidelity_and_gaps`] states which of the four implementations of it each type uses, what
//! each is backed by, and every gap that is still open.

pub(crate) mod build;
pub mod codec;
pub mod color;
pub mod color_transform;
pub mod diagram;
pub mod effect;
pub mod fill;
pub mod geometry;
pub mod graphic;
pub mod guide;
pub mod line;
pub mod nonvisual;
pub mod picture;
pub mod resolve;
pub mod shape3d;
pub mod shape_properties;
pub mod spreadsheet_drawing;
pub mod style;
pub mod table;
pub mod text;
pub mod theme;
pub mod wordprocessing_drawing;

pub use color::{Color, ColorKind, ColorSpec, SchemeColor};
pub use color_transform::{ColorTransform, ColorTransformKind, ColorTransformValue};
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
pub use graphic::{
    Graphic, GraphicData, GraphicDataContent, CHART_GRAPHIC_URI, PICTURE_GRAPHIC_URI,
};
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
pub use spreadsheet_drawing::{
    new_absolute_anchor, new_anchored_picture, new_one_cell_anchor, new_two_cell_anchor,
    AbsoluteAnchor, Anchor, AnchorClientData, AnchorShift, AnchoredObject, CellMarker,
    DrawingConnector, DrawingContentPart, DrawingGraphicFrame, DrawingGroupShape, DrawingPicture,
    DrawingShape, OneCellAnchor, TwoCellAnchor, WorksheetDrawing,
};
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
    default_theme_xml, ColorScheme, ColorSchemeSlot, FontCollection, FontScheme, FontSchemeSlot,
    SupplementalFont, Theme, ThemeFontReference, ThemeInfo, DEFAULT_THEME_XML,
};
