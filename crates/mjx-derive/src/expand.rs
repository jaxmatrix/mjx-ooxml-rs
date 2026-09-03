//! Generating the `FromXml` / `ToXml` impls from the parsed IR.
//!
//! Every path into another crate is fully-qualified with a leading `::`, and every trait call uses
//! UFCS, so the generated code needs no `use` and cannot collide with names in the deriving crate.
//! The output reproduces the hand-written impls byte-for-byte in behavior (name moved not cloned;
//! the `empty && children.is_empty()` self-closing invariant; the both-URI child match).

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::{AttributeModel, AttributeSpec, Container, Presence, TextLeaf, XmlType};

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
                ::mjx_ooxml_core::RawElement::new(
                    self.name,
                    ::core::clone::Clone::clone(&self.attributes),
                    children,
                    empty,
                )
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
                ::mjx_ooxml_core::RawElement::new(
                    self.name,
                    ::core::clone::Clone::clone(&self.attributes),
                    children,
                    empty,
                )
            }
        }
    }
}

/// Generates the typed attribute accessors: one getter and one setter per declared attribute, as
/// inherent methods over the type's retained `attributes` vector.
///
/// Nothing here builds an attribute list. A getter borrows the vector; a setter reaches exactly one
/// element of it. That is what makes an attribute the model has never heard of — and the position,
/// prefix and quote character of one it has — survive a round-trip untouched.
pub(crate) fn xml_attributes_impl(model: &AttributeModel) -> TokenStream {
    let self_ty = &model.self_ty;
    let (impl_generics, type_generics, where_clause) = model.generics.split_for_impl();
    let accessors = model
        .attributes
        .iter()
        .map(|spec| accessor_pair(model, spec));

    quote! {
        impl #impl_generics #self_ty #type_generics #where_clause {
            #(#accessors)*
        }
    }
}

/// The getter/setter pair for one declared attribute.
fn accessor_pair(model: &AttributeModel, spec: &AttributeSpec) -> TokenStream {
    let vis = &model.vis;
    let codec = &spec.codec;
    let getter = &spec.getter;
    let setter = &spec.setter;
    let local = &spec.local;
    let qualified = spec.qualified.as_str();
    let prefix = match &spec.prefix {
        Some(prefix) => quote!(::core::option::Option::Some(#prefix)),
        None => quote!(::core::option::Option::None),
    };

    // The three presences differ in exactly two places: what the getter returns when the attribute
    // is absent, and whether the value is wrapped in an `Option`.
    let (value_ty, present, absent, getter_doc) = match &spec.presence {
        Presence::Required => (
            quote!(<#codec as ::mjx_ooxml_core::AttributeCodec>::Value<'a>),
            quote!(::core::result::Result::Ok(value)),
            quote!(::core::result::Result::Err(
                ::mjx_ooxml_core::AttributeError::Missing { attribute: #qualified }
            )),
            format!(
                "Reads the required `@{qualified}` attribute.\n\n\
                 The value is decoded from the bytes in the file and **not** normalized: an \
                 attribute nobody assigns to keeps its own spelling, quote character and position.\n\n\
                 # Errors\n\
                 [`AttributeError::Missing`](::mjx_ooxml_core::AttributeError::Missing) if the \
                 attribute is absent — a required attribute has no default, and substituting one \
                 would assert something the file does not say — or another \
                 [`AttributeError`](::mjx_ooxml_core::AttributeError) if its value is malformed."
            ),
        ),
        Presence::Optional => (
            quote!(::core::option::Option<<#codec as ::mjx_ooxml_core::AttributeCodec>::Value<'a>>),
            quote!(::core::result::Result::Ok(::core::option::Option::Some(value))),
            quote!(::core::result::Result::Ok(::core::option::Option::None)),
            format!(
                "Reads the optional `@{qualified}` attribute, or `None` when it is absent.\n\n\
                 The value is decoded from the bytes in the file and **not** normalized: an \
                 attribute nobody assigns to keeps its own spelling, quote character and position.\n\n\
                 # Errors\n\
                 An [`AttributeError`](::mjx_ooxml_core::AttributeError) if the attribute is present \
                 but its value is malformed."
            ),
        ),
        Presence::Defaulted(default) => (
            quote!(<#codec as ::mjx_ooxml_core::AttributeCodec>::Value<'a>),
            quote!(::core::result::Result::Ok(value)),
            quote!(::core::result::Result::Ok(#default)),
            format!(
                "Reads the `@{qualified}` attribute, falling back to the schema default when it is \
                 absent.\n\n\
                 The value is decoded from the bytes in the file and **not** normalized: an \
                 attribute nobody assigns to keeps its own spelling, quote character and position. \
                 The default is returned, never written — an absent attribute stays absent.\n\n\
                 # Errors\n\
                 An [`AttributeError`](::mjx_ooxml_core::AttributeError) if the attribute is present \
                 but its value is malformed."
            ),
        ),
    };

    let (input_ty, write, setter_doc) = match &spec.presence {
        Presence::Required => (
            quote!(<#codec as ::mjx_ooxml_core::AttributeCodec>::Input<'_>),
            quote! {
                let encoded = <#codec as ::mjx_ooxml_core::AttributeCodec>::encode(value);
                ::mjx_xml::attribute::set(
                    &mut self.attributes, interner, #prefix, #local, &encoded,
                );
            },
            format!(
                "Writes the required `@{qualified}` attribute, in the one canonical spelling this \
                 kind of value has.\n\n\
                 An attribute already present is rewritten **where it is**, keeping its position \
                 among its siblings and the quote character the file used; a new one is appended, \
                 double-quoted. Every other attribute — including any this type does not model — is \
                 left exactly as it was."
            ),
        ),
        Presence::Optional | Presence::Defaulted(_) => (
            quote!(::core::option::Option<<#codec as ::mjx_ooxml_core::AttributeCodec>::Input<'_>>),
            quote! {
                match value {
                    ::core::option::Option::Some(value) => {
                        let encoded = <#codec as ::mjx_ooxml_core::AttributeCodec>::encode(value);
                        ::mjx_xml::attribute::set(
                            &mut self.attributes, interner, #prefix, #local, &encoded,
                        );
                    }
                    ::core::option::Option::None => {
                        ::mjx_xml::attribute::remove(
                            &mut self.attributes, interner, #prefix, #local,
                        );
                    }
                }
            },
            format!(
                "Writes the `@{qualified}` attribute, in the one canonical spelling this kind of \
                 value has, or removes it entirely when given `None`.\n\n\
                 An attribute already present is rewritten **where it is**, keeping its position \
                 among its siblings and the quote character the file used; a new one is appended, \
                 double-quoted. Every other attribute — including any this type does not model — is \
                 left exactly as it was."
            ),
        ),
    };

    quote! {
        #[doc = #getter_doc]
        #vis fn #getter<'a>(
            &'a self,
            interner: &::mjx_ooxml_core::Interner,
        ) -> ::core::result::Result<#value_ty, ::mjx_ooxml_core::AttributeError> {
            match ::mjx_xml::attribute::find(&self.attributes, interner, #prefix, #local) {
                ::core::option::Option::Some(attribute) => {
                    let raw = ::mjx_xml::attribute::decoded_value(attribute, #qualified)?;
                    let value = <#codec as ::mjx_ooxml_core::AttributeCodec>::decode(raw)
                        .map_err(|invalid| {
                            ::mjx_ooxml_core::InvalidAttributeValue::into_error(invalid, #qualified)
                        })?;
                    #present
                }
                ::core::option::Option::None => #absent,
            }
        }

        #[doc = #setter_doc]
        #vis fn #setter(
            &mut self,
            interner: &mut ::mjx_ooxml_core::Interner,
            value: #input_ty,
        ) {
            #write
        }
    }
}
