//! `w:rPr` (`CT_RPr`) — run properties — and the character-formatting vocabulary it carries:
//! `EG_RPrBase`'s **39 members** (the ticket that named this child said 38 plus `oMath`; the schema
//! itself gives the group exactly 39, and `oMath` is `CT_OnOff`-shaped like nineteen of its siblings,
//! not a fortieth special case — see [`Toggle`]).
//!
//! # `EG_RPrBase`'s 39 members
//!
//! `rStyle`, `rFonts`, `b`, `bCs`, `i`, `iCs`, `caps`, `smallCaps`, `strike`, `dstrike`, `outline`,
//! `shadow`, `emboss`, `imprint`, `noProof`, `snapToGrid`, `vanish`, `webHidden`, `color`, `spacing`,
//! `w`, `kern`, `position`, `sz`, `szCs`, `highlight`, `u`, `effect`, `bdr`, `shd`, `fitText`,
//! `vertAlign`, `rtl`, `cs`, `em`, `lang`, `eastAsianLayout`, `specVanish`, `oMath` — confirmed against
//! `wml.xsd`'s `EG_RPrBase` group and against the generated `RUN_PROPERTIES` child-order table, which
//! gives all 39 the same rank (the group is an `xsd:choice`, `minOccurs="0" maxOccurs="unbounded"`, so
//! the schema imposes no order among them) and gives `rPrChange` (MJXOFF-126's own scope, kept opaque
//! here as [`RunPropertyContent::Change`]) the next rank after.
//!
//! Twenty of the 39 are `CT_OnOff`: `b`, `bCs`, `caps`, `cs`, `dstrike`, `emboss`, `i`, `iCs`,
//! `imprint`, `noProof`, `oMath`, `outline`, `rtl`, `shadow`, `smallCaps`, `snapToGrid`, `specVanish`,
//! `strike`, `vanish`, `webHidden` — one shared type, [`Toggle`], reused across all twenty exactly as
//! [`super::body::Text`] is reused across `t`/`delText`/`instrText`/`delInstrText`.
//!
//! # The `ST_OnOff` default is `true`, not absent, and not `false`
//!
//! `CT_OnOff`'s `val` attribute is `use="optional"` in the schema — `<w:b/>` with no `val` at all is
//! legal — and ECMA-376 Part 1's own prose for every one of these elements states the same rule:
//! "if this element is not present, the default value is to leave the ... formatting applied to
//! current text unchanged. If this element is present without a val attribute, its default value is
//! true, turning the property on" (§17.3.2.1, "b (Bold)"; the other nineteen carry the same sentence
//! for their own property). [`Toggle::value`] applies that default — a `default = true` on the
//! attribute grammar — but only ever within a *present* element; whether the element itself is
//! present at all is a separate question, answered by [`RunProperties`]'s own per-property accessor
//! returning `Option<Toggle>`-shaped results. Three states therefore stay distinguishable end to end:
//! the element absent (`None`), present and on (`Some(true)`, whether from an explicit `val="true"` or
//! a present element with no `val`), and present and explicitly off (`Some(false)`). Nothing here ever
//! collapses these to a bare `bool`.
//!
//! # The three `w:rPr` emptiness states
//!
//! `w:rPr` is [`Run`](super::body::Run)'s own field — a struct with the same
//! `name`/`attributes`/`empty`/content shape as [`super::body::Body`]/[`super::body::Paragraph`], so
//! it inherits the workspace's usual emptiness fidelity: **no `w:rPr` at all** is `Run::run_properties`
//! returning `None`; **`<w:rPr/>` self-closed** and **`<w:rPr></w:rPr>` with a separate end tag** are
//! both `Some`, distinguished by the retained `empty` flag on the [`RunProperties`] value itself and
//! reproduced byte-for-byte because nothing here ever forces `empty` to a fixed value on write — see
//! `crates/mjx-docx/tests/run_properties.rs` for the round-trip proof of all three.
//!
//! # Colour is four attributes, not `a:schemeClr`
//!
//! [`Color`] (`w:color`, `CT_Color`) and [`Underline`]/[`Border`]/[`Shading`]'s own colour attributes
//! carry `val` (a hex triplet or the literal `auto`), `themeColor`, `themeTint` and `themeShade` — four
//! attributes on one element. This is WordprocessingML's own colour model, not DrawingML's
//! `a:schemeClr` with child transform elements; nothing here imports or generalises a `mjx-dml` colour
//! type. Resolving a theme colour against `theme1.xml` is MJXOFF-104's, not this child's.

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, Number, RawAttribute, RawElement, RawName, RawNode, Text as TextCodec,
    ToXml,
};
use mjx_ooxml_types::child_order::RUN_PROPERTIES;
use mjx_ooxml_types::shared::{LanguageTag, TwipsMeasure, VerticalTextPosition};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    BorderStyle, CombineBrackets, DecimalNumber, EighthPointMeasure, EmphasisMark, FontTypeHint,
    HalfPointMeasure, HexadecimalColor, HighlightColor, PointMeasure, ShadingPattern,
    SignedHalfPointMeasure, SignedTwipsMeasure, TextEffect as TextEffectKind, TextScale,
    ThemeColor, ThemeFont, TwoDigitHexadecimalNumber, Underline as UnderlineKind,
};

use super::body::{wml_name, Unmodeled};

// -------------------------------------------------------------------------------------------
// Custom attribute codecs for the generated wire-string wrapper types this child needs.
//
// Each is the same shape as `body.rs`'s `ShortHex`/`WhitespacePreservation`: `mjx-ooxml-types`
// generates these as plain `from_wire`/`to_wire` wrappers (no `FromStr`/`Display`, since they are
// unions the codegen deliberately keeps as wire strings rather than parsing further — see that
// module's own docs), so `Enumeration<T>` cannot carry them and a small tag type is the seam.
// -------------------------------------------------------------------------------------------

/// `ST_HexColor` (`ST_HexColorAuto | s:ST_HexColorRGB`) as an attribute value — the wire string
/// itself (`"auto"` or six hex digits), preserved exactly.
#[derive(Debug)]
pub struct HexColor;

impl AttributeCodec for HexColor {
    type Value<'a> = HexadecimalColor;
    type Input<'a> = HexadecimalColor;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<HexadecimalColor, InvalidAttributeValue> {
        Ok(HexadecimalColor::from_wire(&raw))
    }

    fn encode<'a>(value: HexadecimalColor) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_UcharHexNumber` (one byte, two hex digits) as an attribute value — `themeTint`/`themeShade`
/// and their `Fill` counterparts on [`Color`]/[`Underline`]/[`Border`]/[`Shading`].
#[derive(Debug)]
pub struct ThemeHexDigit;

impl AttributeCodec for ThemeHexDigit {
    type Value<'a> = TwoDigitHexadecimalNumber;
    type Input<'a> = TwoDigitHexadecimalNumber;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<TwoDigitHexadecimalNumber, InvalidAttributeValue> {
        Ok(TwoDigitHexadecimalNumber::from_wire(&raw))
    }

    fn encode<'a>(value: TwoDigitHexadecimalNumber) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_HpsMeasure` (half-points; `sz`, `szCs`, `kern`) as an attribute value.
#[derive(Debug)]
pub struct HalfPoint;

impl AttributeCodec for HalfPoint {
    type Value<'a> = HalfPointMeasure;
    type Input<'a> = HalfPointMeasure;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<HalfPointMeasure, InvalidAttributeValue> {
        Ok(HalfPointMeasure::from_wire(&raw))
    }

    fn encode<'a>(value: HalfPointMeasure) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_SignedHpsMeasure` (signed half-points; `position`) as an attribute value.
#[derive(Debug)]
pub struct SignedHalfPoint;

impl AttributeCodec for SignedHalfPoint {
    type Value<'a> = SignedHalfPointMeasure;
    type Input<'a> = SignedHalfPointMeasure;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<SignedHalfPointMeasure, InvalidAttributeValue> {
        Ok(SignedHalfPointMeasure::from_wire(&raw))
    }

    fn encode<'a>(value: SignedHalfPointMeasure) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_SignedTwipsMeasure` (twentieths of a point, signed; `spacing`) as an attribute value.
#[derive(Debug)]
pub struct SignedTwips;

impl AttributeCodec for SignedTwips {
    type Value<'a> = SignedTwipsMeasure;
    type Input<'a> = SignedTwipsMeasure;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<SignedTwipsMeasure, InvalidAttributeValue> {
        Ok(SignedTwipsMeasure::from_wire(&raw))
    }

    fn encode<'a>(value: SignedTwipsMeasure) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `s:ST_TwipsMeasure` (`fitText`'s required width) as an attribute value.
#[derive(Debug)]
pub struct Twips;

impl AttributeCodec for Twips {
    type Value<'a> = TwipsMeasure;
    type Input<'a> = TwipsMeasure;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<TwipsMeasure, InvalidAttributeValue> {
        Ok(TwipsMeasure::from_wire(&raw))
    }

    fn encode<'a>(value: TwipsMeasure) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_TextScale` (a percentage, `600%`/`0`-style wire spellings both legal; `w`) as an attribute
/// value.
#[derive(Debug)]
pub struct Scale;

impl AttributeCodec for Scale {
    type Value<'a> = TextScale;
    type Input<'a> = TextScale;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<TextScale, InvalidAttributeValue> {
        Ok(TextScale::from_wire(&raw))
    }

    fn encode<'a>(value: TextScale) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `s:ST_Lang` (an RFC 4646 language tag; `lang`'s `val`/`eastAsia`/`bidi`) as an attribute value.
#[derive(Debug)]
pub struct Lang;

impl AttributeCodec for Lang {
    type Value<'a> = LanguageTag;
    type Input<'a> = LanguageTag;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<LanguageTag, InvalidAttributeValue> {
        Ok(LanguageTag::from_wire(&raw))
    }

    fn encode<'a>(value: LanguageTag) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

// -------------------------------------------------------------------------------------------
// Leaf types — one per `EG_RPrBase` complex type, each an attribute-only fidelity leaf in the same
// shape as `body.rs`'s `Break`/`PositionalTab`/`Symbol`: `#[derive(XmlAttributes)]` for the typed
// accessors, plus a hand-written `FromXml`/`ToXml` pair (these have no `children`/`text` framework
// field for the container derive to hang off — an attribute-only element has neither).
// -------------------------------------------------------------------------------------------

/// `CT_OnOff` — a boolean toggle with one optional `val` (`ST_OnOff`). Reused across all twenty
/// `CT_OnOff`-shaped `EG_RPrBase` members (`b`, `bCs`, `caps`, `cs`, `dstrike`, `emboss`, `i`, `iCs`,
/// `imprint`, `noProof`, `oMath`, `outline`, `rtl`, `shadow`, `smallCaps`, `snapToGrid`, `specVanish`,
/// `strike`, `vanish`, `webHidden`), exactly as [`super::body::Text`] is reused across four
/// `EG_RunInnerContent` members — which element this is is `name`, not the Rust type.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = OnOff, accessor = value, default = true))]
pub struct Toggle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Toggle {
    /// Builds a new `local` toggle element (e.g. `"b"`) with no `val` — present, defaulting to on
    /// per [`Toggle::value`]'s own doc comment, until [`Toggle::set_value`] states one explicitly.
    ///
    /// `pub(crate)`, not private: `paragraph_properties.rs` (MJXOFF-96) builds a `w:pPr/w:rPr`
    /// (`CT_ParaRPr`) whose `EG_RPrBase` half is the same twenty `CT_OnOff` shapes this run's `w:rPr`
    /// carries — reusing this constructor is exactly the "reuse rather than restate" MJXOFF-96 is
    /// told to follow for the leaf types this module already owns.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for Toggle {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Toggle {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_String` (`w:rStyle`, "Referenced Character Style", §17.3.2.29) — the id of the character
/// style this run refers to.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = style_id, required))]
pub struct CharacterStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CharacterStyle {
    /// Builds a new `w:rStyle` referring to `style_id`.
    #[must_use]
    pub fn new(interner: &mut Interner, style_id: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "rStyle"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_style_id(interner, style_id);
        value
    }
}

impl FromXml for CharacterStyle {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CharacterStyle {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Fonts` (`w:rFonts`, "Run Fonts", §17.3.2.26) — the fonts this run uses, per script, and the
/// hint that breaks a tie when a character could come from more than one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "hint", prefix = "w", codec = Enumeration<FontTypeHint>, accessor = hint))]
#[xml(attribute(local = "ascii", prefix = "w", codec = TextCodec, accessor = ascii_font))]
#[xml(attribute(local = "hAnsi", prefix = "w", codec = TextCodec, accessor = high_ansi_font))]
#[xml(attribute(local = "eastAsia", prefix = "w", codec = TextCodec, accessor = east_asian_font))]
#[xml(attribute(local = "cs", prefix = "w", codec = TextCodec, accessor = complex_script_font))]
#[xml(attribute(local = "asciiTheme", prefix = "w", codec = Enumeration<ThemeFont>, accessor = ascii_theme_font))]
#[xml(attribute(local = "hAnsiTheme", prefix = "w", codec = Enumeration<ThemeFont>, accessor = high_ansi_theme_font))]
#[xml(attribute(local = "eastAsiaTheme", prefix = "w", codec = Enumeration<ThemeFont>, accessor = east_asian_theme_font))]
#[xml(attribute(local = "cstheme", prefix = "w", codec = Enumeration<ThemeFont>, accessor = complex_script_theme_font))]
pub struct Fonts {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Fonts {
    /// Builds a new, empty `w:rFonts` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "rFonts"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for Fonts {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Fonts {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Color` (`w:color`, "Run Content Color", §17.3.2.6) — see the module's own doc comment for why
/// this is four attributes, not `a:schemeClr`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = HexColor, accessor = hex_value, required))]
#[xml(attribute(local = "themeColor", prefix = "w", codec = Enumeration<ThemeColor>, accessor = theme_color))]
#[xml(attribute(local = "themeTint", prefix = "w", codec = ThemeHexDigit, accessor = theme_tint))]
#[xml(attribute(local = "themeShade", prefix = "w", codec = ThemeHexDigit, accessor = theme_shade))]
pub struct Color {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Color {
    /// Builds a new `w:color` with hex or `"auto"` value `hex_value`.
    #[must_use]
    pub fn new(interner: &mut Interner, hex_value: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "color"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_hex_value(interner, HexadecimalColor::from_wire(hex_value));
        value
    }
}

impl FromXml for Color {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Color {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Underline` (`w:u`, "Underline", §17.3.2.40) — style plus the same four colour attributes as
/// [`Color`]. `style` is optional per the schema (`ST_Underline` carries `none` for "no underline",
/// distinct from the element being absent); `color` defaults to `"auto"` when the element is present
/// without one, a real `default="auto"` in `wml.xsd` rather than prose convention.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<UnderlineKind>, accessor = style))]
#[xml(attribute(local = "color", prefix = "w", codec = HexColor, accessor = color, default = HexadecimalColor::from_wire("auto")))]
#[xml(attribute(local = "themeColor", prefix = "w", codec = Enumeration<ThemeColor>, accessor = theme_color))]
#[xml(attribute(local = "themeTint", prefix = "w", codec = ThemeHexDigit, accessor = theme_tint))]
#[xml(attribute(local = "themeShade", prefix = "w", codec = ThemeHexDigit, accessor = theme_shade))]
pub struct Underline {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Underline {
    /// Builds a new, empty `w:u` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "u"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for Underline {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Underline {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TextEffect` (`w:effect`, "Animated Text Effect", §17.3.2.11) — a required animated-text kind.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TextEffectKind>, accessor = kind, required))]
pub struct TextEffect {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TextEffect {
    /// Builds a new `w:effect` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: TextEffectKind) -> Self {
        let mut value = Self {
            name: wml_name(interner, "effect"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_kind(interner, kind);
        value
    }
}

impl FromXml for TextEffect {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TextEffect {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Border` (`w:bdr`, "Text Border", §17.3.2.4) — a character border: style, the same four colour
/// attributes as [`Color`], a width in eighths of a point, spacing in points, and two on/off flags.
/// `color` defaults to `"auto"` and `space` to `0`, both real `wml.xsd` `default=` attributes.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<BorderStyle>, accessor = style, required))]
#[xml(attribute(local = "color", prefix = "w", codec = HexColor, accessor = color, default = HexadecimalColor::from_wire("auto")))]
#[xml(attribute(local = "themeColor", prefix = "w", codec = Enumeration<ThemeColor>, accessor = theme_color))]
#[xml(attribute(local = "themeTint", prefix = "w", codec = ThemeHexDigit, accessor = theme_tint))]
#[xml(attribute(local = "themeShade", prefix = "w", codec = ThemeHexDigit, accessor = theme_shade))]
#[xml(attribute(local = "sz", prefix = "w", codec = Number<EighthPointMeasure>, accessor = width_eighths_of_a_point))]
#[xml(attribute(local = "space", prefix = "w", codec = Number<PointMeasure>, accessor = spacing_points, default = 0))]
#[xml(attribute(local = "shadow", prefix = "w", codec = OnOff, accessor = shadow))]
#[xml(attribute(local = "frame", prefix = "w", codec = OnOff, accessor = frame))]
pub struct Border {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Border {
    /// Builds a new `w:bdr` of `style`.
    #[must_use]
    pub fn new(interner: &mut Interner, style: BorderStyle) -> Self {
        let mut value = Self {
            name: wml_name(interner, "bdr"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_style(interner, style);
        value
    }
}

impl FromXml for Border {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Border {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Shd` (`w:shd`, "Run Shading", §17.3.2.32) — a shading pattern plus two independent colours
/// (pattern colour, fill colour), each with its own theme-colour triple.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<ShadingPattern>, accessor = pattern, required))]
#[xml(attribute(local = "color", prefix = "w", codec = HexColor, accessor = color))]
#[xml(attribute(local = "themeColor", prefix = "w", codec = Enumeration<ThemeColor>, accessor = theme_color))]
#[xml(attribute(local = "themeTint", prefix = "w", codec = ThemeHexDigit, accessor = theme_tint))]
#[xml(attribute(local = "themeShade", prefix = "w", codec = ThemeHexDigit, accessor = theme_shade))]
#[xml(attribute(local = "fill", prefix = "w", codec = HexColor, accessor = fill_color))]
#[xml(attribute(local = "themeFill", prefix = "w", codec = Enumeration<ThemeColor>, accessor = theme_fill_color))]
#[xml(attribute(local = "themeFillTint", prefix = "w", codec = ThemeHexDigit, accessor = theme_fill_tint))]
#[xml(attribute(local = "themeFillShade", prefix = "w", codec = ThemeHexDigit, accessor = theme_fill_shade))]
pub struct Shading {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Shading {
    /// Builds a new `w:shd` of `pattern`.
    #[must_use]
    pub fn new(interner: &mut Interner, pattern: ShadingPattern) -> Self {
        let mut value = Self {
            name: wml_name(interner, "shd"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_pattern(interner, pattern);
        value
    }
}

impl FromXml for Shading {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Shading {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_VerticalAlignRun` (`w:vertAlign`, "Subscript/Superscript Text", §17.3.2.42) — a required
/// baseline position.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<VerticalTextPosition>, accessor = position, required))]
pub struct VerticalAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl VerticalAlignment {
    /// Builds a new `w:vertAlign` of `position`.
    #[must_use]
    pub fn new(interner: &mut Interner, position: VerticalTextPosition) -> Self {
        let mut value = Self {
            name: wml_name(interner, "vertAlign"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_position(interner, position);
        value
    }
}

impl FromXml for VerticalAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for VerticalAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_FitText` (`w:fitText`, "Manual Run Width", §17.3.2.14) — a required width in twips, and an
/// optional id linking the runs of one manually-fitted span together.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Twips, accessor = width, required))]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id))]
pub struct ManualRunWidth {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ManualRunWidth {
    /// Builds a new `w:fitText` of `width`.
    #[must_use]
    pub fn new(interner: &mut Interner, width: TwipsMeasure) -> Self {
        let mut value = Self {
            name: wml_name(interner, "fitText"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_width(interner, width);
        value
    }
}

impl FromXml for ManualRunWidth {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ManualRunWidth {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Em` (`w:em`, "Emphasis Mark", §17.3.2.12) — a required emphasis-mark kind.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<EmphasisMark>, accessor = mark, required))]
pub struct Emphasis {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Emphasis {
    /// Builds a new `w:em` of `mark`.
    #[must_use]
    pub fn new(interner: &mut Interner, mark: EmphasisMark) -> Self {
        let mut value = Self {
            name: wml_name(interner, "em"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_mark(interner, mark);
        value
    }
}

impl FromXml for Emphasis {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Emphasis {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Language` (`w:lang`, "Languages for Run Content", §17.3.2.20) — the language to spell-check
/// this run's Latin (`val`), East Asian (`eastAsia`) and complex-script (`bidi`) text against.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Lang, accessor = latin))]
#[xml(attribute(local = "eastAsia", prefix = "w", codec = Lang, accessor = east_asian))]
#[xml(attribute(local = "bidi", prefix = "w", codec = Lang, accessor = complex_script))]
pub struct Languages {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Languages {
    /// Builds a new, empty `w:lang` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "lang"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for Languages {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Languages {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_EastAsianLayout` (`w:eastAsianLayout`, "East Asian Typography Settings", §17.3.2.10) —
/// two-lines-in-one and related East Asian layout switches.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<DecimalNumber>, accessor = id))]
#[xml(attribute(local = "combine", prefix = "w", codec = OnOff, accessor = combine_two_lines))]
#[xml(attribute(local = "combineBrackets", prefix = "w", codec = Enumeration<CombineBrackets>, accessor = combine_brackets))]
#[xml(attribute(local = "vert", prefix = "w", codec = OnOff, accessor = vertical))]
#[xml(attribute(local = "vertCompress", prefix = "w", codec = OnOff, accessor = vertical_compressed))]
pub struct EastAsianLayout {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl EastAsianLayout {
    /// Builds a new, empty `w:eastAsianLayout` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "eastAsianLayout"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for EastAsianLayout {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for EastAsianLayout {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Highlight` (`w:highlight`, "Text Highlighting", §17.3.2.15) — a required highlight colour.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<HighlightColor>, accessor = color, required))]
pub struct Highlight {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Highlight {
    /// Builds a new `w:highlight` of `color`.
    #[must_use]
    pub fn new(interner: &mut Interner, color: HighlightColor) -> Self {
        let mut value = Self {
            name: wml_name(interner, "highlight"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_color(interner, color);
        value
    }
}

impl FromXml for Highlight {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Highlight {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_HpsMeasure` — a required half-point measure. Reused for `sz` ("Non-Complex Script Font Size",
/// §17.3.2.38), `szCs` ("Complex Script Font Size", §17.3.2.39) and `kern` ("Font Kerning",
/// §17.3.2.19) — three different properties sharing one wire shape, exactly as [`Toggle`] is reused
/// across the twenty `CT_OnOff` members.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = HalfPoint, accessor = half_points, required))]
pub struct HalfPointMeasureValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl HalfPointMeasureValue {
    /// Builds a new `local` element (`"sz"`, `"szCs"` or `"kern"`) of `half_points`.
    ///
    /// `pub(crate)`: `paragraph_properties.rs` (MJXOFF-96) reuses this for `w:pPr/w:rPr`'s own
    /// `sz`/`szCs`/`kern`, the same three `CT_HpsMeasure` members `EG_RPrBase` gives a run's `w:rPr`.
    #[must_use]
    pub(crate) fn new(interner: &mut Interner, local: &str, half_points: HalfPointMeasure) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_half_points(interner, half_points);
        value
    }
}

impl FromXml for HalfPointMeasureValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for HalfPointMeasureValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_SignedHpsMeasure` (`w:position`, "Vertically Raised or Lowered Text", §17.3.2.24) — a required
/// signed half-point offset.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = SignedHalfPoint, accessor = half_points, required))]
pub struct SignedHalfPointMeasureValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SignedHalfPointMeasureValue {
    /// Builds a new `w:position` of `half_points`.
    #[must_use]
    pub fn new(interner: &mut Interner, half_points: SignedHalfPointMeasure) -> Self {
        let mut value = Self {
            name: wml_name(interner, "position"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_half_points(interner, half_points);
        value
    }
}

impl FromXml for SignedHalfPointMeasureValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SignedHalfPointMeasureValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_SignedTwipsMeasure` (`w:spacing`, "Character Spacing Adjustment", §17.3.2.35) — a required
/// signed measure in twentieths of a point.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = SignedTwips, accessor = twentieths_of_a_point, required))]
pub struct SignedTwipsMeasureValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SignedTwipsMeasureValue {
    /// Builds a new `w:spacing` of `twentieths_of_a_point`.
    #[must_use]
    pub fn new(interner: &mut Interner, twentieths_of_a_point: SignedTwipsMeasure) -> Self {
        let mut value = Self {
            name: wml_name(interner, "spacing"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_twentieths_of_a_point(interner, twentieths_of_a_point);
        value
    }
}

impl FromXml for SignedTwipsMeasureValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SignedTwipsMeasureValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TextScale` (`w:w`, "Expanded/Compressed Text", §17.3.2.43) — an optional horizontal scale
/// percentage; unlike its 38 siblings' `val`, the schema does not require it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Scale, accessor = percentage))]
pub struct TextScaleValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TextScaleValue {
    /// Builds a new, empty `w:w` — `val` absent until [`TextScaleValue::set_percentage`] states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "w"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for TextScaleValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TextScaleValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// RunProperties (CT_RPr) and its content (EG_RPrBase's 39 members, plus rPrChange)
// -------------------------------------------------------------------------------------------

/// One ordered child of a [`RunProperties`]: `EG_RPrBase`'s 39 members, plus `rPrChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunPropertyContent {
    /// `w:rStyle` (§17.3.2.29, "Referenced Character Style") — this child's own type.
    CharacterStyle(CharacterStyle),
    /// `w:rFonts` (§17.3.2.26, "Run Fonts") — this child's own type.
    Fonts(Fonts),
    /// `w:b` (§17.3.2.1, "Bold") — `CT_OnOff`; see [`Toggle`].
    Bold(Toggle),
    /// `w:bCs` (§17.3.2.2, "Complex Script Bold") — `CT_OnOff`.
    BoldComplexScript(Toggle),
    /// `w:i` (§17.3.2.16, "Italics") — `CT_OnOff`.
    Italic(Toggle),
    /// `w:iCs` (§17.3.2.17, "Complex Script Italics") — `CT_OnOff`.
    ItalicComplexScript(Toggle),
    /// `w:caps` (§17.3.2.5, "Display All Characters As Capital Letters") — `CT_OnOff`.
    AllCapitals(Toggle),
    /// `w:smallCaps` (§17.3.2.33, "Small Caps") — `CT_OnOff`.
    SmallCaps(Toggle),
    /// `w:strike` (§17.3.2.37, "Single Strikethrough") — `CT_OnOff`.
    Strikethrough(Toggle),
    /// `w:dstrike` (§17.3.2.9, "Double Strikethrough") — `CT_OnOff`.
    DoubleStrikethrough(Toggle),
    /// `w:outline` (§17.3.2.23, "Display Character Outline") — `CT_OnOff`.
    Outline(Toggle),
    /// `w:shadow` (§17.3.2.31, "Shadow") — `CT_OnOff`.
    Shadow(Toggle),
    /// `w:emboss` (§17.3.2.13, "Embossing") — `CT_OnOff`.
    Embossing(Toggle),
    /// `w:imprint` (§17.3.2.18, "Imprinting") — `CT_OnOff`.
    Imprinting(Toggle),
    /// `w:noProof` (§17.3.2.21, "Do Not Check Spelling or Grammar") — `CT_OnOff`.
    ProofingExempt(Toggle),
    /// `w:snapToGrid` (§17.3.2.34, "Use Document Grid Settings For Inter-Character Spacing") —
    /// `CT_OnOff`.
    SnapToGrid(Toggle),
    /// `w:vanish` (§17.3.2.41, "Hidden Text") — `CT_OnOff`.
    Hidden(Toggle),
    /// `w:webHidden` (§17.3.2.44, "Web Hidden Text") — `CT_OnOff`.
    WebHidden(Toggle),
    /// `w:color` (§17.3.2.6, "Run Content Color") — this child's own type.
    Color(Color),
    /// `w:spacing` (§17.3.2.35, "Character Spacing Adjustment") — signed twentieths of a point.
    CharacterSpacing(SignedTwipsMeasureValue),
    /// `w:w` (§17.3.2.43, "Expanded/Compressed Text") — a horizontal scale percentage.
    CharacterScale(TextScaleValue),
    /// `w:kern` (§17.3.2.19, "Font Kerning") — a half-point kerning threshold.
    Kerning(HalfPointMeasureValue),
    /// `w:position` (§17.3.2.24, "Vertically Raised or Lowered Text") — a signed half-point offset.
    VerticalOffset(SignedHalfPointMeasureValue),
    /// `w:sz` (§17.3.2.38, "Non-Complex Script Font Size") — a half-point font size.
    FontSize(HalfPointMeasureValue),
    /// `w:szCs` (§17.3.2.39, "Complex Script Font Size") — a half-point font size.
    ComplexScriptFontSize(HalfPointMeasureValue),
    /// `w:highlight` (§17.3.2.15, "Text Highlighting") — this child's own type.
    Highlight(Highlight),
    /// `w:u` (§17.3.2.40, "Underline") — this child's own type.
    Underline(Underline),
    /// `w:effect` (§17.3.2.11, "Animated Text Effect") — this child's own type.
    TextEffect(TextEffect),
    /// `w:bdr` (§17.3.2.4, "Text Border") — this child's own type.
    Border(Border),
    /// `w:shd` (§17.3.2.32, "Run Shading") — this child's own type.
    Shading(Shading),
    /// `w:fitText` (§17.3.2.14, "Manual Run Width") — this child's own type.
    ManualRunWidth(ManualRunWidth),
    /// `w:vertAlign` (§17.3.2.42, "Subscript/Superscript Text") — this child's own type.
    VerticalAlignment(VerticalAlignment),
    /// `w:rtl` (§17.3.2.30, "Right To Left Text") — `CT_OnOff`.
    RightToLeft(Toggle),
    /// `w:cs` (§17.3.2.7, "Use Complex Script Formatting on Run") — `CT_OnOff`.
    ComplexScript(Toggle),
    /// `w:em` (§17.3.2.12, "Emphasis Mark") — this child's own type.
    Emphasis(Emphasis),
    /// `w:lang` (§17.3.2.20, "Languages for Run Content") — this child's own type.
    Languages(Languages),
    /// `w:eastAsianLayout` (§17.3.2.10, "East Asian Typography Settings") — this child's own type.
    EastAsianLayout(EastAsianLayout),
    /// `w:specVanish` (§17.3.2.36, "Paragraph Mark Is Always Hidden") — `CT_OnOff`.
    AlwaysHidden(Toggle),
    /// `w:oMath` (§17.3.2.22, "Office Open XML Math") — `CT_OnOff`, like the other nineteen; **not**
    /// a fortieth special case (see the module's own doc comment).
    Math(Toggle),
    /// `w:rPrChange` (`CT_RPrChange`) — the tracked-change wrapper around a previous `w:rPr`;
    /// MJXOFF-126's own scope, kept opaque here.
    Change(Unmodeled),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_RPr` (`w:rPr`, "Run Properties", §17.3.2.28) — a run's character formatting: `EG_RPrBase`'s
/// 39 members, each independently optional, in no schema-imposed relative order (see the module's own
/// doc comment), then an optional `rPrChange`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct RunProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rStyle", variant = CharacterStyle, ty = CharacterStyle),
        child(local = "rFonts", variant = Fonts, ty = Fonts),
        child(local = "b", variant = Bold, ty = Toggle),
        child(local = "bCs", variant = BoldComplexScript, ty = Toggle),
        child(local = "i", variant = Italic, ty = Toggle),
        child(local = "iCs", variant = ItalicComplexScript, ty = Toggle),
        child(local = "caps", variant = AllCapitals, ty = Toggle),
        child(local = "smallCaps", variant = SmallCaps, ty = Toggle),
        child(local = "strike", variant = Strikethrough, ty = Toggle),
        child(local = "dstrike", variant = DoubleStrikethrough, ty = Toggle),
        child(local = "outline", variant = Outline, ty = Toggle),
        child(local = "shadow", variant = Shadow, ty = Toggle),
        child(local = "emboss", variant = Embossing, ty = Toggle),
        child(local = "imprint", variant = Imprinting, ty = Toggle),
        child(local = "noProof", variant = ProofingExempt, ty = Toggle),
        child(local = "snapToGrid", variant = SnapToGrid, ty = Toggle),
        child(local = "vanish", variant = Hidden, ty = Toggle),
        child(local = "webHidden", variant = WebHidden, ty = Toggle),
        child(local = "color", variant = Color, ty = Color),
        child(local = "spacing", variant = CharacterSpacing, ty = SignedTwipsMeasureValue),
        child(local = "w", variant = CharacterScale, ty = TextScaleValue),
        child(local = "kern", variant = Kerning, ty = HalfPointMeasureValue),
        child(local = "position", variant = VerticalOffset, ty = SignedHalfPointMeasureValue),
        child(local = "sz", variant = FontSize, ty = HalfPointMeasureValue),
        child(local = "szCs", variant = ComplexScriptFontSize, ty = HalfPointMeasureValue),
        child(local = "highlight", variant = Highlight, ty = Highlight),
        child(local = "u", variant = Underline, ty = Underline),
        child(local = "effect", variant = TextEffect, ty = TextEffect),
        child(local = "bdr", variant = Border, ty = Border),
        child(local = "shd", variant = Shading, ty = Shading),
        child(local = "fitText", variant = ManualRunWidth, ty = ManualRunWidth),
        child(local = "vertAlign", variant = VerticalAlignment, ty = VerticalAlignment),
        child(local = "rtl", variant = RightToLeft, ty = Toggle),
        child(local = "cs", variant = ComplexScript, ty = Toggle),
        child(local = "em", variant = Emphasis, ty = Emphasis),
        child(local = "lang", variant = Languages, ty = Languages),
        child(local = "eastAsianLayout", variant = EastAsianLayout, ty = EastAsianLayout),
        child(local = "specVanish", variant = AlwaysHidden, ty = Toggle),
        child(local = "oMath", variant = Math, ty = Toggle),
        child(local = "rPrChange", variant = Change, ty = Unmodeled)
    )]
    content: Vec<RunPropertyContent>,
}

impl RunProperties {
    /// Builds a new, empty `w:rPr` — no properties, ready for [`RunProperties`]'s setters.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "rPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The schema rank of an existing content item: every `EG_RPrBase` member shares rank 0 (the
    /// group is an `xsd:choice`, confirmed against the generated [`RUN_PROPERTIES`] table), `rPrChange`
    /// is rank 1, and anything this crate does not model is unranked so a new property is placed
    /// beside its ranked neighbours rather than after markup this model does not understand.
    fn rank(item: &RunPropertyContent) -> Option<u16> {
        match item {
            RunPropertyContent::Change(_) => Some(1),
            RunPropertyContent::Raw(_) => None,
            _ => Some(0),
        }
    }

    /// Removes the first content item for which `is_target` holds, if any.
    fn remove(&mut self, is_target: impl Fn(&RunPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    /// Inserts `item` (whose wire name is `local`) at its schema rank among the existing content —
    /// see [`RunProperties::rank`].
    fn insert(&mut self, local: &str, item: RunPropertyContent) {
        let at = RUN_PROPERTIES.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Replaces (or inserts, at its schema rank, or removes when `value` is `None`) the content item
    /// this property occupies — the one write primitive every whole-value setter in this type uses.
    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&RunPropertyContent) -> bool,
        value: Option<RunPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }
}

/// Declares one `CT_OnOff`-shaped property: a tri-state getter (`None` absent, `Some(true)`/
/// `Some(false)` from the element's own [`Toggle::value`]) and a whole-value setter.
macro_rules! toggle_property {
    ($getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<bool>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    RunPropertyContent::$variant(toggle) => Some(toggle),
                    _ => None,
                })
                .map(|toggle| toggle.value(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value`.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<bool>) {
            let is_target = |item: &RunPropertyContent| matches!(item, RunPropertyContent::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let mut toggle = Toggle::new(interner, $local);
                    toggle.set_value(interner, Some(value));
                    self.set($local, is_target, Some(RunPropertyContent::$variant(toggle)));
                }
            }
        }
    };
}

/// Declares one `CT_HpsMeasure`-shaped property (`sz`, `szCs`, `kern` — see [`HalfPointMeasureValue`]
/// for why the three share one wrapper type): a fallible flattened getter and a whole-value setter
/// that builds the wrapper under its own wire name, mirroring [`toggle_property`].
macro_rules! half_point_property {
    ($getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<HalfPointMeasure>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    RunPropertyContent::$variant(value) => Some(value),
                    _ => None,
                })
                .map(|value| value.half_points(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value` half-points.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<HalfPointMeasure>) {
            let is_target = |item: &RunPropertyContent| matches!(item, RunPropertyContent::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let element = HalfPointMeasureValue::new(interner, $local, value);
                    self.set($local, is_target, Some(RunPropertyContent::$variant(element)));
                }
            }
        }
    };
}

/// Declares one whole-value property: a getter borrowing the typed child if present, and a setter
/// that replaces, inserts (at its schema rank) or removes it.
macro_rules! value_property {
    ($getter:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                RunPropertyContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` replaces \
            or inserts it at its schema rank.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let is_target =
                |item: &RunPropertyContent| matches!(item, RunPropertyContent::$variant(_));
            self.set($local, is_target, value.map(RunPropertyContent::$variant));
        }
    };
}

impl RunProperties {
    toggle_property!(
        bold,
        set_bold,
        Bold,
        "b",
        "`w:b` — whether the run is bold."
    );
    toggle_property!(
        bold_complex_script,
        set_bold_complex_script,
        BoldComplexScript,
        "bCs",
        "`w:bCs` — whether the run is bold, for complex-script text."
    );
    toggle_property!(
        italic,
        set_italic,
        Italic,
        "i",
        "`w:i` — whether the run is italic."
    );
    toggle_property!(
        italic_complex_script,
        set_italic_complex_script,
        ItalicComplexScript,
        "iCs",
        "`w:iCs` — whether the run is italic, for complex-script text."
    );
    toggle_property!(
        all_capitals,
        set_all_capitals,
        AllCapitals,
        "caps",
        "`w:caps` — whether every character displays as a capital letter."
    );
    toggle_property!(
        small_caps,
        set_small_caps,
        SmallCaps,
        "smallCaps",
        "`w:smallCaps` — whether lowercase characters display as small capitals."
    );
    toggle_property!(
        strikethrough,
        set_strikethrough,
        Strikethrough,
        "strike",
        "`w:strike` — whether the run has a single strikethrough."
    );
    toggle_property!(
        double_strikethrough,
        set_double_strikethrough,
        DoubleStrikethrough,
        "dstrike",
        "`w:dstrike` — whether the run has a double strikethrough."
    );
    toggle_property!(
        outline,
        set_outline,
        Outline,
        "outline",
        "`w:outline` — whether the run displays as an outline of its characters."
    );
    toggle_property!(
        shadow,
        set_shadow,
        Shadow,
        "shadow",
        "`w:shadow` — whether the run has a shadow."
    );
    toggle_property!(
        embossing,
        set_embossing,
        Embossing,
        "emboss",
        "`w:emboss` — whether the run displays with an embossed effect."
    );
    toggle_property!(
        imprinting,
        set_imprinting,
        Imprinting,
        "imprint",
        "`w:imprint` — whether the run displays with an imprinted (engraved) effect."
    );
    toggle_property!(
        proofing_exempt,
        set_proofing_exempt,
        ProofingExempt,
        "noProof",
        "`w:noProof` — whether the run is exempt from spelling and grammar checking."
    );
    toggle_property!(
        snap_to_grid,
        set_snap_to_grid,
        SnapToGrid,
        "snapToGrid",
        "`w:snapToGrid` — whether the run's inter-character spacing follows the document grid."
    );
    toggle_property!(
        hidden,
        set_hidden,
        Hidden,
        "vanish",
        "`w:vanish` — whether the run is hidden text."
    );
    toggle_property!(
        web_hidden,
        set_web_hidden,
        WebHidden,
        "webHidden",
        "`w:webHidden` — whether the run is hidden when the document displays as a web page."
    );
    toggle_property!(
        right_to_left,
        set_right_to_left,
        RightToLeft,
        "rtl",
        "`w:rtl` — whether the run displays right-to-left."
    );
    toggle_property!(
        complex_script,
        set_complex_script,
        ComplexScript,
        "cs",
        "`w:cs` — whether the run uses complex-script formatting."
    );
    toggle_property!(
        always_hidden,
        set_always_hidden,
        AlwaysHidden,
        "specVanish",
        "`w:specVanish` — whether this run's paragraph mark is always hidden, even when formatting \
         marks otherwise display."
    );
    toggle_property!(
        math,
        set_math,
        Math,
        "oMath",
        "`w:oMath` — whether the run is typeset as Office Open XML Math."
    );

    value_property!(
        character_style,
        set_character_style,
        CharacterStyle,
        CharacterStyle,
        "rStyle",
        "`w:rStyle` — the character style this run refers to."
    );
    value_property!(
        fonts,
        set_fonts,
        Fonts,
        Fonts,
        "rFonts",
        "`w:rFonts` — the fonts this run uses."
    );
    value_property!(
        color,
        set_color,
        Color,
        Color,
        "color",
        "`w:color` — this run's text colour."
    );
    value_property!(
        character_spacing,
        set_character_spacing,
        CharacterSpacing,
        SignedTwipsMeasureValue,
        "spacing",
        "`w:spacing` — the character spacing adjustment, in twentieths of a point."
    );
    value_property!(
        character_scale,
        set_character_scale,
        CharacterScale,
        TextScaleValue,
        "w",
        "`w:w` — the horizontal character scale, as a percentage."
    );
    half_point_property!(
        kerning,
        set_kerning,
        Kerning,
        "kern",
        "`w:kern` — the font-size threshold, in half-points, above which kerning applies."
    );
    value_property!(
        vertical_offset,
        set_vertical_offset,
        VerticalOffset,
        SignedHalfPointMeasureValue,
        "position",
        "`w:position` — how far the run is raised or lowered from the baseline, in half-points."
    );
    half_point_property!(
        font_size,
        set_font_size,
        FontSize,
        "sz",
        "`w:sz` — the run's font size, in half-points."
    );
    half_point_property!(
        complex_script_font_size,
        set_complex_script_font_size,
        ComplexScriptFontSize,
        "szCs",
        "`w:szCs` — the run's font size for complex-script text, in half-points."
    );
    value_property!(
        highlight,
        set_highlight,
        Highlight,
        Highlight,
        "highlight",
        "`w:highlight` — this run's text-highlight colour."
    );
    value_property!(
        underline,
        set_underline,
        Underline,
        Underline,
        "u",
        "`w:u` — this run's underline."
    );
    value_property!(
        text_effect,
        set_text_effect,
        TextEffect,
        TextEffect,
        "effect",
        "`w:effect` — this run's animated text effect."
    );
    value_property!(
        border,
        set_border,
        Border,
        Border,
        "bdr",
        "`w:bdr` — this run's character border."
    );
    value_property!(
        shading,
        set_shading,
        Shading,
        Shading,
        "shd",
        "`w:shd` — this run's shading."
    );
    value_property!(
        manual_run_width,
        set_manual_run_width,
        ManualRunWidth,
        ManualRunWidth,
        "fitText",
        "`w:fitText` — this run's manually fitted width."
    );
    value_property!(
        vertical_alignment,
        set_vertical_alignment,
        VerticalAlignment,
        VerticalAlignment,
        "vertAlign",
        "`w:vertAlign` — this run's subscript/superscript position."
    );
    value_property!(
        emphasis_mark,
        set_emphasis_mark,
        Emphasis,
        Emphasis,
        "em",
        "`w:em` — this run's emphasis mark."
    );
    value_property!(
        languages,
        set_languages,
        Languages,
        Languages,
        "lang",
        "`w:lang` — the languages to check this run's text against."
    );
    value_property!(
        east_asian_layout,
        set_east_asian_layout,
        EastAsianLayout,
        EastAsianLayout,
        "eastAsianLayout",
        "`w:eastAsianLayout` — this run's East Asian typography settings."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_core::RawDocument;
    use mjx_xml::fidelity;

    use super::super::body::Run;

    const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
        let doc = fidelity::parse(fragment).expect("fragment parses");
        let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml succeeds");
        (typed, doc)
    }

    #[track_caller]
    fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
        doc.root = typed.to_xml(&mut doc.interner);
        let out = fidelity::serialize_to_vec(&doc);
        assert_eq!(
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(expected),
            "round-trip byte mismatch"
        );
    }

    /// `RunProperties::to_xml` always fully rebuilds (`ToXml`'s own doc comment: "`to_xml` rebuilds
    /// every element `from_xml` looked at"), so — unlike a fixture round-trip through `write_back`,
    /// which can copy an untouched element's original bytes verbatim regardless of whether the
    /// rebuild logic is correct — this discriminates a mutation that always emits self-closing: hard
    /// coding `empty = true` anywhere in this type's path would turn the second assertion red.
    #[test]
    fn run_properties_preserves_self_closed_and_separate_end_tag_emptiness() {
        let self_closed = format!(r#"<w:rPr xmlns:w="{W}"/>"#).into_bytes();
        let (rpr, doc): (RunProperties, _) = parse_typed(&self_closed);
        assert!(rpr.content.is_empty());
        assert_round_trips(&rpr, doc, &self_closed);

        let separate_end_tag = format!(r#"<w:rPr xmlns:w="{W}"></w:rPr>"#).into_bytes();
        let (rpr, doc): (RunProperties, _) = parse_typed(&separate_end_tag);
        assert!(rpr.content.is_empty());
        assert_round_trips(&rpr, doc, &separate_end_tag);
    }

    /// The same three states, wired through [`Run`] — proving *this child's own* addition (the `rPr`
    /// child slot on `CT_R`, absent before this child existed) carries the distinction, not just
    /// `RunProperties` in isolation.
    #[test]
    fn run_preserves_the_rpr_emptiness_state_and_its_absence() {
        let cases = [
            (
                "self-closed",
                format!(r#"<w:r xmlns:w="{W}"><w:rPr/><w:t>x</w:t></w:r>"#),
            ),
            (
                "separate end tag",
                format!(r#"<w:r xmlns:w="{W}"><w:rPr></w:rPr><w:t>x</w:t></w:r>"#),
            ),
            (
                "absent",
                format!(r#"<w:r xmlns:w="{W}"><w:t>x</w:t></w:r>"#),
            ),
        ];
        for (label, fragment) in cases {
            let bytes = fragment.into_bytes();
            let (run, doc): (Run, _) = parse_typed(&bytes);
            if label == "absent" {
                assert!(
                    run.run_properties().is_none(),
                    "{label}: run_properties() must be None"
                );
            } else {
                assert!(
                    run.run_properties().is_some(),
                    "{label}: run_properties() must be Some"
                );
            }
            assert_round_trips(&run, doc, &bytes);
        }
    }

    /// The `ST_OnOff` default, sourced from ECMA-376 Part 1's own prose for `b`: present with no
    /// `val` means on; an explicit `val="0"` means off, not absent — the module's own doc comment
    /// names the exact sentence.
    #[test]
    fn toggle_default_is_on_but_an_explicit_off_stays_off() {
        let (present_no_val, doc) =
            parse_typed::<Toggle>(format!(r#"<w:b xmlns:w="{W}"/>"#).as_bytes());
        assert_eq!(present_no_val.value(&doc.interner), Ok(true));

        let (explicit_on, doc) =
            parse_typed::<Toggle>(format!(r#"<w:b xmlns:w="{W}" w:val="1"/>"#).as_bytes());
        assert_eq!(explicit_on.value(&doc.interner), Ok(true));

        let (explicit_off, doc) =
            parse_typed::<Toggle>(format!(r#"<w:b xmlns:w="{W}" w:val="0"/>"#).as_bytes());
        assert_eq!(explicit_off.value(&doc.interner), Ok(false));
    }

    /// [`RunProperties::bold`] keeps the same three states distinguishable at the container level:
    /// `None` (no `w:b` at all) is not the same fact as `Some(false)` (`w:b w:val="0"`, explicitly
    /// off) or `Some(true)` (present, on).
    #[test]
    fn bold_distinguishes_absent_present_on_and_explicit_off() {
        let (no_b, doc) = parse_typed::<RunProperties>(
            format!(r#"<w:rPr xmlns:w="{W}"><w:i/></w:rPr>"#).as_bytes(),
        );
        assert_eq!(no_b.bold(&doc.interner), Ok(None));

        let (on, doc) = parse_typed::<RunProperties>(
            format!(r#"<w:rPr xmlns:w="{W}"><w:b/></w:rPr>"#).as_bytes(),
        );
        assert_eq!(on.bold(&doc.interner), Ok(Some(true)));

        let (off, doc) = parse_typed::<RunProperties>(
            format!(r#"<w:rPr xmlns:w="{W}"><w:b w:val="0"/></w:rPr>"#).as_bytes(),
        );
        assert_eq!(off.bold(&doc.interner), Ok(Some(false)));
    }

    /// [`RunProperties::set_bold`] writes the explicit canonical form (`w:val="false"`) rather than
    /// relying on the default, and `None` removes `w:b` entirely — proving the setter, not just the
    /// getter, keeps the explicit-off state distinct from absent.
    #[test]
    fn set_bold_writes_the_explicit_form_and_none_removes_the_element() {
        let mut interner = Interner::new();
        let mut rpr = RunProperties::new(&mut interner);
        assert_eq!(rpr.bold(&interner), Ok(None));

        rpr.set_bold(&mut interner, Some(false));
        assert_eq!(rpr.bold(&interner), Ok(Some(false)));
        let element = rpr.to_xml(&mut interner);
        let bold_val = element.children.iter().find_map(|child| match child {
            RawNode::Element(el) if interner.resolve(el.name.local) == "b" => el
                .attributes
                .iter()
                .find(|attr| interner.resolve(attr.name.local) == "val")
                .map(|attr| String::from_utf8_lossy(&attr.value).into_owned()),
            _ => None,
        });
        assert_eq!(
            bold_val.as_deref(),
            Some("false"),
            "an explicit false must be written, not omitted"
        );

        rpr.set_bold(&mut interner, None);
        assert_eq!(rpr.bold(&interner), Ok(None));
        assert!(
            rpr.content.is_empty(),
            "removing the only property must leave rPr with no content"
        );
    }

    /// Setting one property (`w:color`) must not disturb another already-present property (`w:b`) in
    /// the same `w:rPr` — the same edit-isolation contract
    /// `crates/mjx-docx/tests/run_properties.rs`'s fixture-level test proves across whole parts,
    /// proved here at the type level: [`RunProperties::insert`] places the new child without
    /// rebuilding or reordering the existing one.
    #[test]
    fn setting_one_property_leaves_another_already_present_property_untouched() {
        let (mut rpr, doc) = parse_typed::<RunProperties>(
            format!(r#"<w:rPr xmlns:w="{W}"><w:b/></w:rPr>"#).as_bytes(),
        );
        let mut interner = doc.interner;
        rpr.set_color(Some(Color::new(&mut interner, "FF0000")));
        assert_eq!(
            rpr.bold(&interner),
            Ok(Some(true)),
            "w:b must be unaffected"
        );
        assert_eq!(
            rpr.color().map(|c| c.hex_value(&interner)),
            Some(Ok(HexadecimalColor::from_wire("FF0000")))
        );
    }
}
