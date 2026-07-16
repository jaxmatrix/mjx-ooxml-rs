//! Wire round-trip tests for the generated shared simple types: every value maps to its exact XSD
//! token and back, comprehensively-named variants resolve from their original OOXML spellings, and
//! the boolean normalizers collapse all spellings.

use std::str::FromStr;

use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces;
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
