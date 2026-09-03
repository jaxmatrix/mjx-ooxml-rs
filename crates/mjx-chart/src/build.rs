//! Small shared readers and builders for chart (`c:`-prefixed) elements, and the fidelity macro for
//! the two text-bearing leaves (`c:v`, `c:f`).
//!
//! These mirror `mjx-dml`'s `build.rs`, which keeps its equivalents `pub(crate)` — a sibling crate
//! cannot borrow them, so the handful this tier needs are re-stated here. The readers back the C1/C2
//! read model; the builders (added in C3) construct the `c:pt` / `c:v` a cache edit rewrites.

use mjx_ooxml_core::{Interner, QuoteStyle, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::child_order::ChildOrder;
use mjx_ooxml_types::namespaces::{DML_CHART, DML_MAIN};
use mjx_xml::text::{escape_attribute, escape_text};

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

// -------------------------------------------------------------------------------------------------
// Builders (C3 — editing caches)
// -------------------------------------------------------------------------------------------------

/// Builds a chart qualified name `c:local` — literal prefix `c` plus the resolved transitional
/// namespace, so a built element serializes as `c:local` and reads back by `(DML_CHART, local)`.
pub(crate) fn chart_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("c")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_CHART.transitional)),
    }
}

/// Builds an unprefixed, double-quoted attribute `local="value"`, escaping `value` for an attribute.
pub(crate) fn chart_attr(interner: &mut Interner, local: &str, value: &str) -> RawAttribute {
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

/// Builds a `c:`-prefixed element with `attributes` and `children` (self-closing when it has no
/// children).
pub(crate) fn chart_element(
    interner: &mut Interner,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    let empty = children.is_empty();
    RawElement::new(chart_name(interner, local), attributes, children, empty)
}

/// Builds a text-bearing `c:local` leaf (`c:v`, `c:f`) carrying `text` as an escaped `Text` child.
/// Empty text yields a self-closing element, matching how the fidelity reader would present it.
pub(crate) fn chart_text_leaf(interner: &mut Interner, local: &str, text: &str) -> RawElement {
    let escaped = escape_text(text);
    let children = if escaped.is_empty() {
        Vec::new()
    } else {
        vec![RawNode::Text(escaped.as_bytes().into())]
    };
    chart_element(interner, local, Vec::new(), children)
}

/// Builds a `<c:local val="value"/>` scalar leaf — the shape of the chart's many single-attribute
/// children (`c:barDir`, `c:grouping`, `c:ptCount`, `c:axId`, `c:delete`, `c:orientation`, …).
pub(crate) fn chart_val_leaf(interner: &mut Interner, local: &str, value: &str) -> RawElement {
    let attr = chart_attr(interner, "val", value);
    chart_element(interner, local, vec![attr], Vec::new())
}

/// Builds a numeric data source holding `values` inline:
/// `<c:local><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="n"/><c:pt idx="i">
/// <c:v>…</c:v></c:pt>…</c:numLit></c:local>`.
///
/// This is the shape of `CT_NumDataSource` — what `c:val`, `c:yVal`, and an error bar's `c:plus`
/// and `c:minus` all are. A literal is used rather than a `c:numRef` because these numbers have no
/// workbook cells behind them: a reference whose formula named none would be a dangling claim. A
/// non-finite value has no XML spelling, so it is written as `0` to keep the point count and the
/// indices aligned with what the caller passed.
pub(crate) fn number_literal_source(
    interner: &mut Interner,
    local: &str,
    values: &[f64],
) -> RawElement {
    let mut children = vec![
        RawNode::Element(chart_text_leaf(interner, "formatCode", "General")),
        RawNode::Element(chart_val_leaf(
            interner,
            "ptCount",
            &values.len().to_string(),
        )),
    ];
    for (index, &value) in values.iter().enumerate() {
        let idx = chart_attr(interner, "idx", &index.to_string());
        let text = f64_wire(value).unwrap_or_else(|| "0".to_owned());
        let v = chart_text_leaf(interner, "v", &text);
        children.push(RawNode::Element(chart_element(
            interner,
            "pt",
            vec![idx],
            vec![RawNode::Element(v)],
        )));
    }
    let literal = chart_element(interner, "numLit", Vec::new(), children);
    chart_element(interner, local, Vec::new(), vec![RawNode::Element(literal)])
}

/// Builds a DrawingML qualified name `a:local` — the `a:` half of a chart part, used by the
/// shape properties (`c:spPr`) and rich text (`c:tx > c:rich`) a chart embeds.
pub(crate) fn dml_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("a")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_MAIN.transitional)),
    }
}

/// Builds an `a:`-prefixed element with `attributes` and `children` (self-closing when empty).
pub(crate) fn dml_element(
    interner: &mut Interner,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    let empty = children.is_empty();
    RawElement::new(dml_name(interner, local), attributes, children, empty)
}

/// Builds a text-bearing `a:local` leaf (`a:t`) carrying `text` as an escaped `Text` child.
pub(crate) fn dml_text_leaf(interner: &mut Interner, local: &str, text: &str) -> RawElement {
    let escaped = escape_text(text);
    let children = if escaped.is_empty() {
        Vec::new()
    } else {
        vec![RawNode::Text(escaped.as_bytes().into())]
    };
    dml_element(interner, local, Vec::new(), children)
}

/// Whether `name` is in the DrawingML main namespace (accepting both URIs), regardless of prefix.
pub(crate) fn is_dml(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_MAIN.transitional) || namespace == DML_MAIN.strict
}

/// The index at which a child named `local` belongs among `existing`, according to `order` — the
/// generated `xsd:sequence` of the complex type being written.
///
/// `existing` yields each present child's local name in document order, or `None` for a node the
/// order does not name (text, a comment, a foreign element) — such a node never moves the insertion
/// point, so unmodelled markup keeps its place. A `local` the order does not name goes at the end.
///
/// This is the raw-node-free form of [`ChildOrder::insert_index`], for the chart models whose
/// children are typed enum values rather than [`RawNode`]s.
pub(crate) fn insert_position<'a>(
    order: &ChildOrder,
    existing: impl Iterator<Item = Option<&'a str>>,
    local: &str,
) -> usize {
    order.insert_index_of_names(
        existing.map(|name| name.and_then(|name| order.rank_of(None, name))),
        local,
    )
}

/// Builds an `xmlns:prefix="uri"` namespace declaration attribute. A freshly authored chart part is
/// its own root — unlike a subtree spliced into a slide, which inherits the slide's declarations — so
/// it must declare the namespaces its `c:`/`a:` elements use.
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

/// Sets an unprefixed attribute `local="value"` on `attributes` — rewriting the existing one in
/// place (preserving order) or appending it.
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
        attributes.push(chart_attr(interner, local, value));
    }
}

/// Formats a finite `f64` as its wire string — Rust's shortest round-trip representation, the exact
/// inverse of the read side's parse (`19.2` ↔ `"19.2"`, `5.0` → `"5"`). A non-finite value
/// (`NaN`/`±inf`) has no valid XML spelling, so it yields `None` and the caller skips it.
pub(crate) fn f64_wire(value: f64) -> Option<String> {
    value.is_finite().then(|| value.to_string())
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
                ::mjx_ooxml_core::RawElement::rebuilt(
                    self.name,
                    self.attributes.clone(),
                    children,
                    empty,
                )
            }
        }
    };
}

pub(crate) use fidelity_element_impls;
