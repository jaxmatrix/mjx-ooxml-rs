//! Shared low-level plumbing every other module in this crate is built from: the `m:` qualified-name
//! builder, namespace/child lookup by local name (both immutable and mutable), and the generic
//! `val`-attribute codec/element helpers the leaf value types (see [`crate::leaf`]) are built from.
//!
//! Mirrors `mjx-dml`'s own `build.rs` and `crates/mjx-dml/src/wordprocessing_drawing.rs`'s `wp_name`/
//! `is_wp`/`wp_child` family — the same shape, for the `m:` namespace instead of `a:`/`wp:`.

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Interner, QuoteStyle, RawAttribute, RawElement, RawName,
    RawNode,
};
use mjx_ooxml_types::namespaces::SHARED_MATH;

/// An `xmlns:m="…"` declaration binding the Office Math namespace — for a caller splicing a freshly
/// built `m:oMath`/`m:oMathPara` into a part that may not already bind it (a blank
/// `word/document.xml` binds only `w`/`r`; `blank.rs`'s own template). Harmless to add redundantly on
/// an equation nested somewhere that already binds `m:` — an extra `xmlns:m` on a descendant simply
/// rebinds the same URI to the same prefix, ordinary and valid XML, the same reasoning
/// `mjx_docx::document::drawing::namespace_declaration`'s own doc comment gives for `wp:`/`a:`/`pic:`.
pub(crate) fn namespace_declaration(interner: &mut Interner) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(interner.intern("xmlns")),
            local: interner.intern("m"),
            namespace: None,
        },
        value: SHARED_MATH.transitional.as_bytes().into(),
        quote: QuoteStyle::Double,
    }
}

/// Builds an `m:local` qualified name in the Office Math namespace.
pub(crate) fn m_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("m")),
        local: interner.intern(local),
        namespace: Some(interner.intern(SHARED_MATH.transitional)),
    }
}

/// Whether `name` is in the Office Math namespace, matching both its Strict and Transitional URIs.
pub(crate) fn is_m(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(SHARED_MATH.transitional) || namespace == SHARED_MATH.strict
}

/// The first `m:`-namespaced element in `children` named `local`.
pub(crate) fn m_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_m(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// Every `m:`-namespaced element in `children` named `local`, in document order.
pub(crate) fn m_children<'a>(
    children: &'a [RawNode],
    interner: &'a Interner,
    local: &'a str,
) -> impl Iterator<Item = &'a RawElement> {
    children.iter().filter_map(move |node| match node {
        RawNode::Element(child)
            if is_m(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// `shared-math.xsd` is `attributeFormDefault="qualified"` (confirmed directly off both the Strict
/// and Transitional editions) — the *only* other modeled schema besides `wml.xsd` with this shape.
/// Every locally-declared attribute (`val`, `alnAt`, …) is therefore written **namespace-qualified**
/// by a real producer, exactly as `w:type`/`w:font` are in WordprocessingML: `<m:chr m:val="…"/>`,
/// never a bare `val`. [`read_val`]/[`val_element`] read and write that `m:`-prefixed spelling.
const VAL_ATTRIBUTE_PREFIX: Option<&str> = Some("m");

/// Reads `element`'s `m:val` attribute through codec `C` — the shape every leaf `CT_*` in this schema
/// shares (`CT_OnOff`, `CT_Shp`, `CT_Integer255`, …). `Ok(None)` means the attribute is absent
/// (legal for every one of these except the four the schema marks `use="required"`, which their own
/// accessors treat as malformed-if-absent, matching every other type in this project). See
/// [`VAL_ATTRIBUTE_PREFIX`] for why the attribute is qualified.
///
/// # Errors
/// [`AttributeError`] if the attribute is present but the codec rejects its value.
pub(crate) fn read_val<'a, C: AttributeCodec>(
    element: &'a RawElement,
    interner: &Interner,
) -> Result<Option<C::Value<'a>>, AttributeError> {
    mjx_xml::attribute::read::<C>(
        &element.attributes,
        interner,
        VAL_ATTRIBUTE_PREFIX,
        "val",
        "m:val",
    )
}

/// Builds a self-closing `<m:{local} m:val="{value}"/>` through codec `C`.
pub(crate) fn val_element<C: AttributeCodec>(
    interner: &mut Interner,
    local: &str,
    value: C::Input<'_>,
) -> RawElement {
    let mut attributes: Vec<RawAttribute> = Vec::new();
    mjx_xml::attribute::write::<C>(
        &mut attributes,
        interner,
        VAL_ATTRIBUTE_PREFIX,
        "val",
        Some(value),
    );
    RawElement::new(m_name(interner, local), attributes, Vec::new(), true)
}

/// Reads the `val` attribute of the first `m:{local}` child of `children`, or `None` if there is no
/// such child or the attribute is absent/invalid — the read-never-fails leniency every optional
/// leaf accessor in this crate applies (an unreadable optional value is simply not there, exactly as
/// `crate::geometry::Transform2D::read` treats a malformed `a:off`).
pub(crate) fn read_val_child<'a, C: AttributeCodec>(
    children: &'a [RawNode],
    interner: &'a Interner,
    local: &str,
) -> Option<C::Value<'a>> {
    m_child(children, interner, local).and_then(|el| read_val::<C>(el, interner).ok().flatten())
}

/// Generates the fidelity `FromXml`/`ToXml` impls for a wrapper `struct` whose fields are exactly
/// `name` / `attributes` / `children` / `empty`: a type that models an element by name and preserves
/// its attributes, children and self-closing flag verbatim, exposing typed accessors *over* that
/// preserved state rather than decomposing it into typed fields. Every structural (non-leaf) type in
/// this crate is built this way — see the crate's own module doc comment for why: `shared-math.xsd`
/// nests `wml`-typed content (`w:rPr` via `CT_CtrlPr`) that `mjx-omml` cannot model, and preserving
/// every child verbatim by default, with typed *reads* layered on top, is what lets an untyped nested
/// `w:rPr` and a typed nested `m:f` share one mechanism instead of two.
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
