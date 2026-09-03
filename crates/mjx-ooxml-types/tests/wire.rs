//! Wire round-trip tests for the generated shared simple types: every value maps to its exact XSD
//! token and back, comprehensively-named variants resolve from their original OOXML spellings, and
//! the boolean normalizers collapse all spellings.

use std::str::FromStr;

use mjx_ooxml_types::drawingml::{
    AutonumberScheme, ColorSchemeSlot, CompoundLine, FontAlignment, LineCap, LineEndLength,
    LineEndType, LineEndWidth, PatternType, PenAlignment, PresetLineDash, PresetShadow,
    PresetShapeType, RectangleAlignment, SchemeColor, TabAlignment, TextAlignment,
    TextCapitalization, TextStrike, TextUnderline,
};
use mjx_ooxml_types::namespaces;
use mjx_ooxml_types::presentationml::{
    Orientation, PlaceholderSize, PlaceholderType, SlideLayoutKind, SlideSizeKind,
};
use mjx_ooxml_types::shared::{
    CalendarType, ConformanceClass, CryptographicProvider, RelativeVerticalAlignment,
    VerticalTextPosition,
};

/// Every `ST_ShapeType` wire token, in `dml-main.xsd` schema order (187 values).
const SHAPE_TYPE_TOKENS: &[&str] = &[
    "line",
    "lineInv",
    "triangle",
    "rtTriangle",
    "rect",
    "diamond",
    "parallelogram",
    "trapezoid",
    "nonIsoscelesTrapezoid",
    "pentagon",
    "hexagon",
    "heptagon",
    "octagon",
    "decagon",
    "dodecagon",
    "star4",
    "star5",
    "star6",
    "star7",
    "star8",
    "star10",
    "star12",
    "star16",
    "star24",
    "star32",
    "roundRect",
    "round1Rect",
    "round2SameRect",
    "round2DiagRect",
    "snipRoundRect",
    "snip1Rect",
    "snip2SameRect",
    "snip2DiagRect",
    "plaque",
    "ellipse",
    "teardrop",
    "homePlate",
    "chevron",
    "pieWedge",
    "pie",
    "blockArc",
    "donut",
    "noSmoking",
    "rightArrow",
    "leftArrow",
    "upArrow",
    "downArrow",
    "stripedRightArrow",
    "notchedRightArrow",
    "bentUpArrow",
    "leftRightArrow",
    "upDownArrow",
    "leftUpArrow",
    "leftRightUpArrow",
    "quadArrow",
    "leftArrowCallout",
    "rightArrowCallout",
    "upArrowCallout",
    "downArrowCallout",
    "leftRightArrowCallout",
    "upDownArrowCallout",
    "quadArrowCallout",
    "bentArrow",
    "uturnArrow",
    "circularArrow",
    "leftCircularArrow",
    "leftRightCircularArrow",
    "curvedRightArrow",
    "curvedLeftArrow",
    "curvedUpArrow",
    "curvedDownArrow",
    "swooshArrow",
    "cube",
    "can",
    "lightningBolt",
    "heart",
    "sun",
    "moon",
    "smileyFace",
    "irregularSeal1",
    "irregularSeal2",
    "foldedCorner",
    "bevel",
    "frame",
    "halfFrame",
    "corner",
    "diagStripe",
    "chord",
    "arc",
    "leftBracket",
    "rightBracket",
    "leftBrace",
    "rightBrace",
    "bracketPair",
    "bracePair",
    "straightConnector1",
    "bentConnector2",
    "bentConnector3",
    "bentConnector4",
    "bentConnector5",
    "curvedConnector2",
    "curvedConnector3",
    "curvedConnector4",
    "curvedConnector5",
    "callout1",
    "callout2",
    "callout3",
    "accentCallout1",
    "accentCallout2",
    "accentCallout3",
    "borderCallout1",
    "borderCallout2",
    "borderCallout3",
    "accentBorderCallout1",
    "accentBorderCallout2",
    "accentBorderCallout3",
    "wedgeRectCallout",
    "wedgeRoundRectCallout",
    "wedgeEllipseCallout",
    "cloudCallout",
    "cloud",
    "ribbon",
    "ribbon2",
    "ellipseRibbon",
    "ellipseRibbon2",
    "leftRightRibbon",
    "verticalScroll",
    "horizontalScroll",
    "wave",
    "doubleWave",
    "plus",
    "flowChartProcess",
    "flowChartDecision",
    "flowChartInputOutput",
    "flowChartPredefinedProcess",
    "flowChartInternalStorage",
    "flowChartDocument",
    "flowChartMultidocument",
    "flowChartTerminator",
    "flowChartPreparation",
    "flowChartManualInput",
    "flowChartManualOperation",
    "flowChartConnector",
    "flowChartPunchedCard",
    "flowChartPunchedTape",
    "flowChartSummingJunction",
    "flowChartOr",
    "flowChartCollate",
    "flowChartSort",
    "flowChartExtract",
    "flowChartMerge",
    "flowChartOfflineStorage",
    "flowChartOnlineStorage",
    "flowChartMagneticTape",
    "flowChartMagneticDisk",
    "flowChartMagneticDrum",
    "flowChartDisplay",
    "flowChartDelay",
    "flowChartAlternateProcess",
    "flowChartOffpageConnector",
    "actionButtonBlank",
    "actionButtonHome",
    "actionButtonHelp",
    "actionButtonInformation",
    "actionButtonForwardNext",
    "actionButtonBackPrevious",
    "actionButtonEnd",
    "actionButtonBeginning",
    "actionButtonReturn",
    "actionButtonDocument",
    "actionButtonSound",
    "actionButtonMovie",
    "gear6",
    "gear9",
    "funnel",
    "mathPlus",
    "mathMinus",
    "mathMultiply",
    "mathDivide",
    "mathEqual",
    "mathNotEqual",
    "cornerTabs",
    "squareTabs",
    "plaqueTabs",
    "chartX",
    "chartStar",
    "chartPlus",
];

/// Asserts each wire token parses to a value that serializes back to the same token.
fn assert_round_trip<T, F, G>(tokens: &[&str], from: F, to: G)
where
    F: Fn(&str) -> Option<T>,
    G: Fn(T) -> &'static str,
    T: Copy,
{
    for &token in tokens {
        let value = from(token).unwrap_or_else(|| panic!("from_wire({token:?}) returned None"));
        assert_eq!(to(value), token, "round-trip mismatch for {token:?}");
    }
}

#[test]
fn calendar_type_round_trips_all_tokens() {
    let tokens = [
        "gregorian",
        "gregorianUs",
        "gregorianMeFrench",
        "gregorianArabic",
        "hijri",
        "hebrew",
        "taiwan",
        "japan",
        "thai",
        "korea",
        "saka",
        "gregorianXlitEnglish",
        "gregorianXlitFrench",
        "none",
    ];
    assert_round_trip(&tokens, CalendarType::from_wire, CalendarType::to_wire);

    // Comprehensive name maps to the cryptic wire token.
    assert_eq!(
        CalendarType::from_wire("gregorianUs"),
        Some(CalendarType::GregorianUnitedStates)
    );
    assert_eq!(CalendarType::GregorianUnitedStates.to_wire(), "gregorianUs");
    assert_eq!(CalendarType::from_wire("bogus"), None);
}

#[test]
fn other_enums_round_trip_and_expose_meaningful_names() {
    assert_round_trip(
        &["rsaAES", "rsaFull", "custom"],
        CryptographicProvider::from_wire,
        CryptographicProvider::to_wire,
    );
    assert_eq!(CryptographicProvider::RsaAes.to_wire(), "rsaAES");

    assert_round_trip(
        &["baseline", "superscript", "subscript"],
        VerticalTextPosition::from_wire,
        VerticalTextPosition::to_wire,
    );

    assert_round_trip(
        &["inline", "top", "center", "bottom", "inside", "outside"],
        RelativeVerticalAlignment::from_wire,
        RelativeVerticalAlignment::to_wire,
    );
}

#[test]
fn preset_shape_type_round_trips_every_token() {
    // The full ST_ShapeType roster (187 tokens) each parses and serializes back exactly — this
    // guards every curated override and proves no two variants collide on a wire token.
    assert_eq!(SHAPE_TYPE_TOKENS.len(), 187);
    assert_round_trip(
        SHAPE_TYPE_TOKENS,
        PresetShapeType::from_wire,
        PresetShapeType::to_wire,
    );
}

#[test]
fn preset_shape_type_exposes_comprehensive_names() {
    // Curated names (from cryptic/abbreviated tokens) map to the exact wire spelling.
    for (token, value) in [
        ("rtTriangle", PresetShapeType::RightTriangle),
        ("roundRect", PresetShapeType::RoundedRectangle),
        (
            "round2SameRect",
            PresetShapeType::RoundSameSideCornersRectangle,
        ),
        (
            "snipRoundRect",
            PresetShapeType::SnipAndRoundSingleCornerRectangle,
        ),
        ("star4", PresetShapeType::FourPointStar),
        ("uturnArrow", PresetShapeType::UTurnArrow),
        (
            "wedgeRoundRectCallout",
            PresetShapeType::WedgeRoundedRectangleCallout,
        ),
    ] {
        assert_eq!(PresetShapeType::from_wire(token), Some(value));
        assert_eq!(value.to_wire(), token);
    }
    // A well-formed token that auto-expands (no override) still resolves.
    assert_eq!(
        PresetShapeType::from_wire("flowChartProcess"),
        Some(PresetShapeType::FlowChartProcess)
    );
    // Unknown / future token: no panic, reported as absent.
    assert_eq!(PresetShapeType::from_wire("notAShape"), None);
    assert_eq!(
        PresetShapeType::from_str("notAShape").unwrap_err().value(),
        "notAShape"
    );
}

#[test]
fn scheme_color_round_trips_all_tokens() {
    let tokens = [
        "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4", "accent5",
        "accent6", "hlink", "folHlink", "phClr", "dk1", "lt1", "dk2", "lt2",
    ];
    assert_round_trip(&tokens, SchemeColor::from_wire, SchemeColor::to_wire);

    // Comprehensive names map to the cryptic theme-slot tokens.
    assert_eq!(
        SchemeColor::from_wire("bg1"),
        Some(SchemeColor::Background1)
    );
    assert_eq!(SchemeColor::from_wire("tx1"), Some(SchemeColor::Text1));
    assert_eq!(
        SchemeColor::from_wire("folHlink"),
        Some(SchemeColor::FollowedHyperlink)
    );
    assert_eq!(SchemeColor::Accent1.to_wire(), "accent1");
    assert_eq!(SchemeColor::from_wire("bogus"), None);
}

/// Every `ST_PresetPatternVal` wire token, in `dml-main.xsd` schema order (54 values).
const PATTERN_TYPE_TOKENS: &[&str] = &[
    "pct5",
    "pct10",
    "pct20",
    "pct25",
    "pct30",
    "pct40",
    "pct50",
    "pct60",
    "pct70",
    "pct75",
    "pct80",
    "pct90",
    "horz",
    "vert",
    "ltHorz",
    "ltVert",
    "dkHorz",
    "dkVert",
    "narHorz",
    "narVert",
    "dashHorz",
    "dashVert",
    "cross",
    "dnDiag",
    "upDiag",
    "ltDnDiag",
    "ltUpDiag",
    "dkDnDiag",
    "dkUpDiag",
    "wdDnDiag",
    "wdUpDiag",
    "dashDnDiag",
    "dashUpDiag",
    "diagCross",
    "smCheck",
    "lgCheck",
    "smGrid",
    "lgGrid",
    "dotGrid",
    "smConfetti",
    "lgConfetti",
    "horzBrick",
    "diagBrick",
    "solidDmnd",
    "openDmnd",
    "dotDmnd",
    "plaid",
    "sphere",
    "weave",
    "divot",
    "shingle",
    "wave",
    "trellis",
    "zigZag",
];

#[test]
fn pattern_type_round_trips_all_tokens() {
    assert_eq!(PATTERN_TYPE_TOKENS.len(), 54);
    assert_round_trip(
        PATTERN_TYPE_TOKENS,
        PatternType::from_wire,
        PatternType::to_wire,
    );

    // Comprehensive names map to the cryptic pattern tokens.
    assert_eq!(
        PatternType::from_wire("pct25"),
        Some(PatternType::Percent25)
    );
    assert_eq!(
        PatternType::from_wire("ltDnDiag"),
        Some(PatternType::LightDownwardDiagonal)
    );
    assert_eq!(
        PatternType::from_wire("smCheck"),
        Some(PatternType::SmallCheckerboard)
    );
    assert_eq!(PatternType::DiagonalCross.to_wire(), "diagCross");
    // An auto-expanded (no-override) token still resolves.
    assert_eq!(
        PatternType::from_wire("trellis"),
        Some(PatternType::Trellis)
    );
    // Unknown / future token: no panic, reported as absent.
    assert_eq!(PatternType::from_wire("notAPattern"), None);
    assert_eq!(
        PatternType::from_str("notAPattern").unwrap_err().value(),
        "notAPattern"
    );
}

#[test]
fn color_scheme_slot_round_trips_all_tokens() {
    let tokens = [
        "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5",
        "accent6", "hlink", "folHlink",
    ];
    assert_eq!(tokens.len(), 12);
    assert_round_trip(
        &tokens,
        ColorSchemeSlot::from_wire,
        ColorSchemeSlot::to_wire,
    );

    // Comprehensive names map to the cryptic dark/light/hyperlink tokens.
    assert_eq!(
        ColorSchemeSlot::from_wire("dk1"),
        Some(ColorSchemeSlot::Dark1)
    );
    assert_eq!(
        ColorSchemeSlot::from_wire("lt2"),
        Some(ColorSchemeSlot::Light2)
    );
    assert_eq!(
        ColorSchemeSlot::from_wire("folHlink"),
        Some(ColorSchemeSlot::FollowedHyperlink)
    );
    assert_eq!(ColorSchemeSlot::Accent1.to_wire(), "accent1");
    assert_eq!(ColorSchemeSlot::from_wire("phClr"), None); // phClr is not a scheme slot
    assert_eq!(ColorSchemeSlot::from_wire("bogus"), None);
}

#[test]
fn line_cap_round_trips_all_tokens() {
    // `ST_LineCap` (`a:ln@cap`), schema order.
    assert_round_trip(&["rnd", "sq", "flat"], LineCap::from_wire, LineCap::to_wire);
    assert_eq!(LineCap::from_wire("rnd"), Some(LineCap::Round));
    assert_eq!(LineCap::Square.to_wire(), "sq");
    assert_eq!(LineCap::from_wire("bogus"), None);
}

#[test]
fn compound_line_round_trips_all_tokens() {
    // `ST_CompoundLine` (`a:ln@cmpd`), schema order.
    assert_round_trip(
        &["sng", "dbl", "thickThin", "thinThick", "tri"],
        CompoundLine::from_wire,
        CompoundLine::to_wire,
    );
    assert_eq!(CompoundLine::from_wire("sng"), Some(CompoundLine::Single));
    assert_eq!(CompoundLine::Triple.to_wire(), "tri");
    // A well-formed token that auto-expands (no override) still resolves.
    assert_eq!(
        CompoundLine::from_wire("thickThin"),
        Some(CompoundLine::ThickThin)
    );
    assert_eq!(CompoundLine::from_wire("bogus"), None);
}

#[test]
fn pen_alignment_round_trips_all_tokens() {
    // `ST_PenAlignment` (`a:ln@algn`), schema order. `in` is a Rust keyword — the comprehensive
    // name `Inset` avoids it.
    assert_round_trip(
        &["ctr", "in"],
        PenAlignment::from_wire,
        PenAlignment::to_wire,
    );
    assert_eq!(PenAlignment::from_wire("ctr"), Some(PenAlignment::Center));
    assert_eq!(PenAlignment::from_wire("in"), Some(PenAlignment::Inset));
    assert_eq!(PenAlignment::Inset.to_wire(), "in");
    assert_eq!(PenAlignment::from_wire("bogus"), None);
}

/// Every `ST_PresetLineDashVal` wire token, in `dml-main.xsd` schema order (11 values).
const PRESET_LINE_DASH_TOKENS: &[&str] = &[
    "solid",
    "dot",
    "dash",
    "lgDash",
    "dashDot",
    "lgDashDot",
    "lgDashDotDot",
    "sysDash",
    "sysDot",
    "sysDashDot",
    "sysDashDotDot",
];

#[test]
fn preset_line_dash_round_trips_all_tokens() {
    assert_eq!(PRESET_LINE_DASH_TOKENS.len(), 11);
    assert_round_trip(
        PRESET_LINE_DASH_TOKENS,
        PresetLineDash::from_wire,
        PresetLineDash::to_wire,
    );

    // Comprehensive names map to the abbreviated dash tokens.
    assert_eq!(
        PresetLineDash::from_wire("lgDashDotDot"),
        Some(PresetLineDash::LargeDashDotDot)
    );
    assert_eq!(
        PresetLineDash::from_wire("sysDashDot"),
        Some(PresetLineDash::SystemDashDot)
    );
    assert_eq!(PresetLineDash::SystemDot.to_wire(), "sysDot");
    // An auto-expanded (no-override) token still resolves.
    assert_eq!(
        PresetLineDash::from_wire("dashDot"),
        Some(PresetLineDash::DashDot)
    );
    assert_eq!(PresetLineDash::from_wire("bogus"), None);
}

#[test]
fn line_end_enums_round_trip_all_tokens() {
    // `ST_LineEndType` (`a:headEnd`/`a:tailEnd@type`), schema order — every token auto-expands.
    assert_round_trip(
        &["none", "triangle", "stealth", "diamond", "oval", "arrow"],
        LineEndType::from_wire,
        LineEndType::to_wire,
    );
    assert_eq!(LineEndType::from_wire("arrow"), Some(LineEndType::Arrow));
    assert_eq!(LineEndType::from_wire("bogus"), None);

    // `ST_LineEndWidth` (`@w`) and `ST_LineEndLength` (`@len`) share the same three tokens.
    assert_round_trip(
        &["sm", "med", "lg"],
        LineEndWidth::from_wire,
        LineEndWidth::to_wire,
    );
    assert_round_trip(
        &["sm", "med", "lg"],
        LineEndLength::from_wire,
        LineEndLength::to_wire,
    );
    assert_eq!(LineEndWidth::from_wire("sm"), Some(LineEndWidth::Small));
    assert_eq!(LineEndLength::from_wire("lg"), Some(LineEndLength::Large));
    assert_eq!(LineEndWidth::Medium.to_wire(), "med");
    assert_eq!(LineEndLength::from_wire("bogus"), None);
}

#[test]
fn preset_shadow_round_trips_all_tokens() {
    // `ST_PresetShadowVal` (`a:prstShdw@prst`) — the 20 numbered presets, schema order.
    let tokens: Vec<String> = (1..=20).map(|n| format!("shdw{n}")).collect();
    let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    assert_eq!(token_refs.len(), 20);
    assert_round_trip(&token_refs, PresetShadow::from_wire, PresetShadow::to_wire);

    // Numbered names map to the `shdw{n}` tokens.
    assert_eq!(
        PresetShadow::from_wire("shdw1"),
        Some(PresetShadow::Shadow1)
    );
    assert_eq!(
        PresetShadow::from_wire("shdw20"),
        Some(PresetShadow::Shadow20)
    );
    assert_eq!(PresetShadow::Shadow13.to_wire(), "shdw13");
    assert_eq!(PresetShadow::from_wire("shdw21"), None);
}

#[test]
fn rectangle_alignment_round_trips_all_tokens() {
    // `ST_RectAlignment` (effect `@algn`), schema order.
    assert_round_trip(
        &["tl", "t", "tr", "l", "ctr", "r", "bl", "b", "br"],
        RectangleAlignment::from_wire,
        RectangleAlignment::to_wire,
    );

    // Compass-abbreviation tokens map to comprehensive names.
    assert_eq!(
        RectangleAlignment::from_wire("tl"),
        Some(RectangleAlignment::TopLeft)
    );
    assert_eq!(
        RectangleAlignment::from_wire("ctr"),
        Some(RectangleAlignment::Center)
    );
    assert_eq!(RectangleAlignment::BottomRight.to_wire(), "br");
    assert_eq!(RectangleAlignment::from_wire("bogus"), None);
}

#[test]
fn placeholder_enums_round_trip_all_tokens() {
    // `ST_PlaceholderType` (`p:ph@type`), schema order.
    assert_round_trip(
        &[
            "title", "body", "ctrTitle", "subTitle", "dt", "sldNum", "ftr", "hdr", "obj", "chart",
            "tbl", "clipArt", "dgm", "media", "sldImg", "pic",
        ],
        PlaceholderType::from_wire,
        PlaceholderType::to_wire,
    );
    // The cryptic tokens carry the ECMA-376 prose names.
    assert_eq!(
        PlaceholderType::from_wire("ctrTitle"),
        Some(PlaceholderType::CenteredTitle)
    );
    assert_eq!(
        PlaceholderType::from_wire("sldNum"),
        Some(PlaceholderType::SlideNumber)
    );
    assert_eq!(PlaceholderType::DateAndTime.to_wire(), "dt");
    assert_eq!(PlaceholderType::from_wire("bogus"), None);

    // `ST_PlaceholderSize` (`p:ph@sz`) and `ST_Direction` (`p:ph@orient`).
    assert_round_trip(
        &["full", "half", "quarter"],
        PlaceholderSize::from_wire,
        PlaceholderSize::to_wire,
    );
    assert_round_trip(
        &["horz", "vert"],
        Orientation::from_wire,
        Orientation::to_wire,
    );
    assert_eq!(
        Orientation::from_wire("vert"),
        Some(Orientation::Vertical),
        "the axis tokens must not stay abbreviated"
    );
}

#[test]
fn slide_layout_kind_round_trips_all_tokens() {
    // `ST_SlideLayoutType` (`p:sldLayout@type`), schema order (36 values).
    assert_round_trip(
        &[
            "title",
            "tx",
            "twoColTx",
            "tbl",
            "txAndChart",
            "chartAndTx",
            "dgm",
            "chart",
            "txAndClipArt",
            "clipArtAndTx",
            "titleOnly",
            "blank",
            "txAndObj",
            "objAndTx",
            "objOnly",
            "obj",
            "txAndMedia",
            "mediaAndTx",
            "objOverTx",
            "txOverObj",
            "txAndTwoObj",
            "twoObjAndTx",
            "twoObjOverTx",
            "fourObj",
            "vertTx",
            "clipArtAndVertTx",
            "vertTitleAndTx",
            "vertTitleAndTxOverChart",
            "twoObj",
            "objAndTwoObj",
            "twoObjAndObj",
            "cust",
            "secHead",
            "twoTxTwoObj",
            "objTx",
            "picTx",
        ],
        SlideLayoutKind::from_wire,
        SlideLayoutKind::to_wire,
    );
    // `obj` is "Title and Object" in the prose — not merely "object", which is `objOnly`.
    assert_eq!(
        SlideLayoutKind::from_wire("obj"),
        Some(SlideLayoutKind::TitleAndObject)
    );
    assert_eq!(
        SlideLayoutKind::from_wire("objOnly"),
        Some(SlideLayoutKind::ObjectOnly)
    );
    assert_eq!(SlideLayoutKind::SectionHeader.to_wire(), "secHead");
    assert_eq!(SlideLayoutKind::TwoColumnText.to_wire(), "twoColTx");
}

#[test]
fn slide_size_kind_round_trips_all_tokens() {
    // `ST_SlideSizeType` (`p:sldSz@type`), schema order.
    assert_round_trip(
        &[
            "screen4x3",
            "letter",
            "A4",
            "35mm",
            "overhead",
            "banner",
            "custom",
            "ledger",
            "A3",
            "B4ISO",
            "B5ISO",
            "B4JIS",
            "B5JIS",
            "hagakiCard",
            "screen16x9",
            "screen16x10",
        ],
        SlideSizeKind::from_wire,
        SlideSizeKind::to_wire,
    );
    // The digit-leading token needs a hand-given name; the wire spelling is untouched.
    assert_eq!(
        SlideSizeKind::from_wire("35mm"),
        Some(SlideSizeKind::Film35Mm)
    );
    assert_eq!(SlideSizeKind::Film35Mm.to_wire(), "35mm");
}

#[test]
fn text_run_property_enums_round_trip_all_tokens() {
    // `ST_TextUnderlineType` (`a:rPr@u`), schema order.
    assert_round_trip(
        &[
            "none",
            "words",
            "sng",
            "dbl",
            "heavy",
            "dotted",
            "dottedHeavy",
            "dash",
            "dashHeavy",
            "dashLong",
            "dashLongHeavy",
            "dotDash",
            "dotDashHeavy",
            "dotDotDash",
            "dotDotDashHeavy",
            "wavy",
            "wavyHeavy",
            "wavyDbl",
        ],
        TextUnderline::from_wire,
        TextUnderline::to_wire,
    );
    // The ECMA-376 §20.1.10.82 titles read modifier-first: `dashHeavy` is "Heavy Dashed".
    assert_eq!(TextUnderline::from_wire("sng"), Some(TextUnderline::Single));
    assert_eq!(
        TextUnderline::from_wire("dashHeavy"),
        Some(TextUnderline::HeavyDashed)
    );
    assert_eq!(TextUnderline::DoubleWavy.to_wire(), "wavyDbl");

    // `ST_TextStrikeType` (`a:rPr@strike`).
    assert_round_trip(
        &["noStrike", "sngStrike", "dblStrike"],
        TextStrike::from_wire,
        TextStrike::to_wire,
    );
    assert_eq!(
        TextStrike::from_wire("dblStrike"),
        Some(TextStrike::DoubleStrike)
    );

    // `ST_TextCapsType` (`a:rPr@cap`) — `none` is an explicit override, not an absence.
    assert_round_trip(
        &["none", "small", "all"],
        TextCapitalization::from_wire,
        TextCapitalization::to_wire,
    );
    assert_eq!(TextCapitalization::Small.to_wire(), "small");
}

#[test]
fn text_paragraph_property_enums_round_trip_all_tokens() {
    // `ST_TextAlignType` (`a:pPr@algn`), schema order.
    assert_round_trip(
        &["l", "ctr", "r", "just", "justLow", "dist", "thaiDist"],
        TextAlignment::from_wire,
        TextAlignment::to_wire,
    );
    assert_eq!(TextAlignment::from_wire("l"), Some(TextAlignment::Left));
    assert_eq!(TextAlignment::ThaiDistributed.to_wire(), "thaiDist");

    // `ST_TextFontAlignType` (`a:pPr@fontAlgn`) — where letters sit relative to the baselines.
    assert_round_trip(
        &["auto", "t", "ctr", "base", "b"],
        FontAlignment::from_wire,
        FontAlignment::to_wire,
    );
    assert_eq!(
        FontAlignment::from_wire("base"),
        Some(FontAlignment::Baseline)
    );
    assert_eq!(FontAlignment::Automatic.to_wire(), "auto");

    // `ST_TextTabAlignType` (`a:tab@algn`).
    assert_round_trip(
        &["l", "ctr", "r", "dec"],
        TabAlignment::from_wire,
        TabAlignment::to_wire,
    );
    assert_eq!(TabAlignment::from_wire("dec"), Some(TabAlignment::Decimal));
}

#[test]
fn autonumber_scheme_round_trips_all_tokens() {
    // `ST_TextAutonumberScheme` (`a:buAutoNum@type`) — all 41 values, schema order.
    let tokens = [
        "alphaLcParenBoth",
        "alphaUcParenBoth",
        "alphaLcParenR",
        "alphaUcParenR",
        "alphaLcPeriod",
        "alphaUcPeriod",
        "arabicParenBoth",
        "arabicParenR",
        "arabicPeriod",
        "arabicPlain",
        "romanLcParenBoth",
        "romanUcParenBoth",
        "romanLcParenR",
        "romanUcParenR",
        "romanLcPeriod",
        "romanUcPeriod",
        "circleNumDbPlain",
        "circleNumWdBlackPlain",
        "circleNumWdWhitePlain",
        "arabicDbPeriod",
        "arabicDbPlain",
        "ea1ChsPeriod",
        "ea1ChsPlain",
        "ea1ChtPeriod",
        "ea1ChtPlain",
        "ea1JpnChsDbPeriod",
        "ea1JpnKorPlain",
        "ea1JpnKorPeriod",
        "arabic1Minus",
        "arabic2Minus",
        "hebrew2Minus",
        "thaiAlphaPeriod",
        "thaiAlphaParenR",
        "thaiAlphaParenBoth",
        "thaiNumPeriod",
        "thaiNumParenR",
        "thaiNumParenBoth",
        "hindiAlphaPeriod",
        "hindiNumPeriod",
        "hindiNumParenR",
        "hindiAlpha1Period",
    ];
    assert_eq!(tokens.len(), 41);
    assert_round_trip(
        &tokens,
        AutonumberScheme::from_wire,
        AutonumberScheme::to_wire,
    );

    // The enumeration table's titles only repeat the token, so names come from the Description
    // column: `alphaLcParenBoth` is "(a), (b), (c), …" and `romanUcPeriod` is "I., II., III., …".
    assert_eq!(
        AutonumberScheme::from_wire("alphaLcParenBoth"),
        Some(AutonumberScheme::LowercaseLetterParenthesesBoth)
    );
    assert_eq!(
        AutonumberScheme::from_wire("romanUcPeriod"),
        Some(AutonumberScheme::UppercaseRomanPeriod)
    );
    // "EA: Simplified Chinese w/ single-byte period"; `ea1` is a family prefix, not a numeral.
    assert_eq!(
        AutonumberScheme::from_wire("ea1ChsPeriod"),
        Some(AutonumberScheme::SimplifiedChinesePeriod)
    );
    // "Bidi Arabic 1 (AraAlpha) / 2 (AraAbjad) with ANSI minus symbol".
    assert_eq!(
        AutonumberScheme::BidirectionalArabicAbjadMinus.to_wire(),
        "arabic2Minus"
    );
    // "Hindi alphabet period - vowels" vs "- consonants".
    assert_eq!(
        AutonumberScheme::from_wire("hindiAlpha1Period"),
        Some(AutonumberScheme::HindiConsonantPeriod)
    );
    assert_eq!(AutonumberScheme::from_wire("bogus"), None);
}

#[test]
fn from_str_reports_unknown_values() {
    assert_eq!(
        ConformanceClass::from_str("strict"),
        Ok(ConformanceClass::Strict)
    );
    let err = ConformanceClass::from_str("loose").unwrap_err();
    assert_eq!(err.value(), "loose");
}

#[test]
fn on_off_family_normalizes_via_support() {
    use mjx_ooxml_types::on_off;
    // ST_OnOff accepts many spellings but collapses to two values.
    assert_eq!(on_off::from_wire("1"), Some(true));
    assert_eq!(on_off::from_wire("on"), Some(true));
    assert_eq!(on_off::from_wire("false"), Some(false));
    assert_eq!(on_off::to_wire(true), "true");
}

#[test]
fn namespaces_are_paired_across_worlds() {
    assert_eq!(
        namespaces::DML_MAIN.transitional,
        "http://schemas.openxmlformats.org/drawingml/2006/main"
    );
    assert_eq!(
        namespaces::DML_MAIN.strict,
        Some("http://purl.oclc.org/ooxml/drawingml/main")
    );
    // for_strict falls back to Transitional when no Strict variant exists.
    assert_eq!(
        namespaces::DML_MAIN.for_strict(true),
        "http://purl.oclc.org/ooxml/drawingml/main"
    );
    assert!(!namespaces::ALL.is_empty());
}

// ---------------------------------------------------------------------------------------------
// WordprocessingML and Office Math — the overridden tokens
//
// Seeded from the naming tables, not from the easy cases. A variant that is a mechanical
// PascalCase of its token round-trips whatever the generator does, so testing those proves only
// that the generator emitted something. These are the values where a human wrote the name down
// from the ECMA-376 prose, so they are the ones that can be wrong. Each assertion pins a *named*
// variant to *exact* wire bytes in both directions: rename an override and the test stops
// compiling; swap two of them and it fails.
// ---------------------------------------------------------------------------------------------

/// `ST_TextEffect` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_text_effect_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::TextEffect;
    assert_eq!(
        TextEffect::from_wire("antsBlack"),
        Some(TextEffect::BlackDashedLine)
    );
    assert_eq!(TextEffect::BlackDashedLine.to_wire(), "antsBlack");
    assert_eq!(
        TextEffect::from_wire("antsRed"),
        Some(TextEffect::MarchingRedAnts)
    );
    assert_eq!(TextEffect::MarchingRedAnts.to_wire(), "antsRed");
    assert_eq!(
        TextEffect::from_wire("blinkBackground"),
        Some(TextEffect::BlinkingBackground)
    );
    assert_eq!(TextEffect::BlinkingBackground.to_wire(), "blinkBackground");
    assert_eq!(
        TextEffect::from_wire("lights"),
        Some(TextEffect::ColoredLights)
    );
    assert_eq!(TextEffect::ColoredLights.to_wire(), "lights");
    assert_eq!(
        TextEffect::from_wire("sparkle"),
        Some(TextEffect::SparklingLights)
    );
    assert_eq!(TextEffect::SparklingLights.to_wire(), "sparkle");
}

/// `ST_Border` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_border_style_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::BorderStyle;
    assert_eq!(
        BorderStyle::from_wire("threeDEmboss"),
        Some(BorderStyle::ThreeDEmboss)
    );
    assert_eq!(BorderStyle::ThreeDEmboss.to_wire(), "threeDEmboss");
    assert_eq!(
        BorderStyle::from_wire("threeDEngrave"),
        Some(BorderStyle::ThreeDEngrave)
    );
    assert_eq!(BorderStyle::ThreeDEngrave.to_wire(), "threeDEngrave");
}

/// `ST_Shd` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_shading_pattern_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::ShadingPattern;
    assert_eq!(
        ShadingPattern::from_wire("pct5"),
        Some(ShadingPattern::Percent5)
    );
    assert_eq!(ShadingPattern::Percent5.to_wire(), "pct5");
    assert_eq!(
        ShadingPattern::from_wire("pct10"),
        Some(ShadingPattern::Percent10)
    );
    assert_eq!(ShadingPattern::Percent10.to_wire(), "pct10");
    assert_eq!(
        ShadingPattern::from_wire("pct12"),
        Some(ShadingPattern::Percent12Point5)
    );
    assert_eq!(ShadingPattern::Percent12Point5.to_wire(), "pct12");
    assert_eq!(
        ShadingPattern::from_wire("pct15"),
        Some(ShadingPattern::Percent15)
    );
    assert_eq!(ShadingPattern::Percent15.to_wire(), "pct15");
    assert_eq!(
        ShadingPattern::from_wire("pct20"),
        Some(ShadingPattern::Percent20)
    );
    assert_eq!(ShadingPattern::Percent20.to_wire(), "pct20");
    assert_eq!(
        ShadingPattern::from_wire("pct25"),
        Some(ShadingPattern::Percent25)
    );
    assert_eq!(ShadingPattern::Percent25.to_wire(), "pct25");
    assert_eq!(
        ShadingPattern::from_wire("pct30"),
        Some(ShadingPattern::Percent30)
    );
    assert_eq!(ShadingPattern::Percent30.to_wire(), "pct30");
    assert_eq!(
        ShadingPattern::from_wire("pct35"),
        Some(ShadingPattern::Percent35)
    );
    assert_eq!(ShadingPattern::Percent35.to_wire(), "pct35");
    assert_eq!(
        ShadingPattern::from_wire("pct37"),
        Some(ShadingPattern::Percent37Point5)
    );
    assert_eq!(ShadingPattern::Percent37Point5.to_wire(), "pct37");
    assert_eq!(
        ShadingPattern::from_wire("pct40"),
        Some(ShadingPattern::Percent40)
    );
    assert_eq!(ShadingPattern::Percent40.to_wire(), "pct40");
    assert_eq!(
        ShadingPattern::from_wire("pct45"),
        Some(ShadingPattern::Percent45)
    );
    assert_eq!(ShadingPattern::Percent45.to_wire(), "pct45");
    assert_eq!(
        ShadingPattern::from_wire("pct50"),
        Some(ShadingPattern::Percent50)
    );
    assert_eq!(ShadingPattern::Percent50.to_wire(), "pct50");
    assert_eq!(
        ShadingPattern::from_wire("pct55"),
        Some(ShadingPattern::Percent55)
    );
    assert_eq!(ShadingPattern::Percent55.to_wire(), "pct55");
    assert_eq!(
        ShadingPattern::from_wire("pct60"),
        Some(ShadingPattern::Percent60)
    );
    assert_eq!(ShadingPattern::Percent60.to_wire(), "pct60");
    assert_eq!(
        ShadingPattern::from_wire("pct62"),
        Some(ShadingPattern::Percent62Point5)
    );
    assert_eq!(ShadingPattern::Percent62Point5.to_wire(), "pct62");
    assert_eq!(
        ShadingPattern::from_wire("pct65"),
        Some(ShadingPattern::Percent65)
    );
    assert_eq!(ShadingPattern::Percent65.to_wire(), "pct65");
    assert_eq!(
        ShadingPattern::from_wire("pct70"),
        Some(ShadingPattern::Percent70)
    );
    assert_eq!(ShadingPattern::Percent70.to_wire(), "pct70");
    assert_eq!(
        ShadingPattern::from_wire("pct75"),
        Some(ShadingPattern::Percent75)
    );
    assert_eq!(ShadingPattern::Percent75.to_wire(), "pct75");
    assert_eq!(
        ShadingPattern::from_wire("pct80"),
        Some(ShadingPattern::Percent80)
    );
    assert_eq!(ShadingPattern::Percent80.to_wire(), "pct80");
    assert_eq!(
        ShadingPattern::from_wire("pct85"),
        Some(ShadingPattern::Percent85)
    );
    assert_eq!(ShadingPattern::Percent85.to_wire(), "pct85");
    assert_eq!(
        ShadingPattern::from_wire("pct87"),
        Some(ShadingPattern::Percent87Point5)
    );
    assert_eq!(ShadingPattern::Percent87Point5.to_wire(), "pct87");
    assert_eq!(
        ShadingPattern::from_wire("pct90"),
        Some(ShadingPattern::Percent90)
    );
    assert_eq!(ShadingPattern::Percent90.to_wire(), "pct90");
    assert_eq!(
        ShadingPattern::from_wire("pct95"),
        Some(ShadingPattern::Percent95)
    );
    assert_eq!(ShadingPattern::Percent95.to_wire(), "pct95");
}

/// `ST_TabJc` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_tab_stop_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::TabStopType;
    assert_eq!(TabStopType::from_wire("num"), Some(TabStopType::List));
    assert_eq!(TabStopType::List.to_wire(), "num");
}

/// `ST_Jc` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_justification_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::Justification;
    assert_eq!(
        Justification::from_wire("both"),
        Some(Justification::Justified)
    );
    assert_eq!(Justification::Justified.to_wire(), "both");
    assert_eq!(
        Justification::from_wire("numTab"),
        Some(Justification::AlignToListTab)
    );
    assert_eq!(Justification::AlignToListTab.to_wire(), "numTab");
    assert_eq!(
        Justification::from_wire("highKashida"),
        Some(Justification::WidestKashida)
    );
    assert_eq!(Justification::WidestKashida.to_wire(), "highKashida");
}

/// `ST_MailMergeOdsoFMDFieldType` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_mail_merge_field_mapping_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::MailMergeFieldMappingType;
    assert_eq!(
        MailMergeFieldMappingType::from_wire("dbColumn"),
        Some(MailMergeFieldMappingType::DatabaseColumn)
    );
    assert_eq!(
        MailMergeFieldMappingType::DatabaseColumn.to_wire(),
        "dbColumn"
    );
}

/// `ST_TextDirection` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_text_flow_direction_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::TextFlowDirection;
    assert_eq!(
        TextFlowDirection::from_wire("tb"),
        Some(TextFlowDirection::TopToBottom)
    );
    assert_eq!(TextFlowDirection::TopToBottom.to_wire(), "tb");
    assert_eq!(
        TextFlowDirection::from_wire("rl"),
        Some(TextFlowDirection::RightToLeft)
    );
    assert_eq!(TextFlowDirection::RightToLeft.to_wire(), "rl");
    assert_eq!(
        TextFlowDirection::from_wire("lr"),
        Some(TextFlowDirection::LeftToRight)
    );
    assert_eq!(TextFlowDirection::LeftToRight.to_wire(), "lr");
    assert_eq!(
        TextFlowDirection::from_wire("tbV"),
        Some(TextFlowDirection::TopToBottomRotated)
    );
    assert_eq!(TextFlowDirection::TopToBottomRotated.to_wire(), "tbV");
    assert_eq!(
        TextFlowDirection::from_wire("rlV"),
        Some(TextFlowDirection::RightToLeftRotated)
    );
    assert_eq!(TextFlowDirection::RightToLeftRotated.to_wire(), "rlV");
    assert_eq!(
        TextFlowDirection::from_wire("lrV"),
        Some(TextFlowDirection::LeftToRightRotated)
    );
    assert_eq!(TextFlowDirection::LeftToRightRotated.to_wire(), "lrV");
    assert_eq!(
        TextFlowDirection::from_wire("btLr"),
        Some(TextFlowDirection::BottomToTopLeftToRight)
    );
    assert_eq!(TextFlowDirection::BottomToTopLeftToRight.to_wire(), "btLr");
    assert_eq!(
        TextFlowDirection::from_wire("lrTb"),
        Some(TextFlowDirection::LeftToRightTopToBottom)
    );
    assert_eq!(TextFlowDirection::LeftToRightTopToBottom.to_wire(), "lrTb");
    assert_eq!(
        TextFlowDirection::from_wire("lrTbV"),
        Some(TextFlowDirection::LeftToRightTopToBottomRotated)
    );
    assert_eq!(
        TextFlowDirection::LeftToRightTopToBottomRotated.to_wire(),
        "lrTbV"
    );
    assert_eq!(
        TextFlowDirection::from_wire("tbLrV"),
        Some(TextFlowDirection::TopToBottomLeftToRightRotated)
    );
    assert_eq!(
        TextFlowDirection::TopToBottomLeftToRightRotated.to_wire(),
        "tbLrV"
    );
    assert_eq!(
        TextFlowDirection::from_wire("tbRl"),
        Some(TextFlowDirection::TopToBottomRightToLeft)
    );
    assert_eq!(TextFlowDirection::TopToBottomRightToLeft.to_wire(), "tbRl");
    assert_eq!(
        TextFlowDirection::from_wire("tbRlV"),
        Some(TextFlowDirection::TopToBottomRightToLeftRotated)
    );
    assert_eq!(
        TextFlowDirection::TopToBottomRightToLeftRotated.to_wire(),
        "tbRlV"
    );
}

/// `ST_DisplacedByCustomXml` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_displaced_by_custom_xml_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::DisplacedByCustomXml;
    assert_eq!(
        DisplacedByCustomXml::from_wire("prev"),
        Some(DisplacedByCustomXml::Previous)
    );
    assert_eq!(DisplacedByCustomXml::Previous.to_wire(), "prev");
}

/// `ST_AnnotationVMerge` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_vertical_merge_revision_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::VerticalMergeRevision;
    assert_eq!(
        VerticalMergeRevision::from_wire("cont"),
        Some(VerticalMergeRevision::Merged)
    );
    assert_eq!(VerticalMergeRevision::Merged.to_wire(), "cont");
    assert_eq!(
        VerticalMergeRevision::from_wire("rest"),
        Some(VerticalMergeRevision::Split)
    );
    assert_eq!(VerticalMergeRevision::Split.to_wire(), "rest");
}

/// `ST_NumberFormat` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_number_format_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::NumberFormat;
    assert_eq!(
        NumberFormat::from_wire("hex"),
        Some(NumberFormat::Hexadecimal)
    );
    assert_eq!(NumberFormat::Hexadecimal.to_wire(), "hex");
    assert_eq!(
        NumberFormat::from_wire("chicago"),
        Some(NumberFormat::ChicagoManualOfStyle)
    );
    assert_eq!(NumberFormat::ChicagoManualOfStyle.to_wire(), "chicago");
    assert_eq!(
        NumberFormat::from_wire("aiueo"),
        Some(NumberFormat::HalfWidthKatakanaAiueo)
    );
    assert_eq!(NumberFormat::HalfWidthKatakanaAiueo.to_wire(), "aiueo");
    assert_eq!(
        NumberFormat::from_wire("aiueoFullWidth"),
        Some(NumberFormat::FullWidthKatakanaAiueo)
    );
    assert_eq!(
        NumberFormat::FullWidthKatakanaAiueo.to_wire(),
        "aiueoFullWidth"
    );
    assert_eq!(
        NumberFormat::from_wire("iroha"),
        Some(NumberFormat::KatakanaIroha)
    );
    assert_eq!(NumberFormat::KatakanaIroha.to_wire(), "iroha");
    assert_eq!(
        NumberFormat::from_wire("irohaFullWidth"),
        Some(NumberFormat::FullWidthKatakanaIroha)
    );
    assert_eq!(
        NumberFormat::FullWidthKatakanaIroha.to_wire(),
        "irohaFullWidth"
    );
    assert_eq!(
        NumberFormat::from_wire("ganada"),
        Some(NumberFormat::KoreanGanada)
    );
    assert_eq!(NumberFormat::KoreanGanada.to_wire(), "ganada");
    assert_eq!(
        NumberFormat::from_wire("chosung"),
        Some(NumberFormat::KoreanChosung)
    );
    assert_eq!(NumberFormat::KoreanChosung.to_wire(), "chosung");
    assert_eq!(
        NumberFormat::from_wire("bahtText"),
        Some(NumberFormat::ThaiBahtText)
    );
    assert_eq!(NumberFormat::ThaiBahtText.to_wire(), "bahtText");
    assert_eq!(
        NumberFormat::from_wire("hebrew1"),
        Some(NumberFormat::HebrewLetters)
    );
    assert_eq!(NumberFormat::HebrewLetters.to_wire(), "hebrew1");
    assert_eq!(
        NumberFormat::from_wire("hebrew2"),
        Some(NumberFormat::HebrewAlphabet)
    );
    assert_eq!(NumberFormat::HebrewAlphabet.to_wire(), "hebrew2");
    assert_eq!(
        NumberFormat::from_wire("arabicAlpha"),
        Some(NumberFormat::ArabicAlphabet)
    );
    assert_eq!(NumberFormat::ArabicAlphabet.to_wire(), "arabicAlpha");
    assert_eq!(
        NumberFormat::from_wire("arabicAbjad"),
        Some(NumberFormat::ArabicAbjadNumerals)
    );
    assert_eq!(NumberFormat::ArabicAbjadNumerals.to_wire(), "arabicAbjad");
    assert_eq!(
        NumberFormat::from_wire("upperLetter"),
        Some(NumberFormat::UppercaseLatinAlphabet)
    );
    assert_eq!(
        NumberFormat::UppercaseLatinAlphabet.to_wire(),
        "upperLetter"
    );
    assert_eq!(
        NumberFormat::from_wire("lowerLetter"),
        Some(NumberFormat::LowercaseLatinAlphabet)
    );
    assert_eq!(
        NumberFormat::LowercaseLatinAlphabet.to_wire(),
        "lowerLetter"
    );
    assert_eq!(
        NumberFormat::from_wire("upperRoman"),
        Some(NumberFormat::UppercaseRomanNumerals)
    );
    assert_eq!(NumberFormat::UppercaseRomanNumerals.to_wire(), "upperRoman");
    assert_eq!(
        NumberFormat::from_wire("lowerRoman"),
        Some(NumberFormat::LowercaseRomanNumerals)
    );
    assert_eq!(NumberFormat::LowercaseRomanNumerals.to_wire(), "lowerRoman");
    assert_eq!(
        NumberFormat::from_wire("russianUpper"),
        Some(NumberFormat::UppercaseRussianAlphabet)
    );
    assert_eq!(
        NumberFormat::UppercaseRussianAlphabet.to_wire(),
        "russianUpper"
    );
    assert_eq!(
        NumberFormat::from_wire("russianLower"),
        Some(NumberFormat::LowercaseRussianAlphabet)
    );
    assert_eq!(
        NumberFormat::LowercaseRussianAlphabet.to_wire(),
        "russianLower"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalFullWidth"),
        Some(NumberFormat::FullWidthArabicNumerals)
    );
    assert_eq!(
        NumberFormat::FullWidthArabicNumerals.to_wire(),
        "decimalFullWidth"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalHalfWidth"),
        Some(NumberFormat::HalfWidthArabicNumerals)
    );
    assert_eq!(
        NumberFormat::HalfWidthArabicNumerals.to_wire(),
        "decimalHalfWidth"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalZero"),
        Some(NumberFormat::InitialZeroArabicNumerals)
    );
    assert_eq!(
        NumberFormat::InitialZeroArabicNumerals.to_wire(),
        "decimalZero"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalFullWidth2"),
        Some(NumberFormat::FullWidthArabicNumeralsAlternate)
    );
    assert_eq!(
        NumberFormat::FullWidthArabicNumeralsAlternate.to_wire(),
        "decimalFullWidth2"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalEnclosedCircle"),
        Some(NumberFormat::DecimalEnclosedInCircle)
    );
    assert_eq!(
        NumberFormat::DecimalEnclosedInCircle.to_wire(),
        "decimalEnclosedCircle"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalEnclosedCircleChinese"),
        Some(NumberFormat::DecimalEnclosedInCircleChinese)
    );
    assert_eq!(
        NumberFormat::DecimalEnclosedInCircleChinese.to_wire(),
        "decimalEnclosedCircleChinese"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalEnclosedFullstop"),
        Some(NumberFormat::DecimalFollowedByPeriod)
    );
    assert_eq!(
        NumberFormat::DecimalFollowedByPeriod.to_wire(),
        "decimalEnclosedFullstop"
    );
    assert_eq!(
        NumberFormat::from_wire("decimalEnclosedParen"),
        Some(NumberFormat::DecimalEnclosedInParenthesis)
    );
    assert_eq!(
        NumberFormat::DecimalEnclosedInParenthesis.to_wire(),
        "decimalEnclosedParen"
    );
    assert_eq!(
        NumberFormat::from_wire("ideographEnclosedCircle"),
        Some(NumberFormat::IdeographEnclosedInCircle)
    );
    assert_eq!(
        NumberFormat::IdeographEnclosedInCircle.to_wire(),
        "ideographEnclosedCircle"
    );
    assert_eq!(
        NumberFormat::from_wire("ideographTraditional"),
        Some(NumberFormat::TraditionalIdeograph)
    );
    assert_eq!(
        NumberFormat::TraditionalIdeograph.to_wire(),
        "ideographTraditional"
    );
    assert_eq!(
        NumberFormat::from_wire("ideographZodiac"),
        Some(NumberFormat::ZodiacIdeograph)
    );
    assert_eq!(NumberFormat::ZodiacIdeograph.to_wire(), "ideographZodiac");
    assert_eq!(
        NumberFormat::from_wire("ideographZodiacTraditional"),
        Some(NumberFormat::TraditionalZodiacIdeograph)
    );
    assert_eq!(
        NumberFormat::TraditionalZodiacIdeograph.to_wire(),
        "ideographZodiacTraditional"
    );
    assert_eq!(
        NumberFormat::from_wire("ideographLegalTraditional"),
        Some(NumberFormat::TraditionalLegalIdeograph)
    );
    assert_eq!(
        NumberFormat::TraditionalLegalIdeograph.to_wire(),
        "ideographLegalTraditional"
    );
    assert_eq!(
        NumberFormat::from_wire("koreanDigital2"),
        Some(NumberFormat::KoreanDigitalAlternate)
    );
    assert_eq!(
        NumberFormat::KoreanDigitalAlternate.to_wire(),
        "koreanDigital2"
    );
    assert_eq!(
        NumberFormat::from_wire("numberInDash"),
        Some(NumberFormat::NumberWithDashes)
    );
    assert_eq!(NumberFormat::NumberWithDashes.to_wire(), "numberInDash");
    assert_eq!(
        NumberFormat::from_wire("thaiNumbers"),
        Some(NumberFormat::ThaiNumerals)
    );
    assert_eq!(NumberFormat::ThaiNumerals.to_wire(), "thaiNumbers");
    assert_eq!(
        NumberFormat::from_wire("vietnameseCounting"),
        Some(NumberFormat::VietnameseNumerals)
    );
    assert_eq!(
        NumberFormat::VietnameseNumerals.to_wire(),
        "vietnameseCounting"
    );
}

/// `ST_VerticalJc` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_vertical_justification_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::VerticalJustification;
    assert_eq!(
        VerticalJustification::from_wire("both"),
        Some(VerticalJustification::Justified)
    );
    assert_eq!(VerticalJustification::Justified.to_wire(), "both");
}

/// `ST_ProofErr` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_proofing_error_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::ProofingErrorType;
    assert_eq!(
        ProofingErrorType::from_wire("spellStart"),
        Some(ProofingErrorType::SpellingStart)
    );
    assert_eq!(ProofingErrorType::SpellingStart.to_wire(), "spellStart");
    assert_eq!(
        ProofingErrorType::from_wire("spellEnd"),
        Some(ProofingErrorType::SpellingEnd)
    );
    assert_eq!(ProofingErrorType::SpellingEnd.to_wire(), "spellEnd");
    assert_eq!(
        ProofingErrorType::from_wire("gramStart"),
        Some(ProofingErrorType::GrammarStart)
    );
    assert_eq!(ProofingErrorType::GrammarStart.to_wire(), "gramStart");
    assert_eq!(
        ProofingErrorType::from_wire("gramEnd"),
        Some(ProofingErrorType::GrammarEnd)
    );
    assert_eq!(ProofingErrorType::GrammarEnd.to_wire(), "gramEnd");
}

/// `ST_Theme` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_theme_font_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::ThemeFont;
    assert_eq!(
        ThemeFont::from_wire("majorAscii"),
        Some(ThemeFont::MajorAscii)
    );
    assert_eq!(ThemeFont::MajorAscii.to_wire(), "majorAscii");
    assert_eq!(
        ThemeFont::from_wire("majorBidi"),
        Some(ThemeFont::MajorComplexScript)
    );
    assert_eq!(ThemeFont::MajorComplexScript.to_wire(), "majorBidi");
    assert_eq!(
        ThemeFont::from_wire("majorHAnsi"),
        Some(ThemeFont::MajorHighAnsi)
    );
    assert_eq!(ThemeFont::MajorHighAnsi.to_wire(), "majorHAnsi");
    assert_eq!(
        ThemeFont::from_wire("minorAscii"),
        Some(ThemeFont::MinorAscii)
    );
    assert_eq!(ThemeFont::MinorAscii.to_wire(), "minorAscii");
    assert_eq!(
        ThemeFont::from_wire("minorBidi"),
        Some(ThemeFont::MinorComplexScript)
    );
    assert_eq!(ThemeFont::MinorComplexScript.to_wire(), "minorBidi");
    assert_eq!(
        ThemeFont::from_wire("minorHAnsi"),
        Some(ThemeFont::MinorHighAnsi)
    );
    assert_eq!(ThemeFont::MinorHighAnsi.to_wire(), "minorHAnsi");
}

/// `ST_Lock` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_locking_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::LockingType;
    assert_eq!(
        LockingType::from_wire("sdtLocked"),
        Some(LockingType::TagCannotBeDeleted)
    );
    assert_eq!(LockingType::TagCannotBeDeleted.to_wire(), "sdtLocked");
    assert_eq!(
        LockingType::from_wire("contentLocked"),
        Some(LockingType::ContentsCannotBeEdited)
    );
    assert_eq!(
        LockingType::ContentsCannotBeEdited.to_wire(),
        "contentLocked"
    );
    assert_eq!(
        LockingType::from_wire("sdtContentLocked"),
        Some(LockingType::ContentsCannotBeEditedAndTagCannotBeDeleted)
    );
    assert_eq!(
        LockingType::ContentsCannotBeEditedAndTagCannotBeDeleted.to_wire(),
        "sdtContentLocked"
    );
}

/// `ST_Direction` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_bidirectional_direction_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::BidirectionalDirection;
    assert_eq!(
        BidirectionalDirection::from_wire("ltr"),
        Some(BidirectionalDirection::LeftToRight)
    );
    assert_eq!(BidirectionalDirection::LeftToRight.to_wire(), "ltr");
    assert_eq!(
        BidirectionalDirection::from_wire("rtl"),
        Some(BidirectionalDirection::RightToLeft)
    );
    assert_eq!(BidirectionalDirection::RightToLeft.to_wire(), "rtl");
}

/// `ST_TblWidth` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_table_width_unit_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::TableWidthUnit;
    assert_eq!(
        TableWidthUnit::from_wire("pct"),
        Some(TableWidthUnit::Percent)
    );
    assert_eq!(TableWidthUnit::Percent.to_wire(), "pct");
    assert_eq!(
        TableWidthUnit::from_wire("dxa"),
        Some(TableWidthUnit::Twips)
    );
    assert_eq!(TableWidthUnit::Twips.to_wire(), "dxa");
}

/// `ST_FtnPos` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_footnote_position_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::FootnotePosition;
    assert_eq!(
        FootnotePosition::from_wire("sectEnd"),
        Some(FootnotePosition::SectionEnd)
    );
    assert_eq!(FootnotePosition::SectionEnd.to_wire(), "sectEnd");
    assert_eq!(
        FootnotePosition::from_wire("docEnd"),
        Some(FootnotePosition::DocumentEnd)
    );
    assert_eq!(FootnotePosition::DocumentEnd.to_wire(), "docEnd");
}

/// `ST_EdnPos` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_endnote_position_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::EndnotePosition;
    assert_eq!(
        EndnotePosition::from_wire("sectEnd"),
        Some(EndnotePosition::SectionEnd)
    );
    assert_eq!(EndnotePosition::SectionEnd.to_wire(), "sectEnd");
    assert_eq!(
        EndnotePosition::from_wire("docEnd"),
        Some(EndnotePosition::DocumentEnd)
    );
    assert_eq!(EndnotePosition::DocumentEnd.to_wire(), "docEnd");
}

/// `ST_RestartNumber` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_numbering_restart_location_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::NumberingRestartLocation;
    assert_eq!(
        NumberingRestartLocation::from_wire("eachSect"),
        Some(NumberingRestartLocation::EachSection)
    );
    assert_eq!(NumberingRestartLocation::EachSection.to_wire(), "eachSect");
}

/// `ST_TargetScreenSz` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_target_screen_size_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::TargetScreenSize;
    assert_eq!(
        TargetScreenSize::from_wire("544x376"),
        Some(TargetScreenSize::Pixels544By376)
    );
    assert_eq!(TargetScreenSize::Pixels544By376.to_wire(), "544x376");
    assert_eq!(
        TargetScreenSize::from_wire("640x480"),
        Some(TargetScreenSize::Pixels640By480)
    );
    assert_eq!(TargetScreenSize::Pixels640By480.to_wire(), "640x480");
    assert_eq!(
        TargetScreenSize::from_wire("720x512"),
        Some(TargetScreenSize::Pixels720By512)
    );
    assert_eq!(TargetScreenSize::Pixels720By512.to_wire(), "720x512");
    assert_eq!(
        TargetScreenSize::from_wire("800x600"),
        Some(TargetScreenSize::Pixels800By600)
    );
    assert_eq!(TargetScreenSize::Pixels800By600.to_wire(), "800x600");
    assert_eq!(
        TargetScreenSize::from_wire("1024x768"),
        Some(TargetScreenSize::Pixels1024By768)
    );
    assert_eq!(TargetScreenSize::Pixels1024By768.to_wire(), "1024x768");
    assert_eq!(
        TargetScreenSize::from_wire("1152x882"),
        Some(TargetScreenSize::Pixels1152By882)
    );
    assert_eq!(TargetScreenSize::Pixels1152By882.to_wire(), "1152x882");
    assert_eq!(
        TargetScreenSize::from_wire("1152x900"),
        Some(TargetScreenSize::Pixels1152By900)
    );
    assert_eq!(TargetScreenSize::Pixels1152By900.to_wire(), "1152x900");
    assert_eq!(
        TargetScreenSize::from_wire("1280x1024"),
        Some(TargetScreenSize::Pixels1280By1024)
    );
    assert_eq!(TargetScreenSize::Pixels1280By1024.to_wire(), "1280x1024");
    assert_eq!(
        TargetScreenSize::from_wire("1600x1200"),
        Some(TargetScreenSize::Pixels1600By1200)
    );
    assert_eq!(TargetScreenSize::Pixels1600By1200.to_wire(), "1600x1200");
    assert_eq!(
        TargetScreenSize::from_wire("1800x1440"),
        Some(TargetScreenSize::Pixels1800By1440)
    );
    assert_eq!(TargetScreenSize::Pixels1800By1440.to_wire(), "1800x1440");
    assert_eq!(
        TargetScreenSize::from_wire("1920x1200"),
        Some(TargetScreenSize::Pixels1920By1200)
    );
    assert_eq!(TargetScreenSize::Pixels1920By1200.to_wire(), "1920x1200");
}

/// `ST_StyleSort` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_style_sort_method_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::StyleSortMethod;
    assert_eq!(
        StyleSortMethod::from_wire("0000"),
        Some(StyleSortMethod::LegacyName)
    );
    assert_eq!(StyleSortMethod::LegacyName.to_wire(), "0000");
    assert_eq!(
        StyleSortMethod::from_wire("0001"),
        Some(StyleSortMethod::LegacyPriority)
    );
    assert_eq!(StyleSortMethod::LegacyPriority.to_wire(), "0001");
    assert_eq!(
        StyleSortMethod::from_wire("0002"),
        Some(StyleSortMethod::LegacyDefault)
    );
    assert_eq!(StyleSortMethod::LegacyDefault.to_wire(), "0002");
    assert_eq!(
        StyleSortMethod::from_wire("0003"),
        Some(StyleSortMethod::LegacyFont)
    );
    assert_eq!(StyleSortMethod::LegacyFont.to_wire(), "0003");
    assert_eq!(
        StyleSortMethod::from_wire("0004"),
        Some(StyleSortMethod::LegacyBasedOn)
    );
    assert_eq!(StyleSortMethod::LegacyBasedOn.to_wire(), "0004");
    assert_eq!(
        StyleSortMethod::from_wire("0005"),
        Some(StyleSortMethod::LegacyType)
    );
    assert_eq!(StyleSortMethod::LegacyType.to_wire(), "0005");
}

/// `ST_FrameLayout` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_frame_layout_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::FrameLayout;
    assert_eq!(FrameLayout::from_wire("cols"), Some(FrameLayout::Columns));
    assert_eq!(FrameLayout::Columns.to_wire(), "cols");
}

/// `ST_TblStyleOverrideType` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_table_style_override_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::TableStyleOverrideType;
    assert_eq!(
        TableStyleOverrideType::from_wire("firstCol"),
        Some(TableStyleOverrideType::FirstColumn)
    );
    assert_eq!(TableStyleOverrideType::FirstColumn.to_wire(), "firstCol");
    assert_eq!(
        TableStyleOverrideType::from_wire("lastCol"),
        Some(TableStyleOverrideType::LastColumn)
    );
    assert_eq!(TableStyleOverrideType::LastColumn.to_wire(), "lastCol");
    assert_eq!(
        TableStyleOverrideType::from_wire("neCell"),
        Some(TableStyleOverrideType::TopRightCell)
    );
    assert_eq!(TableStyleOverrideType::TopRightCell.to_wire(), "neCell");
    assert_eq!(
        TableStyleOverrideType::from_wire("nwCell"),
        Some(TableStyleOverrideType::TopLeftCell)
    );
    assert_eq!(TableStyleOverrideType::TopLeftCell.to_wire(), "nwCell");
    assert_eq!(
        TableStyleOverrideType::from_wire("seCell"),
        Some(TableStyleOverrideType::BottomRightCell)
    );
    assert_eq!(TableStyleOverrideType::BottomRightCell.to_wire(), "seCell");
    assert_eq!(
        TableStyleOverrideType::from_wire("swCell"),
        Some(TableStyleOverrideType::BottomLeftCell)
    );
    assert_eq!(TableStyleOverrideType::BottomLeftCell.to_wire(), "swCell");
}

/// `ST_DocPartBehavior` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_document_part_behavior_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::DocumentPartBehavior;
    assert_eq!(
        DocumentPartBehavior::from_wire("p"),
        Some(DocumentPartBehavior::NewParagraph)
    );
    assert_eq!(DocumentPartBehavior::NewParagraph.to_wire(), "p");
    assert_eq!(
        DocumentPartBehavior::from_wire("pg"),
        Some(DocumentPartBehavior::NewPage)
    );
    assert_eq!(DocumentPartBehavior::NewPage.to_wire(), "pg");
}

/// `ST_DocPartType` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_document_part_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::DocumentPartType;
    assert_eq!(
        DocumentPartType::from_wire("autoExp"),
        Some(DocumentPartType::ReplaceNameWithContent)
    );
    assert_eq!(
        DocumentPartType::ReplaceNameWithContent.to_wire(),
        "autoExp"
    );
    assert_eq!(
        DocumentPartType::from_wire("formFld"),
        Some(DocumentPartType::FormFieldHelpText)
    );
    assert_eq!(DocumentPartType::FormFieldHelpText.to_wire(), "formFld");
    assert_eq!(
        DocumentPartType::from_wire("bbPlcHdr"),
        Some(DocumentPartType::StructuredDocumentTagPlaceholderText)
    );
    assert_eq!(
        DocumentPartType::StructuredDocumentTagPlaceholderText.to_wire(),
        "bbPlcHdr"
    );
    assert_eq!(
        DocumentPartType::from_wire("speller"),
        Some(DocumentPartType::AutoCorrectEntry)
    );
    assert_eq!(DocumentPartType::AutoCorrectEntry.to_wire(), "speller");
    assert_eq!(
        DocumentPartType::from_wire("toolbar"),
        Some(DocumentPartType::AutoTextUserInterfaceEntry)
    );
    assert_eq!(
        DocumentPartType::AutoTextUserInterfaceEntry.to_wire(),
        "toolbar"
    );
}

/// `ST_DocPartGallery` (wml.xsd): every value whose Rust name comes from the prose.
#[test]
fn wordprocessingml_document_part_gallery_overridden_tokens_round_trip() {
    use mjx_ooxml_types::wordprocessingml::DocumentPartGallery;
    assert_eq!(
        DocumentPartGallery::from_wire("docParts"),
        Some(DocumentPartGallery::DocumentParts)
    );
    assert_eq!(DocumentPartGallery::DocumentParts.to_wire(), "docParts");
    assert_eq!(
        DocumentPartGallery::from_wire("coverPg"),
        Some(DocumentPartGallery::CoverPage)
    );
    assert_eq!(DocumentPartGallery::CoverPage.to_wire(), "coverPg");
    assert_eq!(
        DocumentPartGallery::from_wire("eq"),
        Some(DocumentPartGallery::Equations)
    );
    assert_eq!(DocumentPartGallery::Equations.to_wire(), "eq");
    assert_eq!(
        DocumentPartGallery::from_wire("ftrs"),
        Some(DocumentPartGallery::Footers)
    );
    assert_eq!(DocumentPartGallery::Footers.to_wire(), "ftrs");
    assert_eq!(
        DocumentPartGallery::from_wire("hdrs"),
        Some(DocumentPartGallery::Headers)
    );
    assert_eq!(DocumentPartGallery::Headers.to_wire(), "hdrs");
    assert_eq!(
        DocumentPartGallery::from_wire("pgNum"),
        Some(DocumentPartGallery::PageNumbers)
    );
    assert_eq!(DocumentPartGallery::PageNumbers.to_wire(), "pgNum");
    assert_eq!(
        DocumentPartGallery::from_wire("tbls"),
        Some(DocumentPartGallery::Tables)
    );
    assert_eq!(DocumentPartGallery::Tables.to_wire(), "tbls");
    assert_eq!(
        DocumentPartGallery::from_wire("autoTxt"),
        Some(DocumentPartGallery::AutoText)
    );
    assert_eq!(DocumentPartGallery::AutoText.to_wire(), "autoTxt");
    assert_eq!(
        DocumentPartGallery::from_wire("txtBox"),
        Some(DocumentPartGallery::TextBox)
    );
    assert_eq!(DocumentPartGallery::TextBox.to_wire(), "txtBox");
    assert_eq!(
        DocumentPartGallery::from_wire("pgNumT"),
        Some(DocumentPartGallery::PageNumbersAtTop)
    );
    assert_eq!(DocumentPartGallery::PageNumbersAtTop.to_wire(), "pgNumT");
    assert_eq!(
        DocumentPartGallery::from_wire("pgNumB"),
        Some(DocumentPartGallery::PageNumbersAtBottom)
    );
    assert_eq!(DocumentPartGallery::PageNumbersAtBottom.to_wire(), "pgNumB");
    assert_eq!(
        DocumentPartGallery::from_wire("pgNumMargins"),
        Some(DocumentPartGallery::PageNumbersAtMargins)
    );
    assert_eq!(
        DocumentPartGallery::PageNumbersAtMargins.to_wire(),
        "pgNumMargins"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("tblOfContents"),
        Some(DocumentPartGallery::TableOfContents)
    );
    assert_eq!(
        DocumentPartGallery::TableOfContents.to_wire(),
        "tblOfContents"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("bib"),
        Some(DocumentPartGallery::Bibliography)
    );
    assert_eq!(DocumentPartGallery::Bibliography.to_wire(), "bib");
    assert_eq!(
        DocumentPartGallery::from_wire("custQuickParts"),
        Some(DocumentPartGallery::CustomQuickParts)
    );
    assert_eq!(
        DocumentPartGallery::CustomQuickParts.to_wire(),
        "custQuickParts"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custCoverPg"),
        Some(DocumentPartGallery::CustomCoverPage)
    );
    assert_eq!(
        DocumentPartGallery::CustomCoverPage.to_wire(),
        "custCoverPg"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custEq"),
        Some(DocumentPartGallery::CustomEquations)
    );
    assert_eq!(DocumentPartGallery::CustomEquations.to_wire(), "custEq");
    assert_eq!(
        DocumentPartGallery::from_wire("custFtrs"),
        Some(DocumentPartGallery::CustomFooters)
    );
    assert_eq!(DocumentPartGallery::CustomFooters.to_wire(), "custFtrs");
    assert_eq!(
        DocumentPartGallery::from_wire("custHdrs"),
        Some(DocumentPartGallery::CustomHeaders)
    );
    assert_eq!(DocumentPartGallery::CustomHeaders.to_wire(), "custHdrs");
    assert_eq!(
        DocumentPartGallery::from_wire("custPgNum"),
        Some(DocumentPartGallery::CustomPageNumbers)
    );
    assert_eq!(
        DocumentPartGallery::CustomPageNumbers.to_wire(),
        "custPgNum"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custTbls"),
        Some(DocumentPartGallery::CustomTables)
    );
    assert_eq!(DocumentPartGallery::CustomTables.to_wire(), "custTbls");
    assert_eq!(
        DocumentPartGallery::from_wire("custWatermarks"),
        Some(DocumentPartGallery::CustomWatermarks)
    );
    assert_eq!(
        DocumentPartGallery::CustomWatermarks.to_wire(),
        "custWatermarks"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custAutoTxt"),
        Some(DocumentPartGallery::CustomAutoText)
    );
    assert_eq!(DocumentPartGallery::CustomAutoText.to_wire(), "custAutoTxt");
    assert_eq!(
        DocumentPartGallery::from_wire("custTxtBox"),
        Some(DocumentPartGallery::CustomTextBox)
    );
    assert_eq!(DocumentPartGallery::CustomTextBox.to_wire(), "custTxtBox");
    assert_eq!(
        DocumentPartGallery::from_wire("custPgNumT"),
        Some(DocumentPartGallery::CustomPageNumbersAtTop)
    );
    assert_eq!(
        DocumentPartGallery::CustomPageNumbersAtTop.to_wire(),
        "custPgNumT"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custPgNumB"),
        Some(DocumentPartGallery::CustomPageNumbersAtBottom)
    );
    assert_eq!(
        DocumentPartGallery::CustomPageNumbersAtBottom.to_wire(),
        "custPgNumB"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custPgNumMargins"),
        Some(DocumentPartGallery::CustomPageNumbersAtMargins)
    );
    assert_eq!(
        DocumentPartGallery::CustomPageNumbersAtMargins.to_wire(),
        "custPgNumMargins"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custTblOfContents"),
        Some(DocumentPartGallery::CustomTableOfContents)
    );
    assert_eq!(
        DocumentPartGallery::CustomTableOfContents.to_wire(),
        "custTblOfContents"
    );
    assert_eq!(
        DocumentPartGallery::from_wire("custBib"),
        Some(DocumentPartGallery::CustomBibliography)
    );
    assert_eq!(DocumentPartGallery::CustomBibliography.to_wire(), "custBib");
}

/// `ST_Shp` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_delimiter_shape_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::DelimiterShape;
    assert_eq!(
        DelimiterShape::from_wire("match"),
        Some(DelimiterShape::MatchArgument)
    );
    assert_eq!(DelimiterShape::MatchArgument.to_wire(), "match");
}

/// `ST_FType` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_fraction_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::FractionType;
    assert_eq!(FractionType::from_wire("skw"), Some(FractionType::Skewed));
    assert_eq!(FractionType::Skewed.to_wire(), "skw");
    assert_eq!(FractionType::from_wire("lin"), Some(FractionType::Linear));
    assert_eq!(FractionType::Linear.to_wire(), "lin");
}

/// `ST_LimLoc` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_limit_location_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::LimitLocation;
    assert_eq!(
        LimitLocation::from_wire("undOvr"),
        Some(LimitLocation::UnderOver)
    );
    assert_eq!(LimitLocation::UnderOver.to_wire(), "undOvr");
    assert_eq!(
        LimitLocation::from_wire("subSup"),
        Some(LimitLocation::SubscriptSuperscript)
    );
    assert_eq!(LimitLocation::SubscriptSuperscript.to_wire(), "subSup");
}

/// `ST_TopBot` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_top_bottom_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::TopBottom;
    assert_eq!(TopBottom::from_wire("bot"), Some(TopBottom::Bottom));
    assert_eq!(TopBottom::Bottom.to_wire(), "bot");
}

/// `ST_Style` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_math_style_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::MathStyle;
    assert_eq!(MathStyle::from_wire("p"), Some(MathStyle::Plain));
    assert_eq!(MathStyle::Plain.to_wire(), "p");
    assert_eq!(MathStyle::from_wire("b"), Some(MathStyle::Bold));
    assert_eq!(MathStyle::Bold.to_wire(), "b");
    assert_eq!(MathStyle::from_wire("i"), Some(MathStyle::Italic));
    assert_eq!(MathStyle::Italic.to_wire(), "i");
    assert_eq!(MathStyle::from_wire("bi"), Some(MathStyle::BoldItalic));
    assert_eq!(MathStyle::BoldItalic.to_wire(), "bi");
}

/// `ST_Jc` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_justification_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::Justification;
    assert_eq!(
        Justification::from_wire("centerGroup"),
        Some(Justification::CenteredAsGroup)
    );
    assert_eq!(Justification::CenteredAsGroup.to_wire(), "centerGroup");
}

/// `ST_BreakBinSub` (shared-math.xsd): every value whose Rust name comes from the prose.
#[test]
fn officemath_break_binary_subtraction_overridden_tokens_round_trip() {
    use mjx_ooxml_types::officemath::BreakBinarySubtraction;
    assert_eq!(
        BreakBinarySubtraction::from_wire("--"),
        Some(BreakBinarySubtraction::MinusMinus)
    );
    assert_eq!(BreakBinarySubtraction::MinusMinus.to_wire(), "--");
    assert_eq!(
        BreakBinarySubtraction::from_wire("-+"),
        Some(BreakBinarySubtraction::MinusPlus)
    );
    assert_eq!(BreakBinarySubtraction::MinusPlus.to_wire(), "-+");
    assert_eq!(
        BreakBinarySubtraction::from_wire("+-"),
        Some(BreakBinarySubtraction::PlusMinus)
    );
    assert_eq!(BreakBinarySubtraction::PlusMinus.to_wire(), "+-");
}

/// Round-trips **every** enumeration token of a schema through the generated types, and asserts
/// that each one reaches a variant of its own.
///
/// `from_wire` and `to_wire` are emitted from one list, so this cannot fail by a token being
/// missing from one of them. The failure it is built for is two values of one enumeration reaching
/// the *same* Rust variant — the enum then has fewer variants than the schema has values, one of
/// the two tokens can no longer spell itself, and a document that used it is rewritten with the
/// other. The distinct-variant count is asserted directly rather than inferred, so the check holds
/// even for a pair whose round trip happens to survive.
macro_rules! assert_every_token_round_trips_to_its_own_variant {
    ($($ty:path => [$($tok:literal),* $(,)?]),* $(,)?) => {
        $({
            let tokens: &[&str] = &[$($tok),*];
            let mut reached = std::collections::HashSet::new();
            for tok in tokens {
                let parsed = <$ty>::from_wire(tok)
                    .unwrap_or_else(|| panic!("{} does not accept {:?}", stringify!($ty), tok));
                assert_eq!(parsed.to_wire(), *tok, "{} lost {:?}", stringify!($ty), tok);
                assert!(
                    reached.insert(parsed),
                    "{}: {:?} reaches a variant another wire token already reached",
                    stringify!($ty),
                    tok,
                );
            }
            assert_eq!(
                reached.len(),
                tokens.len(),
                "{} has fewer variants than its schema declares values",
                stringify!($ty),
            );
        })*
    };
}

/// Every token of every enumeration in `wml.xsd` — all 733 enumeration values of `wml.xsd`.
#[test]
fn every_wordprocessingml_token_round_trips() {
    use mjx_ooxml_types::wordprocessingml::*;
    assert_every_token_round_trips_to_its_own_variant! {
        HighlightColor => ["black", "blue", "cyan", "green", "magenta", "red", "yellow", "white", "darkBlue", "darkCyan", "darkGreen", "darkMagenta", "darkRed", "darkYellow", "darkGray", "lightGray", "none"],
        AutomaticColor => ["auto"],
        Underline => ["single", "words", "double", "thick", "dotted", "dottedHeavy", "dash", "dashedHeavy", "dashLong", "dashLongHeavy", "dotDash", "dashDotHeavy", "dotDotDash", "dashDotDotHeavy", "wave", "wavyHeavy", "wavyDouble", "none"],
        TextEffect => ["blinkBackground", "lights", "antsBlack", "antsRed", "shimmer", "sparkle", "none"],
        BorderStyle => ["nil", "none", "single", "thick", "double", "dotted", "dashed", "dotDash", "dotDotDash", "triple", "thinThickSmallGap", "thickThinSmallGap", "thinThickThinSmallGap", "thinThickMediumGap", "thickThinMediumGap", "thinThickThinMediumGap", "thinThickLargeGap", "thickThinLargeGap", "thinThickThinLargeGap", "wave", "doubleWave", "dashSmallGap", "dashDotStroked", "threeDEmboss", "threeDEngrave", "outset", "inset", "apples", "archedScallops", "babyPacifier", "babyRattle", "balloons3Colors", "balloonsHotAir", "basicBlackDashes", "basicBlackDots", "basicBlackSquares", "basicThinLines", "basicWhiteDashes", "basicWhiteDots", "basicWhiteSquares", "basicWideInline", "basicWideMidline", "basicWideOutline", "bats", "birds", "birdsFlight", "cabins", "cakeSlice", "candyCorn", "celticKnotwork", "certificateBanner", "chainLink", "champagneBottle", "checkedBarBlack", "checkedBarColor", "checkered", "christmasTree", "circlesLines", "circlesRectangles", "classicalWave", "clocks", "compass", "confetti", "confettiGrays", "confettiOutline", "confettiStreamers", "confettiWhite", "cornerTriangles", "couponCutoutDashes", "couponCutoutDots", "crazyMaze", "creaturesButterfly", "creaturesFish", "creaturesInsects", "creaturesLadyBug", "crossStitch", "cup", "decoArch", "decoArchColor", "decoBlocks", "diamondsGray", "doubleD", "doubleDiamonds", "earth1", "earth2", "earth3", "eclipsingSquares1", "eclipsingSquares2", "eggsBlack", "fans", "film", "firecrackers", "flowersBlockPrint", "flowersDaisies", "flowersModern1", "flowersModern2", "flowersPansy", "flowersRedRose", "flowersRoses", "flowersTeacup", "flowersTiny", "gems", "gingerbreadMan", "gradient", "handmade1", "handmade2", "heartBalloon", "heartGray", "hearts", "heebieJeebies", "holly", "houseFunky", "hypnotic", "iceCreamCones", "lightBulb", "lightning1", "lightning2", "mapPins", "mapleLeaf", "mapleMuffins", "marquee", "marqueeToothed", "moons", "mosaic", "musicNotes", "northwest", "ovals", "packages", "palmsBlack", "palmsColor", "paperClips", "papyrus", "partyFavor", "partyGlass", "pencils", "people", "peopleWaving", "peopleHats", "poinsettias", "postageStamp", "pumpkin1", "pushPinNote2", "pushPinNote1", "pyramids", "pyramidsAbove", "quadrants", "rings", "safari", "sawtooth", "sawtoothGray", "scaredCat", "seattle", "shadowedSquares", "sharksTeeth", "shorebirdTracks", "skyrocket", "snowflakeFancy", "snowflakes", "sombrero", "southwest", "stars", "starsTop", "stars3d", "starsBlack", "starsShadowed", "sun", "swirligig", "tornPaper", "tornPaperBlack", "trees", "triangleParty", "triangles", "triangle1", "triangle2", "triangleCircle1", "triangleCircle2", "shapes1", "shapes2", "twistedLines1", "twistedLines2", "vine", "waveline", "weavingAngles", "weavingBraid", "weavingRibbon", "weavingStrips", "whiteFlowers", "woodwork", "xIllusions", "zanyTriangles", "zigZag", "zigZagStitch", "custom"],
        ShadingPattern => ["nil", "clear", "solid", "horzStripe", "vertStripe", "reverseDiagStripe", "diagStripe", "horzCross", "diagCross", "thinHorzStripe", "thinVertStripe", "thinReverseDiagStripe", "thinDiagStripe", "thinHorzCross", "thinDiagCross", "pct5", "pct10", "pct12", "pct15", "pct20", "pct25", "pct30", "pct35", "pct37", "pct40", "pct45", "pct50", "pct55", "pct60", "pct62", "pct65", "pct70", "pct75", "pct80", "pct85", "pct87", "pct90", "pct95"],
        EmphasisMark => ["none", "dot", "comma", "circle", "underDot"],
        CombineBrackets => ["none", "round", "square", "angle", "curly"],
        HeightRule => ["auto", "exact", "atLeast"],
        TextFrameWrapping => ["auto", "notBeside", "around", "tight", "through", "none"],
        VerticalAnchor => ["text", "margin", "page"],
        HorizontalAnchor => ["text", "margin", "page"],
        DropCap => ["none", "drop", "margin"],
        TabStopType => ["clear", "start", "center", "end", "decimal", "bar", "num", "left", "right"],
        TabStopLeader => ["none", "dot", "hyphen", "underscore", "heavy", "middleDot"],
        LineSpacingRule => ["auto", "exact", "atLeast"],
        Justification => ["start", "center", "end", "both", "mediumKashida", "distribute", "numTab", "highKashida", "lowKashida", "thaiDistribute", "left", "right"],
        TableJustification => ["center", "end", "left", "right", "start"],
        DocumentView => ["none", "print", "outline", "masterPages", "normal", "web"],
        ZoomPreset => ["none", "fullPage", "bestFit", "textFit"],
        ProofingState => ["clean", "dirty"],
        DocumentProtection => ["none", "readOnly", "comments", "trackedChanges", "forms"],
        MailMergeDocumentType => ["catalog", "envelopes", "mailingLabels", "formLetters", "email", "fax"],
        MailMergeDestination => ["newDocument", "printer", "email", "fax"],
        MailMergeFieldMappingType => ["null", "dbColumn"],
        TextFlowDirection => ["tb", "rl", "lr", "tbV", "rlV", "lrV", "btLr", "lrTb", "lrTbV", "tbLrV", "tbRl", "tbRlV"],
        VerticalTextAlignment => ["top", "center", "baseline", "bottom", "auto"],
        DisplacedByCustomXml => ["next", "prev"],
        VerticalMergeRevision => ["cont", "rest"],
        TextBoxTightWrap => ["none", "allLines", "firstAndLastLine", "firstLineOnly", "lastLineOnly"],
        ObjectDrawAspect => ["content", "icon"],
        ObjectUpdateMode => ["always", "onCall"],
        FieldCharacterType => ["begin", "separate", "end"],
        HelpOrStatusTextType => ["text", "autoText"],
        FormFieldTextType => ["regular", "number", "date", "currentTime", "currentDate", "calculated"],
        SectionBreakType => ["nextPage", "nextColumn", "continuous", "evenPage", "oddPage"],
        NumberFormat => ["decimal", "upperRoman", "lowerRoman", "upperLetter", "lowerLetter", "ordinal", "cardinalText", "ordinalText", "hex", "chicago", "ideographDigital", "japaneseCounting", "aiueo", "iroha", "decimalFullWidth", "decimalHalfWidth", "japaneseLegal", "japaneseDigitalTenThousand", "decimalEnclosedCircle", "decimalFullWidth2", "aiueoFullWidth", "irohaFullWidth", "decimalZero", "bullet", "ganada", "chosung", "decimalEnclosedFullstop", "decimalEnclosedParen", "decimalEnclosedCircleChinese", "ideographEnclosedCircle", "ideographTraditional", "ideographZodiac", "ideographZodiacTraditional", "taiwaneseCounting", "ideographLegalTraditional", "taiwaneseCountingThousand", "taiwaneseDigital", "chineseCounting", "chineseLegalSimplified", "chineseCountingThousand", "koreanDigital", "koreanCounting", "koreanLegal", "koreanDigital2", "vietnameseCounting", "russianLower", "russianUpper", "none", "numberInDash", "hebrew1", "hebrew2", "arabicAlpha", "arabicAbjad", "hindiVowels", "hindiConsonants", "hindiNumbers", "hindiCounting", "thaiLetters", "thaiNumbers", "thaiCounting", "bahtText", "dollarText", "custom"],
        PageOrientation => ["portrait", "landscape"],
        PageBorderZOrder => ["front", "back"],
        PageBorderDisplay => ["allPages", "firstPage", "notFirstPage"],
        PageBorderOffset => ["page", "text"],
        ChapterSeparator => ["hyphen", "period", "colon", "emDash", "enDash"],
        LineNumberRestart => ["newPage", "newSection", "continuous"],
        VerticalJustification => ["top", "center", "both", "bottom"],
        DocumentGridType => ["default", "lines", "linesAndChars", "snapToChars"],
        HeaderFooterType => ["even", "default", "first"],
        FootnoteEndnoteType => ["normal", "separator", "continuationSeparator", "continuationNotice"],
        BreakType => ["page", "column", "textWrapping"],
        BreakTextWrappingRestart => ["none", "left", "right", "all"],
        PositionalTabAlignment => ["left", "center", "right"],
        PositionalTabBase => ["margin", "indent"],
        PositionalTabLeader => ["none", "dot", "hyphen", "underscore", "middleDot"],
        ProofingErrorType => ["spellStart", "spellEnd", "gramStart", "gramEnd"],
        EditingGroup => ["none", "everyone", "administrators", "contributors", "editors", "owners", "current"],
        FontTypeHint => ["default", "eastAsia"],
        ThemeFont => ["majorEastAsia", "majorBidi", "majorAscii", "majorHAnsi", "minorEastAsia", "minorBidi", "minorAscii", "minorHAnsi"],
        PhoneticGuideAlignment => ["center", "distributeLetter", "distributeSpace", "left", "right", "rightVertical"],
        LockingType => ["sdtLocked", "contentLocked", "unlocked", "sdtContentLocked"],
        DateStorageFormat => ["text", "date", "dateTime"],
        BidirectionalDirection => ["ltr", "rtl"],
        TableWidthUnit => ["nil", "pct", "dxa", "auto"],
        MergedCellType => ["continue", "restart"],
        TableLayoutType => ["fixed", "autofit"],
        TableOverlap => ["never", "overlap"],
        FootnotePosition => ["pageBottom", "beneathText", "sectEnd", "docEnd"],
        EndnotePosition => ["sectEnd", "docEnd"],
        NumberingRestartLocation => ["continuous", "eachSect", "eachPage"],
        MailMergeSourceType => ["database", "addressBook", "document1", "document2", "text", "email", "native", "legacy", "master"],
        TargetScreenSize => ["544x376", "640x480", "720x512", "800x600", "1024x768", "1152x882", "1152x900", "1280x1024", "1600x1200", "1800x1440", "1920x1200"],
        CharacterSpacingCompression => ["doNotCompress", "compressPunctuation", "compressPunctuationAndJapaneseKana"],
        ColorSchemeSlot => ["dark1", "light1", "dark2", "light2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hyperlink", "followedHyperlink"],
        StyleSortMethod => ["name", "priority", "default", "font", "basedOn", "type", "0000", "0001", "0002", "0003", "0004", "0005"],
        FrameScrollbarVisibility => ["on", "off", "auto"],
        FrameLayout => ["rows", "cols", "none"],
        NumberingLevelSuffix => ["tab", "space", "nothing"],
        MultiLevelType => ["singleLevel", "multilevel", "hybridMultilevel"],
        TableStyleOverrideType => ["wholeTable", "firstRow", "lastRow", "firstCol", "lastCol", "band1Vert", "band2Vert", "band1Horz", "band2Horz", "neCell", "nwCell", "seCell", "swCell"],
        StyleType => ["paragraph", "character", "table", "numbering"],
        FontFamily => ["decorative", "modern", "roman", "script", "swiss", "auto"],
        FontPitch => ["fixed", "variable", "default"],
        ThemeColor => ["dark1", "light1", "dark2", "light2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hyperlink", "followedHyperlink", "none", "background1", "text1", "background2", "text2"],
        DocumentPartBehavior => ["content", "p", "pg"],
        DocumentPartType => ["none", "normal", "autoExp", "toolbar", "speller", "formFld", "bbPlcHdr"],
        DocumentPartGallery => ["placeholder", "any", "default", "docParts", "coverPg", "eq", "ftrs", "hdrs", "pgNum", "tbls", "watermarks", "autoTxt", "txtBox", "pgNumT", "pgNumB", "pgNumMargins", "tblOfContents", "bib", "custQuickParts", "custCoverPg", "custEq", "custFtrs", "custHdrs", "custPgNum", "custTbls", "custWatermarks", "custAutoTxt", "custTxtBox", "custPgNumT", "custPgNumB", "custPgNumMargins", "custTblOfContents", "custBib", "custom1", "custom2", "custom3", "custom4", "custom5"],
        CaptionPosition => ["above", "below", "left", "right"],
    };
}

/// Every token of every enumeration in `shared-math.xsd` — all 30 enumeration values of `shared-math.xsd`.
#[test]
fn every_officemath_token_round_trips() {
    use mjx_ooxml_types::officemath::*;
    assert_every_token_round_trips_to_its_own_variant! {
        DelimiterShape => ["centered", "match"],
        FractionType => ["bar", "skw", "lin", "noBar"],
        LimitLocation => ["undOvr", "subSup"],
        TopBottom => ["top", "bot"],
        ScriptType => ["roman", "script", "fraktur", "double-struck", "sans-serif", "monospace"],
        MathStyle => ["p", "b", "i", "bi"],
        Justification => ["left", "right", "center", "centerGroup"],
        BreakBinaryOperator => ["before", "after", "repeat"],
        BreakBinarySubtraction => ["--", "-+", "+-"],
    };
}

/// `ST_Jc` is declared by **both** `wml.xsd` and `shared-math.xsd`, with different value sets, and
/// `ST_Style` by both `shared-math.xsd` and `dml-chart.xsd`. Module-namespacing per schema is what
/// keeps them apart: there is no bare `Jc`, and neither `Justification` can accept the other's
/// tokens. A single flat namespace would have had to drop or rename one of them.
#[test]
fn the_two_justifications_are_different_types_in_different_modules() {
    use mjx_ooxml_types::officemath::Justification as MathJustification;
    use mjx_ooxml_types::wordprocessingml::Justification as WordJustification;

    // Twelve values against four, and only three tokens in common.
    assert_eq!(
        WordJustification::from_wire("both"),
        Some(WordJustification::Justified)
    );
    assert_eq!(MathJustification::from_wire("both"), None);
    assert_eq!(
        MathJustification::from_wire("centerGroup"),
        Some(MathJustification::CenteredAsGroup)
    );
    assert_eq!(WordJustification::from_wire("centerGroup"), None);

    // The token they share means the same thing and spells itself the same way in both.
    assert_eq!(WordJustification::Center.to_wire(), "center");
    assert_eq!(MathJustification::Center.to_wire(), "center");
}

/// The Transitional-only spellings keep their own variants rather than folding into the value they
/// are equivalent to, because a document that used `left` must be written back as `left`.
#[test]
fn transitional_aliases_keep_their_own_wire_tokens() {
    use mjx_ooxml_types::wordprocessingml::{Justification, StyleSortMethod, TextFlowDirection};

    // Part 4 §14.11.2: `left` is semantically `start`, but it is a different token.
    assert_ne!(Justification::Left, Justification::Start);
    assert_eq!(Justification::Left.to_wire(), "left");
    assert_eq!(Justification::Start.to_wire(), "start");

    // Part 4 §14.11.7: `lrTb` is semantically `tb`.
    assert_ne!(
        TextFlowDirection::LeftToRightTopToBottom,
        TextFlowDirection::TopToBottom
    );
    assert_eq!(TextFlowDirection::LeftToRightTopToBottom.to_wire(), "lrTb");

    // Part 4 §14.11.5: `0000` is semantically `name`.
    assert_ne!(StyleSortMethod::LegacyName, StyleSortMethod::Name);
    assert_eq!(StyleSortMethod::LegacyName.to_wire(), "0000");
}

/// `wml.xsd` restricts three of its measures straight from `shared-commonSimpleTypes.xsd`'s
/// `ST_UnsignedDecimalNumber`. Resolving that alias is what keeps a pixel count a number.
#[test]
fn wordprocessingml_measures_are_numbers_not_strings() {
    use mjx_ooxml_types::wordprocessingml::{EighthPointMeasure, PixelsMeasure, PointMeasure};

    let pixels: PixelsMeasure = 96;
    let eighths: EighthPointMeasure = 4;
    let points: PointMeasure = 12;
    assert_eq!(pixels + u64::from(eighths as u32) + points, 112);
}
// ---------------------------------------------------------------------------------------
// SpreadsheetML (`sml.xsd`) — MJXOFF-145.
//
// Seeded from `SPREADSHEETML_VARIANT_OVERRIDES` in `xtask/src/codegen/spec.rs`, not from the
// tokens the generator already gets right: a variant that is a mechanical PascalCase of its
// token round-trips whatever the naming table says, so it discriminates nothing. Each test
// below pins every *overridden* token of one `ST_*` to its named variant, in both directions,
// against exact wire bytes. All 149 rows of the table are covered.
// ---------------------------------------------------------------------------------------

/// `ST_CellType` (sml.xsd) → `spreadsheetml::CellType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_cell_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::CellType;
    assert_eq!(CellType::from_wire("b"), Some(CellType::Boolean));
    assert_eq!(CellType::Boolean.to_wire(), "b");
    assert_eq!(CellType::from_wire("n"), Some(CellType::Number));
    assert_eq!(CellType::Number.to_wire(), "n");
    assert_eq!(CellType::from_wire("e"), Some(CellType::Error));
    assert_eq!(CellType::Error.to_wire(), "e");
    assert_eq!(CellType::from_wire("s"), Some(CellType::SharedString));
    assert_eq!(CellType::SharedString.to_wire(), "s");
    assert_eq!(CellType::from_wire("str"), Some(CellType::FormulaString));
    assert_eq!(CellType::FormulaString.to_wire(), "str");
    assert_eq!(
        CellType::from_wire("inlineStr"),
        Some(CellType::InlineString)
    );
    assert_eq!(CellType::InlineString.to_wire(), "inlineStr");
}

/// `ST_DdeValueType` (sml.xsd) → `spreadsheetml::DynamicDataExchangeValueType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_dynamic_data_exchange_value_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::DynamicDataExchangeValueType;
    assert_eq!(
        DynamicDataExchangeValueType::from_wire("b"),
        Some(DynamicDataExchangeValueType::Boolean)
    );
    assert_eq!(DynamicDataExchangeValueType::Boolean.to_wire(), "b");
    assert_eq!(
        DynamicDataExchangeValueType::from_wire("n"),
        Some(DynamicDataExchangeValueType::RealNumber)
    );
    assert_eq!(DynamicDataExchangeValueType::RealNumber.to_wire(), "n");
    assert_eq!(
        DynamicDataExchangeValueType::from_wire("e"),
        Some(DynamicDataExchangeValueType::Error)
    );
    assert_eq!(DynamicDataExchangeValueType::Error.to_wire(), "e");
    assert_eq!(
        DynamicDataExchangeValueType::from_wire("str"),
        Some(DynamicDataExchangeValueType::String)
    );
    assert_eq!(DynamicDataExchangeValueType::String.to_wire(), "str");
}

/// `ST_VolValueType` (sml.xsd) → `spreadsheetml::VolatileDependencyValueType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_volatile_dependency_value_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::VolatileDependencyValueType;
    assert_eq!(
        VolatileDependencyValueType::from_wire("b"),
        Some(VolatileDependencyValueType::Boolean)
    );
    assert_eq!(VolatileDependencyValueType::Boolean.to_wire(), "b");
    assert_eq!(
        VolatileDependencyValueType::from_wire("n"),
        Some(VolatileDependencyValueType::RealNumber)
    );
    assert_eq!(VolatileDependencyValueType::RealNumber.to_wire(), "n");
    assert_eq!(
        VolatileDependencyValueType::from_wire("e"),
        Some(VolatileDependencyValueType::Error)
    );
    assert_eq!(VolatileDependencyValueType::Error.to_wire(), "e");
    assert_eq!(
        VolatileDependencyValueType::from_wire("s"),
        Some(VolatileDependencyValueType::String)
    );
    assert_eq!(VolatileDependencyValueType::String.to_wire(), "s");
}

/// `ST_MdxFunctionType` (sml.xsd) → `spreadsheetml::MdxFunctionType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_mdx_function_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::MdxFunctionType;
    assert_eq!(
        MdxFunctionType::from_wire("m"),
        Some(MdxFunctionType::CubeMember)
    );
    assert_eq!(MdxFunctionType::CubeMember.to_wire(), "m");
    assert_eq!(
        MdxFunctionType::from_wire("v"),
        Some(MdxFunctionType::CubeValue)
    );
    assert_eq!(MdxFunctionType::CubeValue.to_wire(), "v");
    assert_eq!(
        MdxFunctionType::from_wire("s"),
        Some(MdxFunctionType::CubeSet)
    );
    assert_eq!(MdxFunctionType::CubeSet.to_wire(), "s");
    assert_eq!(
        MdxFunctionType::from_wire("c"),
        Some(MdxFunctionType::CubeSetCount)
    );
    assert_eq!(MdxFunctionType::CubeSetCount.to_wire(), "c");
    assert_eq!(
        MdxFunctionType::from_wire("r"),
        Some(MdxFunctionType::CubeRankedMember)
    );
    assert_eq!(MdxFunctionType::CubeRankedMember.to_wire(), "r");
    assert_eq!(
        MdxFunctionType::from_wire("p"),
        Some(MdxFunctionType::CubeMemberProperty)
    );
    assert_eq!(MdxFunctionType::CubeMemberProperty.to_wire(), "p");
    assert_eq!(
        MdxFunctionType::from_wire("k"),
        Some(MdxFunctionType::CubeKeyPerformanceIndicatorMember)
    );
    assert_eq!(
        MdxFunctionType::CubeKeyPerformanceIndicatorMember.to_wire(),
        "k"
    );
}

/// `ST_MdxSetOrder` (sml.xsd) → `spreadsheetml::MdxSetOrder`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_mdx_set_order_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::MdxSetOrder;
    assert_eq!(MdxSetOrder::from_wire("u"), Some(MdxSetOrder::Unsorted));
    assert_eq!(MdxSetOrder::Unsorted.to_wire(), "u");
    assert_eq!(MdxSetOrder::from_wire("a"), Some(MdxSetOrder::Ascending));
    assert_eq!(MdxSetOrder::Ascending.to_wire(), "a");
    assert_eq!(MdxSetOrder::from_wire("d"), Some(MdxSetOrder::Descending));
    assert_eq!(MdxSetOrder::Descending.to_wire(), "d");
    assert_eq!(
        MdxSetOrder::from_wire("aa"),
        Some(MdxSetOrder::AlphabeticAscending)
    );
    assert_eq!(MdxSetOrder::AlphabeticAscending.to_wire(), "aa");
    assert_eq!(
        MdxSetOrder::from_wire("ad"),
        Some(MdxSetOrder::AlphabeticDescending)
    );
    assert_eq!(MdxSetOrder::AlphabeticDescending.to_wire(), "ad");
    assert_eq!(
        MdxSetOrder::from_wire("na"),
        Some(MdxSetOrder::NaturalAscending)
    );
    assert_eq!(MdxSetOrder::NaturalAscending.to_wire(), "na");
    assert_eq!(
        MdxSetOrder::from_wire("nd"),
        Some(MdxSetOrder::NaturalDescending)
    );
    assert_eq!(MdxSetOrder::NaturalDescending.to_wire(), "nd");
}

/// `ST_MdxKPIProperty` (sml.xsd) → `spreadsheetml::MdxKeyPerformanceIndicatorProperty`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_mdx_key_performance_indicator_property_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::MdxKeyPerformanceIndicatorProperty;
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::from_wire("v"),
        Some(MdxKeyPerformanceIndicatorProperty::Value)
    );
    assert_eq!(MdxKeyPerformanceIndicatorProperty::Value.to_wire(), "v");
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::from_wire("g"),
        Some(MdxKeyPerformanceIndicatorProperty::Goal)
    );
    assert_eq!(MdxKeyPerformanceIndicatorProperty::Goal.to_wire(), "g");
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::from_wire("s"),
        Some(MdxKeyPerformanceIndicatorProperty::Status)
    );
    assert_eq!(MdxKeyPerformanceIndicatorProperty::Status.to_wire(), "s");
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::from_wire("t"),
        Some(MdxKeyPerformanceIndicatorProperty::Trend)
    );
    assert_eq!(MdxKeyPerformanceIndicatorProperty::Trend.to_wire(), "t");
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::from_wire("w"),
        Some(MdxKeyPerformanceIndicatorProperty::Weight)
    );
    assert_eq!(MdxKeyPerformanceIndicatorProperty::Weight.to_wire(), "w");
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::from_wire("m"),
        Some(MdxKeyPerformanceIndicatorProperty::CurrentTimeMember)
    );
    assert_eq!(
        MdxKeyPerformanceIndicatorProperty::CurrentTimeMember.to_wire(),
        "m"
    );
}

/// `ST_IconSetType` (sml.xsd) → `spreadsheetml::IconSetType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_icon_set_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::IconSetType;
    assert_eq!(
        IconSetType::from_wire("3Arrows"),
        Some(IconSetType::ThreeArrows)
    );
    assert_eq!(IconSetType::ThreeArrows.to_wire(), "3Arrows");
    assert_eq!(
        IconSetType::from_wire("3ArrowsGray"),
        Some(IconSetType::ThreeArrowsGray)
    );
    assert_eq!(IconSetType::ThreeArrowsGray.to_wire(), "3ArrowsGray");
    assert_eq!(
        IconSetType::from_wire("3Flags"),
        Some(IconSetType::ThreeFlags)
    );
    assert_eq!(IconSetType::ThreeFlags.to_wire(), "3Flags");
    assert_eq!(
        IconSetType::from_wire("3TrafficLights1"),
        Some(IconSetType::ThreeTrafficLights)
    );
    assert_eq!(IconSetType::ThreeTrafficLights.to_wire(), "3TrafficLights1");
    assert_eq!(
        IconSetType::from_wire("3TrafficLights2"),
        Some(IconSetType::ThreeTrafficLightsBlack)
    );
    assert_eq!(
        IconSetType::ThreeTrafficLightsBlack.to_wire(),
        "3TrafficLights2"
    );
    assert_eq!(
        IconSetType::from_wire("3Signs"),
        Some(IconSetType::ThreeSigns)
    );
    assert_eq!(IconSetType::ThreeSigns.to_wire(), "3Signs");
    assert_eq!(
        IconSetType::from_wire("3Symbols"),
        Some(IconSetType::ThreeSymbolsCircled)
    );
    assert_eq!(IconSetType::ThreeSymbolsCircled.to_wire(), "3Symbols");
    assert_eq!(
        IconSetType::from_wire("3Symbols2"),
        Some(IconSetType::ThreeSymbols)
    );
    assert_eq!(IconSetType::ThreeSymbols.to_wire(), "3Symbols2");
    assert_eq!(
        IconSetType::from_wire("4Arrows"),
        Some(IconSetType::FourArrows)
    );
    assert_eq!(IconSetType::FourArrows.to_wire(), "4Arrows");
    assert_eq!(
        IconSetType::from_wire("4ArrowsGray"),
        Some(IconSetType::FourArrowsGray)
    );
    assert_eq!(IconSetType::FourArrowsGray.to_wire(), "4ArrowsGray");
    assert_eq!(
        IconSetType::from_wire("4RedToBlack"),
        Some(IconSetType::FourRedToBlack)
    );
    assert_eq!(IconSetType::FourRedToBlack.to_wire(), "4RedToBlack");
    assert_eq!(
        IconSetType::from_wire("4Rating"),
        Some(IconSetType::FourRatings)
    );
    assert_eq!(IconSetType::FourRatings.to_wire(), "4Rating");
    assert_eq!(
        IconSetType::from_wire("4TrafficLights"),
        Some(IconSetType::FourTrafficLights)
    );
    assert_eq!(IconSetType::FourTrafficLights.to_wire(), "4TrafficLights");
    assert_eq!(
        IconSetType::from_wire("5Arrows"),
        Some(IconSetType::FiveArrows)
    );
    assert_eq!(IconSetType::FiveArrows.to_wire(), "5Arrows");
    assert_eq!(
        IconSetType::from_wire("5ArrowsGray"),
        Some(IconSetType::FiveArrowsGray)
    );
    assert_eq!(IconSetType::FiveArrowsGray.to_wire(), "5ArrowsGray");
    assert_eq!(
        IconSetType::from_wire("5Rating"),
        Some(IconSetType::FiveRatings)
    );
    assert_eq!(IconSetType::FiveRatings.to_wire(), "5Rating");
    assert_eq!(
        IconSetType::from_wire("5Quarters"),
        Some(IconSetType::FiveQuarters)
    );
    assert_eq!(IconSetType::FiveQuarters.to_wire(), "5Quarters");
}

/// `ST_TargetScreenSize` (sml.xsd) → `spreadsheetml::TargetScreenSize`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_target_screen_size_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::TargetScreenSize;
    assert_eq!(
        TargetScreenSize::from_wire("544x376"),
        Some(TargetScreenSize::Resolution544By376)
    );
    assert_eq!(TargetScreenSize::Resolution544By376.to_wire(), "544x376");
    assert_eq!(
        TargetScreenSize::from_wire("640x480"),
        Some(TargetScreenSize::Resolution640By480)
    );
    assert_eq!(TargetScreenSize::Resolution640By480.to_wire(), "640x480");
    assert_eq!(
        TargetScreenSize::from_wire("720x512"),
        Some(TargetScreenSize::Resolution720By512)
    );
    assert_eq!(TargetScreenSize::Resolution720By512.to_wire(), "720x512");
    assert_eq!(
        TargetScreenSize::from_wire("800x600"),
        Some(TargetScreenSize::Resolution800By600)
    );
    assert_eq!(TargetScreenSize::Resolution800By600.to_wire(), "800x600");
    assert_eq!(
        TargetScreenSize::from_wire("1024x768"),
        Some(TargetScreenSize::Resolution1024By768)
    );
    assert_eq!(TargetScreenSize::Resolution1024By768.to_wire(), "1024x768");
    assert_eq!(
        TargetScreenSize::from_wire("1152x882"),
        Some(TargetScreenSize::Resolution1152By882)
    );
    assert_eq!(TargetScreenSize::Resolution1152By882.to_wire(), "1152x882");
    assert_eq!(
        TargetScreenSize::from_wire("1152x900"),
        Some(TargetScreenSize::Resolution1152By900)
    );
    assert_eq!(TargetScreenSize::Resolution1152By900.to_wire(), "1152x900");
    assert_eq!(
        TargetScreenSize::from_wire("1280x1024"),
        Some(TargetScreenSize::Resolution1280By1024)
    );
    assert_eq!(
        TargetScreenSize::Resolution1280By1024.to_wire(),
        "1280x1024"
    );
    assert_eq!(
        TargetScreenSize::from_wire("1600x1200"),
        Some(TargetScreenSize::Resolution1600By1200)
    );
    assert_eq!(
        TargetScreenSize::Resolution1600By1200.to_wire(),
        "1600x1200"
    );
    assert_eq!(
        TargetScreenSize::from_wire("1800x1440"),
        Some(TargetScreenSize::Resolution1800By1440)
    );
    assert_eq!(
        TargetScreenSize::Resolution1800By1440.to_wire(),
        "1800x1440"
    );
    assert_eq!(
        TargetScreenSize::from_wire("1920x1200"),
        Some(TargetScreenSize::Resolution1920By1200)
    );
    assert_eq!(
        TargetScreenSize::Resolution1920By1200.to_wire(),
        "1920x1200"
    );
}

/// `ST_DynamicFilterType` (sml.xsd) → `spreadsheetml::DynamicFilterType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_dynamic_filter_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::DynamicFilterType;
    assert_eq!(
        DynamicFilterType::from_wire("Q1"),
        Some(DynamicFilterType::FirstQuarter)
    );
    assert_eq!(DynamicFilterType::FirstQuarter.to_wire(), "Q1");
    assert_eq!(
        DynamicFilterType::from_wire("Q2"),
        Some(DynamicFilterType::SecondQuarter)
    );
    assert_eq!(DynamicFilterType::SecondQuarter.to_wire(), "Q2");
    assert_eq!(
        DynamicFilterType::from_wire("Q3"),
        Some(DynamicFilterType::ThirdQuarter)
    );
    assert_eq!(DynamicFilterType::ThirdQuarter.to_wire(), "Q3");
    assert_eq!(
        DynamicFilterType::from_wire("Q4"),
        Some(DynamicFilterType::FourthQuarter)
    );
    assert_eq!(DynamicFilterType::FourthQuarter.to_wire(), "Q4");
    assert_eq!(
        DynamicFilterType::from_wire("M1"),
        Some(DynamicFilterType::January)
    );
    assert_eq!(DynamicFilterType::January.to_wire(), "M1");
    assert_eq!(
        DynamicFilterType::from_wire("M2"),
        Some(DynamicFilterType::February)
    );
    assert_eq!(DynamicFilterType::February.to_wire(), "M2");
    assert_eq!(
        DynamicFilterType::from_wire("M3"),
        Some(DynamicFilterType::March)
    );
    assert_eq!(DynamicFilterType::March.to_wire(), "M3");
    assert_eq!(
        DynamicFilterType::from_wire("M4"),
        Some(DynamicFilterType::April)
    );
    assert_eq!(DynamicFilterType::April.to_wire(), "M4");
    assert_eq!(
        DynamicFilterType::from_wire("M5"),
        Some(DynamicFilterType::May)
    );
    assert_eq!(DynamicFilterType::May.to_wire(), "M5");
    assert_eq!(
        DynamicFilterType::from_wire("M6"),
        Some(DynamicFilterType::June)
    );
    assert_eq!(DynamicFilterType::June.to_wire(), "M6");
    assert_eq!(
        DynamicFilterType::from_wire("M7"),
        Some(DynamicFilterType::July)
    );
    assert_eq!(DynamicFilterType::July.to_wire(), "M7");
    assert_eq!(
        DynamicFilterType::from_wire("M8"),
        Some(DynamicFilterType::August)
    );
    assert_eq!(DynamicFilterType::August.to_wire(), "M8");
    assert_eq!(
        DynamicFilterType::from_wire("M9"),
        Some(DynamicFilterType::September)
    );
    assert_eq!(DynamicFilterType::September.to_wire(), "M9");
    assert_eq!(
        DynamicFilterType::from_wire("M10"),
        Some(DynamicFilterType::October)
    );
    assert_eq!(DynamicFilterType::October.to_wire(), "M10");
    assert_eq!(
        DynamicFilterType::from_wire("M11"),
        Some(DynamicFilterType::November)
    );
    assert_eq!(DynamicFilterType::November.to_wire(), "M11");
    assert_eq!(
        DynamicFilterType::from_wire("M12"),
        Some(DynamicFilterType::December)
    );
    assert_eq!(DynamicFilterType::December.to_wire(), "M12");
}

/// `ST_PivotFilterType` (sml.xsd) → `spreadsheetml::PivotFilterType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_pivot_filter_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::PivotFilterType;
    assert_eq!(
        PivotFilterType::from_wire("Q1"),
        Some(PivotFilterType::FirstQuarter)
    );
    assert_eq!(PivotFilterType::FirstQuarter.to_wire(), "Q1");
    assert_eq!(
        PivotFilterType::from_wire("Q2"),
        Some(PivotFilterType::SecondQuarter)
    );
    assert_eq!(PivotFilterType::SecondQuarter.to_wire(), "Q2");
    assert_eq!(
        PivotFilterType::from_wire("Q3"),
        Some(PivotFilterType::ThirdQuarter)
    );
    assert_eq!(PivotFilterType::ThirdQuarter.to_wire(), "Q3");
    assert_eq!(
        PivotFilterType::from_wire("Q4"),
        Some(PivotFilterType::FourthQuarter)
    );
    assert_eq!(PivotFilterType::FourthQuarter.to_wire(), "Q4");
    assert_eq!(
        PivotFilterType::from_wire("M1"),
        Some(PivotFilterType::January)
    );
    assert_eq!(PivotFilterType::January.to_wire(), "M1");
    assert_eq!(
        PivotFilterType::from_wire("M2"),
        Some(PivotFilterType::February)
    );
    assert_eq!(PivotFilterType::February.to_wire(), "M2");
    assert_eq!(
        PivotFilterType::from_wire("M3"),
        Some(PivotFilterType::March)
    );
    assert_eq!(PivotFilterType::March.to_wire(), "M3");
    assert_eq!(
        PivotFilterType::from_wire("M4"),
        Some(PivotFilterType::April)
    );
    assert_eq!(PivotFilterType::April.to_wire(), "M4");
    assert_eq!(PivotFilterType::from_wire("M5"), Some(PivotFilterType::May));
    assert_eq!(PivotFilterType::May.to_wire(), "M5");
    assert_eq!(
        PivotFilterType::from_wire("M6"),
        Some(PivotFilterType::June)
    );
    assert_eq!(PivotFilterType::June.to_wire(), "M6");
    assert_eq!(
        PivotFilterType::from_wire("M7"),
        Some(PivotFilterType::July)
    );
    assert_eq!(PivotFilterType::July.to_wire(), "M7");
    assert_eq!(
        PivotFilterType::from_wire("M8"),
        Some(PivotFilterType::August)
    );
    assert_eq!(PivotFilterType::August.to_wire(), "M8");
    assert_eq!(
        PivotFilterType::from_wire("M9"),
        Some(PivotFilterType::September)
    );
    assert_eq!(PivotFilterType::September.to_wire(), "M9");
    assert_eq!(
        PivotFilterType::from_wire("M10"),
        Some(PivotFilterType::October)
    );
    assert_eq!(PivotFilterType::October.to_wire(), "M10");
    assert_eq!(
        PivotFilterType::from_wire("M11"),
        Some(PivotFilterType::November)
    );
    assert_eq!(PivotFilterType::November.to_wire(), "M11");
    assert_eq!(
        PivotFilterType::from_wire("M12"),
        Some(PivotFilterType::December)
    );
    assert_eq!(PivotFilterType::December.to_wire(), "M12");
}

/// `ST_ExternalConnectionType` (sml.xsd) → `spreadsheetml::TextFieldDataType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_text_field_data_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::TextFieldDataType;
    assert_eq!(
        TextFieldDataType::from_wire("MDY"),
        Some(TextFieldDataType::MonthDayYear)
    );
    assert_eq!(TextFieldDataType::MonthDayYear.to_wire(), "MDY");
    assert_eq!(
        TextFieldDataType::from_wire("DMY"),
        Some(TextFieldDataType::DayMonthYear)
    );
    assert_eq!(TextFieldDataType::DayMonthYear.to_wire(), "DMY");
    assert_eq!(
        TextFieldDataType::from_wire("YMD"),
        Some(TextFieldDataType::YearMonthDay)
    );
    assert_eq!(TextFieldDataType::YearMonthDay.to_wire(), "YMD");
    assert_eq!(
        TextFieldDataType::from_wire("MYD"),
        Some(TextFieldDataType::MonthYearDay)
    );
    assert_eq!(TextFieldDataType::MonthYearDay.to_wire(), "MYD");
    assert_eq!(
        TextFieldDataType::from_wire("DYM"),
        Some(TextFieldDataType::DayYearMonth)
    );
    assert_eq!(TextFieldDataType::DayYearMonth.to_wire(), "DYM");
    assert_eq!(
        TextFieldDataType::from_wire("YDM"),
        Some(TextFieldDataType::YearDayMonth)
    );
    assert_eq!(TextFieldDataType::YearDayMonth.to_wire(), "YDM");
    assert_eq!(
        TextFieldDataType::from_wire("EMD"),
        Some(TextFieldDataType::EastAsianYearMonthDay)
    );
    assert_eq!(TextFieldDataType::EastAsianYearMonthDay.to_wire(), "EMD");
}

/// `ST_DvAspect` (sml.xsd) → `spreadsheetml::DataViewAspect`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_data_view_aspect_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::DataViewAspect;
    assert_eq!(
        DataViewAspect::from_wire("DVASPECT_CONTENT"),
        Some(DataViewAspect::Content)
    );
    assert_eq!(DataViewAspect::Content.to_wire(), "DVASPECT_CONTENT");
    assert_eq!(
        DataViewAspect::from_wire("DVASPECT_ICON"),
        Some(DataViewAspect::Icon)
    );
    assert_eq!(DataViewAspect::Icon.to_wire(), "DVASPECT_ICON");
}

/// `ST_OleUpdate` (sml.xsd) → `spreadsheetml::OleUpdateType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_ole_update_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::OleUpdateType;
    assert_eq!(
        OleUpdateType::from_wire("OLEUPDATE_ALWAYS"),
        Some(OleUpdateType::Always)
    );
    assert_eq!(OleUpdateType::Always.to_wire(), "OLEUPDATE_ALWAYS");
    assert_eq!(
        OleUpdateType::from_wire("OLEUPDATE_ONCALL"),
        Some(OleUpdateType::OnCall)
    );
    assert_eq!(OleUpdateType::OnCall.to_wire(), "OLEUPDATE_ONCALL");
}

/// `ST_Axis` (sml.xsd) → `spreadsheetml::PivotTableAxis`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_pivot_table_axis_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::PivotTableAxis;
    assert_eq!(
        PivotTableAxis::from_wire("axisRow"),
        Some(PivotTableAxis::Row)
    );
    assert_eq!(PivotTableAxis::Row.to_wire(), "axisRow");
    assert_eq!(
        PivotTableAxis::from_wire("axisCol"),
        Some(PivotTableAxis::Column)
    );
    assert_eq!(PivotTableAxis::Column.to_wire(), "axisCol");
    assert_eq!(
        PivotTableAxis::from_wire("axisPage"),
        Some(PivotTableAxis::Page)
    );
    assert_eq!(PivotTableAxis::Page.to_wire(), "axisPage");
    assert_eq!(
        PivotTableAxis::from_wire("axisValues"),
        Some(PivotTableAxis::Values)
    );
    assert_eq!(PivotTableAxis::Values.to_wire(), "axisValues");
}

/// `ST_ItemType` (sml.xsd) → `spreadsheetml::PivotItemType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_pivot_item_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::PivotItemType;
    assert_eq!(
        PivotItemType::from_wire("countA"),
        Some(PivotItemType::CountNonEmpty)
    );
    assert_eq!(PivotItemType::CountNonEmpty.to_wire(), "countA");
    assert_eq!(
        PivotItemType::from_wire("stdDev"),
        Some(PivotItemType::StandardDeviation)
    );
    assert_eq!(PivotItemType::StandardDeviation.to_wire(), "stdDev");
    assert_eq!(
        PivotItemType::from_wire("stdDevP"),
        Some(PivotItemType::PopulationStandardDeviation)
    );
    assert_eq!(
        PivotItemType::PopulationStandardDeviation.to_wire(),
        "stdDevP"
    );
    assert_eq!(
        PivotItemType::from_wire("var"),
        Some(PivotItemType::Variance)
    );
    assert_eq!(PivotItemType::Variance.to_wire(), "var");
    assert_eq!(
        PivotItemType::from_wire("varP"),
        Some(PivotItemType::PopulationVariance)
    );
    assert_eq!(PivotItemType::PopulationVariance.to_wire(), "varP");
    assert_eq!(
        PivotItemType::from_wire("grand"),
        Some(PivotItemType::GrandTotal)
    );
    assert_eq!(PivotItemType::GrandTotal.to_wire(), "grand");
}

/// `ST_DataConsolidateFunction` (sml.xsd) → `spreadsheetml::DataConsolidateFunction`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_data_consolidate_function_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::DataConsolidateFunction;
    assert_eq!(
        DataConsolidateFunction::from_wire("count"),
        Some(DataConsolidateFunction::CountNonEmpty)
    );
    assert_eq!(DataConsolidateFunction::CountNonEmpty.to_wire(), "count");
    assert_eq!(
        DataConsolidateFunction::from_wire("countNums"),
        Some(DataConsolidateFunction::CountNumbers)
    );
    assert_eq!(DataConsolidateFunction::CountNumbers.to_wire(), "countNums");
    assert_eq!(
        DataConsolidateFunction::from_wire("stdDev"),
        Some(DataConsolidateFunction::SampleStandardDeviation)
    );
    assert_eq!(
        DataConsolidateFunction::SampleStandardDeviation.to_wire(),
        "stdDev"
    );
    assert_eq!(
        DataConsolidateFunction::from_wire("stdDevp"),
        Some(DataConsolidateFunction::PopulationStandardDeviation)
    );
    assert_eq!(
        DataConsolidateFunction::PopulationStandardDeviation.to_wire(),
        "stdDevp"
    );
    assert_eq!(
        DataConsolidateFunction::from_wire("var"),
        Some(DataConsolidateFunction::SampleVariance)
    );
    assert_eq!(DataConsolidateFunction::SampleVariance.to_wire(), "var");
    assert_eq!(
        DataConsolidateFunction::from_wire("varp"),
        Some(DataConsolidateFunction::PopulationVariance)
    );
    assert_eq!(
        DataConsolidateFunction::PopulationVariance.to_wire(),
        "varp"
    );
}

/// `ST_TotalsRowFunction` (sml.xsd) → `spreadsheetml::TotalsRowFunction`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_totals_row_function_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::TotalsRowFunction;
    assert_eq!(
        TotalsRowFunction::from_wire("count"),
        Some(TotalsRowFunction::CountNonEmpty)
    );
    assert_eq!(TotalsRowFunction::CountNonEmpty.to_wire(), "count");
    assert_eq!(
        TotalsRowFunction::from_wire("countNums"),
        Some(TotalsRowFunction::CountNumbers)
    );
    assert_eq!(TotalsRowFunction::CountNumbers.to_wire(), "countNums");
    assert_eq!(
        TotalsRowFunction::from_wire("stdDev"),
        Some(TotalsRowFunction::EstimatedStandardDeviation)
    );
    assert_eq!(
        TotalsRowFunction::EstimatedStandardDeviation.to_wire(),
        "stdDev"
    );
    assert_eq!(
        TotalsRowFunction::from_wire("var"),
        Some(TotalsRowFunction::EstimatedVariance)
    );
    assert_eq!(TotalsRowFunction::EstimatedVariance.to_wire(), "var");
    assert_eq!(
        TotalsRowFunction::from_wire("custom"),
        Some(TotalsRowFunction::CustomFormula)
    );
    assert_eq!(TotalsRowFunction::CustomFormula.to_wire(), "custom");
}

/// `ST_Comments` (sml.xsd) → `spreadsheetml::CommentDisplay`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_comment_display_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::CommentDisplay;
    assert_eq!(
        CommentDisplay::from_wire("commNone"),
        Some(CommentDisplay::NoComments)
    );
    assert_eq!(CommentDisplay::NoComments.to_wire(), "commNone");
    assert_eq!(
        CommentDisplay::from_wire("commIndicator"),
        Some(CommentDisplay::IndicatorOnly)
    );
    assert_eq!(CommentDisplay::IndicatorOnly.to_wire(), "commIndicator");
    assert_eq!(
        CommentDisplay::from_wire("commIndAndComment"),
        Some(CommentDisplay::IndicatorAndComment)
    );
    assert_eq!(
        CommentDisplay::IndicatorAndComment.to_wire(),
        "commIndAndComment"
    );
}

/// `ST_PatternType` (sml.xsd) → `spreadsheetml::PatternType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_pattern_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::PatternType;
    assert_eq!(
        PatternType::from_wire("gray125"),
        Some(PatternType::Gray12Point5Percent)
    );
    assert_eq!(PatternType::Gray12Point5Percent.to_wire(), "gray125");
    assert_eq!(
        PatternType::from_wire("gray0625"),
        Some(PatternType::Gray6Point25Percent)
    );
    assert_eq!(PatternType::Gray6Point25Percent.to_wire(), "gray0625");
}

/// `ST_CfvoType` (sml.xsd) → `spreadsheetml::ConditionalFormatValueObjectType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_conditional_format_value_object_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::ConditionalFormatValueObjectType;
    assert_eq!(
        ConditionalFormatValueObjectType::from_wire("num"),
        Some(ConditionalFormatValueObjectType::Number)
    );
    assert_eq!(ConditionalFormatValueObjectType::Number.to_wire(), "num");
}

/// `ST_PrintError` (sml.xsd) → `spreadsheetml::PrintError`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_print_error_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::PrintError;
    assert_eq!(PrintError::from_wire("NA"), Some(PrintError::NotAvailable));
    assert_eq!(PrintError::NotAvailable.to_wire(), "NA");
}

/// `ST_DataValidationImeMode` (sml.xsd) → `spreadsheetml::DataValidationImeMode`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_data_validation_ime_mode_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::DataValidationImeMode;
    assert_eq!(
        DataValidationImeMode::from_wire("fullKatakana"),
        Some(DataValidationImeMode::FullWidthKatakana)
    );
    assert_eq!(
        DataValidationImeMode::FullWidthKatakana.to_wire(),
        "fullKatakana"
    );
    assert_eq!(
        DataValidationImeMode::from_wire("halfKatakana"),
        Some(DataValidationImeMode::HalfWidthKatakana)
    );
    assert_eq!(
        DataValidationImeMode::HalfWidthKatakana.to_wire(),
        "halfKatakana"
    );
    assert_eq!(
        DataValidationImeMode::from_wire("fullAlpha"),
        Some(DataValidationImeMode::FullWidthAlphanumeric)
    );
    assert_eq!(
        DataValidationImeMode::FullWidthAlphanumeric.to_wire(),
        "fullAlpha"
    );
    assert_eq!(
        DataValidationImeMode::from_wire("halfAlpha"),
        Some(DataValidationImeMode::HalfWidthAlphanumeric)
    );
    assert_eq!(
        DataValidationImeMode::HalfWidthAlphanumeric.to_wire(),
        "halfAlpha"
    );
    assert_eq!(
        DataValidationImeMode::from_wire("fullHangul"),
        Some(DataValidationImeMode::FullWidthHangul)
    );
    assert_eq!(
        DataValidationImeMode::FullWidthHangul.to_wire(),
        "fullHangul"
    );
    assert_eq!(
        DataValidationImeMode::from_wire("halfHangul"),
        Some(DataValidationImeMode::HalfWidthHangul)
    );
    assert_eq!(
        DataValidationImeMode::HalfWidthHangul.to_wire(),
        "halfHangul"
    );
}

/// `ST_DataValidationType` (sml.xsd) → `spreadsheetml::DataValidationType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_data_validation_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::DataValidationType;
    assert_eq!(
        DataValidationType::from_wire("whole"),
        Some(DataValidationType::WholeNumber)
    );
    assert_eq!(DataValidationType::WholeNumber.to_wire(), "whole");
}

/// `ST_ShowDataAs` (sml.xsd) → `spreadsheetml::ShowDataAs`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_show_data_as_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::ShowDataAs;
    assert_eq!(
        ShowDataAs::from_wire("percentDiff"),
        Some(ShowDataAs::PercentageDifference)
    );
    assert_eq!(ShowDataAs::PercentageDifference.to_wire(), "percentDiff");
    assert_eq!(
        ShowDataAs::from_wire("runTotal"),
        Some(ShowDataAs::RunningTotal)
    );
    assert_eq!(ShowDataAs::RunningTotal.to_wire(), "runTotal");
}

/// `ST_Scope` (sml.xsd) → `spreadsheetml::ConditionalFormattingScope`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_conditional_formatting_scope_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::ConditionalFormattingScope;
    assert_eq!(
        ConditionalFormattingScope::from_wire("data"),
        Some(ConditionalFormattingScope::DataFields)
    );
    assert_eq!(ConditionalFormattingScope::DataFields.to_wire(), "data");
    assert_eq!(
        ConditionalFormattingScope::from_wire("field"),
        Some(ConditionalFormattingScope::FieldIntersections)
    );
    assert_eq!(
        ConditionalFormattingScope::FieldIntersections.to_wire(),
        "field"
    );
}

/// `ST_FileType` (sml.xsd) → `spreadsheetml::FileType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_file_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::FileType;
    assert_eq!(FileType::from_wire("mac"), Some(FileType::Macintosh));
    assert_eq!(FileType::Macintosh.to_wire(), "mac");
    assert_eq!(FileType::from_wire("win"), Some(FileType::Windows));
    assert_eq!(FileType::Windows.to_wire(), "win");
    assert_eq!(FileType::from_wire("lin"), Some(FileType::Linux));
    assert_eq!(FileType::Linux.to_wire(), "lin");
}

/// `ST_HtmlFmt` (sml.xsd) → `spreadsheetml::HtmlFormattingHandling`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_html_formatting_handling_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::HtmlFormattingHandling;
    assert_eq!(
        HtmlFormattingHandling::from_wire("rtf"),
        Some(HtmlFormattingHandling::RichText)
    );
    assert_eq!(HtmlFormattingHandling::RichText.to_wire(), "rtf");
}

/// `ST_ParameterType` (sml.xsd) → `spreadsheetml::ParameterType`: every value whose Rust name is sourced
/// from the ECMA-376 prose rather than mechanically derived from its token.
#[test]
fn spreadsheetml_parameter_type_overridden_tokens_round_trip() {
    use mjx_ooxml_types::spreadsheetml::ParameterType;
    assert_eq!(
        ParameterType::from_wire("prompt"),
        Some(ParameterType::PromptOnRefresh)
    );
    assert_eq!(ParameterType::PromptOnRefresh.to_wire(), "prompt");
}

/// Every token of every enumeration in `sml.xsd` — all 559 enumeration values, in schema
/// order. `sml.xsd` carries a 560th `xsd:enumeration`, the lone `255` inside the anonymous
/// inline member of `ST_TextRotation`'s union; that one is a number, not a named variant.
#[test]
fn every_spreadsheetml_token_round_trips() {
    use mjx_ooxml_types::spreadsheetml::*;
    assert_every_token_round_trips_to_its_own_variant! {
        FilterOperator => ["equal", "lessThan", "lessThanOrEqual", "notEqual", "greaterThanOrEqual", "greaterThan"],
        DynamicFilterType => ["null", "aboveAverage", "belowAverage", "tomorrow", "today", "yesterday", "nextWeek", "thisWeek", "lastWeek", "nextMonth", "thisMonth", "lastMonth", "nextQuarter", "thisQuarter", "lastQuarter", "nextYear", "thisYear", "lastYear", "yearToDate", "Q1", "Q2", "Q3", "Q4", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10", "M11", "M12"],
        IconSetType => ["3Arrows", "3ArrowsGray", "3Flags", "3TrafficLights1", "3TrafficLights2", "3Signs", "3Symbols", "3Symbols2", "4Arrows", "4ArrowsGray", "4RedToBlack", "4Rating", "4TrafficLights", "5Arrows", "5ArrowsGray", "5Rating", "5Quarters"],
        SortBy => ["value", "cellColor", "fontColor", "icon"],
        SortMethod => ["stroke", "pinYin", "none"],
        DateTimeGrouping => ["year", "month", "day", "hour", "minute", "second"],
        CommentTextHorizontalAlignment => ["left", "center", "right", "justify", "distributed"],
        CommentTextVerticalAlignment => ["top", "center", "bottom", "justify", "distributed"],
        CredentialsMethod => ["integrated", "none", "stored", "prompt"],
        HtmlFormattingHandling => ["none", "rtf", "all"],
        ParameterType => ["prompt", "value", "cell"],
        FileType => ["mac", "win", "dos", "lin", "other"],
        TextQualifier => ["doubleQuote", "singleQuote", "none"],
        TextFieldDataType => ["general", "text", "MDY", "DMY", "YMD", "MYD", "DYM", "YDM", "skip", "EMD"],
        PivotCacheSourceType => ["worksheet", "external", "consolidation", "scenario"],
        GroupBy => ["range", "seconds", "minutes", "hours", "days", "months", "quarters", "years"],
        SortType => ["none", "ascending", "descending", "ascendingAlpha", "descendingAlpha", "ascendingNatural", "descendingNatural"],
        ConditionalFormattingScope => ["selection", "data", "field"],
        TopNEvaluationType => ["none", "all", "row", "column"],
        ShowDataAs => ["normal", "difference", "percent", "percentDiff", "runTotal", "percentOfRow", "percentOfCol", "percentOfTotal", "index"],
        PivotItemType => ["data", "default", "sum", "countA", "avg", "max", "min", "product", "count", "stdDev", "stdDevP", "var", "varP", "grand", "blank"],
        PivotTableFormatAction => ["blank", "formatting", "drill", "formula"],
        FieldSortType => ["manual", "ascending", "descending"],
        PivotFilterType => ["unknown", "count", "percent", "sum", "captionEqual", "captionNotEqual", "captionBeginsWith", "captionNotBeginsWith", "captionEndsWith", "captionNotEndsWith", "captionContains", "captionNotContains", "captionGreaterThan", "captionGreaterThanOrEqual", "captionLessThan", "captionLessThanOrEqual", "captionBetween", "captionNotBetween", "valueEqual", "valueNotEqual", "valueGreaterThan", "valueGreaterThanOrEqual", "valueLessThan", "valueLessThanOrEqual", "valueBetween", "valueNotBetween", "dateEqual", "dateNotEqual", "dateOlderThan", "dateOlderThanOrEqual", "dateNewerThan", "dateNewerThanOrEqual", "dateBetween", "dateNotBetween", "tomorrow", "today", "yesterday", "nextWeek", "thisWeek", "lastWeek", "nextMonth", "thisMonth", "lastMonth", "nextQuarter", "thisQuarter", "lastQuarter", "nextYear", "thisYear", "lastYear", "yearToDate", "Q1", "Q2", "Q3", "Q4", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10", "M11", "M12"],
        PivotAreaType => ["none", "normal", "data", "all", "origin", "button", "topEnd", "topRight"],
        PivotTableAxis => ["axisRow", "axisCol", "axisPage", "axisValues"],
        GrowShrinkType => ["insertDelete", "insertClear", "overwriteClear"],
        PhoneticType => ["halfwidthKatakana", "fullwidthKatakana", "Hiragana", "noConversion"],
        PhoneticAlignment => ["noControl", "left", "center", "distributed"],
        RowColumnActionType => ["insertRow", "deleteRow", "insertCol", "deleteCol"],
        RevisionAction => ["add", "delete"],
        FormulaExpression => ["ref", "refError", "area", "areaError", "computedArea"],
        CellType => ["b", "n", "e", "s", "str", "inlineStr"],
        CellFormulaType => ["normal", "array", "dataTable", "shared"],
        Pane => ["bottomRight", "topRight", "bottomLeft", "topLeft"],
        SheetViewType => ["normal", "pageBreakPreview", "pageLayout"],
        DataConsolidateFunction => ["average", "count", "countNums", "max", "min", "product", "stdDev", "stdDevp", "sum", "var", "varp"],
        DataValidationType => ["none", "whole", "decimal", "list", "date", "time", "textLength", "custom"],
        DataValidationOperator => ["between", "notBetween", "equal", "notEqual", "lessThan", "lessThanOrEqual", "greaterThan", "greaterThanOrEqual"],
        DataValidationErrorStyle => ["stop", "warning", "information"],
        DataValidationImeMode => ["noControl", "off", "on", "disabled", "hiragana", "fullKatakana", "halfKatakana", "fullAlpha", "halfAlpha", "fullHangul", "halfHangul"],
        ConditionalFormatType => ["expression", "cellIs", "colorScale", "dataBar", "iconSet", "top10", "uniqueValues", "duplicateValues", "containsText", "notContainsText", "beginsWith", "endsWith", "containsBlanks", "notContainsBlanks", "containsErrors", "notContainsErrors", "timePeriod", "aboveAverage"],
        TimePeriod => ["today", "yesterday", "tomorrow", "last7Days", "thisMonth", "lastMonth", "nextMonth", "thisWeek", "lastWeek", "nextWeek"],
        ConditionalFormattingOperator => ["lessThan", "lessThanOrEqual", "equal", "notEqual", "greaterThanOrEqual", "greaterThan", "between", "notBetween", "containsText", "notContains", "beginsWith", "endsWith"],
        ConditionalFormatValueObjectType => ["num", "percent", "max", "min", "formula", "percentile"],
        PageOrder => ["downThenOver", "overThenDown"],
        PrintOrientation => ["default", "portrait", "landscape"],
        CellComments => ["none", "asDisplayed", "atEnd"],
        PrintError => ["displayed", "blank", "dash", "NA"],
        DataViewAspect => ["DVASPECT_CONTENT", "DVASPECT_ICON"],
        OleUpdateType => ["OLEUPDATE_ALWAYS", "OLEUPDATE_ONCALL"],
        WebSourceType => ["sheet", "printArea", "autoFilter", "range", "chart", "pivotTable", "query", "label"],
        PaneState => ["split", "frozen", "frozenSplit"],
        MdxFunctionType => ["m", "v", "s", "c", "r", "p", "k"],
        MdxSetOrder => ["u", "a", "d", "aa", "ad", "na", "nd"],
        MdxKeyPerformanceIndicatorProperty => ["v", "g", "s", "t", "w", "m"],
        BorderStyle => ["none", "thin", "medium", "dashed", "dotted", "thick", "double", "hair", "mediumDashed", "dashDot", "mediumDashDot", "dashDotDot", "mediumDashDotDot", "slantDashDot"],
        PatternType => ["none", "solid", "mediumGray", "darkGray", "lightGray", "darkHorizontal", "darkVertical", "darkDown", "darkUp", "darkGrid", "darkTrellis", "lightHorizontal", "lightVertical", "lightDown", "lightUp", "lightGrid", "lightTrellis", "gray125", "gray0625"],
        GradientType => ["linear", "path"],
        HorizontalAlignment => ["general", "left", "center", "right", "fill", "justify", "centerContinuous", "distributed"],
        VerticalAlignment => ["top", "center", "bottom", "justify", "distributed"],
        TableStyleType => ["wholeTable", "headerRow", "totalRow", "firstColumn", "lastColumn", "firstRowStripe", "secondRowStripe", "firstColumnStripe", "secondColumnStripe", "firstHeaderCell", "lastHeaderCell", "firstTotalCell", "lastTotalCell", "firstSubtotalColumn", "secondSubtotalColumn", "thirdSubtotalColumn", "firstSubtotalRow", "secondSubtotalRow", "thirdSubtotalRow", "blankRow", "firstColumnSubheading", "secondColumnSubheading", "thirdColumnSubheading", "firstRowSubheading", "secondRowSubheading", "thirdRowSubheading", "pageFieldLabels", "pageFieldValues"],
        FontScheme => ["none", "major", "minor"],
        UnderlineType => ["single", "double", "singleAccounting", "doubleAccounting", "none"],
        DynamicDataExchangeValueType => ["nil", "b", "n", "e", "str"],
        TableType => ["worksheet", "xml", "queryTable"],
        TotalsRowFunction => ["none", "sum", "min", "max", "average", "count", "countNums", "stdDev", "var", "custom"],
        VolatileDependencyType => ["realTimeData", "olapFunctions"],
        VolatileDependencyValueType => ["b", "n", "e", "s"],
        Visibility => ["visible", "hidden", "veryHidden"],
        CommentDisplay => ["commNone", "commIndicator", "commIndAndComment"],
        ObjectDisplay => ["all", "placeholders", "none"],
        SheetState => ["visible", "hidden", "veryHidden"],
        UpdateLinksBehavior => ["userSet", "never", "always"],
        SmartTagDisplay => ["all", "none", "noIndicator"],
        CalculationMode => ["manual", "auto", "autoNoTable"],
        ReferenceMode => ["A1", "R1C1"],
        TargetScreenSize => ["544x376", "640x480", "720x512", "800x600", "1024x768", "1152x882", "1152x900", "1280x1024", "1600x1200", "1800x1440", "1920x1200"],
    };
}

/// `ST_FontFamily` is declared by **both** `wml.xsd` and `sml.xsd`, and they are not the same kind
/// of thing: Word's is an enumeration of family *names*, Excel's a numeric family *code* (§18.18.94
/// — 0 not applicable, 1 Roman … 5 Decorative, 6–14 reserved). Word's was generated and committed
/// first, and it does not move: the per-schema naming engines keep the two apart, so
/// `wordprocessingml::FontFamily` still spells every one of its own tokens.
#[test]
fn the_two_font_families_are_different_kinds_of_type_in_different_modules() {
    use mjx_ooxml_types::spreadsheetml::FontFamilyNumber;
    use mjx_ooxml_types::wordprocessingml::FontFamily;

    // Word's, untouched by the arrival of Excel's.
    assert_eq!(FontFamily::from_wire("swiss"), Some(FontFamily::Swiss));
    assert_eq!(FontFamily::Swiss.to_wire(), "swiss");
    assert_eq!(FontFamily::Decorative.to_wire(), "decorative");
    // Excel's is a number, so it has no tokens at all — a `String` newtype here would have been a
    // silent modelling defect.
    let swiss: FontFamilyNumber = 2;
    let decorative: FontFamilyNumber = 5;
    assert_eq!(swiss + decorative, 7);
}

/// The other symbol `sml.xsd` shares with a sibling schema is `ST_Orientation`, which `dml-chart`
/// also declares. It reaches no committed name, because `dml-chart` contributes a child-order table
/// only and none of its simple types has ever been emitted. The name that *is* committed and could
/// have been disturbed is `presentationml::Orientation`, generated from `pml.xsd`'s `ST_Direction`;
/// Excel's page orientation is `PrintOrientation`, and neither accepts the other's tokens.
#[test]
fn excels_page_orientation_does_not_disturb_the_committed_orientation() {
    use mjx_ooxml_types::presentationml::Orientation;
    use mjx_ooxml_types::spreadsheetml::PrintOrientation;

    assert_eq!(
        Orientation::from_wire("horz"),
        Some(Orientation::Horizontal)
    );
    assert_eq!(Orientation::Horizontal.to_wire(), "horz");
    assert_eq!(Orientation::from_wire("landscape"), None);

    assert_eq!(
        PrintOrientation::from_wire("landscape"),
        Some(PrintOrientation::Landscape)
    );
    assert_eq!(PrintOrientation::Landscape.to_wire(), "landscape");
    assert_eq!(PrintOrientation::from_wire("horz"), None);
}

/// `sml.xsd`'s numeric types stay numbers. `ST_TextRotation` is the one type in the whole emitted
/// corpus whose `xsd:union` members are **anonymous inline** simple types rather than a
/// `memberTypes` list; both restrict `xsd:nonNegativeInteger` (0–180 degrees, plus 255), so the
/// union is that number. The six style-table identifiers are `xsd:unsignedInt` indices.
#[test]
fn spreadsheetml_measures_and_identifiers_are_numbers_not_strings() {
    use mjx_ooxml_types::spreadsheetml::{
        BorderId, CellStyleFormatId, DifferentialFormatId, FillId, FontId, NumberFormatId,
        TextRotation,
    };

    let upright: TextRotation = 0;
    let stacked: TextRotation = 255;
    assert_eq!(stacked - upright, 255);

    let number_format: NumberFormatId = 164;
    let font: FontId = 1;
    let fill: FillId = 2;
    let border: BorderId = 3;
    let cell_style: CellStyleFormatId = 4;
    let differential: DifferentialFormatId = 5;
    assert_eq!(
        number_format + font + fill + border + cell_style + differential,
        179
    );
}

/// The reference types are strings and stay verbatim: a formula or an `A1:B2` range is preserved
/// exactly as the document spelled it, never normalized.
#[test]
fn spreadsheetml_references_are_preserved_verbatim() {
    use mjx_ooxml_types::spreadsheetml::{
        CellRangeReference, CellReference, Formula, ReferenceSequence, SingleCellReference,
    };

    assert_eq!(CellReference::from_wire("B7").to_wire(), "B7");
    assert_eq!(CellRangeReference::from_wire("A1:B2").to_wire(), "A1:B2");
    assert_eq!(SingleCellReference::from_wire("$C$3").to_wire(), "$C$3");
    assert_eq!(
        ReferenceSequence::from_wire("A1:B2 D4:E5").to_wire(),
        "A1:B2 D4:E5"
    );
    assert_eq!(Formula::from_wire("SUM(A1:A9)").to_wire(), "SUM(A1:A9)");
}
