//! DrawingML color: the `EG_ColorChoice` elements (`a:srgbClr`, `a:schemeClr`, `a:sysClr`,
//! `a:prstClr`, `a:scrgbClr`, `a:hslClr`).

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use crate::build::{attr_str, dml_attr, dml_name};

pub use mjx_ooxml_types::drawingml::SchemeColor;

/// Which `EG_ColorChoice` element a [`Color`] is (its element name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorKind {
    /// `a:srgbClr` — an sRGB hex color.
    Srgb,
    /// `a:scrgbClr` — a linear-RGB percentage color.
    ScRgb,
    /// `a:hslClr` — an HSL color.
    Hsl,
    /// `a:sysClr` — a system color.
    System,
    /// `a:schemeClr` — a theme (scheme) color reference.
    Scheme,
    /// `a:prstClr` — a preset (named) color.
    Preset,
    /// An unrecognized color element.
    Unknown,
}

/// A DrawingML color — one `EG_ColorChoice` element with its value attributes and any color-transform
/// children (`a:lumMod`, `a:alpha`, …).
///
/// The element *name* is the color kind; its children are the transforms. This is a **fidelity view**:
/// name, attributes, transform children, and the self-closing flag are preserved verbatim, while
/// [`kind`](Self::kind) / [`hex`](Self::hex) / [`scheme_color`](Self::scheme_color) expose the common
/// cases. Its [`FromXml`]/[`ToXml`] impls are hand-written because the element name is the discriminant,
/// which the derive does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl Color {
    /// Builds an sRGB color `<a:srgbClr val="{hex}"/>` (e.g. `hex = "FF0000"`).
    #[must_use]
    pub fn srgb(interner: &mut Interner, hex: &str) -> Self {
        Self {
            name: dml_name(interner, "srgbClr"),
            attributes: vec![dml_attr(interner, "val", hex)],
            children: Vec::new(),
            empty: true,
        }
    }

    /// Builds a theme color reference `<a:schemeClr val="{scheme}"/>`.
    #[must_use]
    pub fn scheme(interner: &mut Interner, scheme: SchemeColor) -> Self {
        Self {
            name: dml_name(interner, "schemeClr"),
            attributes: vec![dml_attr(interner, "val", scheme.to_wire())],
            children: Vec::new(),
            empty: true,
        }
    }

    /// Which color-choice element this is.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> ColorKind {
        match interner.resolve(self.name.local) {
            "srgbClr" => ColorKind::Srgb,
            "scrgbClr" => ColorKind::ScRgb,
            "hslClr" => ColorKind::Hsl,
            "sysClr" => ColorKind::System,
            "schemeClr" => ColorKind::Scheme,
            "prstClr" => ColorKind::Preset,
            _ => ColorKind::Unknown,
        }
    }

    /// The sRGB hex value (the `val` of an `a:srgbClr`), or `None` if this is not an sRGB color.
    #[must_use]
    pub fn hex(&self, interner: &Interner) -> Option<&str> {
        if self.kind(interner) == ColorKind::Srgb {
            attr_str(&self.attributes, interner, "val")
        } else {
            None
        }
    }

    /// The theme color (the `val` of an `a:schemeClr`), or `None` if this is not a scheme color or its
    /// token is unrecognized.
    #[must_use]
    pub fn scheme_color(&self, interner: &Interner) -> Option<SchemeColor> {
        if self.kind(interner) != ColorKind::Scheme {
            return None;
        }
        attr_str(&self.attributes, interner, "val").and_then(SchemeColor::from_wire)
    }

    /// The raw `val` attribute value (for any color kind), or `None` if absent.
    #[must_use]
    pub fn value(&self, interner: &Interner) -> Option<&str> {
        attr_str(&self.attributes, interner, "val")
    }

    /// The color's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// The color's transform children (`a:lumMod`, `a:alpha`, …), preserved verbatim.
    #[must_use]
    pub fn transforms(&self) -> &[RawNode] {
        &self.children
    }
}

impl FromXml for Color {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Color {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        // Preserve the self-closing flag, but never contradict "self-closing ⇒ no children".
        let empty = self.empty && children.is_empty();
        RawElement {
            name: self.name,
            attributes: self.attributes.clone(),
            children,
            empty,
        }
    }
}
