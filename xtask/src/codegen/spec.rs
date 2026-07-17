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
