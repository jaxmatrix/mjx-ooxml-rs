//! DrawingML color: the `EG_ColorChoice` elements (`a:srgbClr`, `a:schemeClr`, `a:sysClr`,
//! `a:prstClr`, `a:scrgbClr`, `a:hslClr`).

use std::borrow::Cow;

use mjx_ooxml_core::{AttributeCodec, Interner, RawAttribute, RawName, RawNode, Text};

use crate::build::{dml_element, dml_name, fidelity_element_impls, is_dml};
use crate::codec::{Percentage, SixtyThousandthsOfADegree};
use crate::color_transform::{ColorTransform, ColorTransformKind, ColorTransformValue};
use crate::geometry::Fraction;

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
///
/// # Colour transforms
///
/// A colour's children are its `EG_ColorTransform` sequence, and
/// [`Transformed`](ColorSpec::Transformed) is where they live. **Do not build that variant by
/// hand**: the builders — [`with_transform`](Self::with_transform) and the six named conveniences
/// beside it — are the way to attach one, and they maintain the invariant this type relies on, that
/// a `Transformed`'s `base` is never itself a `Transformed`:
///
/// ```
/// use mjx_dml::{ColorSpec, ColorTransform, ColorTransformKind, Fraction, SchemeColor};
///
/// // What PowerPoint writes for "Accent 1, Lighter 40 %".
/// let lighter = ColorSpec::Scheme(SchemeColor::Accent1)
///     .with_luminance_modulation(Fraction::from_ratio(0.6))
///     .with_luminance_offset(Fraction::from_ratio(0.4));
///
/// assert_eq!(lighter.base(), &ColorSpec::Scheme(SchemeColor::Accent1));
/// assert_eq!(lighter.transforms().len(), 2);
///
/// // Anything else in the group goes through the generic builder.
/// let gamma = ColorSpec::Srgb("1F3864".into()).with_transform(ColorTransform::InverseGamma);
/// assert_eq!(gamma.transforms(), &[ColorTransform::InverseGamma]);
/// assert_eq!(gamma.transforms()[0].kind(), ColorTransformKind::InverseGamma);
/// ```
///
/// **Order is part of the markup**, so every builder *appends*: `.with_tint(a).with_shade(b)` and
/// `.with_shade(b).with_tint(a)` are different colours, exactly as the two markups are.
#[derive(Debug, Clone, PartialEq)]
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
    /// A colour carrying `EG_ColorTransform` children, in document order.
    ///
    /// Build it with [`with_transform`](ColorSpec::with_transform) rather than by hand: the
    /// builders keep `base` as one of the three variants above and **never** another
    /// `Transformed`, and never leave `transforms` empty.
    ///
    /// The fields are public, so a caller *can* build a nest, and nothing here panics on one:
    /// [`base`](ColorSpec::base) walks to the colour underneath and [`Color::from_spec`] flattens
    /// the whole chain, innermost transforms first. Only [`transforms`](ColorSpec::transforms) sees
    /// one level, which is every level for a spec the builders made.
    Transformed {
        /// The colour the transforms apply to.
        base: Box<ColorSpec>,
        /// The transforms, in the order they are written and applied.
        transforms: Vec<ColorTransform>,
    },
}

impl ColorSpec {
    /// The colour without its transforms — itself, unless this is
    /// [`Transformed`](Self::Transformed).
    ///
    /// This is what a reader asking *what colour is this* wants: a scheme colour with a `lumMod` on
    /// it is still a scheme colour, and the schema agrees — a transform is a child of a colour, not
    /// a different kind of colour.
    ///
    /// The loop below runs at most once for a spec the builders produced, and terminates for any
    /// spec at all: the variant's fields are public, so a caller *can* hand-build a nest, and
    /// answering one is cheaper than trusting it not to exist.
    #[must_use]
    pub fn base(&self) -> &Self {
        let mut base = self;
        while let Self::Transformed { base: inner, .. } = base {
            base = inner;
        }
        base
    }

    /// The colour's `EG_ColorTransform` children, in document order — empty for a colour that has
    /// none.
    ///
    /// This is every transform the colour carries, because
    /// [`with_transform`](Self::with_transform) keeps a [`Transformed`](Self::Transformed) exactly
    /// one level deep. A caller who hand-builds a nest sees only the outermost level here;
    /// [`Color::from_spec`] still writes all of them, innermost first.
    #[must_use]
    pub fn transforms(&self) -> &[ColorTransform] {
        match self {
            Self::Transformed { transforms, .. } => transforms,
            _ => &[],
        }
    }

    /// This colour with `transform` **appended** to its transform sequence.
    ///
    /// The generic builder: every one of the twenty-eight members of `EG_ColorTransform` is
    /// reachable through it, including the ones with no named convenience. It appends rather than
    /// merges because the sequence is ordered and re-ordering it changes the colour.
    #[must_use]
    pub fn with_transform(self, transform: ColorTransform) -> Self {
        match self {
            Self::Transformed {
                base,
                mut transforms,
            } => {
                transforms.push(transform);
                Self::Transformed { base, transforms }
            }
            plain => Self::Transformed {
                base: Box::new(plain),
                transforms: vec![transform],
            },
        }
    }

    /// This colour with an `a:tint` appended — lightened toward white by `amount` (`1.0` is 100 %).
    #[must_use]
    pub fn with_tint(self, amount: Fraction) -> Self {
        self.with_transform(ColorTransform::Tint(amount))
    }

    /// This colour with an `a:shade` appended — darkened toward black by `amount`.
    #[must_use]
    pub fn with_shade(self, amount: Fraction) -> Self {
        self.with_transform(ColorTransform::Shade(amount))
    }

    /// This colour with an `a:alpha` appended — its opacity set to `amount` (`1.0` is opaque).
    #[must_use]
    pub fn with_alpha(self, amount: Fraction) -> Self {
        self.with_transform(ColorTransform::Alpha(amount))
    }

    /// This colour with an `a:lumMod` appended — its luminance multiplied by `amount`.
    #[must_use]
    pub fn with_luminance_modulation(self, amount: Fraction) -> Self {
        self.with_transform(ColorTransform::LuminanceModulation(amount))
    }

    /// This colour with an `a:lumOff` appended — its luminance shifted by `amount`.
    #[must_use]
    pub fn with_luminance_offset(self, amount: Fraction) -> Self {
        self.with_transform(ColorTransform::LuminanceOffset(amount))
    }

    /// This colour with an `a:satMod` appended — its saturation multiplied by `amount`.
    #[must_use]
    pub fn with_saturation_modulation(self, amount: Fraction) -> Self {
        self.with_transform(ColorTransform::SaturationModulation(amount))
    }
}

/// An `EG_ColorTransform` element's one attribute. Every member of the group that carries a value
/// carries it as `@val`; what that string *means* depends on which element it is, which is why the
/// declared kind is [`Text`] and [`Color::spec`] decides the codec from the element name — the same
/// shape [`Color`]'s own `@val` has one level up.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", codec = Text, accessor = value))]
struct TransformAttributes<A> {
    attributes: A,
}

/// A DrawingML color — one `EG_ColorChoice` element with its value attributes and any color-transform
/// children (`a:lumMod`, `a:alpha`, …).
///
/// The element *name* is the color kind; its children are the transforms. This is a **fidelity view**:
/// name, attributes, transform children, and the self-closing flag are preserved verbatim, while
/// [`kind`](Self::kind) / [`hex`](Self::hex) / [`scheme_color`](Self::scheme_color) expose the common
/// cases. Its [`FromXml`](mjx_ooxml_core::FromXml) / [`ToXml`](mjx_ooxml_core::ToXml) impls come
/// from `fidelity_element_impls!`, because the element name is the discriminant and the container
/// derive does not model that. Its one attribute is declared through the `#[xml(attribute)]`
/// grammar: `@val` is shared by all six kinds, and its *meaning* differs per kind (a hex triplet on
/// `a:srgbClr`, a scheme token on `a:schemeClr`, a system-colour name on `a:sysClr`), so the declared
/// kind is [`Text`] and [`hex`](Self::hex) / [`scheme_color`](Self::scheme_color) interpret it once
/// the element name has said which kind this is.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", codec = Text, accessor = value))]
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
        Self::of_val(interner, "srgbClr", Some(hex))
    }

    /// Builds a theme color reference `<a:schemeClr val="{scheme}"/>`.
    #[must_use]
    pub fn scheme(interner: &mut Interner, scheme: SchemeColor) -> Self {
        Self::of_val(interner, "schemeClr", Some(scheme.to_wire()))
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
        Some(Self::of_val(interner, local, value))
    }

    /// Builds a self-closing `<a:{local} val="{value}"/>` (with no `@val` at all when `value` is
    /// `None`) — the one place this type authors a color element, so every `@val` it writes is
    /// written by the same generated setter that a later edit would use.
    fn of_val(interner: &mut Interner, local: &str, value: Option<&str>) -> Self {
        let mut color = Self {
            name: dml_name(interner, local),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        color.set_value(interner, value);
        color
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
    ///
    /// The file's own letter case is preserved — `ff0000` and `FF0000` are the same color, and this
    /// is not the place to decide which spelling a file should have used.
    #[must_use]
    pub fn hex(&self, interner: &Interner) -> Option<Cow<'_, str>> {
        if self.kind(interner) == ColorKind::Srgb {
            self.value(interner).ok().flatten()
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
        self.value(interner)
            .ok()
            .flatten()
            .and_then(|value| SchemeColor::from_wire(&value))
    }

    /// This color as an interner-free [`ColorSpec`] — sRGB and scheme colors resolve to their first-
    /// class variants; any other kind becomes [`ColorSpec::Other`] carrying its `val`.
    ///
    /// A color carrying `EG_ColorTransform` children answers
    /// [`ColorSpec::Transformed`], with each child read as a
    /// [`ColorTransform`] **in document order** — so `spec` → [`from_spec`](Self::from_spec) keeps a
    /// producer's transforms instead of dropping them. A child the group does not name, or one it
    /// does whose `@val` cannot be read, becomes [`ColorTransform::Other`] and still round-trips.
    ///
    /// It is still a *value* description rather than a fidelity view: a transform child's prefix,
    /// attribute order and any attribute other than `@val` are not represented here (they are
    /// preserved on the [`Color`] itself, which is what re-emits an untouched part). A non-element
    /// child — a comment or stray text, which the schema does not permit inside a color — is not
    /// represented either.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> ColorSpec {
        let base = match self.kind(interner) {
            ColorKind::Srgb => ColorSpec::Srgb(self.raw_value(interner).unwrap_or_default()),
            ColorKind::Scheme => match self.scheme_color(interner) {
                Some(scheme) => ColorSpec::Scheme(scheme),
                None => ColorSpec::Other {
                    kind: ColorKind::Scheme,
                    value: self.raw_value(interner),
                },
            },
            kind => ColorSpec::Other {
                kind,
                value: self.raw_value(interner),
            },
        };
        let transforms = self.transform_specs(interner);
        if transforms.is_empty() {
            base
        } else {
            ColorSpec::Transformed {
                base: Box::new(base),
                transforms,
            }
        }
    }

    /// This color's transform children as [`ColorTransform`]s, in document order.
    fn transform_specs(&self, interner: &Interner) -> Vec<ColorTransform> {
        self.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) => Some(element),
                _ => None,
            })
            .map(|element| {
                let local = interner.resolve(element.name.local);
                let attributes = TransformAttributes {
                    attributes: &element.attributes,
                };
                let value = attributes.value(interner).ok().flatten();
                let kind = is_dml(&element.name, interner)
                    .then(|| ColorTransformKind::from_local_name(local))
                    .flatten();
                let typed = match (kind, value.as_deref()) {
                    (Some(kind), raw) => match kind.value_kind() {
                        ColorTransformValue::Percentage => raw
                            .and_then(|raw| Percentage::decode(Cow::Borrowed(raw)).ok())
                            .and_then(|amount| ColorTransform::from_percentage(kind, amount)),
                        ColorTransformValue::Angle => raw
                            .and_then(|raw| {
                                SixtyThousandthsOfADegree::decode(Cow::Borrowed(raw)).ok()
                            })
                            .and_then(|angle| ColorTransform::from_angle(kind, angle)),
                        ColorTransformValue::Marker => ColorTransform::marker(kind),
                        ColorTransformValue::Raw => None,
                    },
                    (None, _) => None,
                };
                typed.unwrap_or_else(|| {
                    ColorTransform::other(local, value.map(std::borrow::Cow::into_owned))
                })
            })
            .collect()
    }

    /// Builds a color from an interner-free [`ColorSpec`], including its transform children in the
    /// order the spec lists them. Returns `None` only for an [`Other`](ColorSpec::Other) spec whose
    /// kind names no element ([`Unknown`](ColorKind::Unknown)).
    #[must_use]
    pub fn from_spec(interner: &mut Interner, spec: &ColorSpec) -> Option<Self> {
        // Walk down to the colour underneath, remembering each level's transforms on the way. The
        // builders keep a `Transformed` exactly one level deep, so this runs once — but the
        // variant's fields are public and a hand-built nest must **flatten**, not panic and not
        // write a colour inside a colour. The innermost level is written first, because that is
        // what nesting one transformed colour inside another says.
        let mut layers: Vec<&[ColorTransform]> = Vec::new();
        let mut base = spec;
        while let ColorSpec::Transformed {
            base: inner,
            transforms,
        } = base
        {
            layers.push(transforms);
            base = inner;
        }
        let mut color = match base {
            ColorSpec::Srgb(hex) => Self::srgb(interner, hex),
            ColorSpec::Scheme(scheme) => Self::scheme(interner, *scheme),
            ColorSpec::Other { kind, value } => Self::of_kind(interner, *kind, value.as_deref())?,
            // The loop above ended, so `base` is not `Transformed`; answering `None` rather than
            // panicking keeps this total whatever a caller hands in.
            ColorSpec::Transformed { .. } => return None,
        };
        for transforms in layers.into_iter().rev() {
            for transform in transforms {
                color.push_transform(interner, transform);
            }
        }
        Some(color)
    }

    /// Appends one `EG_ColorTransform` child, written as `<a:{local} val="…"/>` (with no `@val` at
    /// all for the five valueless members, and none for an [`Other`](ColorTransform::Other) that had
    /// none).
    fn push_transform(&mut self, interner: &mut Interner, transform: &ColorTransform) {
        let value: Option<Cow<'static, str>> = match transform {
            ColorTransform::Other { value, .. } => {
                value.as_ref().map(|raw| Cow::Owned(raw.clone()))
            }
            typed => match (typed.percentage(), typed.angle()) {
                (Some(amount), _) => Some(Percentage::encode(amount)),
                (_, Some(angle)) => Some(SixtyThousandthsOfADegree::encode(angle)),
                _ => None,
            },
        };
        let local = transform.local_name().to_owned();
        let mut element = dml_element(interner, &local, Vec::new(), Vec::new());
        if let Some(value) = value {
            TransformAttributes {
                attributes: &mut element.attributes,
            }
            .set_value(interner, Some(value.as_ref()));
        }
        self.children.push(RawNode::Element(element));
        self.empty = false;
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

    /// `@val` as an owned string, with a malformed one (unreadable bytes, an undecodable entity)
    /// treated as absent — what [`spec`](Self::spec) wants, since a [`ColorSpec`] is a value
    /// description that carries no error channel.
    fn raw_value(&self, interner: &Interner) -> Option<String> {
        self.value(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }
}

fidelity_element_impls!(Color);
