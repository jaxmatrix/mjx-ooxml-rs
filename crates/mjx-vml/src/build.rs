//! Internal readers and builders for VML elements — attribute access, element construction, and the
//! fidelity leaf macro.
//!
//! # Why attribute access needs its own helpers
//!
//! The fidelity reader resolves an *element's* namespace but leaves an *attribute's* prefix
//! unresolved (attributes are namespaced far less often, and a prefix is what the writer re-emits).
//! VML is the one vocabulary where that matters: a `v:shape` carries both an unprefixed `id` (the
//! shape's own identifier) and, on some children, an `r:id` (a relationship reference). Matching on
//! the local name alone would confuse the two.
//!
//! So this module splits the two cases:
//!
//! - [`attribute`] matches an **unprefixed** attribute exactly (`prefix == None`), which is the
//!   overwhelming majority of VML and is unambiguous;
//! - [`namespaced_attribute`] matches a **prefixed** one, resolving the prefix through any
//!   declaration the element itself carries and otherwise through the conventional prefix
//!   ([`conventional_prefix`]) that ECMA-376 Part 4 §19 binds each VML namespace to in every example
//!   and that every producer emits.

use mjx_ooxml_core::{Interner, QuoteStyle, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::{
    SchemaNamespace, SHARED_RELATIONSHIP_REFERENCE, VML_MAIN, VML_OFFICE_DRAWING,
    VML_PRESENTATION_DRAWING, VML_SPREADSHEET_DRAWING, VML_WORDPROCESSING_DRAWING,
};
use mjx_xml::text::{escape_attribute, unescape_text};

/// The prefix ECMA-376 Part 4 §19 binds `namespace` to in every example, and that every producer
/// emits: `v` for VML itself, `o` for the Office drawing extensions, `p` for the PowerPoint ones,
/// `x` for the spreadsheet ones, `w10` for the wordprocessing ones, and `r` for relationship
/// references.
///
/// Returns `None` for a namespace VML does not use, which makes a lookup against it fail rather
/// than match something by accident.
pub(crate) fn conventional_prefix(namespace: SchemaNamespace) -> Option<&'static str> {
    let uri = namespace.transitional;
    if uri == VML_MAIN.transitional {
        Some("v")
    } else if uri == VML_OFFICE_DRAWING.transitional {
        Some("o")
    } else if uri == VML_PRESENTATION_DRAWING.transitional {
        Some("p")
    } else if uri == VML_SPREADSHEET_DRAWING.transitional {
        Some("x")
    } else if uri == VML_WORDPROCESSING_DRAWING.transitional {
        Some("w10")
    } else if uri == SHARED_RELATIONSHIP_REFERENCE.transitional {
        Some("r")
    } else {
        None
    }
}

/// Whether `name` resolves to `namespace`, accepting either the strict or the transitional URI —
/// the same both-URI match the `mjx-derive` child arms use.
pub(crate) fn name_is(
    name: &RawName,
    interner: &Interner,
    namespace: SchemaNamespace,
    local: &str,
) -> bool {
    if interner.resolve(name.local) != local {
        return false;
    }
    let resolved = name.namespace.map(|symbol| interner.resolve(symbol));
    resolved == Some(namespace.transitional) || resolved == namespace.strict
}

/// The prefix to read or write `namespace`-qualified attributes with on `element`: whichever prefix
/// `element` itself declares for `namespace`, else the conventional one — unless `element` rebinds
/// that conventional prefix to a *different* namespace, in which case there is no usable prefix and
/// a lookup must not match.
pub(crate) fn prefix_for<'a>(
    attributes: &[RawAttribute],
    interner: &'a Interner,
    namespace: SchemaNamespace,
) -> Option<&'a str> {
    let mut rebound_conventional = false;
    let conventional = conventional_prefix(namespace);
    for attribute in attributes {
        let Some(prefix) = attribute.name.prefix else {
            continue;
        };
        if interner.resolve(prefix) != "xmlns" {
            continue;
        }
        let declared = interner.resolve(attribute.name.local);
        let Ok(uri) = std::str::from_utf8(&attribute.value) else {
            continue;
        };
        let uri = unescape_text(uri).unwrap_or(std::borrow::Cow::Borrowed(uri));
        if uri == namespace.transitional || Some(uri.as_ref()) == namespace.strict {
            // A declaration on the element itself wins: it is what the attribute prefix resolves to.
            return Some(declared);
        }
        if Some(declared) == conventional {
            rebound_conventional = true;
        }
    }
    if rebound_conventional {
        return None;
    }
    conventional
}

/// The UTF-8 value of the **unprefixed** attribute named `local`, unescaped, or `None` when the
/// element has none (or its bytes are not UTF-8).
pub(crate) fn attribute<'a>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    local: &str,
) -> Option<std::borrow::Cow<'a, str>> {
    attributes
        .iter()
        .find(|attribute| {
            attribute.name.prefix.is_none() && interner.resolve(attribute.name.local) == local
        })
        .and_then(|attribute| decode(&attribute.value))
}

/// The UTF-8 value of the attribute `local` qualified by `namespace`, unescaped, or `None` when the
/// element has none. See the [module docs](self) for how the prefix is resolved.
pub(crate) fn namespaced_attribute<'a>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    namespace: SchemaNamespace,
    local: &str,
) -> Option<std::borrow::Cow<'a, str>> {
    let prefix = prefix_for(attributes, interner, namespace)?;
    attributes
        .iter()
        .find(|attribute| {
            attribute
                .name
                .prefix
                .is_some_and(|symbol| interner.resolve(symbol) == prefix)
                && interner.resolve(attribute.name.local) == local
        })
        .and_then(|attribute| decode(&attribute.value))
}

/// Decodes a raw attribute value: UTF-8 plus entity unescaping, falling back to the raw text when an
/// entity cannot be decoded (a read accessor never fails on an input we only preserve).
fn decode(value: &[u8]) -> Option<std::borrow::Cow<'_, str>> {
    let raw = std::str::from_utf8(value).ok()?;
    Some(unescape_text(raw).unwrap_or(std::borrow::Cow::Borrowed(raw)))
}

/// Sets the unprefixed attribute `local` to `value`, rewriting the existing one in place (so
/// attribute order is preserved) or appending it.
pub(crate) fn set_attribute(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    local: &str,
    value: &str,
) {
    let symbol = interner.intern(local);
    let escaped: Box<[u8]> = escape_attribute(value).as_bytes().into();
    if let Some(existing) = attributes
        .iter_mut()
        .find(|attribute| attribute.name.prefix.is_none() && attribute.name.local == symbol)
    {
        existing.value = escaped;
        return;
    }
    attributes.push(RawAttribute {
        name: RawName {
            prefix: None,
            local: symbol,
            namespace: None,
        },
        value: escaped,
        quote: QuoteStyle::Double,
    });
}

/// Sets the `namespace`-qualified attribute `local` to `value`, rewriting the existing one in place
/// or appending it. Does nothing when the element rebinds the namespace's conventional prefix to
/// something else — there is then no spelling of the attribute that would mean what the caller asked
/// for, and inventing one would corrupt the part.
pub(crate) fn set_namespaced_attribute(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    namespace: SchemaNamespace,
    local: &str,
    value: &str,
) {
    let Some(prefix) = prefix_for(attributes, interner, namespace).map(str::to_owned) else {
        return;
    };
    let prefix_symbol = interner.intern(&prefix);
    let local_symbol = interner.intern(local);
    let escaped: Box<[u8]> = escape_attribute(value).as_bytes().into();
    if let Some(existing) = attributes.iter_mut().find(|attribute| {
        attribute.name.prefix == Some(prefix_symbol) && attribute.name.local == local_symbol
    }) {
        existing.value = escaped;
        return;
    }
    attributes.push(RawAttribute {
        name: RawName {
            prefix: Some(prefix_symbol),
            local: local_symbol,
            namespace: None,
        },
        value: escaped,
        quote: QuoteStyle::Double,
    });
}

/// Removes the unprefixed attribute `local`, if present.
pub(crate) fn remove_attribute(
    attributes: &mut Vec<RawAttribute>,
    interner: &Interner,
    local: &str,
) {
    let Some(symbol) = interner.get(local) else {
        return;
    };
    attributes
        .retain(|attribute| !(attribute.name.prefix.is_none() && attribute.name.local == symbol));
}

/// An `xmlns:prefix="uri"` declaration. A freshly authored VML part is its own root and must declare
/// the namespaces its elements use.
pub(crate) fn namespace_declaration(
    interner: &mut Interner,
    prefix: &str,
    uri: &str,
) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(interner.intern("xmlns")),
            local: interner.intern(prefix),
            namespace: None,
        },
        value: escape_attribute(uri).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// The conventional prefix for VML itself (`urn:schemas-microsoft-com:vml`).
pub(crate) const VML_PREFIX: &str = "v";
/// The conventional prefix for the Office drawing extensions
/// (`urn:schemas-microsoft-com:office:office`).
pub(crate) const OFFICE_PREFIX: &str = "o";
/// The conventional prefix for the PowerPoint drawing extensions
/// (`urn:schemas-microsoft-com:office:powerpoint`).
pub(crate) const POWERPOINT_PREFIX: &str = "p";
/// The conventional prefix for the spreadsheet drawing extensions
/// (`urn:schemas-microsoft-com:office:excel`).
pub(crate) const EXCEL_PREFIX: &str = "x";

/// A qualified name `prefix:local` resolving to `namespace`'s transitional URI.
pub(crate) fn qname(
    interner: &mut Interner,
    prefix: &str,
    namespace: SchemaNamespace,
    local: &str,
) -> RawName {
    RawName {
        prefix: Some(interner.intern(prefix)),
        local: interner.intern(local),
        namespace: Some(interner.intern(namespace.transitional)),
    }
}

/// A VML element `prefix:local` with `attributes` and `children` (self-closing when it has none).
pub(crate) fn element(
    interner: &mut Interner,
    prefix: &str,
    namespace: SchemaNamespace,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    let empty = children.is_empty();
    RawElement::new(
        qname(interner, prefix, namespace, local),
        attributes,
        children,
        empty,
    )
}

/// The decoded character data directly under `element` — every `Text`/`CData` child concatenated and
/// unescaped. A value that is not UTF-8, or carries a malformed entity, falls back to its raw text
/// rather than failing: this reads inputs the library only preserves.
pub(crate) fn element_text(nodes: &[RawNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            RawNode::Text(bytes) => {
                let raw = String::from_utf8_lossy(bytes);
                match unescape_text(&raw) {
                    Ok(decoded) => text.push_str(&decoded),
                    Err(_) => text.push_str(&raw),
                }
            }
            RawNode::CData(bytes) => text.push_str(&String::from_utf8_lossy(bytes)),
            _ => {}
        }
    }
    text
}

/// Generates the fidelity `FromXml`/`ToXml` impls for a leaf whose fields are exactly `name` /
/// `attributes` / `children` / `empty` — an element this crate addresses by its attributes while
/// re-emitting its subtree verbatim.
macro_rules! fidelity_leaf {
    ($ty:ty) => {
        impl ::mjx_ooxml_core::FromXml for $ty {
            fn from_xml(
                element: &::mjx_ooxml_core::RawElement,
                _interner: &::mjx_ooxml_core::Interner,
            ) -> ::core::result::Result<Self, ::mjx_ooxml_core::FromXmlError> {
                ::core::result::Result::Ok(Self {
                    element: ::core::clone::Clone::clone(element),
                })
            }
        }

        impl ::mjx_ooxml_core::ToXml for $ty {
            fn to_xml(
                &self,
                _interner: &mut ::mjx_ooxml_core::Interner,
            ) -> ::mjx_ooxml_core::RawElement {
                let mut out = ::core::clone::Clone::clone(&self.element);
                // Preserve the self-closing flag, but never contradict "self-closing ⇒ no children".
                out.empty = out.empty && out.children.is_empty();
                out
            }
        }

        impl $ty {
            /// The raw element behind this value, exactly as the part holds it.
            #[must_use]
            pub fn raw(&self) -> &::mjx_ooxml_core::RawElement {
                &self.element
            }

            /// The raw element behind this value, mutably. Editing it through this handle is how a
            /// caller reaches an attribute this crate does not name.
            pub fn raw_mut(&mut self) -> &mut ::mjx_ooxml_core::RawElement {
                &mut self.element
            }
        }
    };
}

pub(crate) use fidelity_leaf;
