//! Curated code-generation data for the shared-types slice.
//!
//! This is the hand-authored knowledge the generator needs: the naming overrides (comprehensive
//! names sourced from the ECMA-376 prose where the token is cryptic), abbreviation expansions, the
//! boolean-family mapping, and the XSD-base → Rust-primitive table. Extending the generator to
//! wml/sml/pml/dml means growing these tables, not changing the engine.

use crate::codegen::naming::NameEngine;

/// The naming engine configured for the shared slice.
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
    // PresentationML placeholders: `p:ph`'s `type`, `sz`, and `orient`. `ST_Direction` is PML's own
    // two-valued axis (`horz`/`vert`), named for what it selects rather than the generic "direction".
    ("ST_PlaceholderType", "PlaceholderType"),
    ("ST_PlaceholderSize", "PlaceholderSize"),
    ("ST_Direction", "Orientation"),
    // PresentationML slide layouts and slide size: `p:sldLayout@type` and `p:sldSz@type`. Both are
    // named `*Kind` because the wire attribute is `type`, which is a Rust keyword in field position.
    ("ST_SlideLayoutType", "SlideLayoutKind"),
    ("ST_SlideSizeType", "SlideSizeKind"),
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
        "xsd:long" | "xsd:integer" => "i64",
        "xsd:int" => "i32",
        "xsd:short" => "i16",
        "xsd:byte" => "i8",
        "xsd:double" => "f64",
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
