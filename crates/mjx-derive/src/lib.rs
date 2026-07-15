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
//! The generated code refers to `mjx-ooxml-core`, `mjx-xml`, and `mjx-ooxml-types` by fully-qualified
//! path, so the deriving crate must depend on those three; `mjx-derive` itself does not.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod expand;
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
