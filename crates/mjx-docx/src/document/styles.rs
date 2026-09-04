//! `word/styles.xml` (`CT_Styles`, the `w:styles` root) — `w:docDefaults`, style definitions,
//! `w:basedOn` chain resolution and `w:latentStyles`.
//!
//! `sample.docx`'s `word/styles.xml` is 2,771 bytes and its `document.xml` references
//! `w:pStyle w:val="PreformattedText"` (MJXOFF-92/MJXOFF-96) — this module is what that reference
//! resolves *to*. `w:docDefaults` is rung one of the effective-properties ladder MJXOFF-106 finishes;
//! this child only has to make every rung it owns readable, and walk `w:basedOn` safely.
//!
//! # `CT_Style/w:pPr` is `CT_PPrGeneral`, not `CT_PPr` — reuse the base, not the paragraph type
//!
//! Verified directly against `wml.xsd`, not assumed from the ticket's own text (which named
//! `CT_PPrGeneral` as "already built" — it is not): `CT_PPr` (a live paragraph's own `w:pPr`,
//! MJXOFF-96) is `CT_PPrBase` **plus** `rPr` (`CT_ParaRPr`), `sectPr` and `pPrChange`.
//! `CT_PPrGeneral` — what `w:style/w:pPr`, `w:pPrDefault/w:pPr` and `w:tblStylePr/w:pPr` all actually
//! are — is `CT_PPrBase` plus `pPrChange` **only**. A style's own paragraph properties may not carry
//! a pilcrow's run properties or a section break; reusing [`super::paragraph_properties::ParagraphProperties`]
//! here would let a caller author both, silently accepting markup no style definition can legally
//! contain. [`StyleParagraphProperties`] is therefore its own container type — but every one of its
//! 33 `CT_PPrBase` leaf types (`Toggle`, `FrameProperties`, `Spacing`, `Indentation`, `ParagraphBorders`,
//! …) is the **exact same struct** `paragraph_properties.rs` already built for a live paragraph's
//! `w:pPr`; only the wiring (a new content enum, a new rank table) is new. `CT_Style/w:rPr`, by
//! contrast, genuinely *is* plain `CT_RPr` — [`super::run_properties::RunProperties`] is reused
//! directly, with no wrapper at all.
//!
//! # `w:tblPr`/`w:trPr`/`w:tcPr` reuse `table_properties.rs`/`tables.rs` directly
//!
//! `CT_Style` and `CT_TblStylePr` both declare `tblPr` (`CT_TblPrBase`), `trPr` (`CT_TrPr`) and `tcPr`
//! (`CT_TcPr`) children — literally the same complex types [`super::table_properties::TableProperties`]/
//! [`super::table_properties::RowProperties`]/[`super::tables::CellProperties`] already model for a
//! live table's own properties (MJXOFF-119, verified against `wml.xsd` directly rather than assumed —
//! see that module's own doc comment). This module reuses all three, both for [`StyleDefinition`]'s
//! own base ("whole table") formatting and for each [`TableStyleOverride`]'s own conditional region —
//! no second, parallel model of table formatting exists here. `w:tblStylePr`'s own conditional-
//! formatting *resolution* (`w:tblLook` matching, banding, the region precedence) lives in
//! `table_regions.rs`; this module only reads and writes the structural elements.
//!
//! # The style index and `w:basedOn` chain walking
//!
//! [`StyleIndex`] is built **once** from a `&StyleSheet` snapshot ([`StyleIndex::build`]) and then
//! reused for as many lookups/chain walks as a caller needs — never rebuilt per property read, which
//! is the anti-pattern the ticket calls out by name ("the ladder in C8 walks this on every property
//! read"). It does not live inside [`StyleSheet`] itself: this crate's [`crate::Document`] reparses
//! every part fresh on each accessor call (see `document/mod.rs`'s own `MainDocument::from_xml`
//! pattern), so there is no long-lived place to cache a derived index across edits either way. A
//! caller who edits the underlying [`StyleSheet`] (adds, removes or renames a style) must call
//! [`StyleIndex::build`] again — the index borrows from the snapshot it was built against and cannot
//! observe a later edit, by construction (its lifetime is tied to that borrow).
//!
//! **Cycle safety: a bounded depth, not a visited-set — and hitting the bound is a typed error,
//! never a silently truncated chain.** [`StyleIndex::based_on_chain`] walks `w:basedOn` from a
//! style upward, accumulating each ancestor into the `Vec` it must return anyway; that
//! accumulation *is* the bound — a chain that has not terminated by [`MAX_BASED_ON_CHAIN_DEPTH`]
//! steps returns `Err(`[`crate::DocxError::BasedOnChainTooDeep`]`)` **instead of** ever returning
//! `Ok` with a partial chain. This distinction is load-bearing, not cosmetic: a caller (MJXOFF-106's
//! effective-properties ladder, eventually) that received a truncated `Ok` chain instead of an
//! `Err` would resolve properties against it and produce a plausible-looking *wrong* answer with
//! nothing red anywhere — silently worse than an outright hang. Proved by mutation, not merely
//! asserted: turning the bound check into a bare `break` (so it returns a 64-element `Ok` chain
//! instead of erroring) is exactly the change
//! `a_self_referencing_based_on_chain_returns_the_typed_error_within_bounded_steps` and
//! `a_mutually_referencing_based_on_chain_returns_the_typed_error_within_bounded_steps` are written
//! to catch — both assert the specific `Err` variant and its `style_id`, not merely that the call
//! returned promptly, and both go red under that mutation (see this child's own PR for the pasted
//! failure).
//!
//! This is also the lower-allocation choice the ticket asks for: a `HashSet<&str>` visited-set
//! would need a *second* data structure on top of the chain the caller already wants back, for
//! chains whose real depth is 2–4 in every fixture measured (`sample.docx`'s own deepest chain,
//! `List → BodyText → Normal`, is depth 2). The one thing a depth cap cannot do that a visited-set
//! can is name *which* style closes the cycle — reporting "chain starting at `X` did not terminate
//! within N steps" without identifying the repeat is judged an acceptable trade for zero extra
//! allocation on every real chain in the corpus, especially since `N` is generous enough (`64`)
//! that no legitimate style hierarchy could ever approach it.
//!
//! **`sample.docx`'s `Normal` style does *not* self-reference** — checked directly against the
//! fixture's own bytes, twice, by two independent methods (a byte-window scan and a real XML
//! parse), both agreeing: `word/styles.xml`'s `<w:style w:styleId="Normal">` carries no `w:basedOn`
//! element at all. (An earlier dispatch brief for this child claimed a self-cycle here — the first
//! check's byte-window scan had overrun a short `Normal` entry into the following `Heading`
//! style's own `w:basedOn`/`w:next` and misattributed both; see this child's own PR/ticket comment
//! for the full correction.) The corpus therefore has no cycle to test against, so
//! `tests/fixtures/style_based_on_cycle.docx` — authored for this child, carrying both a
//! self-referencing style (`"SelfCycle"`) and a mutually-referencing pair (`"MutualA"`/
//! `"MutualB"`) — is the *only* cycle evidence in this suite, and both shapes are asserted to
//! return `Err(DocxError::BasedOnChainTooDeep{ style_id, limit })` specifically, not merely "did
//! not hang". [`based_on_chain`](StyleIndex::based_on_chain) is *also* exercised against
//! `sample.docx`'s own real, non-cyclic chains (`"Normal"` itself, and the deepest one, `List →
//! BodyText → Normal`) to prove the depth cap never false-positives on legitimate input.
//!
//! # Case sensitivity
//!
//! `w:styleId` matching ([`StyleIndex::style_by_id`]) is **case-sensitive** — `styleId` is an XML
//! `ID`-shaped token (`s:ST_String`, but used as a stable machine key throughout `wml.xsd`: every
//! `w:pStyle`/`w:basedOn`/`w:next`/`w:link` reference is a `styleId`, never a display name) and Word
//! itself treats two different-case ids as two different styles. `w:name` matching
//! ([`StyleIndex::style_by_name`]) is **case-insensitive** — Word's own "apply style by name" UI
//! matches this way, and `sample.docx` already shows real producers disagreeing on capitalisation
//! Word's own template uses (`PreformattedText`'s display name is `"Preformatted Text"`; the
//! comparison here is between the *display* names two tools chose, not between id and name). Matching
//! is done via `str::to_lowercase` (a full Unicode case fold — style names are user-facing text, not
//! guaranteed ASCII) rather than `eq_ignore_ascii_case`.
//!
//! # Latent styles and `w:count`
//!
//! [`LatentStyles`] round-trips `w:latentStyles`'s five `def*` defaults and every `w:lsdException`
//! exactly — `sample.docx` carries no `w:latentStyles` element at all (checked directly), so
//! [`tests/fixtures/style_latent_styles.docx`](https://github.com), authored for this child, is the
//! only committed coverage. `w:count` is `minOccurs="0"`/`use="optional"` in `wml.xsd` despite Word
//! always writing it in practice; this module's policy is **preserve, never recompute silently**: a
//! `count` read from a file (or set explicitly) is never rewritten just because
//! [`LatentStyles::push_exception`]/[`LatentStyles::remove_exception`] changed the exception list —
//! the schema does not require the two to agree, and silently "fixing" a value the caller did not
//! touch is exactly the kind of implicit rewrite fidelity-first design avoids. A caller who *wants*
//! `count` kept consistent calls [`LatentStyles::sync_count`] explicitly after editing the list.

use std::borrow::Cow;
use std::collections::HashMap;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, Number, RawAttribute, RawElement, RawName, RawNode, Text as TextCodec,
    ToXml,
};
use mjx_ooxml_types::child_order::{
    DOCUMENT_DEFAULTS, PARAGRAPH_PROPERTIES_GENERAL, STYLES, STYLE_DEFINITION, TABLE_STYLE_OVERRIDE,
};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    DecimalNumber, EightDigitHexadecimalNumber, StyleType, TableStyleOverrideType,
};

use super::body::wml_name;
use super::paragraph_properties::{
    ConditionalFormatting, DecimalNumberValue, FrameProperties, Indentation, NumberingProperties,
    ParagraphAlignment, ParagraphBorders, ParagraphStyle, ParagraphTextFlowDirection, Spacing,
    TabStops, TextBoxTightWrapSetting, VerticalCharacterAlignment,
};
use super::property_macros::{decimal_number_property, toggle_property, value_property};
use super::run_properties::{RunProperties, Shading, Toggle};

// -------------------------------------------------------------------------------------------
// Attribute codecs this module needs beyond what `run_properties.rs`/`paragraph_properties.rs`
// already declared.
// -------------------------------------------------------------------------------------------

/// `s:ST_LongHexNumber` (`w:rsid`'s own `val`, four bytes/eight hex digits) as an attribute value —
/// the wire string itself, preserved exactly, in the same shape as
/// [`super::run_properties::ThemeHexDigit`] for a generated wire-string wrapper.
#[derive(Debug)]
pub struct LongHex;

impl AttributeCodec for LongHex {
    type Value<'a> = EightDigitHexadecimalNumber;
    type Input<'a> = EightDigitHexadecimalNumber;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<EightDigitHexadecimalNumber, InvalidAttributeValue> {
        Ok(EightDigitHexadecimalNumber::from_wire(&raw))
    }

    fn encode<'a>(value: EightDigitHexadecimalNumber) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

// -------------------------------------------------------------------------------------------
// CT_String, reused across w:name/w:aliases/w:basedOn/w:next/w:link — one wire shape (a required
// `val`), five different meanings carried by the accessor name that reads it, exactly as `Toggle`
// (CT_OnOff) is reused across twenty `EG_RPrBase` members: "which element this is is `name`, not the
// Rust type" (see that type's own doc comment in `run_properties.rs`).
// -------------------------------------------------------------------------------------------

/// `CT_String` — a single required `val` string. Reused for `w:name` ("Primary Style Name",
/// §17.7.4.14), `w:aliases` ("Alternate Style Names", §17.7.4.1 — a comma-separated list, kept as one
/// opaque string; this type does not split it), `w:basedOn` ("Parent Style ID", §17.7.4.2), `w:next`
/// ("Style For Next Paragraph", §17.7.4.15) and `w:link` ("Linked Style Reference", §17.7.4.12).
/// `w:pStyle`/`w:rStyle` keep their own [`ParagraphStyle`]/[`super::run_properties::CharacterStyle`]
/// types rather than this one — those names are specific to *which kind* of style a paragraph or run
/// refers to, a distinction worth keeping in the type; nothing here is similarly at risk of being
/// read as the wrong kind of reference.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = value, required))]
pub struct StyleString {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl StyleString {
    /// Builds a new `local` element (`"name"`, `"aliases"`, `"basedOn"`, `"next"` or `"link"`) with
    /// value `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, value: &str) -> Self {
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

impl FromXml for StyleString {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for StyleString {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_LongHexNumber` (`w:rsid`, "Revision Save Id", the style-definition flavour, §17.7.4.16... the
/// same complex type as a paragraph's own `w:rsid*` family) — a required four-byte hex id.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = LongHex, accessor = value, required))]
pub struct RevisionSaveId {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl RevisionSaveId {
    /// Builds a new `w:rsid` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: EightDigitHexadecimalNumber) -> Self {
        let mut item = Self {
            name: wml_name(interner, "rsid"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for RevisionSaveId {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for RevisionSaveId {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_LsdException` (`w:lsdException`, "Latent Style Exception", §17.7.4.13) — one named override
/// against `w:latentStyles`'s own five defaults. Attribute-only (no children of its own in the
/// schema), so this is the same shape as [`ConditionalFormatting`]/[`super::run_properties::Toggle`]:
/// a hand-written `FromXml`/`ToXml` pair plus a derived attribute surface.
///
/// Named `LatentStyleException`, never `Lsd*` — a reader should not need `wml.xsd` open to know what
/// this is.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = name, required))]
#[xml(attribute(local = "locked", prefix = "w", codec = OnOff, accessor = locked))]
#[xml(attribute(local = "uiPriority", prefix = "w", codec = Number<DecimalNumber>, accessor = ui_priority))]
#[xml(attribute(local = "semiHidden", prefix = "w", codec = OnOff, accessor = semi_hidden))]
#[xml(attribute(local = "unhideWhenUsed", prefix = "w", codec = OnOff, accessor = unhide_when_used))]
#[xml(attribute(local = "qFormat", prefix = "w", codec = OnOff, accessor = q_format))]
pub struct LatentStyleException {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LatentStyleException {
    /// Builds a new `w:lsdException` naming `style_name` (the built-in style's display name, e.g.
    /// `"heading 1"`), every optional attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner, style_name: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "lsdException"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_name(interner, style_name);
        item
    }
}

impl FromXml for LatentStyleException {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LatentStyleException {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_PPrGeneral (w:pPr as carried by a style definition, w:pPrDefault or w:tblStylePr) —
// CT_PPrBase's 33 members, reusing every leaf type paragraph_properties.rs already built, then
// pPrChange (kept opaque, MJXOFF-126's scope). See the module's own doc comment for why this is not
// `ParagraphProperties` (`CT_PPr`) reused.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`StyleParagraphProperties`]: `CT_PPrBase`'s 33 members (see
/// `paragraph_properties.rs`'s own doc comment for the full list — identical here), then
/// `pPrChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleParagraphPropertyContent {
    /// `w:pStyle`.
    Style(ParagraphStyle),
    /// `w:keepNext` — `CT_OnOff`.
    KeepWithNext(Toggle),
    /// `w:keepLines` — `CT_OnOff`.
    KeepLinesTogether(Toggle),
    /// `w:pageBreakBefore` — `CT_OnOff`.
    PageBreakBefore(Toggle),
    /// `w:framePr`.
    Frame(FrameProperties),
    /// `w:widowControl` — `CT_OnOff`.
    WidowControl(Toggle),
    /// `w:numPr`.
    Numbering(NumberingProperties),
    /// `w:suppressLineNumbers` — `CT_OnOff`.
    SuppressLineNumbers(Toggle),
    /// `w:pBdr`.
    Borders(ParagraphBorders),
    /// `w:shd`.
    Shading(Shading),
    /// `w:tabs`.
    TabStops(TabStops),
    /// `w:suppressAutoHyphens` — `CT_OnOff`.
    SuppressAutoHyphens(Toggle),
    /// `w:kinsoku` — `CT_OnOff`.
    EastAsianLineBreakingRules(Toggle),
    /// `w:wordWrap` — `CT_OnOff`.
    WordWrap(Toggle),
    /// `w:overflowPunct` — `CT_OnOff`.
    OverflowPunctuation(Toggle),
    /// `w:topLinePunct` — `CT_OnOff`.
    CompressPunctuationAtLineStart(Toggle),
    /// `w:autoSpaceDE` — `CT_OnOff`.
    AutoSpaceLatinAndEastAsian(Toggle),
    /// `w:autoSpaceDN` — `CT_OnOff`.
    AutoSpaceEastAsianAndNumbers(Toggle),
    /// `w:bidi` — `CT_OnOff`.
    RightToLeftLayout(Toggle),
    /// `w:adjustRightInd` — `CT_OnOff`.
    AdjustRightIndentForDocumentGrid(Toggle),
    /// `w:snapToGrid` — `CT_OnOff`.
    SnapToGrid(Toggle),
    /// `w:spacing`.
    Spacing(Spacing),
    /// `w:ind`.
    Indentation(Indentation),
    /// `w:contextualSpacing` — `CT_OnOff`.
    ContextualSpacing(Toggle),
    /// `w:mirrorIndents` — `CT_OnOff`.
    MirrorIndents(Toggle),
    /// `w:suppressOverlap` — `CT_OnOff`.
    SuppressOverlap(Toggle),
    /// `w:jc`.
    Alignment(ParagraphAlignment),
    /// `w:textDirection`.
    TextDirection(ParagraphTextFlowDirection),
    /// `w:textAlignment`.
    VerticalCharacterAlignment(VerticalCharacterAlignment),
    /// `w:textboxTightWrap`.
    TextBoxTightWrap(TextBoxTightWrapSetting),
    /// `w:outlineLvl`.
    OutlineLevel(DecimalNumberValue),
    /// `w:divId`.
    AssociatedHtmlDivId(DecimalNumberValue),
    /// `w:cnfStyle`.
    ConditionalFormatting(ConditionalFormatting),
    /// `w:pPrChange` (`CT_PPrChange`) — the tracked-change wrapper around a previous `w:pPr`. Real
    /// on a live paragraph's own `w:pPr`; schema-illegal (but harmlessly typed rather than dropped
    /// to `Raw` — see `crate::document::body::Hyperlink`'s own doc comment for the identical,
    /// established reason) on a style/doc-default/numbering-level `w:pPr`, none of which this crate
    /// ever authors one on.
    Change(super::revisions::ParagraphPropertiesChange),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_PPrGeneral` — the paragraph-formatting base a style definition (`w:style/w:pPr`), the
/// document-wide paragraph default (`w:pPrDefault/w:pPr`) and a table-style conditional-formatting
/// override (`w:tblStylePr/w:pPr`) all share: `CT_PPrBase`'s 33 members, then `pPrChange`. **Not**
/// [`super::paragraph_properties::ParagraphProperties`] (`CT_PPr`) — see the module's own doc
/// comment for why a style's own paragraph properties may not carry a pilcrow's run properties or a
/// section break, and this type therefore cannot either.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct StyleParagraphProperties {
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
        child(local = "pPrChange", variant = Change, ty = super::revisions::ParagraphPropertiesChange)
    )]
    content: Vec<StyleParagraphPropertyContent>,
}

impl StyleParagraphProperties {
    /// Builds a new, empty `w:pPr` (`CT_PPrGeneral`) — no properties, ready for this type's setters.
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
    /// [`PARAGRAPH_PROPERTIES_GENERAL`] table by the item's own wire name.
    fn rank(item: &StyleParagraphPropertyContent) -> Option<u16> {
        let local = match item {
            StyleParagraphPropertyContent::Style(_) => "pStyle",
            StyleParagraphPropertyContent::KeepWithNext(_) => "keepNext",
            StyleParagraphPropertyContent::KeepLinesTogether(_) => "keepLines",
            StyleParagraphPropertyContent::PageBreakBefore(_) => "pageBreakBefore",
            StyleParagraphPropertyContent::Frame(_) => "framePr",
            StyleParagraphPropertyContent::WidowControl(_) => "widowControl",
            StyleParagraphPropertyContent::Numbering(_) => "numPr",
            StyleParagraphPropertyContent::SuppressLineNumbers(_) => "suppressLineNumbers",
            StyleParagraphPropertyContent::Borders(_) => "pBdr",
            StyleParagraphPropertyContent::Shading(_) => "shd",
            StyleParagraphPropertyContent::TabStops(_) => "tabs",
            StyleParagraphPropertyContent::SuppressAutoHyphens(_) => "suppressAutoHyphens",
            StyleParagraphPropertyContent::EastAsianLineBreakingRules(_) => "kinsoku",
            StyleParagraphPropertyContent::WordWrap(_) => "wordWrap",
            StyleParagraphPropertyContent::OverflowPunctuation(_) => "overflowPunct",
            StyleParagraphPropertyContent::CompressPunctuationAtLineStart(_) => "topLinePunct",
            StyleParagraphPropertyContent::AutoSpaceLatinAndEastAsian(_) => "autoSpaceDE",
            StyleParagraphPropertyContent::AutoSpaceEastAsianAndNumbers(_) => "autoSpaceDN",
            StyleParagraphPropertyContent::RightToLeftLayout(_) => "bidi",
            StyleParagraphPropertyContent::AdjustRightIndentForDocumentGrid(_) => "adjustRightInd",
            StyleParagraphPropertyContent::SnapToGrid(_) => "snapToGrid",
            StyleParagraphPropertyContent::Spacing(_) => "spacing",
            StyleParagraphPropertyContent::Indentation(_) => "ind",
            StyleParagraphPropertyContent::ContextualSpacing(_) => "contextualSpacing",
            StyleParagraphPropertyContent::MirrorIndents(_) => "mirrorIndents",
            StyleParagraphPropertyContent::SuppressOverlap(_) => "suppressOverlap",
            StyleParagraphPropertyContent::Alignment(_) => "jc",
            StyleParagraphPropertyContent::TextDirection(_) => "textDirection",
            StyleParagraphPropertyContent::VerticalCharacterAlignment(_) => "textAlignment",
            StyleParagraphPropertyContent::TextBoxTightWrap(_) => "textboxTightWrap",
            StyleParagraphPropertyContent::OutlineLevel(_) => "outlineLvl",
            StyleParagraphPropertyContent::AssociatedHtmlDivId(_) => "divId",
            StyleParagraphPropertyContent::ConditionalFormatting(_) => "cnfStyle",
            StyleParagraphPropertyContent::Change(_) => "pPrChange",
            StyleParagraphPropertyContent::Raw(_) => return None,
        };
        PARAGRAPH_PROPERTIES_GENERAL.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&StyleParagraphPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: StyleParagraphPropertyContent) {
        let at = PARAGRAPH_PROPERTIES_GENERAL
            .insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&StyleParagraphPropertyContent) -> bool,
        value: Option<StyleParagraphPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    toggle_property!(
        StyleParagraphPropertyContent,
        keep_with_next,
        set_keep_with_next,
        KeepWithNext,
        "keepNext",
        "`w:keepNext` — whether this style's paragraphs stay on the same page as the one after them."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        keep_lines_together,
        set_keep_lines_together,
        KeepLinesTogether,
        "keepLines",
        "`w:keepLines` — whether this style's paragraph lines all stay on one page."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        page_break_before,
        set_page_break_before,
        PageBreakBefore,
        "pageBreakBefore",
        "`w:pageBreakBefore` — whether this style's paragraphs start on a new page."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        widow_control,
        set_widow_control,
        WidowControl,
        "widowControl",
        "`w:widowControl` — whether this style's first/last line may display alone on a page."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        suppress_line_numbers,
        set_suppress_line_numbers,
        SuppressLineNumbers,
        "suppressLineNumbers",
        "`w:suppressLineNumbers` — whether line numbering skips this style's paragraphs."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        suppress_auto_hyphens,
        set_suppress_auto_hyphens,
        SuppressAutoHyphens,
        "suppressAutoHyphens",
        "`w:suppressAutoHyphens` — whether automatic hyphenation is suppressed."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        east_asian_line_breaking_rules,
        set_east_asian_line_breaking_rules,
        EastAsianLineBreakingRules,
        "kinsoku",
        "`w:kinsoku` — whether East Asian line-breaking rules apply."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        word_wrap,
        set_word_wrap,
        WordWrap,
        "wordWrap",
        "`w:wordWrap` — whether a line may break within a word that would otherwise overflow."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        overflow_punctuation,
        set_overflow_punctuation,
        OverflowPunctuation,
        "overflowPunct",
        "`w:overflowPunct` — whether punctuation may extend past the text extents."
    );
    toggle_property!(
        StyleParagraphPropertyContent,
        compress_punctuation_at_line_start,
        set_compress_punctuation_at_line_start,
        CompressPunctuationAtLineStart,
        "topLinePunct",
        "`w:topLinePunct` — whether punctuation compresses at the start of a line."
    );
    toggle_property!(StyleParagraphPropertyContent, auto_space_latin_and_east_asian, set_auto_space_latin_and_east_asian, AutoSpaceLatinAndEastAsian, "autoSpaceDE", "`w:autoSpaceDE` — whether spacing between Latin and East Asian text is adjusted automatically.");
    toggle_property!(StyleParagraphPropertyContent, auto_space_east_asian_and_numbers, set_auto_space_east_asian_and_numbers, AutoSpaceEastAsianAndNumbers, "autoSpaceDN", "`w:autoSpaceDN` — whether spacing between East Asian text and numbers is adjusted automatically.");
    toggle_property!(
        StyleParagraphPropertyContent,
        right_to_left_layout,
        set_right_to_left_layout,
        RightToLeftLayout,
        "bidi",
        "`w:bidi` — whether this style's paragraphs lay out right-to-left."
    );
    toggle_property!(StyleParagraphPropertyContent, adjust_right_indent_for_document_grid, set_adjust_right_indent_for_document_grid, AdjustRightIndentForDocumentGrid, "adjustRightInd", "`w:adjustRightInd` — whether the right indent is adjusted automatically when using the document grid.");
    toggle_property!(
        StyleParagraphPropertyContent,
        snap_to_grid,
        set_snap_to_grid,
        SnapToGrid,
        "snapToGrid",
        "`w:snapToGrid` — whether inter-line spacing follows the document grid."
    );
    toggle_property!(StyleParagraphPropertyContent, contextual_spacing, set_contextual_spacing, ContextualSpacing, "contextualSpacing", "`w:contextualSpacing` — whether spacing above/below is ignored between paragraphs of the same style.");
    toggle_property!(
        StyleParagraphPropertyContent,
        mirror_indents,
        set_mirror_indents,
        MirrorIndents,
        "mirrorIndents",
        "`w:mirrorIndents` — whether the left/right indents are used as inside/outside indents."
    );
    toggle_property!(StyleParagraphPropertyContent, suppress_overlap, set_suppress_overlap, SuppressOverlap, "suppressOverlap", "`w:suppressOverlap` — whether this style's text frame is prevented from overlapping others.");

    value_property!(
        StyleParagraphPropertyContent,
        style,
        set_style,
        Style,
        ParagraphStyle,
        "pStyle",
        "`w:pStyle` — the paragraph style this style's own `w:pPr` refers to (rare, but legal)."
    );
    value_property!(
        StyleParagraphPropertyContent,
        frame,
        set_frame,
        Frame,
        FrameProperties,
        "framePr",
        "`w:framePr` — this style's legacy text-frame properties."
    );
    value_property!(
        StyleParagraphPropertyContent,
        numbering,
        set_numbering,
        Numbering,
        NumberingProperties,
        "numPr",
        "`w:numPr` — this style's numbering-definition reference."
    );
    value_property!(
        StyleParagraphPropertyContent,
        borders,
        set_borders,
        Borders,
        ParagraphBorders,
        "pBdr",
        "`w:pBdr` — this style's borders."
    );
    value_property!(
        StyleParagraphPropertyContent,
        shading,
        set_shading,
        Shading,
        Shading,
        "shd",
        "`w:shd` — this style's shading."
    );
    value_property!(
        StyleParagraphPropertyContent,
        tab_stops,
        set_tab_stops,
        TabStops,
        TabStops,
        "tabs",
        "`w:tabs` — this style's custom tab stops."
    );
    value_property!(
        StyleParagraphPropertyContent,
        spacing,
        set_spacing,
        Spacing,
        Spacing,
        "spacing",
        "`w:spacing` — this style's spacing above/below and between its own lines. **Found missing by \
         MJXOFF-106**: this style's own `w:spacing` was modelled structurally (round-trips through \
         `StyleParagraphPropertyContent::Spacing`) but had no typed accessor at all — a caller could \
         not read or write a style's spacing until this effective-properties ladder needed to walk \
         it. Added with the same `value_property!` macro every sibling accessor here already uses."
    );
    value_property!(
        StyleParagraphPropertyContent,
        indentation,
        set_indentation,
        Indentation,
        Indentation,
        "ind",
        "`w:ind` — this style's indentation. **Found missing by MJXOFF-106**, the same gap as \
         [`StyleParagraphProperties::spacing`] — see that accessor's own doc comment."
    );
    value_property!(
        StyleParagraphPropertyContent,
        alignment,
        set_alignment,
        Alignment,
        ParagraphAlignment,
        "jc",
        "`w:jc` — this style's justification."
    );
    value_property!(
        StyleParagraphPropertyContent,
        text_direction,
        set_text_direction,
        TextDirection,
        ParagraphTextFlowDirection,
        "textDirection",
        "`w:textDirection` — this style's text flow direction."
    );
    value_property!(
        StyleParagraphPropertyContent,
        vertical_character_alignment,
        set_vertical_character_alignment,
        VerticalCharacterAlignment,
        VerticalCharacterAlignment,
        "textAlignment",
        "`w:textAlignment` — how this style's characters align vertically on the line."
    );
    value_property!(
        StyleParagraphPropertyContent,
        text_box_tight_wrap,
        set_text_box_tight_wrap,
        TextBoxTightWrap,
        TextBoxTightWrapSetting,
        "textboxTightWrap",
        "`w:textboxTightWrap` — whether surrounding paragraphs tight-wrap to this style's text box \
         contents."
    );
    value_property!(
        StyleParagraphPropertyContent,
        conditional_formatting,
        set_conditional_formatting,
        ConditionalFormatting,
        ConditionalFormatting,
        "cnfStyle",
        "`w:cnfStyle` — this style's conditional-formatting reference."
    );

    decimal_number_property!(
        StyleParagraphPropertyContent,
        outline_level,
        set_outline_level,
        OutlineLevel,
        "outlineLvl",
        "`w:outlineLvl` — this style's associated outline level."
    );
    decimal_number_property!(
        StyleParagraphPropertyContent,
        associated_html_div_id,
        set_associated_html_div_id,
        AssociatedHtmlDivId,
        "divId",
        "`w:divId` — this style's associated HTML `div` id."
    );

    /// The tracked-change wrapper around a previous `w:pPr` (`w:pPrChange`), or `None` if this
    /// `w:pPr` carries none — real when this is a live paragraph's own properties; schema-illegal
    /// (see [`StyleParagraphPropertyContent::Change`]'s own doc comment) on a style/doc-default/
    /// numbering-level `w:pPr`.
    #[must_use]
    pub fn change(&self) -> Option<&super::revisions::ParagraphPropertiesChange> {
        self.content.iter().find_map(|item| match item {
            StyleParagraphPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// [`StyleParagraphProperties::change`], mutably.
    pub fn change_mut(&mut self) -> Option<&mut super::revisions::ParagraphPropertiesChange> {
        self.content.iter_mut().find_map(|item| match item {
            StyleParagraphPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }
}

// -------------------------------------------------------------------------------------------
// CT_DocDefaults (w:docDefaults) — rung one of the effective-properties ladder: the run and
// paragraph properties every style (and every un-styled run/paragraph) ultimately falls back to.
// -------------------------------------------------------------------------------------------

/// One ordered child of a [`DefaultRunProperties`]: `CT_RPrDefault`'s content is `rPr?` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultRunPropertyContent {
    /// `w:rPr` (`CT_RPr`) — reuses [`RunProperties`] directly; a document default's run properties
    /// are exactly the same shape as a run's own.
    RunProperties(RunProperties),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_RPrDefault` (`w:docDefaults/w:rPrDefault`, "Run Properties Default Values") — the run
/// properties every style and every un-styled run falls back to when nothing more specific sets a
/// property.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DefaultRunProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "rPr", variant = RunProperties, ty = RunProperties))]
    content: Vec<DefaultRunPropertyContent>,
}

impl DefaultRunProperties {
    /// Builds a new, empty `w:rPrDefault`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "rPrDefault"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This default's own `w:rPr`, or `None` if absent (legal — `CT_RPrDefault`'s `rPr` is
    /// `minOccurs="0"`, so `<w:rPrDefault/>` is itself valid, if pointless).
    #[must_use]
    pub fn run_properties(&self) -> Option<&RunProperties> {
        self.content.iter().find_map(|item| match item {
            DefaultRunPropertyContent::RunProperties(properties) => Some(properties),
            DefaultRunPropertyContent::Raw(_) => None,
        })
    }

    /// This default's own `w:rPr`, mutably, or `None` if absent — see
    /// [`DefaultRunProperties::run_properties_or_insert`] to create one.
    pub fn run_properties_mut(&mut self) -> Option<&mut RunProperties> {
        self.content.iter_mut().find_map(|item| match item {
            DefaultRunPropertyContent::RunProperties(properties) => Some(properties),
            DefaultRunPropertyContent::Raw(_) => None,
        })
    }

    /// This default's own `w:rPr`, mutably — creating an empty one if absent.
    pub fn run_properties_or_insert(&mut self, interner: &mut Interner) -> &mut RunProperties {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, DefaultRunPropertyContent::RunProperties(_)))
        {
            self.content.push(DefaultRunPropertyContent::RunProperties(
                RunProperties::new(interner),
            ));
            self.empty = false;
        }
        self.run_properties_mut()
            .expect("just ensured a RunProperties variant is present")
    }
}

/// One ordered child of a [`DefaultParagraphProperties`]: `CT_PPrDefault`'s content is `pPr?` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultParagraphPropertyContent {
    /// `w:pPr` (`CT_PPrGeneral`) — [`StyleParagraphProperties`], not
    /// [`super::paragraph_properties::ParagraphProperties`]; see the module's own doc comment.
    ParagraphProperties(StyleParagraphProperties),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_PPrDefault` (`w:docDefaults/w:pPrDefault`, "Paragraph Properties Default Values") — the
/// paragraph properties every style and every un-styled paragraph falls back to.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DefaultParagraphProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = ParagraphProperties, ty = StyleParagraphProperties)
    )]
    content: Vec<DefaultParagraphPropertyContent>,
}

impl DefaultParagraphProperties {
    /// Builds a new, empty `w:pPrDefault`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "pPrDefault"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This default's own `w:pPr`, or `None` if absent.
    #[must_use]
    pub fn paragraph_properties(&self) -> Option<&StyleParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            DefaultParagraphPropertyContent::ParagraphProperties(properties) => Some(properties),
            DefaultParagraphPropertyContent::Raw(_) => None,
        })
    }

    /// This default's own `w:pPr`, mutably, or `None` if absent — see
    /// [`DefaultParagraphProperties::paragraph_properties_or_insert`] to create one.
    pub fn paragraph_properties_mut(&mut self) -> Option<&mut StyleParagraphProperties> {
        self.content.iter_mut().find_map(|item| match item {
            DefaultParagraphPropertyContent::ParagraphProperties(properties) => Some(properties),
            DefaultParagraphPropertyContent::Raw(_) => None,
        })
    }

    /// This default's own `w:pPr`, mutably — creating an empty one if absent.
    pub fn paragraph_properties_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut StyleParagraphProperties {
        if !self.content.iter().any(|item| {
            matches!(
                item,
                DefaultParagraphPropertyContent::ParagraphProperties(_)
            )
        }) {
            self.content
                .push(DefaultParagraphPropertyContent::ParagraphProperties(
                    StyleParagraphProperties::new(interner),
                ));
            self.empty = false;
        }
        self.paragraph_properties_mut()
            .expect("just ensured a ParagraphProperties variant is present")
    }
}

/// One ordered child of [`DocumentDefaults`]: `CT_DocDefaults`'s content is `rPrDefault?,
/// pPrDefault?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentDefaultsContent {
    /// `w:rPrDefault`.
    RunProperties(DefaultRunProperties),
    /// `w:pPrDefault`.
    ParagraphProperties(DefaultParagraphProperties),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_DocDefaults` (`w:docDefaults`, "Document Default Paragraph and Run Properties",
/// §17.7.4.7) — rung one of the effective-properties ladder: the run and paragraph properties every
/// style, and every un-styled run/paragraph, ultimately falls back to. Readable entirely on its own,
/// independent of any style definition — this is what makes it "rung one".
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DocumentDefaults {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rPrDefault", variant = RunProperties, ty = DefaultRunProperties),
        child(local = "pPrDefault", variant = ParagraphProperties, ty = DefaultParagraphProperties)
    )]
    content: Vec<DocumentDefaultsContent>,
}

impl DocumentDefaults {
    /// Builds a new, empty `w:docDefaults`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "docDefaults"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &DocumentDefaultsContent) -> Option<u16> {
        let local = match item {
            DocumentDefaultsContent::RunProperties(_) => "rPrDefault",
            DocumentDefaultsContent::ParagraphProperties(_) => "pPrDefault",
            DocumentDefaultsContent::Raw(_) => return None,
        };
        DOCUMENT_DEFAULTS.rank_of(None, local)
    }

    fn insert(&mut self, local: &str, item: DocumentDefaultsContent) {
        let at =
            DOCUMENT_DEFAULTS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// This document's own default run properties (`w:rPrDefault`), or `None` if absent.
    #[must_use]
    pub fn run_properties_default(&self) -> Option<&DefaultRunProperties> {
        self.content.iter().find_map(|item| match item {
            DocumentDefaultsContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This document's own default run properties, mutably, or `None` if absent — see
    /// [`DocumentDefaults::run_properties_default_or_insert`] to create one.
    pub fn run_properties_default_mut(&mut self) -> Option<&mut DefaultRunProperties> {
        self.content.iter_mut().find_map(|item| match item {
            DocumentDefaultsContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This document's own default run properties, mutably — creating an empty `w:rPrDefault` at its
    /// schema rank if absent.
    pub fn run_properties_default_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut DefaultRunProperties {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, DocumentDefaultsContent::RunProperties(_)))
        {
            let value = DefaultRunProperties::new(interner);
            self.insert("rPrDefault", DocumentDefaultsContent::RunProperties(value));
        }
        self.run_properties_default_mut()
            .expect("just ensured a RunProperties variant is present")
    }

    /// Removes `w:rPrDefault` entirely, returning it, or `None` if this `w:docDefaults` carried
    /// none.
    pub fn remove_run_properties_default(&mut self) -> Option<DefaultRunProperties> {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, DocumentDefaultsContent::RunProperties(_)))?;
        match self.content.remove(at) {
            DocumentDefaultsContent::RunProperties(value) => Some(value),
            _ => unreachable!("the found index only ever names a RunProperties item"),
        }
    }

    /// This document's own default paragraph properties (`w:pPrDefault`), or `None` if absent.
    #[must_use]
    pub fn paragraph_properties_default(&self) -> Option<&DefaultParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            DocumentDefaultsContent::ParagraphProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This document's own default paragraph properties, mutably, or `None` if absent — see
    /// [`DocumentDefaults::paragraph_properties_default_or_insert`] to create one.
    pub fn paragraph_properties_default_mut(&mut self) -> Option<&mut DefaultParagraphProperties> {
        self.content.iter_mut().find_map(|item| match item {
            DocumentDefaultsContent::ParagraphProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This document's own default paragraph properties, mutably — creating an empty `w:pPrDefault`
    /// at its schema rank if absent.
    pub fn paragraph_properties_default_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut DefaultParagraphProperties {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, DocumentDefaultsContent::ParagraphProperties(_)))
        {
            let value = DefaultParagraphProperties::new(interner);
            self.insert(
                "pPrDefault",
                DocumentDefaultsContent::ParagraphProperties(value),
            );
        }
        self.paragraph_properties_default_mut()
            .expect("just ensured a ParagraphProperties variant is present")
    }

    /// Removes `w:pPrDefault` entirely, returning it, or `None` if this `w:docDefaults` carried
    /// none.
    pub fn remove_paragraph_properties_default(&mut self) -> Option<DefaultParagraphProperties> {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, DocumentDefaultsContent::ParagraphProperties(_)))?;
        match self.content.remove(at) {
            DocumentDefaultsContent::ParagraphProperties(value) => Some(value),
            _ => unreachable!("the found index only ever names a ParagraphProperties item"),
        }
    }
}

// -------------------------------------------------------------------------------------------
// CT_LatentStyles (w:latentStyles) — the style pane's defaults for built-in styles a document has
// not (yet) instantiated as a real w:style, plus named exceptions to those defaults.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`LatentStyles`]: `CT_LatentStyles`' content is `lsdException*` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatentStyleContent {
    /// `w:lsdException` (`CT_LsdException`).
    Exception(LatentStyleException),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_LatentStyles` (`w:latentStyles`, "Latent Style Information", §17.7.4.9) — five style-pane
/// defaults (locked state, UI priority, semi-hidden, unhide-when-used, quick-format) applied to every
/// built-in style this document has not defined for itself, plus a list of named exceptions
/// overriding those defaults for specific built-in style names.
///
/// `sample.docx` carries no `w:latentStyles` at all — see the module's own doc comment for how this
/// type's round-trip is proved instead, and for the `w:count` preservation policy.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "defLockedState", prefix = "w", codec = OnOff, accessor = default_locked_state))]
#[xml(attribute(local = "defUIPriority", prefix = "w", codec = Number<DecimalNumber>, accessor = default_ui_priority))]
#[xml(attribute(local = "defSemiHidden", prefix = "w", codec = OnOff, accessor = default_semi_hidden))]
#[xml(attribute(local = "defUnhideWhenUsed", prefix = "w", codec = OnOff, accessor = default_unhide_when_used))]
#[xml(attribute(local = "defQFormat", prefix = "w", codec = OnOff, accessor = default_q_format))]
#[xml(attribute(local = "count", prefix = "w", codec = Number<DecimalNumber>, accessor = count))]
pub struct LatentStyles {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "lsdException", variant = Exception, ty = LatentStyleException))]
    content: Vec<LatentStyleContent>,
}

impl LatentStyles {
    /// Builds a new, empty `w:latentStyles` — every default and `count` absent, no exceptions.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "latentStyles"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `w:lsdException`, in document order.
    pub fn exceptions(&self) -> impl Iterator<Item = &LatentStyleException> {
        self.content.iter().filter_map(|item| match item {
            LatentStyleContent::Exception(exception) => Some(exception),
            LatentStyleContent::Raw(_) => None,
        })
    }

    /// How many `w:lsdException` children this element carries right now — **not** the same as
    /// [`LatentStyles::count`], which is a separate, independently-settable attribute (see the
    /// module's own doc comment on `w:count`'s preservation policy).
    #[must_use]
    pub fn exception_count(&self) -> usize {
        self.exceptions().count()
    }

    /// Appends `exception` as this list's new last `w:lsdException`. Does **not** touch `w:count` —
    /// call [`LatentStyles::sync_count`] afterwards if the caller wants the two kept consistent.
    pub fn push_exception(&mut self, exception: LatentStyleException) {
        self.content.push(LatentStyleContent::Exception(exception));
        self.empty = false;
    }

    /// Removes and returns the `w:lsdException` at `index` (counting only exceptions, not unmodelled
    /// nodes), or `None` if there is no such exception. Does **not** touch `w:count`.
    pub fn remove_exception(&mut self, index: usize) -> Option<LatentStyleException> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, LatentStyleContent::Exception(_)))
            .nth(index)
            .map(|(at, _)| at)?;
        match self.content.remove(at) {
            LatentStyleContent::Exception(exception) => Some(exception),
            LatentStyleContent::Raw(_) => {
                unreachable!("the filtered index only ever names an Exception item")
            }
        }
    }

    /// Sets `w:count` to this element's own [`LatentStyles::exception_count`] — the explicit,
    /// opt-in way to keep the two consistent after editing the exception list; see the module's own
    /// doc comment for why `push_exception`/`remove_exception` do not do this automatically.
    pub fn sync_count(&mut self, interner: &mut Interner) {
        let count = self.exception_count();
        self.set_count(interner, Some(i64::try_from(count).unwrap_or(i64::MAX)));
    }
}

// -------------------------------------------------------------------------------------------
// CT_TblStylePr (w:style/w:tblStylePr) — one conditional-formatting override inside a table style.
// `tblPr`/`trPr`/`tcPr` are literally `CT_TblPrBase`/`CT_TrPr`/`CT_TcPr` (verified against
// `wml.xsd`), the exact types `table_properties.rs`/`tables.rs` already model for a live table's own
// properties — reused directly here, not restated. Matching these against a table's own cells
// (`w:cnfStyle`, banding) is `table_regions.rs`'s own job.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`TableStyleOverride`]: `CT_TblStylePr`'s sequence is `pPr?, rPr?, tblPr?,
/// trPr?, tcPr?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableStyleOverrideContent {
    /// `w:pPr` (`CT_PPrGeneral`) — [`StyleParagraphProperties`], reused.
    ParagraphProperties(StyleParagraphProperties),
    /// `w:rPr` (`CT_RPr`) — [`RunProperties`], reused.
    RunProperties(RunProperties),
    /// `w:tblPr` (`CT_TblPrBase`) — [`super::table_properties::TableProperties`], reused directly
    /// (MJXOFF-119).
    TableProperties(super::table_properties::TableProperties),
    /// `w:trPr` (`CT_TrPr`) — [`super::table_properties::RowProperties`], reused directly.
    TableRowProperties(super::table_properties::RowProperties),
    /// `w:tcPr` (`CT_TcPr`) — [`super::tables::CellProperties`], reused directly.
    TableCellProperties(super::tables::CellProperties),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_TblStylePr` (`w:tblStylePr`, "Table Style Conditional Formatting Properties", §17.7.6.6) —
/// one region's formatting override inside a table style (`w:style[@type='table']`).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<TableStyleOverrideType>, accessor = region, required))]
pub struct TableStyleOverride {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = ParagraphProperties, ty = StyleParagraphProperties),
        child(local = "rPr", variant = RunProperties, ty = RunProperties),
        child(local = "tblPr", variant = TableProperties, ty = super::table_properties::TableProperties),
        child(local = "trPr", variant = TableRowProperties, ty = super::table_properties::RowProperties),
        child(local = "tcPr", variant = TableCellProperties, ty = super::tables::CellProperties)
    )]
    content: Vec<TableStyleOverrideContent>,
}

impl TableStyleOverride {
    /// Builds a new `w:tblStylePr` for `region`, with no properties of its own yet.
    #[must_use]
    pub fn new(interner: &mut Interner, region: TableStyleOverrideType) -> Self {
        let mut value = Self {
            name: wml_name(interner, "tblStylePr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_region(interner, region);
        value
    }

    fn rank(item: &TableStyleOverrideContent) -> Option<u16> {
        let local = match item {
            TableStyleOverrideContent::ParagraphProperties(_) => "pPr",
            TableStyleOverrideContent::RunProperties(_) => "rPr",
            TableStyleOverrideContent::TableProperties(_) => "tblPr",
            TableStyleOverrideContent::TableRowProperties(_) => "trPr",
            TableStyleOverrideContent::TableCellProperties(_) => "tcPr",
            TableStyleOverrideContent::Raw(_) => return None,
        };
        TABLE_STYLE_OVERRIDE.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&TableStyleOverrideContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: TableStyleOverrideContent) {
        let at =
            TABLE_STYLE_OVERRIDE.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// This override's own `w:pPr`, or `None` if absent.
    #[must_use]
    pub fn paragraph_properties(&self) -> Option<&StyleParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            TableStyleOverrideContent::ParagraphProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:pPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_paragraph_properties(&mut self, value: Option<StyleParagraphProperties>) {
        let is_target = |item: &TableStyleOverrideContent| {
            matches!(item, TableStyleOverrideContent::ParagraphProperties(_))
        };
        self.set(
            "pPr",
            is_target,
            value.map(TableStyleOverrideContent::ParagraphProperties),
        );
    }

    /// This override's own `w:rPr`, or `None` if absent.
    #[must_use]
    pub fn run_properties(&self) -> Option<&RunProperties> {
        self.content.iter().find_map(|item| match item {
            TableStyleOverrideContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:rPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_run_properties(&mut self, value: Option<RunProperties>) {
        let is_target = |item: &TableStyleOverrideContent| {
            matches!(item, TableStyleOverrideContent::RunProperties(_))
        };
        self.set(
            "rPr",
            is_target,
            value.map(TableStyleOverrideContent::RunProperties),
        );
    }

    /// This override's own `w:tblPr`, or `None` if absent.
    #[must_use]
    pub fn table_properties(&self) -> Option<&super::table_properties::TableProperties> {
        self.content.iter().find_map(|item| match item {
            TableStyleOverrideContent::TableProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:tblPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_table_properties(
        &mut self,
        value: Option<super::table_properties::TableProperties>,
    ) {
        let is_target = |item: &TableStyleOverrideContent| {
            matches!(item, TableStyleOverrideContent::TableProperties(_))
        };
        self.set(
            "tblPr",
            is_target,
            value.map(TableStyleOverrideContent::TableProperties),
        );
    }

    /// This override's own `w:trPr`, or `None` if absent.
    #[must_use]
    pub fn row_properties(&self) -> Option<&super::table_properties::RowProperties> {
        self.content.iter().find_map(|item| match item {
            TableStyleOverrideContent::TableRowProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:trPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_row_properties(&mut self, value: Option<super::table_properties::RowProperties>) {
        let is_target = |item: &TableStyleOverrideContent| {
            matches!(item, TableStyleOverrideContent::TableRowProperties(_))
        };
        self.set(
            "trPr",
            is_target,
            value.map(TableStyleOverrideContent::TableRowProperties),
        );
    }

    /// This override's own `w:tcPr`, or `None` if absent.
    #[must_use]
    pub fn cell_properties(&self) -> Option<&super::tables::CellProperties> {
        self.content.iter().find_map(|item| match item {
            TableStyleOverrideContent::TableCellProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:tcPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_cell_properties(&mut self, value: Option<super::tables::CellProperties>) {
        let is_target = |item: &TableStyleOverrideContent| {
            matches!(item, TableStyleOverrideContent::TableCellProperties(_))
        };
        self.set(
            "tcPr",
            is_target,
            value.map(TableStyleOverrideContent::TableCellProperties),
        );
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&TableStyleOverrideContent) -> bool,
        value: Option<TableStyleOverrideContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }
}

// -------------------------------------------------------------------------------------------
// CT_Style (w:style) — one style definition's full surface.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`StyleDefinition`]: `CT_Style`'s sequence, in full (see the module's own
/// doc comment for why `tblPr`/`trPr`/`tcPr` are opaque).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleDefinitionContent {
    /// `w:name` (§17.7.4.14, "Primary Style Name") — this style's own display name.
    Name(StyleString),
    /// `w:aliases` (§17.7.4.1, "Alternate Style Names") — a comma-separated list, kept as one
    /// string.
    Aliases(StyleString),
    /// `w:basedOn` (§17.7.4.2, "Parent Style ID") — the `styleId` this style inherits from.
    BasedOn(StyleString),
    /// `w:next` (§17.7.4.15, "Style For Next Paragraph") — the `styleId` a new paragraph typed
    /// after one in this style switches to.
    Next(StyleString),
    /// `w:link` (§17.7.4.12, "Linked Style Reference") — the paired paragraph/character `styleId`.
    Link(StyleString),
    /// `w:autoRedefine` (§17.7.4.3, "Automatically Merge User Formatting Into Style Definition") —
    /// `CT_OnOff`.
    AutoRedefine(Toggle),
    /// `w:hidden` (§17.7.4.11, "Hide Style From User Interface") — `CT_OnOff`.
    Hidden(Toggle),
    /// `w:uiPriority` (§17.7.4.24, "Optional User Interface Sorting Order").
    UiPriority(DecimalNumberValue),
    /// `w:semiHidden` (§17.7.4.22, "Hide Style From Main User Interface") — `CT_OnOff`.
    SemiHidden(Toggle),
    /// `w:unhideWhenUsed` (§17.7.4.26, "Remove Semi-Hidden Property When Style Is Used") —
    /// `CT_OnOff`.
    UnhideWhenUsed(Toggle),
    /// `w:qFormat` (§17.7.4.20, "Primary Style") — `CT_OnOff`.
    QuickFormat(Toggle),
    /// `w:locked` (§17.7.4.13, "Style Cannot Be Applied") — `CT_OnOff`.
    Locked(Toggle),
    /// `w:personal` (§17.7.4.17) — `CT_OnOff`.
    Personal(Toggle),
    /// `w:personalCompose` (§17.7.4.18) — `CT_OnOff`.
    PersonalCompose(Toggle),
    /// `w:personalReply` (§17.7.4.19) — `CT_OnOff`.
    PersonalReply(Toggle),
    /// `w:rsid` (§17.7.4.21, "Revision Identifier for Style Definition").
    Rsid(RevisionSaveId),
    /// `w:pPr` (`CT_PPrGeneral`) — [`StyleParagraphProperties`], **not**
    /// [`super::paragraph_properties::ParagraphProperties`]; see the module's own doc comment.
    ParagraphProperties(StyleParagraphProperties),
    /// `w:rPr` (`CT_RPr`) — [`RunProperties`], reused directly.
    RunProperties(RunProperties),
    /// `w:tblPr` (`CT_TblPrBase`) — this table style's own base ("whole table") formatting; reused
    /// directly (MJXOFF-119).
    TableProperties(super::table_properties::TableProperties),
    /// `w:trPr` (`CT_TrPr`) — reused directly.
    TableRowProperties(super::table_properties::RowProperties),
    /// `w:tcPr` (`CT_TcPr`) — reused directly.
    TableCellProperties(super::tables::CellProperties),
    /// `w:tblStylePr` (`CT_TblStylePr`, repeatable) — [`TableStyleOverride`].
    TableStyleOverride(TableStyleOverride),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Style` (`w:style`, "Style Definition", §17.7.4.17... the element itself, §17.7.9.13) — one
/// named style: its kind, identity, inheritance references, visibility/behaviour flags, and the
/// paragraph/run/table properties it sets. Named `StyleDefinition`, never `Style` alone (a bare
/// `Style` would collide in spirit with the many `*Style` reference types this crate already has —
/// `ParagraphStyle`, `CharacterStyle` — which name a *reference to* a style, not a style itself).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<StyleType>, accessor = kind))]
#[xml(attribute(local = "styleId", prefix = "w", codec = TextCodec, accessor = style_id))]
#[xml(attribute(local = "default", prefix = "w", codec = OnOff, accessor = is_default))]
#[xml(attribute(local = "customStyle", prefix = "w", codec = OnOff, accessor = is_custom))]
pub struct StyleDefinition {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "name", variant = Name, ty = StyleString),
        child(local = "aliases", variant = Aliases, ty = StyleString),
        child(local = "basedOn", variant = BasedOn, ty = StyleString),
        child(local = "next", variant = Next, ty = StyleString),
        child(local = "link", variant = Link, ty = StyleString),
        child(local = "autoRedefine", variant = AutoRedefine, ty = Toggle),
        child(local = "hidden", variant = Hidden, ty = Toggle),
        child(local = "uiPriority", variant = UiPriority, ty = DecimalNumberValue),
        child(local = "semiHidden", variant = SemiHidden, ty = Toggle),
        child(local = "unhideWhenUsed", variant = UnhideWhenUsed, ty = Toggle),
        child(local = "qFormat", variant = QuickFormat, ty = Toggle),
        child(local = "locked", variant = Locked, ty = Toggle),
        child(local = "personal", variant = Personal, ty = Toggle),
        child(local = "personalCompose", variant = PersonalCompose, ty = Toggle),
        child(local = "personalReply", variant = PersonalReply, ty = Toggle),
        child(local = "rsid", variant = Rsid, ty = RevisionSaveId),
        child(local = "pPr", variant = ParagraphProperties, ty = StyleParagraphProperties),
        child(local = "rPr", variant = RunProperties, ty = RunProperties),
        child(local = "tblPr", variant = TableProperties, ty = super::table_properties::TableProperties),
        child(local = "trPr", variant = TableRowProperties, ty = super::table_properties::RowProperties),
        child(local = "tcPr", variant = TableCellProperties, ty = super::tables::CellProperties),
        child(local = "tblStylePr", variant = TableStyleOverride, ty = TableStyleOverride)
    )]
    content: Vec<StyleDefinitionContent>,
}

impl StyleDefinition {
    /// Builds a new style of `kind`, identified by `style_id`, with no properties of its own yet.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: StyleType, style_id: &str) -> Self {
        let mut value = Self {
            name: wml_name(interner, "style"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_kind(interner, Some(kind));
        value.set_style_id(interner, Some(style_id));
        value
    }

    fn rank(item: &StyleDefinitionContent) -> Option<u16> {
        let local = match item {
            StyleDefinitionContent::Name(_) => "name",
            StyleDefinitionContent::Aliases(_) => "aliases",
            StyleDefinitionContent::BasedOn(_) => "basedOn",
            StyleDefinitionContent::Next(_) => "next",
            StyleDefinitionContent::Link(_) => "link",
            StyleDefinitionContent::AutoRedefine(_) => "autoRedefine",
            StyleDefinitionContent::Hidden(_) => "hidden",
            StyleDefinitionContent::UiPriority(_) => "uiPriority",
            StyleDefinitionContent::SemiHidden(_) => "semiHidden",
            StyleDefinitionContent::UnhideWhenUsed(_) => "unhideWhenUsed",
            StyleDefinitionContent::QuickFormat(_) => "qFormat",
            StyleDefinitionContent::Locked(_) => "locked",
            StyleDefinitionContent::Personal(_) => "personal",
            StyleDefinitionContent::PersonalCompose(_) => "personalCompose",
            StyleDefinitionContent::PersonalReply(_) => "personalReply",
            StyleDefinitionContent::Rsid(_) => "rsid",
            StyleDefinitionContent::ParagraphProperties(_) => "pPr",
            StyleDefinitionContent::RunProperties(_) => "rPr",
            StyleDefinitionContent::TableProperties(_) => "tblPr",
            StyleDefinitionContent::TableRowProperties(_) => "trPr",
            StyleDefinitionContent::TableCellProperties(_) => "tcPr",
            StyleDefinitionContent::TableStyleOverride(_) => "tblStylePr",
            StyleDefinitionContent::Raw(_) => return None,
        };
        STYLE_DEFINITION.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&StyleDefinitionContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: StyleDefinitionContent) {
        let at = STYLE_DEFINITION.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&StyleDefinitionContent) -> bool,
        value: Option<StyleDefinitionContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// Sets a `StyleString`-shaped child (`w:name`/`w:aliases`/`w:basedOn`/`w:next`/`w:link`):
    /// `None` removes it; `Some(text)` builds a fresh [`StyleString`] under `local` and replaces or
    /// inserts it at its schema rank.
    fn set_string_child(
        &mut self,
        interner: &mut Interner,
        local: &'static str,
        is_target: impl Fn(&StyleDefinitionContent) -> bool,
        wrap: impl Fn(StyleString) -> StyleDefinitionContent,
        value: Option<&str>,
    ) {
        match value {
            None => self.remove(is_target),
            Some(text) => {
                let element = StyleString::new(interner, local, text);
                self.set(local, is_target, Some(wrap(element)));
            }
        }
    }

    /// `w:name` — this style's own display name, or `None` if absent.
    #[must_use]
    pub fn name(&self) -> Option<&StyleString> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::Name(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:name`: `None` removes it; `Some(text)` replaces or inserts it.
    pub fn set_name(&mut self, interner: &mut Interner, value: Option<&str>) {
        self.set_string_child(
            interner,
            "name",
            |item| matches!(item, StyleDefinitionContent::Name(_)),
            StyleDefinitionContent::Name,
            value,
        );
    }

    /// `w:aliases` — this style's alternate names, comma-separated in one string, or `None` if
    /// absent.
    #[must_use]
    pub fn aliases(&self) -> Option<&StyleString> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::Aliases(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:aliases`: `None` removes it; `Some(text)` replaces or inserts it.
    pub fn set_aliases(&mut self, interner: &mut Interner, value: Option<&str>) {
        self.set_string_child(
            interner,
            "aliases",
            |item| matches!(item, StyleDefinitionContent::Aliases(_)),
            StyleDefinitionContent::Aliases,
            value,
        );
    }

    /// `w:basedOn` — the `styleId` this style inherits from, or `None` if this style is a root
    /// (inherits from nothing).
    #[must_use]
    pub fn based_on(&self) -> Option<&StyleString> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::BasedOn(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:basedOn`: `None` removes it; `Some(style_id)` replaces or inserts it.
    pub fn set_based_on(&mut self, interner: &mut Interner, value: Option<&str>) {
        self.set_string_child(
            interner,
            "basedOn",
            |item| matches!(item, StyleDefinitionContent::BasedOn(_)),
            StyleDefinitionContent::BasedOn,
            value,
        );
    }

    /// `w:next` — the `styleId` a new paragraph typed after one in this style switches to, or
    /// `None` if this style is used for the next paragraph too.
    #[must_use]
    pub fn next(&self) -> Option<&StyleString> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::Next(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:next`: `None` removes it; `Some(style_id)` replaces or inserts it.
    pub fn set_next(&mut self, interner: &mut Interner, value: Option<&str>) {
        self.set_string_child(
            interner,
            "next",
            |item| matches!(item, StyleDefinitionContent::Next(_)),
            StyleDefinitionContent::Next,
            value,
        );
    }

    /// `w:link` — the paired paragraph/character `styleId`, or `None` if this style is not linked.
    /// Resolving this against the style sheet (both directions, and reporting a missing or
    /// wrong-kind target) is [`StyleIndex::resolve_link`].
    #[must_use]
    pub fn link(&self) -> Option<&StyleString> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::Link(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:link`: `None` removes it; `Some(style_id)` replaces or inserts it.
    pub fn set_link(&mut self, interner: &mut Interner, value: Option<&str>) {
        self.set_string_child(
            interner,
            "link",
            |item| matches!(item, StyleDefinitionContent::Link(_)),
            StyleDefinitionContent::Link,
            value,
        );
    }

    toggle_property!(
        StyleDefinitionContent,
        auto_redefine,
        set_auto_redefine,
        AutoRedefine,
        "autoRedefine",
        "`w:autoRedefine` — whether Word merges a user's direct formatting back into this style \
         definition."
    );
    toggle_property!(
        StyleDefinitionContent,
        hidden,
        set_hidden,
        Hidden,
        "hidden",
        "`w:hidden` — whether this style is hidden from the user interface entirely."
    );
    toggle_property!(
        StyleDefinitionContent,
        semi_hidden,
        set_semi_hidden,
        SemiHidden,
        "semiHidden",
        "`w:semiHidden` — whether this style is hidden from the main (but not the full) style list."
    );
    toggle_property!(
        StyleDefinitionContent,
        unhide_when_used,
        set_unhide_when_used,
        UnhideWhenUsed,
        "unhideWhenUsed",
        "`w:unhideWhenUsed` — whether using this style clears its own `w:semiHidden`."
    );
    toggle_property!(
        StyleDefinitionContent,
        quick_format,
        set_quick_format,
        QuickFormat,
        "qFormat",
        "`w:qFormat` — whether this style is promoted to the primary (quick-format) style gallery."
    );
    toggle_property!(
        StyleDefinitionContent,
        locked,
        set_locked,
        Locked,
        "locked",
        "`w:locked` — whether this style is locked against being applied (e.g. under formatting \
         restrictions)."
    );
    toggle_property!(
        StyleDefinitionContent,
        personal,
        set_personal,
        Personal,
        "personal",
        "`w:personal` — whether this is a personal (mail-merge e-mail) style."
    );
    toggle_property!(
        StyleDefinitionContent,
        personal_compose,
        set_personal_compose,
        PersonalCompose,
        "personalCompose",
        "`w:personalCompose` — whether this personal style applies while composing."
    );
    toggle_property!(
        StyleDefinitionContent,
        personal_reply,
        set_personal_reply,
        PersonalReply,
        "personalReply",
        "`w:personalReply` — whether this personal style applies while replying."
    );

    decimal_number_property!(
        StyleDefinitionContent,
        ui_priority,
        set_ui_priority,
        UiPriority,
        "uiPriority",
        "`w:uiPriority` — this style's sort position in the user interface's style gallery."
    );

    value_property!(
        StyleDefinitionContent,
        rsid,
        set_rsid,
        Rsid,
        RevisionSaveId,
        "rsid",
        "`w:rsid` — the revision-save id this style definition was last edited under."
    );

    /// This style's own `w:pPr` (`CT_PPrGeneral`), or `None` if absent.
    #[must_use]
    pub fn paragraph_properties(&self) -> Option<&StyleParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::ParagraphProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This style's own `w:pPr`, mutably, or `None` if absent — see
    /// [`StyleDefinition::paragraph_properties_or_insert`] to create one.
    pub fn paragraph_properties_mut(&mut self) -> Option<&mut StyleParagraphProperties> {
        self.content.iter_mut().find_map(|item| match item {
            StyleDefinitionContent::ParagraphProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This style's own `w:pPr`, mutably — creating an empty one at its schema rank if absent.
    pub fn paragraph_properties_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut StyleParagraphProperties {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, StyleDefinitionContent::ParagraphProperties(_)))
        {
            let value = StyleParagraphProperties::new(interner);
            self.insert("pPr", StyleDefinitionContent::ParagraphProperties(value));
        }
        self.paragraph_properties_mut()
            .expect("just ensured a ParagraphProperties variant is present")
    }

    /// This style's own `w:rPr` (`CT_RPr`), or `None` if absent.
    #[must_use]
    pub fn run_properties(&self) -> Option<&RunProperties> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This style's own `w:rPr`, mutably, or `None` if absent — see
    /// [`StyleDefinition::run_properties_or_insert`] to create one.
    pub fn run_properties_mut(&mut self) -> Option<&mut RunProperties> {
        self.content.iter_mut().find_map(|item| match item {
            StyleDefinitionContent::RunProperties(value) => Some(value),
            _ => None,
        })
    }

    /// This style's own `w:rPr`, mutably — creating an empty one at its schema rank if absent.
    pub fn run_properties_or_insert(&mut self, interner: &mut Interner) -> &mut RunProperties {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, StyleDefinitionContent::RunProperties(_)))
        {
            let value = RunProperties::new(interner);
            self.insert("rPr", StyleDefinitionContent::RunProperties(value));
        }
        self.run_properties_mut()
            .expect("just ensured a RunProperties variant is present")
    }

    /// This table style's own base ("whole table") `w:tblPr`, or `None` if absent.
    #[must_use]
    pub fn table_properties(&self) -> Option<&super::table_properties::TableProperties> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::TableProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:tblPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_table_properties(
        &mut self,
        value: Option<super::table_properties::TableProperties>,
    ) {
        let is_target = |item: &StyleDefinitionContent| {
            matches!(item, StyleDefinitionContent::TableProperties(_))
        };
        self.set(
            "tblPr",
            is_target,
            value.map(StyleDefinitionContent::TableProperties),
        );
    }

    /// This table style's own base `w:trPr`, or `None` if absent.
    #[must_use]
    pub fn row_properties(&self) -> Option<&super::table_properties::RowProperties> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::TableRowProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:trPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_row_properties(&mut self, value: Option<super::table_properties::RowProperties>) {
        let is_target = |item: &StyleDefinitionContent| {
            matches!(item, StyleDefinitionContent::TableRowProperties(_))
        };
        self.set(
            "trPr",
            is_target,
            value.map(StyleDefinitionContent::TableRowProperties),
        );
    }

    /// This table style's own base `w:tcPr`, or `None` if absent.
    #[must_use]
    pub fn cell_properties(&self) -> Option<&super::tables::CellProperties> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::TableCellProperties(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:tcPr`: `None` removes it; `Some(value)` replaces or inserts it at its schema rank.
    pub fn set_cell_properties(&mut self, value: Option<super::tables::CellProperties>) {
        let is_target = |item: &StyleDefinitionContent| {
            matches!(item, StyleDefinitionContent::TableCellProperties(_))
        };
        self.set(
            "tcPr",
            is_target,
            value.map(StyleDefinitionContent::TableCellProperties),
        );
    }

    /// Every `w:tblStylePr` this table style carries, in document order.
    pub fn table_style_overrides(&self) -> impl Iterator<Item = &TableStyleOverride> {
        self.content.iter().filter_map(|item| match item {
            StyleDefinitionContent::TableStyleOverride(value) => Some(value),
            _ => None,
        })
    }

    /// Appends `override_` as this style's new last `w:tblStylePr`.
    pub fn push_table_style_override(&mut self, override_: TableStyleOverride) {
        self.insert(
            "tblStylePr",
            StyleDefinitionContent::TableStyleOverride(override_),
        );
    }

    /// Removes and returns the `w:tblStylePr` at `index` (counting only overrides, not unmodelled
    /// nodes), or `None` if there is no such override.
    pub fn remove_table_style_override(&mut self, index: usize) -> Option<TableStyleOverride> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, StyleDefinitionContent::TableStyleOverride(_)))
            .nth(index)
            .map(|(at, _)| at)?;
        match self.content.remove(at) {
            StyleDefinitionContent::TableStyleOverride(value) => Some(value),
            _ => unreachable!("the filtered index only ever names a TableStyleOverride item"),
        }
    }
}

// -------------------------------------------------------------------------------------------
// CT_Styles (w:styles) — the style definitions part's own root.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`StyleSheet`]: `CT_Styles`' sequence is `docDefaults?, latentStyles?,
/// style*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleSheetContent {
    /// `w:docDefaults`.
    DocumentDefaults(DocumentDefaults),
    /// `w:latentStyles`.
    LatentStyles(LatentStyles),
    /// `w:style` (repeatable).
    Style(StyleDefinition),
    /// Any other child — an unknown element (Word's own `w14:`/`w15:` extensions land here) —
    /// preserved verbatim.
    Raw(RawNode),
}

/// `CT_Styles` (`w:styles`, the `word/styles.xml` part's own root, §17.7.4.23) — every style
/// definition this document carries, plus the document-wide defaults and latent-style table they
/// build on.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct StyleSheet {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "docDefaults", variant = DocumentDefaults, ty = DocumentDefaults),
        child(local = "latentStyles", variant = LatentStyles, ty = LatentStyles),
        child(local = "style", variant = Style, ty = StyleDefinition)
    )]
    content: Vec<StyleSheetContent>,
}

impl StyleSheet {
    /// Builds a new, empty `w:styles` — no defaults, no latent styles, no style definitions.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "styles"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &StyleSheetContent) -> Option<u16> {
        let local = match item {
            StyleSheetContent::DocumentDefaults(_) => "docDefaults",
            StyleSheetContent::LatentStyles(_) => "latentStyles",
            StyleSheetContent::Style(_) => "style",
            StyleSheetContent::Raw(_) => return None,
        };
        STYLES.rank_of(None, local)
    }

    fn insert(&mut self, local: &str, item: StyleSheetContent) {
        let at = STYLES.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// This style sheet's own `w:docDefaults`, or `None` if absent.
    #[must_use]
    pub fn document_defaults(&self) -> Option<&DocumentDefaults> {
        self.content.iter().find_map(|item| match item {
            StyleSheetContent::DocumentDefaults(value) => Some(value),
            _ => None,
        })
    }

    /// This style sheet's own `w:docDefaults`, mutably, or `None` if absent — see
    /// [`StyleSheet::document_defaults_or_insert`] to create one.
    pub fn document_defaults_mut(&mut self) -> Option<&mut DocumentDefaults> {
        self.content.iter_mut().find_map(|item| match item {
            StyleSheetContent::DocumentDefaults(value) => Some(value),
            _ => None,
        })
    }

    /// This style sheet's own `w:docDefaults`, mutably — creating an empty one at its schema rank
    /// if absent.
    pub fn document_defaults_or_insert(
        &mut self,
        interner: &mut Interner,
    ) -> &mut DocumentDefaults {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, StyleSheetContent::DocumentDefaults(_)))
        {
            let value = DocumentDefaults::new(interner);
            self.insert("docDefaults", StyleSheetContent::DocumentDefaults(value));
        }
        self.document_defaults_mut()
            .expect("just ensured a DocumentDefaults variant is present")
    }

    /// This style sheet's own `w:latentStyles`, or `None` if absent — `sample.docx` has none (see
    /// the module's own doc comment).
    #[must_use]
    pub fn latent_styles(&self) -> Option<&LatentStyles> {
        self.content.iter().find_map(|item| match item {
            StyleSheetContent::LatentStyles(value) => Some(value),
            _ => None,
        })
    }

    /// This style sheet's own `w:latentStyles`, mutably, or `None` if absent — see
    /// [`StyleSheet::latent_styles_or_insert`] to create one.
    pub fn latent_styles_mut(&mut self) -> Option<&mut LatentStyles> {
        self.content.iter_mut().find_map(|item| match item {
            StyleSheetContent::LatentStyles(value) => Some(value),
            _ => None,
        })
    }

    /// This style sheet's own `w:latentStyles`, mutably — creating an empty one at its schema rank
    /// if absent.
    pub fn latent_styles_or_insert(&mut self, interner: &mut Interner) -> &mut LatentStyles {
        if !self
            .content
            .iter()
            .any(|item| matches!(item, StyleSheetContent::LatentStyles(_)))
        {
            let value = LatentStyles::new(interner);
            self.insert("latentStyles", StyleSheetContent::LatentStyles(value));
        }
        self.latent_styles_mut()
            .expect("just ensured a LatentStyles variant is present")
    }

    /// Removes `w:docDefaults` entirely, returning it, or `None` if this style sheet carried none.
    pub fn remove_document_defaults(&mut self) -> Option<DocumentDefaults> {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, StyleSheetContent::DocumentDefaults(_)))?;
        match self.content.remove(at) {
            StyleSheetContent::DocumentDefaults(value) => Some(value),
            _ => unreachable!("the found index only ever names a DocumentDefaults item"),
        }
    }

    /// Removes `w:latentStyles` entirely, returning it, or `None` if this style sheet carried none.
    pub fn remove_latent_styles(&mut self) -> Option<LatentStyles> {
        let at = self
            .content
            .iter()
            .position(|item| matches!(item, StyleSheetContent::LatentStyles(_)))?;
        match self.content.remove(at) {
            StyleSheetContent::LatentStyles(value) => Some(value),
            _ => unreachable!("the found index only ever names a LatentStyles item"),
        }
    }

    /// Every `w:style` this style sheet defines, in document order.
    pub fn styles(&self) -> impl Iterator<Item = &StyleDefinition> {
        self.content.iter().filter_map(|item| match item {
            StyleSheetContent::Style(style) => Some(style),
            _ => None,
        })
    }

    /// How many `w:style` definitions this style sheet carries.
    #[must_use]
    pub fn style_count(&self) -> usize {
        self.styles().count()
    }

    /// The first style whose `w:styleId` is exactly `style_id` (case-sensitive — see the module's
    /// own doc comment), or `None` if none matches.
    #[must_use]
    pub fn style_by_id(&self, style_id: &str, interner: &Interner) -> Option<&StyleDefinition> {
        self.styles()
            .find(|style| style.style_id(interner).ok().flatten().as_deref() == Some(style_id))
    }

    /// The same style, mutably.
    pub fn style_by_id_mut(
        &mut self,
        style_id: &str,
        interner: &Interner,
    ) -> Option<&mut StyleDefinition> {
        self.content.iter_mut().find_map(|item| match item {
            StyleSheetContent::Style(style)
                if style.style_id(interner).ok().flatten().as_deref() == Some(style_id) =>
            {
                Some(style)
            }
            _ => None,
        })
    }

    /// Appends `style` as this style sheet's new last `w:style`.
    pub fn add_style(&mut self, style: StyleDefinition) {
        self.insert("style", StyleSheetContent::Style(style));
    }

    /// Removes the first style whose `w:styleId` is exactly `style_id`, returning it, or `None` if
    /// no such style exists.
    pub fn remove_style(&mut self, style_id: &str, interner: &Interner) -> Option<StyleDefinition> {
        let at = self.content.iter().position(|item| match item {
            StyleSheetContent::Style(style) => {
                style.style_id(interner).ok().flatten().as_deref() == Some(style_id)
            }
            _ => false,
        })?;
        match self.content.remove(at) {
            StyleSheetContent::Style(style) => Some(style),
            _ => unreachable!("the found index only ever names a Style item"),
        }
    }
}

// -------------------------------------------------------------------------------------------
// StyleIndex — built once from a StyleSheet snapshot, then reused for as many lookups/chain walks
// as a caller needs. See the module's own doc comment for the full design rationale (why this is
// not cached inside StyleSheet, and why cycle safety is a bounded depth rather than a visited-set).
// -------------------------------------------------------------------------------------------

/// The greatest number of `w:basedOn` hops [`StyleIndex::based_on_chain`] walks before treating the
/// chain as broken — almost certainly a cycle — rather than terminating normally. Generous relative
/// to any real style hierarchy (`sample.docx`'s own deepest chain is 2 hops; Word's built-in style
/// gallery nests at most a handful of levels), so a legitimate document never approaches it. See the
/// module's own doc comment for why this bound, not a visited-set, is how a cycle is detected.
pub const MAX_BASED_ON_CHAIN_DEPTH: usize = 64;

/// The result of resolving a style's `w:link` against a [`StyleIndex`] — a defect to report, never
/// to panic on (see the module's own doc comment and this child's own ticket).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedStyleResolution<'a> {
    /// `w:link` resolves to a style of the complementary paragraph/character kind, as ECMA-376
    /// expects.
    Resolved(&'a StyleDefinition),
    /// This style carries no `w:link`.
    NoLink,
    /// `w:link` names a `styleId` this style sheet does not define — a defect, not fatal.
    TargetMissing,
    /// `w:link` resolves, but the target is not the complementary paragraph/character kind — a
    /// defect, not fatal. `found` is the target's own `w:type`, if it has one.
    KindMismatch {
        /// The linked style's own kind, if it states one.
        found: Option<StyleType>,
    },
}

/// A style sheet's styles, indexed by `w:styleId` (case-sensitive) and by `w:name` (case-insensitive,
/// full Unicode case fold), built once from a `&`[`StyleSheet`] snapshot and reused for every lookup
/// and `w:basedOn` walk a caller needs against that snapshot. See the module's own doc comment for
/// the complete design rationale.
#[derive(Debug)]
pub struct StyleIndex<'a> {
    /// Every style, in document order — index `i` here is what `by_id`/`by_name`/`defaults` point
    /// into.
    entries: Vec<&'a StyleDefinition>,
    /// `styleId` (exact, case-sensitive) → index into `entries`. Owned, not borrowed: a `styleId`
    /// read through the typed attribute accessor is a `Cow<str>` tied to the `&Interner` borrow
    /// passed to [`StyleIndex::build`], not to `'a` (the [`StyleSheet`] snapshot's own lifetime) —
    /// the two are independent borrows, so the key is copied once here rather than fought over.
    by_id: HashMap<String, usize>,
    /// `w:name`, lowercased → index into `entries`. The *first* style with a given lowercased name
    /// wins if more than one collides (a malformed but not impossible file); later duplicates are
    /// simply unreachable by name (still reachable by id).
    by_name: HashMap<String, usize>,
    /// `ST_StyleType` → index into `entries`, for the first style found with `w:default="true"` of
    /// that kind. A file naming two defaults of the same kind is likewise not rejected — the first
    /// one wins, later ones are still reachable by id/name.
    defaults: HashMap<StyleType, usize>,
}

impl<'a> StyleIndex<'a> {
    /// Builds an index over every style in `style_sheet`, resolving each one's `styleId`, `w:name`
    /// and default-ness once.
    ///
    /// # Errors
    /// Returns [`crate::DocxError`] if any style's `styleId`, `w:name`, `w:type` or `w:default`
    /// attribute is present but malformed.
    pub fn build(
        style_sheet: &'a StyleSheet,
        interner: &Interner,
    ) -> Result<Self, crate::DocxError> {
        let entries: Vec<&StyleDefinition> = style_sheet.styles().collect();
        let mut by_id = HashMap::with_capacity(entries.len());
        let mut by_name = HashMap::with_capacity(entries.len());
        let mut defaults = HashMap::new();

        for (index, style) in entries.iter().enumerate() {
            if let Some(style_id) = style.style_id(interner).map_err(FromXmlError::from)? {
                by_id.entry(style_id.into_owned()).or_insert(index);
            }
            if let Some(display_name) = style
                .name()
                .map(|n| n.value(interner))
                .transpose()
                .map_err(FromXmlError::from)?
            {
                by_name.entry(display_name.to_lowercase()).or_insert(index);
            }
            let is_default = style.is_default(interner).map_err(FromXmlError::from)?;
            let kind = style.kind(interner).map_err(FromXmlError::from)?;
            if is_default == Some(true) {
                if let Some(kind) = kind {
                    defaults.entry(kind).or_insert(index);
                }
            }
        }

        Ok(Self {
            entries,
            by_id,
            by_name,
            defaults,
        })
    }

    /// The style whose `w:styleId` is exactly `style_id` (case-sensitive), or `None`.
    #[must_use]
    pub fn style_by_id(&self, style_id: &str) -> Option<&'a StyleDefinition> {
        self.by_id.get(style_id).map(|&index| self.entries[index])
    }

    /// The style whose `w:name` matches `name` case-insensitively (full Unicode case fold), or
    /// `None`.
    #[must_use]
    pub fn style_by_name(&self, name: &str) -> Option<&'a StyleDefinition> {
        self.by_name
            .get(&name.to_lowercase())
            .map(|&index| self.entries[index])
    }

    /// The style marked `w:default="true"` for `kind`, or `None` if this style sheet names none.
    #[must_use]
    pub fn default_style(&self, kind: StyleType) -> Option<&'a StyleDefinition> {
        self.defaults.get(&kind).map(|&index| self.entries[index])
    }

    /// Walks `w:basedOn` from the style named `style_id` upward: index 0 is that style itself, each
    /// following entry its next ancestor, ending at a root (a style with no `w:basedOn`, or whose
    /// `w:basedOn` names a `styleId` this style sheet does not define — see the module's own doc
    /// comment for why an unresolvable *ancestor* is a graceful stop rather than an error).
    ///
    /// The returned `Vec` **is** the depth bound: this walk allocates nothing beyond the chain it
    /// must hand back anyway, and returns [`crate::DocxError::BasedOnChainTooDeep`] the moment that
    /// chain would exceed [`MAX_BASED_ON_CHAIN_DEPTH`] entries — see the module's own doc comment
    /// for why this, not a visited-set, is this crate's cycle-safety mechanism.
    ///
    /// # Errors
    /// Returns [`crate::DocxError::UnknownStyleId`] if `style_id` itself (the chain's own starting
    /// point, not an ancestor) is not in this style sheet, or
    /// [`crate::DocxError::BasedOnChainTooDeep`] if the walk does not terminate within
    /// [`MAX_BASED_ON_CHAIN_DEPTH`] steps.
    pub fn based_on_chain(
        &self,
        style_id: &str,
        interner: &Interner,
    ) -> Result<Vec<&'a StyleDefinition>, crate::DocxError> {
        let mut chain: Vec<&StyleDefinition> = Vec::new();
        let mut current: Cow<'_, str> = Cow::Borrowed(style_id);
        loop {
            if chain.len() >= MAX_BASED_ON_CHAIN_DEPTH {
                return Err(crate::DocxError::BasedOnChainTooDeep {
                    style_id: style_id.to_owned(),
                    limit: MAX_BASED_ON_CHAIN_DEPTH,
                });
            }
            let Some(style) = self.style_by_id(current.as_ref()) else {
                if chain.is_empty() {
                    return Err(crate::DocxError::UnknownStyleId(style_id.to_owned()));
                }
                break;
            };
            chain.push(style);
            let parent = style
                .based_on()
                .map(|reference| reference.value(interner))
                .transpose()
                .map_err(FromXmlError::from)?;
            match parent {
                Some(parent_id) if !parent_id.is_empty() => {
                    current = Cow::Owned(parent_id.into_owned());
                }
                _ => break,
            }
        }
        Ok(chain)
    }

    /// Resolves `style_id`'s own `w:link`, in either direction (a paragraph style linking to its
    /// character style, or vice versa) — see [`LinkedStyleResolution`] for the reportable outcomes.
    ///
    /// # Errors
    /// Returns [`crate::DocxError::UnknownStyleId`] if `style_id` itself is not in this style
    /// sheet, or [`crate::DocxError`] if an attribute involved is present but malformed.
    pub fn resolve_link(
        &self,
        style_id: &str,
        interner: &Interner,
    ) -> Result<LinkedStyleResolution<'a>, crate::DocxError> {
        let style = self
            .style_by_id(style_id)
            .ok_or_else(|| crate::DocxError::UnknownStyleId(style_id.to_owned()))?;
        let Some(link) = style.link() else {
            return Ok(LinkedStyleResolution::NoLink);
        };
        let target_id = link.value(interner).map_err(FromXmlError::from)?;
        let Some(target) = self.style_by_id(&target_id) else {
            return Ok(LinkedStyleResolution::TargetMissing);
        };
        let this_kind = style.kind(interner).map_err(FromXmlError::from)?;
        let target_kind = target.kind(interner).map_err(FromXmlError::from)?;
        let complementary = matches!(
            (this_kind, target_kind),
            (Some(StyleType::Paragraph), Some(StyleType::Character))
                | (Some(StyleType::Character), Some(StyleType::Paragraph))
        );
        if complementary {
            Ok(LinkedStyleResolution::Resolved(target))
        } else {
            Ok(LinkedStyleResolution::KindMismatch { found: target_kind })
        }
    }
}
