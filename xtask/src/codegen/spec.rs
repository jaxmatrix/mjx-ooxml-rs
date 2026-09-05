//! Curated code-generation data for the shared-types slice.
//!
//! This is the hand-authored knowledge the generator needs: the naming overrides (comprehensive
//! names sourced from the ECMA-376 prose where the token is cryptic), abbreviation expansions, the
//! boolean-family mapping, and the XSD-base → Rust-primitive table. Extending the generator to
//! wml/sml/pml/dml means growing these tables, not changing the engine.

use crate::codegen::naming::NameEngine;

/// The naming engine for the three schemas whose `ST_*` symbol sets are disjoint and so can
/// share one table: `shared-commonSimpleTypes.xsd`, `dml-main.xsd` and `pml.xsd`.
///
/// `wml.xsd` and `shared-math.xsd` have their own engines below, because they redeclare symbols
/// these three already use — see the note there.
pub const ENGINE: NameEngine = NameEngine {
    type_overrides: TYPE_OVERRIDES,
    variant_overrides: VARIANT_OVERRIDES,
    abbreviations: ABBREVIATIONS,
};

/// lowercase word → PascalCase expansion, applied per word during name construction.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("alg", "Algorithm"),
    ("crypt", "Cryptographic"),
    ("prov", "Provider"),
];

/// `ST_*` → comprehensive Rust type name (only where the mechanical name is not self-explanatory).
const TYPE_OVERRIDES: &[(&str, &str)] = &[
    ("ST_Lang", "LanguageTag"),
    ("ST_String", "XmlString"),
    ("ST_Xstring", "EscapedString"),
    ("ST_ColorType", "Color"),
    ("ST_VerticalAlignRun", "VerticalTextPosition"),
    ("ST_XAlign", "RelativeHorizontalAlignment"),
    ("ST_YAlign", "RelativeVerticalAlignment"),
    // DrawingML preset-shape geometry: the `prst` token of `a:prstGeom`.
    ("ST_ShapeType", "PresetShapeType"),
    // DrawingML theme colors: the `val` token of `a:schemeClr`.
    ("ST_SchemeColorVal", "SchemeColor"),
    // DrawingML pattern fills: the `prst` token of `a:pattFill`.
    ("ST_PresetPatternVal", "PatternType"),
    // DrawingML theme color-scheme slots: the `a:clrScheme` slot names + `p:clrMap` targets.
    ("ST_ColorSchemeIndex", "ColorSchemeSlot"),
    // DrawingML line (outline) properties: `a:ln`'s attributes and its head/tail line-end sub-elements.
    ("ST_LineCap", "LineCap"),
    ("ST_CompoundLine", "CompoundLine"),
    ("ST_PenAlignment", "PenAlignment"),
    ("ST_PresetLineDashVal", "PresetLineDash"),
    ("ST_LineEndType", "LineEndType"),
    ("ST_LineEndWidth", "LineEndWidth"),
    ("ST_LineEndLength", "LineEndLength"),
    // DrawingML effects: the preset shadow kind (`a:prstShdw`) and the rectangle alignment shared by
    // shadow/reflection effects.
    ("ST_PresetShadowVal", "PresetShadow"),
    ("ST_RectAlignment", "RectangleAlignment"),
    // DrawingML fill-overlay blend mode: `a:fillOverlay@blend`.
    ("ST_BlendMode", "BlendMode"),
    // DrawingML text: run properties (`a:rPr@u`/`@strike`/`@cap`) and paragraph properties
    // (`a:pPr@algn`/`@fontAlgn`, `a:tab@algn`, `a:buAutoNum@type`). Each is named for what it selects
    // rather than the schema's generic "…Type" suffix.
    ("ST_TextUnderlineType", "TextUnderline"),
    ("ST_TextStrikeType", "TextStrike"),
    ("ST_TextCapsType", "TextCapitalization"),
    ("ST_TextAlignType", "TextAlignment"),
    ("ST_TextFontAlignType", "FontAlignment"),
    ("ST_TextTabAlignType", "TabAlignment"),
    ("ST_TextAutonumberScheme", "AutonumberScheme"),
    // DrawingML text framing: `a:tcPr`/`a:bodyPr`'s `@anchor`, `@vert` and `@horzOverflow`. The
    // vertical one is named `TextDirection` because its own values include `horz` (Horizontal) — it
    // selects which way text flows, so naming it "vertical" would misdescribe most of its range.
    ("ST_TextAnchoringType", "TextAnchoring"),
    ("ST_TextVerticalType", "TextDirection"),
    ("ST_TextHorzOverflowType", "TextHorizontalOverflow"),
    // DrawingML table styles: the tri-state take on a style property (`a:tcTxStyle@b`/`@i`), named
    // for its three-way on/off/inherit sense rather than the schema's generic "…Type" suffix.
    // `ST_FontCollectionIndex` (major/minor/none) needs no override — it auto-expands cleanly.
    ("ST_OnOffStyleType", "OnOffStyle"),
    // DrawingML 3-D: the bevel preset (`a:bevel@prst`), the light-rig kind (`a:lightRig@rig`) and
    // its direction (`@dir`), the surface material (`a:sp3d`/`a:cell3D@prstMaterial`), and the preset
    // camera view (`a:camera@prst`). Each is named for what it selects, dropping the `…Type` suffix.
    ("ST_BevelPresetType", "BevelPreset"),
    ("ST_LightRigType", "LightRigType"),
    ("ST_LightRigDirection", "LightRigDirection"),
    ("ST_PresetMaterialType", "PresetMaterial"),
    ("ST_PresetCameraType", "PresetCamera"),
    // DrawingML custom geometry: how a freeform path is filled (`a:custGeom`'s `a:path@fill`). Named
    // for what it selects, dropping the `…Mode` schema suffix would lose meaning, so it is kept.
    ("ST_PathFillMode", "PathFillMode"),
    // PresentationML placeholders: `p:ph`'s `type`, `sz`, and `orient`. `ST_Direction` is PML's own
    // two-valued axis (`horz`/`vert`), named for what it selects rather than the generic "direction".
    ("ST_PlaceholderType", "PlaceholderType"),
    ("ST_PlaceholderSize", "PlaceholderSize"),
    ("ST_Direction", "Orientation"),
    // PresentationML slide layouts and slide size: `p:sldLayout@type` and `p:sldSz@type`. Both are
    // named `*Kind` because the wire attribute is `type`, which is a Rust keyword in field position.
    ("ST_SlideLayoutType", "SlideLayoutKind"),
    ("ST_SlideSizeType", "SlideSizeKind"),
    // DrawingML WordprocessingDrawing (`wp:`, MJXOFF-131): `H`/`V` are the schema's own contraction
    // for "Horizontal"/"Vertical" — `CT_PosH`/`CT_PosV`'s own element names are `positionH`/
    // `positionV`, and ECMA-376 Part 1 §20.4.2.6/`.7` and `.9`/`.10` spell the concepts out in full
    // ("Relative Horizontal/Vertical Positioning Alignment", "Position Horizontal/Vertical
    // Relative Base") — so the mechanical `AlignH`/`AlignV`/`RelFromH`/`RelFromV` the engine would
    // otherwise emit are expanded here rather than left as bare letter suffixes.
    ("ST_AlignH", "HorizontalAlignment"),
    ("ST_AlignV", "VerticalAlignment"),
    ("ST_RelFromH", "HorizontalRelativeFrom"),
    ("ST_RelFromV", "VerticalRelativeFrom"),
];

/// (`ST_*`, wire value) → comprehensive Rust variant name, for cryptic tokens (from ECMA-376 prose).
const VARIANT_OVERRIDES: &[(&str, &str, &str)] = &[
    ("ST_CalendarType", "gregorianUs", "GregorianUnitedStates"),
    (
        "ST_CalendarType",
        "gregorianMeFrench",
        "GregorianMiddleEastFrench",
    ),
    (
        "ST_CalendarType",
        "gregorianXlitEnglish",
        "GregorianTransliteratedEnglish",
    ),
    (
        "ST_CalendarType",
        "gregorianXlitFrench",
        "GregorianTransliteratedFrench",
    ),
    ("ST_AlgType", "typeAny", "Any"),
    // `ST_PathFillMode` (`a:path@fill`): `norm` is the default "normal" fill. The rest
    // (`none`, `lighten`, `lightenLess`, `darken`, `darkenLess`) auto-expand cleanly.
    ("ST_PathFillMode", "norm", "Normal"),
    // `ST_ShapeType` (`a:prstGeom@prst`): expand the cryptic/abbreviated tokens. Well-formed tokens
    // (`flowChartProcess`, `actionButtonHome`, `hexagon`, `mathPlus`, …) auto-expand and need no row.
    // The exact wire token is preserved on each generated variant's doc comment.
    ("ST_ShapeType", "line", "StraightLine"),
    ("ST_ShapeType", "lineInv", "StraightLineInverse"),
    ("ST_ShapeType", "rtTriangle", "RightTriangle"),
    ("ST_ShapeType", "rect", "Rectangle"),
    ("ST_ShapeType", "star4", "FourPointStar"),
    ("ST_ShapeType", "star5", "FivePointStar"),
    ("ST_ShapeType", "star6", "SixPointStar"),
    ("ST_ShapeType", "star7", "SevenPointStar"),
    ("ST_ShapeType", "star8", "EightPointStar"),
    ("ST_ShapeType", "star10", "TenPointStar"),
    ("ST_ShapeType", "star12", "TwelvePointStar"),
    ("ST_ShapeType", "star16", "SixteenPointStar"),
    ("ST_ShapeType", "star24", "TwentyFourPointStar"),
    ("ST_ShapeType", "star32", "ThirtyTwoPointStar"),
    ("ST_ShapeType", "roundRect", "RoundedRectangle"),
    ("ST_ShapeType", "round1Rect", "RoundSingleCornerRectangle"),
    (
        "ST_ShapeType",
        "round2SameRect",
        "RoundSameSideCornersRectangle",
    ),
    (
        "ST_ShapeType",
        "round2DiagRect",
        "RoundDiagonalCornersRectangle",
    ),
    (
        "ST_ShapeType",
        "snipRoundRect",
        "SnipAndRoundSingleCornerRectangle",
    ),
    ("ST_ShapeType", "snip1Rect", "SnipSingleCornerRectangle"),
    (
        "ST_ShapeType",
        "snip2SameRect",
        "SnipSameSideCornersRectangle",
    ),
    (
        "ST_ShapeType",
        "snip2DiagRect",
        "SnipDiagonalCornersRectangle",
    ),
    ("ST_ShapeType", "diagStripe", "DiagonalStripe"),
    ("ST_ShapeType", "uturnArrow", "UTurnArrow"),
    ("ST_ShapeType", "wedgeRectCallout", "WedgeRectangleCallout"),
    (
        "ST_ShapeType",
        "wedgeRoundRectCallout",
        "WedgeRoundedRectangleCallout",
    ),
    // `ST_SchemeColorVal` (`a:schemeClr@val`): expand the cryptic theme-slot tokens. `accent1`..`accent6`
    // auto-expand and need no row.
    ("ST_SchemeColorVal", "bg1", "Background1"),
    ("ST_SchemeColorVal", "tx1", "Text1"),
    ("ST_SchemeColorVal", "bg2", "Background2"),
    ("ST_SchemeColorVal", "tx2", "Text2"),
    ("ST_SchemeColorVal", "hlink", "Hyperlink"),
    ("ST_SchemeColorVal", "folHlink", "FollowedHyperlink"),
    ("ST_SchemeColorVal", "phClr", "PlaceholderColor"),
    ("ST_SchemeColorVal", "dk1", "Dark1"),
    ("ST_SchemeColorVal", "lt1", "Light1"),
    ("ST_SchemeColorVal", "dk2", "Dark2"),
    ("ST_SchemeColorVal", "lt2", "Light2"),
    // `ST_PresetPatternVal` (`a:pattFill@prst`): expand the cryptic pattern tokens to the ECMA-376
    // prose names. `cross`/`plaid`/`sphere`/`weave`/`divot`/`shingle`/`wave`/`trellis` auto-expand
    // and need no row. Abbreviations: `pct`→Percent, `lt`→Light, `dk`→Dark, `nar`→Narrow,
    // `dash`→Dashed, `dn`→Downward, `up`→Upward, `wd`→Wide, `horz`→Horizontal, `vert`→Vertical,
    // `sm`→Small, `lg`→Large, `dot`→Dotted, `dmnd`→Diamond, `diag`→Diagonal, `check`→Checkerboard.
    ("ST_PresetPatternVal", "pct5", "Percent5"),
    ("ST_PresetPatternVal", "pct10", "Percent10"),
    ("ST_PresetPatternVal", "pct20", "Percent20"),
    ("ST_PresetPatternVal", "pct25", "Percent25"),
    ("ST_PresetPatternVal", "pct30", "Percent30"),
    ("ST_PresetPatternVal", "pct40", "Percent40"),
    ("ST_PresetPatternVal", "pct50", "Percent50"),
    ("ST_PresetPatternVal", "pct60", "Percent60"),
    ("ST_PresetPatternVal", "pct70", "Percent70"),
    ("ST_PresetPatternVal", "pct75", "Percent75"),
    ("ST_PresetPatternVal", "pct80", "Percent80"),
    ("ST_PresetPatternVal", "pct90", "Percent90"),
    ("ST_PresetPatternVal", "horz", "Horizontal"),
    ("ST_PresetPatternVal", "vert", "Vertical"),
    ("ST_PresetPatternVal", "ltHorz", "LightHorizontal"),
    ("ST_PresetPatternVal", "ltVert", "LightVertical"),
    ("ST_PresetPatternVal", "dkHorz", "DarkHorizontal"),
    ("ST_PresetPatternVal", "dkVert", "DarkVertical"),
    ("ST_PresetPatternVal", "narHorz", "NarrowHorizontal"),
    ("ST_PresetPatternVal", "narVert", "NarrowVertical"),
    ("ST_PresetPatternVal", "dashHorz", "DashedHorizontal"),
    ("ST_PresetPatternVal", "dashVert", "DashedVertical"),
    ("ST_PresetPatternVal", "dnDiag", "DownwardDiagonal"),
    ("ST_PresetPatternVal", "upDiag", "UpwardDiagonal"),
    ("ST_PresetPatternVal", "ltDnDiag", "LightDownwardDiagonal"),
    ("ST_PresetPatternVal", "ltUpDiag", "LightUpwardDiagonal"),
    ("ST_PresetPatternVal", "dkDnDiag", "DarkDownwardDiagonal"),
    ("ST_PresetPatternVal", "dkUpDiag", "DarkUpwardDiagonal"),
    ("ST_PresetPatternVal", "wdDnDiag", "WideDownwardDiagonal"),
    ("ST_PresetPatternVal", "wdUpDiag", "WideUpwardDiagonal"),
    (
        "ST_PresetPatternVal",
        "dashDnDiag",
        "DashedDownwardDiagonal",
    ),
    ("ST_PresetPatternVal", "dashUpDiag", "DashedUpwardDiagonal"),
    ("ST_PresetPatternVal", "diagCross", "DiagonalCross"),
    ("ST_PresetPatternVal", "smCheck", "SmallCheckerboard"),
    ("ST_PresetPatternVal", "lgCheck", "LargeCheckerboard"),
    ("ST_PresetPatternVal", "smGrid", "SmallGrid"),
    ("ST_PresetPatternVal", "lgGrid", "LargeGrid"),
    ("ST_PresetPatternVal", "dotGrid", "DottedGrid"),
    ("ST_PresetPatternVal", "smConfetti", "SmallConfetti"),
    ("ST_PresetPatternVal", "lgConfetti", "LargeConfetti"),
    ("ST_PresetPatternVal", "horzBrick", "HorizontalBrick"),
    ("ST_PresetPatternVal", "diagBrick", "DiagonalBrick"),
    ("ST_PresetPatternVal", "solidDmnd", "SolidDiamond"),
    ("ST_PresetPatternVal", "openDmnd", "OpenDiamond"),
    ("ST_PresetPatternVal", "dotDmnd", "DottedDiamond"),
    ("ST_PresetPatternVal", "zigZag", "ZigZag"),
    // `ST_ColorSchemeIndex` (`a:clrScheme` slot names / `p:clrMap` targets): expand the cryptic
    // dark/light and hyperlink tokens. `accent1`..`accent6` auto-expand and need no row.
    ("ST_ColorSchemeIndex", "dk1", "Dark1"),
    ("ST_ColorSchemeIndex", "lt1", "Light1"),
    ("ST_ColorSchemeIndex", "dk2", "Dark2"),
    ("ST_ColorSchemeIndex", "lt2", "Light2"),
    ("ST_ColorSchemeIndex", "hlink", "Hyperlink"),
    ("ST_ColorSchemeIndex", "folHlink", "FollowedHyperlink"),
    // `ST_LineCap` (`a:ln@cap`): expand the abbreviated end-cap tokens (ECMA-376 §20.1.10.31).
    ("ST_LineCap", "rnd", "Round"),
    ("ST_LineCap", "sq", "Square"),
    // `flat` auto-expands.
    // `ST_CompoundLine` (`a:ln@cmpd`): expand the abbreviated compound-line tokens (§20.1.10.15).
    // `thickThin`/`thinThick` auto-expand.
    ("ST_CompoundLine", "sng", "Single"),
    ("ST_CompoundLine", "dbl", "Double"),
    ("ST_CompoundLine", "tri", "Triple"),
    // `ST_PenAlignment` (`a:ln@algn`): expand the pen-alignment tokens (§20.1.10.40). `in` is also a
    // Rust keyword, so it must not fall through to the mechanical `In`.
    ("ST_PenAlignment", "ctr", "Center"),
    ("ST_PenAlignment", "in", "Inset"),
    // `ST_PresetLineDashVal` (`a:prstDash@val`): expand the abbreviated dash tokens (§20.1.10.48).
    // `lg`→Large, `sys`→System; `solid`/`dot`/`dash`/`dashDot` auto-expand.
    ("ST_PresetLineDashVal", "lgDash", "LargeDash"),
    ("ST_PresetLineDashVal", "lgDashDot", "LargeDashDot"),
    ("ST_PresetLineDashVal", "lgDashDotDot", "LargeDashDotDot"),
    ("ST_PresetLineDashVal", "sysDash", "SystemDash"),
    ("ST_PresetLineDashVal", "sysDot", "SystemDot"),
    ("ST_PresetLineDashVal", "sysDashDot", "SystemDashDot"),
    ("ST_PresetLineDashVal", "sysDashDotDot", "SystemDashDotDot"),
    // `ST_LineEndType` (`a:headEnd`/`a:tailEnd@type`, §20.1.10.33): all tokens
    // (`none`/`triangle`/`stealth`/`diamond`/`oval`/`arrow`) auto-expand — no rows needed.
    // `ST_LineEndWidth` (`@w`) / `ST_LineEndLength` (`@len`): expand the size tokens (§20.1.10.34/.32).
    ("ST_LineEndWidth", "sm", "Small"),
    ("ST_LineEndWidth", "med", "Medium"),
    ("ST_LineEndWidth", "lg", "Large"),
    ("ST_LineEndLength", "sm", "Small"),
    ("ST_LineEndLength", "med", "Medium"),
    ("ST_LineEndLength", "lg", "Large"),
    // `ST_PresetShadowVal` (`a:prstShdw@prst`, §20.1.10.50): 20 numbered preset shadows with no
    // semantic name — `ShadowN` is the clearest faithful form (the mechanical split gives `Shdw1`).
    ("ST_PresetShadowVal", "shdw1", "Shadow1"),
    ("ST_PresetShadowVal", "shdw2", "Shadow2"),
    ("ST_PresetShadowVal", "shdw3", "Shadow3"),
    ("ST_PresetShadowVal", "shdw4", "Shadow4"),
    ("ST_PresetShadowVal", "shdw5", "Shadow5"),
    ("ST_PresetShadowVal", "shdw6", "Shadow6"),
    ("ST_PresetShadowVal", "shdw7", "Shadow7"),
    ("ST_PresetShadowVal", "shdw8", "Shadow8"),
    ("ST_PresetShadowVal", "shdw9", "Shadow9"),
    ("ST_PresetShadowVal", "shdw10", "Shadow10"),
    ("ST_PresetShadowVal", "shdw11", "Shadow11"),
    ("ST_PresetShadowVal", "shdw12", "Shadow12"),
    ("ST_PresetShadowVal", "shdw13", "Shadow13"),
    ("ST_PresetShadowVal", "shdw14", "Shadow14"),
    ("ST_PresetShadowVal", "shdw15", "Shadow15"),
    ("ST_PresetShadowVal", "shdw16", "Shadow16"),
    ("ST_PresetShadowVal", "shdw17", "Shadow17"),
    ("ST_PresetShadowVal", "shdw18", "Shadow18"),
    ("ST_PresetShadowVal", "shdw19", "Shadow19"),
    ("ST_PresetShadowVal", "shdw20", "Shadow20"),
    // `ST_RectAlignment` (effect `@algn`, §20.1.10.53): expand the compass-abbreviation tokens.
    ("ST_RectAlignment", "tl", "TopLeft"),
    ("ST_RectAlignment", "t", "Top"),
    ("ST_RectAlignment", "tr", "TopRight"),
    ("ST_RectAlignment", "l", "Left"),
    ("ST_RectAlignment", "ctr", "Center"),
    ("ST_RectAlignment", "r", "Right"),
    ("ST_RectAlignment", "bl", "BottomLeft"),
    ("ST_RectAlignment", "b", "Bottom"),
    ("ST_RectAlignment", "br", "BottomRight"),
    // `ST_BlendMode` (`a:fillOverlay@blend`, §20.1.10.11): expand the abbreviated multiply token.
    // `over`/`screen`/`darken`/`lighten` auto-expand and need no row.
    ("ST_BlendMode", "mult", "Multiply"),
    // `ST_PlaceholderType` (`p:ph@type`, §19.7.10): every name is the enumeration's official title
    // from the Part 1 table ("ctrTitle (Centered Title)", "dt (Date and Time)", …).
    // `body`/`chart`/`clipArt`/`media`/`title` auto-expand and need no row.
    ("ST_PlaceholderType", "ctrTitle", "CenteredTitle"),
    ("ST_PlaceholderType", "subTitle", "Subtitle"),
    ("ST_PlaceholderType", "dt", "DateAndTime"),
    ("ST_PlaceholderType", "ftr", "Footer"),
    ("ST_PlaceholderType", "hdr", "Header"),
    ("ST_PlaceholderType", "sldNum", "SlideNumber"),
    ("ST_PlaceholderType", "sldImg", "SlideImage"),
    ("ST_PlaceholderType", "obj", "Object"),
    ("ST_PlaceholderType", "pic", "Picture"),
    ("ST_PlaceholderType", "tbl", "Table"),
    ("ST_PlaceholderType", "dgm", "Diagram"),
    // `ST_SlideLayoutType` (`p:sldLayout@type`, §19.7.15): the Part 1 table gives each value an
    // official title — `obj` is "Title and Object", not merely "object", and the multi-object
    // arrangements are plural. Names below are those titles; `blank`/`chart` auto-expand.
    ("ST_SlideLayoutType", "tx", "Text"),
    ("ST_SlideLayoutType", "twoColTx", "TwoColumnText"),
    ("ST_SlideLayoutType", "tbl", "Table"),
    ("ST_SlideLayoutType", "txAndChart", "TextAndChart"),
    ("ST_SlideLayoutType", "chartAndTx", "ChartAndText"),
    ("ST_SlideLayoutType", "dgm", "Diagram"),
    ("ST_SlideLayoutType", "txAndClipArt", "TextAndClipArt"),
    ("ST_SlideLayoutType", "clipArtAndTx", "ClipArtAndText"),
    ("ST_SlideLayoutType", "txAndObj", "TextAndObject"),
    ("ST_SlideLayoutType", "objAndTx", "ObjectAndText"),
    ("ST_SlideLayoutType", "objOnly", "ObjectOnly"),
    ("ST_SlideLayoutType", "obj", "TitleAndObject"),
    ("ST_SlideLayoutType", "txAndMedia", "TextAndMedia"),
    ("ST_SlideLayoutType", "mediaAndTx", "MediaAndText"),
    ("ST_SlideLayoutType", "objOverTx", "ObjectOverText"),
    ("ST_SlideLayoutType", "txOverObj", "TextOverObject"),
    ("ST_SlideLayoutType", "txAndTwoObj", "TextAndTwoObjects"),
    ("ST_SlideLayoutType", "twoObjAndTx", "TwoObjectsAndText"),
    ("ST_SlideLayoutType", "twoObjOverTx", "TwoObjectsOverText"),
    ("ST_SlideLayoutType", "fourObj", "FourObjects"),
    ("ST_SlideLayoutType", "vertTx", "VerticalText"),
    (
        "ST_SlideLayoutType",
        "clipArtAndVertTx",
        "ClipArtAndVerticalText",
    ),
    (
        "ST_SlideLayoutType",
        "vertTitleAndTx",
        "VerticalTitleAndText",
    ),
    (
        "ST_SlideLayoutType",
        "vertTitleAndTxOverChart",
        "VerticalTitleAndTextOverChart",
    ),
    ("ST_SlideLayoutType", "twoObj", "TwoObjects"),
    ("ST_SlideLayoutType", "objAndTwoObj", "ObjectAndTwoObjects"),
    ("ST_SlideLayoutType", "twoObjAndObj", "TwoObjectsAndObject"),
    ("ST_SlideLayoutType", "cust", "Custom"),
    ("ST_SlideLayoutType", "secHead", "SectionHeader"),
    ("ST_SlideLayoutType", "twoTxTwoObj", "TwoTextAndTwoObjects"),
    ("ST_SlideLayoutType", "objTx", "TitleObjectAndCaption"),
    ("ST_SlideLayoutType", "picTx", "PictureAndCaption"),
    // `ST_Direction` (`p:ph@orient`, §19.7.2): the two abbreviated axis tokens ("horz (Horizontal)").
    ("ST_Direction", "horz", "Horizontal"),
    ("ST_Direction", "vert", "Vertical"),
    // `ST_SlideSizeType` (`p:sldSz@type`, §19.7.18): only the digit-leading token needs a name (the
    // mechanical one would be `N35Mm`); the paper and screen sizes auto-expand acceptably.
    ("ST_SlideSizeType", "35mm", "Film35Mm"),
    // `ST_TextUnderlineType` (`a:rPr@u`, §20.1.10.82): names are the enumeration table's official
    // titles ("dashHeavy (Text Underline Enum ( Heavy Dashed ))"), which read modifier-first —
    // `HeavyDashed`, not `DashedHeavy`. `none`/`words`/`heavy`/`dotted`/`wavy`/`dotDash`/`dotDotDash`
    // already match their titles and need no row.
    ("ST_TextUnderlineType", "sng", "Single"),
    ("ST_TextUnderlineType", "dbl", "Double"),
    ("ST_TextUnderlineType", "dash", "Dashed"),
    ("ST_TextUnderlineType", "dottedHeavy", "HeavyDotted"),
    ("ST_TextUnderlineType", "dashHeavy", "HeavyDashed"),
    ("ST_TextUnderlineType", "dashLong", "LongDashed"),
    ("ST_TextUnderlineType", "dashLongHeavy", "HeavyLongDashed"),
    ("ST_TextUnderlineType", "dotDashHeavy", "HeavyDotDash"),
    ("ST_TextUnderlineType", "dotDotDashHeavy", "HeavyDotDotDash"),
    ("ST_TextUnderlineType", "wavyHeavy", "HeavyWavy"),
    ("ST_TextUnderlineType", "wavyDbl", "DoubleWavy"),
    // `ST_TextStrikeType` (`a:rPr@strike`, §20.1.10.79): "No Strike" / "Single Strike" / "Double
    // Strike". `noStrike` auto-expands to the title already.
    ("ST_TextStrikeType", "sngStrike", "SingleStrike"),
    ("ST_TextStrikeType", "dblStrike", "DoubleStrike"),
    // `ST_TextCapsType` (`a:rPr@cap`, §20.1.10.64) needs no rows: `none`/`small`/`all` are the titles.
    // `ST_TextAlignType` (`a:pPr@algn`, §20.1.10.59): paragraph alignment.
    ("ST_TextAlignType", "l", "Left"),
    ("ST_TextAlignType", "ctr", "Center"),
    ("ST_TextAlignType", "r", "Right"),
    ("ST_TextAlignType", "just", "Justified"),
    ("ST_TextAlignType", "justLow", "JustifiedLow"),
    ("ST_TextAlignType", "dist", "Distributed"),
    ("ST_TextAlignType", "thaiDist", "ThaiDistributed"),
    // `ST_TextFontAlignType` (`a:pPr@fontAlgn`, §20.1.10.66): where letters sit between the baselines.
    ("ST_TextFontAlignType", "auto", "Automatic"),
    ("ST_TextFontAlignType", "t", "Top"),
    ("ST_TextFontAlignType", "ctr", "Center"),
    ("ST_TextFontAlignType", "base", "Baseline"),
    ("ST_TextFontAlignType", "b", "Bottom"),
    // `ST_TextTabAlignType` (`a:tab@algn`, §20.1.10.80).
    ("ST_TextTabAlignType", "l", "Left"),
    ("ST_TextTabAlignType", "ctr", "Center"),
    // `ST_TextAnchoringType` (`a:tcPr@anchor`, `a:bodyPr@anchor`): the titles ECMA gives each token
    // in its "Text Anchor Enum" column (§20.1.10.60).
    ("ST_TextAnchoringType", "t", "Top"),
    ("ST_TextAnchoringType", "ctr", "Center"),
    ("ST_TextAnchoringType", "b", "Bottom"),
    ("ST_TextAnchoringType", "just", "Justified"),
    ("ST_TextAnchoringType", "dist", "Distributed"),
    // `ST_TextVerticalType` (`a:tcPr@vert`, `a:bodyPr@vert`): the "Vertical Text Type Enum" titles
    // (§20.1.10.83). `wordArtVertRtl` is titled "Vertical WordArt Right to Left" rather than
    // following its siblings' word order, and the prose wins over consistency — the same call the
    // underline names made for `dashHeavy` ("Heavy Dashed").
    ("ST_TextVerticalType", "horz", "Horizontal"),
    ("ST_TextVerticalType", "vert", "Vertical"),
    ("ST_TextVerticalType", "vert270", "Vertical270"),
    ("ST_TextVerticalType", "wordArtVert", "WordArtVertical"),
    ("ST_TextVerticalType", "eaVert", "EastAsianVertical"),
    ("ST_TextVerticalType", "mongolianVert", "MongolianVertical"),
    (
        "ST_TextVerticalType",
        "wordArtVertRtl",
        "VerticalWordArtRightToLeft",
    ),
    // `ST_TextHorzOverflowType` (`a:tcPr@horzOverflow`): §20.1.10.62.
    ("ST_TextHorzOverflowType", "overflow", "Overflow"),
    ("ST_TextHorzOverflowType", "clip", "Clip"),
    ("ST_TextTabAlignType", "r", "Right"),
    ("ST_TextTabAlignType", "dec", "Decimal"),
    // `ST_TextAutonumberScheme` (`a:buAutoNum@type`, §20.1.10.61) — the bullet numbering schemes.
    // Unusually, the table's titles merely repeat the wire token, so each name is derived from the
    // **Description** column instead: `alphaLcParenBoth` is described as "(a), (b), (c), …", i.e.
    // lowercase letters wrapped in parentheses on both sides. The three axes compose:
    //   numerals   — LowercaseLetter / UppercaseLetter / LowercaseRoman / UppercaseRoman / Arabic / …
    //   punctuation— ParenthesesBoth "(a)" / ParenthesisRight "a)" / Period "a." / Plain "a"
    (
        "ST_TextAutonumberScheme",
        "alphaLcParenBoth",
        "LowercaseLetterParenthesesBoth",
    ),
    (
        "ST_TextAutonumberScheme",
        "alphaLcParenR",
        "LowercaseLetterParenthesisRight",
    ),
    (
        "ST_TextAutonumberScheme",
        "alphaLcPeriod",
        "LowercaseLetterPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "alphaUcParenBoth",
        "UppercaseLetterParenthesesBoth",
    ),
    (
        "ST_TextAutonumberScheme",
        "alphaUcParenR",
        "UppercaseLetterParenthesisRight",
    ),
    (
        "ST_TextAutonumberScheme",
        "alphaUcPeriod",
        "UppercaseLetterPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "romanLcParenBoth",
        "LowercaseRomanParenthesesBoth",
    ),
    (
        "ST_TextAutonumberScheme",
        "romanLcParenR",
        "LowercaseRomanParenthesisRight",
    ),
    (
        "ST_TextAutonumberScheme",
        "romanLcPeriod",
        "LowercaseRomanPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "romanUcParenBoth",
        "UppercaseRomanParenthesesBoth",
    ),
    (
        "ST_TextAutonumberScheme",
        "romanUcParenR",
        "UppercaseRomanParenthesisRight",
    ),
    (
        "ST_TextAutonumberScheme",
        "romanUcPeriod",
        "UppercaseRomanPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "arabicParenBoth",
        "ArabicParenthesesBoth",
    ),
    (
        "ST_TextAutonumberScheme",
        "arabicParenR",
        "ArabicParenthesisRight",
    ),
    ("ST_TextAutonumberScheme", "arabicPeriod", "ArabicPeriod"),
    ("ST_TextAutonumberScheme", "arabicPlain", "ArabicPlain"),
    // "Dbl-byte Arabic numbers" (with and without a double-byte period).
    (
        "ST_TextAutonumberScheme",
        "arabicDbPeriod",
        "DoubleByteArabicPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "arabicDbPlain",
        "DoubleByteArabicPlain",
    ),
    // "Bidi <script> N with ANSI minus symbol"; the parentheticals name the two Arabic systems.
    (
        "ST_TextAutonumberScheme",
        "arabic1Minus",
        "BidirectionalArabicAlphabeticMinus",
    ),
    (
        "ST_TextAutonumberScheme",
        "arabic2Minus",
        "BidirectionalArabicAbjadMinus",
    ),
    (
        "ST_TextAutonumberScheme",
        "hebrew2Minus",
        "BidirectionalHebrewMinus",
    ),
    // Circled numbers: double-byte, and the two Wingdings sets.
    (
        "ST_TextAutonumberScheme",
        "circleNumDbPlain",
        "DoubleByteCircledNumberPlain",
    ),
    (
        "ST_TextAutonumberScheme",
        "circleNumWdBlackPlain",
        "WingdingsBlackCircledNumberPlain",
    ),
    (
        "ST_TextAutonumberScheme",
        "circleNumWdWhitePlain",
        "WingdingsWhiteCircledNumberPlain",
    ),
    // East Asian ("EA:" in the descriptions); `ea1` is the spec's family prefix, not a numeral.
    (
        "ST_TextAutonumberScheme",
        "ea1ChsPeriod",
        "SimplifiedChinesePeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "ea1ChsPlain",
        "SimplifiedChinesePlain",
    ),
    (
        "ST_TextAutonumberScheme",
        "ea1ChtPeriod",
        "TraditionalChinesePeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "ea1ChtPlain",
        "TraditionalChinesePlain",
    ),
    (
        "ST_TextAutonumberScheme",
        "ea1JpnChsDbPeriod",
        "JapaneseDoubleBytePeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "ea1JpnKorPeriod",
        "JapaneseKoreanPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "ea1JpnKorPlain",
        "JapaneseKoreanPlain",
    ),
    // Hindi: the alphabet forms are distinguished as vowels vs consonants by the descriptions.
    (
        "ST_TextAutonumberScheme",
        "hindiAlphaPeriod",
        "HindiVowelPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "hindiAlpha1Period",
        "HindiConsonantPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "hindiNumPeriod",
        "HindiNumberPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "hindiNumParenR",
        "HindiNumberParenthesisRight",
    ),
    // Thai.
    (
        "ST_TextAutonumberScheme",
        "thaiAlphaPeriod",
        "ThaiLetterPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "thaiAlphaParenR",
        "ThaiLetterParenthesisRight",
    ),
    (
        "ST_TextAutonumberScheme",
        "thaiAlphaParenBoth",
        "ThaiLetterParenthesesBoth",
    ),
    (
        "ST_TextAutonumberScheme",
        "thaiNumPeriod",
        "ThaiNumberPeriod",
    ),
    (
        "ST_TextAutonumberScheme",
        "thaiNumParenR",
        "ThaiNumberParenthesisRight",
    ),
    (
        "ST_TextAutonumberScheme",
        "thaiNumParenBoth",
        "ThaiNumberParenthesesBoth",
    ),
    // `ST_OnOffStyleType` (§20.1.10.36): `on`/`off` auto-expand; `def` means "follow parent / theme
    // settings", which the ECMA prose titles "Default".
    ("ST_OnOffStyleType", "def", "Default"),
    // `ST_LightRigDirection` (`a:lightRig@dir`, §20.1.10.31): the compass-abbreviation tokens, as
    // `ST_RectAlignment` above (there is no `ctr` here — a light has a direction, not a centre).
    ("ST_LightRigDirection", "tl", "TopLeft"),
    ("ST_LightRigDirection", "t", "Top"),
    ("ST_LightRigDirection", "tr", "TopRight"),
    ("ST_LightRigDirection", "l", "Left"),
    ("ST_LightRigDirection", "r", "Right"),
    ("ST_LightRigDirection", "bl", "BottomLeft"),
    ("ST_LightRigDirection", "b", "Bottom"),
    ("ST_LightRigDirection", "br", "BottomRight"),
    // `ST_LightRigType` (`a:lightRig@rig`, §20.1.10.32): the `nPt` tokens abbreviate "point". Every
    // other token (`legacyFlat1`, `brightRoom`, `sunset`, …) auto-expands cleanly.
    ("ST_LightRigType", "threePt", "ThreePoint"),
    ("ST_LightRigType", "twoPt", "TwoPoint"),
    // `ST_PresetMaterialType` (`@prstMaterial`, §20.1.10.50): `dk` abbreviates "dark", and `softmetal`
    // has no camel hump to split on. The rest (`legacyMatte`, `warmMatte`, `translucentPowder`, …)
    // auto-expand.
    ("ST_PresetMaterialType", "dkEdge", "DarkEdge"),
    ("ST_PresetMaterialType", "softmetal", "SoftMetal"),
];

// ---------------------------------------------------------------------------------------------
// WordprocessingML (`wml.xsd`) and Office Math (`shared-math.xsd`)
//
// An `ST_*` symbol is scoped to the schema that declares it, and OOXML reuses symbols across
// schemas with different meanings. `ST_Jc` is declared by **both** `wml.xsd` (twelve values, a
// paragraph's horizontal alignment) and `shared-math.xsd` (four, a math paragraph's), and
// `ST_Direction` by **both** `wml.xsd` (`ltr`/`rtl`) and `pml.xsd` (`horz`/`vert`, already emitted
// as `Orientation`). One flat table keyed on the bare symbol cannot express two meanings, and a
// row written for one schema would silently apply to the other.
//
// So the naming data is partitioned the way the symbols are: **one `NameEngine` per emitted
// module**, each with its own overrides. `naming.rs` is unchanged — the engine already takes its
// tables by reference, so adding a schema still means growing the tables.
// ---------------------------------------------------------------------------------------------

/// The naming engine for the WordprocessingML slice — `wml.xsd`, all 110 simple types.
pub const WORDPROCESSINGML_ENGINE: NameEngine = NameEngine {
    type_overrides: WORDPROCESSINGML_TYPE_OVERRIDES,
    variant_overrides: WORDPROCESSINGML_VARIANT_OVERRIDES,
    abbreviations: WORDPROCESSINGML_ABBREVIATIONS,
};

/// The naming engine for the Office Math slice — `shared-math.xsd`, all 14 simple types.
pub const OFFICEMATH_ENGINE: NameEngine = NameEngine {
    type_overrides: OFFICEMATH_TYPE_OVERRIDES,
    variant_overrides: OFFICEMATH_VARIANT_OVERRIDES,
    abbreviations: &[],
};

/// lowercase word → PascalCase expansion for WordprocessingML tokens.
///
/// Deliberately small: only the four abbreviations that recur across several types. Everything
/// else is an explicit variant override, so an expansion can never reach a token it was not
/// written for. `horz`/`vert`/`diag` carry `ST_Shd`'s stripe and cross patterns and
/// `ST_TblStyleOverrideType`'s banding; `chars` carries `ST_DocGrid`.
const WORDPROCESSINGML_ABBREVIATIONS: &[(&str, &str)] = &[
    ("chars", "Characters"),
    ("diag", "Diagonal"),
    ("horz", "Horizontal"),
    ("vert", "Vertical"),
];

/// `ST_*` → comprehensive Rust type name for `wml.xsd`, where the mechanical name is not
/// self-explanatory. Names are sourced from the ECMA-376 Part 1 §17.18 prose; the 32 types with no
/// row here already expand cleanly (`ST_HighlightColor`, `ST_NumberFormat`, `ST_ThemeColor`, …).
const WORDPROCESSINGML_TYPE_OVERRIDES: &[(&str, &str)] = &[
    ("ST_LongHexNumber", "EightDigitHexadecimalNumber"),
    ("ST_ShortHexNumber", "FourDigitHexadecimalNumber"),
    ("ST_UcharHexNumber", "TwoDigitHexadecimalNumber"),
    ("ST_DecimalNumberOrPercent", "DecimalNumberOrPercentage"),
    ("ST_HpsMeasure", "HalfPointMeasure"),
    ("ST_SignedHpsMeasure", "SignedHalfPointMeasure"),
    ("ST_TextScalePercent", "TextScalePercentage"),
    ("ST_MeasurementOrPercent", "MeasurementOrPercentage"),
    ("ST_HexColorAuto", "AutomaticColor"),
    ("ST_HexColor", "HexadecimalColor"),
    ("ST_Border", "BorderStyle"),
    ("ST_Shd", "ShadingPattern"),
    ("ST_Em", "EmphasisMark"),
    ("ST_Wrap", "TextFrameWrapping"),
    ("ST_VAnchor", "VerticalAnchor"),
    ("ST_HAnchor", "HorizontalAnchor"),
    ("ST_TabJc", "TabStopType"),
    ("ST_TabTlc", "TabStopLeader"),
    ("ST_Jc", "Justification"),
    ("ST_JcTable", "TableJustification"),
    ("ST_View", "DocumentView"),
    ("ST_Zoom", "ZoomPreset"),
    ("ST_Proof", "ProofingState"),
    ("ST_DocType", "DocumentClassification"),
    ("ST_DocProtect", "DocumentProtection"),
    ("ST_MailMergeDocType", "MailMergeDocumentType"),
    ("ST_MailMergeDest", "MailMergeDestination"),
    ("ST_MailMergeOdsoFMDFieldType", "MailMergeFieldMappingType"),
    ("ST_TextDirection", "TextFlowDirection"),
    ("ST_TextAlignment", "VerticalTextAlignment"),
    ("ST_AnnotationVMerge", "VerticalMergeRevision"),
    ("ST_TextboxTightWrap", "TextBoxTightWrap"),
    ("ST_FldCharType", "FieldCharacterType"),
    ("ST_InfoTextType", "HelpOrStatusTextType"),
    ("ST_FFHelpTextVal", "FormFieldHelpText"),
    ("ST_FFStatusTextVal", "FormFieldStatusText"),
    ("ST_FFName", "FormFieldName"),
    ("ST_FFTextType", "FormFieldTextType"),
    ("ST_SectionMark", "SectionBreakType"),
    ("ST_PageBorderZOrder", "PageBorderZOrder"),
    ("ST_ChapterSep", "ChapterSeparator"),
    ("ST_VerticalJc", "VerticalJustification"),
    ("ST_DocGrid", "DocumentGridType"),
    ("ST_HdrFtr", "HeaderFooterType"),
    ("ST_FtnEdn", "FootnoteEndnoteType"),
    ("ST_BrType", "BreakType"),
    ("ST_BrClear", "BreakTextWrappingRestart"),
    ("ST_PTabAlignment", "PositionalTabAlignment"),
    ("ST_PTabRelativeTo", "PositionalTabBase"),
    ("ST_PTabLeader", "PositionalTabLeader"),
    ("ST_ProofErr", "ProofingErrorType"),
    ("ST_EdGrp", "EditingGroup"),
    ("ST_Hint", "FontTypeHint"),
    ("ST_Theme", "ThemeFont"),
    ("ST_RubyAlign", "PhoneticGuideAlignment"),
    ("ST_Lock", "LockingType"),
    ("ST_SdtDateMappingType", "DateStorageFormat"),
    ("ST_Direction", "BidirectionalDirection"),
    ("ST_TblWidth", "TableWidthUnit"),
    ("ST_Merge", "MergedCellType"),
    ("ST_Cnf", "ConditionalFormattingBitmask"),
    ("ST_TblLayoutType", "TableLayoutType"),
    ("ST_TblOverlap", "TableOverlap"),
    ("ST_FtnPos", "FootnotePosition"),
    ("ST_EdnPos", "EndnotePosition"),
    ("ST_RestartNumber", "NumberingRestartLocation"),
    ("ST_TargetScreenSz", "TargetScreenSize"),
    ("ST_CharacterSpacing", "CharacterSpacingCompression"),
    ("ST_WmlColorSchemeIndex", "ColorSchemeSlot"),
    ("ST_StyleSort", "StyleSortMethod"),
    ("ST_FrameScrollbar", "FrameScrollbarVisibility"),
    ("ST_LevelSuffix", "NumberingLevelSuffix"),
    ("ST_TblStyleOverrideType", "TableStyleOverrideType"),
    ("ST_Pitch", "FontPitch"),
    ("ST_DocPartBehavior", "DocumentPartBehavior"),
    ("ST_DocPartType", "DocumentPartType"),
    ("ST_DocPartGallery", "DocumentPartGallery"),
    ("ST_CaptionPos", "CaptionPosition"),
];

/// (`ST_*`, wire value) → comprehensive Rust variant name for `wml.xsd`.
///
/// Every row is a token a reader cannot decode from its spelling, and every name comes from the
/// ECMA-376 prose (Part 1 §17.18, or Part 4 §14.11 for the Transitional-only additions) — never
/// from a guess. The 564 values with no row expand cleanly on their own.
const WORDPROCESSINGML_VARIANT_OVERRIDES: &[(&str, &str, &str)] = &[
    // `w:effect@val` (§17.18.87). The token says `ants`; the prose names the animation.
    ("ST_TextEffect", "antsBlack", "BlackDashedLine"),
    ("ST_TextEffect", "antsRed", "MarchingRedAnts"),
    ("ST_TextEffect", "blinkBackground", "BlinkingBackground"),
    ("ST_TextEffect", "lights", "ColoredLights"),
    ("ST_TextEffect", "sparkle", "SparklingLights"),
    // the two 3-D borders: the mechanical split lower-cases the `D`. Every other token of the 193,
    // the art borders included, is already self-describing English.
    ("ST_Border", "threeDEmboss", "ThreeDEmboss"),
    ("ST_Border", "threeDEngrave", "ThreeDEngrave"),
    // `w:shd@val` fill percentages (§17.18.78). `pct12`, `pct37`, `pct62` and `pct87` are 12.5%,
    // 37.5%, 62.5% and 87.5% — the token truncates the fraction, so the mechanical name would lie.
    ("ST_Shd", "pct5", "Percent5"),
    ("ST_Shd", "pct10", "Percent10"),
    ("ST_Shd", "pct12", "Percent12Point5"),
    ("ST_Shd", "pct15", "Percent15"),
    ("ST_Shd", "pct20", "Percent20"),
    ("ST_Shd", "pct25", "Percent25"),
    ("ST_Shd", "pct30", "Percent30"),
    ("ST_Shd", "pct35", "Percent35"),
    ("ST_Shd", "pct37", "Percent37Point5"),
    ("ST_Shd", "pct40", "Percent40"),
    ("ST_Shd", "pct45", "Percent45"),
    ("ST_Shd", "pct50", "Percent50"),
    ("ST_Shd", "pct55", "Percent55"),
    ("ST_Shd", "pct60", "Percent60"),
    ("ST_Shd", "pct62", "Percent62Point5"),
    ("ST_Shd", "pct65", "Percent65"),
    ("ST_Shd", "pct70", "Percent70"),
    ("ST_Shd", "pct75", "Percent75"),
    ("ST_Shd", "pct80", "Percent80"),
    ("ST_Shd", "pct85", "Percent85"),
    ("ST_Shd", "pct87", "Percent87Point5"),
    ("ST_Shd", "pct90", "Percent90"),
    ("ST_Shd", "pct95", "Percent95"),
    // `w:tab@val` (§17.18.84): `num` is the *list* tab, not a number.
    ("ST_TabJc", "num", "List"),
    // `w:jc@val` (§17.18.44). `left`/`right` are the Transitional spellings of `start`/`end`
    // (Part 4 §14.11.2) and keep their own variants, because the wire token must round-trip.
    ("ST_Jc", "both", "Justified"),
    ("ST_Jc", "numTab", "AlignToListTab"),
    ("ST_Jc", "highKashida", "WidestKashida"),
    ("ST_MailMergeOdsoFMDFieldType", "dbColumn", "DatabaseColumn"),
    // `w:textDirection@val` (§17.18.93). The first six are Part 1; the last six are the Transitional
    // aliases (Part 4 §14.11.7), each semantically equal to one of the first six but a distinct
    // wire token, so each keeps its own variant.
    ("ST_TextDirection", "tb", "TopToBottom"),
    ("ST_TextDirection", "rl", "RightToLeft"),
    ("ST_TextDirection", "lr", "LeftToRight"),
    ("ST_TextDirection", "tbV", "TopToBottomRotated"),
    ("ST_TextDirection", "rlV", "RightToLeftRotated"),
    ("ST_TextDirection", "lrV", "LeftToRightRotated"),
    ("ST_TextDirection", "btLr", "BottomToTopLeftToRight"),
    ("ST_TextDirection", "lrTb", "LeftToRightTopToBottom"),
    ("ST_TextDirection", "lrTbV", "LeftToRightTopToBottomRotated"),
    ("ST_TextDirection", "tbLrV", "TopToBottomLeftToRightRotated"),
    ("ST_TextDirection", "tbRl", "TopToBottomRightToLeft"),
    ("ST_TextDirection", "tbRlV", "TopToBottomRightToLeftRotated"),
    ("ST_DisplacedByCustomXml", "prev", "Previous"),
    // §17.18.1: `cont` is a vertically **merged** cell, `rest` a vertically **split** one.
    ("ST_AnnotationVMerge", "cont", "Merged"),
    ("ST_AnnotationVMerge", "rest", "Split"),
    // `w:numFmt@val` (§17.18.59), the largest curation in this schema. `aiueo`, `iroha`, `ganada`,
    // `chosung`, `chicago`, `bahtText` and the zodiac/legal ideograph formats name writing systems a
    // reader cannot infer from the token; `decimalEnclosedFullstop` is *followed by* a period rather
    // than enclosed by one, so the token itself is misleading.
    ("ST_NumberFormat", "hex", "Hexadecimal"),
    ("ST_NumberFormat", "chicago", "ChicagoManualOfStyle"),
    ("ST_NumberFormat", "aiueo", "HalfWidthKatakanaAiueo"),
    (
        "ST_NumberFormat",
        "aiueoFullWidth",
        "FullWidthKatakanaAiueo",
    ),
    ("ST_NumberFormat", "iroha", "KatakanaIroha"),
    (
        "ST_NumberFormat",
        "irohaFullWidth",
        "FullWidthKatakanaIroha",
    ),
    ("ST_NumberFormat", "ganada", "KoreanGanada"),
    ("ST_NumberFormat", "chosung", "KoreanChosung"),
    ("ST_NumberFormat", "bahtText", "ThaiBahtText"),
    ("ST_NumberFormat", "hebrew1", "HebrewLetters"),
    ("ST_NumberFormat", "hebrew2", "HebrewAlphabet"),
    ("ST_NumberFormat", "arabicAlpha", "ArabicAlphabet"),
    ("ST_NumberFormat", "arabicAbjad", "ArabicAbjadNumerals"),
    ("ST_NumberFormat", "upperLetter", "UppercaseLatinAlphabet"),
    ("ST_NumberFormat", "lowerLetter", "LowercaseLatinAlphabet"),
    ("ST_NumberFormat", "upperRoman", "UppercaseRomanNumerals"),
    ("ST_NumberFormat", "lowerRoman", "LowercaseRomanNumerals"),
    (
        "ST_NumberFormat",
        "russianUpper",
        "UppercaseRussianAlphabet",
    ),
    (
        "ST_NumberFormat",
        "russianLower",
        "LowercaseRussianAlphabet",
    ),
    (
        "ST_NumberFormat",
        "decimalFullWidth",
        "FullWidthArabicNumerals",
    ),
    (
        "ST_NumberFormat",
        "decimalHalfWidth",
        "HalfWidthArabicNumerals",
    ),
    (
        "ST_NumberFormat",
        "decimalZero",
        "InitialZeroArabicNumerals",
    ),
    (
        "ST_NumberFormat",
        "decimalFullWidth2",
        "FullWidthArabicNumeralsAlternate",
    ),
    (
        "ST_NumberFormat",
        "decimalEnclosedCircle",
        "DecimalEnclosedInCircle",
    ),
    (
        "ST_NumberFormat",
        "decimalEnclosedCircleChinese",
        "DecimalEnclosedInCircleChinese",
    ),
    (
        "ST_NumberFormat",
        "decimalEnclosedFullstop",
        "DecimalFollowedByPeriod",
    ),
    (
        "ST_NumberFormat",
        "decimalEnclosedParen",
        "DecimalEnclosedInParenthesis",
    ),
    (
        "ST_NumberFormat",
        "ideographEnclosedCircle",
        "IdeographEnclosedInCircle",
    ),
    (
        "ST_NumberFormat",
        "ideographTraditional",
        "TraditionalIdeograph",
    ),
    ("ST_NumberFormat", "ideographZodiac", "ZodiacIdeograph"),
    (
        "ST_NumberFormat",
        "ideographZodiacTraditional",
        "TraditionalZodiacIdeograph",
    ),
    (
        "ST_NumberFormat",
        "ideographLegalTraditional",
        "TraditionalLegalIdeograph",
    ),
    (
        "ST_NumberFormat",
        "koreanDigital2",
        "KoreanDigitalAlternate",
    ),
    ("ST_NumberFormat", "numberInDash", "NumberWithDashes"),
    ("ST_NumberFormat", "thaiNumbers", "ThaiNumerals"),
    (
        "ST_NumberFormat",
        "vietnameseCounting",
        "VietnameseNumerals",
    ),
    ("ST_VerticalJc", "both", "Justified"),
    ("ST_ProofErr", "spellStart", "SpellingStart"),
    ("ST_ProofErr", "spellEnd", "SpellingEnd"),
    ("ST_ProofErr", "gramStart", "GrammarStart"),
    ("ST_ProofErr", "gramEnd", "GrammarEnd"),
    // `w:rFonts@*Theme` (§17.18.90). `bidi` is the **complex script** slot; `hAnsi` is **high ANSI**,
    // which the mechanical split would render `Hansi`. `Ascii` is spelled out so the pair reads alike.
    ("ST_Theme", "majorAscii", "MajorAscii"),
    ("ST_Theme", "majorBidi", "MajorComplexScript"),
    ("ST_Theme", "majorHAnsi", "MajorHighAnsi"),
    ("ST_Theme", "minorAscii", "MinorAscii"),
    ("ST_Theme", "minorBidi", "MinorComplexScript"),
    ("ST_Theme", "minorHAnsi", "MinorHighAnsi"),
    // `w:lock@val` (§17.18.50): `sdt` is a structured document tag.
    ("ST_Lock", "sdtLocked", "TagCannotBeDeleted"),
    ("ST_Lock", "contentLocked", "ContentsCannotBeEdited"),
    (
        "ST_Lock",
        "sdtContentLocked",
        "ContentsCannotBeEditedAndTagCannotBeDeleted",
    ),
    ("ST_Direction", "ltr", "LeftToRight"),
    ("ST_Direction", "rtl", "RightToLeft"),
    // `w:tblW@type` (§17.18.91): `dxa` is twentieths of a point — this project's `Twips`.
    ("ST_TblWidth", "pct", "Percent"),
    ("ST_TblWidth", "dxa", "Twips"),
    ("ST_FtnPos", "sectEnd", "SectionEnd"),
    ("ST_FtnPos", "docEnd", "DocumentEnd"),
    ("ST_EdnPos", "sectEnd", "SectionEnd"),
    ("ST_EdnPos", "docEnd", "DocumentEnd"),
    ("ST_RestartNumber", "eachSect", "EachSection"),
    // `w:targetScreenSz@val` (§17.18.86): a pixel resolution. The mechanical name would be digit-leading.
    ("ST_TargetScreenSz", "544x376", "Pixels544By376"),
    ("ST_TargetScreenSz", "640x480", "Pixels640By480"),
    ("ST_TargetScreenSz", "720x512", "Pixels720By512"),
    ("ST_TargetScreenSz", "800x600", "Pixels800By600"),
    ("ST_TargetScreenSz", "1024x768", "Pixels1024By768"),
    ("ST_TargetScreenSz", "1152x882", "Pixels1152By882"),
    ("ST_TargetScreenSz", "1152x900", "Pixels1152By900"),
    ("ST_TargetScreenSz", "1280x1024", "Pixels1280By1024"),
    ("ST_TargetScreenSz", "1600x1200", "Pixels1600By1200"),
    ("ST_TargetScreenSz", "1800x1440", "Pixels1800By1440"),
    ("ST_TargetScreenSz", "1920x1200", "Pixels1920By1200"),
    // `w:stylePaneSortMethod@val` (§17.18.82). The six numeric tokens are Transitional-only aliases
    // of the six named ones (Part 4 §14.11.5) — `0000` is `name`, `0001` is `priority`, `0002` is
    // `default`, `0003` is `font`, `0004` is `basedOn`, `0005` is `type`. They keep separate
    // variants because the wire token must round-trip, and the mechanical name would be `N0000`.
    ("ST_StyleSort", "0000", "LegacyName"),
    ("ST_StyleSort", "0001", "LegacyPriority"),
    ("ST_StyleSort", "0002", "LegacyDefault"),
    ("ST_StyleSort", "0003", "LegacyFont"),
    ("ST_StyleSort", "0004", "LegacyBasedOn"),
    ("ST_StyleSort", "0005", "LegacyType"),
    ("ST_FrameLayout", "cols", "Columns"),
    // `w:tblStylePr@type` (§17.18.92). The compass tokens are corner cells: `ne` is top **right**,
    // `nw` top left, `se` bottom right, `sw` bottom left. The `band*Vert`/`band*Horz` values are
    // handled by the abbreviation table.
    ("ST_TblStyleOverrideType", "firstCol", "FirstColumn"),
    ("ST_TblStyleOverrideType", "lastCol", "LastColumn"),
    ("ST_TblStyleOverrideType", "neCell", "TopRightCell"),
    ("ST_TblStyleOverrideType", "nwCell", "TopLeftCell"),
    ("ST_TblStyleOverrideType", "seCell", "BottomRightCell"),
    ("ST_TblStyleOverrideType", "swCell", "BottomLeftCell"),
    // §17.18.15: `p` ensures the entry is in a new paragraph, `pg` on a new page.
    ("ST_DocPartBehavior", "p", "NewParagraph"),
    ("ST_DocPartBehavior", "pg", "NewPage"),
    // §17.18.17. Every name here comes from the prose: `speller` is an AutoCorrect entry and
    // `toolbar` an AutoText user-interface entry — neither is inferable from its token.
    ("ST_DocPartType", "autoExp", "ReplaceNameWithContent"),
    ("ST_DocPartType", "formFld", "FormFieldHelpText"),
    (
        "ST_DocPartType",
        "bbPlcHdr",
        "StructuredDocumentTagPlaceholderText",
    ),
    ("ST_DocPartType", "speller", "AutoCorrectEntry"),
    ("ST_DocPartType", "toolbar", "AutoTextUserInterfaceEntry"),
    // §17.18.16, the glossary-document galleries. `cust` is `Custom`, `pg`/`pgNum` page numbers,
    // `eq` equations, `ftrs`/`hdrs` footers and headers, `tbls` tables, `txtBox` a text box and `bib`
    // the bibliography. `any`, `default`, `placeholder`, `watermarks` and `custom1`–`custom5` need no
    // row.
    ("ST_DocPartGallery", "docParts", "DocumentParts"),
    ("ST_DocPartGallery", "coverPg", "CoverPage"),
    ("ST_DocPartGallery", "eq", "Equations"),
    ("ST_DocPartGallery", "ftrs", "Footers"),
    ("ST_DocPartGallery", "hdrs", "Headers"),
    ("ST_DocPartGallery", "pgNum", "PageNumbers"),
    ("ST_DocPartGallery", "tbls", "Tables"),
    ("ST_DocPartGallery", "autoTxt", "AutoText"),
    ("ST_DocPartGallery", "txtBox", "TextBox"),
    ("ST_DocPartGallery", "pgNumT", "PageNumbersAtTop"),
    ("ST_DocPartGallery", "pgNumB", "PageNumbersAtBottom"),
    ("ST_DocPartGallery", "pgNumMargins", "PageNumbersAtMargins"),
    ("ST_DocPartGallery", "tblOfContents", "TableOfContents"),
    ("ST_DocPartGallery", "bib", "Bibliography"),
    ("ST_DocPartGallery", "custQuickParts", "CustomQuickParts"),
    ("ST_DocPartGallery", "custCoverPg", "CustomCoverPage"),
    ("ST_DocPartGallery", "custEq", "CustomEquations"),
    ("ST_DocPartGallery", "custFtrs", "CustomFooters"),
    ("ST_DocPartGallery", "custHdrs", "CustomHeaders"),
    ("ST_DocPartGallery", "custPgNum", "CustomPageNumbers"),
    ("ST_DocPartGallery", "custTbls", "CustomTables"),
    ("ST_DocPartGallery", "custWatermarks", "CustomWatermarks"),
    ("ST_DocPartGallery", "custAutoTxt", "CustomAutoText"),
    ("ST_DocPartGallery", "custTxtBox", "CustomTextBox"),
    ("ST_DocPartGallery", "custPgNumT", "CustomPageNumbersAtTop"),
    (
        "ST_DocPartGallery",
        "custPgNumB",
        "CustomPageNumbersAtBottom",
    ),
    (
        "ST_DocPartGallery",
        "custPgNumMargins",
        "CustomPageNumbersAtMargins",
    ),
    (
        "ST_DocPartGallery",
        "custTblOfContents",
        "CustomTableOfContents",
    ),
    ("ST_DocPartGallery", "custBib", "CustomBibliography"),
];

/// `ST_*` → comprehensive Rust type name for `shared-math.xsd` (ECMA-376 Part 1 §22.1.3).
/// `ST_SpacingRule` is the one type whose mechanical name already reads correctly.
const OFFICEMATH_TYPE_OVERRIDES: &[(&str, &str)] = &[
    ("ST_Integer255", "Integer1To255"),
    ("ST_Integer2", "IntegerMinus2To2"),
    ("ST_UnSignedInteger", "UnsignedInteger"),
    ("ST_Char", "Character"),
    ("ST_Shp", "DelimiterShape"),
    ("ST_FType", "FractionType"),
    ("ST_LimLoc", "LimitLocation"),
    ("ST_TopBot", "TopBottom"),
    ("ST_Script", "ScriptType"),
    ("ST_Style", "MathStyle"),
    ("ST_Jc", "Justification"),
    ("ST_BreakBin", "BreakBinaryOperator"),
    ("ST_BreakBinSub", "BreakBinarySubtraction"),
];

/// (`ST_*`, wire value) → comprehensive Rust variant name for `shared-math.xsd`.
const OFFICEMATH_VARIANT_OVERRIDES: &[(&str, &str, &str)] = &[
    // §22.1.3.10: the delimiter is either centred on its argument or matched to the argument's shape.
    ("ST_Shp", "match", "MatchArgument"),
    // §22.1.3.4. `noBar` — the stack object — already reads as itself.
    ("ST_FType", "skw", "Skewed"),
    ("ST_FType", "lin", "Linear"),
    // §22.1.3.8: limits sit above and below the base, or beside it as sub/superscripts.
    ("ST_LimLoc", "undOvr", "UnderOver"),
    ("ST_LimLoc", "subSup", "SubscriptSuperscript"),
    ("ST_TopBot", "bot", "Bottom"),
    // §22.1.3.12: a single-letter token per math style.
    ("ST_Style", "p", "Plain"),
    ("ST_Style", "b", "Bold"),
    ("ST_Style", "i", "Italic"),
    ("ST_Style", "bi", "BoldItalic"),
    // §22.1.3.7: `centerGroup` centres the math paragraph as a group rather than each instance.
    ("ST_Jc", "centerGroup", "CenteredAsGroup"),
    // §22.1.3.2: the tokens are punctuation pairs. Without these rows all three would sanitize to
    // the *same* identifier, because the word splitter finds no alphanumerics in any of them.
    ("ST_BreakBinSub", "--", "MinusMinus"),
    ("ST_BreakBinSub", "-+", "MinusPlus"),
    ("ST_BreakBinSub", "+-", "PlusMinus"),
];

// ---------------------------------------------------------------------------------------------
// SpreadsheetML (`sml.xsd`)
//
// A fourth engine, for the same reason the two above exist: `sml.xsd` redeclares `ST_FontFamily`
// (which `wml.xsd` also declares, as an enumeration of family *names*, while Excel's is a numeric
// family code) and `ST_Orientation` (which `dml-chart.xsd` also declares). A row written for one
// schema must not reach the other, and the already-emitted `wordprocessingml::FontFamily` must not
// move because a later schema arrived.
// ---------------------------------------------------------------------------------------------

/// The naming engine for the SpreadsheetML slice — `sml.xsd`, all 96 simple types.
pub const SPREADSHEETML_ENGINE: NameEngine = NameEngine {
    type_overrides: SPREADSHEETML_TYPE_OVERRIDES,
    variant_overrides: SPREADSHEETML_VARIANT_OVERRIDES,
    abbreviations: SPREADSHEETML_ABBREVIATIONS,
};

/// lowercase word → PascalCase expansion for SpreadsheetML tokens.
///
/// Deliberately small, and every row is used by more than one value or type. `col` carries
/// `ST_rwColActionType` and `ST_ShowDataAs`; `max`/`min` carry the four aggregate-function
/// families; `ref` carries `ST_FormulaExpression`; `avg` carries `ST_ItemType`. Everything else is
/// an explicit variant override, so an expansion can never reach a token it was not written for.
const SPREADSHEETML_ABBREVIATIONS: &[(&str, &str)] = &[
    ("avg", "Average"),
    ("col", "Column"),
    ("max", "Maximum"),
    ("min", "Minimum"),
    ("ref", "Reference"),
];

/// `ST_*` → comprehensive Rust type name for `sml.xsd`, where the mechanical name is not
/// self-explanatory. Names are sourced from the ECMA-376 Part 1 §18.18 prose (§18.18.N is given for
/// each), or from Part 4 §15.8 for the two types Transitional adds. The 57 types with no row here
/// already expand cleanly (`ST_BorderStyle`, `ST_PatternType`, `ST_TableStyleType`, …).
const SPREADSHEETML_TYPE_OVERRIDES: &[(&str, &str)] = &[
    // Cell and range references (§18.18.7, §18.18.62, §18.18.63, §18.18.76). Four distinct kinds
    // of reference whose symbols differ by one letter; the prose titles keep them apart.
    ("ST_CellRef", "CellReference"),
    ("ST_Ref", "CellRangeReference"),
    ("ST_RefA", "SingleCellReference"),
    ("ST_Sqref", "ReferenceSequence"),
    // §18.18.86 and Part 4 §15.8.2: the prose states the digit count, and `hexBinary`'s `length`
    // facet is in octets, so `length="4"` is eight digits and `length="2"` is four. Named the way
    // `wordprocessingml` names the same shape.
    ("ST_UnsignedIntHex", "EightDigitHexadecimalNumber"),
    ("ST_UnsignedShortHex", "FourDigitHexadecimalNumber"),
    // §18.18.80 / §18.18.81. The mechanical names would be `TextHalign`/`TextValign`: the word
    // splitter cannot see `HAlign` as two words, so the H and V vanish into lowercase.
    ("ST_TextHAlign", "CommentTextHorizontalAlignment"),
    ("ST_TextVAlign", "CommentTextVerticalAlignment"),
    ("ST_CredMethod", "CredentialsMethod"),   // §18.18.16
    ("ST_HtmlFmt", "HtmlFormattingHandling"), // §18.18.41
    // §18.18.27. The symbol says "external connection", but the type is the datatype a *text
    // import field* is parsed as — the prose title is "Text Field Datatype" and the values are
    // date component orders.
    ("ST_ExternalConnectionType", "TextFieldDataType"),
    ("ST_SourceType", "PivotCacheSourceType"), // §18.18.75 "PivotCache Type"
    ("ST_Scope", "ConditionalFormattingScope"), // §18.18.67
    ("ST_Type", "TopNEvaluationType"),         // §18.18.84; a bare `Type` names nothing
    ("ST_ItemType", "PivotItemType"),          // §18.18.43
    ("ST_FormatAction", "PivotTableFormatAction"), // §18.18.34
    ("ST_Axis", "PivotTableAxis"),             // §18.18.1
    ("ST_rwColActionType", "RowColumnActionType"), // §18.18.66; the symbol is not even PascalCase
    ("ST_CfType", "ConditionalFormatType"),    // §18.18.12
    ("ST_CfvoType", "ConditionalFormatValueObjectType"), // §18.18.13
    ("ST_DvAspect", "DataViewAspect"),         // §18.18.24
    ("ST_OleUpdate", "OleUpdateType"),         // §18.18.49
    // §18.18.45. The mechanical name is `MdxKpiproperty` — the splitter cannot break `KPIProperty`
    // apart, and KPI is not a word a reader of this crate should have to know.
    ("ST_MdxKPIProperty", "MdxKeyPerformanceIndicatorProperty"),
    ("ST_NumFmtId", "NumberFormatId"),         // §18.18.47
    ("ST_CellStyleXfId", "CellStyleFormatId"), // §18.18.10 "Cell Style Format Id"
    // §18.18.25 with §18.8.15: `dxf` is a *differential* formatting record.
    ("ST_DxfId", "DifferentialFormatId"),
    ("ST_UnderlineValues", "UnderlineType"), // §18.18.85; `Values` is noise
    // §18.18.94. Excel's `ST_FontFamily` is a *number* (0 = not applicable, 1 = Roman … 5 =
    // Decorative, 6–14 reserved), not `wml`'s enumeration of the same name.
    ("ST_FontFamily", "FontFamilyNumber"),
    ("ST_DdeValueType", "DynamicDataExchangeValueType"), // §18.18.23
    ("ST_VolDepType", "VolatileDependencyType"),         // §18.18.90
    ("ST_VolValueType", "VolatileDependencyValueType"),  // §18.18.91
    ("ST_Comments", "CommentDisplay"),                   // §18.18.14 "Comment Display Types"
    ("ST_Objects", "ObjectDisplay"),                     // §18.18.48 "Object Display Types"
    ("ST_SmartTagShow", "SmartTagDisplay"),              // §18.18.71 "Smart Tag Display Types"
    ("ST_UpdateLinks", "UpdateLinksBehavior"),           // §18.18.87
    ("ST_CalcMode", "CalculationMode"),                  // §18.18.4
    ("ST_RefMode", "ReferenceMode"),                     // §18.18.64
    // §18.18.50: "Print orientation for this sheet." Naming the print intent also keeps this type
    // distinct from `dml-chart`'s `ST_Orientation`, which is an axis direction.
    ("ST_Orientation", "PrintOrientation"),
    // §18.18.61: the qualifier that denotes string data when text is imported from a file.
    ("ST_Qualifier", "TextQualifier"),
];

/// (`ST_*`, wire value) → comprehensive Rust variant name for `sml.xsd`.
///
/// Seeded from the tokens a reader cannot decode: single letters, digit-leading icon-set and screen
/// resolution names, OLE constants, and the abbreviated statistical aggregates. Every name is
/// sourced from the ECMA-376 Part 1 §18.18 prose — either the value's own "friendly name" or, where
/// that is wrong or useless, its Description column, noted at the row.
const SPREADSHEETML_VARIANT_OVERRIDES: &[(&str, &str, &str)] = &[
    // §18.18.11: the archetypal cryptic family. `s` is a *shared* string (an index into the shared
    // string table) and `str` is the string *result of a formula* — neither is inferable.
    ("ST_CellType", "b", "Boolean"),
    ("ST_CellType", "n", "Number"),
    ("ST_CellType", "e", "Error"),
    ("ST_CellType", "s", "SharedString"),
    ("ST_CellType", "str", "FormulaString"),
    ("ST_CellType", "inlineStr", "InlineString"),
    // §18.18.23: the same letters again, with different meanings — `n` is a real number here.
    ("ST_DdeValueType", "b", "Boolean"),
    ("ST_DdeValueType", "n", "RealNumber"),
    ("ST_DdeValueType", "e", "Error"),
    ("ST_DdeValueType", "str", "String"),
    // §18.18.91.
    ("ST_VolValueType", "b", "Boolean"),
    ("ST_VolValueType", "n", "RealNumber"),
    ("ST_VolValueType", "e", "Error"),
    ("ST_VolValueType", "s", "String"),
    // §18.18.44: one letter per MDX cube function.
    ("ST_MdxFunctionType", "m", "CubeMember"),
    ("ST_MdxFunctionType", "v", "CubeValue"),
    ("ST_MdxFunctionType", "s", "CubeSet"),
    ("ST_MdxFunctionType", "c", "CubeSetCount"),
    ("ST_MdxFunctionType", "r", "CubeRankedMember"),
    ("ST_MdxFunctionType", "p", "CubeMemberProperty"),
    (
        "ST_MdxFunctionType",
        "k",
        "CubeKeyPerformanceIndicatorMember",
    ),
    // §18.18.46.
    ("ST_MdxSetOrder", "u", "Unsorted"),
    ("ST_MdxSetOrder", "a", "Ascending"),
    ("ST_MdxSetOrder", "d", "Descending"),
    ("ST_MdxSetOrder", "aa", "AlphabeticAscending"),
    ("ST_MdxSetOrder", "ad", "AlphabeticDescending"),
    ("ST_MdxSetOrder", "na", "NaturalAscending"),
    ("ST_MdxSetOrder", "nd", "NaturalDescending"),
    // §18.18.45.
    ("ST_MdxKPIProperty", "v", "Value"),
    ("ST_MdxKPIProperty", "g", "Goal"),
    ("ST_MdxKPIProperty", "s", "Status"),
    ("ST_MdxKPIProperty", "t", "Trend"),
    ("ST_MdxKPIProperty", "w", "Weight"),
    ("ST_MdxKPIProperty", "m", "CurrentTimeMember"),
    // §18.18.42: every token is digit-leading, so every mechanical name would be `N3Arrows` and
    // friends. The trailing digits are variants of one icon set, and the prose names them.
    ("ST_IconSetType", "3Arrows", "ThreeArrows"),
    ("ST_IconSetType", "3ArrowsGray", "ThreeArrowsGray"),
    ("ST_IconSetType", "3Flags", "ThreeFlags"),
    ("ST_IconSetType", "3TrafficLights1", "ThreeTrafficLights"),
    (
        "ST_IconSetType",
        "3TrafficLights2",
        "ThreeTrafficLightsBlack",
    ),
    ("ST_IconSetType", "3Signs", "ThreeSigns"),
    ("ST_IconSetType", "3Symbols", "ThreeSymbolsCircled"),
    ("ST_IconSetType", "3Symbols2", "ThreeSymbols"),
    ("ST_IconSetType", "4Arrows", "FourArrows"),
    ("ST_IconSetType", "4ArrowsGray", "FourArrowsGray"),
    ("ST_IconSetType", "4RedToBlack", "FourRedToBlack"),
    ("ST_IconSetType", "4Rating", "FourRatings"),
    ("ST_IconSetType", "4TrafficLights", "FourTrafficLights"),
    ("ST_IconSetType", "5Arrows", "FiveArrows"),
    ("ST_IconSetType", "5ArrowsGray", "FiveArrowsGray"),
    ("ST_IconSetType", "5Rating", "FiveRatings"),
    ("ST_IconSetType", "5Quarters", "FiveQuarters"),
    // §18.18.79: pixel resolutions. Mechanical names would be `N544X376`.
    ("ST_TargetScreenSize", "544x376", "Resolution544By376"),
    ("ST_TargetScreenSize", "640x480", "Resolution640By480"),
    ("ST_TargetScreenSize", "720x512", "Resolution720By512"),
    ("ST_TargetScreenSize", "800x600", "Resolution800By600"),
    ("ST_TargetScreenSize", "1024x768", "Resolution1024By768"),
    ("ST_TargetScreenSize", "1152x882", "Resolution1152By882"),
    ("ST_TargetScreenSize", "1152x900", "Resolution1152By900"),
    ("ST_TargetScreenSize", "1280x1024", "Resolution1280By1024"),
    ("ST_TargetScreenSize", "1600x1200", "Resolution1600By1200"),
    ("ST_TargetScreenSize", "1800x1440", "Resolution1800By1440"),
    ("ST_TargetScreenSize", "1920x1200", "Resolution1920By1200"),
    // §18.18.26 and §18.18.59 both carry the calendar-quarter and calendar-month filters. The
    // friendly names are ordinals ("1st Month"), but each Description says which month it is —
    // "Shows the dates that are in January, regardless of year".
    ("ST_DynamicFilterType", "Q1", "FirstQuarter"),
    ("ST_DynamicFilterType", "Q2", "SecondQuarter"),
    ("ST_DynamicFilterType", "Q3", "ThirdQuarter"),
    ("ST_DynamicFilterType", "Q4", "FourthQuarter"),
    ("ST_DynamicFilterType", "M1", "January"),
    ("ST_DynamicFilterType", "M2", "February"),
    ("ST_DynamicFilterType", "M3", "March"),
    ("ST_DynamicFilterType", "M4", "April"),
    ("ST_DynamicFilterType", "M5", "May"),
    ("ST_DynamicFilterType", "M6", "June"),
    ("ST_DynamicFilterType", "M7", "July"),
    ("ST_DynamicFilterType", "M8", "August"),
    ("ST_DynamicFilterType", "M9", "September"),
    ("ST_DynamicFilterType", "M10", "October"),
    ("ST_DynamicFilterType", "M11", "November"),
    ("ST_DynamicFilterType", "M12", "December"),
    ("ST_PivotFilterType", "Q1", "FirstQuarter"),
    ("ST_PivotFilterType", "Q2", "SecondQuarter"),
    ("ST_PivotFilterType", "Q3", "ThirdQuarter"),
    ("ST_PivotFilterType", "Q4", "FourthQuarter"),
    ("ST_PivotFilterType", "M1", "January"),
    ("ST_PivotFilterType", "M2", "February"),
    ("ST_PivotFilterType", "M3", "March"),
    ("ST_PivotFilterType", "M4", "April"),
    ("ST_PivotFilterType", "M5", "May"),
    ("ST_PivotFilterType", "M6", "June"),
    ("ST_PivotFilterType", "M7", "July"),
    ("ST_PivotFilterType", "M8", "August"),
    ("ST_PivotFilterType", "M9", "September"),
    ("ST_PivotFilterType", "M10", "October"),
    ("ST_PivotFilterType", "M11", "November"),
    ("ST_PivotFilterType", "M12", "December"),
    // §18.18.27: date component orders. The friendly name printed for `MYD` is "Month Day Year",
    // which is the same as `MDY`'s and is an error in the published table — its Description says
    // "day, month, year" order for `DMY` and "month, year, day" for `MYD`, and the Descriptions
    // are what these rows follow. Naming both from the friendly-name column would have collapsed
    // two wire tokens onto one variant, which the generator refuses.
    ("ST_ExternalConnectionType", "MDY", "MonthDayYear"),
    ("ST_ExternalConnectionType", "DMY", "DayMonthYear"),
    ("ST_ExternalConnectionType", "YMD", "YearMonthDay"),
    ("ST_ExternalConnectionType", "MYD", "MonthYearDay"),
    ("ST_ExternalConnectionType", "DYM", "DayYearMonth"),
    ("ST_ExternalConnectionType", "YDM", "YearDayMonth"),
    ("ST_ExternalConnectionType", "EMD", "EastAsianYearMonthDay"),
    // §18.18.24 / §18.18.49: OLE constants carried verbatim into the markup. The prefix is the
    // type's name repeated, so the variant keeps only what distinguishes it.
    ("ST_DvAspect", "DVASPECT_CONTENT", "Content"),
    ("ST_DvAspect", "DVASPECT_ICON", "Icon"),
    ("ST_OleUpdate", "OLEUPDATE_ALWAYS", "Always"),
    ("ST_OleUpdate", "OLEUPDATE_ONCALL", "OnCall"),
    // §18.18.1: the `axis` prefix repeats the type. The published friendly name for `axisPage` is
    // "Include Count Filter", which belongs to a different table entirely — its Description is
    // "Page axis", and that is what this row follows.
    ("ST_Axis", "axisRow", "Row"),
    ("ST_Axis", "axisCol", "Column"),
    ("ST_Axis", "axisPage", "Page"),
    ("ST_Axis", "axisValues", "Values"),
    // §18.18.43. `countA` is Excel's COUNTA — a count of non-empty cells; §18.18.17 states the
    // equivalence ("the Count consolidation function works the same as the COUNTA worksheet
    // function") and §18.18.83 names it "Non Empty Cell Count".
    ("ST_ItemType", "countA", "CountNonEmpty"),
    ("ST_ItemType", "stdDev", "StandardDeviation"),
    ("ST_ItemType", "stdDevP", "PopulationStandardDeviation"),
    ("ST_ItemType", "var", "Variance"),
    ("ST_ItemType", "varP", "PopulationVariance"),
    ("ST_ItemType", "grand", "GrandTotal"),
    // §18.18.17: here `count` is the COUNTA-equivalent and `countNums` the COUNT-equivalent, and
    // the `p` suffix marks the whole-population form rather than the sample estimate.
    ("ST_DataConsolidateFunction", "count", "CountNonEmpty"),
    ("ST_DataConsolidateFunction", "countNums", "CountNumbers"),
    (
        "ST_DataConsolidateFunction",
        "stdDev",
        "SampleStandardDeviation",
    ),
    (
        "ST_DataConsolidateFunction",
        "stdDevp",
        "PopulationStandardDeviation",
    ),
    ("ST_DataConsolidateFunction", "var", "SampleVariance"),
    ("ST_DataConsolidateFunction", "varp", "PopulationVariance"),
    // §18.18.83.
    ("ST_TotalsRowFunction", "count", "CountNonEmpty"),
    ("ST_TotalsRowFunction", "countNums", "CountNumbers"),
    (
        "ST_TotalsRowFunction",
        "stdDev",
        "EstimatedStandardDeviation",
    ),
    ("ST_TotalsRowFunction", "var", "EstimatedVariance"),
    ("ST_TotalsRowFunction", "custom", "CustomFormula"),
    // §18.18.14: the `comm` prefix is the type's own name abbreviated.
    ("ST_Comments", "commNone", "NoComments"),
    ("ST_Comments", "commIndicator", "IndicatorOnly"),
    ("ST_Comments", "commIndAndComment", "IndicatorAndComment"),
    // §18.18.55: the trailing digits are a fraction, not a count — 0.125 and 0.0625 grey.
    ("ST_PatternType", "gray125", "Gray12Point5Percent"),
    ("ST_PatternType", "gray0625", "Gray6Point25Percent"),
    // §18.18.13: `num` is a literal number, as against `percent`/`percentile`/`formula`.
    ("ST_CfvoType", "num", "Number"),
    // §18.18.60: `#N/A`, the not-available error value.
    ("ST_PrintError", "NA", "NotAvailable"),
    // §18.18.19: every one of these six is a full-width or half-width input mode; `alpha` is the
    // alphanumeric mode.
    (
        "ST_DataValidationImeMode",
        "fullKatakana",
        "FullWidthKatakana",
    ),
    (
        "ST_DataValidationImeMode",
        "halfKatakana",
        "HalfWidthKatakana",
    ),
    (
        "ST_DataValidationImeMode",
        "fullAlpha",
        "FullWidthAlphanumeric",
    ),
    (
        "ST_DataValidationImeMode",
        "halfAlpha",
        "HalfWidthAlphanumeric",
    ),
    ("ST_DataValidationImeMode", "fullHangul", "FullWidthHangul"),
    ("ST_DataValidationImeMode", "halfHangul", "HalfWidthHangul"),
    // §18.18.21: a validation against a whole number, as against `decimal`.
    ("ST_DataValidationType", "whole", "WholeNumber"),
    // §18.18.70.
    ("ST_ShowDataAs", "percentDiff", "PercentageDifference"),
    ("ST_ShowDataAs", "runTotal", "RunningTotal"),
    // §18.18.67: a bare `Data`/`Field` says nothing about what the scope covers.
    ("ST_Scope", "data", "DataFields"),
    ("ST_Scope", "field", "FieldIntersections"),
    // §18.18.29: the text-import file's platform.
    ("ST_FileType", "mac", "Macintosh"),
    ("ST_FileType", "win", "Windows"),
    ("ST_FileType", "lin", "Linux"),
    // §18.18.41: `rtf` here means "honour the imported rich text", not the RTF file format.
    ("ST_HtmlFmt", "rtf", "RichText"),
    // §18.18.54: the parameter's value is asked for each time the query refreshes.
    ("ST_ParameterType", "prompt", "PromptOnRefresh"),
];

// ---------------------------------------------------------------------------------------------
// DrawingML Diagram (`dml-diagram.xsd`, the `dgm:` namespace — SmartArt)
//
// `dml-diagram.xsd` redeclares three symbols the schemas above already use with different
// meanings: `ST_Direction` (`dml-diagram`'s is `norm`/`rev`, a traversal-order flag; `pml`'s
// `horz`/`vert` is already emitted as `Orientation`, `wml`'s `ltr`/`rtl` as
// `BidirectionalDirection`), `ST_TextDirection` (`dml-diagram`'s is `fromT`/`fromB`; `wml`'s
// `tb`/`rl`/`lr`/… is already emitted as `TextFlowDirection`), and `ST_VerticalAlignment`
// (`dml-diagram`'s is `t`/`mid`/`b`/`none`; `sml`'s `top`/`center`/`bottom`/`justify`/`distributed`
// is already emitted as `CommentTextVerticalAlignment`). Each gets its own name below rather than
// reusing or moving an already-committed one.
// ---------------------------------------------------------------------------------------------

/// The naming engine for the DrawingML Diagram slice — `dml-diagram.xsd`, all 66 simple types.
pub const DIAGRAM_ENGINE: NameEngine = NameEngine {
    type_overrides: DIAGRAM_TYPE_OVERRIDES,
    variant_overrides: DIAGRAM_VARIANT_OVERRIDES,
    abbreviations: DIAGRAM_ABBREVIATIONS,
};

/// lowercase word → PascalCase expansion for DrawingML Diagram tokens.
///
/// `dml-diagram.xsd`'s vocabulary is unusually short-hand (single letters, `Ctr`/`Marg`/`Sz`-style
/// contractions), so this table is larger than the other engines'. Every entry is a fragment with
/// **one consistent meaning everywhere it appears in this schema** — checked by hand against every
/// enumeration in ECMA-376 Part 1 §21.4.7 before being added; a fragment whose meaning depends on
/// context (`rev`, `sp`) is left out of this table and given a per-value override instead, below.
const DIAGRAM_ABBREVIATIONS: &[(&str, &str)] = &[
    ("alg", "Algorithm"),
    ("hier", "Hierarchy"),
    ("horz", "Horizontal"),
    ("vert", "Vertical"),
    ("ctr", "Center"),
    ("mid", "Middle"),
    // Single-letter positional codes — §21.4.7.13 `ST_ChildAlignment` and consistent across every
    // other enumeration that reuses them (`ST_ConstraintType`, `ST_HierarchyAlignment`, …).
    ("t", "Top"),
    ("b", "Bottom"),
    ("l", "Left"),
    ("r", "Right"),
    ("h", "Height"),
    ("w", "Width"),
    // §21.4.7.6 `ST_AxisType` — reused by `ST_ConstraintRelationship` (§21.4.7.20) with the same
    // meaning.
    ("ch", "Child"),
    ("des", "Descendant"),
    ("par", "Parent"),
    ("ancst", "Ancestor"),
    ("preced", "Preceding"),
    // `parTrans`/`sibTrans` (§21.4.7.51 `ST_PtType`) — the transition point between two data
    // points.
    ("trans", "Transition"),
    ("sib", "Sibling"),
    // §21.4.7.23 `ST_CxnType`'s `presOf`/`presParOf`, §21.4.7.51 `ST_PtType`'s `pres`.
    ("pres", "Presentation"),
    ("doc", "Document"),
    ("asst", "Assistant"),
    // §21.4.7.26 `ST_Direction`'s `norm`, reused by `ST_ElementType`'s `norm`/`nonNorm`.
    ("norm", "Normal"),
    // `ST_ContinueDirection`'s `revDir` (§21.4.7.22) and `ST_FunctionType`'s `revPos`
    // (§21.4.7.33) both mean "reverse of"; `ST_Direction`'s bare `rev` (§21.4.7.26) means
    // "Reversed" as a whole word instead, so that one value is overridden below.
    ("rev", "Reverse"),
    // The constraint/rule attribute vocabulary, §21.4.7.21 `ST_ConstraintType`.
    ("marg", "Margin"),
    ("pad", "Padding"),
    ("dist", "Distance"),
    ("diam", "Diameter"),
    ("prim", "Primary"),
    ("sec", "Secondary"),
    ("sz", "Size"),
    ("off", "Offset"),
    ("beg", "Beginning"),
    // §21.4.7.49 `ST_ParameterId`'s alignment parameters (`horzAlign`, `chAlign`, …) and
    // §21.4.7.13/§21.4.7.56 `ST_ChildAlignment`/`ST_SecondaryChildAlignment`.
    ("align", "Alignment"),
    ("cust", "Custom"),
    // §21.4.6.6 `dir` (Diagram Direction) and its many parameter-id compounds (`linDir`,
    // `flowDir`, `contDir`, `txDir`, `txBlDir`, …).
    ("dir", "Direction"),
    ("cont", "Continue"),
    ("gr", "Grow"),
    ("bl", "Block"),
    ("tx", "Text"),
    ("shp", "Shape"),
    // §21.4.7.35 `ST_GrowDirection` reuses `pyra`/`acct` from the pyramid algorithm parameters.
    ("pyra", "Pyramid"),
    ("acct", "Accent"),
    // Comparison operators shared by §21.4.7.10 `ST_BoolOperator` and §21.4.7.32
    // `ST_FunctionOperator`.
    ("equ", "Equal"),
    ("neq", "NotEqual"),
    ("gt", "GreaterThan"),
    ("lt", "LessThan"),
    ("gte", "GreaterThanOrEqual"),
    ("lte", "LessThanOrEqual"),
    // §21.4.7.33 `ST_FunctionType`.
    ("cnt", "Count"),
    ("pos", "Position"),
    ("var", "Variable"),
    // §21.4.7.38 `ST_HueDir`.
    ("cw", "Clockwise"),
    ("ccw", "CounterClockwise"),
    // §21.4.7.37 `ST_HierBranchStyle`.
    ("std", "Standard"),
    ("init", "Initial"),
    // §21.4.7.30 `ST_FlowDirection`.
    ("col", "Column"),
    // §21.4.7.54 `ST_ResizeHandlesStr`.
    ("rel", "Relative"),
    // §21.4.7.52 `ST_PyramidAccentPosition`.
    ("bef", "Before"),
    ("aft", "After"),
    // §21.4.7.8 `ST_BendPoint`.
    ("def", "Default"),
    // §21.4.7.11 `ST_Breakpoint`.
    ("cnv", "Canvas"),
    ("bal", "Balanced"),
    // §21.4.7.1 `ST_AlgorithmType`'s `lin` (Linear Algorithm) and §21.4.7.42/.57
    // `ST_LinearDirection`/`ST_SecondaryLinearDirection`'s `linDir`/`secLinDir` parameter ids.
    ("lin", "Linear"),
    // §21.4.3.4 `prSet`'s `loTypeId`/`qsTypeId`/`csTypeId` families and §21.4.6.8 `orgChart` share
    // this fragment; `chMax`/`chPref` (§21.4.6.4/.5) need `pref` too.
    ("org", "Organization"),
    ("pref", "Preference"),
    ("bul", "Bullets"),
    ("lvl", "Level"),
    // `ST_ParameterVal`/`ST_FunctionValue`'s trailing `Val`.
    ("val", "Value"),
];

/// `ST_*` → comprehensive Rust type name for `dml-diagram.xsd`, where the mechanical name is not
/// self-explanatory, or where the mechanical name would collide with an already-committed type
/// from another schema. Names are sourced from the ECMA-376 Part 1 §21.4 prose.
const DIAGRAM_TYPE_OVERRIDES: &[(&str, &str)] = &[
    // Collisions with already-emitted types of the same bare `ST_*` symbol — see the module note
    // above. `dgm:dir@val` (norm/rev) selects the order layout children are traversed in.
    ("ST_Direction", "TraversalDirection"),
    // `dgm:param[@type='txDir']@val` (fromT/fromB) — which end of the shape text starts from.
    ("ST_TextDirection", "DiagramTextFlowOrigin"),
    // `dgm:param[@type='vertAlign']@val` (t/mid/b/none) — the whole-diagram vertical alignment
    // parameter; distinct from the per-node `ST_NodeVerticalAlignment` (t/mid/b, no `none`).
    ("ST_VerticalAlignment", "LayoutVerticalAlignment"),
    // §21.4.7.3/.2/.54 name these types by their wire-level `…Str` suffix ("a string to display in
    // the UI"); the suffix is an implementation detail of the schema, not part of the concept.
    ("ST_AnimOneStr", "OneByOneAnimation"),
    ("ST_AnimLvlStr", "LevelAnimation"),
    ("ST_ResizeHandlesStr", "ResizeHandleBehavior"),
    // §21.4.7.66 "Property Set Customized Value" — `pr` does not appear as a fragment anywhere
    // else in this schema's *type symbols* (only in the `prSet` *element* name, which this table
    // does not name), so it is spelled out here rather than risking a broad `pr` → `Property`
    // abbreviation nothing else needs.
    ("ST_PrSetCustVal", "PropertySetCustomValue"),
    // §21.4.7.51 `ST_PtType` — `Pt` is short for `dgm:pt`'s own name (Point); the bare mechanical
    // name `PtType` keeps the abbreviation the naming convention says to expand. This is the `type`
    // attribute of a point, so "the kind of point this is" — `PointType`.
    ("ST_PtType", "PointType"),
    // §21.4.7.23 `ST_CxnType` — likewise `Cxn` is short for `dgm:cxn`'s own name (Connection); this
    // is the `type` attribute of a connection, so "the kind of connection this is" —
    // `ConnectionType`.
    ("ST_CxnType", "ConnectionType"),
];

/// (`ST_*` type, wire value) → Rust variant name for `dml-diagram.xsd`, where the mechanical name
/// is not self-explanatory, or the abbreviation table above cannot apply because the fragment's
/// meaning is not consistent across every type that uses it. Sourced from the ECMA-376 Part 1
/// §21.4.7 prose tables; `ST_ConstraintType` and `ST_ParameterId` are large and idiosyncratic enough
/// that every value is written out explicitly here rather than left to the abbreviation cascade, so
/// each one is directly traceable to its prose entry rather than to an interaction between table
/// rows.
const DIAGRAM_VARIANT_OVERRIDES: &[(&str, &str, &str)] = &[
    // §21.4.7.26 `ST_Direction` — `norm` auto-resolves to `Normal` via the abbreviation table;
    // `rev` needs the adjective form "Reversed" here, where `ST_ContinueDirection`'s `revDir` and
    // `ST_FunctionType`'s `revPos` both want the prefix form "Reverse" the abbreviation gives.
    ("ST_Direction", "rev", "Reversed"),
    // §21.4.7.37 `ST_HierBranchStyle` — "Hanging" (adjective) rather than the abbreviation
    // cascade's bare "Hang".
    ("ST_HierBranchStyle", "hang", "Hanging"),
    // §21.4.7.4 `ST_ArrowheadStyle`.
    ("ST_ArrowheadStyle", "arr", "ArrowheadPresent"),
    ("ST_ArrowheadStyle", "noArr", "NoArrowhead"),
    // §21.4.7.17 `ST_ConnectorDimension` and §21.4.7.29 `ST_FallbackDimension` both restrict to
    // `1D`/`2D`, a digit-leading token `sanitize_ident` would otherwise prefix with `N`.
    ("ST_ConnectorDimension", "1D", "OneDimension"),
    ("ST_ConnectorDimension", "2D", "TwoDimensions"),
    ("ST_FallbackDimension", "1D", "OneDimension"),
    ("ST_FallbackDimension", "2D", "TwoDimensions"),
    // §21.4.7.12 `ST_CenterShapeMapping` — `fNode` is "First Node", not the letter `f` plus `Node`.
    ("ST_CenterShapeMapping", "fNode", "FirstNode"),
    // §21.4.7.64 `ST_VariableType` — `animOne`/`animLvl` want different grammatical forms of
    // "animate" ("Animate One" vs. "Animation Level"), so neither can go in the abbreviation table
    // without breaking the other.
    ("ST_VariableType", "animOne", "AnimateOne"),
    ("ST_VariableType", "animLvl", "AnimationLevel"),
    // §21.4.7.2 `ST_AnimLvlStr` — `lvl`/`ctr` name *which* level/center behaviour is enabled, not
    // bare "Level"/"Center".
    ("ST_AnimLvlStr", "lvl", "ByLevel"),
    ("ST_AnimLvlStr", "ctr", "FromCenter"),
    // §21.4.7.11 `ST_Breakpoint` — "End of Canvas", not the abbreviation cascade's "EndCanvas".
    ("ST_Breakpoint", "endCnv", "EndOfCanvas"),
    // §21.4.7.59/.60 `ST_TextAnchorHorizontal`/`ST_TextAnchorVertical`'s `…Ch` values want the full
    // "With Children" the prose titles give, not the abbreviation cascade's bare "…Child".
    (
        "ST_ParameterId",
        "txAnchorHorzCh",
        "TextAnchorHorizontalWithChildren",
    ),
    (
        "ST_ParameterId",
        "txAnchorVertCh",
        "TextAnchorVerticalWithChildren",
    ),
    // §21.4.7.21 `ST_ConstraintType` (Constraint Type) — every value, from the ECMA-376 Part 1
    // §21.4.7.21 table. `ctrX`/`ctrY` are transcribed exactly as the prose names them ("Center
    // Height" / "Center Width") even though the pairing with the attribute name reads backwards —
    // the naming convention sources names from the prose rather than correcting it.
    ("ST_ConstraintType", "alignOff", "AlignmentOffset"),
    ("ST_ConstraintType", "begMarg", "BeginningMargin"),
    ("ST_ConstraintType", "begPad", "BeginningPadding"),
    ("ST_ConstraintType", "bendDist", "BendingDistance"),
    ("ST_ConstraintType", "bMarg", "BottomMargin"),
    ("ST_ConstraintType", "bOff", "BottomOffset"),
    ("ST_ConstraintType", "connDist", "ConnectionDistance"),
    ("ST_ConstraintType", "ctrX", "CenterHeight"),
    ("ST_ConstraintType", "ctrXOff", "CenterXOffset"),
    ("ST_ConstraintType", "ctrY", "CenterWidth"),
    ("ST_ConstraintType", "ctrYOff", "CenterYOffset"),
    ("ST_ConstraintType", "diam", "Diameter"),
    ("ST_ConstraintType", "endMarg", "EndMargin"),
    ("ST_ConstraintType", "endPad", "EndPadding"),
    ("ST_ConstraintType", "hArH", "ArrowheadHeight"),
    ("ST_ConstraintType", "hOff", "HeightOffset"),
    ("ST_ConstraintType", "lMarg", "LeftMargin"),
    ("ST_ConstraintType", "lOff", "LeftOffset"),
    ("ST_ConstraintType", "none", "Unknown"),
    ("ST_ConstraintType", "primFontSz", "PrimaryFontSize"),
    ("ST_ConstraintType", "pyraAcctRatio", "PyramidAccentRatio"),
    ("ST_ConstraintType", "rMarg", "RightMargin"),
    ("ST_ConstraintType", "rOff", "RightOffset"),
    ("ST_ConstraintType", "secFontSz", "SecondaryFontSize"),
    ("ST_ConstraintType", "secSibSp", "SecondarySiblingSpacing"),
    ("ST_ConstraintType", "sibSp", "SiblingSpacing"),
    ("ST_ConstraintType", "sp", "Spacing"),
    ("ST_ConstraintType", "stemThick", "StemThickness"),
    ("ST_ConstraintType", "tMarg", "TopMargin"),
    ("ST_ConstraintType", "tOff", "TopOffset"),
    ("ST_ConstraintType", "userA", "UserDefinedA"),
    ("ST_ConstraintType", "userB", "UserDefinedB"),
    ("ST_ConstraintType", "userC", "UserDefinedC"),
    ("ST_ConstraintType", "userD", "UserDefinedD"),
    ("ST_ConstraintType", "userE", "UserDefinedE"),
    ("ST_ConstraintType", "userF", "UserDefinedF"),
    ("ST_ConstraintType", "userG", "UserDefinedG"),
    ("ST_ConstraintType", "userH", "UserDefinedH"),
    ("ST_ConstraintType", "userI", "UserDefinedI"),
    ("ST_ConstraintType", "userJ", "UserDefinedJ"),
    ("ST_ConstraintType", "userK", "UserDefinedK"),
    ("ST_ConstraintType", "userL", "UserDefinedL"),
    ("ST_ConstraintType", "userM", "UserDefinedM"),
    ("ST_ConstraintType", "userN", "UserDefinedN"),
    ("ST_ConstraintType", "userO", "UserDefinedO"),
    ("ST_ConstraintType", "userP", "UserDefinedP"),
    ("ST_ConstraintType", "userQ", "UserDefinedQ"),
    ("ST_ConstraintType", "userR", "UserDefinedR"),
    ("ST_ConstraintType", "userS", "UserDefinedS"),
    ("ST_ConstraintType", "userT", "UserDefinedT"),
    ("ST_ConstraintType", "userU", "UserDefinedU"),
    ("ST_ConstraintType", "userV", "UserDefinedV"),
    ("ST_ConstraintType", "userW", "UserDefinedW"),
    ("ST_ConstraintType", "userX", "UserDefinedX"),
    ("ST_ConstraintType", "userY", "UserDefinedY"),
    ("ST_ConstraintType", "userZ", "UserDefinedZ"),
    ("ST_ConstraintType", "wArH", "ArrowheadWidth"),
    ("ST_ConstraintType", "wOff", "WidthOffset"),
    // §21.4.7.49 `ST_ParameterId` (Parameter Identifier) — every value, from the ECMA-376 Part 1
    // §21.4.7.49 table.
    ("ST_ParameterId", "alignTx", "TextAlignment"),
    ("ST_ParameterId", "ar", "AspectRatio"),
    ("ST_ParameterId", "begPts", "BeginningPoints"),
    ("ST_ParameterId", "begSty", "BeginningArrowheadStyle"),
    ("ST_ParameterId", "bkPtFixedVal", "BreakpointFixedValue"),
    ("ST_ParameterId", "bkpt", "Breakpoint"),
    ("ST_ParameterId", "connRout", "ConnectionRoute"),
    ("ST_ParameterId", "ctrShpMap", "CenterShapeMapping"),
    ("ST_ParameterId", "dim", "ConnectorDimension"),
    ("ST_ParameterId", "dstNode", "DestinationNode"),
    ("ST_ParameterId", "endPts", "EndPoints"),
    ("ST_ParameterId", "endSty", "EndStyle"),
    ("ST_ParameterId", "fallback", "FallbackScale"),
    ("ST_ParameterId", "hierAlign", "HierarchyAlignment"),
    (
        "ST_ParameterId",
        "lnSpAfChP",
        "LineSpacingAfterChildrenParagraph",
    ),
    (
        "ST_ParameterId",
        "lnSpAfParP",
        "LineSpacingAfterParentParagraph",
    ),
    ("ST_ParameterId", "lnSpCh", "LineSpacingChildren"),
    ("ST_ParameterId", "lnSpPar", "LineSpacingParent"),
    (
        "ST_ParameterId",
        "parTxLTRAlign",
        "ParentTextLeftToRightAlignment",
    ),
    (
        "ST_ParameterId",
        "parTxRTLAlign",
        "ParentTextRightToLeftAlignment",
    ),
    (
        "ST_ParameterId",
        "pyraAcctBkgdNode",
        "PyramidAccentBackgroundNode",
    ),
    ("ST_ParameterId", "pyraAcctPos", "PyramidAccentPosition"),
    ("ST_ParameterId", "pyraAcctTxMar", "PyramidAccentTextMargin"),
    ("ST_ParameterId", "pyraAcctTxNode", "PyramidAccentTextNode"),
    ("ST_ParameterId", "pyraLvlNode", "PyramidLevelNode"),
    ("ST_ParameterId", "rtShortDist", "RouteShortestDistance"),
    (
        "ST_ParameterId",
        "shpTxLTRAlignCh",
        "ShapeTextLeftToRightAlignment",
    ),
    (
        "ST_ParameterId",
        "shpTxRTLAlignCh",
        "ShapeTextRightToLeftAlignment",
    ),
    ("ST_ParameterId", "spanAng", "SpanAngle"),
    ("ST_ParameterId", "srcNode", "SourceNode"),
    ("ST_ParameterId", "stAng", "StartAngle"),
    ("ST_ParameterId", "stBulletLvl", "StartBulletsAtLevel"),
    ("ST_ParameterId", "stElem", "StartingElement"),
    ("ST_ParameterId", "txAnchorHorz", "TextAnchorHorizontal"),
    ("ST_ParameterId", "txAnchorVert", "TextAnchorVertical"),
    ("ST_ParameterId", "txBlDir", "TextBlockDirection"),
    ("ST_ParameterId", "txDir", "TextDirection"),
    ("ST_ParameterId", "bendPt", "BendPoint"),
    ("ST_ParameterId", "rotPath", "RotationPath"),
    ("ST_ParameterId", "autoTxRot", "AutoTextRotation"),
    // §21.4.7.1 `ST_AlgorithmType`'s `conn` (Connector Algorithm) means the connector-routing
    // algorithm; §21.4.7.48 `ST_OutputShapeType`'s `conn` (Connection) names a shape kind — the
    // same three letters, two different concepts, so neither can go in the abbreviation table.
    ("ST_AlgorithmType", "conn", "Connector"),
    ("ST_AlgorithmType", "sp", "Space"),
    ("ST_OutputShapeType", "conn", "Connection"),
    // §21.4.7.5 `ST_AutoTextRotation`.
    ("ST_AutoTextRotation", "upr", "Upright"),
    ("ST_AutoTextRotation", "grav", "Gravity"),
    // §21.4.7.19 `ST_ConnectorRouting`.
    ("ST_ConnectorRouting", "stra", "Straight"),
];

/// Two-valued types → the `crate::support` normalizer module that handles all wire spellings.
/// Modeled as Rust `bool`.
pub const BOOL_TYPES: &[(&str, &str)] = &[("ST_OnOff", "on_off"), ("ST_TrueFalse", "true_false")];

/// Three-valued (true / false / blank) types → normalizer module. Modeled as `Option<bool>`.
pub const OPTIONAL_BOOL_TYPES: &[(&str, &str)] = &[("ST_TrueFalseBlank", "true_false_blank")];

/// Types intentionally not emitted (subsumed by another representation).
pub const SKIP_TYPES: &[&str] = &["ST_OnOff1"]; // the `on`/`off` half of the ST_OnOff union.

/// Maps an XSD numeric base to its Rust primitive, or `None` if not a plain numeric restriction.
pub fn primitive_for(base: &str) -> Option<&'static str> {
    Some(match base {
        "xsd:unsignedLong" => "u64",
        "xsd:unsignedInt" => "u32",
        "xsd:unsignedShort" => "u16",
        "xsd:unsignedByte" => "u8",
        "xsd:nonNegativeInteger" => "u64",
        "xsd:long" | "xsd:integer" => "i64",
        "xsd:int" => "i32",
        "xsd:short" => "i16",
        "xsd:byte" => "i8",
        "xsd:double" => "f64",
        // `shared-commonSimpleTypes.xsd` aliases, which `wml.xsd` restricts directly
        // (`ST_PixelsMeasure`, `ST_EighthPointMeasure`, `ST_PointMeasure`). Resolving them here
        // keeps a count of pixels a number rather than a string newtype.
        "s:ST_UnsignedDecimalNumber" => "u64",
        "s:ST_DecimalNumber" => "i64",
        _ => return None,
    })
}

/// Looks up the boolean normalizer module for a type, and whether it is optional (three-valued).
pub fn bool_kind(st_name: &str) -> Option<(&'static str, bool)> {
    if let Some((_, f)) = BOOL_TYPES.iter().find(|(n, _)| *n == st_name) {
        return Some((f, false));
    }
    if let Some((_, f)) = OPTIONAL_BOOL_TYPES.iter().find(|(n, _)| *n == st_name) {
        return Some((f, true));
    }
    None
}

/// The complex types whose child order a serializer in this workspace holds as a named constant.
///
/// Each row is `(Rust constant, schema file stem, XSD symbol, one-line description)`. The generated
/// tables cover **every** complex type of every emitted schema and are reachable by symbol; this
/// curated list is what gives the ones we actually write markup for a self-explanatory name, the
/// same discipline the simple-type allowlist follows. A name is never derived mechanically from a
/// cryptic symbol — `CT_CatAx` is `CATEGORY_AXIS` because ECMA-376 Part 1 calls `c:catAx` a Category
/// Axis, not because an abbreviation table guessed it.
///
/// Grow this list when a new model starts placing children; the table behind it is already there.
pub const CHILD_ORDER_EXPORTS: &[(&str, &str, &str, &str)] = &[
    // ---- DrawingML -------------------------------------------------------------------------
    (
        "CUSTOM_GEOMETRY_2D",
        "dml-main",
        "CT_CustomGeometry2D",
        "A freeform shape's geometry",
    ),
    (
        "EFFECT_LIST",
        "dml-main",
        "CT_EffectList",
        "The eight effects a shape can carry, at most one of each",
    ),
    (
        "GROUP_SHAPE_PROPERTIES",
        "dml-main",
        "CT_GroupShapeProperties",
        "A group shape's visual properties",
    ),
    (
        "GROUP_TRANSFORM_2D",
        "dml-main",
        "CT_GroupTransform2D",
        "A group's position, size and child coordinate space",
    ),
    (
        "LINE_PROPERTIES",
        "dml-main",
        "CT_LineProperties",
        "An outline's fill, dash, join and ends",
    ),
    (
        "PATH_2D",
        "dml-main",
        "CT_Path2D",
        "One path of a freeform geometry — a repeating choice of drawing commands",
    ),
    (
        "SHAPE_3D",
        "dml-main",
        "CT_Shape3D",
        "A shape's 3-D bevels and material",
    ),
    (
        "SHAPE_PROPERTIES",
        "dml-main",
        "CT_ShapeProperties",
        "A shape's transform, geometry, fill, line and effects",
    ),
    (
        "TABLE_CELL_BORDER_STYLE",
        "dml-main",
        "CT_TableCellBorderStyle",
        "The eight edges a table style paints on a cell",
    ),
    (
        "TABLE_CELL_PROPERTIES",
        "dml-main",
        "CT_TableCellProperties",
        "A table cell's borders, fill and insets",
    ),
    (
        "TABLE_CELL_3D",
        "dml-main",
        "CT_Cell3D",
        "A table cell's 3-D bevel and lighting",
    ),
    (
        "TABLE_PART_STYLE",
        "dml-main",
        "CT_TablePartStyle",
        "One band or corner of a table style",
    ),
    (
        "TABLE_PROPERTIES",
        "dml-main",
        "CT_TableProperties",
        "A table's fill, effects and style reference",
    ),
    (
        "TABLE_STYLE",
        "dml-main",
        "CT_TableStyle",
        "A whole table style: its background and thirteen part slots",
    ),
    (
        "TABLE_STYLE_CELL_STYLE",
        "dml-main",
        "CT_TableStyleCellStyle",
        "The cell formatting one part of a table style applies",
    ),
    (
        "TABLE_STYLE_TEXT_STYLE",
        "dml-main",
        "CT_TableStyleTextStyle",
        "The text formatting one part of a table style applies",
    ),
    (
        "TEXT_CHARACTER_PROPERTIES",
        "dml-main",
        "CT_TextCharacterProperties",
        "A run's character formatting",
    ),
    (
        "TEXT_LIST_STYLE",
        "dml-main",
        "CT_TextListStyle",
        "A default plus nine per-level paragraph property sets",
    ),
    (
        "TEXT_PARAGRAPH_PROPERTIES",
        "dml-main",
        "CT_TextParagraphProperties",
        "A paragraph's spacing, bullet, tabs and default run properties",
    ),
    (
        "TRANSFORM_2D",
        "dml-main",
        "CT_Transform2D",
        "A shape's position and size",
    ),
    // ---- DrawingML charts ------------------------------------------------------------------
    (
        "AREA_3D_CHART",
        "dml-chart",
        "CT_Area3DChart",
        "A three-dimensional area plot (`c:area3DChart`)",
    ),
    (
        "AREA_CHART",
        "dml-chart",
        "CT_AreaChart",
        "An area plot (`c:areaChart`)",
    ),
    (
        "AREA_SERIES",
        "dml-chart",
        "CT_AreaSer",
        "One series of an area plot (`c:areaChart > c:ser`)",
    ),
    (
        "BAR_3D_CHART",
        "dml-chart",
        "CT_Bar3DChart",
        "A three-dimensional bar plot (`c:bar3DChart`)",
    ),
    (
        "BAR_CHART",
        "dml-chart",
        "CT_BarChart",
        "A bar/column plot (`c:barChart`)",
    ),
    (
        "BAR_SERIES",
        "dml-chart",
        "CT_BarSer",
        "One series of a bar plot (`c:barChart > c:ser`)",
    ),
    (
        "BUBBLE_CHART",
        "dml-chart",
        "CT_BubbleChart",
        "A bubble plot (`c:bubbleChart`)",
    ),
    (
        "BUBBLE_SERIES",
        "dml-chart",
        "CT_BubbleSer",
        "One series of a bubble plot (`c:bubbleChart > c:ser`)",
    ),
    (
        "CATEGORY_AXIS",
        "dml-chart",
        "CT_CatAx",
        "A category axis (`c:catAx`)",
    ),
    (
        "CHART",
        "dml-chart",
        "CT_Chart",
        "A chart's title, plot area and legend",
    ),
    (
        "DATA_LABEL",
        "dml-chart",
        "CT_DLbl",
        "One point's data-label override (`c:dLbl`)",
    ),
    (
        "DATA_LABELS",
        "dml-chart",
        "CT_DLbls",
        "A plot's or a series' data-label settings (`c:dLbls`)",
    ),
    (
        "DATA_POINT_FORMAT",
        "dml-chart",
        "CT_DPt",
        "One point's own formatting (`c:dPt`)",
    ),
    (
        "DATE_AXIS",
        "dml-chart",
        "CT_DateAx",
        "A date axis (`c:dateAx`)",
    ),
    (
        "DOUGHNUT_CHART",
        "dml-chart",
        "CT_DoughnutChart",
        "A doughnut plot (`c:doughnutChart`)",
    ),
    (
        "ERROR_BARS",
        "dml-chart",
        "CT_ErrBars",
        "A series' error bars (`c:errBars`)",
    ),
    (
        "LINE_3D_CHART",
        "dml-chart",
        "CT_Line3DChart",
        "A three-dimensional line plot (`c:line3DChart`)",
    ),
    (
        "LINE_CHART",
        "dml-chart",
        "CT_LineChart",
        "A line plot (`c:lineChart`)",
    ),
    (
        "LINE_SERIES",
        "dml-chart",
        "CT_LineSer",
        "One series of a line or stock plot (`c:lineChart > c:ser`)",
    ),
    (
        "OF_PIE_CHART",
        "dml-chart",
        "CT_OfPieChart",
        "A pie-of-pie or bar-of-pie plot (`c:ofPieChart`)",
    ),
    (
        "PIE_3D_CHART",
        "dml-chart",
        "CT_Pie3DChart",
        "A three-dimensional pie plot (`c:pie3DChart`)",
    ),
    (
        "PIE_CHART",
        "dml-chart",
        "CT_PieChart",
        "A pie plot (`c:pieChart`)",
    ),
    (
        "PIE_SERIES",
        "dml-chart",
        "CT_PieSer",
        "One series of a pie, doughnut or pie-of-pie plot (`c:pieChart > c:ser`)",
    ),
    (
        "RADAR_CHART",
        "dml-chart",
        "CT_RadarChart",
        "A radar plot (`c:radarChart`)",
    ),
    (
        "RADAR_SERIES",
        "dml-chart",
        "CT_RadarSer",
        "One series of a radar plot (`c:radarChart > c:ser`)",
    ),
    (
        "SCALING",
        "dml-chart",
        "CT_Scaling",
        "An axis' orientation and explicit bounds",
    ),
    (
        "SCATTER_CHART",
        "dml-chart",
        "CT_ScatterChart",
        "An X/Y scatter plot (`c:scatterChart`)",
    ),
    (
        "SCATTER_SERIES",
        "dml-chart",
        "CT_ScatterSer",
        "One series of a scatter plot (`c:scatterChart > c:ser`)",
    ),
    (
        "SERIES_AXIS",
        "dml-chart",
        "CT_SerAx",
        "A series axis (`c:serAx`)",
    ),
    (
        "STOCK_CHART",
        "dml-chart",
        "CT_StockChart",
        "A high-low-close stock plot (`c:stockChart`)",
    ),
    (
        "SURFACE_3D_CHART",
        "dml-chart",
        "CT_Surface3DChart",
        "A three-dimensional surface plot (`c:surface3DChart`)",
    ),
    (
        "SURFACE_CHART",
        "dml-chart",
        "CT_SurfaceChart",
        "A surface plot seen from above (`c:surfaceChart`)",
    ),
    (
        "SURFACE_SERIES",
        "dml-chart",
        "CT_SurfaceSer",
        "One series of a surface plot (`c:surfaceChart > c:ser`)",
    ),
    (
        "TRENDLINE",
        "dml-chart",
        "CT_Trendline",
        "A series' trendline (`c:trendline`)",
    ),
    (
        "VALUE_AXIS",
        "dml-chart",
        "CT_ValAx",
        "A value axis (`c:valAx`)",
    ),
    // ---- PresentationML --------------------------------------------------------------------
    (
        "GRAPHIC_FRAME",
        "pml",
        "CT_GraphicalObjectFrame",
        "The frame a table, chart or diagram sits in on a slide",
    ),
    (
        "PRESENTATION",
        "pml",
        "CT_Presentation",
        "The presentation part's own children",
    ),
    // ---- DrawingML Diagram -----------------------------------------------------------------
    (
        "LAYOUT_VARIABLE_PROPERTY_SET",
        "dml-diagram",
        "CT_LayoutVariablePropertySet",
        "A `dgm:varLst`'s nine named layout-variable overrides, in schema order",
    ),
    // ---- WordprocessingML ------------------------------------------------------------------
    (
        "DOCUMENT_BASE",
        "wml",
        "CT_DocumentBase",
        "The page background a document and a glossary document both start from (`w:background`)",
    ),
    (
        "DOCUMENT",
        "wml",
        "CT_Document",
        "The `w:document` root's own content — the background it extends, then the body",
    ),
    (
        "BODY",
        "wml",
        "CT_Body",
        "A document's or glossary document's body: block-level content, then the last section's \
         properties (`w:body`)",
    ),
    (
        "PARAGRAPH",
        "wml",
        "CT_P",
        "One paragraph — its properties, then its runs and other inline content (`w:p`)",
    ),
    (
        "PARAGRAPH_PROPERTIES_BASE",
        "wml",
        "CT_PPrBase",
        "The paragraph-formatting properties shared by a paragraph's `w:pPr` and a paragraph mark's \
         run properties container",
    ),
    (
        "PARAGRAPH_PROPERTIES",
        "wml",
        "CT_PPr",
        "A paragraph's own properties (`w:pPr`): `CT_PPrBase`'s children, then the paragraph mark's \
         run properties, the last section's properties and the tracked-change wrapper",
    ),
    (
        "PARAGRAPH_MARK_RUN_PROPERTIES",
        "wml",
        "CT_ParaRPr",
        "The formatting of the paragraph mark itself (`w:pPr/w:rPr`): the tracked-change group, then \
         `EG_RPrBase`'s members, then the tracked-change wrapper",
    ),
    (
        "PARAGRAPH_BORDERS",
        "wml",
        "CT_PBdr",
        "The six borders a paragraph can carry (`w:pBdr`)",
    ),
    (
        "NUMBERING_PROPERTIES",
        "wml",
        "CT_NumPr",
        "A paragraph's numbering-definition reference (`w:numPr`): level and definition id, then \
         tracked-change markers",
    ),
    (
        "RUN",
        "wml",
        "CT_R",
        "One run — its properties, then its text and other inline content (`w:r`)",
    ),
    (
        "RUN_PROPERTIES",
        "wml",
        "CT_RPr",
        "A run's character formatting (`w:rPr`)",
    ),
    (
        "SECTION_PROPERTIES_BASE",
        "wml",
        "CT_SectPrBase",
        "A section's page, column and layout properties, before the change-tracking wrapper adds \
         its own attributes",
    ),
    (
        "SECTION_PROPERTIES",
        "wml",
        "CT_SectPr",
        "A section's own header/footer references, page, column and layout properties, and its \
         change-tracking wrapper (`w:sectPr`) — `CT_SectPrBase`'s children with `EG_HdrFtrReferences` \
         ahead of them and `sectPrChange` after",
    ),
    (
        "PAGE_BORDERS",
        "wml",
        "CT_PageBorders",
        "The four borders drawn around every page in a section (`w:pgBorders`): top, left, bottom, \
         right",
    ),
    (
        "PARAGRAPH_PROPERTIES_GENERAL",
        "wml",
        "CT_PPrGeneral",
        "`CT_PPrBase`'s children plus the tracked-change wrapper, with neither a run's own \
         properties nor a section's — what `w:docDefaults`, a style definition, a numbering level \
         and a table style override all carry as their own `w:pPr`",
    ),
    (
        "DOCUMENT_DEFAULTS",
        "wml",
        "CT_DocDefaults",
        "A document's own default run and paragraph properties (`w:docDefaults`), the bottom rung \
         of the style-resolution ladder",
    ),
    (
        "DEFAULT_RUN_PROPERTIES",
        "wml",
        "CT_RPrDefault",
        "The document default's own run properties (`w:docDefaults/w:rPrDefault`)",
    ),
    (
        "DEFAULT_PARAGRAPH_PROPERTIES",
        "wml",
        "CT_PPrDefault",
        "The document default's own paragraph properties (`w:docDefaults/w:pPrDefault`)",
    ),
    (
        "LATENT_STYLES",
        "wml",
        "CT_LatentStyles",
        "The style pane's latent-style defaults and per-style exceptions (`w:latentStyles`)",
    ),
    (
        "STYLE_DEFINITION",
        "wml",
        "CT_Style",
        "One style definition (`w:style`): its identity, its `basedOn`/`next`/`link` references, \
         its flags, and the paragraph, run and table properties it carries",
    ),
    (
        "STYLES",
        "wml",
        "CT_Styles",
        "The style definitions part's own root (`w:styles`): the document defaults, the latent-style \
         table, then every style definition",
    ),
    (
        "TABLE_STYLE_OVERRIDE",
        "wml",
        "CT_TblStylePr",
        "One conditional-formatting override inside a table style (`w:style/w:tblStylePr`)",
    ),
    (
        "NUMBERING",
        "wml",
        "CT_Numbering",
        "The numbering definitions part's own root (`w:numbering`): picture bullets, abstract \
         numbering definitions, then numbering instances",
    ),
    (
        "ABSTRACT_NUMBERING",
        "wml",
        "CT_AbstractNum",
        "One abstract numbering definition (`w:abstractNum`): its identity, its style links, then \
         up to nine numbering levels",
    ),
    (
        "NUMBERING_INSTANCE",
        "wml",
        "CT_Num",
        "One concrete numbering definition instance (`w:num`): the abstract definition it uses, \
         then any per-level overrides",
    ),
    (
        "NUMBERING_LEVEL_OVERRIDE",
        "wml",
        "CT_NumLvl",
        "One numbering instance's override of a single level (`w:num/w:lvlOverride`): a start \
         override, or a whole replacement level",
    ),
    (
        "NUMBERING_LEVEL",
        "wml",
        "CT_Lvl",
        "One numbering level's own formatting (`w:abstractNum/w:lvl`, and the replacement level a \
         `w:lvlOverride` may carry)",
    ),
    (
        "CELL_PROPERTIES",
        "wml",
        "CT_TcPr",
        "A table cell's own properties (`w:tcPr`) — MJXOFF-116 places `gridSpan`/`hMerge`/`vMerge` \
         (the grid-structural members) at their schema rank; everything else stays raw, typed by \
         MJXOFF-119",
    ),
    (
        "TABLE_PROPERTIES_BASE",
        "wml",
        "CT_TblPrBase",
        "The table-formatting properties shared by a table's own `w:tblPr` and a table style \
         conditional-formatting override's `w:tblStylePr/w:tblPr`",
    ),
    (
        "TABLE_ROW_PROPERTIES_BASE",
        "wml",
        "CT_TrPrBase",
        "The row-formatting properties shared by a row's own `w:trPr` and a table style conditional-\
         formatting override's `w:tblStylePr/w:trPr`",
    ),
    (
        "TABLE_EXCEPTION_PROPERTIES_BASE",
        "wml",
        "CT_TblPrExBase",
        "The table properties a single row may override (`w:tblPrEx`) — a subset of `CT_TblPrBase`",
    ),
    (
        "TABLE_BORDERS",
        "wml",
        "CT_TblBorders",
        "The eight borders a table (or a table style) can carry (`w:tblBorders`)",
    ),
    (
        "CELL_BORDERS",
        "wml",
        "CT_TcBorders",
        "The ten borders a table cell (or a table style) can carry (`w:tcBorders`) — `CT_TblBorders`'s \
         eight plus the two diagonals",
    ),
    (
        "TABLE_CELL_MARGINS",
        "wml",
        "CT_TblCellMar",
        "A table's default cell margins (`w:tblCellMar`)",
    ),
    (
        "CELL_MARGINS",
        "wml",
        "CT_TcMar",
        "One cell's own margins (`w:tcMar`), overriding the table's `w:tblCellMar`",
    ),
    (
        "FORM_FIELD_CHECK_BOX",
        "wml",
        "CT_FFCheckBox",
        "A checkbox form field's own size (fixed or automatic), default and checked state \
         (`w:checkBox`)",
    ),
    (
        "FORM_FIELD_DROP_DOWN_LIST",
        "wml",
        "CT_FFDDList",
        "A drop-down-list form field's own entries and selection (`w:ddList`)",
    ),
    (
        "FORM_FIELD_TEXT_INPUT",
        "wml",
        "CT_FFTextInput",
        "A text-input form field's own kind, default text, maximum length and display format \
         (`w:textInput`)",
    ),
    (
        "FOOTNOTE_PROPERTIES",
        "wml",
        "CT_FtnProps",
        "A section's own footnote settings (`w:sectPr/w:footnotePr`): position, number format, then \
         the shared `EG_FtnEdnNumProps` start/restart pair",
    ),
    (
        "ENDNOTE_PROPERTIES",
        "wml",
        "CT_EdnProps",
        "A section's own endnote settings (`w:sectPr/w:endnotePr`) — the same shape as \
         `CT_FtnProps`, with `CT_EdnPos`'s narrower two-value position instead of `CT_FtnPos`'s four",
    ),
    // ---- WordprocessingML document-configuration parts (MJXOFF-136) -------------------------
    (
        "SETTINGS",
        "wml",
        "CT_Settings",
        "The document settings part's own root (`word/settings.xml`, `w:settings`) — 98 \
         independently optional children, in schema order",
    ),
    (
        "COMPAT",
        "wml",
        "CT_Compat",
        "The compatibility-option flags a document carries forward from the application that last \
         saved it (`w:compat`), then any number of named `w:compatSetting` entries",
    ),
    (
        "WEB_SETTINGS",
        "wml",
        "CT_WebSettings",
        "The web settings part's own root (`word/webSettings.xml`, `w:webSettings`): a legacy \
         frameset or `w:div` tree, then the save-as-web flags",
    ),
    (
        "MAIL_MERGE",
        "wml",
        "CT_MailMerge",
        "A document's own mail-merge configuration (`w:settings/w:mailMerge`): document type and \
         data source, then the merge/print/view options and the ODSO data-source description",
    ),
    (
        "ODSO",
        "wml",
        "CT_Odso",
        "An Office Data Source Object description (`w:mailMerge/w:odso`): the connection, the \
         source table, then field-mapping and per-recipient data",
    ),
    (
        "FONT",
        "wml",
        "CT_Font",
        "One font table entry (`word/fontTable.xml`'s own `w:font`): alternate name, PANOSE \
         classification, character set, family, pitch and signature, then the four embedded-font \
         relationships (regular/bold/italic/bold-italic)",
    ),
    // ---- DrawingML WordprocessingDrawing (MJXOFF-131) ---------------------------------------
    (
        "WP_INLINE",
        "dml-wordprocessingDrawing",
        "CT_Inline",
        "An inline drawing's own placement: extent, effect extent, non-visual properties, then the \
         `a:graphic` it wraps",
    ),
    (
        "WP_ANCHOR",
        "dml-wordprocessingDrawing",
        "CT_Anchor",
        "A floating drawing's own placement: simple position, horizontal/vertical position, extent, \
         effect extent, the wrap mode choice, non-visual properties, then the `a:graphic` it wraps",
    ),
    (
        "WP_WRAP_SQUARE",
        "dml-wordprocessingDrawing",
        "CT_WrapSquare",
        "A square-wrap drawing's own effect extent",
    ),
    (
        "WP_WRAP_TIGHT",
        "dml-wordprocessingDrawing",
        "CT_WrapTight",
        "A tight-wrap drawing's own wrap polygon",
    ),
    (
        "WP_WRAP_THROUGH",
        "dml-wordprocessingDrawing",
        "CT_WrapThrough",
        "A through-wrap drawing's own wrap polygon",
    ),
    (
        "WP_WRAP_TOP_AND_BOTTOM",
        "dml-wordprocessingDrawing",
        "CT_WrapTopBottom",
        "A top-and-bottom-wrap drawing's own effect extent",
    ),
    (
        "WP_WRAP_PATH",
        "dml-wordprocessingDrawing",
        "CT_WrapPath",
        "A wrap polygon's own start point, then two or more line-to points",
    ),
    (
        "WP_GRAPHIC_FRAME",
        "dml-wordprocessingDrawing",
        "CT_GraphicFrame",
        "An inline OLE-style graphic frame's own non-visual properties, transform, then the \
         `a:graphic` it wraps",
    ),
    (
        "WP_WORDPROCESSING_GROUP",
        "dml-wordprocessingDrawing",
        "CT_WordprocessingGroup",
        "A group of Word shapes' own non-visual properties, transform, then its member shapes",
    ),
    (
        "WP_WORDPROCESSING_CANVAS",
        "dml-wordprocessingDrawing",
        "CT_WordprocessingCanvas",
        "A drawing canvas's own background, whole-canvas formatting, then its member shapes",
    ),
    (
        "WP_CONTENT_PART",
        "dml-wordprocessingDrawing",
        "CT_WordprocessingContentPart",
        "An ink content part's own non-visual properties, then its transform",
    ),
    (
        "WP_CONTENT_PART_NON_VISUAL",
        "dml-wordprocessingDrawing",
        "CT_WordprocessingContentPartNonVisual",
        "An ink content part's own identity and lock list",
    ),
    (
        "WP_TEXTBOX_INFO",
        "dml-wordprocessingDrawing",
        "CT_TextboxInfo",
        "A shape's own text box content, then its extension list — `mjx-docx` places `w:txbxContent` \
         at this rank; the shape (`CT_WordprocessingShape`) and its text box content \
         (`CT_TxbxContent`) are WordprocessingML-content-shaped and so live in `mjx-docx`, not here \
         — see `mjx_dml::wordprocessing_drawing`'s own module doc",
    ),
    // ---- Office Math / shared-math (MJXOFF-134) ----------------------------------------------
    // Every `shared-math` complex type whose children form a real sequence worth auditing order
    // for. The nineteen leaf `val`-attribute types (`CT_Integer255`, `CT_OnOff`, `CT_Shp`, …),
    // `CT_ManualBreak` (attribute-only) and the seven single-optional-child wrappers (`CT_MC`, and
    // the six `mjx-omml::ControlOnlyProperties` shapes — `CT_FuncPr`/`CT_LimLowPr`/`CT_LimUppPr`/
    // `CT_SPrePr`/`CT_SSubPr`/`CT_SSupPr`) have no ordering question to ask (zero or one child), the
    // same reason `dml-wordprocessingDrawing`'s own single-child leaves are absent above.
    (
        "MATH", "shared-math", "CT_OMath",
        "One equation: a repeating choice of math objects and runs (`m:oMath`)",
    ),
    (
        "MATH_PARAGRAPH", "shared-math", "CT_OMathPara",
        "A paragraph of display equations: its own properties, then one or more `m:oMath` (`m:oMathPara`)",
    ),
    (
        "MATH_PARAGRAPH_PROPERTIES", "shared-math", "CT_OMathParaPr",
        "A math paragraph's own justification",
    ),
    (
        "MATH_ARGUMENT", "shared-math", "CT_OMathArg",
        "One math argument slot: an optional size override, its own math content, then a trailing \
         control-properties pass-through — the recursive core every object bottoms out at",
    ),
    (
        "MATH_ARGUMENT_PROPERTIES", "shared-math", "CT_OMathArgPr",
        "An argument's own size override",
    ),
    (
        "MATH_RUN", "shared-math", "CT_R",
        "One run of math content: its own properties, then a repeating choice of text and run inner \
         content (`m:r`)",
    ),
    (
        "MATH_RUN_PROPERTIES", "shared-math", "CT_RPR",
        "A run's own literal/normal-text/script-style choice, manual break and alignment flag \
         (`m:rPr`) — distinct from `w:rPr`",
    ),
    (
        "MATH_PROPERTIES", "shared-math", "CT_MathPr",
        "The document-level math settings: default font, break rule, display defaults, margins/\
         spacing, default justification, n-ary/integral limit placement (`m:mathPr`)",
    ),
    (
        "ACCENT", "shared-math", "CT_Acc",
        "An accent's own properties, then its base (`m:acc`)",
    ),
    (
        "ACCENT_PROPERTIES", "shared-math", "CT_AccPr",
        "An accent's own combining character, then control properties",
    ),
    (
        "BAR", "shared-math", "CT_Bar",
        "A bar's own properties, then its base (`m:bar`)",
    ),
    (
        "BAR_PROPERTIES", "shared-math", "CT_BarPr",
        "A bar's own position, then control properties",
    ),
    (
        "MATH_BOX", "shared-math", "CT_Box",
        "A box's own properties, then its base (`m:box`)",
    ),
    (
        "MATH_BOX_PROPERTIES", "shared-math", "CT_BoxPr",
        "A box's own emulation/break/diff/alignment flags, then control properties",
    ),
    (
        "BORDER_BOX", "shared-math", "CT_BorderBox",
        "A border box's own properties, then its base (`m:borderBox`)",
    ),
    (
        "BORDER_BOX_PROPERTIES", "shared-math", "CT_BorderBoxPr",
        "A border box's own edge-visibility and strike flags, then control properties",
    ),
    (
        "DELIMITER", "shared-math", "CT_D",
        "A delimiter's own properties, then one or more enclosed arguments (`m:d`)",
    ),
    (
        "DELIMITER_PROPERTIES", "shared-math", "CT_DPr",
        "A delimiter's own bracket/separator characters, growth and shape, then control properties",
    ),
    (
        "EQUATION_ARRAY", "shared-math", "CT_EqArr",
        "An equation array's own properties, then one or more rows (`m:eqArr`)",
    ),
    (
        "EQUATION_ARRAY_PROPERTIES", "shared-math", "CT_EqArrPr",
        "An equation array's own base alignment and spacing, then control properties",
    ),
    (
        "FRACTION", "shared-math", "CT_F",
        "A fraction's own properties, then its numerator and denominator (`m:f`)",
    ),
    (
        "FRACTION_PROPERTIES", "shared-math", "CT_FPr",
        "A fraction's own bar style, then control properties",
    ),
    (
        "FUNCTION_APPLY", "shared-math", "CT_Func",
        "A function-apply's own properties, its own name, then the applied argument (`m:func`)",
    ),
    (
        "GROUP_CHARACTER", "shared-math", "CT_GroupChr",
        "A group character's own properties, then its base (`m:groupChr`)",
    ),
    (
        "GROUP_CHARACTER_PROPERTIES", "shared-math", "CT_GroupChrPr",
        "A group character's own glyph, position and justification, then control properties",
    ),
    (
        "LOWER_LIMIT", "shared-math", "CT_LimLow",
        "A lower-limit's own properties, its base, then the limit (`m:limLow`)",
    ),
    (
        "UPPER_LIMIT", "shared-math", "CT_LimUpp",
        "An upper-limit's own properties, its base, then the limit (`m:limUpp`)",
    ),
    (
        "MATRIX", "shared-math", "CT_M",
        "A matrix's own properties, then one or more rows (`m:m`)",
    ),
    (
        "MATRIX_ROW", "shared-math", "CT_MR",
        "One matrix row: one or more cells (`m:mr`)",
    ),
    (
        "MATRIX_PROPERTIES", "shared-math", "CT_MPr",
        "A matrix's own baseline, placeholder, spacing and column properties, then control properties",
    ),
    (
        "MATRIX_COLUMNS", "shared-math", "CT_MCS",
        "A matrix's own per-column properties: one or more entries (`m:mcs`)",
    ),
    (
        "MATRIX_COLUMN_PROPERTIES", "shared-math", "CT_MCPr",
        "One matrix-column-properties entry's own span count, then justification",
    ),
    (
        "NARY_OPERATOR", "shared-math", "CT_Nary",
        "An n-ary operator's own properties, its lower and upper limit, then its operand (`m:nary`)",
    ),
    (
        "NARY_OPERATOR_PROPERTIES", "shared-math", "CT_NaryPr",
        "An n-ary operator's own glyph, limit location and growth/hide flags, then control properties",
    ),
    (
        "PHANTOM", "shared-math", "CT_Phant",
        "A phantom's own properties, then its base (`m:phant`)",
    ),
    (
        "PHANTOM_PROPERTIES", "shared-math", "CT_PhantPr",
        "A phantom's own visibility and zero-metric flags, then control properties",
    ),
    (
        "RADICAL", "shared-math", "CT_Rad",
        "A radical's own properties, its degree, then its radicand (`m:rad`)",
    ),
    (
        "RADICAL_PROPERTIES", "shared-math", "CT_RadPr",
        "A radical's own degree-hide flag, then control properties",
    ),
    (
        "PRE_SCRIPT", "shared-math", "CT_SPre",
        "A pre-sub-superscript's own properties, its subscript, superscript, then its base (`m:sPre`)",
    ),
    (
        "SUBSCRIPT", "shared-math", "CT_SSub",
        "A subscript's own properties, its base, then the subscript (`m:sSub`)",
    ),
    (
        "SUBSCRIPT_SUPERSCRIPT", "shared-math", "CT_SSubSup",
        "A combined subscript-superscript's own properties, its base, subscript, then superscript \
         (`m:sSubSup`)",
    ),
    (
        "SUBSCRIPT_SUPERSCRIPT_PROPERTIES", "shared-math", "CT_SSubSupPr",
        "A combined subscript-superscript's own alignment flag, then control properties",
    ),
    (
        "SUPERSCRIPT", "shared-math", "CT_SSup",
        "A superscript's own properties, its base, then the superscript (`m:sSup`)",
    ),

    // ---- SpreadsheetML (MJXOFF-132) ----------------------------------------------------------
    //
    // The five types the Phase D children place children into first: the two part roots, the
    // styles part's root, and the two types the cell store is built from. `WORKSHEET`'s 39 slots
    // are the largest `xsd:sequence` in the workspace, which is precisely why no writer should be
    // holding that order in its head.
    //
    // `ROW` and `CELL` would be ambiguous in this flat namespace — a Word table row and a
    // DrawingML table cell are both already here under other names — so both are qualified.
    (
        "WORKSHEET", "sml", "CT_Worksheet",
        "A worksheet's 39 children, from `sheetPr` to `extLst` (`x:worksheet`)",
    ),
    (
        "WORKBOOK", "sml", "CT_Workbook",
        "A workbook's 19 children, from `fileVersion` to `extLst` (`x:workbook`)",
    ),
    (
        "STYLESHEET", "sml", "CT_Stylesheet",
        "The styles part's 11 children: number formats, the three resource tables (fonts, fills, \
         borders), the two xf tables, cell styles, dxfs, table styles, colours and `extLst` \
         (`x:styleSheet`)",
    ),
    (
        "WORKSHEET_ROW", "sml", "CT_Row",
        "One row of a worksheet: its cells, then `extLst` (`x:row`)",
    ),
    (
        "WORKSHEET_CELL", "sml", "CT_Cell",
        "One cell: its formula, cached value, inline string, then `extLst` (`x:c`)",
    ),

    // ---- The styles part's resource tables (MJXOFF-105) ---------------------------------------
    //
    // Four more types inside `CT_Stylesheet` whose children a writer places rather than appends.
    // Each is qualified `STYLESHEET_` for the reason `WORKSHEET_ROW` and `WORKSHEET_CELL` are
    // qualified: `CT_Border`, `CT_Color*` and `CT_Dxf` name concepts that already exist under other
    // schemas in this flat namespace, and a bare `BORDER` beside `CELL_BORDERS` would read as the
    // same thing.
    (
        "STYLESHEET_BORDER", "sml", "CT_Border",
        "One border of the styles part's border table: its **nine** edges, from `start` to \
         `horizontal` (`x:border`)",
    ),
    (
        "STYLESHEET_PATTERN_FILL", "sml", "CT_PatternFill",
        "A pattern fill's foreground colour, then its background colour (`x:patternFill`)",
    ),
    (
        "STYLESHEET_DIFFERENTIAL_FORMAT", "sml", "CT_Dxf",
        "One differential format's seven children: font, number format, fill, alignment, border, \
         protection, then `extLst` (`x:dxf`)",
    ),
    (
        "STYLESHEET_COLOR_TABLE", "sml", "CT_Colors",
        "The colour table's indexed palette, then its most-recently-used colours (`x:colors`)",
    ),
];

/// Reports naming-override rows that no emitted type or value matched.
///
/// An override is a claim about a schema — *this `ST_*` symbol exists, and it carries this wire
/// value*. A row whose symbol or value is misspelled makes no name and produces no error: it simply
/// does nothing, and the token it was written for keeps the mechanical name the row exists to
/// replace. With hundreds of hand-authored rows sourced from a PDF that failure is likely enough to
/// be worth closing, so `codegen` fails on a dead row.
///
/// `emitted` is every simple type an engine's modules actually rendered; both tables are checked
/// against it. Returns the offending rows as human-readable strings, empty when all are live.
pub fn unused_overrides(
    engine: &NameEngine,
    emitted: &[crate::codegen::xsd::SimpleType],
) -> Vec<String> {
    use crate::codegen::xsd::SimpleKind;

    let mut dead = Vec::new();
    for (st_name, _) in engine.type_overrides {
        if !emitted.iter().any(|t| t.name == *st_name) {
            dead.push(format!(
                "type override for `{st_name}`, which is not emitted"
            ));
        }
    }
    for (st_name, wire, _) in engine.variant_overrides {
        let live = emitted.iter().any(|t| {
            t.name == *st_name
                && matches!(&t.kind, SimpleKind::Enumeration { values, .. }
                    if values.iter().any(|v| v == wire))
        });
        if !live {
            dead.push(format!(
                "variant override for `{st_name}` value {wire:?}, which the schema does not declare"
            ));
        }
    }
    dead
}
