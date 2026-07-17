//! Small shared builders/readers for DrawingML elements — used by the geometry and color/fill models
//! to construct `a:`-prefixed elements and read/write their attributes, keeping the byte-level fidelity
//! rules in one place.

use mjx_ooxml_core::{FromXml, Interner, QuoteStyle, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::DML_MAIN;
use mjx_xml::text::escape_attribute;

use crate::color::Color;
use crate::geometry::{Angle, Fraction};

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

/// Builds a prefixed, double-quoted attribute `prefix:local="value"` with the namespace left
/// unresolved (only the literal prefix is kept) — mirroring how the fidelity reader stores a
/// prefixed attribute such as `r:embed`, so a built value round-trips identically. The `prefix`'s
/// binding to a namespace is the containing part's responsibility (declared on its root element).
pub(crate) fn prefixed_attr(
    interner: &mut Interner,
    prefix: &str,
    local: &str,
    value: &str,
) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(interner.intern(prefix)),
            local: interner.intern(local),
            namespace: None,
        },
        value: escape_attribute(value).as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// Builds an `a:`-prefixed DrawingML element with `attributes` and `children` (self-closing when it
/// has no children), for the fill builders that assemble small nested element trees.
pub(crate) fn dml_element(
    interner: &mut Interner,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    let empty = children.is_empty();
    RawElement {
        name: dml_name(interner, local),
        attributes,
        children,
        empty,
    }
}

/// Whether `name` is in the DrawingML-main namespace (accepting both its transitional and strict
/// URIs), regardless of prefix.
pub(crate) fn is_dml(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_MAIN.transitional) || namespace == DML_MAIN.strict
}

/// The first element in `children` named `(DML_MAIN, local)` — matching on the resolved namespace
/// (both URIs), never the prefix. Takes a node slice so the fill accessors can search a wrapper's
/// own `children` without rebuilding a [`RawElement`].
pub(crate) fn dml_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_dml(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
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

/// The UTF-8 value of the first attribute whose **local** name is `local`, regardless of prefix (or
/// `None` if absent / not UTF-8). Used for the relationship attributes on `a:blip` (`r:embed` /
/// `r:link`), whose prefix the fidelity reader leaves unresolved and whose locals are unambiguous.
pub(crate) fn attr_by_local<'a>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    local: &str,
) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| interner.resolve(attribute.name.local) == local)
        .and_then(|attribute| std::str::from_utf8(&attribute.value).ok())
}

/// Parses a DrawingML percentage (`ST_Percentage` family) to a [`Fraction`]: the integer form
/// (`50000` = 50%, native/100000) or an explicit-percent form (`50%`). `1.0` is 100%.
pub(crate) fn parse_percentage(s: &str) -> Option<Fraction> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix('%') {
        stripped
            .trim()
            .parse::<f64>()
            .ok()
            .map(|value| Fraction::from_ratio(value / 100.0))
    } else {
        s.parse::<f64>()
            .ok()
            .map(|value| Fraction::from_ratio(value / 100_000.0))
    }
}

/// Parses a DrawingML angle attribute (`ST_Angle` family, 60000ths of a degree) to an [`Angle`].
pub(crate) fn parse_angle(s: &str) -> Option<Angle> {
    s.trim()
        .parse::<f64>()
        .ok()
        .map(|value| Angle::from_degrees(value / 60_000.0))
}

/// The first `EG_ColorChoice` child of `element`, read as a [`Color`] — used wherever a wrapper
/// element holds one color (a gradient `gs`, a `fgClr`/`bgClr`, a `clrScheme` slot).
pub(crate) fn first_color_child(element: &RawElement, interner: &Interner) -> Option<Color> {
    element.children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_dml(&child.name, interner)
                && Color::is_choice_local(interner.resolve(child.name.local)) =>
        {
            Color::from_xml(child, interner).ok()
        }
        _ => None,
    })
}

/// Generates the fidelity `FromXml`/`ToXml` impls for a wrapper `struct` whose fields are exactly
/// `name` / `attributes` / `children` / `empty` — a type that models an element by name and preserves
/// its attributes, children, and self-closing flag verbatim (like `color::Color`). Each fill kind is
/// such a wrapper, so this keeps their identical (de)serialization in one place.
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
