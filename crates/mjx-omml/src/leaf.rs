//! The leaf value types: elements shaped as "one wire name, one attribute, no element children".
//!
//! **Twenty XSD symbols share one Rust mechanism here** — `CT_Integer255`, `CT_Integer2`,
//! `CT_SpacingRule`, `CT_UnSignedInteger`, `CT_Char`, `CT_OnOff`, `CT_String`, `CT_XAlign`,
//! `CT_YAlign`, `CT_Shp`, `CT_FType`, `CT_LimLoc`, `CT_TopBot`, `CT_Script`, `CT_Style`,
//! `CT_OMathJc`, `CT_BreakBin`, `CT_BreakBinSub`, `CT_TwipsMeasure` and `CT_ManualBreak` — the same
//! "one shape, many meanings, the wire name carries which" reuse `mjx-docx` already establishes for
//! `CT_OnOff` (`run_properties::Toggle`) and `CT_String` (`styles::StyleString`; see that module's own
//! comment). Rather than twenty structurally-identical Rust types, [`crate::support::read_val_child`]/
//! [`crate::support::val_element`] read and write the shared `val` attribute through whichever
//! [`AttributeCodec`] the caller names, and each higher-level `*Pr` accessor in [`crate::objects`] and
//! [`crate::math`] calls one of the typed wrappers below once, by name.
//!
//! Also here: [`RunProperties`] (`m:rPr`, `CT_RPR` — Office Math's *own* run properties, distinct
//! from `w:rPr`), [`ControlProperties`] (`m:ctrlPr`, `CT_CtrlPr` — the layering-tension type; see the
//! crate's own module doc comment), and [`Text`] (`m:t`, `CT_Text`).

use mjx_ooxml_core::{
    AttributeCodec, Interner, InvalidAttributeValue, Number, RawAttribute, RawElement, RawName,
    RawNode,
};
use mjx_ooxml_types::officemath::{Character, ScriptType};
use mjx_ooxml_types::support::OnOff as OnOffCodec;
use std::borrow::Cow;

use crate::support::{fidelity_element_impls, m_child, m_name, read_val_child, val_element};

/// `ST_Char`/`CT_Char`'s `val` as an attribute value. Never rejects — the schema's
/// `xsd:maxLength="1"` is not enforced here, per this project's "read never normalizes" rule (a
/// producer that writes more than one character still opens; canonicalization is what
/// [`AttributeCodec::encode`] alone is responsible for, and this codec has one output per value
/// regardless of length).
#[derive(Debug)]
pub(crate) struct CharacterCodec;

impl AttributeCodec for CharacterCodec {
    type Value<'a> = Character;
    type Input<'a> = Character;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Character, InvalidAttributeValue> {
        Ok(Character::from_wire(&raw))
    }

    fn encode<'a>(value: Character) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `s:ST_String`/`CT_String`'s `val` as an attribute value (`m:mathFont`'s font name). Never rejects.
#[derive(Debug)]
pub(crate) struct MathFontNameCodec;

impl AttributeCodec for MathFontNameCodec {
    type Value<'a> = mjx_ooxml_types::shared::XmlString;
    type Input<'a> = mjx_ooxml_types::shared::XmlString;

    fn decode<'a>(
        raw: Cow<'a, str>,
    ) -> Result<mjx_ooxml_types::shared::XmlString, InvalidAttributeValue> {
        Ok(mjx_ooxml_types::shared::XmlString::from_wire(&raw))
    }

    fn encode<'a>(value: mjx_ooxml_types::shared::XmlString) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `s:ST_TwipsMeasure`/`CT_TwipsMeasure`'s `val` as an attribute value (`m:lMargin`, `m:preSp`, …).
/// Never rejects: the union type (`ST_UnsignedDecimalNumber | ST_PositiveUniversalMeasure`) is kept
/// as its wire string, exactly as `mjx-ooxml-types` generates it — see that type's own doc comment
/// for why `mjx-docx`'s page margins do the same rather than parsing the union.
#[derive(Debug)]
pub(crate) struct TwipsCodec;

impl AttributeCodec for TwipsCodec {
    type Value<'a> = mjx_ooxml_types::shared::TwipsMeasure;
    type Input<'a> = mjx_ooxml_types::shared::TwipsMeasure;

    fn decode<'a>(
        raw: Cow<'a, str>,
    ) -> Result<mjx_ooxml_types::shared::TwipsMeasure, InvalidAttributeValue> {
        Ok(mjx_ooxml_types::shared::TwipsMeasure::from_wire(&raw))
    }

    fn encode<'a>(value: mjx_ooxml_types::shared::TwipsMeasure) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// Reads the `val` of the first `m:{local}` child of `children` as an `ST_OnOff` boolean, applying
/// the family's own convention: the element's mere **presence** already means `true` (`val` absent),
/// `val="false"`/`0`/`off` negates it, and the element's **absence** means "unstated" (`None`) —
/// exactly [`mjx_docx::document::run_properties::Toggle::value`]'s own semantics (unnamed here since
/// this crate cannot depend on `mjx-docx`; restated because the schema shape — an unadorned toggle
/// element — is identical). A `val` present but unreadable is treated as absent, the same leniency
/// every other optional accessor in this crate applies.
pub(crate) fn read_onoff_child(
    children: &[RawNode],
    interner: &Interner,
    local: &str,
) -> Option<bool> {
    let element = m_child(children, interner, local)?;
    match crate::support::read_val::<OnOffCodec>(element, interner) {
        Ok(Some(value)) => Some(value),
        Ok(None) => Some(true),
        Err(_) => None,
    }
}

/// Builds `<m:{local}/>` for `true` or `<m:{local} m:val="false"/>` for `false` — the write side of
/// [`read_onoff_child`], always writing the canonical spelling.
pub(crate) fn onoff_element(interner: &mut Interner, local: &str, value: bool) -> RawElement {
    if value {
        RawElement::new(m_name(interner, local), Vec::new(), Vec::new(), true)
    } else {
        val_element::<OnOffCodec>(interner, local, false)
    }
}

// -------------------------------------------------------------------------------------------------
// CT_ManualBreak (m:brk) — the one leaf whose attribute is not `val`.
// -------------------------------------------------------------------------------------------------

/// Reads `m:alnAt` of the first `m:brk` child, or `None` if there is none or it carries no `alnAt`.
/// `shared-math.xsd` is `attributeFormDefault="qualified"` (see `crate::support`'s own
/// `VAL_ATTRIBUTE_PREFIX` doc comment), so the wire attribute is `m:alnAt`, never a bare `alnAt`.
pub(crate) fn read_manual_break_align_at(children: &[RawNode], interner: &Interner) -> Option<i64> {
    let element = m_child(children, interner, "brk")?;
    mjx_xml::attribute::read::<Number<i64>>(
        &element.attributes,
        interner,
        Some("m"),
        "alnAt",
        "m:alnAt",
    )
    .ok()
    .flatten()
}

// -------------------------------------------------------------------------------------------------
// CT_RPR (m:rPr) — Office Math's own run properties. Distinct from `w:rPr`: `lit`/`nor`/`scr`/`sty`/
// `brk`/`aln`, none of them WordprocessingML.
// -------------------------------------------------------------------------------------------------

/// `m:rPr` (`CT_RPR`, §22.1.2.85 "Run Properties (Math)") — literal-run flag, normal-text-or-script-
/// style choice, manual break, and alignment-point flag. Distinct from `w:rPr`
/// ([`ControlProperties`]'s own concern).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(RunProperties);

impl RunProperties {
    /// Builds an empty `<m:rPr/>`; every field starts unstated.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: m_name(interner, "rPr"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// `m:lit` — whether this run's contents are literal text, exempt from the math-typesetting
    /// transformations (italicisation of single-letter identifiers, and so on).
    #[must_use]
    pub fn literal(&self, interner: &Interner) -> Option<bool> {
        read_onoff_child(&self.children, interner, "lit")
    }

    /// `m:nor` — whether this run is "normal text" (exempt from math typesetting), the other half of
    /// `EG_ScriptStyle`'s choice with [`RunProperties::script`]/[`RunProperties::style`].
    #[must_use]
    pub fn normal_text(&self, interner: &Interner) -> Option<bool> {
        read_onoff_child(&self.children, interner, "nor")
    }

    /// `m:scr` (`CT_Script`) — the script typeface applied to this run (roman, fraktur, …).
    #[must_use]
    pub fn script(&self, interner: &Interner) -> Option<ScriptType> {
        read_val_child::<mjx_ooxml_core::Enumeration<ScriptType>>(&self.children, interner, "scr")
    }

    /// `m:sty` (`CT_Style`) — the character style applied to this run (plain, bold, italic, bold
    /// italic).
    #[must_use]
    pub fn style(&self, interner: &Interner) -> Option<mjx_ooxml_types::officemath::MathStyle> {
        read_val_child::<mjx_ooxml_core::Enumeration<mjx_ooxml_types::officemath::MathStyle>>(
            &self.children,
            interner,
            "sty",
        )
    }

    /// `m:brk` (`CT_ManualBreak`) — a manual line break's own alignment point (`@alnAt`), or `None`
    /// if this run declares no manual break.
    #[must_use]
    pub fn manual_break_align_at(&self, interner: &Interner) -> Option<i64> {
        read_manual_break_align_at(&self.children, interner)
    }

    /// `m:aln` — whether this run participates in alignment-point alignment.
    #[must_use]
    pub fn alignment(&self, interner: &Interner) -> Option<bool> {
        read_onoff_child(&self.children, interner, "aln")
    }
}

// -------------------------------------------------------------------------------------------------
// CT_CtrlPr (m:ctrlPr) — the layering-tension type. See the crate's own module doc comment: every
// legal child (`w:rPr`, `w:ins`/`CT_MathCtrlIns`, `w:del`/`CT_MathCtrlDel`) is a WordprocessingML
// type this crate — sitting *below* `mjx-docx` — cannot model. Preserved wholesale, raw, exactly as
// `mjx-dml`'s `WordprocessingGroup`/`WordprocessingCanvas` preserve their own WordprocessingML-typed
// member content for the identical reason. `mjx-docx` (which depends on this crate) adds typed
// accessors *over* a `ControlProperties`' raw children where it needs them — see
// `crates/mjx-docx/src/document/revisions.rs`'s own `CT_MathCtrlIns`/`CT_MathCtrlDel` types.
// -------------------------------------------------------------------------------------------------

/// `m:ctrlPr` (`CT_CtrlPr`, §22.1.2.24 "Control Properties") — every math object's own optional
/// pass-through of run-level control properties (bold/italic/etc., or a tracked math-control
/// insertion/deletion) onto its container run. Every child is WordprocessingML-typed; see this
/// module's own doc comment for why they are preserved raw rather than decomposed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(ControlProperties);

impl ControlProperties {
    /// Builds an empty `<m:ctrlPr/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: m_name(interner, "ctrlPr"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// The raw children this control-properties element carries — a `w:rPr`, a `w:ins`
    /// (`CT_MathCtrlIns`) or a `w:del` (`CT_MathCtrlDel`), per `EG_RPrMath`'s choice. `mjx-docx`
    /// reads these through its own typed `CT_MathCtrlIns`/`CT_MathCtrlDel`/`RunProperties`.
    #[must_use]
    pub fn raw_children(&self) -> &[RawNode] {
        &self.children
    }

    /// Replaces this control-properties element's children wholesale — the write side
    /// [`ControlProperties::raw_children`] has none of, because a typed `w:rPr`/`w:ins`/`w:del` is
    /// `mjx-docx`'s to build, not this crate's.
    pub fn set_raw_children(&mut self, children: Vec<RawNode>) {
        self.empty = children.is_empty() && self.empty;
        self.children = children;
    }
}

// -------------------------------------------------------------------------------------------------
// CT_Text (m:t) — simple content: an `s:ST_String` value plus an optional `xml:space`.
// -------------------------------------------------------------------------------------------------

/// `m:t` (`CT_Text`, §22.1.2.101 "Text") — one run's literal character data.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = mjx_ooxml_types::namespaces::SHARED_MATH)]
#[xml(attribute(local = "space", prefix = "xml", codec = mjx_ooxml_core::Text, accessor = xml_space))]
pub struct Text {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    text: String,
}

impl Text {
    /// Builds `<m:t>{text}</m:t>` (self-closing when `text` is empty).
    #[must_use]
    pub fn new(interner: &mut Interner, text: &str) -> Self {
        Self {
            name: m_name(interner, "t"),
            attributes: Vec::new(),
            empty: text.is_empty(),
            text: text.to_owned(),
        }
    }

    /// This run's literal character data.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces this run's literal character data. Empties `empty` iff `text` is non-empty — a
    /// producer that wants a genuinely self-closing `<m:t/>` for empty text gets it, exactly as
    /// `mjx-dml`'s own text leaves do.
    pub fn set_text(&mut self, text: &str) {
        self.empty = text.is_empty();
        self.text = text.to_owned();
    }
}
