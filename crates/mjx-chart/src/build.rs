//! Small shared readers for chart (`c:`-prefixed) elements, and the fidelity macro for the two
//! text-bearing leaves (`c:v`, `c:f`).
//!
//! These mirror `mjx-dml`'s `build.rs`, which keeps its equivalents `pub(crate)` — a sibling crate
//! cannot borrow them, so the handful this tier needs are re-stated here. C1 is **read-only**: there
//! are no element/attribute *builders*, only readers over the parsed [`RawElement`] tree and the
//! `fidelity_element_impls!` macro that clones a leaf's subtree verbatim.

use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::namespaces::DML_CHART;

/// Whether `name` is in the chart namespace (accepting both its transitional and strict URIs),
/// regardless of prefix — the same both-URI match the derive uses for a typed child.
pub(crate) fn is_chart(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_CHART.transitional) || namespace == DML_CHART.strict
}

/// The `@attr` value of the first element in `nodes` named `(DML_CHART, local)`, matching on the
/// resolved namespace (both URIs), never the prefix — for reading a scalar off a child this tier
/// keeps in the `Raw` bucket (`c:barDir@val`, `c:idx@val`) without promoting it to a typed variant.
pub(crate) fn raw_child_attr<'a>(
    nodes: impl Iterator<Item = &'a RawNode>,
    interner: &Interner,
    local: &str,
    attr: &str,
) -> Option<&'a str> {
    for node in nodes {
        if let RawNode::Element(child) = node {
            if is_chart(&child.name, interner) && interner.resolve(child.name.local) == local {
                return attr_str(&child.attributes, interner, attr);
            }
        }
    }
    None
}

/// The UTF-8 value of the first unprefixed attribute named `local`, or `None` if absent (or not
/// UTF-8). Chart attribute values this tier reads (`@val`, `@idx`) contain no XML-special characters.
pub(crate) fn attr_str<'a>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    local: &str,
) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| {
            attribute.name.prefix.is_none() && interner.resolve(attribute.name.local) == local
        })
        .and_then(|attribute| std::str::from_utf8(&attribute.value).ok())
}

/// Reads an unsigned-integer attribute (`xsd:unsignedInt`, e.g. `c:pt@idx`, `c:idx@val`) as a `u32`.
pub(crate) fn attr_u32(
    attributes: &[RawAttribute],
    interner: &Interner,
    local: &str,
) -> Option<u32> {
    attr_str(attributes, interner, local).and_then(|s| s.trim().parse().ok())
}

/// The decoded text content of an element's `nodes` — every `Text`/`CData` child concatenated and
/// unescaped.
///
/// This is a **read** accessor only; round-trip fidelity never flows through it (a leaf re-emits by
/// cloning its raw subtree, see [`fidelity_element_impls!`]). Unescaping resolves entities so a
/// label like `R&amp;D` reads back as `R&D`; a value that is not valid UTF-8 or carries a malformed
/// entity falls back to the raw bytes rather than failing an accessor.
pub(crate) fn element_text(nodes: &[RawNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        if let RawNode::Text(bytes) | RawNode::CData(bytes) = node {
            let raw = String::from_utf8_lossy(bytes);
            match mjx_xml::text::unescape_text(&raw) {
                Ok(decoded) => text.push_str(&decoded),
                Err(_) => text.push_str(&raw),
            }
        }
    }
    text
}

/// Generates the fidelity `FromXml`/`ToXml` impls for a wrapper `struct` whose fields are exactly
/// `name` / `attributes` / `children` / `empty` — a leaf that models an element by name and
/// preserves its attributes, children and self-closing flag verbatim (`c:v`, `c:f`). Copied from
/// `mjx-dml`'s identical macro, which is `pub(crate)` there.
macro_rules! fidelity_element_impls {
    ($ty:ty) => {
        impl ::mjx_ooxml_core::FromXml for $ty {
            fn from_xml(
                element: &::mjx_ooxml_core::RawElement,
                _interner: &::mjx_ooxml_core::Interner,
            ) -> Result<Self, ::mjx_ooxml_core::FromXmlError> {
                Ok(Self {
                    name: element.name,
                    attributes: element.attributes.clone(),
                    children: element.children.clone(),
                    empty: element.empty,
                })
            }
        }

        impl ::mjx_ooxml_core::ToXml for $ty {
            fn to_xml(
                &self,
                _interner: &mut ::mjx_ooxml_core::Interner,
            ) -> ::mjx_ooxml_core::RawElement {
                let children = self.children.clone();
                // Preserve the self-closing flag, but never contradict "self-closing ⇒ no children".
                let empty = self.empty && children.is_empty();
                ::mjx_ooxml_core::RawElement {
                    name: self.name,
                    attributes: self.attributes.clone(),
                    children,
                    empty,
                }
            }
        }
    };
}

pub(crate) use fidelity_element_impls;
