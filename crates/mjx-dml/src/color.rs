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

/// An interner-free description of a [`Color`] — the friendly value an interner-less caller reads and
/// writes (see [`Color::spec`] / [`Color::from_spec`]). The two first-class kinds carry their value;
/// any other `EG_ColorChoice` kind is preserved as [`Other`](ColorSpec::Other) with its raw `val`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorSpec {
    /// `a:srgbClr` — an sRGB hex value like `"FF0000"` (no leading `#`).
    Srgb(String),
    /// `a:schemeClr` — a theme color.
    Scheme(SchemeColor),
    /// Any other color kind (`a:sysClr`, `a:prstClr`, `a:scrgbClr`, `a:hslClr`), carrying its kind
    /// and raw `val` (if any). Also represents an absent color ([`Unknown`](ColorKind::Unknown), no
    /// value) so a color-less `a:solidFill` round-trips.
    Other {
        /// The color-choice element kind.
        kind: ColorKind,
        /// The raw `val` attribute, if present.
        value: Option<String>,
    },
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

    /// The `EG_ColorChoice` element local name for a [`ColorKind`], or `None` for
    /// [`Unknown`](ColorKind::Unknown).
    fn kind_local(kind: ColorKind) -> Option<&'static str> {
        Some(match kind {
            ColorKind::Srgb => "srgbClr",
            ColorKind::ScRgb => "scrgbClr",
            ColorKind::Hsl => "hslClr",
            ColorKind::System => "sysClr",
            ColorKind::Scheme => "schemeClr",
            ColorKind::Preset => "prstClr",
            ColorKind::Unknown => return None,
        })
    }

    /// Builds a color of `kind` carrying an optional `val` attribute — the generic path behind
    /// [`from_spec`](Self::from_spec) for kinds other than sRGB / scheme. Returns `None` for
    /// [`Unknown`](ColorKind::Unknown), which names no element.
    #[must_use]
    pub fn of_kind(interner: &mut Interner, kind: ColorKind, value: Option<&str>) -> Option<Self> {
        let local = Self::kind_local(kind)?;
        let attributes = value
            .map(|value| vec![dml_attr(interner, "val", value)])
            .unwrap_or_default();
        Some(Self {
            name: dml_name(interner, local),
            attributes,
            children: Vec::new(),
            empty: true,
        })
    }

    /// The six `EG_ColorChoice` element local names (`a:srgbClr`, `a:schemeClr`, …), in schema order.
    pub(crate) const CHOICE_LOCALS: [&'static str; 6] = [
        "srgbClr",
        "schemeClr",
        "sysClr",
        "scrgbClr",
        "hslClr",
        "prstClr",
    ];

    /// Whether `local` names one of the six `EG_ColorChoice` elements.
    #[must_use]
    pub(crate) fn is_choice_local(local: &str) -> bool {
        Self::CHOICE_LOCALS.contains(&local)
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

    /// This color as an interner-free [`ColorSpec`] — sRGB and scheme colors resolve to their first-
    /// class variants; any other kind becomes [`ColorSpec::Other`] carrying its `val`. Transform
    /// children are not represented (the spec is a value description, not a fidelity view).
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> ColorSpec {
        match self.kind(interner) {
            ColorKind::Srgb => ColorSpec::Srgb(self.value(interner).unwrap_or_default().to_owned()),
            ColorKind::Scheme => match self.scheme_color(interner) {
                Some(scheme) => ColorSpec::Scheme(scheme),
                None => ColorSpec::Other {
                    kind: ColorKind::Scheme,
                    value: self.value(interner).map(str::to_owned),
                },
            },
            kind => ColorSpec::Other {
                kind,
                value: self.value(interner).map(str::to_owned),
            },
        }
    }

    /// Builds a color from an interner-free [`ColorSpec`]. Returns `None` only for an
    /// [`Other`](ColorSpec::Other) spec whose kind names no element ([`Unknown`](ColorKind::Unknown)).
    #[must_use]
    pub fn from_spec(interner: &mut Interner, spec: &ColorSpec) -> Option<Self> {
        match spec {
            ColorSpec::Srgb(hex) => Some(Self::srgb(interner, hex)),
            ColorSpec::Scheme(scheme) => Some(Self::scheme(interner, *scheme)),
            ColorSpec::Other { kind, value } => Self::of_kind(interner, *kind, value.as_deref()),
        }
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
