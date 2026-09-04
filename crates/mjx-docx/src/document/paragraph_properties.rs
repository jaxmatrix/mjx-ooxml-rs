//! `w:pPr` (`CT_PPr`) — a paragraph's own properties — and the 33 `CT_PPrBase` members it shares
//! with the numbering levels and styles that build on the same base (MJXOFF-101, MJXOFF-109).
//!
//! # `CT_PPrBase`'s 33 children
//!
//! `pStyle`, `keepNext`, `keepLines`, `pageBreakBefore`, `framePr`, `widowControl`, `numPr`,
//! `suppressLineNumbers`, `pBdr`, `shd`, `tabs`, `suppressAutoHyphens`, `kinsoku`, `wordWrap`,
//! `overflowPunct`, `topLinePunct`, `autoSpaceDE`, `autoSpaceDN`, `bidi`, `adjustRightInd`,
//! `snapToGrid`, `spacing`, `ind`, `contextualSpacing`, `mirrorIndents`, `suppressOverlap`, `jc`,
//! `textDirection`, `textAlignment`, `textboxTightWrap`, `outlineLvl`, `divId`, `cnfStyle` —
//! thirty-three, confirmed against `wml.xsd`'s own `CT_PPrBase` sequence and against the generated
//! `PARAGRAPH_PROPERTIES_BASE` child-order table (MJXOFF-90), which gives them ranks 0 through 32.
//! [`ParagraphProperties`] (`CT_PPr`, `w:pPr`) extends that with `rPr` (`CT_ParaRPr`, rank 33),
//! `sectPr` (rank 34) and `pPrChange` (rank 35) — the base's particle comes before the derived type's
//! own, exactly as MJXOFF-90's `complexContent` fix produces, so this module *consumes*
//! `PARAGRAPH_PROPERTIES` (`CT_PPr`'s own generated table) rather than reasoning about the splice by
//! hand.
//!
//! One correction to this child's own brief: `kinsoku` (child #13 above) is **`CT_OnOff`** in
//! `wml.xsd`, not the two-attribute `CT_Kinsoku` complex type the ticket's "Complex types" bullet
//! names — that type is `w:noLineBreaksAfter`/`w:noLineBreaksBefore` inside document settings, an
//! element with no relationship to paragraph properties. `w:kinsoku` here is a plain toggle ("Use
//! East Asian Typography Rules for First and Last Character per Line", §17.3.1.16), modelled with
//! [`super::run_properties::Toggle`] like its seventeen sibling switches. Two more of the ticket's
//! named complex types have no home here: `CT_DecimalNumberOrPrecent` is `w:summaryLength` (a
//! glossary-document setting), not any `CT_PPrBase` child; and `CT_ParaRPrOriginal` is reachable only
//! through `w:pPrChange/w:rPr`, which is `w:pPrChange` semantics — MJXOFF-126's scope, kept opaque
//! here.
//!
//! # The paragraph mark is not a run
//!
//! `w:pPr/w:rPr` (`CT_ParaRPr`, [`ParagraphMarkRunProperties`]) describes **the pilcrow itself** —
//! the formatting a new paragraph typed after this one inherits — never a run's own `w:rPr`
//! ([`super::run_properties::RunProperties`], MJXOFF-94). The two are different Rust types, reached
//! through different accessors ([`Paragraph::paragraph_mark_properties`](super::body::Paragraph::paragraph_mark_properties)
//! versus [`Run::run_properties`](super::body::Run::run_properties)), so a caller cannot pass one
//! where the other is expected. `CT_ParaRPr` *is* `EG_RPrBase` (the same 39 members
//! [`super::run_properties::RunProperties`] carries) plus `EG_ParaRPrTrackChanges` (`ins`, `del`,
//! `moveFrom`, `moveTo` — MJXOFF-126's own semantics, given their field now rather than dropped to
//! the unknown bucket) and `rPrChange` (MJXOFF-126 again). Every one of the 39 formatting leaf types
//! — [`super::run_properties::Toggle`], [`super::run_properties::Fonts`],
//! [`super::run_properties::Color`], … — is **reused directly from `run_properties.rs`**, not
//! restated: this module only adds the wiring a second element needs (its own content enum and
//! accessors), never a second definition of what a `w:b` or a `w:rFonts` is.
//!
//! # `w:line` cannot be read without `w:lineRule`
//!
//! [`Spacing`]'s `line`/`lineRule` pair is the subtle one: `w:line`'s integer means 240ths of a line
//! when `w:lineRule="auto"` and twentieths of a point (twips) when it is `exact`/`atLeast` — the same
//! wire value is two different physical quantities depending on a sibling attribute. There is
//! deliberately no `Spacing::line`/`Spacing::line_rule` accessor pair; [`Spacing::line_spacing`] is
//! the only way to read either, and it always returns both together as one [`LineSpacing`] — see that
//! method's own doctest for the proof.
//!
//! # `w:ind`'s logical and physical spellings
//!
//! [`Indentation`] preserves whichever spelling (`w:start`/`w:end`, bidi-aware and Strict-compatible,
//! or the older, Transitional-only `w:left`/`w:right`) a file used, rather than normalizing on write
//! — all four (plus their `…Chars` siblings) stay independently readable and settable. ECMA-376 Part
//! 1's own prose documents an explicit override for the `…Chars` siblings ("if `endChars` is
//! specified, `end` is ignored", §17.3.1.12) but says nothing about `start` versus `left` directly;
//! Annex M records that `start`/`end`/`startChars`/`endChars` were *added* to `CT_Ind` for Strict
//! compatibility while `left`/`right` are the older, Transitional-only spelling that predates them.
//! Given that evidence but no literal spec sentence, [`Indentation::leading_edge`]/
//! [`Indentation::trailing_edge`] **prefer the logical spelling when both are present** — the newer,
//! Strict-compatible one — and document that choice; a caller who disagrees still has
//! `start`/`left`/`end`/`right` individually.

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, Number, RawAttribute, RawElement, RawName, RawNode, Text as TextCodec,
    ToXml,
};
use mjx_ooxml_types::child_order::{
    NUMBERING_PROPERTIES, PARAGRAPH_BORDERS, PARAGRAPH_MARK_RUN_PROPERTIES, PARAGRAPH_PROPERTIES,
};
use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    ConditionalFormattingBitmask, DecimalNumber, DropCap, HalfPointMeasure, HeightRule,
    HorizontalAnchor, Justification, LineSpacingRule, SignedTwipsMeasure, TabStopLeader,
    TabStopType, TextBoxTightWrap, TextFlowDirection, TextFrameWrapping, VerticalAnchor,
    VerticalTextAlignment,
};

use super::body::{wml_name, Unmodeled};
use super::run_properties::{
    Border, CharacterStyle, Color, EastAsianLayout, Emphasis, Fonts, HalfPointMeasureValue,
    Highlight, Languages, ManualRunWidth, Shading, SignedHalfPointMeasureValue, SignedTwips,
    SignedTwipsMeasureValue, TextEffect, TextScaleValue, Toggle, Twips, Underline,
    VerticalAlignment,
};

// -------------------------------------------------------------------------------------------
// A custom attribute codec this module needs beyond what `run_properties.rs` already declared.
// -------------------------------------------------------------------------------------------

/// `ST_Cnf` (a twelve-character `[01]*` bitmask) as an attribute value — the wire string itself,
/// preserved exactly, in the same "never reject on read" shape as
/// [`super::run_properties::HexColor`] for a generated wire-string wrapper.
#[derive(Debug)]
pub struct ConditionalFormattingBits;

impl AttributeCodec for ConditionalFormattingBits {
    type Value<'a> = ConditionalFormattingBitmask;
    type Input<'a> = ConditionalFormattingBitmask;

    fn decode<'a>(
        raw: Cow<'a, str>,
    ) -> Result<ConditionalFormattingBitmask, InvalidAttributeValue> {
        Ok(ConditionalFormattingBitmask::from_wire(&raw))
    }

    fn encode<'a>(value: ConditionalFormattingBitmask) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

// -------------------------------------------------------------------------------------------
// Leaf types — one per remaining `CT_PPrBase`-family complex type, in `run_properties.rs`'s own
// shape for an attribute-only element: `#[derive(XmlAttributes)]` plus a hand-written `FromXml`/
// `ToXml` pair, since these have no `children`/`text` framework field for the container derive.
// -------------------------------------------------------------------------------------------

/// `CT_String` (`w:pStyle`, "Referenced Paragraph Style", §17.3.1.27) — the id of the paragraph
/// style this paragraph refers to.
///
/// The same wire shape as [`super::run_properties::CharacterStyle`] (`w:rStyle`), given its own type
/// rather than reused: `CharacterStyle`'s own name is specific to *character* styles, and returning a
/// value literally called `CharacterStyle` from a *paragraph* style accessor would be exactly the
/// misleading-identifier problem the naming convention exists to prevent — unlike
/// [`super::run_properties::Toggle`] or [`super::body::Text`], whose shared names carry no such wrong
/// specificity.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = style_id, required))]
pub struct ParagraphStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ParagraphStyle {
    /// Builds a new `w:pStyle` referring to `style_id`.
    #[must_use]
    pub fn new(interner: &mut Interner, style_id: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "pStyle"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_style_id(interner, style_id);
        value
    }
}

impl FromXml for ParagraphStyle {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ParagraphStyle {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_DecimalNumber` — a required signed integer `val`. Reused for `w:outlineLvl` ("Associated
/// Outline Level", §17.3.1.20), `w:divId` ("Associated HTML div ID", §17.3.1.10), and `w:numPr`'s own
/// `w:ilvl`/`w:numId` — four different properties sharing one wire shape, exactly as
/// `run_properties.rs`'s [`HalfPointMeasureValue`] is reused across `sz`/`szCs`/`kern`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Number<DecimalNumber>, accessor = value, required))]
pub struct DecimalNumberValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DecimalNumberValue {
    /// Builds a new `local` element (`"outlineLvl"`, `"divId"`, `"ilvl"` or `"numId"`) of `value`.
    #[must_use]
    fn new(interner: &mut Interner, local: &str, value: DecimalNumber) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for DecimalNumberValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DecimalNumberValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Jc` (`w:jc`, "Paragraph Alignment", §17.3.1.13) — a required justification value.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<Justification>, accessor = value, required))]
pub struct ParagraphAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ParagraphAlignment {
    /// Builds a new `w:jc` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: Justification) -> Self {
        let mut item = Self {
            name: wml_name(interner, "jc"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for ParagraphAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ParagraphAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TextDirection` (`w:textDirection`, "Paragraph Text Flow Direction", §17.3.1.41) — a required
/// text-flow-direction value.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TextFlowDirection>, accessor = value, required))]
pub struct ParagraphTextFlowDirection {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ParagraphTextFlowDirection {
    /// Builds a new `w:textDirection` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: TextFlowDirection) -> Self {
        let mut item = Self {
            name: wml_name(interner, "textDirection"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for ParagraphTextFlowDirection {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ParagraphTextFlowDirection {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TextAlignment` (`w:textAlignment`, "Vertical Character Alignment on Line", §17.3.1.39) — a
/// required vertical-alignment value, distinct from `w:vertAlign`
/// ([`super::run_properties::VerticalAlignment`], subscript/superscript).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<VerticalTextAlignment>, accessor = value, required))]
pub struct VerticalCharacterAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl VerticalCharacterAlignment {
    /// Builds a new `w:textAlignment` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: VerticalTextAlignment) -> Self {
        let mut item = Self {
            name: wml_name(interner, "textAlignment"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for VerticalCharacterAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for VerticalCharacterAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TextboxTightWrap` (`w:textboxTightWrap`, "Allow Surrounding Paragraphs to Tight Wrap to Text
/// Box Contents", §17.3.1.40) — a required tight-wrap value.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TextBoxTightWrap>, accessor = value, required))]
pub struct TextBoxTightWrapSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TextBoxTightWrapSetting {
    /// Builds a new `w:textboxTightWrap` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: TextBoxTightWrap) -> Self {
        let mut item = Self {
            name: wml_name(interner, "textboxTightWrap"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for TextBoxTightWrapSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TextBoxTightWrapSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_Cnf` (`w:cnfStyle`, "Paragraph Conditional Formatting", §17.3.1.8) — a twelve-bit conditional
/// formatting reference plus the twelve individual region flags it can also carry directly. Its
/// meaning when this paragraph is a table row is MJXOFF-116's — this type only reads and writes the
/// element structurally.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = ConditionalFormattingBits, accessor = bitmask))]
#[xml(attribute(local = "firstRow", prefix = "w", codec = OnOff, accessor = first_row))]
#[xml(attribute(local = "lastRow", prefix = "w", codec = OnOff, accessor = last_row))]
#[xml(attribute(local = "firstColumn", prefix = "w", codec = OnOff, accessor = first_column))]
#[xml(attribute(local = "lastColumn", prefix = "w", codec = OnOff, accessor = last_column))]
#[xml(attribute(local = "oddVBand", prefix = "w", codec = OnOff, accessor = odd_vertical_band))]
#[xml(attribute(local = "evenVBand", prefix = "w", codec = OnOff, accessor = even_vertical_band))]
#[xml(attribute(local = "oddHBand", prefix = "w", codec = OnOff, accessor = odd_horizontal_band))]
#[xml(attribute(local = "evenHBand", prefix = "w", codec = OnOff, accessor = even_horizontal_band))]
#[xml(attribute(local = "firstRowFirstColumn", prefix = "w", codec = OnOff, accessor = first_row_first_column))]
#[xml(attribute(local = "firstRowLastColumn", prefix = "w", codec = OnOff, accessor = first_row_last_column))]
#[xml(attribute(local = "lastRowFirstColumn", prefix = "w", codec = OnOff, accessor = last_row_first_column))]
#[xml(attribute(local = "lastRowLastColumn", prefix = "w", codec = OnOff, accessor = last_row_last_column))]
pub struct ConditionalFormatting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ConditionalFormatting {
    /// Builds a new, empty `w:cnfStyle` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "cnfStyle"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for ConditionalFormatting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ConditionalFormatting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_FramePr` (`w:framePr`, "Text Frame Properties", §17.3.1.11) — a paragraph turned into a legacy
/// text frame: size, position, anchoring and wrap.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "dropCap", prefix = "w", codec = Enumeration<DropCap>, accessor = drop_cap))]
#[xml(attribute(local = "lines", prefix = "w", codec = Number<DecimalNumber>, accessor = drop_cap_lines))]
#[xml(attribute(local = "w", prefix = "w", codec = Twips, accessor = width))]
#[xml(attribute(local = "h", prefix = "w", codec = Twips, accessor = height))]
#[xml(attribute(local = "vSpace", prefix = "w", codec = Twips, accessor = vertical_spacing))]
#[xml(attribute(local = "hSpace", prefix = "w", codec = Twips, accessor = horizontal_spacing))]
#[xml(attribute(local = "wrap", prefix = "w", codec = Enumeration<TextFrameWrapping>, accessor = wrap))]
#[xml(attribute(local = "hAnchor", prefix = "w", codec = Enumeration<HorizontalAnchor>, accessor = horizontal_anchor))]
#[xml(attribute(local = "vAnchor", prefix = "w", codec = Enumeration<VerticalAnchor>, accessor = vertical_anchor))]
#[xml(attribute(local = "x", prefix = "w", codec = SignedTwips, accessor = x))]
#[xml(attribute(local = "xAlign", prefix = "w", codec = Enumeration<RelativeHorizontalAlignment>, accessor = x_alignment))]
#[xml(attribute(local = "y", prefix = "w", codec = SignedTwips, accessor = y))]
#[xml(attribute(local = "yAlign", prefix = "w", codec = Enumeration<RelativeVerticalAlignment>, accessor = y_alignment))]
#[xml(attribute(local = "hRule", prefix = "w", codec = Enumeration<HeightRule>, accessor = height_rule))]
#[xml(attribute(local = "anchorLock", prefix = "w", codec = OnOff, accessor = anchor_lock))]
pub struct FrameProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl FrameProperties {
    /// Builds a new, empty `w:framePr` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "framePr"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for FrameProperties {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for FrameProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_TabStop` (`w:tab`, "Custom Tab Stop", §17.3.1.37) — one custom tab stop's position, alignment
/// and leader. A stop whose alignment (`w:val`) is `clear` **removes** an inherited stop at that
/// position rather than adding a new one — this type does not special-case that value, so a `clear`
/// entry survives reading, iteration and writing exactly like any other stop; dropping it during an
/// edit would silently change the paragraph's inherited tab layout.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<TabStopType>, accessor = alignment, required))]
#[xml(attribute(local = "leader", prefix = "w", codec = Enumeration<TabStopLeader>, accessor = leader))]
#[xml(attribute(local = "pos", prefix = "w", codec = SignedTwips, accessor = position, required))]
pub struct TabStop {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TabStop {
    /// Builds a new `w:tab` of `alignment` at `position`, with no leader.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        alignment: TabStopType,
        position: SignedTwipsMeasure,
    ) -> Self {
        let mut value = Self {
            name: wml_name(interner, "tab"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_alignment(interner, alignment);
        value.set_position(interner, position);
        value
    }
}

impl FromXml for TabStop {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TabStop {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_Spacing (w:spacing) — the line/lineRule trap.
// -------------------------------------------------------------------------------------------

/// `w:spacing/@line` paired with `w:spacing/@lineRule` — the only way [`Spacing::line_spacing`]
/// returns either. `ST_LineSpacingRule::Auto` means `value` is 240ths of a line; `Exact`/`AtLeast`
/// mean `value` is in twentieths of a point (twips), exact or a minimum respectively — the same wire
/// integer is a different physical quantity depending on `rule`, which is exactly why this crate
/// never hands back one without the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSpacing {
    /// `w:lineRule` — which physical unit `value` is in.
    pub rule: LineSpacingRule,
    /// `w:line`'s own value, in the unit `rule` names.
    pub value: SignedTwipsMeasure,
}

/// `CT_Spacing` (`w:spacing`, "Spacing Between Lines and Above/Below Paragraph", §17.3.1.33) —
/// spacing above and below this paragraph, and between its own lines.
///
/// `w:beforeAutospacing`/`w:afterAutospacing`, when on, **override** `w:before`/`w:after` — the word
/// processor computes the space instead of using the fixed twips value, which stays in the file
/// (readable through [`Spacing::before`]/[`Spacing::after`]) but does not apply while autospacing is
/// on. Neither `before`/`after` accessor consults the autospacing flag itself; a caller applying this
/// paragraph's effective spacing checks [`Spacing::before_autospacing`]/[`Spacing::after_autospacing`]
/// first.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "before", prefix = "w", codec = Twips, accessor = before))]
#[xml(attribute(local = "beforeLines", prefix = "w", codec = Number<DecimalNumber>, accessor = before_lines))]
#[xml(attribute(local = "beforeAutospacing", prefix = "w", codec = OnOff, accessor = before_autospacing))]
#[xml(attribute(local = "after", prefix = "w", codec = Twips, accessor = after))]
#[xml(attribute(local = "afterLines", prefix = "w", codec = Number<DecimalNumber>, accessor = after_lines))]
#[xml(attribute(local = "afterAutospacing", prefix = "w", codec = OnOff, accessor = after_autospacing))]
pub struct Spacing {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Spacing {
    /// Builds a new, empty `w:spacing` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "spacing"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// Reads `w:line` and `w:lineRule` together, or `None` if `w:line` is absent — `w:lineRule` alone
    /// names no line height to interpret. When `w:line` is present without `w:lineRule`, the schema's
    /// own default (`auto`) is applied, matching every other defaulted attribute in this workspace:
    /// the default is *returned*, never *written*.
    ///
    /// There is no `Spacing::line` or `Spacing::line_rule` — this is the only accessor that reaches
    /// either, and it always hands back both:
    ///
    /// ```
    /// use mjx_docx::{LineSpacing, Spacing};
    /// use mjx_ooxml_core::Interner;
    /// use mjx_ooxml_types::wordprocessingml::{LineSpacingRule, SignedTwipsMeasure};
    ///
    /// let mut interner = Interner::new();
    /// let mut spacing = Spacing::new(&mut interner);
    /// assert_eq!(spacing.line_spacing(&interner), Ok(None));
    ///
    /// spacing.set_line_spacing(
    ///     &mut interner,
    ///     Some(LineSpacing {
    ///         rule: LineSpacingRule::Exact,
    ///         value: SignedTwipsMeasure::from_wire("480"),
    ///     }),
    /// );
    ///
    /// // Reading `w:line` back out always comes paired with `w:lineRule` — there is no accessor
    /// // that returns the bare integer alone, so a caller cannot misinterpret it as twips when the
    /// // file actually said `auto` (240ths of a line), or vice versa.
    /// let read = spacing.line_spacing(&interner).expect("valid").expect("set above");
    /// assert_eq!(read.rule, LineSpacingRule::Exact);
    /// assert_eq!(read.value, SignedTwipsMeasure::from_wire("480"));
    /// ```
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:line` or `w:lineRule` is present but malformed.
    pub fn line_spacing(&self, interner: &Interner) -> Result<Option<LineSpacing>, AttributeError> {
        let Some(value) = mjx_xml::attribute::read::<SignedTwips>(
            &self.attributes,
            interner,
            Some("w"),
            "line",
            "w:line",
        )?
        else {
            return Ok(None);
        };
        let rule = mjx_xml::attribute::read::<Enumeration<LineSpacingRule>>(
            &self.attributes,
            interner,
            Some("w"),
            "lineRule",
            "w:lineRule",
        )?
        .unwrap_or(LineSpacingRule::Auto);
        Ok(Some(LineSpacing { rule, value }))
    }

    /// Writes `w:line` and `w:lineRule` together: `None` removes both; `Some(LineSpacing { rule,
    /// value })` writes both, `rule` always explicit (never relying on the schema default), so a
    /// later read is never ambiguous about which unit `value` is in.
    pub fn set_line_spacing(&mut self, interner: &mut Interner, value: Option<LineSpacing>) {
        match value {
            None => {
                mjx_xml::attribute::write::<SignedTwips>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "line",
                    None,
                );
                mjx_xml::attribute::write::<Enumeration<LineSpacingRule>>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "lineRule",
                    None,
                );
            }
            Some(LineSpacing { rule, value }) => {
                mjx_xml::attribute::write::<SignedTwips>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "line",
                    Some(value),
                );
                mjx_xml::attribute::write::<Enumeration<LineSpacingRule>>(
                    &mut self.attributes,
                    interner,
                    Some("w"),
                    "lineRule",
                    Some(rule),
                );
                self.empty = false;
            }
        }
    }
}

impl FromXml for Spacing {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Spacing {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_Ind (w:ind) — logical versus physical indentation.
// -------------------------------------------------------------------------------------------

/// `CT_Ind` (`w:ind`, "Paragraph Indentation", §17.3.1.12) — leading- and trailing-edge indentation,
/// the first-line adjustment, and their character-unit and legacy-physical variants.
///
/// `w:startChars`/`w:endChars`/`w:hangingChars`/`w:firstLineChars` **supersede** their twips-based
/// sibling when both are present — ECMA-376 Part 1's own prose for each ("if the `endChars` attribute
/// is specified, then [`Indentation::end`]'s value is ignored") — but this type does not resolve
/// that itself: it
/// hands back all twelve attributes independently, exactly as the file carries them. See
/// [`Indentation::leading_edge`]/[`Indentation::trailing_edge`] for the one precedence this type does
/// resolve (logical versus physical), and the module's own doc comment for why.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "start", prefix = "w", codec = SignedTwips, accessor = start))]
#[xml(attribute(local = "startChars", prefix = "w", codec = Number<DecimalNumber>, accessor = start_chars))]
#[xml(attribute(local = "end", prefix = "w", codec = SignedTwips, accessor = end))]
#[xml(attribute(local = "endChars", prefix = "w", codec = Number<DecimalNumber>, accessor = end_chars))]
#[xml(attribute(local = "left", prefix = "w", codec = SignedTwips, accessor = left))]
#[xml(attribute(local = "leftChars", prefix = "w", codec = Number<DecimalNumber>, accessor = left_chars))]
#[xml(attribute(local = "right", prefix = "w", codec = SignedTwips, accessor = right))]
#[xml(attribute(local = "rightChars", prefix = "w", codec = Number<DecimalNumber>, accessor = right_chars))]
#[xml(attribute(local = "hanging", prefix = "w", codec = Twips, accessor = hanging))]
#[xml(attribute(local = "hangingChars", prefix = "w", codec = Number<DecimalNumber>, accessor = hanging_chars))]
#[xml(attribute(local = "firstLine", prefix = "w", codec = Twips, accessor = first_line))]
#[xml(attribute(local = "firstLineChars", prefix = "w", codec = Number<DecimalNumber>, accessor = first_line_chars))]
pub struct Indentation {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Indentation {
    /// Builds a new, empty `w:ind` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "ind"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }

    /// This paragraph's leading-edge indentation (the left edge in a left-to-right paragraph),
    /// resolved between `w:start` and `w:left` when a file carries both: `w:start` wins — see the
    /// module's own doc comment for why.
    ///
    /// # Errors
    /// An [`AttributeError`] if either attribute is present but malformed.
    pub fn leading_edge(
        &self,
        interner: &Interner,
    ) -> Result<Option<SignedTwipsMeasure>, AttributeError> {
        match self.start(interner)? {
            Some(value) => Ok(Some(value)),
            None => self.left(interner),
        }
    }

    /// This paragraph's trailing-edge indentation (the right edge in a left-to-right paragraph),
    /// resolved between `w:end` and `w:right` when a file carries both: `w:end` wins — see
    /// [`Indentation::leading_edge`].
    ///
    /// # Errors
    /// An [`AttributeError`] if either attribute is present but malformed.
    pub fn trailing_edge(
        &self,
        interner: &Interner,
    ) -> Result<Option<SignedTwipsMeasure>, AttributeError> {
        match self.end(interner)? {
            Some(value) => Ok(Some(value)),
            None => self.right(interner),
        }
    }
}

impl FromXml for Indentation {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Indentation {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_Tabs / CT_TabStop (w:tabs)
// -------------------------------------------------------------------------------------------

/// One ordered child of [`TabStops`]: `CT_Tabs`' own content is `tab+` alone — a homogeneous,
/// repeatable list with no schema order to enforce among its members, so this module never sorts or
/// ranks them; [`TabStops::push_tab`] simply appends, and reading returns exactly the file's own
/// order (including any `clear` stops — see [`TabStop`]'s own doc comment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabStopContent {
    /// `w:tab` (`CT_TabStop`).
    Tab(TabStop),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Tabs` (`w:tabs`, "Set of Custom Tab Stops", §17.3.1.38) — an ordered list of custom tab stops.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct TabStops {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tab", variant = Tab, ty = TabStop))]
    content: Vec<TabStopContent>,
}

impl TabStops {
    /// Builds a new, empty `w:tabs`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "tabs"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every custom tab stop, in document order — including any whose alignment is `clear`.
    pub fn tabs(&self) -> impl Iterator<Item = &TabStop> {
        self.content.iter().filter_map(|item| match item {
            TabStopContent::Tab(tab) => Some(tab),
            TabStopContent::Raw(_) => None,
        })
    }

    /// How many tab stops this list holds.
    #[must_use]
    pub fn tab_count(&self) -> usize {
        self.tabs().count()
    }

    /// Appends `tab` as this list's new last stop.
    pub fn push_tab(&mut self, tab: TabStop) {
        self.content.push(TabStopContent::Tab(tab));
        self.empty = false;
    }

    /// Removes and returns the tab stop at `index` (counting only tab stops, not unmodelled nodes),
    /// or `None` if there is no such stop.
    pub fn remove_tab(&mut self, index: usize) -> Option<TabStop> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, TabStopContent::Tab(_)))
            .nth(index)
            .map(|(at, _)| at)?;
        match self.content.remove(at) {
            TabStopContent::Tab(tab) => Some(tab),
            TabStopContent::Raw(_) => unreachable!("the filtered index only ever names a Tab item"),
        }
    }
}

// -------------------------------------------------------------------------------------------
// Generic property-declaration macros, parametrized over the content enum so
// `ParagraphPropertyContent` and `ParagraphMarkRunPropertyContent` share one definition each rather
// than each restating the getter/setter logic — the same shape as `run_properties.rs`'s own macros,
// generalized over which enum a given container type actually holds.
// -------------------------------------------------------------------------------------------

/// Declares one `CT_OnOff`-shaped property on the container type the macro is invoked inside: a
/// tri-state getter and a whole-value setter, exactly as `run_properties.rs`'s own `toggle_property!`
/// — generalized over `$enum_ty` so both this module's container types can use it.
macro_rules! toggle_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<bool>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    $enum_ty::$variant(toggle) => Some(toggle),
                    _ => None,
                })
                .map(|toggle| toggle.value(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value`.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<bool>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let mut toggle = Toggle::new(interner, $local);
                    toggle.set_value(interner, Some(value));
                    self.set($local, is_target, Some($enum_ty::$variant(toggle)));
                }
            }
        }
    };
}

/// Declares one whole-value property: a borrowing getter and a replace-insert-or-remove setter,
/// generalized the same way as [`toggle_property!`].
macro_rules! value_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                $enum_ty::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` replaces \
            or inserts it at its schema rank.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            self.set($local, is_target, value.map($enum_ty::$variant));
        }
    };
}

/// Declares one `CT_DecimalNumber`-shaped property: a fallible flattened getter and a whole-value
/// setter that builds [`DecimalNumberValue`] under its own wire name.
macro_rules! decimal_number_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<i64>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    $enum_ty::$variant(value) => Some(value),
                    _ => None,
                })
                .map(|value| value.value(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value`.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<i64>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let element = DecimalNumberValue::new(interner, $local, value);
                    self.set($local, is_target, Some($enum_ty::$variant(element)));
                }
            }
        }
    };
}

/// Declares one `CT_HpsMeasure`-shaped property (`sz`, `szCs`, `kern`): a fallible flattened getter
/// and a whole-value setter that builds [`HalfPointMeasureValue`] under its own wire name.
macro_rules! half_point_property {
    ($enum_ty:ident, $getter:ident, $setter:ident, $variant:ident, $local:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $getter(&self, interner: &Interner) -> Result<Option<HalfPointMeasure>, AttributeError> {
            self.content
                .iter()
                .find_map(|item| match item {
                    $enum_ty::$variant(value) => Some(value),
                    _ => None,
                })
                .map(|value| value.half_points(interner))
                .transpose()
        }

        #[doc = concat!("Sets `w:", $local, "`: `None` removes the element; `Some(value)` ensures it \
            is present with `w:val` written explicitly as `value` half-points.")]
        pub fn $setter(&mut self, interner: &mut Interner, value: Option<HalfPointMeasure>) {
            let is_target = |item: &$enum_ty| matches!(item, $enum_ty::$variant(_));
            match value {
                None => self.remove(is_target),
                Some(value) => {
                    let element = HalfPointMeasureValue::new(interner, $local, value);
                    self.set($local, is_target, Some($enum_ty::$variant(element)));
                }
            }
        }
    };
}

// -------------------------------------------------------------------------------------------
// CT_PBdr (w:pBdr)
// -------------------------------------------------------------------------------------------

/// One ordered child of [`ParagraphBorders`]: `CT_PBdr`'s sequence is `top, left, bottom, right,
/// between, bar` — every one a [`Border`] (`CT_Border`, MJXOFF-94's own type). This module defines no
/// second border type; reusing MJXOFF-94's is exactly the "consume, do not re-create" the ticket asks
/// for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphBorderContent {
    /// `w:top` (§17.3.1.42, "Paragraph Border Above Identical Paragraphs").
    Top(Border),
    /// `w:left` (§17.3.1.17, "Left Paragraph Border").
    Left(Border),
    /// `w:bottom` (§17.3.1.7, "Paragraph Border Below Identical Paragraphs").
    Bottom(Border),
    /// `w:right` (§17.3.1.28, "Right Paragraph Border").
    Right(Border),
    /// `w:between` (§17.3.1.5, "Paragraph Border Between Identical Paragraphs").
    Between(Border),
    /// `w:bar` (§17.3.1.4, "Paragraph Border Between Facing Pages").
    Bar(Border),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_PBdr` (`w:pBdr`, "Paragraph Borders", §17.3.1.24) — the six borders a paragraph can carry.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ParagraphBorders {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = Border),
        child(local = "left", variant = Left, ty = Border),
        child(local = "bottom", variant = Bottom, ty = Border),
        child(local = "right", variant = Right, ty = Border),
        child(local = "between", variant = Between, ty = Border),
        child(local = "bar", variant = Bar, ty = Border)
    )]
    content: Vec<ParagraphBorderContent>,
}

impl ParagraphBorders {
    /// Builds a new, empty `w:pBdr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "pBdr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &ParagraphBorderContent) -> Option<u16> {
        let local = match item {
            ParagraphBorderContent::Top(_) => "top",
            ParagraphBorderContent::Left(_) => "left",
            ParagraphBorderContent::Bottom(_) => "bottom",
            ParagraphBorderContent::Right(_) => "right",
            ParagraphBorderContent::Between(_) => "between",
            ParagraphBorderContent::Bar(_) => "bar",
            ParagraphBorderContent::Raw(_) => return None,
        };
        PARAGRAPH_BORDERS.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&ParagraphBorderContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: ParagraphBorderContent) {
        let at =
            PARAGRAPH_BORDERS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&ParagraphBorderContent) -> bool,
        value: Option<ParagraphBorderContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    value_property!(
        ParagraphBorderContent,
        top,
        set_top,
        Top,
        Border,
        "top",
        "`w:top` — the border above this paragraph."
    );
    value_property!(
        ParagraphBorderContent,
        left,
        set_left,
        Left,
        Border,
        "left",
        "`w:left` — the border to this paragraph's left."
    );
    value_property!(
        ParagraphBorderContent,
        bottom,
        set_bottom,
        Bottom,
        Border,
        "bottom",
        "`w:bottom` — the border below this paragraph."
    );
    value_property!(
        ParagraphBorderContent,
        right,
        set_right,
        Right,
        Border,
        "right",
        "`w:right` — the border to this paragraph's right."
    );
    value_property!(
        ParagraphBorderContent,
        between,
        set_between,
        Between,
        Border,
        "between",
        "`w:between` — the border between this paragraph and an identically-formatted neighbour."
    );
    value_property!(
        ParagraphBorderContent,
        bar,
        set_bar,
        Bar,
        Border,
        "bar",
        "`w:bar` — the border drawn between facing pages for this paragraph."
    );
}

// -------------------------------------------------------------------------------------------
// CT_NumPr (w:numPr)
// -------------------------------------------------------------------------------------------

/// One ordered child of [`NumberingProperties`]: `CT_NumPr`'s sequence is `ilvl, numId,
/// numberingChange, ins`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberingPropertyContent {
    /// `w:ilvl` (`CT_DecimalNumber`) — the numbering level this paragraph uses.
    Level(DecimalNumberValue),
    /// `w:numId` (`CT_DecimalNumber`) — the numbering definition instance this paragraph uses. What
    /// it resolves to is MJXOFF-109's numbering definitions.
    Definition(DecimalNumberValue),
    /// `w:numberingChange` (`CT_TrackChangeNumbering`) — MJXOFF-126's tracked-change semantics.
    NumberingChange(Unmodeled),
    /// `w:ins` (`CT_TrackChange`) — marks the whole `w:numPr` as tracked-inserted; MJXOFF-126.
    Inserted(Unmodeled),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_NumPr` (`w:numPr`, "Numbering Definition Instance Reference", §17.3.1.19) — the level/id pair
/// a paragraph references into a numbering definition. Read and written **structurally** here; what
/// the pair *resolves to* is MJXOFF-109's numbering definitions and MJXOFF-104's effective-properties
/// ladder.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct NumberingProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "ilvl", variant = Level, ty = DecimalNumberValue),
        child(local = "numId", variant = Definition, ty = DecimalNumberValue),
        child(local = "numberingChange", variant = NumberingChange, ty = Unmodeled),
        child(local = "ins", variant = Inserted, ty = Unmodeled)
    )]
    content: Vec<NumberingPropertyContent>,
}

impl NumberingProperties {
    /// Builds a new, empty `w:numPr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "numPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &NumberingPropertyContent) -> Option<u16> {
        let local = match item {
            NumberingPropertyContent::Level(_) => "ilvl",
            NumberingPropertyContent::Definition(_) => "numId",
            NumberingPropertyContent::NumberingChange(_) => "numberingChange",
            NumberingPropertyContent::Inserted(_) => "ins",
            NumberingPropertyContent::Raw(_) => return None,
        };
        NUMBERING_PROPERTIES.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&NumberingPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: NumberingPropertyContent) {
        let at =
            NUMBERING_PROPERTIES.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&NumberingPropertyContent) -> bool,
        value: Option<NumberingPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    decimal_number_property!(
        NumberingPropertyContent,
        level,
        set_level,
        Level,
        "ilvl",
        "`w:ilvl` — the numbering level this paragraph uses."
    );
    decimal_number_property!(
        NumberingPropertyContent,
        numbering_id,
        set_numbering_id,
        Definition,
        "numId",
        "`w:numId` — the numbering definition instance this paragraph uses."
    );
}

// -------------------------------------------------------------------------------------------
// CT_ParaRPr (w:pPr/w:rPr) — the paragraph mark's own run properties. NOT a run's w:rPr — see the
// module's own doc comment ("The paragraph mark is not a run").
// -------------------------------------------------------------------------------------------

/// One ordered child of [`ParagraphMarkRunProperties`]: `EG_ParaRPrTrackChanges` (`ins`, `del`,
/// `moveFrom`, `moveTo` — MJXOFF-126's own semantics), then `EG_RPrBase`'s 39 members (the same ones
/// [`super::run_properties::RunPropertyContent`] holds, reusing every leaf type MJXOFF-94 already
/// built — see the module's own doc comment), then `rPrChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphMarkRunPropertyContent {
    /// `w:ins` (`CT_TrackChange`) — MJXOFF-126.
    Inserted(Unmodeled),
    /// `w:del` (`CT_TrackChange`) — MJXOFF-126.
    Deleted(Unmodeled),
    /// `w:moveFrom` (`CT_TrackChange`) — MJXOFF-126.
    MovedFrom(Unmodeled),
    /// `w:moveTo` (`CT_TrackChange`) — MJXOFF-126.
    MovedTo(Unmodeled),
    /// `w:rStyle` (§17.3.2.29) — the paragraph mark's referenced character style.
    CharacterStyle(CharacterStyle),
    /// `w:rFonts` (§17.3.2.26).
    Fonts(Fonts),
    /// `w:b` (§17.3.2.1) — `CT_OnOff`.
    Bold(Toggle),
    /// `w:bCs` (§17.3.2.2) — `CT_OnOff`.
    BoldComplexScript(Toggle),
    /// `w:i` (§17.3.2.16) — `CT_OnOff`.
    Italic(Toggle),
    /// `w:iCs` (§17.3.2.17) — `CT_OnOff`.
    ItalicComplexScript(Toggle),
    /// `w:caps` (§17.3.2.5) — `CT_OnOff`.
    AllCapitals(Toggle),
    /// `w:smallCaps` (§17.3.2.33) — `CT_OnOff`.
    SmallCaps(Toggle),
    /// `w:strike` (§17.3.2.37) — `CT_OnOff`.
    Strikethrough(Toggle),
    /// `w:dstrike` (§17.3.2.9) — `CT_OnOff`.
    DoubleStrikethrough(Toggle),
    /// `w:outline` (§17.3.2.23) — `CT_OnOff`.
    Outline(Toggle),
    /// `w:shadow` (§17.3.2.31) — `CT_OnOff`.
    Shadow(Toggle),
    /// `w:emboss` (§17.3.2.13) — `CT_OnOff`.
    Embossing(Toggle),
    /// `w:imprint` (§17.3.2.18) — `CT_OnOff`.
    Imprinting(Toggle),
    /// `w:noProof` (§17.3.2.21) — `CT_OnOff`.
    ProofingExempt(Toggle),
    /// `w:snapToGrid` (§17.3.2.34) — `CT_OnOff`.
    SnapToGrid(Toggle),
    /// `w:vanish` (§17.3.2.41) — `CT_OnOff`.
    Hidden(Toggle),
    /// `w:webHidden` (§17.3.2.44) — `CT_OnOff`.
    WebHidden(Toggle),
    /// `w:color` (§17.3.2.6).
    Color(Color),
    /// `w:spacing` (§17.3.2.35) — signed twentieths of a point. Not to be confused with
    /// [`Spacing`] (`CT_Spacing`, `w:pPr/w:spacing`) — same local name, unrelated complex type.
    CharacterSpacing(SignedTwipsMeasureValue),
    /// `w:w` (§17.3.2.43) — a horizontal scale percentage.
    CharacterScale(TextScaleValue),
    /// `w:kern` (§17.3.2.19) — a half-point kerning threshold.
    Kerning(HalfPointMeasureValue),
    /// `w:position` (§17.3.2.24) — a signed half-point offset.
    VerticalOffset(SignedHalfPointMeasureValue),
    /// `w:sz` (§17.3.2.38) — a half-point font size.
    FontSize(HalfPointMeasureValue),
    /// `w:szCs` (§17.3.2.39) — a half-point font size.
    ComplexScriptFontSize(HalfPointMeasureValue),
    /// `w:highlight` (§17.3.2.15).
    Highlight(Highlight),
    /// `w:u` (§17.3.2.40).
    Underline(Underline),
    /// `w:effect` (§17.3.2.11).
    TextEffect(TextEffect),
    /// `w:bdr` (§17.3.2.4) — the pilcrow's own character border, distinct from [`ParagraphBorders`]
    /// (`w:pBdr`, the paragraph's borders).
    Border(Border),
    /// `w:shd` (§17.3.2.32) — the pilcrow's own shading, distinct from [`ParagraphPropertyContent::Shading`]
    /// (`w:pPr/w:shd`, the paragraph's own shading).
    Shading(Shading),
    /// `w:fitText` (§17.3.2.14).
    ManualRunWidth(ManualRunWidth),
    /// `w:vertAlign` (§17.3.2.42).
    VerticalAlignment(VerticalAlignment),
    /// `w:rtl` (§17.3.2.30) — `CT_OnOff`.
    RightToLeft(Toggle),
    /// `w:cs` (§17.3.2.7) — `CT_OnOff`.
    ComplexScript(Toggle),
    /// `w:em` (§17.3.2.12).
    Emphasis(Emphasis),
    /// `w:lang` (§17.3.2.20).
    Languages(Languages),
    /// `w:eastAsianLayout` (§17.3.2.10).
    EastAsianLayout(EastAsianLayout),
    /// `w:specVanish` (§17.3.2.36) — `CT_OnOff`.
    AlwaysHidden(Toggle),
    /// `w:oMath` (§17.3.2.22) — `CT_OnOff`, like the other nineteen toggles.
    Math(Toggle),
    /// `w:rPrChange` (`CT_ParaRPrChange`) — MJXOFF-126's own scope, kept opaque here.
    Change(Unmodeled),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_ParaRPr` (`w:pPr/w:rPr`, "Run Properties for the Paragraph Mark", §17.3.1.29) — the pilcrow's
/// own character formatting: `EG_ParaRPrTrackChanges`, then `EG_RPrBase`'s 39 members (each
/// independently optional, in no schema-imposed relative order — an `xsd:choice`, exactly as
/// [`super::run_properties::RunProperties`]'s own), then `rPrChange`.
///
/// **This is not a run's `w:rPr`.** Reached only from [`ParagraphProperties::paragraph_mark_properties`]
/// (and its `_mut`/`_or_insert` companions) — never from [`Run::run_properties`](super::body::Run::run_properties),
/// which returns [`super::run_properties::RunProperties`] instead. Setting a property here can never
/// touch a run's own `w:rPr`, and setting a run's property can never touch this: they are different
/// Rust types stored in different trees ([`Paragraph::properties`](super::body::Paragraph::properties)
/// versus [`Run::run_properties`](super::body::Run::run_properties)), so there is no code path between
/// them.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ParagraphMarkRunProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "ins", variant = Inserted, ty = Unmodeled),
        child(local = "del", variant = Deleted, ty = Unmodeled),
        child(local = "moveFrom", variant = MovedFrom, ty = Unmodeled),
        child(local = "moveTo", variant = MovedTo, ty = Unmodeled),
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
    content: Vec<ParagraphMarkRunPropertyContent>,
}

impl ParagraphMarkRunProperties {
    /// Builds a new, empty `w:pPr/w:rPr` — no properties, ready for this type's setters.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "rPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The schema rank of an existing content item, computed from the generated
    /// [`PARAGRAPH_MARK_RUN_PROPERTIES`] table by the item's own wire name — never a hand-rolled
    /// number, so a schema change moves this insertion point too.
    fn rank(item: &ParagraphMarkRunPropertyContent) -> Option<u16> {
        let local = match item {
            ParagraphMarkRunPropertyContent::Inserted(_) => "ins",
            ParagraphMarkRunPropertyContent::Deleted(_) => "del",
            ParagraphMarkRunPropertyContent::MovedFrom(_) => "moveFrom",
            ParagraphMarkRunPropertyContent::MovedTo(_) => "moveTo",
            ParagraphMarkRunPropertyContent::Change(_) => "rPrChange",
            ParagraphMarkRunPropertyContent::Raw(_) => return None,
            _ => "b", // any EG_RPrBase member — all share one rank; "b" is just a representative.
        };
        PARAGRAPH_MARK_RUN_PROPERTIES.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&ParagraphMarkRunPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: ParagraphMarkRunPropertyContent) {
        let at = PARAGRAPH_MARK_RUN_PROPERTIES
            .insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&ParagraphMarkRunPropertyContent) -> bool,
        value: Option<ParagraphMarkRunPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    toggle_property!(
        ParagraphMarkRunPropertyContent,
        bold,
        set_bold,
        Bold,
        "b",
        "`w:b` — whether the paragraph mark is bold."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        bold_complex_script,
        set_bold_complex_script,
        BoldComplexScript,
        "bCs",
        "`w:bCs` — whether the paragraph mark is bold, for complex-script text."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        italic,
        set_italic,
        Italic,
        "i",
        "`w:i` — whether the paragraph mark is italic."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        italic_complex_script,
        set_italic_complex_script,
        ItalicComplexScript,
        "iCs",
        "`w:iCs` — whether the paragraph mark is italic, for complex-script text."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        all_capitals,
        set_all_capitals,
        AllCapitals,
        "caps",
        "`w:caps` — whether the paragraph mark displays as a capital letter."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        small_caps,
        set_small_caps,
        SmallCaps,
        "smallCaps",
        "`w:smallCaps` — whether the paragraph mark displays as a small capital."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        strikethrough,
        set_strikethrough,
        Strikethrough,
        "strike",
        "`w:strike` — whether the paragraph mark has a single strikethrough."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        double_strikethrough,
        set_double_strikethrough,
        DoubleStrikethrough,
        "dstrike",
        "`w:dstrike` — whether the paragraph mark has a double strikethrough."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        outline,
        set_outline,
        Outline,
        "outline",
        "`w:outline` — whether the paragraph mark displays as an outline."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        shadow,
        set_shadow,
        Shadow,
        "shadow",
        "`w:shadow` — whether the paragraph mark has a shadow."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        embossing,
        set_embossing,
        Embossing,
        "emboss",
        "`w:emboss` — whether the paragraph mark displays embossed."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        imprinting,
        set_imprinting,
        Imprinting,
        "imprint",
        "`w:imprint` — whether the paragraph mark displays imprinted."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        proofing_exempt,
        set_proofing_exempt,
        ProofingExempt,
        "noProof",
        "`w:noProof` — whether the paragraph mark is exempt from spelling and grammar checking."
    );
    toggle_property!(ParagraphMarkRunPropertyContent, snap_to_grid, set_snap_to_grid, SnapToGrid, "snapToGrid", "`w:snapToGrid` — whether the paragraph mark's inter-character spacing follows the document grid.");
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        hidden,
        set_hidden,
        Hidden,
        "vanish",
        "`w:vanish` — whether the paragraph mark is hidden text."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        web_hidden,
        set_web_hidden,
        WebHidden,
        "webHidden",
        "`w:webHidden` — whether the paragraph mark is hidden when displayed as a web page."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        right_to_left,
        set_right_to_left,
        RightToLeft,
        "rtl",
        "`w:rtl` — whether the paragraph mark displays right-to-left."
    );
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        complex_script,
        set_complex_script,
        ComplexScript,
        "cs",
        "`w:cs` — whether the paragraph mark uses complex-script formatting."
    );
    toggle_property!(ParagraphMarkRunPropertyContent, always_hidden, set_always_hidden, AlwaysHidden, "specVanish", "`w:specVanish` — whether the paragraph mark is always hidden, even when formatting marks otherwise display.");
    toggle_property!(
        ParagraphMarkRunPropertyContent,
        math,
        set_math,
        Math,
        "oMath",
        "`w:oMath` — whether the paragraph mark is typeset as Office Open XML Math."
    );

    value_property!(
        ParagraphMarkRunPropertyContent,
        character_style,
        set_character_style,
        CharacterStyle,
        CharacterStyle,
        "rStyle",
        "`w:rStyle` — the character style the paragraph mark refers to."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        fonts,
        set_fonts,
        Fonts,
        Fonts,
        "rFonts",
        "`w:rFonts` — the fonts the paragraph mark uses."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        color,
        set_color,
        Color,
        Color,
        "color",
        "`w:color` — the paragraph mark's text colour."
    );
    value_property!(ParagraphMarkRunPropertyContent, character_spacing, set_character_spacing, CharacterSpacing, SignedTwipsMeasureValue, "spacing", "`w:spacing` — the paragraph mark's character spacing adjustment, in twentieths of a point.");
    value_property!(
        ParagraphMarkRunPropertyContent,
        character_scale,
        set_character_scale,
        CharacterScale,
        TextScaleValue,
        "w",
        "`w:w` — the paragraph mark's horizontal character scale, as a percentage."
    );
    half_point_property!(ParagraphMarkRunPropertyContent, kerning, set_kerning, Kerning, "kern", "`w:kern` — the font-size threshold, in half-points, above which kerning applies to the paragraph mark.");
    value_property!(ParagraphMarkRunPropertyContent, vertical_offset, set_vertical_offset, VerticalOffset, SignedHalfPointMeasureValue, "position", "`w:position` — how far the paragraph mark is raised or lowered from the baseline, in half-points.");
    half_point_property!(
        ParagraphMarkRunPropertyContent,
        font_size,
        set_font_size,
        FontSize,
        "sz",
        "`w:sz` — the paragraph mark's font size, in half-points."
    );
    half_point_property!(
        ParagraphMarkRunPropertyContent,
        complex_script_font_size,
        set_complex_script_font_size,
        ComplexScriptFontSize,
        "szCs",
        "`w:szCs` — the paragraph mark's font size for complex-script text, in half-points."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        highlight,
        set_highlight,
        Highlight,
        Highlight,
        "highlight",
        "`w:highlight` — the paragraph mark's text-highlight colour."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        underline,
        set_underline,
        Underline,
        Underline,
        "u",
        "`w:u` — the paragraph mark's underline."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        text_effect,
        set_text_effect,
        TextEffect,
        TextEffect,
        "effect",
        "`w:effect` — the paragraph mark's animated text effect."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        border,
        set_border,
        Border,
        Border,
        "bdr",
        "`w:bdr` — the paragraph mark's own character border."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        shading,
        set_shading,
        Shading,
        Shading,
        "shd",
        "`w:shd` — the paragraph mark's own shading."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        manual_run_width,
        set_manual_run_width,
        ManualRunWidth,
        ManualRunWidth,
        "fitText",
        "`w:fitText` — the paragraph mark's manually fitted width."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        vertical_alignment,
        set_vertical_alignment,
        VerticalAlignment,
        VerticalAlignment,
        "vertAlign",
        "`w:vertAlign` — the paragraph mark's subscript/superscript position."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        emphasis_mark,
        set_emphasis_mark,
        Emphasis,
        Emphasis,
        "em",
        "`w:em` — the paragraph mark's emphasis mark."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        languages,
        set_languages,
        Languages,
        Languages,
        "lang",
        "`w:lang` — the languages to check the paragraph mark's text against."
    );
    value_property!(
        ParagraphMarkRunPropertyContent,
        east_asian_layout,
        set_east_asian_layout,
        EastAsianLayout,
        EastAsianLayout,
        "eastAsianLayout",
        "`w:eastAsianLayout` — the paragraph mark's East Asian typography settings."
    );
}

// -------------------------------------------------------------------------------------------
// CT_PPr (w:pPr) — the paragraph's own properties: CT_PPrBase's 33 children, then rPr, sectPr,
// pPrChange.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`ParagraphProperties`]: `CT_PPrBase`'s 33 members (see the module's own doc
/// comment), then `rPr` (`CT_ParaRPr`), `sectPr` (`CT_SectPr`) and `pPrChange` (`CT_PPrChange`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphPropertyContent {
    /// `w:pStyle` (§17.3.1.27, "Referenced Paragraph Style").
    Style(ParagraphStyle),
    /// `w:keepNext` (§17.3.1.15, "Keep Paragraph With Next Paragraph") — `CT_OnOff`.
    KeepWithNext(Toggle),
    /// `w:keepLines` (§17.3.1.14, "Keep All Lines On One Page") — `CT_OnOff`.
    KeepLinesTogether(Toggle),
    /// `w:pageBreakBefore` (§17.3.1.23, "Start Paragraph on Next Page") — `CT_OnOff`.
    PageBreakBefore(Toggle),
    /// `w:framePr` (§17.3.1.11, "Text Frame Properties").
    Frame(FrameProperties),
    /// `w:widowControl` (§17.3.1.44, "Allow First/Last Line to Display on a Separate Page") —
    /// `CT_OnOff`.
    WidowControl(Toggle),
    /// `w:numPr` (§17.3.1.19, "Numbering Definition Instance Reference").
    Numbering(NumberingProperties),
    /// `w:suppressLineNumbers` (§17.3.1.35, "Suppress Line Numbers for Paragraph") — `CT_OnOff`.
    SuppressLineNumbers(Toggle),
    /// `w:pBdr` (§17.3.1.24, "Paragraph Borders").
    Borders(ParagraphBorders),
    /// `w:shd` (§17.3.1.31, "Paragraph Shading") — reuses [`Shading`] (`CT_Shd`), the same complex
    /// type as a run's own `w:shd`.
    Shading(Shading),
    /// `w:tabs` (§17.3.1.38, "Set of Custom Tab Stops").
    TabStops(TabStops),
    /// `w:suppressAutoHyphens` (§17.3.1.34, "Suppress Hyphenation for Paragraph") — `CT_OnOff`.
    SuppressAutoHyphens(Toggle),
    /// `w:kinsoku` (§17.3.1.16, "Use East Asian Typography Rules for First and Last Character per
    /// Line") — `CT_OnOff`, **not** the two-attribute `CT_Kinsoku` type (see the module's own doc
    /// comment for why the ticket's own "Complex types" bullet is wrong about this one).
    EastAsianLineBreakingRules(Toggle),
    /// `w:wordWrap` (§17.3.1.45, "Allow Line Breaking At Character Level") — `CT_OnOff`.
    WordWrap(Toggle),
    /// `w:overflowPunct` (§17.3.1.21, "Allow Punctuation to Extend Past Text Extents") — `CT_OnOff`.
    OverflowPunctuation(Toggle),
    /// `w:topLinePunct` (§17.3.1.43, "Compress Punctuation at Start of a Line") — `CT_OnOff`.
    CompressPunctuationAtLineStart(Toggle),
    /// `w:autoSpaceDE` (§17.3.1.2, "Automatically Adjust Spacing of Latin and East Asian Text") —
    /// `CT_OnOff`.
    AutoSpaceLatinAndEastAsian(Toggle),
    /// `w:autoSpaceDN` (§17.3.1.3, "Automatically Adjust Spacing of East Asian Text and Numbers") —
    /// `CT_OnOff`.
    AutoSpaceEastAsianAndNumbers(Toggle),
    /// `w:bidi` (§17.3.1.6, "Right to Left Paragraph Layout") — `CT_OnOff`.
    RightToLeftLayout(Toggle),
    /// `w:adjustRightInd` (§17.3.1.1, "Automatically Adjust Right Indent When Using Document Grid")
    /// — `CT_OnOff`.
    AdjustRightIndentForDocumentGrid(Toggle),
    /// `w:snapToGrid` (§17.3.1.32, "Use Document Grid Settings for Inter-Line Paragraph Spacing") —
    /// `CT_OnOff`, distinct from a run's own `w:snapToGrid` (inter-*character* spacing).
    SnapToGrid(Toggle),
    /// `w:spacing` (§17.3.1.33, "Spacing Between Lines and Above/Below Paragraph").
    Spacing(Spacing),
    /// `w:ind` (§17.3.1.12, "Paragraph Indentation").
    Indentation(Indentation),
    /// `w:contextualSpacing` (§17.3.1.9, "Ignore Spacing Above and Below When Using Identical
    /// Styles") — `CT_OnOff`.
    ContextualSpacing(Toggle),
    /// `w:mirrorIndents` (§17.3.1.18, "Use Left/Right Indents as Inside/Outside Indents") —
    /// `CT_OnOff`.
    MirrorIndents(Toggle),
    /// `w:suppressOverlap` (§17.3.1.36, "Prevent Text Frames From Overlapping") — `CT_OnOff`.
    SuppressOverlap(Toggle),
    /// `w:jc` (§17.3.1.13, "Paragraph Alignment").
    Alignment(ParagraphAlignment),
    /// `w:textDirection` (§17.3.1.41, "Paragraph Text Flow Direction").
    TextDirection(ParagraphTextFlowDirection),
    /// `w:textAlignment` (§17.3.1.39, "Vertical Character Alignment on Line").
    VerticalCharacterAlignment(VerticalCharacterAlignment),
    /// `w:textboxTightWrap` (§17.3.1.40, "Allow Surrounding Paragraphs to Tight Wrap to Text Box
    /// Contents").
    TextBoxTightWrap(TextBoxTightWrapSetting),
    /// `w:outlineLvl` (§17.3.1.20, "Associated Outline Level").
    OutlineLevel(DecimalNumberValue),
    /// `w:divId` (§17.3.1.10, "Associated HTML div ID").
    AssociatedHtmlDivId(DecimalNumberValue),
    /// `w:cnfStyle` (§17.3.1.8, "Paragraph Conditional Formatting").
    ConditionalFormatting(ConditionalFormatting),
    /// `w:rPr` (`CT_ParaRPr`, "Run Properties for the Paragraph Mark", §17.3.1.29) — **not** a run's
    /// `w:rPr`; see the module's own doc comment.
    ParagraphMarkProperties(ParagraphMarkRunProperties),
    /// `w:sectPr` (`CT_SectPr`) — the section this paragraph ends (a "next page"/"continuous"/…
    /// section break). Its own content is MJXOFF-106's; the field exists here so a caller can reach
    /// it structurally rather than walking raw XML.
    SectionProperties(Unmodeled),
    /// `w:pPrChange` (`CT_PPrChange`) — the tracked-change wrapper around a previous `w:pPr`;
    /// MJXOFF-126's own scope, kept opaque here.
    Change(Unmodeled),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_PPr` (`w:pPr`, "Paragraph Properties", §17.3.1.26) — a paragraph's own properties: everything
/// `CT_PPrBase` declares (33 independently optional members, no schema-imposed relative order among
/// them — an `xsd:sequence`, so order *is* validity, unlike the run-properties `xsd:choice`), then
/// the paragraph mark's own run properties, the section this paragraph ends, and the tracked-change
/// wrapper.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct ParagraphProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pStyle", variant = Style, ty = ParagraphStyle),
        child(local = "keepNext", variant = KeepWithNext, ty = Toggle),
        child(local = "keepLines", variant = KeepLinesTogether, ty = Toggle),
        child(local = "pageBreakBefore", variant = PageBreakBefore, ty = Toggle),
        child(local = "framePr", variant = Frame, ty = FrameProperties),
        child(local = "widowControl", variant = WidowControl, ty = Toggle),
        child(local = "numPr", variant = Numbering, ty = NumberingProperties),
        child(local = "suppressLineNumbers", variant = SuppressLineNumbers, ty = Toggle),
        child(local = "pBdr", variant = Borders, ty = ParagraphBorders),
        child(local = "shd", variant = Shading, ty = Shading),
        child(local = "tabs", variant = TabStops, ty = TabStops),
        child(local = "suppressAutoHyphens", variant = SuppressAutoHyphens, ty = Toggle),
        child(local = "kinsoku", variant = EastAsianLineBreakingRules, ty = Toggle),
        child(local = "wordWrap", variant = WordWrap, ty = Toggle),
        child(local = "overflowPunct", variant = OverflowPunctuation, ty = Toggle),
        child(local = "topLinePunct", variant = CompressPunctuationAtLineStart, ty = Toggle),
        child(local = "autoSpaceDE", variant = AutoSpaceLatinAndEastAsian, ty = Toggle),
        child(local = "autoSpaceDN", variant = AutoSpaceEastAsianAndNumbers, ty = Toggle),
        child(local = "bidi", variant = RightToLeftLayout, ty = Toggle),
        child(local = "adjustRightInd", variant = AdjustRightIndentForDocumentGrid, ty = Toggle),
        child(local = "snapToGrid", variant = SnapToGrid, ty = Toggle),
        child(local = "spacing", variant = Spacing, ty = Spacing),
        child(local = "ind", variant = Indentation, ty = Indentation),
        child(local = "contextualSpacing", variant = ContextualSpacing, ty = Toggle),
        child(local = "mirrorIndents", variant = MirrorIndents, ty = Toggle),
        child(local = "suppressOverlap", variant = SuppressOverlap, ty = Toggle),
        child(local = "jc", variant = Alignment, ty = ParagraphAlignment),
        child(local = "textDirection", variant = TextDirection, ty = ParagraphTextFlowDirection),
        child(local = "textAlignment", variant = VerticalCharacterAlignment, ty = VerticalCharacterAlignment),
        child(local = "textboxTightWrap", variant = TextBoxTightWrap, ty = TextBoxTightWrapSetting),
        child(local = "outlineLvl", variant = OutlineLevel, ty = DecimalNumberValue),
        child(local = "divId", variant = AssociatedHtmlDivId, ty = DecimalNumberValue),
        child(local = "cnfStyle", variant = ConditionalFormatting, ty = ConditionalFormatting),
        child(local = "rPr", variant = ParagraphMarkProperties, ty = ParagraphMarkRunProperties),
        child(local = "sectPr", variant = SectionProperties, ty = Unmodeled),
        child(local = "pPrChange", variant = Change, ty = Unmodeled)
    )]
    content: Vec<ParagraphPropertyContent>,
}

impl ParagraphProperties {
    /// Builds a new, empty `w:pPr` — no properties, ready for this type's setters.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "pPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The schema rank of an existing content item, computed from the generated
    /// [`PARAGRAPH_PROPERTIES`] table by the item's own wire name — `CT_PPrBase` (and `CT_PPr`'s own
    /// splice) is an `xsd:sequence`, so every one of the 36 members has its own unique rank, unlike
    /// [`ParagraphMarkRunProperties::rank`]'s two-tier `xsd:choice`.
    fn rank(item: &ParagraphPropertyContent) -> Option<u16> {
        let local = match item {
            ParagraphPropertyContent::Style(_) => "pStyle",
            ParagraphPropertyContent::KeepWithNext(_) => "keepNext",
            ParagraphPropertyContent::KeepLinesTogether(_) => "keepLines",
            ParagraphPropertyContent::PageBreakBefore(_) => "pageBreakBefore",
            ParagraphPropertyContent::Frame(_) => "framePr",
            ParagraphPropertyContent::WidowControl(_) => "widowControl",
            ParagraphPropertyContent::Numbering(_) => "numPr",
            ParagraphPropertyContent::SuppressLineNumbers(_) => "suppressLineNumbers",
            ParagraphPropertyContent::Borders(_) => "pBdr",
            ParagraphPropertyContent::Shading(_) => "shd",
            ParagraphPropertyContent::TabStops(_) => "tabs",
            ParagraphPropertyContent::SuppressAutoHyphens(_) => "suppressAutoHyphens",
            ParagraphPropertyContent::EastAsianLineBreakingRules(_) => "kinsoku",
            ParagraphPropertyContent::WordWrap(_) => "wordWrap",
            ParagraphPropertyContent::OverflowPunctuation(_) => "overflowPunct",
            ParagraphPropertyContent::CompressPunctuationAtLineStart(_) => "topLinePunct",
            ParagraphPropertyContent::AutoSpaceLatinAndEastAsian(_) => "autoSpaceDE",
            ParagraphPropertyContent::AutoSpaceEastAsianAndNumbers(_) => "autoSpaceDN",
            ParagraphPropertyContent::RightToLeftLayout(_) => "bidi",
            ParagraphPropertyContent::AdjustRightIndentForDocumentGrid(_) => "adjustRightInd",
            ParagraphPropertyContent::SnapToGrid(_) => "snapToGrid",
            ParagraphPropertyContent::Spacing(_) => "spacing",
            ParagraphPropertyContent::Indentation(_) => "ind",
            ParagraphPropertyContent::ContextualSpacing(_) => "contextualSpacing",
            ParagraphPropertyContent::MirrorIndents(_) => "mirrorIndents",
            ParagraphPropertyContent::SuppressOverlap(_) => "suppressOverlap",
            ParagraphPropertyContent::Alignment(_) => "jc",
            ParagraphPropertyContent::TextDirection(_) => "textDirection",
            ParagraphPropertyContent::VerticalCharacterAlignment(_) => "textAlignment",
            ParagraphPropertyContent::TextBoxTightWrap(_) => "textboxTightWrap",
            ParagraphPropertyContent::OutlineLevel(_) => "outlineLvl",
            ParagraphPropertyContent::AssociatedHtmlDivId(_) => "divId",
            ParagraphPropertyContent::ConditionalFormatting(_) => "cnfStyle",
            ParagraphPropertyContent::ParagraphMarkProperties(_) => "rPr",
            ParagraphPropertyContent::SectionProperties(_) => "sectPr",
            ParagraphPropertyContent::Change(_) => "pPrChange",
            ParagraphPropertyContent::Raw(_) => return None,
        };
        PARAGRAPH_PROPERTIES.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&ParagraphPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: ParagraphPropertyContent) {
        let at =
            PARAGRAPH_PROPERTIES.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&ParagraphPropertyContent) -> bool,
        value: Option<ParagraphPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    // The eighteen `CT_OnOff` members.
    toggle_property!(
        ParagraphPropertyContent,
        keep_with_next,
        set_keep_with_next,
        KeepWithNext,
        "keepNext",
        "`w:keepNext` — whether this paragraph stays on the same page as the one after it."
    );
    toggle_property!(
        ParagraphPropertyContent,
        keep_lines_together,
        set_keep_lines_together,
        KeepLinesTogether,
        "keepLines",
        "`w:keepLines` — whether this paragraph's lines all stay on one page."
    );
    toggle_property!(
        ParagraphPropertyContent,
        page_break_before,
        set_page_break_before,
        PageBreakBefore,
        "pageBreakBefore",
        "`w:pageBreakBefore` — whether this paragraph starts on a new page."
    );
    toggle_property!(
        ParagraphPropertyContent,
        widow_control,
        set_widow_control,
        WidowControl,
        "widowControl",
        "`w:widowControl` — whether this paragraph's first/last line may display alone on a page."
    );
    toggle_property!(
        ParagraphPropertyContent,
        suppress_line_numbers,
        set_suppress_line_numbers,
        SuppressLineNumbers,
        "suppressLineNumbers",
        "`w:suppressLineNumbers` — whether line numbering skips this paragraph."
    );
    toggle_property!(
        ParagraphPropertyContent,
        suppress_auto_hyphens,
        set_suppress_auto_hyphens,
        SuppressAutoHyphens,
        "suppressAutoHyphens",
        "`w:suppressAutoHyphens` — whether automatic hyphenation is suppressed for this paragraph."
    );
    toggle_property!(ParagraphPropertyContent, east_asian_line_breaking_rules, set_east_asian_line_breaking_rules, EastAsianLineBreakingRules, "kinsoku", "`w:kinsoku` — whether East Asian line-breaking rules apply to this paragraph's first and last characters per line.");
    toggle_property!(
        ParagraphPropertyContent,
        word_wrap,
        set_word_wrap,
        WordWrap,
        "wordWrap",
        "`w:wordWrap` — whether a line may break within a word that would otherwise overflow."
    );
    toggle_property!(
        ParagraphPropertyContent,
        overflow_punctuation,
        set_overflow_punctuation,
        OverflowPunctuation,
        "overflowPunct",
        "`w:overflowPunct` — whether punctuation may extend past the text extents."
    );
    toggle_property!(
        ParagraphPropertyContent,
        compress_punctuation_at_line_start,
        set_compress_punctuation_at_line_start,
        CompressPunctuationAtLineStart,
        "topLinePunct",
        "`w:topLinePunct` — whether punctuation compresses at the start of a line."
    );
    toggle_property!(ParagraphPropertyContent, auto_space_latin_and_east_asian, set_auto_space_latin_and_east_asian, AutoSpaceLatinAndEastAsian, "autoSpaceDE", "`w:autoSpaceDE` — whether spacing between Latin and East Asian text is adjusted automatically.");
    toggle_property!(ParagraphPropertyContent, auto_space_east_asian_and_numbers, set_auto_space_east_asian_and_numbers, AutoSpaceEastAsianAndNumbers, "autoSpaceDN", "`w:autoSpaceDN` — whether spacing between East Asian text and numbers is adjusted automatically.");
    toggle_property!(
        ParagraphPropertyContent,
        right_to_left_layout,
        set_right_to_left_layout,
        RightToLeftLayout,
        "bidi",
        "`w:bidi` — whether this paragraph lays out right-to-left."
    );
    toggle_property!(ParagraphPropertyContent, adjust_right_indent_for_document_grid, set_adjust_right_indent_for_document_grid, AdjustRightIndentForDocumentGrid, "adjustRightInd", "`w:adjustRightInd` — whether the right indent is adjusted automatically when using the document grid.");
    toggle_property!(
        ParagraphPropertyContent,
        snap_to_grid,
        set_snap_to_grid,
        SnapToGrid,
        "snapToGrid",
        "`w:snapToGrid` — whether this paragraph's inter-line spacing follows the document grid."
    );
    toggle_property!(ParagraphPropertyContent, contextual_spacing, set_contextual_spacing, ContextualSpacing, "contextualSpacing", "`w:contextualSpacing` — whether spacing above/below is ignored between paragraphs of the same style.");
    toggle_property!(
        ParagraphPropertyContent,
        mirror_indents,
        set_mirror_indents,
        MirrorIndents,
        "mirrorIndents",
        "`w:mirrorIndents` — whether the left/right indents are used as inside/outside indents."
    );
    toggle_property!(ParagraphPropertyContent, suppress_overlap, set_suppress_overlap, SuppressOverlap, "suppressOverlap", "`w:suppressOverlap` — whether this paragraph's text frame is prevented from overlapping others.");

    // The thirteen straightforward whole-value members.
    value_property!(
        ParagraphPropertyContent,
        style,
        set_style,
        Style,
        ParagraphStyle,
        "pStyle",
        "`w:pStyle` — the paragraph style this paragraph refers to."
    );
    value_property!(
        ParagraphPropertyContent,
        frame,
        set_frame,
        Frame,
        FrameProperties,
        "framePr",
        "`w:framePr` — this paragraph's legacy text-frame properties."
    );
    value_property!(
        ParagraphPropertyContent,
        numbering,
        set_numbering,
        Numbering,
        NumberingProperties,
        "numPr",
        "`w:numPr` — this paragraph's numbering-definition reference."
    );
    value_property!(
        ParagraphPropertyContent,
        borders,
        set_borders,
        Borders,
        ParagraphBorders,
        "pBdr",
        "`w:pBdr` — this paragraph's borders."
    );
    value_property!(
        ParagraphPropertyContent,
        shading,
        set_shading,
        Shading,
        Shading,
        "shd",
        "`w:shd` — this paragraph's shading."
    );
    value_property!(
        ParagraphPropertyContent,
        tab_stops,
        set_tab_stops,
        TabStops,
        TabStops,
        "tabs",
        "`w:tabs` — this paragraph's custom tab stops."
    );
    value_property!(
        ParagraphPropertyContent,
        alignment,
        set_alignment,
        Alignment,
        ParagraphAlignment,
        "jc",
        "`w:jc` — this paragraph's justification."
    );
    value_property!(
        ParagraphPropertyContent,
        text_direction,
        set_text_direction,
        TextDirection,
        ParagraphTextFlowDirection,
        "textDirection",
        "`w:textDirection` — this paragraph's text flow direction."
    );
    value_property!(
        ParagraphPropertyContent,
        vertical_character_alignment,
        set_vertical_character_alignment,
        VerticalCharacterAlignment,
        VerticalCharacterAlignment,
        "textAlignment",
        "`w:textAlignment` — how this paragraph's characters align vertically on the line."
    );
    value_property!(ParagraphPropertyContent, text_box_tight_wrap, set_text_box_tight_wrap, TextBoxTightWrap, TextBoxTightWrapSetting, "textboxTightWrap", "`w:textboxTightWrap` — whether surrounding paragraphs tight-wrap to this paragraph's text box contents.");
    value_property!(ParagraphPropertyContent, conditional_formatting, set_conditional_formatting, ConditionalFormatting, ConditionalFormatting, "cnfStyle", "`w:cnfStyle` — this paragraph's conditional formatting reference. Its meaning inside a table is MJXOFF-116's.");

    decimal_number_property!(
        ParagraphPropertyContent,
        outline_level,
        set_outline_level,
        OutlineLevel,
        "outlineLvl",
        "`w:outlineLvl` — this paragraph's associated outline level."
    );
    decimal_number_property!(
        ParagraphPropertyContent,
        associated_html_div_id,
        set_associated_html_div_id,
        AssociatedHtmlDivId,
        "divId",
        "`w:divId` — this paragraph's associated HTML `div` id."
    );

    /// This paragraph's spacing (`w:spacing`), or `None` if it carries none.
    #[must_use]
    pub fn spacing(&self) -> Option<&Spacing> {
        self.content.iter().find_map(|item| match item {
            ParagraphPropertyContent::Spacing(spacing) => Some(spacing),
            _ => None,
        })
    }

    /// Sets `w:spacing`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    /// Editing one paragraph's spacing this way never touches any other paragraph, or this
    /// paragraph's own runs or paragraph-mark properties — it replaces exactly this one content item.
    pub fn set_spacing(&mut self, value: Option<Spacing>) {
        let is_target =
            |item: &ParagraphPropertyContent| matches!(item, ParagraphPropertyContent::Spacing(_));
        self.set(
            "spacing",
            is_target,
            value.map(ParagraphPropertyContent::Spacing),
        );
    }

    /// This paragraph's indentation (`w:ind`), or `None` if it carries none.
    #[must_use]
    pub fn indentation(&self) -> Option<&Indentation> {
        self.content.iter().find_map(|item| match item {
            ParagraphPropertyContent::Indentation(indentation) => Some(indentation),
            _ => None,
        })
    }

    /// Sets `w:ind`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_indentation(&mut self, value: Option<Indentation>) {
        let is_target = |item: &ParagraphPropertyContent| {
            matches!(item, ParagraphPropertyContent::Indentation(_))
        };
        self.set(
            "ind",
            is_target,
            value.map(ParagraphPropertyContent::Indentation),
        );
    }

    /// The paragraph mark's own run properties (`w:pPr/w:rPr`, [`ParagraphMarkRunProperties`]) — the
    /// pilcrow's formatting, **not** this paragraph's runs' formatting (see the module's own doc
    /// comment) — or `None` if this `w:pPr` carries none.
    #[must_use]
    pub fn paragraph_mark_properties(&self) -> Option<&ParagraphMarkRunProperties> {
        self.content.iter().find_map(|item| match item {
            ParagraphPropertyContent::ParagraphMarkProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The paragraph mark's own run properties, mutably, or `None` if this `w:pPr` carries none — see
    /// [`ParagraphProperties::paragraph_mark_properties_or_insert`] to create one.
    pub fn paragraph_mark_properties_mut(&mut self) -> Option<&mut ParagraphMarkRunProperties> {
        self.content.iter_mut().find_map(|item| match item {
            ParagraphPropertyContent::ParagraphMarkProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The paragraph mark's own run properties, mutably — creating an empty `w:rPr` at its schema
    /// rank if this `w:pPr` does not already carry one. Setting a property through the result can
    /// never disturb this paragraph's own runs' `w:rPr` — they are different content vectors on
    /// different types (see the module's own doc comment).
    pub fn paragraph_mark_properties_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut ParagraphMarkRunProperties {
        let is_target = |item: &ParagraphPropertyContent| {
            matches!(item, ParagraphPropertyContent::ParagraphMarkProperties(_))
        };
        if !self.content.iter().any(is_target) {
            self.insert(
                "rPr",
                ParagraphPropertyContent::ParagraphMarkProperties(ParagraphMarkRunProperties::new(
                    interner,
                )),
            );
        }
        match self.content.iter_mut().find_map(|item| match item {
            ParagraphPropertyContent::ParagraphMarkProperties(properties) => Some(properties),
            _ => None,
        }) {
            Some(properties) => properties,
            None => unreachable!("just found or inserted above"),
        }
    }

    /// The section this paragraph ends (`w:sectPr`), or `None` if it carries none. The section's own
    /// content is MJXOFF-106's.
    #[must_use]
    pub fn section_properties(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            ParagraphPropertyContent::SectionProperties(section) => Some(section),
            _ => None,
        })
    }

    /// The tracked-change wrapper around a previous `w:pPr` (`w:pPrChange`), or `None` if this
    /// `w:pPr` carries none. Its semantics are MJXOFF-126's.
    #[must_use]
    pub fn change(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            ParagraphPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_core::RawDocument;
    use mjx_xml::fidelity;

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

    /// Would this pass if the work were not done? No: a container derive that hard-codes `empty =
    /// true` (or drops content) turns the second half of each pair red, exactly as
    /// `run_properties.rs`'s own version of this test documents.
    #[test]
    fn paragraph_properties_preserves_self_closed_and_separate_end_tag_emptiness() {
        let self_closed = format!(r#"<w:pPr xmlns:w="{W}"/>"#).into_bytes();
        let (ppr, doc): (ParagraphProperties, _) = parse_typed(&self_closed);
        assert!(ppr.content.is_empty());
        assert_round_trips(&ppr, doc, &self_closed);

        let separate_end_tag = format!(r#"<w:pPr xmlns:w="{W}"></w:pPr>"#).into_bytes();
        let (ppr, doc): (ParagraphProperties, _) = parse_typed(&separate_end_tag);
        assert!(ppr.content.is_empty());
        assert_round_trips(&ppr, doc, &separate_end_tag);
    }

    /// Would this pass if the work were not done? No: if `Spacing::line_spacing` read `w:line` alone
    /// (ignoring `w:lineRule`), this would still compile and pass with the *wrong* rule for the
    /// second case — the doctest on [`Spacing::line_spacing`] is the "no such accessor exists"
    /// half of the proof; this is the "the value that does exist is correctly paired" half.
    #[test]
    fn line_spacing_pairs_the_rule_with_the_value_on_read_and_write() {
        let mut interner = Interner::new();
        let mut spacing = Spacing::new(&mut interner);
        assert_eq!(spacing.line_spacing(&interner), Ok(None));

        spacing.set_line_spacing(
            &mut interner,
            Some(LineSpacing {
                rule: LineSpacingRule::AtLeast,
                value: SignedTwipsMeasure::from_wire("240"),
            }),
        );
        let read = spacing
            .line_spacing(&interner)
            .expect("valid")
            .expect("set above");
        assert_eq!(read.rule, LineSpacingRule::AtLeast);
        assert_eq!(read.value, SignedTwipsMeasure::from_wire("240"));

        spacing.set_line_spacing(&mut interner, None);
        assert_eq!(spacing.line_spacing(&interner), Ok(None));
    }

    /// `w:lineRule`'s own schema default (`auto`) is applied — never written — when `w:line` is
    /// present without it.
    #[test]
    fn line_present_without_line_rule_defaults_the_rule_to_auto() {
        let fragment = format!(r#"<w:spacing xmlns:w="{W}" w:line="360"/>"#).into_bytes();
        let (spacing, doc): (Spacing, _) = parse_typed(&fragment);
        let read = spacing
            .line_spacing(&doc.interner)
            .expect("valid")
            .expect("line present");
        assert_eq!(read.rule, LineSpacingRule::Auto);
        assert_eq!(read.value, SignedTwipsMeasure::from_wire("360"));
    }

    /// Would this pass if the work were not done? No: if `leading_edge`/`trailing_edge` simply read
    /// `w:left`/`w:right` (or averaged the two, or picked whichever came first in the file), this
    /// fragment — `start` and `left` both present, `start` written second — would read back `100`
    /// instead of `720`.
    #[test]
    fn leading_and_trailing_edge_prefer_the_logical_spelling_when_both_are_present() {
        let fragment = format!(
            r#"<w:ind xmlns:w="{W}" w:left="100" w:start="720" w:right="50" w:end="480"/>"#
        )
        .into_bytes();
        let (ind, doc): (Indentation, _) = parse_typed(&fragment);
        assert_eq!(
            ind.leading_edge(&doc.interner),
            Ok(Some(SignedTwipsMeasure::from_wire("720")))
        );
        assert_eq!(
            ind.trailing_edge(&doc.interner),
            Ok(Some(SignedTwipsMeasure::from_wire("480")))
        );
        // Both spellings stay individually readable — nothing is normalized away.
        assert_eq!(
            ind.left(&doc.interner),
            Ok(Some(SignedTwipsMeasure::from_wire("100")))
        );
        assert_eq!(
            ind.right(&doc.interner),
            Ok(Some(SignedTwipsMeasure::from_wire("50")))
        );
    }

    #[test]
    fn leading_edge_falls_back_to_the_physical_spelling_when_logical_is_absent() {
        let fragment = format!(r#"<w:ind xmlns:w="{W}" w:left="200"/>"#).into_bytes();
        let (ind, doc): (Indentation, _) = parse_typed(&fragment);
        assert_eq!(
            ind.leading_edge(&doc.interner),
            Ok(Some(SignedTwipsMeasure::from_wire("200")))
        );
    }

    /// Would this pass if the work were not done? No: a model that special-cased `val="clear"` (to
    /// drop it, or to treat it as removing a sibling within the same list rather than a runtime
    /// concept the paragraph's inherited tabs resolve) would either lose this tab stop on round-trip
    /// or reorder it relative to its neighbour.
    #[test]
    fn a_clear_tab_stop_round_trips_like_any_other() {
        let fragment = format!(
            r#"<w:tabs xmlns:w="{W}"><w:tab w:val="clear" w:pos="720"/><w:tab w:val="left" w:pos="1440"/></w:tabs>"#
        )
        .into_bytes();
        let (tabs, doc): (TabStops, _) = parse_typed(&fragment);
        assert_eq!(tabs.tab_count(), 2);
        let first = tabs.tabs().next().expect("first tab");
        assert_eq!(first.alignment(&doc.interner), Ok(TabStopType::Clear));
        assert_round_trips(&tabs, doc, &fragment);
    }

    /// The central trap this child exists to avoid: setting the paragraph's own justification must
    /// not touch the paragraph mark's `w:rPr`, and editing the paragraph mark's own properties must
    /// not touch the paragraph's justification. Both directions asserted, matching the ticket's own
    /// "Done when" wording.
    ///
    /// Would this pass if the work were not done? No: if `set_alignment` rebuilt or cleared the whole
    /// `content` vector instead of replacing one item, `w:rPr/w:b` would read back `None`, not
    /// `Some(true)`.
    #[test]
    fn setting_justification_leaves_the_paragraph_mark_rpr_untouched_and_vice_versa() {
        let fragment =
            format!(r#"<w:pPr xmlns:w="{W}"><w:rPr><w:b/></w:rPr></w:pPr>"#).into_bytes();
        let (mut ppr, doc): (ParagraphProperties, _) = parse_typed(&fragment);
        let mut interner = doc.interner;

        ppr.set_alignment(Some(ParagraphAlignment::new(
            &mut interner,
            Justification::Center,
        )));
        let mark = ppr
            .paragraph_mark_properties()
            .expect("still carries w:rPr");
        assert_eq!(
            mark.bold(&interner),
            Ok(Some(true)),
            "w:rPr/w:b must be unaffected by set_alignment"
        );

        ppr.paragraph_mark_properties_or_insert(&mut interner)
            .set_italic(&mut interner, Some(true));
        assert_eq!(
            ppr.alignment().map(|a| a.value(&interner)),
            Some(Ok(Justification::Center)),
            "w:jc must be unaffected by editing the paragraph mark"
        );
        let mark = ppr
            .paragraph_mark_properties()
            .expect("still carries w:rPr");
        assert_eq!(
            mark.bold(&interner),
            Ok(Some(true)),
            "w:rPr/w:b must survive editing w:i"
        );
        assert_eq!(mark.italic(&interner), Ok(Some(true)));
    }

    /// The mutation gate: a `w:pPr` whose two children are stored out of `xsd:sequence` order (`jc`
    /// before `pStyle`) becomes schema-ordered once one of them is edited — `insert_index_of_names`
    /// computes the correct position from `PARAGRAPH_PROPERTIES`, not from where the caller happened
    /// to put things.
    ///
    /// Would this pass if the work were not done? No — see this child's own report for the mutation
    /// proof (`ParagraphProperties::insert` neutralized to always append) confirming this test goes
    /// red.
    #[test]
    fn an_out_of_order_ppr_becomes_schema_ordered_after_editing_one_property() {
        let fragment = format!(
            r#"<w:pPr xmlns:w="{W}"><w:jc w:val="center"/><w:pStyle w:val="Heading1"/></w:pPr>"#
        )
        .into_bytes();
        let doc = fidelity::parse(&fragment).expect("fragment parses");
        assert!(
            mjx_ooxml_types::child_order::PARAGRAPH_PROPERTIES
                .first_out_of_order(&doc.root, &doc.interner, "w:pPr")
                .is_some(),
            "the fixture must genuinely start out of schema order"
        );

        let mut ppr = ParagraphProperties::from_xml(&doc.root, &doc.interner)
            .expect("parses as ParagraphProperties");
        let mut interner = doc.interner;

        ppr.set_style(Some(ParagraphStyle::new(&mut interner, "Heading1")));

        let rebuilt = ppr.to_xml(&mut interner);
        assert_eq!(
            mjx_ooxml_types::child_order::PARAGRAPH_PROPERTIES
                .first_out_of_order(&rebuilt, &interner, "w:pPr"),
            None,
            "PARAGRAPH_PROPERTIES.insert_index_of_names must place pStyle before jc"
        );
    }
}
