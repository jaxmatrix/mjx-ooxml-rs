//! `mjx-derive` — `#[derive(FromXml, ToXml)]` for the mjx-ooxml typed models.
//!
//! These two derives generate the `FromXml` / `ToXml` trait impls (defined in `mjx-ooxml-core`) that
//! a typed model implements to parse itself out of, and rebuild, a raw preservation-tree element —
//! reproducing the fidelity discipline of the hand-written impls exactly (preserve the element name
//! with its prefix, all attributes, the self-closing flag, and every child the type does not itself
//! model).
//!
//! # Deriving on a struct
//!
//! A derivable struct has exactly three "framework" fields identified by name — `name: RawName`,
//! `attributes: Vec<RawAttribute>`, `empty: bool` — plus exactly one content field:
//!
//! - a **container** field marked `#[xml(children, child(local = "..", variant = .., ty = ..))]`,
//!   of type `Vec<SomeContentEnum>`, where the enum's typed variants are declared by the `child(..)`
//!   entries and an implicit `Raw(RawNode)` catch-all variant preserves everything unmatched; or
//! - a **text** field marked `#[xml(text)]`, of type `String`, holding decoded character data.
//!
//! A struct-level `#[xml(namespace = DML_MAIN)]` sets the default namespace for every `child`; a bare
//! namespace ident is resolved against `mjx_ooxml_types::namespaces`, and a multi-segment path is used
//! verbatim. Children are matched on `(namespace, local)`, accepting both the strict and transitional
//! namespace URIs, and never on prefix. See the crate `mjx-dml` for the reference usage.
//!
//! # Deriving typed attributes
//!
//! `#[derive(XmlAttributes)]` is separate from the two above and composes with them (or with a
//! hand-written pair of impls, or with none): it asks only for the retained
//! `attributes: Vec<RawAttribute>` field and generates a **getter and a setter over that vector** for
//! each declared attribute. It never rebuilds the vector, which is the whole point — an attribute the
//! model has never heard of keeps its position, its prefix and its quote character, and so does one
//! the model knows about but nobody assigned to.
//!
//! Attributes are declared at the **struct** level, because there is no field to hang them on:
//!
//! ```ignore
//! #[derive(FromXml, ToXml, XmlAttributes)]
//! #[xml(namespace = DML_MAIN)]
//! #[xml(attribute(local = "val", codec = HexColorRgb, required))]
//! #[xml(attribute(local = "rtlCol", codec = OnOff, default = false))]
//! #[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
//! #[xml(attribute(local = "embed", prefix = "r", codec = Text, accessor = image_relationship))]
//! struct SolidColor { name: RawName, attributes: Vec<RawAttribute>, empty: bool, /* .. */ }
//! ```
//!
//! | key | meaning |
//! |-----|---------|
//! | `local = ".."` | **required.** The exact wire local name. |
//! | `codec = ..` | **required.** A type implementing [`AttributeCodec`], which decides how values read and write. A `Type`, so `Enumeration<LineCap>` is fine. |
//! | `prefix = ".."` | Matches and writes a prefixed attribute (`r:embed`). Absent means an unprefixed one, which per XML is in no namespace. |
//! | `accessor = ident` | The Rust base name. Defaults to the wire name in snake case; set it wherever the wire token is cryptic, which the naming convention requires. |
//! | `required` | Absent is [`AttributeError::Missing`], never a substituted value. |
//! | `default = expr` | The schema default, returned when the attribute is absent. Never written: an absent attribute stays absent. Contradicts `required`. |
//!
//! Neither `required` nor `default` makes the attribute *optional* — that is the third case, and it
//! is what you get by writing neither. The three presences differ only in the getter's return type
//! (`T`, `Option<T>`, `T`) and in what an absent attribute means.
//!
//! ## Read never normalizes; a write does
//!
//! A getter takes `&self` and cannot change the file: `rtlCol='on'` that nobody touched still writes
//! `on`, single-quoted, in its original position. `set_rtl_col(true)` writes the canonical `true`.
//! That asymmetry is the contract — a grammar that canonicalized on read would rewrite every file it
//! opened, invisibly, because our reader and our writer would agree with each other.
//!
//! [`AttributeCodec`]: mjx_ooxml_core::AttributeCodec
//! [`AttributeError::Missing`]: mjx_ooxml_core::AttributeError::Missing
//!
//! The generated code refers to `mjx-ooxml-core`, `mjx-xml`, and `mjx-ooxml-types` by fully-qualified
//! path, so the deriving crate must depend on those three; `mjx-derive` itself does not.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod expand;
mod naming;
mod parse;

/// Derives `FromXml` — parses the type out of a raw element, matching modeled children by
/// `(namespace, local)` and preserving everything else. See the crate docs for the `#[xml(..)]`
/// attribute grammar.
#[proc_macro_derive(FromXml, attributes(xml))]
pub fn derive_from_xml(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse::XmlType::from_derive_input(&input) {
        Ok(model) => expand::from_xml_impl(&model).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives `ToXml` — rebuilds a raw element from the type, reusing the preserved name, attributes,
/// and unmodeled children. See the crate docs for the `#[xml(..)]` attribute grammar.
#[proc_macro_derive(ToXml, attributes(xml))]
pub fn derive_to_xml(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse::XmlType::from_derive_input(&input) {
        Ok(model) => expand::to_xml_impl(&model).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives typed accessors for the attributes declared with struct-level
/// `#[xml(attribute(local = .., codec = .., ..))]`.
///
/// One getter and one setter per attribute, as inherent methods over the type's retained
/// `attributes: Vec<RawAttribute>` — which stays the source of truth, so unknown attributes, their
/// order, their prefixes and their quote characters all survive. See the crate docs for the grammar.
#[proc_macro_derive(XmlAttributes, attributes(xml))]
pub fn derive_xml_attributes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse::AttributeModel::from_derive_input(&input) {
        Ok(model) => expand::xml_attributes_impl(&model).into(),
        Err(error) => error.to_compile_error().into(),
    }
}
