// Documentation-only module tree: each page is prose in `docs/guide/*.md`, so it reads on a source
// host as well as on the rendered docs page. No page declares an item. Mirrors `mjx_pptx::guide`'s,
// `mjx_docx::guide`'s, `mjx_xlsx::guide`'s, `mjx_ooxml::guide`'s and `mjx_opc::guide`'s shape — see
// any of them for why each module imports the crate's public vocabulary: so that the guide's
// intra-doc links resolve.
//
// This set is written for a caller who has a shape and wants it filled, outlined, positioned or
// coloured, and for a format crate being wired onto this one. It is deliberately *not* a tour of the
// public surface: a page that walked a thousand items would teach nobody anything and would rot on
// contact with the next change, and rustdoc already carries every item's own doc comment
// (MJXOFF-217).
#![doc = include_str!("../docs/guide/README.md")]

/// Everything a guide page may link to, in one place.
macro_rules! guide_vocabulary {
    () => {
        #[allow(unused_imports)]
        use crate::{
            default_theme_xml, geometry, resolve_character_properties, resolve_color,
            resolve_effects, resolve_fill, resolve_line, spreadsheet_drawing,
            wordprocessing_drawing, AbsoluteAnchor, Anchor, AnchorShift, Angle, Bullet,
            BulletColor, BulletSize, BulletTypeface, CharacterProperties, CharacterPropertiesSpec,
            Color, ColorMap, ColorSchemeSlot, ColorSpec, CustomGeometry, CustomGeometrySpec,
            DrawCommand, EffectList, EffectListSpec, Emu, Fill, FillSpec, FontCollection,
            FontScheme, FontSize, FontSlot, Fraction, GeometryGuide, GeometryGuideList, Graphic,
            GraphicData, GraphicDataContent, GroupFill, GuideContext, GuideError, GuideOperator,
            IndentLevel, LineProperties, LineSpec, LineWidth, NoFill, NonVisualDrawingProps,
            OneCellAnchor, Paragraph, ParagraphContent, ParagraphProperties,
            ParagraphPropertiesSpec, Path2D, Path2DList, Path2DSpec, PatternFill, Picture,
            PictureFill, PictureNonVisual, Position, PresetGeometry, ResolvedColor,
            ResolvedCustomGeometry, RunContent, Scene3D, Scene3DSpec, SchemeColors, Shape3D,
            Shape3DSpec, ShapeGeometry, ShapeGeometryChoice, ShapeProperties, Size, SolidFill,
            StyleMatrixReference, Text, TextBody, TextBodyContent, TextField, TextFont,
            TextLineBreak, TextListStyle, TextPoint, TextRun, Theme, ThemeFontReference, ThemeInfo,
            Transform2D, TwoCellAnchor, WorksheetDrawing, DEFAULT_THEME_XML,
        };
    };
}

guide_vocabulary!();

/// Which format meets which DrawingML, which host wrapper lives here rather than there, and the
/// distinction between a complex type and the namespace its element sits in.
pub mod reaching_the_shared_types {
    #![doc = include_str!("../docs/guide/reaching_the_shared_types.md")]
    guide_vocabulary!();
}

/// The fill, the outline and the colour model — and the one thing `ColorSpec` cannot say.
pub mod filling_outlining_and_colour {
    #![doc = include_str!("../docs/guide/filling_outlining_and_colour.md")]
    guide_vocabulary!();
}

/// Where a shape is, what shape it is, and the guide-formula language its coordinates are written in.
pub mod geometry_and_placement {
    #![doc = include_str!("../docs/guide/geometry_and_placement.md")]
    guide_vocabulary!();
}

/// `a:txBody` down to `a:t`, and the difference between merging a format onto a run and replacing it.
pub mod text_bodies {
    #![doc = include_str!("../docs/guide/text_bodies.md")]
    guide_vocabulary!();
}

/// Reading a theme, resolving a name into a colour through it, and the one rule for authoring one.
pub mod the_theme {
    #![doc = include_str!("../docs/guide/the_theme.md")]
    guide_vocabulary!();
}

/// The four serialization mechanisms, what each is backed by, and every gap that is still open.
pub mod fidelity_and_gaps {
    #![doc = include_str!("../docs/guide/fidelity_and_gaps.md")]
    guide_vocabulary!();
}
