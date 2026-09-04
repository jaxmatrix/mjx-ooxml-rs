//! `word/fontTable.xml` (`CT_FontsList`, the `w:fonts` root, §17.15.2.24) — MJXOFF-136's own file:
//! [`FontTable`] and one font's own entry, [`Font`].
//!
//! # Embedded fonts are opaque bytes with an opaque obfuscation key
//!
//! `w:embedRegular`/`w:embedBold`/`w:embedItalic`/`w:embedBoldItalic` (`CT_FontRel`) each name a
//! relationship to a binary `fontdata` part and, when the embedded font is obfuscated (Word's own
//! "font embedding" feature XORs the first 32 bytes against a GUID derived from `fontKey`), carry
//! that key as a plain GUID string. **Neither this file nor `mjx-opc` ever de-obfuscates, re-encodes
//! or otherwise interprets the payload or the key** — [`FontRel::font_key`] hands back the GUID
//! string exactly as written, and the binary part itself is reached (and preserved byte-identical on
//! an untouched save) through the ordinary [`mjx_opc::Package`] part/relationship machinery every
//! other binary part in this workspace already uses (an embedded picture, for one).

use mjx_ooxml_core::{
    AttributeError, Enumeration, FromXml, FromXmlError, Interner, RawAttribute, RawElement,
    RawName, RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::FONT;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::FontFamily as FontFamilyKind;
use mjx_ooxml_types::wordprocessingml::FontPitch;

use super::body::wml_name;
use super::run_properties::{ThemeHexDigit, Toggle};
use super::styles::{LongHex, StyleString};

// `ThemeHexDigit` (`ST_UcharHexNumber`, `run_properties.rs`) covers `w:charset`; `LongHex`
// (`ST_LongHexNumber`, `styles.rs`) covers `w:sig`'s four `usbN`/two `csbN` bitmasks. Both are
// generated wire-string wrappers reused rather than restated.

// =================================================================================================
// Attribute-only leaves: CT_Panose, CT_FontFamily, CT_Pitch.
// =================================================================================================

/// `w:panose1` (`CT_Panose`, §17.15.2.29) — the ten-byte PANOSE type-classification number, as its
/// own hex string (opaque; never decoded byte-by-byte here).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = hex, required))]
pub struct Panose {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Panose {
    /// Builds a new `w:panose1` carrying `hex` (twenty hex digits) verbatim.
    #[must_use]
    pub fn new(interner: &mut Interner, hex: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "panose1"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_hex(interner, hex);
        item
    }
}

impl FromXml for Panose {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Panose {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:family` (`CT_FontFamily`, §17.15.2.17) — the font's own family classification (roman, swiss,
/// modern, script, decorative, or unknown).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<FontFamilyKind>, accessor = kind, required))]
pub struct FontFamily {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FontFamily {
    /// Builds a new `w:family` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: FontFamilyKind) -> Self {
        let mut item = Self {
            name: wml_name(interner, "family"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for FontFamily {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FontFamily {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:pitch` (`CT_Pitch`, §17.15.2.30) — whether the font is fixed-pitch, variable-pitch, or
/// default.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<FontPitch>, accessor = kind, required))]
pub struct Pitch {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Pitch {
    /// Builds a new `w:pitch` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: FontPitch) -> Self {
        let mut item = Self {
            name: wml_name(interner, "pitch"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for Pitch {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Pitch {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:charset` (`CT_Charset`, §17.15.2.7) — the font's own legacy character-set byte, plus the
/// modern character-encoding name Word writes alongside it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = ThemeHexDigit, accessor = legacy_byte))]
#[xml(attribute(local = "characterSet", prefix = "w", codec = TextCodec, accessor = character_set))]
pub struct Charset {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Charset {
    /// Builds a new, empty `w:charset` — both attributes absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "charset"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for Charset {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Charset {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:sig` (`CT_FontSig`, §17.15.2.32) — the font's own Unicode/codepage signature: four "USB"
/// bitmasks (which Unicode subranges the font covers) and two "CSB" bitmasks (which codepages).
/// Every bitmask is opaque hex, never decoded bit-by-bit here.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "usb0", prefix = "w", codec = LongHex, accessor = unicode_subrange_0, required))]
#[xml(attribute(local = "usb1", prefix = "w", codec = LongHex, accessor = unicode_subrange_1, required))]
#[xml(attribute(local = "usb2", prefix = "w", codec = LongHex, accessor = unicode_subrange_2, required))]
#[xml(attribute(local = "usb3", prefix = "w", codec = LongHex, accessor = unicode_subrange_3, required))]
#[xml(attribute(local = "csb0", prefix = "w", codec = LongHex, accessor = codepage_subrange_0, required))]
#[xml(attribute(local = "csb1", prefix = "w", codec = LongHex, accessor = codepage_subrange_1, required))]
pub struct FontSignature {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FontSignature {
    /// Builds a new, empty `w:sig` — every attribute is `required`, so a caller should set all six
    /// before writing this out.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "sig"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for FontSignature {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FontSignature {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:embedRegular`/`w:embedBold`/`w:embedItalic`/`w:embedBoldItalic` (`CT_FontRel`, §17.15.2.16) —
/// one embedded-font relationship: `CT_Rel`'s own `r:id` plus an optional obfuscation key and
/// whether the payload is a subset. See the module's own doc comment: the payload and the key are
/// both opaque here.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "r", codec = TextCodec, accessor = relationship_id, required))]
#[xml(attribute(local = "fontKey", prefix = "w", codec = TextCodec, accessor = font_key))]
#[xml(attribute(local = "subsetted", prefix = "w", codec = OnOff, accessor = subsetted))]
pub struct FontRel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FontRel {
    /// Builds a new `local` element (`"embedRegular"`, `"embedBold"`, `"embedItalic"` or
    /// `"embedBoldItalic"`) pointing at `relationship_id`; the payload itself, and its `fontKey`
    /// obfuscation key, are the caller's own [`mjx_opc::Package`] job to relate and never
    /// re-encoded here.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, relationship_id: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_relationship_id(interner, relationship_id);
        item
    }
}

impl FromXml for FontRel {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FontRel {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// CT_Font, CT_FontsList
// =================================================================================================

/// `w:font` (`CT_Font`, §17.15.2.23) — one font table entry: identity, classification, and (when
/// present) its own embedded-font relationships. Resolving the font a run names (`w:rFonts`) to its
/// entry here is what C8's effective-font reading needs.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = font_name, required))]
pub struct Font {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    content: Vec<FontContent>,
}

/// One child of [`Font`]: `CT_Font`'s own eleven, ranked from the generated [`FONT`] table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontContent {
    /// `w:altName`.
    AlternateName(StyleString),
    /// `w:panose1`.
    Panose(Panose),
    /// `w:charset`.
    Charset(Charset),
    /// `w:family`.
    Family(FontFamily),
    /// `w:notTrueType`.
    NotTrueType(Toggle),
    /// `w:pitch`.
    Pitch(Pitch),
    /// `w:sig`.
    Signature(FontSignature),
    /// `w:embedRegular`.
    EmbedRegular(FontRel),
    /// `w:embedBold`.
    EmbedBold(FontRel),
    /// `w:embedItalic`.
    EmbedItalic(FontRel),
    /// `w:embedBoldItalic`.
    EmbedBoldItalic(FontRel),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl Font {
    /// Builds a new `w:font` named `font_name`, every other child absent until a setter states
    /// one.
    #[must_use]
    pub fn new(interner: &mut Interner, font_name: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "font"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        item.set_font_name(interner, font_name);
        item
    }

    fn local(item: &FontContent) -> Option<&'static str> {
        Some(match item {
            FontContent::AlternateName(_) => "altName",
            FontContent::Panose(_) => "panose1",
            FontContent::Charset(_) => "charset",
            FontContent::Family(_) => "family",
            FontContent::NotTrueType(_) => "notTrueType",
            FontContent::Pitch(_) => "pitch",
            FontContent::Signature(_) => "sig",
            FontContent::EmbedRegular(_) => "embedRegular",
            FontContent::EmbedBold(_) => "embedBold",
            FontContent::EmbedItalic(_) => "embedItalic",
            FontContent::EmbedBoldItalic(_) => "embedBoldItalic",
            FontContent::Raw(_) => return None,
        })
    }

    fn rank(item: &FontContent) -> Option<u16> {
        Self::local(item).and_then(|local| FONT.rank_of(None, local))
    }

    fn remove(&mut self, is_target: impl Fn(&FontContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: FontContent) {
        let at = FONT.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&FontContent) -> bool,
        value: Option<FontContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    super::property_macros::value_property!(
        FontContent,
        alternate_name,
        set_alternate_name,
        AlternateName,
        StyleString,
        "altName",
        "`w:altName`."
    );
    super::property_macros::value_property!(
        FontContent,
        panose,
        set_panose,
        Panose,
        Panose,
        "panose1",
        "`w:panose1`."
    );
    super::property_macros::value_property!(
        FontContent,
        charset,
        set_charset,
        Charset,
        Charset,
        "charset",
        "`w:charset`."
    );
    super::property_macros::value_property!(
        FontContent,
        family,
        set_family,
        Family,
        FontFamily,
        "family",
        "`w:family`."
    );
    super::property_macros::toggle_property!(
        FontContent,
        not_true_type,
        set_not_true_type,
        NotTrueType,
        "notTrueType",
        "`w:notTrueType`."
    );
    super::property_macros::value_property!(
        FontContent,
        pitch,
        set_pitch,
        Pitch,
        Pitch,
        "pitch",
        "`w:pitch`."
    );
    super::property_macros::value_property!(
        FontContent,
        signature,
        set_signature,
        Signature,
        FontSignature,
        "sig",
        "`w:sig`."
    );
    super::property_macros::value_property!(FontContent, embed_regular, set_embed_regular, EmbedRegular, FontRel, "embedRegular", "`w:embedRegular` — the embedded regular-weight font's own relationship. Payload and obfuscation key are opaque (see the module's own doc comment).");
    super::property_macros::value_property!(
        FontContent,
        embed_bold,
        set_embed_bold,
        EmbedBold,
        FontRel,
        "embedBold",
        "`w:embedBold`."
    );
    super::property_macros::value_property!(
        FontContent,
        embed_italic,
        set_embed_italic,
        EmbedItalic,
        FontRel,
        "embedItalic",
        "`w:embedItalic`."
    );
    super::property_macros::value_property!(
        FontContent,
        embed_bold_italic,
        set_embed_bold_italic,
        EmbedBoldItalic,
        FontRel,
        "embedBoldItalic",
        "`w:embedBoldItalic`."
    );
}

impl FromXml for Font {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut content = Vec::with_capacity(element.children.len());
        for node in &element.children {
            let RawNode::Element(child) = node else {
                content.push(FontContent::Raw(node.clone()));
                continue;
            };
            let namespace = child.name.namespace.map(|s| interner.resolve(s));
            let is_wml = namespace == Some(mjx_ooxml_types::namespaces::WML.transitional)
                || namespace == mjx_ooxml_types::namespaces::WML.strict;
            let local = interner.resolve(child.name.local);
            let item = if is_wml && local == "altName" {
                FontContent::AlternateName(StyleString::from_xml(child, interner)?)
            } else if is_wml && local == "panose1" {
                FontContent::Panose(Panose::from_xml(child, interner)?)
            } else if is_wml && local == "charset" {
                FontContent::Charset(Charset::from_xml(child, interner)?)
            } else if is_wml && local == "family" {
                FontContent::Family(FontFamily::from_xml(child, interner)?)
            } else if is_wml && local == "notTrueType" {
                FontContent::NotTrueType(Toggle::from_xml(child, interner)?)
            } else if is_wml && local == "pitch" {
                FontContent::Pitch(Pitch::from_xml(child, interner)?)
            } else if is_wml && local == "sig" {
                FontContent::Signature(FontSignature::from_xml(child, interner)?)
            } else if is_wml && local == "embedRegular" {
                FontContent::EmbedRegular(FontRel::from_xml(child, interner)?)
            } else if is_wml && local == "embedBold" {
                FontContent::EmbedBold(FontRel::from_xml(child, interner)?)
            } else if is_wml && local == "embedItalic" {
                FontContent::EmbedItalic(FontRel::from_xml(child, interner)?)
            } else if is_wml && local == "embedBoldItalic" {
                FontContent::EmbedBoldItalic(FontRel::from_xml(child, interner)?)
            } else {
                FontContent::Raw(node.clone())
            };
            content.push(item);
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            content,
        })
    }
}

impl ToXml for Font {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                FontContent::AlternateName(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::Panose(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::Charset(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::Family(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::NotTrueType(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::Pitch(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::Signature(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::EmbedRegular(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::EmbedBold(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::EmbedItalic(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::EmbedBoldItalic(value) => RawNode::Element(value.to_xml(interner)),
                FontContent::Raw(node) => node.clone(),
            })
            .collect::<Vec<_>>();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `word/fontTable.xml`'s own root (`w:fonts`, `CT_FontsList`, §17.15.2.24) — the font table part.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FontTable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "font", variant = Entry, ty = Font))]
    content: Vec<FontTableContent>,
}

/// One child of [`FontTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontTableContent {
    /// `w:font` — repeatable.
    Entry(Font),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FontTable {
    /// Builds a new, empty `word/fontTable.xml` root.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "fonts"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every font entry, in document order.
    pub fn fonts(&self) -> impl Iterator<Item = &Font> {
        self.content.iter().filter_map(|item| match item {
            FontTableContent::Entry(value) => Some(value),
            FontTableContent::Raw(_) => None,
        })
    }

    /// The font entry named `name` (its own `w:name` attribute), if one exists — what resolving a
    /// run's `w:rFonts` reference to its table entry needs (C8).
    #[must_use]
    pub fn font(&self, interner: &Interner, name: &str) -> Option<&Font> {
        self.fonts()
            .find(|font| font.font_name(interner).ok().as_deref() == Some(name))
    }

    /// Appends `font` — the schema imposes no order among `w:font` siblings.
    pub fn add_font(&mut self, font: Font) {
        self.content.push(FontTableContent::Entry(font));
        self.empty = false;
    }
}
