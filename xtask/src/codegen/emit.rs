//! Renders committed Rust source for the shared simple types. Pure: `bytes → String`.

// This module emits source code, so explicit trailing newlines in `write!` are intentional
// (they lay out the generated Rust); `write_with_newline` would be noise here.
#![allow(clippy::write_with_newline)]

use std::fmt::Write as _;

use anyhow::{bail, Result};

use std::collections::BTreeMap;

use crate::codegen::naming::NameEngine;
use crate::codegen::spec;
use crate::codegen::xsd::{parse_simple_types, SimpleKind, SimpleType};

/// Module-level doc block for the `shared` module (see [`file_header`]).
pub const SHARED_MODULE_DOC: &str =
    "//! Comprehensively-named OOXML simple types (see the naming convention in `PLAN.md`).\n\
     //!\n\
     //! Each item records its original `ST_*` symbol and exact wire token(s).\n\n";

/// Module-level doc block for the `drawingml` module (see [`file_header`]).
pub const DRAWINGML_MODULE_DOC: &str =
    "//! Comprehensively-named DrawingML simple types (see the naming convention in `PLAN.md`).\n\
     //!\n\
     //! Selected from `dml-main.xsd`; each item records its original `ST_*` symbol and exact wire\n\
     //! token(s). Types join the allowlist as the DrawingML workstream ports them.\n\n";

/// Module-level doc block for the `presentationml` module (see [`file_header`]).
pub const PRESENTATIONML_MODULE_DOC: &str =
    "//! Comprehensively-named PresentationML simple types (see the naming convention in \
     `PLAN.md`).\n\
     //!\n\
     //! Selected from `pml.xsd`; each item records its original `ST_*` symbol and exact wire\n\
     //! token(s). Types join the allowlist as the PresentationML workstream ports them.\n\n";

/// Module-level doc block for the `wordprocessingml` module (see [`file_header`]).
pub const WORDPROCESSINGML_MODULE_DOC: &str =
    "//! Comprehensively-named WordprocessingML simple types (see the naming convention in \
     `PLAN.md`).\n\
     //!\n\
     //! The **whole** `ST_*` family of `wml.xsd`, not a slice: `mjx-docx` is modelled on top of\n\
     //! this vocabulary, so a missing type would be an enum invented somewhere else. Each item\n\
     //! records its original `ST_*` symbol and exact wire token(s).\n\n";

/// Module-level doc block for the `spreadsheetml` module (see [`file_header`]).
pub const SPREADSHEETML_MODULE_DOC: &str =
    "//! Comprehensively-named SpreadsheetML simple types (see the naming convention in \
     `PLAN.md`).\n\
     //!\n\
     //! The **whole** `ST_*` family of `sml.xsd`, not a slice: `mjx-sml` and `mjx-xlsx` are\n\
     //! modelled on top of this vocabulary, so a missing type would be an enum invented somewhere\n\
     //! else. Each item records its original `ST_*` symbol and exact wire token(s).\n\n";

/// Module-level doc block for the `officemath` module (see [`file_header`]).
pub const OFFICEMATH_MODULE_DOC: &str =
    "//! Comprehensively-named Office Math (OMML) simple types (see the naming convention in \
     `PLAN.md`).\n\
     //!\n\
     //! The whole `ST_*` family of `shared-math.xsd`. Each item records its original `ST_*` symbol\n\
     //! and exact wire token(s).\n\n";

/// Which of a schema's simple types a module emits.
#[derive(Debug, Clone, Copy)]
pub enum Selection<'a> {
    /// Every named `xsd:simpleType` the schema declares, in schema order.
    Everything,
    /// Only the listed types — for schemas where the un-curated remainder would be hundreds of
    /// names nobody has read yet. The list grows as each type is given a comprehensive name.
    Allowlist(&'a [&'a str]),
}

/// What one emitted module produced: its source, and the types that went into it.
///
/// The types come back so the caller can check the naming tables against reality — an override row
/// naming a type or a wire value the schema does not declare is a typo, and
/// [`spec::unused_overrides`] turns it into a build failure rather than a name nobody notices.
#[derive(Debug)]
pub struct EmittedModule {
    /// The rendered Rust source, before rustfmt.
    pub source: String,
    /// The simple types actually emitted, in schema order.
    pub types: Vec<SimpleType>,
}

/// Renders one module of simple types from a schema, under `module_doc`, named by `engine`.
///
/// Fails rather than emitting Rust that would not compile — or, worse, would compile with two
/// different wire tokens collapsed onto one variant: two emitted types that reach the same Rust
/// name, or two values of one enumeration that reach the same variant, are hard errors here.
pub fn emit_types(
    xsd: &[u8],
    source_note: &str,
    module_doc: &str,
    engine: &NameEngine,
    selection: Selection<'_>,
) -> Result<EmittedModule> {
    let all = parse_simple_types(xsd)?;
    let types: Vec<SimpleType> = match selection {
        Selection::Everything => all,
        Selection::Allowlist(list) => all
            .into_iter()
            .filter(|st| list.contains(&st.name.as_str()))
            .collect(),
    };

    let mut out = String::new();
    out.push_str(&file_header(source_note, module_doc));
    let mut type_names: BTreeMap<String, String> = BTreeMap::new();
    for st in &types {
        if spec::SKIP_TYPES.contains(&st.name.as_str()) {
            out.push_str(&emit_simple_type(st, engine));
            continue;
        }
        let rust = engine.type_name(&st.name);
        if let Some(first) = type_names.insert(rust.clone(), st.name.clone()) {
            bail!(
                "naming collision in {source_note}: `{first}` and `{}` both become `{rust}`",
                st.name
            );
        }
        // A two-valued type never becomes an enum — it becomes `bool` / `Option<bool>`, and its
        // wire spellings are normalized by `crate::support` rather than named one variant each.
        if let (SimpleKind::Enumeration { values, .. }, None) =
            (&st.kind, spec::bool_kind(&st.name))
        {
            let mut variants: BTreeMap<String, &str> = BTreeMap::new();
            for wire in values {
                let variant = engine.variant_name(&st.name, wire);
                if let Some(first) = variants.insert(variant.clone(), wire) {
                    bail!(
                        "naming collision in {source_note}: `{}`'s values {first:?} and {wire:?} \
                         both become `{variant}`, which would collapse two wire tokens onto one \
                         Rust value",
                        st.name
                    );
                }
            }
        }
        out.push_str(&emit_simple_type(st, engine));
    }
    Ok(EmittedModule { source: out, types })
}

/// Renders the Rust source for one simple type (skip comment, bool alias, enum, newtype, or numeric
/// alias — the classification the shared and selected emitters share).
fn emit_simple_type(st: &SimpleType, engine: &NameEngine) -> String {
    if spec::SKIP_TYPES.contains(&st.name.as_str()) {
        return format!(
            "// `{}` — subsumed by another representation; intentionally not emitted.\n\n",
            st.name
        );
    }
    if let Some((normalizer, optional)) = spec::bool_kind(&st.name) {
        return emit_bool_alias(&st.name, normalizer, optional, engine);
    }
    match &st.kind {
        SimpleKind::Enumeration { base, values } => emit_enum(&st.name, base, values, engine),
        SimpleKind::Restriction { base, pattern } => {
            if pattern.is_none() {
                if let Some(primitive) = spec::primitive_for(base) {
                    let phrase = format!("base `{base}`");
                    return emit_primitive_alias(&st.name, &phrase, primitive, engine);
                }
            }
            emit_string_newtype(&st.name, base, pattern.as_deref(), engine)
        }
        SimpleKind::Union { members } => {
            let note = format!("union of {}", members.join(" | "));
            // A union every member of which is the same number *is* that number. `sml.xsd`'s
            // `ST_TextRotation` unions two `xsd:nonNegativeInteger` restrictions (0–180 degrees,
            // plus 255); a `String` newtype for a count of degrees would be a worse model than the
            // one the schema states. Mixed unions stay strings, because no single Rust primitive
            // holds both halves.
            if let Some(primitive) = single_primitive(members) {
                let phrase = format!("a {note}");
                return emit_primitive_alias(&st.name, &phrase, primitive, engine);
            }
            emit_string_newtype(&st.name, &note, None, engine)
        }
        SimpleKind::List { item } => {
            let note = format!("list of {item}");
            emit_string_newtype(&st.name, &note, None, engine)
        }
    }
}

/// The Rust primitive a union collapses to, or `None` unless it has members and every one of them
/// resolves — through [`spec::primitive_for`] — to the *same* primitive.
fn single_primitive(members: &[String]) -> Option<&'static str> {
    let mut resolved = members.iter().map(|m| spec::primitive_for(m));
    let first = resolved.next().flatten()?;
    resolved.all(|p| p == Some(first)).then_some(first)
}

fn file_header(source_note: &str, module_doc: &str) -> String {
    format!(
        "// @generated by xtask — do not edit.\n\
         //\n\
         // Regenerate with: cargo run -p xtask -- codegen\n\
         // Source: {source_note}\n\
         #![allow(clippy::enum_variant_names)]\n\
         {module_doc}"
    )
}

fn emit_enum(st_name: &str, base: &str, values: &[String], engine: &NameEngine) -> String {
    let type_name = engine.type_name(st_name);
    let variants: Vec<(String, &str)> = values
        .iter()
        .map(|wire| (engine.variant_name(st_name, wire), wire.as_str()))
        .collect();

    let mut s = String::new();
    let _ = write!(
        s,
        "/// `{st_name}` — OOXML enumeration (base `{base}`). Wire tokens are preserved exactly.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
         pub enum {type_name} {{\n"
    );
    for (variant, wire) in &variants {
        let _ = write!(s, "    /// Wire value `{wire}`.\n    {variant},\n");
    }
    s.push_str("}\n\n");

    let _ = write!(s, "impl {type_name} {{\n");
    s.push_str("    /// Parses this value from its exact OOXML wire token.\n");
    s.push_str("    #[must_use]\n    pub fn from_wire(s: &str) -> Option<Self> {\n        Some(match s {\n");
    for (variant, wire) in &variants {
        let _ = writeln!(s, "            {wire:?} => Self::{variant},");
    }
    s.push_str("            _ => return None,\n        })\n    }\n\n");
    s.push_str("    /// The exact OOXML wire token for this value.\n");
    s.push_str(
        "    #[must_use]\n    pub fn to_wire(self) -> &'static str {\n        match self {\n",
    );
    for (variant, wire) in &variants {
        let _ = writeln!(s, "            Self::{variant} => {wire:?},");
    }
    s.push_str("        }\n    }\n}\n\n");

    let _ = write!(
        s,
        "impl core::fmt::Display for {type_name} {{\n\
         \x20   fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{\n\
         \x20       f.write_str(self.to_wire())\n    }}\n}}\n\n\
         impl core::str::FromStr for {type_name} {{\n\
         \x20   type Err = crate::UnknownWireValue;\n\
         \x20   fn from_str(s: &str) -> Result<Self, Self::Err> {{\n\
         \x20       Self::from_wire(s).ok_or_else(|| crate::UnknownWireValue::new(s))\n    }}\n}}\n\n"
    );
    s
}

fn emit_string_newtype(
    st_name: &str,
    base_note: &str,
    pattern: Option<&str>,
    engine: &NameEngine,
) -> String {
    let type_name = engine.type_name(st_name);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "/// `{st_name}` — string-valued OOXML type (base `{base_note}`)."
    );
    if let Some(p) = pattern {
        let _ = writeln!(
            s,
            "///\n/// XSD pattern `{p}`. Validation is deferred; the value is stored verbatim."
        );
    }
    let _ = write!(
        s,
        "#[derive(Debug, Clone, PartialEq, Eq, Hash)]\n\
         pub struct {type_name}(pub String);\n\n\
         impl {type_name} {{\n\
         \x20   /// Wraps a wire string.\n    #[must_use]\n\
         \x20   pub fn from_wire(s: &str) -> Self {{\n        Self(s.to_owned())\n    }}\n\n\
         \x20   /// The wire string.\n    #[must_use]\n\
         \x20   pub fn to_wire(&self) -> &str {{\n        &self.0\n    }}\n}}\n\n\
         impl core::fmt::Display for {type_name} {{\n\
         \x20   fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{\n\
         \x20       f.write_str(&self.0)\n    }}\n}}\n\n"
    );
    s
}

/// `base_phrase` is the parenthesised provenance, already spelled — `base \`xsd:int\`` for a plain
/// restriction, `a union of …` for a union every member of which is the same number.
fn emit_primitive_alias(
    st_name: &str,
    base_phrase: &str,
    primitive: &str,
    engine: &NameEngine,
) -> String {
    let type_name = engine.type_name(st_name);
    format!(
        "/// `{st_name}` — numeric OOXML type ({base_phrase}); a `{primitive}`.\n\
         pub type {type_name} = {primitive};\n\n"
    )
}

fn emit_bool_alias(st_name: &str, normalizer: &str, optional: bool, engine: &NameEngine) -> String {
    let type_name = engine.type_name(st_name);
    let rust_ty = if optional { "Option<bool>" } else { "bool" };
    format!(
        "/// `{st_name}` — a two-valued OOXML toggle, modeled as `{rust_ty}`.\n\
         ///\n\
         /// Every wire spelling is normalized on read and one canonical form is written; see\n\
         /// [`crate::support::{normalizer}`].\n\
         pub type {type_name} = {rust_ty};\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0"?>
        <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">
          <xsd:simpleType name="ST_VerticalAlignRun">
            <xsd:restriction base="xsd:string">
              <xsd:enumeration value="baseline"/>
              <xsd:enumeration value="superscript"/>
            </xsd:restriction>
          </xsd:simpleType>
          <xsd:simpleType name="ST_OnOff">
            <xsd:union memberTypes="xsd:boolean ST_OnOff1"/>
          </xsd:simpleType>
          <xsd:simpleType name="ST_OnOff1">
            <xsd:restriction base="xsd:string">
              <xsd:enumeration value="on"/>
            </xsd:restriction>
          </xsd:simpleType>
          <xsd:simpleType name="ST_Lang">
            <xsd:restriction base="xsd:string"/>
          </xsd:simpleType>
          <xsd:simpleType name="ST_UnsignedDecimalNumber">
            <xsd:restriction base="xsd:unsignedLong"/>
          </xsd:simpleType>
        </xsd:schema>"#;

    fn shared(xsd: &[u8]) -> Result<String> {
        Ok(emit_types(
            xsd,
            "test",
            SHARED_MODULE_DOC,
            &spec::ENGINE,
            Selection::Everything,
        )?
        .source)
    }

    #[test]
    fn emits_expected_shapes() {
        let src = shared(SAMPLE).unwrap();
        // enum with comprehensive names + wire mapping
        assert!(src.contains("pub enum VerticalTextPosition"));
        assert!(src.contains("Baseline,"));
        assert!(src.contains("\"superscript\" => Self::Superscript,"));
        // ST_OnOff -> bool alias; ST_OnOff1 skipped
        assert!(src.contains("pub type OnOff = bool;"));
        assert!(src.contains("subsumed by another representation"));
        assert!(!src.contains("pub enum OnOff1"));
        // string newtype + numeric alias
        assert!(src.contains("pub struct LanguageTag(pub String);"));
        assert!(src.contains("pub type UnsignedDecimalNumber = u64;"));
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(shared(SAMPLE).unwrap(), shared(SAMPLE).unwrap());
    }

    #[test]
    fn an_allowlist_emits_only_what_it_names() {
        let src = emit_types(
            SAMPLE,
            "test",
            SHARED_MODULE_DOC,
            &spec::ENGINE,
            Selection::Allowlist(&["ST_Lang"]),
        )
        .unwrap();
        assert!(src.source.contains("pub struct LanguageTag(pub String);"));
        assert!(!src.source.contains("VerticalTextPosition"));
        assert_eq!(src.types.len(), 1);
    }

    /// Two enumeration values that reach the same Rust variant would compile — and silently make
    /// one of the two wire tokens unwritable. The generator refuses instead.
    #[test]
    fn two_values_reaching_one_variant_is_a_hard_error() {
        const CLASHING: &[u8] = br#"<?xml version="1.0"?>
            <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">
              <xsd:simpleType name="ST_Clash">
                <xsd:restriction base="xsd:string">
                  <xsd:enumeration value="--"/>
                  <xsd:enumeration value="-+"/>
                </xsd:restriction>
              </xsd:simpleType>
            </xsd:schema>"#;
        let err = emit_types(
            CLASHING,
            "test",
            SHARED_MODULE_DOC,
            &spec::ENGINE,
            Selection::Everything,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("collapse two wire tokens"), "{err}");
    }

    /// A union whose members are all the same number is that number. Without this, `sml.xsd`'s
    /// `ST_TextRotation` — a count of degrees — would be a `String` newtype.
    #[test]
    fn a_union_of_one_numeric_base_becomes_that_number() {
        const UNION: &[u8] = br#"<?xml version="1.0"?>
            <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">
              <xsd:simpleType name="ST_Rotation">
                <xsd:union>
                  <xsd:simpleType>
                    <xsd:restriction base="xsd:nonNegativeInteger">
                      <xsd:maxInclusive value="180"/>
                    </xsd:restriction>
                  </xsd:simpleType>
                  <xsd:simpleType>
                    <xsd:restriction base="xsd:nonNegativeInteger">
                      <xsd:enumeration value="255"/>
                    </xsd:restriction>
                  </xsd:simpleType>
                </xsd:union>
              </xsd:simpleType>
            </xsd:schema>"#;
        let src = shared(UNION).unwrap();
        assert!(src.contains("pub type Rotation = u64;"), "{src}");
        assert!(!src.contains("pub struct Rotation"), "{src}");
        // The provenance stays in the docs, both members of it.
        assert!(
            src.contains("a union of xsd:nonNegativeInteger | xsd:nonNegativeInteger"),
            "{src}"
        );
    }

    /// A union of *different* kinds has no single primitive, so it stays a verbatim string —
    /// `ST_OnOff`'s shape, and every `memberTypes` union in `dml-main`.
    #[test]
    fn a_mixed_union_stays_a_string_newtype() {
        const MIXED: &[u8] = br#"<?xml version="1.0"?>
            <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">
              <xsd:simpleType name="ST_Mixed">
                <xsd:union memberTypes="xsd:unsignedLong s:ST_UniversalMeasure"/>
              </xsd:simpleType>
            </xsd:schema>"#;
        let src = shared(MIXED).unwrap();
        assert!(src.contains("pub struct Mixed(pub String);"), "{src}");
        assert!(!src.contains("pub type Mixed"), "{src}");
    }

    /// The same rule one level up: two `ST_*` types that reach one Rust type name.
    #[test]
    fn two_types_reaching_one_name_is_a_hard_error() {
        const CLASHING: &[u8] = br#"<?xml version="1.0"?>
            <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">
              <xsd:simpleType name="ST_Thing">
                <xsd:restriction base="xsd:string"/>
              </xsd:simpleType>
              <xsd:simpleType name="ST_THING">
                <xsd:restriction base="xsd:string"/>
              </xsd:simpleType>
            </xsd:schema>"#;
        let err = emit_types(
            CLASHING,
            "test",
            SHARED_MODULE_DOC,
            &spec::ENGINE,
            Selection::Everything,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("both become `Thing`"), "{err}");
    }
}
