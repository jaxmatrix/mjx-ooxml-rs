//! Wire round-trip tests for the generated shared simple types: every value maps to its exact XSD
//! token and back, comprehensively-named variants resolve from their original OOXML spellings, and
//! the boolean normalizers collapse all spellings.

use std::str::FromStr;

use mjx_ooxml_types::namespaces;
use mjx_ooxml_types::shared::{
    CalendarType, ConformanceClass, CryptographicProvider, RelativeVerticalAlignment,
    VerticalTextPosition,
};

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
