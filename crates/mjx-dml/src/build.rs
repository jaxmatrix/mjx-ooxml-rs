//! Small shared builders/readers for DrawingML elements — used by the geometry and color/fill models
//! to construct `a:`-prefixed elements and read/write their attributes, keeping the byte-level fidelity
//! rules in one place.

use mjx_ooxml_core::{Interner, QuoteStyle, RawAttribute, RawName};
use mjx_ooxml_types::namespaces::DML_MAIN;
use mjx_xml::text::escape_attribute;

/// Builds a DrawingML qualified name `a:local` — literal prefix `a` plus the resolved transitional
/// namespace, so a built element serializes as `a:local` and reads back by `(DML_MAIN, local)`.
pub(crate) fn dml_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("a")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_MAIN.transitional)),
    }
}

/// Builds an unprefixed, double-quoted attribute `local="value"`, escaping `value` for an attribute.
pub(crate) fn dml_attr(interner: &mut Interner, local: &str, value: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: None,
            local: interner.intern(local),
            namespace: None,
        },
        value: escape_attribute(value).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// Sets an unprefixed attribute `local="value"` on `attributes` — rewriting the existing one in
/// place (preserving order) or appending it — with `value` escaped for an attribute.
pub(crate) fn set_attr(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    local: &str,
    value: &str,
) {
    let sym = interner.intern(local);
    if let Some(attribute) = attributes
        .iter_mut()
        .find(|attribute| attribute.name.prefix.is_none() && attribute.name.local == sym)
    {
        attribute.value = escape_attribute(value).as_bytes().into();
    } else {
        attributes.push(dml_attr(interner, local, value));
    }
}

/// The UTF-8 value of the first unprefixed attribute named `local`, or `None` if absent (or the bytes
/// are not UTF-8). The value is returned verbatim — the attribute values these models read (guide
/// names/formulas, color `val`s) contain no XML-special characters in practice.
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
