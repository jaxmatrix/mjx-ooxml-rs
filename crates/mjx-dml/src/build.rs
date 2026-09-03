//! Small shared builders and finders for DrawingML **elements** — the `a:`-prefixed names this
//! crate constructs, and the by-name child searches its accessors are made of.
//!
//! There is nothing here about attributes. There used to be: six hand-written readers and five
//! writers, called 193 times across the crate. Every one of those call sites is now a declaration
//! (`#[xml(attribute(..))]`) whose accessor is a single call to [`mjx_xml::attribute::read`] or
//! [`mjx_xml::attribute::write`], so there is exactly one path from a wire attribute to a typed
//! value and exactly one back. A helper family with two callers left is the half-migrated family
//! MJXOFF-89 exists to warn about, so the family is gone rather than reduced.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::DML_MAIN;

use crate::color::Color;
use crate::fill::Fill;

/// Builds a DrawingML qualified name `a:local` — literal prefix `a` plus the resolved transitional
/// namespace, so a built element serializes as `a:local` and reads back by `(DML_MAIN, local)`.
pub(crate) fn dml_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("a")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_MAIN.transitional)),
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
    RawElement::new(dml_name(interner, local), attributes, children, empty)
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

/// The first element in `children` named `(DML_MAIN, local)`, mutably — the writer's counterpart to
/// [`dml_child`], for the models that edit a child **in place** (setting attributes on an existing
/// `a:off`) rather than rebuilding it, so the child's unmodeled attributes survive the edit.
pub(crate) fn dml_child_mut<'a>(
    children: &'a mut [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a mut RawElement> {
    children.iter_mut().find_map(|node| match node {
        RawNode::Element(child)
            if is_dml(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
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

/// The first `EG_FillProperties` child of `children` (any of the six fill element names), read as a
/// [`RawElement`] — the stroke fill of a line, or the overlay fill of a `fillOverlay` effect. Takes a
/// node slice so an accessor can search a wrapper's own `children` without rebuilding a [`RawElement`].
pub(crate) fn first_fill_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_dml(&child.name, interner)
                && Fill::is_fill_local(interner.resolve(child.name.local)) =>
        {
            Some(child)
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
