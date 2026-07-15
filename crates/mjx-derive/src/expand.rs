//! Generating the `FromXml` / `ToXml` impls from the parsed IR.
//!
//! Every path into another crate is fully-qualified with a leading `::`, and every trait call uses
//! UFCS, so the generated code needs no `use` and cannot collide with names in the deriving crate.
//! The output reproduces the hand-written impls byte-for-byte in behavior (name moved not cloned;
//! the `empty && children.is_empty()` self-closing invariant; the both-URI child match).

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::{Container, TextLeaf, XmlType};

/// Generates the `FromXml` impl.
pub(crate) fn from_xml_impl(model: &XmlType) -> TokenStream {
    match model {
        XmlType::Container(container) => container_from_xml(container),
        XmlType::TextLeaf(leaf) => text_from_xml(leaf),
    }
}

/// Generates the `ToXml` impl.
pub(crate) fn to_xml_impl(model: &XmlType) -> TokenStream {
    match model {
        XmlType::Container(container) => container_to_xml(container),
        XmlType::TextLeaf(leaf) => text_to_xml(leaf),
    }
}

fn container_from_xml(container: &Container) -> TokenStream {
    let self_ty = &container.self_ty;
    let (impl_generics, type_generics, where_clause) = container.generics.split_for_impl();
    let content_field = &container.content_field;
    let enum_path = &container.enum_path;
    let raw = &container.raw_variant;

    let child_arms = container.children.iter().map(|child| {
        let local = &child.local;
        let namespace = &child.namespace;
        let variant = &child.variant;
        let child_ty = &child.ty;
        quote! {
            if local == #local
                && (namespace == ::core::option::Option::Some(#namespace.transitional)
                    || namespace == #namespace.strict)
            {
                content.push(#enum_path::#variant(
                    <#child_ty as ::mjx_ooxml_core::FromXml>::from_xml(child_element, interner)?,
                ));
                continue;
            }
        }
    });

    quote! {
        impl #impl_generics ::mjx_ooxml_core::FromXml for #self_ty #type_generics #where_clause {
            fn from_xml(
                element: &::mjx_ooxml_core::RawElement,
                interner: &::mjx_ooxml_core::Interner,
            ) -> ::core::result::Result<Self, ::mjx_ooxml_core::FromXmlError> {
                let mut content = ::std::vec::Vec::with_capacity(element.children.len());
                for child in &element.children {
                    if let ::mjx_ooxml_core::RawNode::Element(child_element) = child {
                        let local = interner.resolve(child_element.name.local);
                        let namespace =
                            child_element.name.namespace.map(|symbol| interner.resolve(symbol));
                        #(#child_arms)*
                    }
                    content.push(#enum_path::#raw(::core::clone::Clone::clone(child)));
                }
                ::core::result::Result::Ok(Self {
                    name: element.name,
                    attributes: ::core::clone::Clone::clone(&element.attributes),
                    empty: element.empty,
                    #content_field: content,
                })
            }
        }
    }
}

fn container_to_xml(container: &Container) -> TokenStream {
    let self_ty = &container.self_ty;
    let (impl_generics, type_generics, where_clause) = container.generics.split_for_impl();
    let content_field = &container.content_field;
    let enum_path = &container.enum_path;
    let raw = &container.raw_variant;

    let variant_arms = container.children.iter().map(|child| {
        let variant = &child.variant;
        let child_ty = &child.ty;
        quote! {
            #enum_path::#variant(value) => children.push(
                ::mjx_ooxml_core::RawNode::Element(
                    <#child_ty as ::mjx_ooxml_core::ToXml>::to_xml(value, interner),
                ),
            ),
        }
    });

    quote! {
        impl #impl_generics ::mjx_ooxml_core::ToXml for #self_ty #type_generics #where_clause {
            fn to_xml(
                &self,
                interner: &mut ::mjx_ooxml_core::Interner,
            ) -> ::mjx_ooxml_core::RawElement {
                let mut children = ::std::vec::Vec::with_capacity(self.#content_field.len());
                for item in &self.#content_field {
                    match item {
                        #(#variant_arms)*
                        #enum_path::#raw(node) => {
                            children.push(::core::clone::Clone::clone(node));
                        }
                    }
                }
                let empty = self.empty && children.is_empty();
                ::mjx_ooxml_core::RawElement {
                    name: self.name,
                    attributes: ::core::clone::Clone::clone(&self.attributes),
                    children,
                    empty,
                }
            }
        }
    }
}

fn text_from_xml(leaf: &TextLeaf) -> TokenStream {
    let self_ty = &leaf.self_ty;
    let (impl_generics, type_generics, where_clause) = leaf.generics.split_for_impl();
    let text_field = &leaf.text_field;

    quote! {
        impl #impl_generics ::mjx_ooxml_core::FromXml for #self_ty #type_generics #where_clause {
            fn from_xml(
                element: &::mjx_ooxml_core::RawElement,
                _interner: &::mjx_ooxml_core::Interner,
            ) -> ::core::result::Result<Self, ::mjx_ooxml_core::FromXmlError> {
                let mut text = ::std::string::String::new();
                for child in &element.children {
                    match child {
                        ::mjx_ooxml_core::RawNode::Text(bytes) => {
                            let raw = ::core::str::from_utf8(bytes)
                                .map_err(|_| ::mjx_ooxml_core::FromXmlError::InvalidUtf8)?;
                            let decoded = ::mjx_xml::text::unescape_text(raw).map_err(|error| {
                                ::mjx_ooxml_core::FromXmlError::InvalidEntity(
                                    ::std::string::ToString::to_string(&error),
                                )
                            })?;
                            text.push_str(&decoded);
                        }
                        ::mjx_ooxml_core::RawNode::CData(bytes) => {
                            let raw = ::core::str::from_utf8(bytes)
                                .map_err(|_| ::mjx_ooxml_core::FromXmlError::InvalidUtf8)?;
                            text.push_str(raw);
                        }
                        _ => {}
                    }
                }
                ::core::result::Result::Ok(Self {
                    name: element.name,
                    attributes: ::core::clone::Clone::clone(&element.attributes),
                    empty: element.empty,
                    #text_field: text,
                })
            }
        }
    }
}

fn text_to_xml(leaf: &TextLeaf) -> TokenStream {
    let self_ty = &leaf.self_ty;
    let (impl_generics, type_generics, where_clause) = leaf.generics.split_for_impl();
    let text_field = &leaf.text_field;

    quote! {
        impl #impl_generics ::mjx_ooxml_core::ToXml for #self_ty #type_generics #where_clause {
            fn to_xml(
                &self,
                _interner: &mut ::mjx_ooxml_core::Interner,
            ) -> ::mjx_ooxml_core::RawElement {
                let mut children = ::std::vec::Vec::new();
                if !self.#text_field.is_empty() {
                    let escaped = ::mjx_xml::text::escape_text(&self.#text_field);
                    children.push(::mjx_ooxml_core::RawNode::Text(
                        ::core::convert::Into::into(escaped.as_bytes()),
                    ));
                }
                let empty = self.empty && children.is_empty();
                ::mjx_ooxml_core::RawElement {
                    name: self.name,
                    attributes: ::core::clone::Clone::clone(&self.attributes),
                    children,
                    empty,
                }
            }
        }
    }
}
