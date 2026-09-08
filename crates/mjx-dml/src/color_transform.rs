//! DrawingML colour transforms: the `EG_ColorTransform` group (`a:tint`, `a:lumMod`, `a:gamma`, …).
//!
//! Every `EG_ColorChoice` element — `a:srgbClr`, `a:schemeClr`, `a:sysClr`, `a:prstClr`,
//! `a:scrgbClr`, `a:hslClr` — has exactly one content model: `EG_ColorTransform` repeated zero or
//! more times (ECMA-376 Part 1 §20.1.2.3.32 `CT_SRgbColor` and its five siblings). So *any* child of
//! a colour is a transform, and [`ColorTransform`] is the interner-free description of one.
//!
//! # Order is part of the markup
//!
//! The group is a `xsd:choice` with `maxOccurs="unbounded"`, so the same transforms in a different
//! order are a **different colour**: `<a:lumMod val="50000"/><a:lumOff val="50000"/>` is not
//! `<a:lumOff val="50000"/><a:lumMod val="50000"/>`, because
//! [`resolve_color`](crate::resolve_color) applies them left to right onto a running value. That is
//! why [`ColorSpec`](crate::ColorSpec) carries a `Vec` rather than a set, and why every builder on
//! it **appends**.
//!
//! # Two representations, and why both
//!
//! [`ColorTransformKind`] names the twenty-eight members of the group plus
//! [`Other`](ColorTransformKind::Other); [`ColorTransform`] is the same twenty-nine with the value
//! each carries. That is the shape [`ColorKind`](crate::ColorKind) and
//! [`ColorSpec`](crate::ColorSpec) already have one layer up, for the same two reasons: a caller
//! that only wants to *ask* which transform this is should not have to destructure a payload, and
//! the two bindings project the kind as an enumeration (which their own suites check member by
//! member) and the value as a class.
//!
//! # What the payload is, per member
//!
//! * **Twenty-four are percentages** ([`Fraction`]): `1.0` is 100 %. The wire form is either
//!   1000ths of a percent (`val="50000"`) or an explicit percentage (`val="50%"`); both are read and
//!   the first is written, exactly as [`crate::codec::Percentage`] documents.
//! * **Two are angles** ([`Angle`]) — `a:hue` and `a:hueOff` — written in 60000ths of a degree.
//! * **Five carry nothing at all**: `a:comp`, `a:inv`, `a:gray`, `a:gamma`, `a:invGamma` are empty
//!   elements whose complex types (`CT_ComplementTransform` and its four siblings) declare no
//!   attribute and no content.
//! * **[`Other`](ColorTransform::Other)** carries the element's local name and its raw `@val`, and is
//!   how a transform this model cannot read survives a round trip — an element the group does not
//!   name, or one it does whose `@val` is missing or unparseable. It exists for the same reason
//!   [`ColorSpec::Other`](crate::ColorSpec::Other) does: a description that can only hold what it
//!   understands loses the rest, and losing a producer's markup is the one thing this project is
//!   built not to do.

use crate::geometry::{Angle, Fraction};

/// Which member of `EG_ColorTransform` a [`ColorTransform`] is — its element name, without its value.
///
/// The twenty-eight members are listed in the order `dml-main.xsd` lists them, which is neither
/// alphabetical nor grouped by channel; keeping the schema's order makes the two lists comparable by
/// eye. [`Other`](Self::Other) is not a member of the group: it is this model's bucket for an
/// element in the group's position that it cannot read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorTransformKind {
    /// `a:tint` — lightens toward white by a percentage. `ST_PositiveFixedPercentage`.
    Tint,
    /// `a:shade` — darkens toward black by a percentage. `ST_PositiveFixedPercentage`.
    Shade,
    /// `a:comp` — the complement of the colour. No value.
    Complement,
    /// `a:inv` — the inverse of the colour. No value.
    Inverse,
    /// `a:gray` — the colour converted to grayscale. No value.
    Grayscale,
    /// `a:alpha` — sets the opacity. `ST_PositiveFixedPercentage`.
    Alpha,
    /// `a:alphaOff` — shifts the opacity by a signed percentage. `ST_FixedPercentage`.
    AlphaOffset,
    /// `a:alphaMod` — multiplies the opacity by a percentage. `ST_PositivePercentage`.
    AlphaModulation,
    /// `a:hue` — sets the hue angle. `ST_PositiveFixedAngle`.
    Hue,
    /// `a:hueOff` — shifts the hue by a signed angle. `ST_Angle`.
    HueOffset,
    /// `a:hueMod` — multiplies the hue by a percentage. `ST_PositivePercentage`.
    HueModulation,
    /// `a:sat` — sets the saturation. `ST_Percentage`.
    Saturation,
    /// `a:satOff` — shifts the saturation by a percentage. `ST_Percentage`.
    SaturationOffset,
    /// `a:satMod` — multiplies the saturation by a percentage. `ST_Percentage`.
    SaturationModulation,
    /// `a:lum` — sets the luminance. `ST_Percentage`.
    Luminance,
    /// `a:lumOff` — shifts the luminance by a percentage. `ST_Percentage`.
    LuminanceOffset,
    /// `a:lumMod` — multiplies the luminance by a percentage. `ST_Percentage`.
    LuminanceModulation,
    /// `a:red` — sets the red channel. `ST_Percentage`.
    Red,
    /// `a:redOff` — shifts the red channel by a percentage. `ST_Percentage`.
    RedOffset,
    /// `a:redMod` — multiplies the red channel by a percentage. `ST_Percentage`.
    RedModulation,
    /// `a:green` — sets the green channel. `ST_Percentage`.
    Green,
    /// `a:greenOff` — shifts the green channel by a percentage. `ST_Percentage`.
    GreenOffset,
    /// `a:greenMod` — multiplies the green channel by a percentage. `ST_Percentage`.
    GreenModulation,
    /// `a:blue` — sets the blue channel. `ST_Percentage`.
    Blue,
    /// `a:blueOff` — shifts the blue channel by a percentage. `ST_Percentage`.
    BlueOffset,
    /// `a:blueMod` — multiplies the blue channel by a percentage. `ST_Percentage`.
    BlueModulation,
    /// `a:gamma` — applies the sRGB gamma curve. No value.
    Gamma,
    /// `a:invGamma` — applies the inverse sRGB gamma curve. No value.
    InverseGamma,
    /// Not a member of the group: an element in a colour's child position that this model does not
    /// read, kept by name and raw value so it survives a round trip.
    Other,
}

/// What a [`ColorTransformKind`] carries on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorTransformValue {
    /// A percentage, read and written by [`crate::codec::Percentage`].
    Percentage,
    /// An angle in 60000ths of a degree, read and written by
    /// [`crate::codec::SixtyThousandthsOfADegree`].
    Angle,
    /// Nothing — an empty element with no attributes.
    Marker,
    /// A raw string, because this model could not read it as anything better.
    Raw,
}

/// Every member of `EG_ColorTransform`, in the order `dml-main.xsd` declares them, with its local
/// name and the kind of value it carries. [`ColorTransformKind::Other`] is deliberately absent: it
/// names no element.
const MEMBERS: [(ColorTransformKind, &str, ColorTransformValue); 28] = {
    use ColorTransformKind as K;
    use ColorTransformValue::{Angle as A, Marker as M, Percentage as P};
    [
        (K::Tint, "tint", P),
        (K::Shade, "shade", P),
        (K::Complement, "comp", M),
        (K::Inverse, "inv", M),
        (K::Grayscale, "gray", M),
        (K::Alpha, "alpha", P),
        (K::AlphaOffset, "alphaOff", P),
        (K::AlphaModulation, "alphaMod", P),
        (K::Hue, "hue", A),
        (K::HueOffset, "hueOff", A),
        (K::HueModulation, "hueMod", P),
        (K::Saturation, "sat", P),
        (K::SaturationOffset, "satOff", P),
        (K::SaturationModulation, "satMod", P),
        (K::Luminance, "lum", P),
        (K::LuminanceOffset, "lumOff", P),
        (K::LuminanceModulation, "lumMod", P),
        (K::Red, "red", P),
        (K::RedOffset, "redOff", P),
        (K::RedModulation, "redMod", P),
        (K::Green, "green", P),
        (K::GreenOffset, "greenOff", P),
        (K::GreenModulation, "greenMod", P),
        (K::Blue, "blue", P),
        (K::BlueOffset, "blueOff", P),
        (K::BlueModulation, "blueMod", P),
        (K::Gamma, "gamma", M),
        (K::InverseGamma, "invGamma", M),
    ]
};

impl ColorTransformKind {
    /// The twenty-eight members of `EG_ColorTransform`, in schema order. [`Other`](Self::Other) is
    /// not among them, because it names no element.
    ///
    /// ```
    /// use mjx_dml::ColorTransformKind;
    ///
    /// assert_eq!(ColorTransformKind::ALL.len(), 28);
    /// assert_eq!(ColorTransformKind::ALL[0], ColorTransformKind::Tint);
    /// ```
    pub const ALL: [Self; 28] = {
        let mut all = [Self::Other; 28];
        let mut index = 0;
        while index < MEMBERS.len() {
            all[index] = MEMBERS[index].0;
            index += 1;
        }
        all
    };

    /// This member's element local name (`ColorTransformKind::LuminanceModulation` → `"lumMod"`), or
    /// `None` for [`Other`](Self::Other), which names no element.
    ///
    /// ```
    /// use mjx_dml::ColorTransformKind;
    ///
    /// assert_eq!(ColorTransformKind::LuminanceModulation.local_name(), Some("lumMod"));
    /// assert_eq!(ColorTransformKind::Other.local_name(), None);
    /// ```
    #[must_use]
    pub fn local_name(self) -> Option<&'static str> {
        MEMBERS
            .iter()
            .find_map(|(kind, local, _)| (*kind == self).then_some(*local))
    }

    /// The member a local name names, or `None` if the group has no such element.
    ///
    /// ```
    /// use mjx_dml::ColorTransformKind;
    ///
    /// assert_eq!(ColorTransformKind::from_local_name("satMod"),
    ///            Some(ColorTransformKind::SaturationModulation));
    /// assert_eq!(ColorTransformKind::from_local_name("extLst"), None);
    /// ```
    #[must_use]
    pub fn from_local_name(local: &str) -> Option<Self> {
        MEMBERS
            .iter()
            .find_map(|(kind, name, _)| (*name == local).then_some(*kind))
    }

    /// What this member carries on the wire — a percentage, an angle, nothing, or (for
    /// [`Other`](Self::Other)) a raw string.
    ///
    /// ```
    /// use mjx_dml::{ColorTransformKind, ColorTransformValue};
    ///
    /// assert_eq!(ColorTransformKind::Tint.value_kind(), ColorTransformValue::Percentage);
    /// assert_eq!(ColorTransformKind::HueOffset.value_kind(), ColorTransformValue::Angle);
    /// assert_eq!(ColorTransformKind::Gamma.value_kind(), ColorTransformValue::Marker);
    /// ```
    #[must_use]
    pub fn value_kind(self) -> ColorTransformValue {
        MEMBERS
            .iter()
            .find_map(|(kind, _, value)| (*kind == self).then_some(*value))
            .unwrap_or(ColorTransformValue::Raw)
    }
}

/// One `EG_ColorTransform` child of a colour, as an interner-free value.
///
/// Build one directly (`ColorTransform::LuminanceModulation(Fraction::from_ratio(0.75))`), or from a
/// [`ColorTransformKind`] and a value with [`from_percentage`](Self::from_percentage),
/// [`from_angle`](Self::from_angle) or [`marker`](Self::marker) — which is what the two bindings do,
/// because neither language can construct a Rust enumeration variant that carries a payload.
///
/// See the [module documentation](self) for what each member means and what it carries.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorTransform {
    /// `a:tint` — lightens toward white.
    Tint(Fraction),
    /// `a:shade` — darkens toward black.
    Shade(Fraction),
    /// `a:comp` — the complement of the colour.
    Complement,
    /// `a:inv` — the inverse of the colour.
    Inverse,
    /// `a:gray` — the colour converted to grayscale.
    Grayscale,
    /// `a:alpha` — sets the opacity.
    Alpha(Fraction),
    /// `a:alphaOff` — shifts the opacity.
    AlphaOffset(Fraction),
    /// `a:alphaMod` — multiplies the opacity.
    AlphaModulation(Fraction),
    /// `a:hue` — sets the hue angle.
    Hue(Angle),
    /// `a:hueOff` — shifts the hue angle.
    HueOffset(Angle),
    /// `a:hueMod` — multiplies the hue.
    HueModulation(Fraction),
    /// `a:sat` — sets the saturation.
    Saturation(Fraction),
    /// `a:satOff` — shifts the saturation.
    SaturationOffset(Fraction),
    /// `a:satMod` — multiplies the saturation.
    SaturationModulation(Fraction),
    /// `a:lum` — sets the luminance.
    Luminance(Fraction),
    /// `a:lumOff` — shifts the luminance.
    LuminanceOffset(Fraction),
    /// `a:lumMod` — multiplies the luminance.
    LuminanceModulation(Fraction),
    /// `a:red` — sets the red channel.
    Red(Fraction),
    /// `a:redOff` — shifts the red channel.
    RedOffset(Fraction),
    /// `a:redMod` — multiplies the red channel.
    RedModulation(Fraction),
    /// `a:green` — sets the green channel.
    Green(Fraction),
    /// `a:greenOff` — shifts the green channel.
    GreenOffset(Fraction),
    /// `a:greenMod` — multiplies the green channel.
    GreenModulation(Fraction),
    /// `a:blue` — sets the blue channel.
    Blue(Fraction),
    /// `a:blueOff` — shifts the blue channel.
    BlueOffset(Fraction),
    /// `a:blueMod` — multiplies the blue channel.
    BlueModulation(Fraction),
    /// `a:gamma` — applies the sRGB gamma curve.
    Gamma,
    /// `a:invGamma` — applies the inverse sRGB gamma curve.
    InverseGamma,
    /// An element in a colour's child position this model does not read — one the group does not
    /// name, or one it does whose `@val` is absent or unparseable — kept by local name and raw
    /// value so it round-trips.
    Other {
        /// The element's local name, without a prefix.
        name: String,
        /// Its `@val` attribute, if it had one.
        value: Option<String>,
    },
}

impl ColorTransform {
    /// Which member of the group this is.
    ///
    /// ```
    /// use mjx_dml::{ColorTransform, ColorTransformKind, Fraction};
    ///
    /// let tint = ColorTransform::Tint(Fraction::from_ratio(0.5));
    /// assert_eq!(tint.kind(), ColorTransformKind::Tint);
    /// ```
    #[must_use]
    pub fn kind(&self) -> ColorTransformKind {
        use ColorTransformKind as K;
        match self {
            Self::Tint(_) => K::Tint,
            Self::Shade(_) => K::Shade,
            Self::Complement => K::Complement,
            Self::Inverse => K::Inverse,
            Self::Grayscale => K::Grayscale,
            Self::Alpha(_) => K::Alpha,
            Self::AlphaOffset(_) => K::AlphaOffset,
            Self::AlphaModulation(_) => K::AlphaModulation,
            Self::Hue(_) => K::Hue,
            Self::HueOffset(_) => K::HueOffset,
            Self::HueModulation(_) => K::HueModulation,
            Self::Saturation(_) => K::Saturation,
            Self::SaturationOffset(_) => K::SaturationOffset,
            Self::SaturationModulation(_) => K::SaturationModulation,
            Self::Luminance(_) => K::Luminance,
            Self::LuminanceOffset(_) => K::LuminanceOffset,
            Self::LuminanceModulation(_) => K::LuminanceModulation,
            Self::Red(_) => K::Red,
            Self::RedOffset(_) => K::RedOffset,
            Self::RedModulation(_) => K::RedModulation,
            Self::Green(_) => K::Green,
            Self::GreenOffset(_) => K::GreenOffset,
            Self::GreenModulation(_) => K::GreenModulation,
            Self::Blue(_) => K::Blue,
            Self::BlueOffset(_) => K::BlueOffset,
            Self::BlueModulation(_) => K::BlueModulation,
            Self::Gamma => K::Gamma,
            Self::InverseGamma => K::InverseGamma,
            Self::Other { .. } => K::Other,
        }
    }

    /// The element local name this transform writes — its kind's, except for
    /// [`Other`](Self::Other), which writes the name it was read under.
    #[must_use]
    pub fn local_name(&self) -> &str {
        match self {
            Self::Other { name, .. } => name,
            // Every other variant's kind is a real member, so it names an element.
            other => other.kind().local_name().unwrap_or_default(),
        }
    }

    /// The percentage this transform carries, or `None` if it carries an angle, nothing, or a value
    /// this model could not read.
    ///
    /// ```
    /// use mjx_dml::{ColorTransform, Fraction};
    ///
    /// assert_eq!(ColorTransform::Tint(Fraction::from_ratio(0.5)).percentage(),
    ///            Some(Fraction::from_ratio(0.5)));
    /// assert_eq!(ColorTransform::Gamma.percentage(), None);
    /// ```
    #[must_use]
    pub fn percentage(&self) -> Option<Fraction> {
        match self {
            Self::Tint(value)
            | Self::Shade(value)
            | Self::Alpha(value)
            | Self::AlphaOffset(value)
            | Self::AlphaModulation(value)
            | Self::HueModulation(value)
            | Self::Saturation(value)
            | Self::SaturationOffset(value)
            | Self::SaturationModulation(value)
            | Self::Luminance(value)
            | Self::LuminanceOffset(value)
            | Self::LuminanceModulation(value)
            | Self::Red(value)
            | Self::RedOffset(value)
            | Self::RedModulation(value)
            | Self::Green(value)
            | Self::GreenOffset(value)
            | Self::GreenModulation(value)
            | Self::Blue(value)
            | Self::BlueOffset(value)
            | Self::BlueModulation(value) => Some(*value),
            _ => None,
        }
    }

    /// The angle this transform carries — only `a:hue` and `a:hueOff` do.
    #[must_use]
    pub fn angle(&self) -> Option<Angle> {
        match self {
            Self::Hue(angle) | Self::HueOffset(angle) => Some(*angle),
            _ => None,
        }
    }

    /// The raw `@val` this transform was read under, for an [`Other`](Self::Other) only. A typed
    /// member reports its value through [`percentage`](Self::percentage) or [`angle`](Self::angle).
    #[must_use]
    pub fn raw_value(&self) -> Option<&str> {
        match self {
            Self::Other { value, .. } => value.as_deref(),
            _ => None,
        }
    }

    /// A percentage-valued transform of `kind`, or `None` if that member does not take a percentage.
    ///
    /// This is the constructor the two bindings call: neither Python nor JavaScript can name a Rust
    /// enumeration variant that carries a payload, so both build a transform from its kind and its
    /// value instead.
    ///
    /// ```
    /// use mjx_dml::{ColorTransform, ColorTransformKind, Fraction};
    ///
    /// let dim = ColorTransform::from_percentage(
    ///     ColorTransformKind::LuminanceModulation,
    ///     Fraction::from_ratio(0.75),
    /// );
    /// assert_eq!(dim, Some(ColorTransform::LuminanceModulation(Fraction::from_ratio(0.75))));
    /// assert_eq!(ColorTransform::from_percentage(ColorTransformKind::Gamma,
    ///                                            Fraction::from_ratio(1.0)), None);
    /// ```
    #[must_use]
    pub fn from_percentage(kind: ColorTransformKind, value: Fraction) -> Option<Self> {
        use ColorTransformKind as K;
        Some(match kind {
            K::Tint => Self::Tint(value),
            K::Shade => Self::Shade(value),
            K::Alpha => Self::Alpha(value),
            K::AlphaOffset => Self::AlphaOffset(value),
            K::AlphaModulation => Self::AlphaModulation(value),
            K::HueModulation => Self::HueModulation(value),
            K::Saturation => Self::Saturation(value),
            K::SaturationOffset => Self::SaturationOffset(value),
            K::SaturationModulation => Self::SaturationModulation(value),
            K::Luminance => Self::Luminance(value),
            K::LuminanceOffset => Self::LuminanceOffset(value),
            K::LuminanceModulation => Self::LuminanceModulation(value),
            K::Red => Self::Red(value),
            K::RedOffset => Self::RedOffset(value),
            K::RedModulation => Self::RedModulation(value),
            K::Green => Self::Green(value),
            K::GreenOffset => Self::GreenOffset(value),
            K::GreenModulation => Self::GreenModulation(value),
            K::Blue => Self::Blue(value),
            K::BlueOffset => Self::BlueOffset(value),
            K::BlueModulation => Self::BlueModulation(value),
            K::Complement
            | K::Inverse
            | K::Grayscale
            | K::Hue
            | K::HueOffset
            | K::Gamma
            | K::InverseGamma
            | K::Other => return None,
        })
    }

    /// An angle-valued transform of `kind` — `a:hue` or `a:hueOff` — or `None` for any other member.
    ///
    /// ```
    /// use mjx_dml::{Angle, ColorTransform, ColorTransformKind};
    ///
    /// let turn = ColorTransform::from_angle(ColorTransformKind::HueOffset,
    ///                                       Angle::from_degrees(30.0));
    /// assert_eq!(turn, Some(ColorTransform::HueOffset(Angle::from_degrees(30.0))));
    /// ```
    #[must_use]
    pub fn from_angle(kind: ColorTransformKind, value: Angle) -> Option<Self> {
        match kind {
            ColorTransformKind::Hue => Some(Self::Hue(value)),
            ColorTransformKind::HueOffset => Some(Self::HueOffset(value)),
            _ => None,
        }
    }

    /// A valueless transform of `kind` — `a:comp`, `a:inv`, `a:gray`, `a:gamma` or `a:invGamma` — or
    /// `None` for any member that carries a value.
    ///
    /// ```
    /// use mjx_dml::{ColorTransform, ColorTransformKind};
    ///
    /// assert_eq!(ColorTransform::marker(ColorTransformKind::InverseGamma),
    ///            Some(ColorTransform::InverseGamma));
    /// assert_eq!(ColorTransform::marker(ColorTransformKind::Tint), None);
    /// ```
    #[must_use]
    pub fn marker(kind: ColorTransformKind) -> Option<Self> {
        use ColorTransformKind as K;
        match kind {
            K::Complement => Some(Self::Complement),
            K::Inverse => Some(Self::Inverse),
            K::Grayscale => Some(Self::Grayscale),
            K::Gamma => Some(Self::Gamma),
            K::InverseGamma => Some(Self::InverseGamma),
            _ => None,
        }
    }

    /// A transform this model does not read, kept by local name and raw value.
    #[must_use]
    pub fn other(name: &str, value: Option<String>) -> Self {
        Self::Other {
            name: name.to_owned(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every member the schema declares is reachable both ways, and no two share a local name.
    #[test]
    fn every_member_maps_to_exactly_one_local_name() {
        let mut names: Vec<&str> = Vec::new();
        for kind in ColorTransformKind::ALL {
            let local = kind.local_name().expect("a member names an element");
            assert_eq!(ColorTransformKind::from_local_name(local), Some(kind));
            names.push(local);
        }
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two members share a local name");
        assert_eq!(count, 28, "EG_ColorTransform has twenty-eight members");
    }

    /// Every member is constructible from its kind through exactly one of the three constructors,
    /// which is what makes the whole group reachable from a language with no payload enumerations.
    #[test]
    fn every_member_is_reachable_from_its_kind() {
        for kind in ColorTransformKind::ALL {
            let built = [
                ColorTransform::from_percentage(kind, Fraction::from_ratio(0.5)),
                ColorTransform::from_angle(kind, Angle::from_degrees(30.0)),
                ColorTransform::marker(kind),
            ];
            let built: Vec<_> = built.into_iter().flatten().collect();
            assert_eq!(
                built.len(),
                1,
                "{kind:?} is built by {} of the three constructors, not one",
                built.len()
            );
            assert_eq!(built[0].kind(), kind);
            assert_eq!(Some(built[0].local_name()), kind.local_name());
        }
    }

    /// `Other` names no element and is built by none of the three typed constructors.
    #[test]
    fn other_is_outside_the_group() {
        assert_eq!(ColorTransformKind::Other.local_name(), None);
        assert!(!ColorTransformKind::ALL.contains(&ColorTransformKind::Other));
        assert_eq!(
            ColorTransform::from_percentage(ColorTransformKind::Other, Fraction::from_ratio(1.0)),
            None
        );
        assert_eq!(ColorTransform::marker(ColorTransformKind::Other), None);
        let kept = ColorTransform::other("extLst", Some("7".into()));
        assert_eq!(kept.local_name(), "extLst");
        assert_eq!(kept.raw_value(), Some("7"));
        assert_eq!(kept.kind(), ColorTransformKind::Other);
    }

    /// The value kind agrees with which constructor builds the member.
    #[test]
    fn the_value_kind_agrees_with_the_constructors() {
        for kind in ColorTransformKind::ALL {
            match kind.value_kind() {
                ColorTransformValue::Percentage => {
                    assert!(
                        ColorTransform::from_percentage(kind, Fraction::from_ratio(0.5)).is_some()
                    )
                }
                ColorTransformValue::Angle => {
                    assert!(ColorTransform::from_angle(kind, Angle::from_degrees(1.0)).is_some());
                }
                ColorTransformValue::Marker => assert!(ColorTransform::marker(kind).is_some()),
                ColorTransformValue::Raw => panic!("{kind:?} is a member and cannot be raw"),
            }
        }
        assert_eq!(
            ColorTransformKind::Other.value_kind(),
            ColorTransformValue::Raw
        );
    }
}
