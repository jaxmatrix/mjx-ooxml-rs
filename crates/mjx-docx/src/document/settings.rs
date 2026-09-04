//! `word/settings.xml` (`CT_Settings`, the `w:settings` root, §17.15.1.94) — MJXOFF-136's own file:
//! [`DocumentSettings`] and the ~50-type cluster ECMA-376 Part 1's §17.15 groups under it.
//!
//! # Why this file exists
//!
//! MJXOFF-69 through MJXOFF-74 allotted Word six children and, between them, named no owner for
//! `word/settings.xml`, `word/webSettings.xml`, `word/fontTable.xml` or `word/recipients.xml` — a
//! third of `wml.xsd`'s fourteen global elements. `CT_Settings` alone declares **98 child elements**
//! (verified directly against `wml.xsd`, not assumed), most of them independently optional flags a
//! real document uses a handful of at a time. [`DocumentSettings`]/[`SettingsContent`] model every
//! one of the 98 — see the giant match in [`DocumentSettings::local`] for the full inventory — so
//! that an element this crate does not otherwise care about (a `w:doNotHyphenateCaps` nobody asked
//! for) still round-trips exactly, and so that an element in a namespace this crate has never heard
//! of (`w14:`, `w15:`, a future extension) falls into [`SettingsContent::Raw`] **in its original
//! position relative to its known neighbours** — never dropped, never reordered. `webSettings.xml`
//! (`super::web_settings`), `fontTable.xml` (`super::font_table`) and mail merge/`recipients.xml`
//! (`super::mail_merge`) are each their own sibling file; this one is `CT_Settings` alone.
//!
//! # `MJXOFF-113`'s ad-hoc read, replaced
//!
//! `crate::document::mod::Document::even_and_odd_headers` used to scan `word/settings.xml`'s raw
//! tree by hand for a bare `w:evenAndOddHeaders`, because MJXOFF-113 needed the one flag and this
//! child had not landed yet. [`DocumentSettings::even_and_odd_headers`] replaces it; the ad-hoc scan
//! and its `is_wml_element` helper call are gone from `mod.rs` (the helper itself survives — it is
//! also `check_header_footer_root`'s).
//!
//! # Tiers: what gets a full authoring surface, and why
//!
//! Every one of the 98 `CT_Settings` children is **modelled** — none is silently dropped into an
//! opaque bucket, and every one round-trips byte-identically through [`SettingsContent`]. They split
//! into three tiers of *how much accessor surface* they get, each chosen for a defensible reason
//! rather than left unstated (the "no 'later phase' notes" rule this child's own ticket names):
//!
//! 1. **Full get/set, individually named** — every `CT_OnOff` flag (via
//!    [`super::property_macros::toggle_property`]), every `CT_DecimalNumber` and `CT_TwipsMeasure`
//!    leaf, and the handful of "own type" settings another Phase C child explicitly needs by name
//!    (`w:evenAndOddHeaders`, `w:mirrorMargins`, `w:defaultTabStop`, `w:trackRevisions` — note the
//!    *wire* local name, not the ticket's own "trackChanges" shorthand; `wml.xsd:2923` spells it
//!    `trackRevisions` — `w:documentProtection`, `w:footnotePr`/`w:endnotePr`). This is every scalar
//!    leaf: there is no reason a `bool`/`i64`/twips accessor should cost more to add than to leave
//!    out, so all of them get one.
//! 2. **Full get/set, borrowing the nested type** — every "own type" child (`w:view`, `w:zoom`,
//!    `w:documentProtection`, `w:compat`, `w:docVars`, `w:rsids`, `w:mailMerge`, `w:captions`, …)
//!    returns `Option<&T>`/takes `Option<T>` for its own richly-typed value, whose *own* fields are
//!    then reached through *that* type's own accessors — mirrors
//!    [`super::annotations::FootnoteProperties`] handing back its own typed children rather than
//!    `RunProperties` re-exposing every one of `Border`'s nine attributes itself.
//! 3. **Read-only, typed** — `sl:schemaLibrary` alone. It is a foreign-schema reference (the Smart
//!    Tag schema library namespace, `sl:`, which no schema this workspace validates against
//!    declares) that `CT_Settings`'s own `xsd:sequence` names as a child but this crate has no
//!    reason to parse further: it is preserved exactly, in position, through
//!    [`SettingsContent::Raw`] — the same unknown-element bucket a `w14:`/`w15:` extension falls
//!    into, just a schema-known one. Nothing here authors a new one.
//!
//! No cluster is read-only in the stronger sense (typed-but-no-setter) except the sixty-two
//! individual `w:compat` flags below `w:compatSetting` in schema order — see
//! [`Compatibility`]'s own doc comment for why those sixty-two get a single **name-keyed** accessor
//! pair instead of sixty-two bespoke methods.
//!
//! # The password hash is opaque bytes, never recomputed
//!
//! [`DocumentProtection`]/[`super::web_settings::WriteProtectionSetting`]'s (writeProtection lives on
//! this same [`SettingsContent`], see [`DocumentSettings::write_protection`])
//! `hashValue`/`saltValue`/legacy `hash`/`salt` attributes are `xsd:base64Binary`, typed here as plain
//! text (`Text`/`Cow<str>`) — the base64 *is* the wire form, so there is nothing to decode. Nothing in
//! this module ever computes, verifies or clears one: a caller who sets `edit`/`formatting`/
//! `enforcement` and leaves the hash alone gets the file's own previous hash back unchanged; a caller
//! who wants a different password writes the exact bytes Word's own algorithm would have produced,
//! which is out of scope for a structural editor (mirrors `crate::document::fields`'s own
//! `ValueTooLong` precedent: refuse what would author invalid markup, never silently "fix" what a
//! setter does not itself touch).

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Enumeration, FromXml, FromXmlError, Interner,
    InvalidAttributeValue, Number, RawAttribute, RawElement, RawName, RawNode, Text as TextCodec,
    ToXml,
};
use mjx_ooxml_types::child_order::{COMPAT, SETTINGS};
use mjx_ooxml_types::shared::TwipsMeasure;
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    CaptionPosition, CharacterSpacingCompression, ChapterSeparator, ColorSchemeSlot,
    DecimalNumberOrPercentage, DocumentClassification, DocumentProtection as EditRestriction,
    DocumentView, NumberFormat, PixelsMeasure, ProofingState, StyleSortMethod, ZoomPreset,
};

use super::body::{wml_name, RelationshipReference, Unmodeled};
use super::paragraph_properties::DecimalNumberValue;
use super::property_macros::{toggle_property, value_property};
use super::run_properties::{Lang, Languages, Toggle, Twips};
use super::styles::{LongHex, RevisionSaveId, StyleString};

// =================================================================================================
// Attribute codecs this module needs beyond what `run_properties.rs`/`styles.rs` already declared —
// every one a generated wire-string wrapper (`from_wire`/`to_wire`, no `FromStr`), the same shape as
// `run_properties::HexColor`.
// =================================================================================================

/// `ST_DecimalNumberOrPercent` (`w:zoom/@percent`, `w:summaryLength/@val`,
/// `w:readModeInkLockDown/@fontSz`) as an attribute value — the wire string itself (a decimal number
/// or a `%`-suffixed percentage), preserved exactly.
#[derive(Debug)]
pub struct DecimalOrPercent;

impl AttributeCodec for DecimalOrPercent {
    type Value<'a> = DecimalNumberOrPercentage;
    type Input<'a> = DecimalNumberOrPercentage;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<DecimalNumberOrPercentage, InvalidAttributeValue> {
        Ok(DecimalNumberOrPercentage::from_wire(&raw))
    }

    fn encode<'a>(value: DecimalNumberOrPercentage) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

/// `ST_DocType` (`w:documentType/@val`) as an attribute value — an unrestricted wire string.
#[derive(Debug)]
pub struct DocTypeCodec;

impl AttributeCodec for DocTypeCodec {
    type Value<'a> = DocumentClassification;
    type Input<'a> = DocumentClassification;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<DocumentClassification, InvalidAttributeValue> {
        Ok(DocumentClassification::from_wire(&raw))
    }

    fn encode<'a>(value: DocumentClassification) -> Cow<'a, str> {
        Cow::Owned(value.to_wire().to_owned())
    }
}

// =================================================================================================
// DocumentSettings (CT_Settings) and its content — see the module's own doc comment for the tier
// each child was given.
// =================================================================================================

/// `word/settings.xml`'s own root (`w:settings`, `CT_Settings`) — see the module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DocumentSettings {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "writeProtection", variant = WriteProtection, ty = WriteProtectionSetting),
        child(local = "view", variant = View, ty = ViewSetting),
        child(local = "zoom", variant = Zoom, ty = ZoomSetting),
        child(local = "removePersonalInformation", variant = RemovePersonalInformation, ty = Toggle),
        child(local = "removeDateAndTime", variant = RemoveDateAndTime, ty = Toggle),
        child(local = "doNotDisplayPageBoundaries", variant = DoNotDisplayPageBoundaries, ty = Toggle),
        child(local = "displayBackgroundShape", variant = DisplayBackgroundShape, ty = Toggle),
        child(local = "printPostScriptOverText", variant = PrintPostScriptOverText, ty = Toggle),
        child(local = "printFractionalCharacterWidth", variant = PrintFractionalCharacterWidth, ty = Toggle),
        child(local = "printFormsData", variant = PrintFormsData, ty = Toggle),
        child(local = "embedTrueTypeFonts", variant = EmbedTrueTypeFonts, ty = Toggle),
        child(local = "embedSystemFonts", variant = EmbedSystemFonts, ty = Toggle),
        child(local = "saveSubsetFonts", variant = SaveSubsetFonts, ty = Toggle),
        child(local = "saveFormsData", variant = SaveFormsData, ty = Toggle),
        child(local = "mirrorMargins", variant = MirrorMargins, ty = Toggle),
        child(local = "alignBordersAndEdges", variant = AlignBordersAndEdges, ty = Toggle),
        child(local = "bordersDoNotSurroundHeader", variant = BordersDoNotSurroundHeader, ty = Toggle),
        child(local = "bordersDoNotSurroundFooter", variant = BordersDoNotSurroundFooter, ty = Toggle),
        child(local = "gutterAtTop", variant = GutterAtTop, ty = Toggle),
        child(local = "hideSpellingErrors", variant = HideSpellingErrors, ty = Toggle),
        child(local = "hideGrammaticalErrors", variant = HideGrammaticalErrors, ty = Toggle),
        child(local = "activeWritingStyle", variant = ActiveWritingStyle, ty = WritingStyleSetting),
        child(local = "proofState", variant = ProofState, ty = ProofSettings),
        child(local = "formsDesign", variant = FormsDesign, ty = Toggle),
        child(local = "attachedTemplate", variant = AttachedTemplate, ty = RelationshipReference),
        child(local = "linkStyles", variant = LinkStyles, ty = Toggle),
        child(local = "stylePaneFormatFilter", variant = StylePaneFormatFilter, ty = StylePaneFilter),
        child(local = "stylePaneSortMethod", variant = StylePaneSortMethod, ty = StyleSortSetting),
        child(local = "documentType", variant = DocumentType, ty = DocumentTypeSetting),
        child(local = "mailMerge", variant = MailMerge, ty = super::mail_merge::MailMergeSettings),
        child(local = "revisionView", variant = RevisionView, ty = TrackChangesView),
        child(local = "trackRevisions", variant = TrackRevisions, ty = Toggle),
        child(local = "doNotTrackMoves", variant = DoNotTrackMoves, ty = Toggle),
        child(local = "doNotTrackFormatting", variant = DoNotTrackFormatting, ty = Toggle),
        child(local = "documentProtection", variant = DocumentProtectionChild, ty = DocumentProtection),
        child(local = "autoFormatOverride", variant = AutoFormatOverride, ty = Toggle),
        child(local = "styleLockTheme", variant = StyleLockTheme, ty = Toggle),
        child(local = "styleLockQFSet", variant = StyleLockQuickFormatSet, ty = Toggle),
        child(local = "defaultTabStop", variant = DefaultTabStop, ty = TwipsMeasureValue),
        child(local = "autoHyphenation", variant = AutoHyphenation, ty = Toggle),
        child(local = "consecutiveHyphenLimit", variant = ConsecutiveHyphenLimit, ty = DecimalNumberValue),
        child(local = "hyphenationZone", variant = HyphenationZone, ty = TwipsMeasureValue),
        child(local = "doNotHyphenateCaps", variant = DoNotHyphenateCaps, ty = Toggle),
        child(local = "showEnvelope", variant = ShowEnvelope, ty = Toggle),
        child(local = "summaryLength", variant = SummaryLength, ty = DecimalOrPercentValue),
        child(local = "clickAndTypeStyle", variant = ClickAndTypeStyle, ty = StyleString),
        child(local = "defaultTableStyle", variant = DefaultTableStyle, ty = StyleString),
        child(local = "evenAndOddHeaders", variant = EvenAndOddHeaders, ty = Toggle),
        child(local = "bookFoldRevPrinting", variant = BookFoldRevPrinting, ty = Toggle),
        child(local = "bookFoldPrinting", variant = BookFoldPrinting, ty = Toggle),
        child(local = "bookFoldPrintingSheets", variant = BookFoldPrintingSheets, ty = DecimalNumberValue),
        child(local = "drawingGridHorizontalSpacing", variant = DrawingGridHorizontalSpacing, ty = TwipsMeasureValue),
        child(local = "drawingGridVerticalSpacing", variant = DrawingGridVerticalSpacing, ty = TwipsMeasureValue),
        child(local = "displayHorizontalDrawingGridEvery", variant = DisplayHorizontalDrawingGridEvery, ty = DecimalNumberValue),
        child(local = "displayVerticalDrawingGridEvery", variant = DisplayVerticalDrawingGridEvery, ty = DecimalNumberValue),
        child(local = "doNotUseMarginsForDrawingGridOrigin", variant = DoNotUseMarginsForDrawingGridOrigin, ty = Toggle),
        child(local = "drawingGridHorizontalOrigin", variant = DrawingGridHorizontalOrigin, ty = TwipsMeasureValue),
        child(local = "drawingGridVerticalOrigin", variant = DrawingGridVerticalOrigin, ty = TwipsMeasureValue),
        child(local = "doNotShadeFormData", variant = DoNotShadeFormData, ty = Toggle),
        child(local = "noPunctuationKerning", variant = NoPunctuationKerning, ty = Toggle),
        child(local = "characterSpacingControl", variant = CharacterSpacingControl, ty = CharacterSpacingSetting),
        child(local = "printTwoOnOne", variant = PrintTwoOnOne, ty = Toggle),
        child(local = "strictFirstAndLastChars", variant = StrictFirstAndLastChars, ty = Toggle),
        child(local = "noLineBreaksAfter", variant = NoLineBreaksAfter, ty = Kinsoku),
        child(local = "noLineBreaksBefore", variant = NoLineBreaksBefore, ty = Kinsoku),
        child(local = "savePreviewPicture", variant = SavePreviewPicture, ty = Toggle),
        child(local = "doNotValidateAgainstSchema", variant = DoNotValidateAgainstSchema, ty = Toggle),
        child(local = "saveInvalidXml", variant = SaveInvalidXml, ty = Toggle),
        child(local = "ignoreMixedContent", variant = IgnoreMixedContent, ty = Toggle),
        child(local = "alwaysShowPlaceholderText", variant = AlwaysShowPlaceholderText, ty = Toggle),
        child(local = "doNotDemarcateInvalidXml", variant = DoNotDemarcateInvalidXml, ty = Toggle),
        child(local = "saveXmlDataOnly", variant = SaveXmlDataOnly, ty = Toggle),
        child(local = "useXSLTWhenSaving", variant = UseXsltWhenSaving, ty = Toggle),
        child(local = "saveThroughXslt", variant = SaveThroughXslt, ty = SaveThroughXsltSetting),
        child(local = "showXMLTags", variant = ShowXmlTags, ty = Toggle),
        child(local = "alwaysMergeEmptyNamespace", variant = AlwaysMergeEmptyNamespace, ty = Toggle),
        child(local = "updateFields", variant = UpdateFields, ty = Toggle),
        child(local = "hdrShapeDefaults", variant = HeaderShapeDefaults, ty = Unmodeled),
        child(local = "footnotePr", variant = FootnoteProperties, ty = FootnoteDocumentDefaults),
        child(local = "endnotePr", variant = EndnoteProperties, ty = EndnoteDocumentDefaults),
        child(local = "compat", variant = Compat, ty = Compatibility),
        child(local = "docVars", variant = DocVars, ty = DocumentVariables),
        child(local = "rsids", variant = Rsids, ty = DocumentRevisionSaveIds),
        child(local = "mathPr", ns = SHARED_MATH, variant = MathProperties, ty = mjx_omml::MathProperties),
        child(local = "attachedSchema", variant = AttachedSchema, ty = StyleString),
        child(local = "themeFontLang", variant = ThemeFontLang, ty = Languages),
        child(local = "clrSchemeMapping", variant = ColorSchemeMappingChild, ty = ColorSchemeMapping),
        child(local = "doNotIncludeSubdocsInStats", variant = DoNotIncludeSubdocsInStats, ty = Toggle),
        child(local = "doNotAutoCompressPictures", variant = DoNotAutoCompressPictures, ty = Toggle),
        child(local = "forceUpgrade", variant = ForceUpgrade, ty = Unmodeled),
        child(local = "captions", variant = Captions, ty = CaptionsSetting),
        child(local = "readModeInkLockDown", variant = ReadModeInkLockDown, ty = ReadingModeInkLockDown),
        child(local = "smartTagType", variant = SmartTagType, ty = SmartTagTypeEntry),
        child(local = "shapeDefaults", variant = ShapeDefaults, ty = Unmodeled),
        child(local = "doNotEmbedSmartTags", variant = DoNotEmbedSmartTags, ty = Toggle),
        child(local = "decimalSymbol", variant = DecimalSymbol, ty = StyleString),
        child(local = "listSeparator", variant = ListSeparator, ty = StyleString)
    )]
    content: Vec<SettingsContent>,
}

/// One child of [`DocumentSettings`]: `CT_Settings`'s own 98, then anything this crate does not
/// model (an unknown extension, or the one schema-known-but-foreign `sl:schemaLibrary` — see the
/// module's own doc comment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsContent {
    /// `w:writeProtection` (§17.15.1.101).
    WriteProtection(WriteProtectionSetting),
    /// `w:view` (§17.15.1.99).
    View(ViewSetting),
    /// `w:zoom` (§17.15.1.104).
    Zoom(ZoomSetting),
    /// `w:removePersonalInformation` — `CT_OnOff`.
    RemovePersonalInformation(Toggle),
    /// `w:removeDateAndTime` — `CT_OnOff`.
    RemoveDateAndTime(Toggle),
    /// `w:doNotDisplayPageBoundaries` — `CT_OnOff`.
    DoNotDisplayPageBoundaries(Toggle),
    /// `w:displayBackgroundShape` — `CT_OnOff`.
    DisplayBackgroundShape(Toggle),
    /// `w:printPostScriptOverText` — `CT_OnOff`.
    PrintPostScriptOverText(Toggle),
    /// `w:printFractionalCharacterWidth` — `CT_OnOff`.
    PrintFractionalCharacterWidth(Toggle),
    /// `w:printFormsData` — `CT_OnOff`.
    PrintFormsData(Toggle),
    /// `w:embedTrueTypeFonts` — `CT_OnOff`.
    EmbedTrueTypeFonts(Toggle),
    /// `w:embedSystemFonts` — `CT_OnOff`.
    EmbedSystemFonts(Toggle),
    /// `w:saveSubsetFonts` — `CT_OnOff`.
    SaveSubsetFonts(Toggle),
    /// `w:saveFormsData` — `CT_OnOff`.
    SaveFormsData(Toggle),
    /// `w:mirrorMargins` (§17.15.1.71) — needed by C9's section model.
    MirrorMargins(Toggle),
    /// `w:alignBordersAndEdges` — `CT_OnOff`.
    AlignBordersAndEdges(Toggle),
    /// `w:bordersDoNotSurroundHeader` — `CT_OnOff`.
    BordersDoNotSurroundHeader(Toggle),
    /// `w:bordersDoNotSurroundFooter` — `CT_OnOff`.
    BordersDoNotSurroundFooter(Toggle),
    /// `w:gutterAtTop` — `CT_OnOff`.
    GutterAtTop(Toggle),
    /// `w:hideSpellingErrors` — `CT_OnOff`.
    HideSpellingErrors(Toggle),
    /// `w:hideGrammaticalErrors` — `CT_OnOff`.
    HideGrammaticalErrors(Toggle),
    /// `w:activeWritingStyle` (§17.15.1.1) — repeatable.
    ActiveWritingStyle(WritingStyleSetting),
    /// `w:proofState` (§17.15.1.80).
    ProofState(ProofSettings),
    /// `w:formsDesign` — `CT_OnOff`.
    FormsDesign(Toggle),
    /// `w:attachedTemplate` (§17.15.1.4) — `CT_Rel`.
    AttachedTemplate(RelationshipReference),
    /// `w:linkStyles` — `CT_OnOff`.
    LinkStyles(Toggle),
    /// `w:stylePaneFormatFilter` (§17.15.1.90).
    StylePaneFormatFilter(StylePaneFilter),
    /// `w:stylePaneSortMethod` (§17.15.1.91).
    StylePaneSortMethod(StyleSortSetting),
    /// `w:documentType` (§17.15.1.28).
    DocumentType(DocumentTypeSetting),
    /// `w:mailMerge` (§17.15.1.66) — `super::mail_merge`'s own cluster.
    MailMerge(super::mail_merge::MailMergeSettings),
    /// `w:revisionView` (§17.15.1.85).
    RevisionView(TrackChangesView),
    /// `w:trackRevisions` (§17.15.1.97) — `CT_OnOff`. **Not** `w:trackChanges`; `wml.xsd` spells the
    /// wire element `trackRevisions` (verified directly).
    TrackRevisions(Toggle),
    /// `w:doNotTrackMoves` — `CT_OnOff`.
    DoNotTrackMoves(Toggle),
    /// `w:doNotTrackFormatting` — `CT_OnOff`.
    DoNotTrackFormatting(Toggle),
    /// `w:documentProtection` (§17.15.1.29) — the password hash is preserved exactly, never
    /// recomputed (see the module's own doc comment).
    DocumentProtectionChild(DocumentProtection),
    /// `w:autoFormatOverride` — `CT_OnOff`.
    AutoFormatOverride(Toggle),
    /// `w:styleLockTheme` — `CT_OnOff`.
    StyleLockTheme(Toggle),
    /// `w:styleLockQFSet` — `CT_OnOff`.
    StyleLockQuickFormatSet(Toggle),
    /// `w:defaultTabStop` (§17.15.1.20) — needed by C4's tab resolution.
    DefaultTabStop(TwipsMeasureValue),
    /// `w:autoHyphenation` — `CT_OnOff`.
    AutoHyphenation(Toggle),
    /// `w:consecutiveHyphenLimit` — `CT_DecimalNumber`.
    ConsecutiveHyphenLimit(DecimalNumberValue),
    /// `w:hyphenationZone` — `CT_TwipsMeasure`.
    HyphenationZone(TwipsMeasureValue),
    /// `w:doNotHyphenateCaps` — `CT_OnOff`.
    DoNotHyphenateCaps(Toggle),
    /// `w:showEnvelope` — `CT_OnOff`.
    ShowEnvelope(Toggle),
    /// `w:summaryLength` — `CT_DecimalNumberOrPrecent`.
    SummaryLength(DecimalOrPercentValue),
    /// `w:clickAndTypeStyle` — `CT_String`.
    ClickAndTypeStyle(StyleString),
    /// `w:defaultTableStyle` — `CT_String`.
    DefaultTableStyle(StyleString),
    /// `w:evenAndOddHeaders` (§17.15.1.32) — `CT_OnOff`. Replaces MJXOFF-113's ad-hoc read; see
    /// [`DocumentSettings::even_and_odd_headers`].
    EvenAndOddHeaders(Toggle),
    /// `w:bookFoldRevPrinting` — `CT_OnOff`.
    BookFoldRevPrinting(Toggle),
    /// `w:bookFoldPrinting` — `CT_OnOff`.
    BookFoldPrinting(Toggle),
    /// `w:bookFoldPrintingSheets` — `CT_DecimalNumber`.
    BookFoldPrintingSheets(DecimalNumberValue),
    /// `w:drawingGridHorizontalSpacing` — `CT_TwipsMeasure`.
    DrawingGridHorizontalSpacing(TwipsMeasureValue),
    /// `w:drawingGridVerticalSpacing` — `CT_TwipsMeasure`.
    DrawingGridVerticalSpacing(TwipsMeasureValue),
    /// `w:displayHorizontalDrawingGridEvery` — `CT_DecimalNumber`.
    DisplayHorizontalDrawingGridEvery(DecimalNumberValue),
    /// `w:displayVerticalDrawingGridEvery` — `CT_DecimalNumber`.
    DisplayVerticalDrawingGridEvery(DecimalNumberValue),
    /// `w:doNotUseMarginsForDrawingGridOrigin` — `CT_OnOff`.
    DoNotUseMarginsForDrawingGridOrigin(Toggle),
    /// `w:drawingGridHorizontalOrigin` — `CT_TwipsMeasure`.
    DrawingGridHorizontalOrigin(TwipsMeasureValue),
    /// `w:drawingGridVerticalOrigin` — `CT_TwipsMeasure`.
    DrawingGridVerticalOrigin(TwipsMeasureValue),
    /// `w:doNotShadeFormData` — `CT_OnOff`.
    DoNotShadeFormData(Toggle),
    /// `w:noPunctuationKerning` — `CT_OnOff`.
    NoPunctuationKerning(Toggle),
    /// `w:characterSpacingControl` (§17.15.1.10).
    CharacterSpacingControl(CharacterSpacingSetting),
    /// `w:printTwoOnOne` — `CT_OnOff`.
    PrintTwoOnOne(Toggle),
    /// `w:strictFirstAndLastChars` — `CT_OnOff`.
    StrictFirstAndLastChars(Toggle),
    /// `w:noLineBreaksAfter` (§17.15.1.73) — `CT_Kinsoku`.
    NoLineBreaksAfter(Kinsoku),
    /// `w:noLineBreaksBefore` (§17.15.1.74) — `CT_Kinsoku`.
    NoLineBreaksBefore(Kinsoku),
    /// `w:savePreviewPicture` — `CT_OnOff`.
    SavePreviewPicture(Toggle),
    /// `w:doNotValidateAgainstSchema` — `CT_OnOff`.
    DoNotValidateAgainstSchema(Toggle),
    /// `w:saveInvalidXml` — `CT_OnOff`.
    SaveInvalidXml(Toggle),
    /// `w:ignoreMixedContent` — `CT_OnOff`.
    IgnoreMixedContent(Toggle),
    /// `w:alwaysShowPlaceholderText` — `CT_OnOff`.
    AlwaysShowPlaceholderText(Toggle),
    /// `w:doNotDemarcateInvalidXml` — `CT_OnOff`.
    DoNotDemarcateInvalidXml(Toggle),
    /// `w:saveXmlDataOnly` — `CT_OnOff`.
    SaveXmlDataOnly(Toggle),
    /// `w:useXSLTWhenSaving` — `CT_OnOff`.
    UseXsltWhenSaving(Toggle),
    /// `w:saveThroughXslt` (§17.15.1.87).
    SaveThroughXslt(SaveThroughXsltSetting),
    /// `w:showXMLTags` — `CT_OnOff`.
    ShowXmlTags(Toggle),
    /// `w:alwaysMergeEmptyNamespace` — `CT_OnOff`.
    AlwaysMergeEmptyNamespace(Toggle),
    /// `w:updateFields` — `CT_OnOff`.
    UpdateFields(Toggle),
    /// `w:hdrShapeDefaults` (§17.15.1.44) — VML office-drawing defaults (`o:` namespace),
    /// structurally opaque: the wrapping element is recognised and placed correctly; its own `o:`
    /// content is preserved verbatim rather than modelled (see [`Unmodeled`]'s own doc comment).
    HeaderShapeDefaults(Unmodeled),
    /// `w:footnotePr` (§17.15.1.37) — the document's own footnote defaults, needed by C14.
    FootnoteProperties(FootnoteDocumentDefaults),
    /// `w:endnotePr` (§17.15.1.33) — the document's own endnote defaults, needed by C14.
    EndnoteProperties(EndnoteDocumentDefaults),
    /// `w:compat` (§17.15.1.13).
    Compat(Compatibility),
    /// `w:docVars` (§17.15.1.27).
    DocVars(DocumentVariables),
    /// `w:rsids` (§17.15.1.86) — C15's `w:rsid`-family data.
    Rsids(DocumentRevisionSaveIds),
    /// `m:mathPr` (§17.15.1.70, `CT_MathPr`) — `mjx-omml`'s own type, wired here per MJXOFF-134's
    /// module doc, which names this exact seam.
    MathProperties(mjx_omml::MathProperties),
    /// `w:attachedSchema` — `CT_String`, repeatable.
    AttachedSchema(StyleString),
    /// `w:themeFontLang` (§17.15.1.94).
    ThemeFontLang(Languages),
    /// `w:clrSchemeMapping` (§17.15.1.11).
    ColorSchemeMappingChild(ColorSchemeMapping),
    /// `w:doNotIncludeSubdocsInStats` — `CT_OnOff`.
    DoNotIncludeSubdocsInStats(Toggle),
    /// `w:doNotAutoCompressPictures` — `CT_OnOff`.
    DoNotAutoCompressPictures(Toggle),
    /// `w:forceUpgrade` — `CT_Empty`, structurally opaque via [`Unmodeled`] (never carries content
    /// or attributes per the schema; modelled this way rather than a bespoke zero-field type so its
    /// own presence/absence is still exactly round-tripped).
    ForceUpgrade(Unmodeled),
    /// `w:captions` (§17.15.1.9).
    Captions(CaptionsSetting),
    /// `w:readModeInkLockDown` (§17.15.1.83).
    ReadModeInkLockDown(ReadingModeInkLockDown),
    /// `w:smartTagType` — repeatable.
    SmartTagType(SmartTagTypeEntry),
    /// `w:shapeDefaults` (§17.15.1.88) — see [`SettingsContent::HeaderShapeDefaults`].
    ShapeDefaults(Unmodeled),
    /// `w:doNotEmbedSmartTags` — `CT_OnOff`.
    DoNotEmbedSmartTags(Toggle),
    /// `w:decimalSymbol` — `CT_String`.
    DecimalSymbol(StyleString),
    /// `w:listSeparator` — `CT_String`.
    ListSeparator(StyleString),
    /// Any other child: an element this crate does not model (a `w14:`/`w15:` extension, a future
    /// schema addition) **or** `sl:schemaLibrary` (§17.15.1.89 references it; the `sl:` schema
    /// library namespace is foreign to every schema this workspace validates against) — preserved
    /// verbatim, in its original position relative to every known neighbour.
    Raw(RawNode),
}

impl DocumentSettings {
    /// Builds a new, empty `w:settings` — every child absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "settings"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This content item's own wire local name, or `None` for [`SettingsContent::Raw`] — the single
    /// source of truth [`DocumentSettings::rank`] (schema placement) and every setter's `is_target`
    /// match both key off, mirroring `crate::document::fields`'s own `checkbox_local`/
    /// `text_input_local` pattern, generalized to `CT_Settings`'s full 97 named children.
    fn local(item: &SettingsContent) -> Option<&'static str> {
        Some(match item {
            SettingsContent::WriteProtection(_) => "writeProtection",
            SettingsContent::View(_) => "view",
            SettingsContent::Zoom(_) => "zoom",
            SettingsContent::RemovePersonalInformation(_) => "removePersonalInformation",
            SettingsContent::RemoveDateAndTime(_) => "removeDateAndTime",
            SettingsContent::DoNotDisplayPageBoundaries(_) => "doNotDisplayPageBoundaries",
            SettingsContent::DisplayBackgroundShape(_) => "displayBackgroundShape",
            SettingsContent::PrintPostScriptOverText(_) => "printPostScriptOverText",
            SettingsContent::PrintFractionalCharacterWidth(_) => "printFractionalCharacterWidth",
            SettingsContent::PrintFormsData(_) => "printFormsData",
            SettingsContent::EmbedTrueTypeFonts(_) => "embedTrueTypeFonts",
            SettingsContent::EmbedSystemFonts(_) => "embedSystemFonts",
            SettingsContent::SaveSubsetFonts(_) => "saveSubsetFonts",
            SettingsContent::SaveFormsData(_) => "saveFormsData",
            SettingsContent::MirrorMargins(_) => "mirrorMargins",
            SettingsContent::AlignBordersAndEdges(_) => "alignBordersAndEdges",
            SettingsContent::BordersDoNotSurroundHeader(_) => "bordersDoNotSurroundHeader",
            SettingsContent::BordersDoNotSurroundFooter(_) => "bordersDoNotSurroundFooter",
            SettingsContent::GutterAtTop(_) => "gutterAtTop",
            SettingsContent::HideSpellingErrors(_) => "hideSpellingErrors",
            SettingsContent::HideGrammaticalErrors(_) => "hideGrammaticalErrors",
            SettingsContent::ActiveWritingStyle(_) => "activeWritingStyle",
            SettingsContent::ProofState(_) => "proofState",
            SettingsContent::FormsDesign(_) => "formsDesign",
            SettingsContent::AttachedTemplate(_) => "attachedTemplate",
            SettingsContent::LinkStyles(_) => "linkStyles",
            SettingsContent::StylePaneFormatFilter(_) => "stylePaneFormatFilter",
            SettingsContent::StylePaneSortMethod(_) => "stylePaneSortMethod",
            SettingsContent::DocumentType(_) => "documentType",
            SettingsContent::MailMerge(_) => "mailMerge",
            SettingsContent::RevisionView(_) => "revisionView",
            SettingsContent::TrackRevisions(_) => "trackRevisions",
            SettingsContent::DoNotTrackMoves(_) => "doNotTrackMoves",
            SettingsContent::DoNotTrackFormatting(_) => "doNotTrackFormatting",
            SettingsContent::DocumentProtectionChild(_) => "documentProtection",
            SettingsContent::AutoFormatOverride(_) => "autoFormatOverride",
            SettingsContent::StyleLockTheme(_) => "styleLockTheme",
            SettingsContent::StyleLockQuickFormatSet(_) => "styleLockQFSet",
            SettingsContent::DefaultTabStop(_) => "defaultTabStop",
            SettingsContent::AutoHyphenation(_) => "autoHyphenation",
            SettingsContent::ConsecutiveHyphenLimit(_) => "consecutiveHyphenLimit",
            SettingsContent::HyphenationZone(_) => "hyphenationZone",
            SettingsContent::DoNotHyphenateCaps(_) => "doNotHyphenateCaps",
            SettingsContent::ShowEnvelope(_) => "showEnvelope",
            SettingsContent::SummaryLength(_) => "summaryLength",
            SettingsContent::ClickAndTypeStyle(_) => "clickAndTypeStyle",
            SettingsContent::DefaultTableStyle(_) => "defaultTableStyle",
            SettingsContent::EvenAndOddHeaders(_) => "evenAndOddHeaders",
            SettingsContent::BookFoldRevPrinting(_) => "bookFoldRevPrinting",
            SettingsContent::BookFoldPrinting(_) => "bookFoldPrinting",
            SettingsContent::BookFoldPrintingSheets(_) => "bookFoldPrintingSheets",
            SettingsContent::DrawingGridHorizontalSpacing(_) => "drawingGridHorizontalSpacing",
            SettingsContent::DrawingGridVerticalSpacing(_) => "drawingGridVerticalSpacing",
            SettingsContent::DisplayHorizontalDrawingGridEvery(_) => {
                "displayHorizontalDrawingGridEvery"
            }
            SettingsContent::DisplayVerticalDrawingGridEvery(_) => {
                "displayVerticalDrawingGridEvery"
            }
            SettingsContent::DoNotUseMarginsForDrawingGridOrigin(_) => {
                "doNotUseMarginsForDrawingGridOrigin"
            }
            SettingsContent::DrawingGridHorizontalOrigin(_) => "drawingGridHorizontalOrigin",
            SettingsContent::DrawingGridVerticalOrigin(_) => "drawingGridVerticalOrigin",
            SettingsContent::DoNotShadeFormData(_) => "doNotShadeFormData",
            SettingsContent::NoPunctuationKerning(_) => "noPunctuationKerning",
            SettingsContent::CharacterSpacingControl(_) => "characterSpacingControl",
            SettingsContent::PrintTwoOnOne(_) => "printTwoOnOne",
            SettingsContent::StrictFirstAndLastChars(_) => "strictFirstAndLastChars",
            SettingsContent::NoLineBreaksAfter(_) => "noLineBreaksAfter",
            SettingsContent::NoLineBreaksBefore(_) => "noLineBreaksBefore",
            SettingsContent::SavePreviewPicture(_) => "savePreviewPicture",
            SettingsContent::DoNotValidateAgainstSchema(_) => "doNotValidateAgainstSchema",
            SettingsContent::SaveInvalidXml(_) => "saveInvalidXml",
            SettingsContent::IgnoreMixedContent(_) => "ignoreMixedContent",
            SettingsContent::AlwaysShowPlaceholderText(_) => "alwaysShowPlaceholderText",
            SettingsContent::DoNotDemarcateInvalidXml(_) => "doNotDemarcateInvalidXml",
            SettingsContent::SaveXmlDataOnly(_) => "saveXmlDataOnly",
            SettingsContent::UseXsltWhenSaving(_) => "useXSLTWhenSaving",
            SettingsContent::SaveThroughXslt(_) => "saveThroughXslt",
            SettingsContent::ShowXmlTags(_) => "showXMLTags",
            SettingsContent::AlwaysMergeEmptyNamespace(_) => "alwaysMergeEmptyNamespace",
            SettingsContent::UpdateFields(_) => "updateFields",
            SettingsContent::HeaderShapeDefaults(_) => "hdrShapeDefaults",
            SettingsContent::FootnoteProperties(_) => "footnotePr",
            SettingsContent::EndnoteProperties(_) => "endnotePr",
            SettingsContent::Compat(_) => "compat",
            SettingsContent::DocVars(_) => "docVars",
            SettingsContent::Rsids(_) => "rsids",
            SettingsContent::MathProperties(_) => "mathPr",
            SettingsContent::AttachedSchema(_) => "attachedSchema",
            SettingsContent::ThemeFontLang(_) => "themeFontLang",
            SettingsContent::ColorSchemeMappingChild(_) => "clrSchemeMapping",
            SettingsContent::DoNotIncludeSubdocsInStats(_) => "doNotIncludeSubdocsInStats",
            SettingsContent::DoNotAutoCompressPictures(_) => "doNotAutoCompressPictures",
            SettingsContent::ForceUpgrade(_) => "forceUpgrade",
            SettingsContent::Captions(_) => "captions",
            SettingsContent::ReadModeInkLockDown(_) => "readModeInkLockDown",
            SettingsContent::SmartTagType(_) => "smartTagType",
            SettingsContent::ShapeDefaults(_) => "shapeDefaults",
            SettingsContent::DoNotEmbedSmartTags(_) => "doNotEmbedSmartTags",
            SettingsContent::DecimalSymbol(_) => "decimalSymbol",
            SettingsContent::ListSeparator(_) => "listSeparator",
            SettingsContent::Raw(_) => return None,
        })
    }

    /// This content item's schema rank, from the generated [`SETTINGS`] table — never hand-ordered
    /// (see the module's own doc comment and A7c's own history).
    fn rank(item: &SettingsContent) -> Option<u16> {
        Self::local(item).and_then(|local| SETTINGS.rank_of(None, local))
    }

    /// Removes the first content item for which `is_target` holds, if any.
    fn remove(&mut self, is_target: impl Fn(&SettingsContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    /// Inserts `item` (whose wire name is `local`) at its schema rank among the existing content.
    fn insert(&mut self, local: &str, item: SettingsContent) {
        let at = SETTINGS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Replaces (or inserts, at rank, or removes when `value` is `None`) the content item this
    /// setting occupies — the one write primitive every setter below uses.
    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&SettingsContent) -> bool,
        value: Option<SettingsContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// Every content item, in document order — the escape hatch for a caller that needs to walk
    /// settings this type gives no bespoke accessor for (e.g. every `w:smartTagType`/
    /// `w:attachedSchema` at once).
    #[must_use]
    pub fn content(&self) -> &[SettingsContent] {
        &self.content
    }

    toggle_property!(SettingsContent, remove_personal_information, set_remove_personal_information, RemovePersonalInformation, "removePersonalInformation", "`w:removePersonalInformation`.");
    toggle_property!(SettingsContent, remove_date_and_time, set_remove_date_and_time, RemoveDateAndTime, "removeDateAndTime", "`w:removeDateAndTime`.");
    toggle_property!(SettingsContent, do_not_display_page_boundaries, set_do_not_display_page_boundaries, DoNotDisplayPageBoundaries, "doNotDisplayPageBoundaries", "`w:doNotDisplayPageBoundaries`.");
    toggle_property!(SettingsContent, display_background_shape, set_display_background_shape, DisplayBackgroundShape, "displayBackgroundShape", "`w:displayBackgroundShape`.");
    toggle_property!(SettingsContent, print_post_script_over_text, set_print_post_script_over_text, PrintPostScriptOverText, "printPostScriptOverText", "`w:printPostScriptOverText`.");
    toggle_property!(SettingsContent, print_fractional_character_width, set_print_fractional_character_width, PrintFractionalCharacterWidth, "printFractionalCharacterWidth", "`w:printFractionalCharacterWidth`.");
    toggle_property!(SettingsContent, print_forms_data, set_print_forms_data, PrintFormsData, "printFormsData", "`w:printFormsData`.");
    toggle_property!(SettingsContent, embed_true_type_fonts, set_embed_true_type_fonts, EmbedTrueTypeFonts, "embedTrueTypeFonts", "`w:embedTrueTypeFonts`.");
    toggle_property!(SettingsContent, embed_system_fonts, set_embed_system_fonts, EmbedSystemFonts, "embedSystemFonts", "`w:embedSystemFonts`.");
    toggle_property!(SettingsContent, save_subset_fonts, set_save_subset_fonts, SaveSubsetFonts, "saveSubsetFonts", "`w:saveSubsetFonts`.");
    toggle_property!(SettingsContent, save_forms_data, set_save_forms_data, SaveFormsData, "saveFormsData", "`w:saveFormsData`.");
    toggle_property!(SettingsContent, mirror_margins, set_mirror_margins, MirrorMargins, "mirrorMargins", "`w:mirrorMargins` — needed by C9's section model.");
    toggle_property!(SettingsContent, align_borders_and_edges, set_align_borders_and_edges, AlignBordersAndEdges, "alignBordersAndEdges", "`w:alignBordersAndEdges`.");
    toggle_property!(SettingsContent, borders_do_not_surround_header, set_borders_do_not_surround_header, BordersDoNotSurroundHeader, "bordersDoNotSurroundHeader", "`w:bordersDoNotSurroundHeader`.");
    toggle_property!(SettingsContent, borders_do_not_surround_footer, set_borders_do_not_surround_footer, BordersDoNotSurroundFooter, "bordersDoNotSurroundFooter", "`w:bordersDoNotSurroundFooter`.");
    toggle_property!(SettingsContent, gutter_at_top, set_gutter_at_top, GutterAtTop, "gutterAtTop", "`w:gutterAtTop`.");
    toggle_property!(SettingsContent, hide_spelling_errors, set_hide_spelling_errors, HideSpellingErrors, "hideSpellingErrors", "`w:hideSpellingErrors`.");
    toggle_property!(SettingsContent, hide_grammatical_errors, set_hide_grammatical_errors, HideGrammaticalErrors, "hideGrammaticalErrors", "`w:hideGrammaticalErrors`.");
    toggle_property!(SettingsContent, forms_design, set_forms_design, FormsDesign, "formsDesign", "`w:formsDesign`.");
    toggle_property!(SettingsContent, link_styles, set_link_styles, LinkStyles, "linkStyles", "`w:linkStyles`.");
    toggle_property!(SettingsContent, track_revisions, set_track_revisions, TrackRevisions, "trackRevisions", "`w:trackRevisions` — needed by C15. Not `w:trackChanges`; see this type's own doc comment.");
    toggle_property!(SettingsContent, do_not_track_moves, set_do_not_track_moves, DoNotTrackMoves, "doNotTrackMoves", "`w:doNotTrackMoves`.");
    toggle_property!(SettingsContent, do_not_track_formatting, set_do_not_track_formatting, DoNotTrackFormatting, "doNotTrackFormatting", "`w:doNotTrackFormatting`.");
    toggle_property!(SettingsContent, auto_format_override, set_auto_format_override, AutoFormatOverride, "autoFormatOverride", "`w:autoFormatOverride`.");
    toggle_property!(SettingsContent, style_lock_theme, set_style_lock_theme, StyleLockTheme, "styleLockTheme", "`w:styleLockTheme`.");
    toggle_property!(SettingsContent, style_lock_quick_format_set, set_style_lock_quick_format_set, StyleLockQuickFormatSet, "styleLockQFSet", "`w:styleLockQFSet`.");
    toggle_property!(SettingsContent, auto_hyphenation, set_auto_hyphenation, AutoHyphenation, "autoHyphenation", "`w:autoHyphenation`.");
    toggle_property!(SettingsContent, do_not_hyphenate_caps, set_do_not_hyphenate_caps, DoNotHyphenateCaps, "doNotHyphenateCaps", "`w:doNotHyphenateCaps`.");
    toggle_property!(SettingsContent, show_envelope, set_show_envelope, ShowEnvelope, "showEnvelope", "`w:showEnvelope`.");
    toggle_property!(SettingsContent, even_and_odd_headers, set_even_and_odd_headers, EvenAndOddHeaders, "evenAndOddHeaders", "`w:evenAndOddHeaders` — replaces MJXOFF-113's ad-hoc read.");
    toggle_property!(SettingsContent, book_fold_rev_printing, set_book_fold_rev_printing, BookFoldRevPrinting, "bookFoldRevPrinting", "`w:bookFoldRevPrinting`.");
    toggle_property!(SettingsContent, book_fold_printing, set_book_fold_printing, BookFoldPrinting, "bookFoldPrinting", "`w:bookFoldPrinting`.");
    toggle_property!(SettingsContent, do_not_use_margins_for_drawing_grid_origin, set_do_not_use_margins_for_drawing_grid_origin, DoNotUseMarginsForDrawingGridOrigin, "doNotUseMarginsForDrawingGridOrigin", "`w:doNotUseMarginsForDrawingGridOrigin`.");
    toggle_property!(SettingsContent, do_not_shade_form_data, set_do_not_shade_form_data, DoNotShadeFormData, "doNotShadeFormData", "`w:doNotShadeFormData`.");
    toggle_property!(SettingsContent, no_punctuation_kerning, set_no_punctuation_kerning, NoPunctuationKerning, "noPunctuationKerning", "`w:noPunctuationKerning`.");
    toggle_property!(SettingsContent, print_two_on_one, set_print_two_on_one, PrintTwoOnOne, "printTwoOnOne", "`w:printTwoOnOne`.");
    toggle_property!(SettingsContent, strict_first_and_last_chars, set_strict_first_and_last_chars, StrictFirstAndLastChars, "strictFirstAndLastChars", "`w:strictFirstAndLastChars`.");
    toggle_property!(SettingsContent, save_preview_picture, set_save_preview_picture, SavePreviewPicture, "savePreviewPicture", "`w:savePreviewPicture`.");
    toggle_property!(SettingsContent, do_not_validate_against_schema, set_do_not_validate_against_schema, DoNotValidateAgainstSchema, "doNotValidateAgainstSchema", "`w:doNotValidateAgainstSchema`.");
    toggle_property!(SettingsContent, save_invalid_xml, set_save_invalid_xml, SaveInvalidXml, "saveInvalidXml", "`w:saveInvalidXml`.");
    toggle_property!(SettingsContent, ignore_mixed_content, set_ignore_mixed_content, IgnoreMixedContent, "ignoreMixedContent", "`w:ignoreMixedContent`.");
    toggle_property!(SettingsContent, always_show_placeholder_text, set_always_show_placeholder_text, AlwaysShowPlaceholderText, "alwaysShowPlaceholderText", "`w:alwaysShowPlaceholderText`.");
    toggle_property!(SettingsContent, do_not_demarcate_invalid_xml, set_do_not_demarcate_invalid_xml, DoNotDemarcateInvalidXml, "doNotDemarcateInvalidXml", "`w:doNotDemarcateInvalidXml`.");
    toggle_property!(SettingsContent, save_xml_data_only, set_save_xml_data_only, SaveXmlDataOnly, "saveXmlDataOnly", "`w:saveXmlDataOnly`.");
    toggle_property!(SettingsContent, use_xslt_when_saving, set_use_xslt_when_saving, UseXsltWhenSaving, "useXSLTWhenSaving", "`w:useXSLTWhenSaving`.");
    toggle_property!(SettingsContent, show_xml_tags, set_show_xml_tags, ShowXmlTags, "showXMLTags", "`w:showXMLTags`.");
    toggle_property!(SettingsContent, always_merge_empty_namespace, set_always_merge_empty_namespace, AlwaysMergeEmptyNamespace, "alwaysMergeEmptyNamespace", "`w:alwaysMergeEmptyNamespace`.");
    toggle_property!(SettingsContent, update_fields, set_update_fields, UpdateFields, "updateFields", "`w:updateFields`.");
    toggle_property!(SettingsContent, do_not_include_subdocs_in_stats, set_do_not_include_subdocs_in_stats, DoNotIncludeSubdocsInStats, "doNotIncludeSubdocsInStats", "`w:doNotIncludeSubdocsInStats`.");
    toggle_property!(SettingsContent, do_not_auto_compress_pictures, set_do_not_auto_compress_pictures, DoNotAutoCompressPictures, "doNotAutoCompressPictures", "`w:doNotAutoCompressPictures`.");
    toggle_property!(SettingsContent, do_not_embed_smart_tags, set_do_not_embed_smart_tags, DoNotEmbedSmartTags, "doNotEmbedSmartTags", "`w:doNotEmbedSmartTags`.");

    value_property!(SettingsContent, write_protection, set_write_protection, WriteProtection, WriteProtectionSetting, "writeProtection", "`w:writeProtection`.");
    value_property!(SettingsContent, view, set_view, View, ViewSetting, "view", "`w:view`.");
    value_property!(SettingsContent, zoom, set_zoom, Zoom, ZoomSetting, "zoom", "`w:zoom`.");
    value_property!(SettingsContent, proof_state, set_proof_state, ProofState, ProofSettings, "proofState", "`w:proofState`.");
    value_property!(SettingsContent, attached_template, set_attached_template, AttachedTemplate, RelationshipReference, "attachedTemplate", "`w:attachedTemplate`.");
    value_property!(SettingsContent, style_pane_format_filter, set_style_pane_format_filter, StylePaneFormatFilter, StylePaneFilter, "stylePaneFormatFilter", "`w:stylePaneFormatFilter`.");
    value_property!(SettingsContent, style_pane_sort_method, set_style_pane_sort_method, StylePaneSortMethod, StyleSortSetting, "stylePaneSortMethod", "`w:stylePaneSortMethod`.");
    value_property!(SettingsContent, document_type, set_document_type, DocumentType, DocumentTypeSetting, "documentType", "`w:documentType`.");
    value_property!(SettingsContent, mail_merge, set_mail_merge, MailMerge, super::mail_merge::MailMergeSettings, "mailMerge", "`w:mailMerge` — needed for a mail-merge document to stay one.");
    value_property!(SettingsContent, revision_view, set_revision_view, RevisionView, TrackChangesView, "revisionView", "`w:revisionView`.");
    value_property!(SettingsContent, document_protection, set_document_protection, DocumentProtectionChild, DocumentProtection, "documentProtection", "`w:documentProtection` — needed by C9. The password hash is preserved exactly; see the module's own doc comment.");
    value_property!(SettingsContent, default_tab_stop, set_default_tab_stop_element, DefaultTabStop, TwipsMeasureValue, "defaultTabStop", "`w:defaultTabStop` — needed by C4's tab resolution.");

    /// `w:defaultTabStop`'s own twips value, flattened — `None` if the element is absent, `Some`
    /// (fallibly) otherwise. The convenience form of [`DocumentSettings::default_tab_stop`] for a
    /// caller (C4's tab resolution) that wants the number, not the wrapper element.
    pub fn default_tab_stop_twips(
        &self,
        interner: &Interner,
    ) -> Result<Option<TwipsMeasure>, AttributeError> {
        self.default_tab_stop().map(|value| value.twips(interner)).transpose()
    }

    /// Sets `w:defaultTabStop` to `twips`, building the wrapper element for the caller — the
    /// convenience form of [`DocumentSettings::set_default_tab_stop_element`].
    pub fn set_default_tab_stop(&mut self, interner: &mut Interner, twips: TwipsMeasure) {
        self.set_default_tab_stop_element(Some(TwipsMeasureValue::new(
            interner,
            "defaultTabStop",
            twips,
        )));
    }
    value_property!(SettingsContent, consecutive_hyphen_limit, set_consecutive_hyphen_limit, ConsecutiveHyphenLimit, DecimalNumberValue, "consecutiveHyphenLimit", "`w:consecutiveHyphenLimit`.");
    value_property!(SettingsContent, hyphenation_zone, set_hyphenation_zone, HyphenationZone, TwipsMeasureValue, "hyphenationZone", "`w:hyphenationZone`.");
    value_property!(SettingsContent, summary_length, set_summary_length, SummaryLength, DecimalOrPercentValue, "summaryLength", "`w:summaryLength`.");
    value_property!(SettingsContent, click_and_type_style, set_click_and_type_style, ClickAndTypeStyle, StyleString, "clickAndTypeStyle", "`w:clickAndTypeStyle`.");
    value_property!(SettingsContent, default_table_style, set_default_table_style, DefaultTableStyle, StyleString, "defaultTableStyle", "`w:defaultTableStyle`.");
    value_property!(SettingsContent, book_fold_printing_sheets, set_book_fold_printing_sheets, BookFoldPrintingSheets, DecimalNumberValue, "bookFoldPrintingSheets", "`w:bookFoldPrintingSheets`.");
    value_property!(SettingsContent, drawing_grid_horizontal_spacing, set_drawing_grid_horizontal_spacing, DrawingGridHorizontalSpacing, TwipsMeasureValue, "drawingGridHorizontalSpacing", "`w:drawingGridHorizontalSpacing`.");
    value_property!(SettingsContent, drawing_grid_vertical_spacing, set_drawing_grid_vertical_spacing, DrawingGridVerticalSpacing, TwipsMeasureValue, "drawingGridVerticalSpacing", "`w:drawingGridVerticalSpacing`.");
    value_property!(SettingsContent, display_horizontal_drawing_grid_every, set_display_horizontal_drawing_grid_every, DisplayHorizontalDrawingGridEvery, DecimalNumberValue, "displayHorizontalDrawingGridEvery", "`w:displayHorizontalDrawingGridEvery`.");
    value_property!(SettingsContent, display_vertical_drawing_grid_every, set_display_vertical_drawing_grid_every, DisplayVerticalDrawingGridEvery, DecimalNumberValue, "displayVerticalDrawingGridEvery", "`w:displayVerticalDrawingGridEvery`.");
    value_property!(SettingsContent, drawing_grid_horizontal_origin, set_drawing_grid_horizontal_origin, DrawingGridHorizontalOrigin, TwipsMeasureValue, "drawingGridHorizontalOrigin", "`w:drawingGridHorizontalOrigin`.");
    value_property!(SettingsContent, drawing_grid_vertical_origin, set_drawing_grid_vertical_origin, DrawingGridVerticalOrigin, TwipsMeasureValue, "drawingGridVerticalOrigin", "`w:drawingGridVerticalOrigin`.");
    value_property!(SettingsContent, character_spacing_control, set_character_spacing_control, CharacterSpacingControl, CharacterSpacingSetting, "characterSpacingControl", "`w:characterSpacingControl`.");
    value_property!(SettingsContent, no_line_breaks_after, set_no_line_breaks_after, NoLineBreaksAfter, Kinsoku, "noLineBreaksAfter", "`w:noLineBreaksAfter`.");
    value_property!(SettingsContent, no_line_breaks_before, set_no_line_breaks_before, NoLineBreaksBefore, Kinsoku, "noLineBreaksBefore", "`w:noLineBreaksBefore`.");
    value_property!(SettingsContent, save_through_xslt, set_save_through_xslt, SaveThroughXslt, SaveThroughXsltSetting, "saveThroughXslt", "`w:saveThroughXslt`.");
    value_property!(SettingsContent, header_shape_defaults, set_header_shape_defaults, HeaderShapeDefaults, Unmodeled, "hdrShapeDefaults", "`w:hdrShapeDefaults` — VML office-drawing defaults, structurally opaque (see the module's own doc comment).");
    value_property!(SettingsContent, footnote_properties, set_footnote_properties, FootnoteProperties, FootnoteDocumentDefaults, "footnotePr", "`w:footnotePr` — needed by C14.");
    value_property!(SettingsContent, endnote_properties, set_endnote_properties, EndnoteProperties, EndnoteDocumentDefaults, "endnotePr", "`w:endnotePr` — needed by C14.");
    value_property!(SettingsContent, compat, set_compat, Compat, Compatibility, "compat", "`w:compat`.");
    value_property!(SettingsContent, doc_vars, set_doc_vars, DocVars, DocumentVariables, "docVars", "`w:docVars`.");
    value_property!(SettingsContent, rsids, set_rsids, Rsids, DocumentRevisionSaveIds, "rsids", "`w:rsids` — C15's `w:rsid`-family data.");
    value_property!(SettingsContent, math_properties, set_math_properties, MathProperties, mjx_omml::MathProperties, "mathPr", "`m:mathPr` — `mjx-omml`'s own type.");
    /// `w:themeFontLang` — the document's own theme-font language triple.
    #[must_use]
    pub fn theme_font_languages(&self) -> Option<&Languages> {
        self.content.iter().find_map(|item| match item {
            SettingsContent::ThemeFontLang(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:themeFontLang`. `value` is renamed to `themeFontLang`
    /// regardless of what it carried (`value` is [`Languages`] — the same `CT_Language` shape
    /// `w:lang` uses — so a value built via [`Languages::new`] still carries that element's own
    /// name until this setter corrects it; mirrors [`super::run_properties::Border::renamed`]'s own
    /// reasoning for a reused wire shape under several names).
    pub fn set_theme_font_languages(&mut self, interner: &mut Interner, value: Option<Languages>) {
        let is_target = |item: &SettingsContent| matches!(item, SettingsContent::ThemeFontLang(_));
        let value = value.map(|languages| languages.renamed(interner, "themeFontLang"));
        self.set(
            "themeFontLang",
            is_target,
            value.map(SettingsContent::ThemeFontLang),
        );
    }
    value_property!(SettingsContent, color_scheme_mapping, set_color_scheme_mapping, ColorSchemeMappingChild, ColorSchemeMapping, "clrSchemeMapping", "`w:clrSchemeMapping`.");
    value_property!(SettingsContent, captions, set_captions, Captions, CaptionsSetting, "captions", "`w:captions`.");
    value_property!(SettingsContent, read_mode_ink_lock_down, set_read_mode_ink_lock_down, ReadModeInkLockDown, ReadingModeInkLockDown, "readModeInkLockDown", "`w:readModeInkLockDown`.");
    value_property!(SettingsContent, shape_defaults, set_shape_defaults, ShapeDefaults, Unmodeled, "shapeDefaults", "`w:shapeDefaults` — see the module's own doc comment.");
    value_property!(SettingsContent, decimal_symbol, set_decimal_symbol, DecimalSymbol, StyleString, "decimalSymbol", "`w:decimalSymbol`.");
    value_property!(SettingsContent, list_separator, set_list_separator, ListSeparator, StyleString, "listSeparator", "`w:listSeparator`.");

    /// Every `w:activeWritingStyle`, in document order.
    pub fn active_writing_styles(&self) -> impl Iterator<Item = &WritingStyleSetting> {
        self.content.iter().filter_map(|item| match item {
            SettingsContent::ActiveWritingStyle(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:activeWritingStyle` at its schema rank.
    pub fn add_active_writing_style(&mut self, value: WritingStyleSetting) {
        self.insert("activeWritingStyle", SettingsContent::ActiveWritingStyle(value));
    }

    /// Every `w:attachedSchema`, in document order.
    pub fn attached_schemas(&self) -> impl Iterator<Item = &StyleString> {
        self.content.iter().filter_map(|item| match item {
            SettingsContent::AttachedSchema(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:attachedSchema` at its schema rank.
    pub fn add_attached_schema(&mut self, value: StyleString) {
        self.insert("attachedSchema", SettingsContent::AttachedSchema(value));
    }

    /// Every `w:smartTagType`, in document order.
    pub fn smart_tag_types(&self) -> impl Iterator<Item = &SmartTagTypeEntry> {
        self.content.iter().filter_map(|item| match item {
            SettingsContent::SmartTagType(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:smartTagType` at its schema rank.
    pub fn add_smart_tag_type(&mut self, value: SmartTagTypeEntry) {
        self.insert("smartTagType", SettingsContent::SmartTagType(value));
    }
}

// =================================================================================================
// Generic element wrappers reused across several `CT_Settings` leaves.
// =================================================================================================

/// `CT_TwipsMeasure` — a required measure in twentieths of a point, reused across
/// `w:defaultTabStop`, `w:hyphenationZone` and the four drawing-grid spacing/origin elements — one
/// wire shape under six different names, exactly as
/// [`super::paragraph_properties::DecimalNumberValue`] is reused across `CT_DecimalNumber`'s several.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Twips, accessor = twips, required))]
pub struct TwipsMeasureValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TwipsMeasureValue {
    /// Builds a new `local` element of `twips`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, twips: TwipsMeasure) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_twips(interner, twips);
        item
    }
}

impl FromXml for TwipsMeasureValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TwipsMeasureValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `CT_DecimalNumberOrPrecent` (`w:summaryLength`) — a required decimal number or percentage.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = DecimalOrPercent, accessor = value, required))]
pub struct DecimalOrPercentValue {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DecimalOrPercentValue {
    /// Builds a new `local` element of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, value: DecimalNumberOrPercentage) -> Self {
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

impl FromXml for DecimalOrPercentValue {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DecimalOrPercentValue {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// =================================================================================================
// Attribute-only leaves: CT_View, CT_Zoom, CT_WritingStyle, CT_Proof, CT_DocType,
// CT_StylePaneFilter, CT_StyleSort, CT_TrackChangesView, CT_Kinsoku, CT_SaveThroughXslt,
// CT_ColorSchemeMapping, CT_ReadingModeInkLockDown, CT_CharacterSpacing, CT_Charset (font table
// reuses `ThemeHexDigit`) — every accessor here is generated in full by `mjx_derive::XmlAttributes`.
// =================================================================================================

/// `w:writeProtection` (`CT_WriteProtection`, §17.15.1.101) — whether the document recommends
/// (never enforces on its own) opening read-only, plus the password hash guarding a change to this
/// flag. The hash is opaque text, never recomputed — see the module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "recommended", prefix = "w", codec = OnOff, accessor = recommended))]
#[xml(attribute(local = "algorithmName", prefix = "w", codec = TextCodec, accessor = algorithm_name))]
#[xml(attribute(local = "hashValue", prefix = "w", codec = TextCodec, accessor = hash_value))]
#[xml(attribute(local = "saltValue", prefix = "w", codec = TextCodec, accessor = salt_value))]
#[xml(attribute(local = "spinCount", prefix = "w", codec = Number<i64>, accessor = spin_count))]
#[xml(attribute(local = "cryptProviderType", prefix = "w", codec = TextCodec, accessor = crypt_provider_type))]
#[xml(attribute(local = "cryptAlgorithmClass", prefix = "w", codec = TextCodec, accessor = crypt_algorithm_class))]
#[xml(attribute(local = "cryptAlgorithmType", prefix = "w", codec = TextCodec, accessor = crypt_algorithm_type))]
#[xml(attribute(local = "cryptAlgorithmSid", prefix = "w", codec = Number<i64>, accessor = crypt_algorithm_sid))]
#[xml(attribute(local = "cryptSpinCount", prefix = "w", codec = Number<i64>, accessor = crypt_spin_count))]
#[xml(attribute(local = "cryptProvider", prefix = "w", codec = TextCodec, accessor = crypt_provider))]
#[xml(attribute(local = "algIdExt", prefix = "w", codec = LongHex, accessor = algorithm_id_extension))]
#[xml(attribute(local = "algIdExtSource", prefix = "w", codec = TextCodec, accessor = algorithm_id_extension_source))]
#[xml(attribute(local = "cryptProviderTypeExt", prefix = "w", codec = LongHex, accessor = crypt_provider_type_extension))]
#[xml(attribute(local = "cryptProviderTypeExtSource", prefix = "w", codec = TextCodec, accessor = crypt_provider_type_extension_source))]
#[xml(attribute(local = "hash", prefix = "w", codec = TextCodec, accessor = legacy_hash))]
#[xml(attribute(local = "salt", prefix = "w", codec = TextCodec, accessor = legacy_salt))]
pub struct WriteProtectionSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl WriteProtectionSetting {
    /// Builds a new, empty `w:writeProtection` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "writeProtection"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for WriteProtectionSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for WriteProtectionSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:view` (`CT_View`, §17.15.1.99) — which view Word should open the document in.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<DocumentView>, accessor = kind, required))]
pub struct ViewSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ViewSetting {
    /// Builds a new `w:view` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: DocumentView) -> Self {
        let mut item = Self {
            name: wml_name(interner, "view"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for ViewSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ViewSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:zoom` (`CT_Zoom`, §17.15.1.104) — the document's own zoom level.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<ZoomPreset>, accessor = preset))]
#[xml(attribute(local = "percent", prefix = "w", codec = DecimalOrPercent, accessor = percent, required))]
pub struct ZoomSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ZoomSetting {
    /// Builds a new `w:zoom` of `percent`, no preset stated.
    #[must_use]
    pub fn new(interner: &mut Interner, percent: DecimalNumberOrPercentage) -> Self {
        let mut item = Self {
            name: wml_name(interner, "zoom"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_percent(interner, percent);
        item
    }
}

impl FromXml for ZoomSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ZoomSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:activeWritingStyle` (`CT_WritingStyle`, §17.15.1.1) — one proofing language/engine
/// combination Word has checked spelling/grammar with in this document. Repeatable.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lang", prefix = "w", codec = Lang, accessor = language, required))]
#[xml(attribute(local = "vendorID", prefix = "w", codec = TextCodec, accessor = vendor_id, required))]
#[xml(attribute(local = "dllVersion", prefix = "w", codec = TextCodec, accessor = dll_version, required))]
#[xml(attribute(local = "nlCheck", prefix = "w", codec = OnOff, accessor = natural_language_check, default = false))]
#[xml(attribute(local = "checkStyle", prefix = "w", codec = OnOff, accessor = check_style, required))]
#[xml(attribute(local = "appName", prefix = "w", codec = TextCodec, accessor = application_name, required))]
pub struct WritingStyleSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl WritingStyleSetting {
    /// Builds a new, empty `w:activeWritingStyle` — every attribute absent until a setter states
    /// one (all six are `required`/`default`-backed per the schema, so a caller should set all
    /// six before writing this out).
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "activeWritingStyle"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for WritingStyleSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for WritingStyleSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:proofState` (`CT_Proof`, §17.15.1.80) — whether Word's last spelling/grammar pass found the
/// document clean.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "spelling", prefix = "w", codec = Enumeration<ProofingState>, accessor = spelling))]
#[xml(attribute(local = "grammar", prefix = "w", codec = Enumeration<ProofingState>, accessor = grammar))]
pub struct ProofSettings {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ProofSettings {
    /// Builds a new, empty `w:proofState` — both attributes absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "proofState"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for ProofSettings {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ProofSettings {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:stylePaneFormatFilter` (`CT_StylePaneFilter`, §17.15.1.90) — which categories of style the
/// Styles pane shows, plus a raw bitmask (`val`) some Word versions write alongside the named
/// flags.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "allStyles", prefix = "w", codec = OnOff, accessor = all_styles))]
#[xml(attribute(local = "customStyles", prefix = "w", codec = OnOff, accessor = custom_styles))]
#[xml(attribute(local = "latentStyles", prefix = "w", codec = OnOff, accessor = latent_styles))]
#[xml(attribute(local = "stylesInUse", prefix = "w", codec = OnOff, accessor = styles_in_use))]
#[xml(attribute(local = "headingStyles", prefix = "w", codec = OnOff, accessor = heading_styles))]
#[xml(attribute(local = "numberingStyles", prefix = "w", codec = OnOff, accessor = numbering_styles))]
#[xml(attribute(local = "tableStyles", prefix = "w", codec = OnOff, accessor = table_styles))]
#[xml(attribute(local = "directFormattingOnRuns", prefix = "w", codec = OnOff, accessor = direct_formatting_on_runs))]
#[xml(attribute(local = "directFormattingOnParagraphs", prefix = "w", codec = OnOff, accessor = direct_formatting_on_paragraphs))]
#[xml(attribute(local = "directFormattingOnNumbering", prefix = "w", codec = OnOff, accessor = direct_formatting_on_numbering))]
#[xml(attribute(local = "directFormattingOnTables", prefix = "w", codec = OnOff, accessor = direct_formatting_on_tables))]
#[xml(attribute(local = "clearFormatting", prefix = "w", codec = OnOff, accessor = clear_formatting))]
#[xml(attribute(local = "top3HeadingStyles", prefix = "w", codec = OnOff, accessor = top_three_heading_styles))]
#[xml(attribute(local = "visibleStyles", prefix = "w", codec = OnOff, accessor = visible_styles))]
#[xml(attribute(local = "alternateStyleNames", prefix = "w", codec = OnOff, accessor = alternate_style_names))]
#[xml(attribute(local = "val", prefix = "w", codec = super::body::ShortHex, accessor = bitmask))]
pub struct StylePaneFilter {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl StylePaneFilter {
    /// Builds a new, empty `w:stylePaneFormatFilter` — every attribute absent until a setter
    /// states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "stylePaneFormatFilter"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for StylePaneFilter {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for StylePaneFilter {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:stylePaneSortMethod` (`CT_StyleSort`, §17.15.1.91) — how the Styles pane orders its list.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<StyleSortMethod>, accessor = method, required))]
pub struct StyleSortSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl StyleSortSetting {
    /// Builds a new `w:stylePaneSortMethod` of `method`.
    #[must_use]
    pub fn new(interner: &mut Interner, method: StyleSortMethod) -> Self {
        let mut item = Self {
            name: wml_name(interner, "stylePaneSortMethod"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_method(interner, method);
        item
    }
}

impl FromXml for StyleSortSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for StyleSortSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:documentType` (`CT_DocType`, §17.15.1.28) — a free-form classification string (`"letter"`
/// and similar; unrestricted, so `Word` and templates may write anything).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = DocTypeCodec, accessor = value, required))]
pub struct DocumentTypeSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DocumentTypeSetting {
    /// Builds a new `w:documentType` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: DocumentClassification) -> Self {
        let mut item = Self {
            name: wml_name(interner, "documentType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for DocumentTypeSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DocumentTypeSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:revisionView` (`CT_TrackChangesView`, §17.15.1.85) — which kinds of tracked change the
/// editing view shows.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "markup", prefix = "w", codec = OnOff, accessor = markup))]
#[xml(attribute(local = "comments", prefix = "w", codec = OnOff, accessor = comments))]
#[xml(attribute(local = "insDel", prefix = "w", codec = OnOff, accessor = insertions_and_deletions))]
#[xml(attribute(local = "formatting", prefix = "w", codec = OnOff, accessor = formatting))]
#[xml(attribute(local = "inkAnnotations", prefix = "w", codec = OnOff, accessor = ink_annotations))]
pub struct TrackChangesView {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl TrackChangesView {
    /// Builds a new, empty `w:revisionView` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "revisionView"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for TrackChangesView {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for TrackChangesView {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:documentProtection` (`CT_DocProtect`, §17.15.1.29) — the editing restriction (if any) and the
/// password hash guarding a change to it. **The hash is opaque text, preserved exactly — never
/// recomputed, never cleared** (see the module's own doc comment).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "edit", prefix = "w", codec = Enumeration<EditRestriction>, accessor = edit_restriction))]
#[xml(attribute(local = "formatting", prefix = "w", codec = OnOff, accessor = formatting))]
#[xml(attribute(local = "enforcement", prefix = "w", codec = OnOff, accessor = enforcement))]
#[xml(attribute(local = "algorithmName", prefix = "w", codec = TextCodec, accessor = algorithm_name))]
#[xml(attribute(local = "hashValue", prefix = "w", codec = TextCodec, accessor = hash_value))]
#[xml(attribute(local = "saltValue", prefix = "w", codec = TextCodec, accessor = salt_value))]
#[xml(attribute(local = "spinCount", prefix = "w", codec = Number<i64>, accessor = spin_count))]
#[xml(attribute(local = "cryptProviderType", prefix = "w", codec = TextCodec, accessor = crypt_provider_type))]
#[xml(attribute(local = "cryptAlgorithmClass", prefix = "w", codec = TextCodec, accessor = crypt_algorithm_class))]
#[xml(attribute(local = "cryptAlgorithmType", prefix = "w", codec = TextCodec, accessor = crypt_algorithm_type))]
#[xml(attribute(local = "cryptAlgorithmSid", prefix = "w", codec = Number<i64>, accessor = crypt_algorithm_sid))]
#[xml(attribute(local = "cryptSpinCount", prefix = "w", codec = Number<i64>, accessor = crypt_spin_count))]
#[xml(attribute(local = "cryptProvider", prefix = "w", codec = TextCodec, accessor = crypt_provider))]
#[xml(attribute(local = "algIdExt", prefix = "w", codec = LongHex, accessor = algorithm_id_extension))]
#[xml(attribute(local = "algIdExtSource", prefix = "w", codec = TextCodec, accessor = algorithm_id_extension_source))]
#[xml(attribute(local = "cryptProviderTypeExt", prefix = "w", codec = LongHex, accessor = crypt_provider_type_extension))]
#[xml(attribute(local = "cryptProviderTypeExtSource", prefix = "w", codec = TextCodec, accessor = crypt_provider_type_extension_source))]
#[xml(attribute(local = "hash", prefix = "w", codec = TextCodec, accessor = legacy_hash))]
#[xml(attribute(local = "salt", prefix = "w", codec = TextCodec, accessor = legacy_salt))]
pub struct DocumentProtection {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DocumentProtection {
    /// Builds a new, empty `w:documentProtection` — every attribute absent until a setter states
    /// one. Building this value never computes a password hash; a caller preserving an existing
    /// protection while changing `edit`/`formatting`/`enforcement` should read the existing value
    /// (via [`DocumentSettings::document_protection`]) and mutate it in place rather than starting
    /// from here.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "documentProtection"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for DocumentProtection {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DocumentProtection {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:noLineBreaksAfter`/`w:noLineBreaksBefore` (`CT_Kinsoku`, §17.15.1.73/74) — the Japanese line
/// -breaking (kinsoku shori) character set that may not end/start a line, one wire shape under two
/// names.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lang", prefix = "w", codec = Lang, accessor = language, required))]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = characters, required))]
pub struct Kinsoku {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Kinsoku {
    /// Builds a new `local` element (`"noLineBreaksAfter"` or `"noLineBreaksBefore"`) — both
    /// attributes are `required`, so a caller should set both before writing this out.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for Kinsoku {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Kinsoku {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:saveThroughXslt` (`CT_SaveThroughXslt`, §17.15.1.87) — the XSLT transform Word applies on
/// save, as a relationship to its own part.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "r", codec = TextCodec, accessor = relationship_id))]
#[xml(attribute(local = "solutionID", prefix = "w", codec = TextCodec, accessor = solution_id))]
pub struct SaveThroughXsltSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SaveThroughXsltSetting {
    /// Builds a new, empty `w:saveThroughXslt` — both attributes absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "saveThroughXslt"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for SaveThroughXsltSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SaveThroughXsltSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:clrSchemeMapping` (`CT_ColorSchemeMapping`, §17.15.1.11) — which theme colour slot each of
/// the twelve legacy colour-scheme roles maps to.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "bg1", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = background_1))]
#[xml(attribute(local = "t1", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = text_1))]
#[xml(attribute(local = "bg2", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = background_2))]
#[xml(attribute(local = "t2", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = text_2))]
#[xml(attribute(local = "accent1", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = accent_1))]
#[xml(attribute(local = "accent2", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = accent_2))]
#[xml(attribute(local = "accent3", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = accent_3))]
#[xml(attribute(local = "accent4", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = accent_4))]
#[xml(attribute(local = "accent5", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = accent_5))]
#[xml(attribute(local = "accent6", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = accent_6))]
#[xml(attribute(local = "hyperlink", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = hyperlink))]
#[xml(attribute(local = "followedHyperlink", prefix = "w", codec = Enumeration<ColorSchemeSlot>, accessor = followed_hyperlink))]
pub struct ColorSchemeMapping {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ColorSchemeMapping {
    /// Builds a new, empty `w:clrSchemeMapping` — every attribute absent until a setter states
    /// one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "clrSchemeMapping"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for ColorSchemeMapping {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ColorSchemeMapping {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:readModeInkLockDown` (`CT_ReadingModeInkLockDown`, §17.15.1.83) — the fixed page size and
/// font-size scale Read Mode locks to when ink annotations are present.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "actualPg", prefix = "w", codec = OnOff, accessor = actual_page, required))]
#[xml(attribute(local = "w", prefix = "w", codec = Number<PixelsMeasure>, accessor = width, required))]
#[xml(attribute(local = "h", prefix = "w", codec = Number<PixelsMeasure>, accessor = height, required))]
#[xml(attribute(local = "fontSz", prefix = "w", codec = DecimalOrPercent, accessor = font_size, required))]
pub struct ReadingModeInkLockDown {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl ReadingModeInkLockDown {
    /// Builds a new, empty `w:readModeInkLockDown` — every attribute is `required`, so a caller
    /// should set all four before writing this out.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "readModeInkLockDown"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for ReadingModeInkLockDown {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for ReadingModeInkLockDown {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:characterSpacingControl` (`CT_CharacterSpacing`, §17.15.1.10) — how far East Asian
/// punctuation compression goes.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<CharacterSpacingCompression>, accessor = compression, required))]
pub struct CharacterSpacingSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CharacterSpacingSetting {
    /// Builds a new `w:characterSpacingControl` of `compression`.
    #[must_use]
    pub fn new(interner: &mut Interner, compression: CharacterSpacingCompression) -> Self {
        let mut item = Self {
            name: wml_name(interner, "characterSpacingControl"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_compression(interner, compression);
        item
    }
}

impl FromXml for CharacterSpacingSetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CharacterSpacingSetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:compatSetting` (`CT_CompatSetting`, §17.15.1.14) — one named, freeform compatibility
/// override; none of its three attributes is `required` per the schema.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = name))]
#[xml(attribute(local = "uri", prefix = "w", codec = TextCodec, accessor = uri))]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = value))]
pub struct CompatibilitySetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CompatibilitySetting {
    /// Builds a new, empty `w:compatSetting` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "compatSetting"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for CompatibilitySetting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CompatibilitySetting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:docVar` (`CT_DocVar`, §17.15.1.25) — one document variable: a name/value pair a field's
/// `DOCVARIABLE` instruction can read.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = name, required))]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = value, required))]
pub struct DocumentVariable {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DocumentVariable {
    /// Builds a new, empty `w:docVar` — both attributes are `required`, so a caller should set
    /// both before writing this out.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "docVar"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for DocumentVariable {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DocumentVariable {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:docVars` (`CT_DocVars`, §17.15.1.26) — the document's own `w:docVar` list.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DocumentVariables {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "docVar", variant = Var, ty = DocumentVariable))]
    content: Vec<DocumentVariablesContent>,
}

/// One child of [`DocumentVariables`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentVariablesContent {
    /// `w:docVar`.
    Var(DocumentVariable),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl DocumentVariables {
    /// Builds a new, empty `w:docVars`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "docVars"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `w:docVar`, in document order.
    pub fn variables(&self) -> impl Iterator<Item = &DocumentVariable> {
        self.content.iter().filter_map(|item| match item {
            DocumentVariablesContent::Var(value) => Some(value),
            DocumentVariablesContent::Raw(_) => None,
        })
    }

    /// The value of the document variable named `name`, if one exists.
    #[must_use]
    pub fn variable(&self, interner: &Interner, name: &str) -> Option<Cow<'_, str>> {
        self.variables().find_map(|var| {
            let found = var.name(interner).ok()?;
            (found == name).then(|| var.value(interner).ok()).flatten()
        })
    }

    /// Appends `variable` — the schema imposes no order among `w:docVar` siblings, so a new one
    /// always lands after every existing child (known or not).
    pub fn add_variable(&mut self, variable: DocumentVariable) {
        self.content.push(DocumentVariablesContent::Var(variable));
        self.empty = false;
    }

    /// Removes every `w:docVar` named `name`. Returns whether any were removed.
    pub fn remove_variable(&mut self, interner: &Interner, name: &str) -> bool {
        let before = self.content.len();
        self.content.retain(|item| match item {
            DocumentVariablesContent::Var(var) => {
                var.name(interner).ok().as_deref() != Some(name)
            }
            DocumentVariablesContent::Raw(_) => true,
        });
        self.content.len() != before
    }
}

/// `w:rsids` (`CT_DocRsids`, §17.15.1.86) — the document-wide roster of revision save ids C15's
/// `w:rsid`-family attributes reference, reusing [`super::styles::RevisionSaveId`]
/// (`CT_LongHexNumber`) — one wire shape under three names now (`w:rsid` in a style definition,
/// `w:rsidRoot`/`w:rsid` here).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct DocumentRevisionSaveIds {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rsidRoot", variant = Root, ty = RevisionSaveId),
        child(local = "rsid", variant = Entry, ty = RevisionSaveId)
    )]
    content: Vec<DocumentRevisionSaveIdsContent>,
}

/// One child of [`DocumentRevisionSaveIds`] — `w:rsidRoot` always ranks before every `w:rsid`
/// (`CT_DocRsids`'s own two-slot `xsd:sequence`; trivial enough to state directly rather than add a
/// third generated table for a two-slot type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentRevisionSaveIdsContent {
    /// `w:rsidRoot` — the save id the document had when the current `rsid` roster was last reset.
    Root(RevisionSaveId),
    /// `w:rsid` — one save id in the roster, repeatable.
    Entry(RevisionSaveId),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl DocumentRevisionSaveIds {
    /// Builds a new, empty `w:rsids`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "rsids"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// `w:rsidRoot`, if stated.
    #[must_use]
    pub fn root(&self) -> Option<&RevisionSaveId> {
        self.content.iter().find_map(|item| match item {
            DocumentRevisionSaveIdsContent::Root(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:rsidRoot`. `value`'s own element name is corrected to
    /// `rsidRoot` regardless of what it carried (mirrors [`super::run_properties::Border::renamed`]'s
    /// own reasoning for a reused wire shape under several names).
    pub fn set_root(&mut self, interner: &mut Interner, value: Option<RevisionSaveId>) {
        self.content
            .retain(|item| !matches!(item, DocumentRevisionSaveIdsContent::Root(_)));
        if let Some(value) = value {
            self.content.insert(
                0,
                DocumentRevisionSaveIdsContent::Root(value.renamed(interner, "rsidRoot")),
            );
        }
        self.empty = false;
    }

    /// Every `w:rsid` entry, in document order.
    pub fn entries(&self) -> impl Iterator<Item = &RevisionSaveId> {
        self.content.iter().filter_map(|item| match item {
            DocumentRevisionSaveIdsContent::Entry(value) => Some(value),
            _ => None,
        })
    }

    /// Appends `value` (renamed to `rsid` regardless of what it carried) as a new roster entry.
    pub fn add_entry(&mut self, interner: &mut Interner, value: RevisionSaveId) {
        self.content.push(DocumentRevisionSaveIdsContent::Entry(
            value.renamed(interner, "rsid"),
        ));
        self.empty = false;
    }
}

/// `w:compat` (`CT_Compat`, §17.15.1.13) — the compatibility-option flags a document carries
/// forward from the application that last saved it, then any number of `w:compatSetting` entries.
///
/// **The sixty-two individual `w:compat` flags are not modelled individually.** Every one is a bare
/// `CT_OnOff` (`w:wpJustification`, `w:noTabHangInd`, `w:usePrinterMetrics`, …) that nothing in
/// Phase C's own dependency list names by name, and modelling sixty-two near-identical single-use
/// enum variants (each needing its own [`super::property_macros::toggle_property`] invocation to be
/// worth anything) would add sixty-two API surfaces for zero call sites. They still round-trip
/// **exactly**, in position: [`CompatibilityContent::Raw`] is this crate's normal unknown-element
/// bucket, and every one of the sixty-two falls into it precisely because this type's own
/// `#[xml(children, ..)]` list names only `w:compatSetting`. `w:compatSetting` itself — the
/// schema's own generic, freeform escape hatch (§17.15.1.14, three optional string attributes) —
/// gets full accessors below, since it is the one child a caller plausibly wants to add without
/// this crate growing a bespoke type for it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Compatibility {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "compatSetting", variant = Setting, ty = CompatibilitySetting))]
    content: Vec<CompatibilityContent>,
}

/// One child of [`Compatibility`] this crate names — see that type's own doc comment for why the
/// sixty-two flag elements are not enumerated here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityContent {
    /// `w:compatSetting`.
    Setting(CompatibilitySetting),
    /// Any other child — every one of the sixty-two named flags, and any future extension —
    /// preserved verbatim, in position.
    Raw(RawNode),
}

impl Compatibility {
    /// Builds a new, empty `w:compat`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "compat"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `w:compatSetting`, in document order.
    pub fn settings(&self) -> impl Iterator<Item = &CompatibilitySetting> {
        self.content.iter().filter_map(|item| match item {
            CompatibilityContent::Setting(value) => Some(value),
            CompatibilityContent::Raw(_) => None,
        })
    }

    /// The `w:compatSetting` named `name` (its own `w:name` attribute), if one exists.
    #[must_use]
    pub fn setting(&self, interner: &Interner, name: &str) -> Option<&CompatibilitySetting> {
        self.settings()
            .find(|setting| setting.name(interner).ok().flatten().as_deref() == Some(name))
    }

    /// Appends `value` at its schema rank — always after every flag, known or not, since
    /// `w:compatSetting` is `CT_Compat`'s own last-ranked child.
    pub fn add_setting(&mut self, value: CompatibilitySetting) {
        let rank = |item: &CompatibilityContent| match item {
            CompatibilityContent::Setting(_) => COMPAT.rank_of(None, "compatSetting"),
            CompatibilityContent::Raw(_) => None,
        };
        let at = COMPAT.insert_index_of_names(self.content.iter().map(rank), "compatSetting");
        self.content.insert(at, CompatibilityContent::Setting(value));
        self.empty = false;
    }
}

/// `w:footnote` (`CT_FtnEdnSepRef`, §17.15.1.38) inside `w:settings/w:footnotePr`, or `w:endnote`
/// inside `w:settings/w:endnotePr` — one wire shape under two names, naming (by id) a footnote or
/// endnote in `word/footnotes.xml`/`word/endnotes.xml` that serves as the document's own separator,
/// continuation separator or continuation notice.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "w", codec = Number<i64>, accessor = id, required))]
pub struct SeparatorReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SeparatorReference {
    /// Builds a new `local` element (`"footnote"` or `"endnote"`) naming footnote/endnote `id`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, id: i64) -> Self {
        let mut item = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_id(interner, id);
        item
    }
}

impl FromXml for SeparatorReference {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SeparatorReference {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:footnotePr` (`CT_FtnDocProps`, §17.15.1.37) — the document's own footnote defaults: position,
/// number format, start number and restart rule (`CT_FtnProps`'s own four, reusing
/// [`super::annotations::FootnotePositionElement`]/`NumberFormatElement`/`NumberRestartElement`
/// exactly as that module's own section-level [`super::annotations::FootnoteProperties`] does), then
/// up to three [`SeparatorReference`]s naming which footnotes serve as separator/continuation
/// markers. Needed by C14.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct FootnoteDocumentDefaults {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pos", variant = Position, ty = super::annotations::FootnotePositionElement),
        child(local = "numFmt", variant = NumberFormat, ty = super::annotations::NumberFormatElement),
        child(local = "numStart", variant = NumberStart, ty = DecimalNumberValue),
        child(local = "numRestart", variant = NumberRestart, ty = super::annotations::NumberRestartElement),
        child(local = "footnote", variant = Reference, ty = SeparatorReference)
    )]
    content: Vec<FootnoteDocumentDefaultsContent>,
}

/// One child of [`FootnoteDocumentDefaults`]: `CT_FtnProps`'s own four, in schema order, then
/// `w:footnote` (repeatable, up to three) — five slots, hand-ordered directly from `wml.xsd` rather
/// than adding a fifth generated table for a five-slot type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootnoteDocumentDefaultsContent {
    /// `w:pos`.
    Position(super::annotations::FootnotePositionElement),
    /// `w:numFmt`.
    NumberFormat(super::annotations::NumberFormatElement),
    /// `w:numStart`.
    NumberStart(DecimalNumberValue),
    /// `w:numRestart`.
    NumberRestart(super::annotations::NumberRestartElement),
    /// `w:footnote` — repeatable, up to three.
    Reference(SeparatorReference),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl FootnoteDocumentDefaults {
    /// Builds a new, empty `w:footnotePr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "footnotePr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// This content item's schema rank (0..=4; see the enum's own doc comment).
    fn rank(item: &FootnoteDocumentDefaultsContent) -> Option<u16> {
        match item {
            FootnoteDocumentDefaultsContent::Position(_) => Some(0),
            FootnoteDocumentDefaultsContent::NumberFormat(_) => Some(1),
            FootnoteDocumentDefaultsContent::NumberStart(_) => Some(2),
            FootnoteDocumentDefaultsContent::NumberRestart(_) => Some(3),
            FootnoteDocumentDefaultsContent::Reference(_) => Some(4),
            FootnoteDocumentDefaultsContent::Raw(_) => None,
        }
    }

    fn insert_at_rank(&mut self, item: FootnoteDocumentDefaultsContent) {
        let rank = Self::rank(&item);
        let mut at = self.content.len();
        for (index, existing) in self.content.iter().enumerate() {
            if let (Some(rank), Some(existing_rank)) = (rank, Self::rank(existing)) {
                if existing_rank > rank {
                    at = index;
                    break;
                }
            }
        }
        self.content.insert(at, item);
        self.empty = false;
    }

    /// `w:numStart`, if stated.
    #[must_use]
    pub fn number_start(&self) -> Option<&DecimalNumberValue> {
        self.content.iter().find_map(|item| match item {
            FootnoteDocumentDefaultsContent::NumberStart(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:numStart`.
    pub fn set_number_start(&mut self, value: Option<DecimalNumberValue>) {
        self.content
            .retain(|item| !matches!(item, FootnoteDocumentDefaultsContent::NumberStart(_)));
        if let Some(value) = value {
            self.insert_at_rank(FootnoteDocumentDefaultsContent::NumberStart(value));
        }
    }

    /// Every `w:footnote` separator reference, in document order.
    pub fn separator_references(&self) -> impl Iterator<Item = &SeparatorReference> {
        self.content.iter().filter_map(|item| match item {
            FootnoteDocumentDefaultsContent::Reference(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:footnote` separator reference.
    pub fn add_separator_reference(&mut self, value: SeparatorReference) {
        self.insert_at_rank(FootnoteDocumentDefaultsContent::Reference(value));
    }
}

/// `w:endnotePr` (`CT_EdnDocProps`, §17.15.1.33) — as [`FootnoteDocumentDefaults`], for endnotes;
/// distinct only because `w:pos`'s own value type differs (`CT_EdnPos`, two values, vs.
/// `CT_FtnPos`'s four — mirrors [`super::annotations::EndnoteProperties`]'s own reasoning). Needed
/// by C14.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct EndnoteDocumentDefaults {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pos", variant = Position, ty = super::annotations::EndnotePositionElement),
        child(local = "numFmt", variant = NumberFormat, ty = super::annotations::NumberFormatElement),
        child(local = "numStart", variant = NumberStart, ty = DecimalNumberValue),
        child(local = "numRestart", variant = NumberRestart, ty = super::annotations::NumberRestartElement),
        child(local = "endnote", variant = Reference, ty = SeparatorReference)
    )]
    content: Vec<EndnoteDocumentDefaultsContent>,
}

/// One child of [`EndnoteDocumentDefaults`] — see [`FootnoteDocumentDefaultsContent`]'s own doc
/// comment for the ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndnoteDocumentDefaultsContent {
    /// `w:pos`.
    Position(super::annotations::EndnotePositionElement),
    /// `w:numFmt`.
    NumberFormat(super::annotations::NumberFormatElement),
    /// `w:numStart`.
    NumberStart(DecimalNumberValue),
    /// `w:numRestart`.
    NumberRestart(super::annotations::NumberRestartElement),
    /// `w:endnote` — repeatable, up to three.
    Reference(SeparatorReference),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl EndnoteDocumentDefaults {
    /// Builds a new, empty `w:endnotePr`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "endnotePr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &EndnoteDocumentDefaultsContent) -> Option<u16> {
        match item {
            EndnoteDocumentDefaultsContent::Position(_) => Some(0),
            EndnoteDocumentDefaultsContent::NumberFormat(_) => Some(1),
            EndnoteDocumentDefaultsContent::NumberStart(_) => Some(2),
            EndnoteDocumentDefaultsContent::NumberRestart(_) => Some(3),
            EndnoteDocumentDefaultsContent::Reference(_) => Some(4),
            EndnoteDocumentDefaultsContent::Raw(_) => None,
        }
    }

    fn insert_at_rank(&mut self, item: EndnoteDocumentDefaultsContent) {
        let rank = Self::rank(&item);
        let mut at = self.content.len();
        for (index, existing) in self.content.iter().enumerate() {
            if let (Some(rank), Some(existing_rank)) = (rank, Self::rank(existing)) {
                if existing_rank > rank {
                    at = index;
                    break;
                }
            }
        }
        self.content.insert(at, item);
        self.empty = false;
    }

    /// `w:numStart`, if stated.
    #[must_use]
    pub fn number_start(&self) -> Option<&DecimalNumberValue> {
        self.content.iter().find_map(|item| match item {
            EndnoteDocumentDefaultsContent::NumberStart(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:numStart`.
    pub fn set_number_start(&mut self, value: Option<DecimalNumberValue>) {
        self.content
            .retain(|item| !matches!(item, EndnoteDocumentDefaultsContent::NumberStart(_)));
        if let Some(value) = value {
            self.insert_at_rank(EndnoteDocumentDefaultsContent::NumberStart(value));
        }
    }

    /// Every `w:endnote` separator reference, in document order.
    pub fn separator_references(&self) -> impl Iterator<Item = &SeparatorReference> {
        self.content.iter().filter_map(|item| match item {
            EndnoteDocumentDefaultsContent::Reference(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:endnote` separator reference.
    pub fn add_separator_reference(&mut self, value: SeparatorReference) {
        self.insert_at_rank(EndnoteDocumentDefaultsContent::Reference(value));
    }
}

/// `w:smartTagType` (`CT_SmartTagType`, §17.15.1.89) — one recognized smart-tag type, repeatable.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "namespaceuri", prefix = "w", codec = TextCodec, accessor = namespace_uri))]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = name))]
#[xml(attribute(local = "url", prefix = "w", codec = TextCodec, accessor = url))]
pub struct SmartTagTypeEntry {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SmartTagTypeEntry {
    /// Builds a new, empty `w:smartTagType` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "smartTagType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for SmartTagTypeEntry {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SmartTagTypeEntry {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:caption` (`CT_Caption`, §17.15.1.7) — one caption label AutoCaption/manual captioning uses.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = name, required))]
#[xml(attribute(local = "pos", prefix = "w", codec = Enumeration<CaptionPosition>, accessor = position))]
#[xml(attribute(local = "chapNum", prefix = "w", codec = OnOff, accessor = chapter_number))]
#[xml(attribute(local = "heading", prefix = "w", codec = Number<i64>, accessor = heading_style_level))]
#[xml(attribute(local = "noLabel", prefix = "w", codec = OnOff, accessor = no_label))]
#[xml(attribute(local = "numFmt", prefix = "w", codec = Enumeration<NumberFormat>, accessor = number_format))]
#[xml(attribute(local = "sep", prefix = "w", codec = Enumeration<ChapterSeparator>, accessor = chapter_separator))]
pub struct CaptionLabel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl CaptionLabel {
    /// Builds a new `w:caption` labelled `name` — every other attribute absent until a setter
    /// states one.
    #[must_use]
    pub fn new(interner: &mut Interner, name: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "caption"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_name(interner, name);
        item
    }
}

impl FromXml for CaptionLabel {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for CaptionLabel {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:autoCaption` (`CT_AutoCaption`, §17.15.1.6) — one file-type-to-caption-label mapping for
/// automatic captioning.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", prefix = "w", codec = TextCodec, accessor = object_name, required))]
#[xml(attribute(local = "caption", prefix = "w", codec = TextCodec, accessor = caption_name, required))]
pub struct AutoCaptionEntry {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl AutoCaptionEntry {
    /// Builds a new, empty `w:autoCaption` — both attributes are `required`, so a caller should
    /// set both before writing this out.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "autoCaption"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for AutoCaptionEntry {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for AutoCaptionEntry {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:autoCaptions` (`CT_AutoCaptions`, §17.15.1.5) — the automatic-captioning table.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct AutoCaptionsSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "autoCaption", variant = Entry, ty = AutoCaptionEntry))]
    content: Vec<AutoCaptionsContent>,
}

/// One child of [`AutoCaptionsSetting`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoCaptionsContent {
    /// `w:autoCaption` — repeatable.
    Entry(AutoCaptionEntry),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl AutoCaptionsSetting {
    /// Builds a new, empty `w:autoCaptions`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "autoCaptions"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `w:autoCaption`, in document order.
    pub fn entries(&self) -> impl Iterator<Item = &AutoCaptionEntry> {
        self.content.iter().filter_map(|item| match item {
            AutoCaptionsContent::Entry(value) => Some(value),
            AutoCaptionsContent::Raw(_) => None,
        })
    }

    /// Appends `value` — the schema imposes no order among `w:autoCaption` siblings.
    pub fn add_entry(&mut self, value: AutoCaptionEntry) {
        self.content.push(AutoCaptionsContent::Entry(value));
        self.empty = false;
    }
}

/// `w:captions` (`CT_Captions`, §17.15.1.9) — the document's own caption-label and
/// automatic-captioning configuration.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct CaptionsSetting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "caption", variant = Label, ty = CaptionLabel),
        child(local = "autoCaptions", variant = AutoCaptions, ty = AutoCaptionsSetting)
    )]
    content: Vec<CaptionsContent>,
}

/// One child of [`CaptionsSetting`]: `w:caption` (repeatable, at least one per the schema) always
/// ranks before the single optional `w:autoCaptions` — `CT_Captions`'s own two-slot
/// `xsd:sequence`, hand-ordered directly for the same reason as
/// [`DocumentRevisionSaveIdsContent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptionsContent {
    /// `w:caption` — repeatable.
    Label(CaptionLabel),
    /// `w:autoCaptions`.
    AutoCaptions(AutoCaptionsSetting),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

impl CaptionsSetting {
    /// Builds a new, empty `w:captions`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "captions"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &CaptionsContent) -> Option<u16> {
        match item {
            CaptionsContent::Label(_) => Some(0),
            CaptionsContent::AutoCaptions(_) => Some(1),
            CaptionsContent::Raw(_) => None,
        }
    }

    /// Every `w:caption`, in document order.
    pub fn labels(&self) -> impl Iterator<Item = &CaptionLabel> {
        self.content.iter().filter_map(|item| match item {
            CaptionsContent::Label(value) => Some(value),
            _ => None,
        })
    }

    /// Appends a new `w:caption` label at its rank (always before `w:autoCaptions`, after every
    /// existing label).
    pub fn add_label(&mut self, value: CaptionLabel) {
        let mut at = self.content.len();
        for (index, existing) in self.content.iter().enumerate() {
            if Self::rank(existing) == Some(1) {
                at = index;
                break;
            }
        }
        self.content.insert(at, CaptionsContent::Label(value));
        self.empty = false;
    }

    /// `w:autoCaptions`, if stated.
    #[must_use]
    pub fn auto_captions(&self) -> Option<&AutoCaptionsSetting> {
        self.content.iter().find_map(|item| match item {
            CaptionsContent::AutoCaptions(value) => Some(value),
            _ => None,
        })
    }

    /// Sets (or, with `None`, removes) `w:autoCaptions`.
    pub fn set_auto_captions(&mut self, value: Option<AutoCaptionsSetting>) {
        self.content
            .retain(|item| !matches!(item, CaptionsContent::AutoCaptions(_)));
        if let Some(value) = value {
            self.content.push(CaptionsContent::AutoCaptions(value));
        }
        self.empty = false;
    }
}
