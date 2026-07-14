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
