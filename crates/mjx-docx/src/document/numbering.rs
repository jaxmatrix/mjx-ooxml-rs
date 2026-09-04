//! `word/numbering.xml` (`CT_Numbering`, the `w:numbering` root) — abstract numbering definitions,
//! numbering instances, level overrides, picture bullets, and the two-hop resolution from a
//! paragraph's `w:numPr` to the `w:lvl` it actually uses.
//!
//! # Two-level indirection, and why a naive reader gets it wrong
//!
//! A paragraph's [`super::paragraph_properties::NumberingProperties`] (`w:numPr`, MJXOFF-96) carries
//! an `ilvl` and a `numId` — neither means anything on its own. `numId` names a [`NumberingInstance`]
//! (`w:num`); that instance names an [`AbstractNumbering`] (`w:abstractNum`) via its own
//! `abstractNumId`; the abstract definition holds up to nine [`NumberingLevel`]s (`w:lvl`), one per
//! `ilvl`. **`numId` is a lookup key into `w:num`, never an index into `w:abstractNum` and never an
//! index into the numbering part's own child order** — two different instances may name the *same*
//! abstract definition, `numId` values need not be contiguous or ascending, and a document may define
//! abstract numbering definitions its `w:num` never uses at all. [`NumberingIndex`] is built once
//! (from a `&`[`Numbering`] snapshot) and indexes both hops by their real keys, never by position —
//! the same design [`super::styles::StyleIndex`] (MJXOFF-101) already uses for `styleId`, for the same
//! reason: this lookup runs once per paragraph in MJXOFF-106's ladder, so it must not rescan.
//!
//! On top of the two hops, `w:num` may carry a `w:lvlOverride` for a given `ilvl`: a `w:startOverride`
//! (changing only where that level starts counting, for *this instance alone*) and/or a whole
//! replacement `w:lvl` (replacing every property of that level, again for this instance alone — the
//! sibling instance sharing the same abstract definition is untouched either way). [`NumberingIndex::resolve`]
//! applies this layer; see its own doc comment and `tests/numbering.rs` for the fixture this project
//! authored specifically because no committed corpus fixture carries `word/numbering.xml` at all (see
//! "No fixture in the corpus" below) — two `w:num` instances sharing one `w:abstractNum`, with
//! deliberately non-contiguous, non-ascending `numId`s, where only one carries a `w:startOverride`.
//!
//! # `numId = 0` is not a lookup failure
//!
//! `w:numPr/w:numId="0"` is Word's own reserved sentinel for "this paragraph carries no numbering" —
//! not a reference to a `w:num` that happens not to exist. [`NumberingIndex::resolve`] and
//! [`crate::Document::resolve_numbering`] both special-case it first, returning
//! [`NumberingLookup::None`], before any lookup runs. A `numId` that is genuinely absent from the part
//! (any other unmatched value) is [`crate::DocxError::UnknownNumberingId`] — a defect to report, never
//! silently treated the same as "no numbering", and never a panic.
//!
//! # Style-linked numbering: the seam between two parts
//!
//! `w:abstractNum/w:numStyleLink` delegates an abstract definition's own levels entirely: instead of
//! reading this definition's own `w:lvl` children (typically absent when `numStyleLink` is present),
//! resolution must find the numbering-type style (`w:style/@type="numbering"`) it names in
//! `word/styles.xml`, read *that style's own* `w:pPr/w:numPr/w:numId`, and resolve **that** `numId`
//! instead — a second, independent redirect through [`super::styles::StyleIndex`] (MJXOFF-101). A
//! resolver that reads the numStyleLink-carrying definition's own (typically empty) `w:lvl` list
//! silently returns nothing, or the wrong list, rather than following the redirect — the trap this
//! child's own ticket names explicitly. [`crate::Document::resolve_numbering`] is where this redirect
//! lives, not [`NumberingIndex`] itself: each redirect hop crosses from `numbering.xml`'s own parsed
//! tree to `styles.xml`'s (two independent [`mjx_ooxml_core::Interner`]s, since each OPC part is
//! parsed on its own — see [`mjx_opc::Package::part_tree`]'s own doc comment), so the chain is walked
//! at the [`crate::Document`] level, one part parse at a time, never holding two parts' trees open at
//! once. The chain is bounded ([`MAX_NUM_STYLE_LINK_DEPTH`]), exactly as
//! [`super::styles::MAX_BASED_ON_CHAIN_DEPTH`] bounds `w:basedOn`, and for the same reason: hitting the
//! bound is [`crate::DocxError::NumberingStyleLinkTooDeep`], never a silently truncated or infinite
//! walk. `w:styleLink` (the reverse annotation — "this abstract definition backs numbering style X")
//! is read structurally ([`AbstractNumbering::style_link`]) but not actively resolved: it is
//! informational metadata Word's UI uses to group a numbering style with its backing definition, not
//! something a caller follows outward the way `numStyleLink` must be.
//!
//! # This module does not compute a paragraph's displayed number
//!
//! [`NumberingIndex::resolve`] and [`crate::Document::resolve_numbering`] answer "which level
//! properties and which starting value apply to this paragraph" — they deliberately stop there. Turning
//! that into the actual displayed number ("1.2.3", "iv", a bullet glyph) requires walking every
//! *preceding* paragraph in the same list to count how many times each level has been used, honouring
//! [`NumberingLevel::restart_after_level`] (a level resets when a lower-numbered level advances),
//! restart on entering a higher-numbered level implicitly, and continuation of a list's count across a
//! section break — none of which this module models. Shipping a counter that only handles a flat,
//! single-level list without any of that would be actively misleading rather than merely incomplete;
//! this module does not attempt it. Rendering a list's displayed text remains explicitly out of scope
//! (this child's own ticket), and MJXOFF-106's combined `docDefaults` → style → numbering → direct
//! effective-properties ladder is the deliberately separate consumer this resolver is built for.
//!
//! # Picture bullets
//!
//! [`NumberingPictureBullet`] (`w:numPicBullet`) models the reference (`numPicBulletId`, matched
//! against a level's own [`NumberingLevel::picture_bullet_id`]) and preserves whichever payload a real
//! file carries — `w:pict` (legacy VML, the overwhelmingly common case in practice) or `w:drawing`
//! (DrawingML) — as opaque [`super::body::Unmodeled`], the same treatment `body.rs` already gives both
//! elements wherever they appear inline in run content. A typed VML model is MJXOFF-113's; a typed
//! DrawingML-in-Word model is MJXOFF-131's. Round-tripping and reading which payload kind is present
//! ([`NumberingPictureBullet::picture`] / [`NumberingPictureBullet::drawing`]) work today; authoring a
//! *new* picture bullet's actual VML/DrawingML payload from scratch does not — this module gives no
//! constructor for one, deliberately, rather than a constructor nobody could use correctly without
//! MJXOFF-113/MJXOFF-131 already existing.
//!
//! # `CT_Lvl`'s `pPr`/`rPr` are `CT_PPrGeneral`/`CT_RPr`, confirmed directly against `wml.xsd`
//!
//! `w:lvl/w:pPr` is `CT_PPrGeneral` — the exact same complex type `w:style/w:pPr` and
//! `w:docDefaults/w:pPrDefault/w:pPr` already are (MJXOFF-101) — **not** a live paragraph's own
//! `CT_PPr`. [`NumberingLevel::paragraph_properties`] reuses
//! [`super::styles::StyleParagraphProperties`] directly rather than restating it, for the identical
//! reason MJXOFF-101's own module doc gives: a numbering level's own paragraph properties may not
//! legally carry a pilcrow's run properties or a section break. `w:lvl/w:rPr` is plain `CT_RPr`;
//! [`NumberingLevel::run_properties`] reuses [`super::run_properties::RunProperties`] directly.
//!
//! # `CT_NumRestart` is not this module's — verified against `wml.xsd`
//!
//! `wml.xsd` has exactly one use of `CT_NumRestart`: the `numRestart` element of
//! `EG_FtnEdnNumProps`, referenced only by `CT_FtnProps`/`CT_EdnProps` — footnote/endnote numbering
//! restart (MJXOFF-124's scope), unreachable from `CT_Numbering`. A numbering *level*'s own restart is
//! [`NumberingLevel::restart_after_level`] (`w:lvlRestart`, a plain `CT_DecimalNumber`). Likewise,
//! `CT_TrackChangeNumbering` (the type this child's own ticket names as in-scope) has exactly two uses
//! in `wml.xsd`: `CT_NumPr`'s own `numberingChange` (already modelled as opaque in
//! `paragraph_properties.rs`, MJXOFF-96) and `CT_FldChar`'s `numberingChange` — neither reachable from
//! `CT_Numbering` either. Both corrections are checked directly against the schema, not merely assumed
//! from the ticket's own "Types in scope" bullet.
//!
//! # No fixture in the corpus contains `word/numbering.xml`
//!
//! All five committed Word fixtures predate this child and carry no numbering part at all — checked
//! directly. `tests/fixtures/numbering_definitions.docx`, authored for this child (see
//! `tests/numbering.rs`), is the only committed round-trip and schema-validity evidence for
//! `word/numbering.xml`; it is also where the shared-abstract-definition/`w:startOverride` trap and
//! the style-link redirect are proved against a real container, not only against in-memory values.
//! `tests/fixtures/paragraph_properties.docx` (MJXOFF-96) already carries a real `w:numPr` while
//! having *no* `word/numbering.xml` relationship at all — a genuine dangling numbering reference
//! already in the committed corpus, and the primary evidence (not a synthetic substitute) that a
//! dangling `numId` is a typed error, never a panic.

use std::collections::HashMap;

use mjx_ooxml_core::{
    AttributeError, Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement,
    RawName, RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::{
    ABSTRACT_NUMBERING, NUMBERING, NUMBERING_INSTANCE, NUMBERING_LEVEL, NUMBERING_LEVEL_OVERRIDE,
};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    DecimalNumber, EightDigitHexadecimalNumber, Justification,
    MultiLevelType as MultiLevelTypeValue, NumberFormat,
    NumberingLevelSuffix as NumberingLevelSuffixValue,
};

use super::body::{wml_name, Unmodeled};
use super::paragraph_properties::{DecimalNumberValue, ParagraphAlignment, ParagraphStyle};
use super::property_macros::{decimal_number_property, toggle_property, value_property};
use super::run_properties::{RunProperties, SignedTwips, Toggle, Twips};
use super::styles::{LongHex, StyleParagraphProperties, StyleString};

use crate::error::DocxError;

// -------------------------------------------------------------------------------------------
// CT_LongHexNumber, reused for w:nsid/w:tmpl the same way StyleString is reused for
// w:name/w:aliases/w:basedOn/w:next/w:link: one wire shape, local-name-parameterized.
// -------------------------------------------------------------------------------------------

/// `CT_LongHexNumber` — a required eight-hex-digit `val`. Reused for `w:nsid` ("Numbering Set
/// Identifier", §17.9.5) and `w:tmpl` ("Original Numbering Template Reference", §17.9.24) — both an
/// abstract numbering definition's own diagnostic hex ids, not resolved against anything, kept for
/// fidelity. The same complex type as [`super::styles::RevisionSaveId`] (`w:rsid`) uses, given its own
/// type here rather than reused: `RevisionSaveId`'s own name is specific to revision-save ids, and
/// `w:nsid`/`w:tmpl` are not one.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = LongHex, accessor = value, required))]
pub struct HexIdentifier {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl HexIdentifier {
    /// Builds a new `local` element (`"nsid"` or `"tmpl"`) of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, value: EightDigitHexadecimalNumber) -> Self {
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

impl FromXml for HexIdentifier {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for HexIdentifier {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_MultiLevelType (w:multiLevelType) — whether an abstract definition is single-level,
// multilevel, or a "hybrid" (Word's own auto-generated multilevel list) definition.
// -------------------------------------------------------------------------------------------

/// `CT_MultiLevelType` (`w:multiLevelType`, "Numbering Definition Type", §17.9.10) — whether an
/// abstract numbering definition is a single level, a genuine multilevel list, or Word's own
/// auto-generated ("hybrid") multilevel list.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<MultiLevelTypeValue>, accessor = kind, required))]
pub struct MultiLevelKind {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl MultiLevelKind {
    /// Builds a new `w:multiLevelType` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: MultiLevelTypeValue) -> Self {
        let mut item = Self {
            name: wml_name(interner, "multiLevelType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_kind(interner, kind);
        item
    }
}

impl FromXml for MultiLevelKind {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for MultiLevelKind {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_LevelSuffix (w:suff) — what follows the level's number: a tab, a space, or nothing.
// -------------------------------------------------------------------------------------------

/// `CT_LevelSuffix` (`w:suff`, "Content Between Numbering Symbol and Paragraph Text", §17.9.20) —
/// what separates a level's rendered number from the paragraph text that follows it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<NumberingLevelSuffixValue>, accessor = suffix, required))]
pub struct LevelSuffix {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LevelSuffix {
    /// Builds a new `w:suff` of `suffix`.
    #[must_use]
    pub fn new(interner: &mut Interner, suffix: NumberingLevelSuffixValue) -> Self {
        let mut item = Self {
            name: wml_name(interner, "suff"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_suffix(interner, suffix);
        item
    }
}

impl FromXml for LevelSuffix {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LevelSuffix {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_NumFmt (w:numFmt) — the level's number format, plus the custom-format string ST_NumberFormat's
// own "custom" value refers to.
// -------------------------------------------------------------------------------------------

/// `CT_NumFmt` (`w:numFmt`, "Numbering Format", §17.9.16) — a level's number format
/// ([`NumberFormat`]'s 63 values — generated, consumed here, never hand-written), plus the optional
/// custom format string `val="custom"` refers to.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<NumberFormat>, accessor = format, required))]
#[xml(attribute(local = "format", prefix = "w", codec = TextCodec, accessor = custom_format))]
pub struct LevelNumberFormat {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LevelNumberFormat {
    /// Builds a new `w:numFmt` of `format`, with no custom format string yet.
    #[must_use]
    pub fn new(interner: &mut Interner, format: NumberFormat) -> Self {
        let mut item = Self {
            name: wml_name(interner, "numFmt"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_format(interner, format);
        item
    }
}

impl FromXml for LevelNumberFormat {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LevelNumberFormat {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_LevelText (w:lvlText) — the level's number template, e.g. "%1." or "%1.%2)" — plus the parsed
// placeholder grammar the ticket asks this type to expose alongside the raw string.
// -------------------------------------------------------------------------------------------

/// One parsed piece of a [`LevelTextTemplate`]'s `%1`-`%9` placeholder grammar — see
/// [`LevelTextTemplate::segments`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelTextSegment {
    /// Literal text, copied through unchanged (including an escaped `%%`, which contributes one
    /// literal `%`).
    Literal(String),
    /// `%1`-`%9` — substitute the counter for the named level (1-based, matching the wire spelling;
    /// level 1 is `ilvl = 0`).
    Level(u8),
}

/// `CT_LevelText` (`w:lvlText`, "Numbering Level Text", §17.9.9) — the level's number template. A
/// multi-level label like `"%1.%2)"` and a bullet glyph like `"•"` are both legal `val` strings; use
/// [`LevelTextTemplate::segments`] to tell them apart without hand-rolling the `%`-placeholder
/// grammar. [`LevelTextTemplate::raw`] always returns the wire string exactly as written, regardless
/// of what [`segments`](Self::segments) parses out of it — nothing here ever rewrites `val`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = TextCodec, accessor = raw))]
#[xml(attribute(local = "null", prefix = "w", codec = OnOff, accessor = is_null))]
pub struct LevelTextTemplate {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LevelTextTemplate {
    /// Builds a new `w:lvlText` with `val` set to `raw`, exactly as given (no placeholder parsing or
    /// validation on the way in — this constructor never rejects untrusted-looking input, matching
    /// the rest of this crate's read path).
    #[must_use]
    pub fn new(interner: &mut Interner, raw: &str) -> Self {
        let mut item = Self {
            name: wml_name(interner, "lvlText"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_raw(interner, Some(raw));
        item
    }

    /// Parses [`LevelTextTemplate::raw`]'s `%1`-`%9` placeholder grammar: `%` followed by a digit
    /// `1`-`9` becomes [`LevelTextSegment::Level`]; `%%` is the escape for one literal `%`
    /// ([`LevelTextSegment::Literal`]); any other character (including a `%` followed by anything
    /// else, or a trailing lone `%`) is copied through literally — this never rejects a malformed
    /// template, it just has nothing special to do with it. Adjacent literal characters are merged
    /// into one [`LevelTextSegment::Literal`]. Returns `None` (never `Some(vec![])`) exactly when
    /// [`LevelTextTemplate::raw`] itself returns `None` — `val` is `use="optional"`.
    ///
    /// ```
    /// # fn main() -> Result<(), mjx_docx::DocxError> {
    /// use mjx_ooxml_core::Interner;
    /// use mjx_docx::{LevelTextSegment, LevelTextTemplate};
    ///
    /// let mut interner = Interner::default();
    /// let template = LevelTextTemplate::new(&mut interner, "%1.%2) 100%% done");
    /// assert_eq!(
    ///     template.segments(&interner).expect("valid w:val"),
    ///     Some(vec![
    ///         LevelTextSegment::Level(1),
    ///         LevelTextSegment::Literal(".".to_owned()),
    ///         LevelTextSegment::Level(2),
    ///         LevelTextSegment::Literal(") 100% done".to_owned()),
    ///     ])
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`AttributeError`] if `w:val` is present but not valid text (untrusted-input
    /// decode failure — see [`LevelTextTemplate::raw`]).
    pub fn segments(
        &self,
        interner: &Interner,
    ) -> Result<Option<Vec<LevelTextSegment>>, AttributeError> {
        let Some(raw) = self.raw(interner)? else {
            return Ok(None);
        };
        let mut segments = Vec::new();
        let mut literal = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '%' {
                literal.push(ch);
                continue;
            }
            match chars.peek() {
                Some(&next) if next.is_ascii_digit() && next != '0' => {
                    if !literal.is_empty() {
                        segments.push(LevelTextSegment::Literal(std::mem::take(&mut literal)));
                    }
                    // Safe: `next` is one of the ASCII digits '1'..='9', so this is 1..=9.
                    #[allow(clippy::cast_possible_truncation)]
                    let level = (next as u32 - '0' as u32) as u8;
                    segments.push(LevelTextSegment::Level(level));
                    chars.next();
                }
                Some('%') => {
                    literal.push('%');
                    chars.next();
                }
                _ => literal.push('%'),
            }
        }
        if !literal.is_empty() {
            segments.push(LevelTextSegment::Literal(literal));
        }
        Ok(Some(segments))
    }
}

impl FromXml for LevelTextTemplate {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LevelTextTemplate {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_LvlLegacy (w:legacy) — three attributes describing this level's legacy (pre-Word-97-ish)
// spacing behaviour; no children.
// -------------------------------------------------------------------------------------------

/// `CT_LvlLegacy` (`w:legacy`, "Legacy Numbering Level Properties Data", §17.9.6) — legacy indentation
/// behaviour a level can carry for backward compatibility with older numbering engines.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "legacy", prefix = "w", codec = OnOff, accessor = is_legacy))]
#[xml(attribute(local = "legacySpace", prefix = "w", codec = Twips, accessor = legacy_space))]
#[xml(attribute(local = "legacyIndent", prefix = "w", codec = SignedTwips, accessor = legacy_indent))]
pub struct LevelLegacyFormatting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LevelLegacyFormatting {
    /// Builds a new, empty `w:legacy` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "legacy"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for LevelLegacyFormatting {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LevelLegacyFormatting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_NumPicBullet (w:numPicBullet) — a choice of pict/drawing, preserved opaque; see the module's
// own doc comment for the MJXOFF-113/MJXOFF-131 boundary.
// -------------------------------------------------------------------------------------------

/// One child of a [`NumberingPictureBullet`]: `CT_NumPicBullet`'s content is a choice of `pict` or
/// `drawing`, each preserved opaque — see the module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberingPictureBulletContent {
    /// `w:pict` (`CT_Picture`) — legacy VML. MJXOFF-113 owns a typed model.
    Picture(Unmodeled),
    /// `w:drawing` (`CT_Drawing`) — DrawingML. MJXOFF-131 owns a typed model.
    Drawing(Unmodeled),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_NumPicBullet` (`w:numPicBullet`, "Picture Numbering Symbol Definition", §17.9.15) — a picture
/// used as a level's numbering symbol, referenced by [`NumberingLevel::picture_bullet_id`].
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "numPicBulletId", prefix = "w", codec = Number<DecimalNumber>, accessor = picture_bullet_id, required))]
pub struct NumberingPictureBullet {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pict", variant = Picture, ty = Unmodeled),
        child(local = "drawing", variant = Drawing, ty = Unmodeled)
    )]
    content: Vec<NumberingPictureBulletContent>,
}

impl NumberingPictureBullet {
    /// This picture bullet's own `w:pict` (legacy VML) payload, or `None` if it carries `w:drawing`
    /// instead (or, on a non-conformant file, neither).
    #[must_use]
    pub fn picture(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            NumberingPictureBulletContent::Picture(value) => Some(value),
            _ => None,
        })
    }

    /// This picture bullet's own `w:drawing` (DrawingML) payload, or `None` if it carries `w:pict`
    /// instead.
    #[must_use]
    pub fn drawing(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            NumberingPictureBulletContent::Drawing(value) => Some(value),
            _ => None,
        })
    }
}

// -------------------------------------------------------------------------------------------
// CT_Lvl (w:lvl, and the replacement level a w:lvlOverride may carry) — the full per-level surface.
// -------------------------------------------------------------------------------------------

/// One ordered child of a [`NumberingLevel`]: `CT_Lvl`'s sequence, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberingLevelContent {
    /// `w:start` (`CT_DecimalNumber`) — the value this level starts counting from.
    Start(DecimalNumberValue),
    /// `w:numFmt` — this level's number format.
    Format(LevelNumberFormat),
    /// `w:lvlRestart` (`CT_DecimalNumber`) — the level (1-based) whose advance resets this level's
    /// own count back to its start.
    RestartAfterLevel(DecimalNumberValue),
    /// `w:pStyle` — the paragraph style this level associates with its own list text.
    Style(ParagraphStyle),
    /// `w:isLgl` (`CT_OnOff`) — whether this level renders using legal (all-Arabic) numbering
    /// regardless of its own `w:numFmt`.
    IsLegalNumbering(Toggle),
    /// `w:suff` — what follows this level's rendered number.
    Suffix(LevelSuffix),
    /// `w:lvlText` — this level's number template.
    Text(LevelTextTemplate),
    /// `w:lvlPicBulletId` (`CT_DecimalNumber`) — the `w:numPicBulletId` this level's picture bullet
    /// uses, when [`NumberingLevelContent::Format`]'s own value is
    /// [`NumberFormat::Bullet`](mjx_ooxml_types::wordprocessingml::NumberFormat::Bullet) and the
    /// bullet glyph is a picture rather than text.
    PictureBulletId(DecimalNumberValue),
    /// `w:legacy` — legacy indentation behaviour.
    Legacy(LevelLegacyFormatting),
    /// `w:lvlJc` (`CT_Jc`) — this level's own justification.
    Alignment(ParagraphAlignment),
    /// `w:pPr` (`CT_PPrGeneral`) — this level's own paragraph properties; see the module's own doc
    /// comment for why this is [`StyleParagraphProperties`], not a live paragraph's `CT_PPr`.
    ParagraphProperties(StyleParagraphProperties),
    /// `w:rPr` (`CT_RPr`) — this level's own run properties, applied to its rendered number.
    RunProperties(RunProperties),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Lvl` (`w:lvl`, "Numbering Level Definition", §17.9.7) — one numbering level's own formatting:
/// where it starts, how it is formatted and rendered, and the paragraph/run properties associated
/// with it. Appears both as an abstract definition's own level ([`AbstractNumbering::level`]) and, in
/// full replacement form, inside a `w:lvlOverride` ([`NumberingLevelOverride::replacement_level`]).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "ilvl", prefix = "w", codec = Number<DecimalNumber>, accessor = index, required))]
#[xml(attribute(local = "tplc", prefix = "w", codec = LongHex, accessor = template_id))]
#[xml(attribute(local = "tentative", prefix = "w", codec = OnOff, accessor = is_tentative))]
pub struct NumberingLevel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "start", variant = Start, ty = DecimalNumberValue),
        child(local = "numFmt", variant = Format, ty = LevelNumberFormat),
        child(local = "lvlRestart", variant = RestartAfterLevel, ty = DecimalNumberValue),
        child(local = "pStyle", variant = Style, ty = ParagraphStyle),
        child(local = "isLgl", variant = IsLegalNumbering, ty = Toggle),
        child(local = "suff", variant = Suffix, ty = LevelSuffix),
        child(local = "lvlText", variant = Text, ty = LevelTextTemplate),
        child(local = "lvlPicBulletId", variant = PictureBulletId, ty = DecimalNumberValue),
        child(local = "legacy", variant = Legacy, ty = LevelLegacyFormatting),
        child(local = "lvlJc", variant = Alignment, ty = ParagraphAlignment),
        child(local = "pPr", variant = ParagraphProperties, ty = StyleParagraphProperties),
        child(local = "rPr", variant = RunProperties, ty = RunProperties)
    )]
    content: Vec<NumberingLevelContent>,
}

impl NumberingLevel {
    /// Builds a new `w:lvl` at `ilvl`, with no properties of its own yet.
    #[must_use]
    pub fn new(interner: &mut Interner, ilvl: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, "lvl"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_index(interner, ilvl);
        value
    }

    fn rank(item: &NumberingLevelContent) -> Option<u16> {
        let local = match item {
            NumberingLevelContent::Start(_) => "start",
            NumberingLevelContent::Format(_) => "numFmt",
            NumberingLevelContent::RestartAfterLevel(_) => "lvlRestart",
            NumberingLevelContent::Style(_) => "pStyle",
            NumberingLevelContent::IsLegalNumbering(_) => "isLgl",
            NumberingLevelContent::Suffix(_) => "suff",
            NumberingLevelContent::Text(_) => "lvlText",
            NumberingLevelContent::PictureBulletId(_) => "lvlPicBulletId",
            NumberingLevelContent::Legacy(_) => "legacy",
            NumberingLevelContent::Alignment(_) => "lvlJc",
            NumberingLevelContent::ParagraphProperties(_) => "pPr",
            NumberingLevelContent::RunProperties(_) => "rPr",
            NumberingLevelContent::Raw(_) => return None,
        };
        NUMBERING_LEVEL.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&NumberingLevelContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: NumberingLevelContent) {
        let at = NUMBERING_LEVEL.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&NumberingLevelContent) -> bool,
        value: Option<NumberingLevelContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    decimal_number_property!(
        NumberingLevelContent,
        start,
        set_start,
        Start,
        "start",
        "`w:start` — the value this level starts counting from."
    );
    decimal_number_property!(
        NumberingLevelContent,
        restart_after_level,
        set_restart_after_level,
        RestartAfterLevel,
        "lvlRestart",
        "`w:lvlRestart` — the level (1-based) whose advance resets this level's own count."
    );
    decimal_number_property!(
        NumberingLevelContent,
        picture_bullet_id,
        set_picture_bullet_id,
        PictureBulletId,
        "lvlPicBulletId",
        "`w:lvlPicBulletId` — the `w:numPicBulletId` this level's picture bullet uses."
    );

    toggle_property!(
        NumberingLevelContent,
        is_legal_numbering,
        set_is_legal_numbering,
        IsLegalNumbering,
        "isLgl",
        "`w:isLgl` — whether this level renders using legal (all-Arabic) numbering regardless of its \
         own `w:numFmt`."
    );

    value_property!(
        NumberingLevelContent,
        format,
        set_format,
        Format,
        LevelNumberFormat,
        "numFmt",
        "`w:numFmt` — this level's number format."
    );
    value_property!(
        NumberingLevelContent,
        paragraph_style,
        set_paragraph_style,
        Style,
        ParagraphStyle,
        "pStyle",
        "`w:pStyle` — the paragraph style this level associates with its own list text."
    );
    value_property!(
        NumberingLevelContent,
        suffix,
        set_suffix,
        Suffix,
        LevelSuffix,
        "suff",
        "`w:suff` — what follows this level's rendered number."
    );
    value_property!(
        NumberingLevelContent,
        text_template,
        set_text_template,
        Text,
        LevelTextTemplate,
        "lvlText",
        "`w:lvlText` — this level's number template."
    );
    value_property!(
        NumberingLevelContent,
        legacy,
        set_legacy,
        Legacy,
        LevelLegacyFormatting,
        "legacy",
        "`w:legacy` — legacy indentation behaviour."
    );
    value_property!(
        NumberingLevelContent,
        paragraph_properties,
        set_paragraph_properties,
        ParagraphProperties,
        StyleParagraphProperties,
        "pPr",
        "`w:pPr` — this level's own paragraph properties."
    );
    value_property!(
        NumberingLevelContent,
        run_properties,
        set_run_properties,
        RunProperties,
        RunProperties,
        "rPr",
        "`w:rPr` — this level's own run properties, applied to its rendered number."
    );

    /// This level's own `w:lvlJc` (justification), or `None` if absent.
    #[must_use]
    pub fn alignment(&self) -> Option<&ParagraphAlignment> {
        self.content.iter().find_map(|item| match item {
            NumberingLevelContent::Alignment(value) => Some(value),
            _ => None,
        })
    }

    /// Sets `w:lvlJc`: `None` removes it; `Some(value)` builds a fresh [`ParagraphAlignment`] under
    /// the `lvlJc` wire name (not `jc` — a paragraph's own justification and a level's own use the
    /// same `CT_Jc` shape under two different element names) and replaces or inserts it at its
    /// schema rank.
    pub fn set_alignment(&mut self, interner: &mut Interner, value: Option<Justification>) {
        let is_target =
            |item: &NumberingLevelContent| matches!(item, NumberingLevelContent::Alignment(_));
        match value {
            None => self.remove(is_target),
            Some(value) => {
                let element = ParagraphAlignment::new_named(interner, "lvlJc", value);
                self.set(
                    "lvlJc",
                    is_target,
                    Some(NumberingLevelContent::Alignment(element)),
                );
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// CT_NumLvl (w:lvlOverride) — one numbering instance's per-level override: a start override and/or
// a whole replacement level.
// -------------------------------------------------------------------------------------------

/// One ordered child of a [`NumberingLevelOverride`]: `CT_NumLvl`'s sequence is `startOverride?,
/// lvl?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberingLevelOverrideContent {
    /// `w:startOverride` (`CT_DecimalNumber`) — overrides only where this level starts counting, for
    /// this instance alone.
    StartOverride(DecimalNumberValue),
    /// `w:lvl` — a whole replacement level, for this instance alone.
    ReplacementLevel(NumberingLevel),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_NumLvl` (`w:lvlOverride`, "Numbering Level Override Definition", §17.9.8) — one numbering
/// instance's override of a single abstract-definition level, keyed by `ilvl`. See the module's own
/// doc comment for how [`NumberingIndex::resolve`] applies this layer: a `w:startOverride` here
/// changes only the *effective start* for the instance that carries it, never the abstract definition
/// (and so never a sibling instance sharing it), and a [`NumberingLevelOverrideContent::ReplacementLevel`]
/// here replaces every property of the level, not only the start.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "ilvl", prefix = "w", codec = Number<DecimalNumber>, accessor = index, required))]
pub struct NumberingLevelOverride {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "startOverride", variant = StartOverride, ty = DecimalNumberValue),
        child(local = "lvl", variant = ReplacementLevel, ty = NumberingLevel)
    )]
    content: Vec<NumberingLevelOverrideContent>,
}

impl NumberingLevelOverride {
    /// Builds a new `w:lvlOverride` at `ilvl`, with neither a start override nor a replacement level
    /// yet.
    #[must_use]
    pub fn new(interner: &mut Interner, ilvl: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, "lvlOverride"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_index(interner, ilvl);
        value
    }

    fn rank(item: &NumberingLevelOverrideContent) -> Option<u16> {
        let local = match item {
            NumberingLevelOverrideContent::StartOverride(_) => "startOverride",
            NumberingLevelOverrideContent::ReplacementLevel(_) => "lvl",
            NumberingLevelOverrideContent::Raw(_) => return None,
        };
        NUMBERING_LEVEL_OVERRIDE.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&NumberingLevelOverrideContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: NumberingLevelOverrideContent) {
        let at = NUMBERING_LEVEL_OVERRIDE
            .insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&NumberingLevelOverrideContent) -> bool,
        value: Option<NumberingLevelOverrideContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    decimal_number_property!(
        NumberingLevelOverrideContent,
        start_override,
        set_start_override,
        StartOverride,
        "startOverride",
        "`w:startOverride` — overrides only where this level starts counting, for this instance \
         alone."
    );

    value_property!(
        NumberingLevelOverrideContent,
        replacement_level,
        set_replacement_level,
        ReplacementLevel,
        NumberingLevel,
        "lvl",
        "`w:lvl` — a whole replacement level, for this instance alone."
    );
}

// -------------------------------------------------------------------------------------------
// CT_Num (w:num) — one numbering instance: which abstract definition it uses, plus its own
// per-level overrides.
// -------------------------------------------------------------------------------------------

/// One ordered child of a [`NumberingInstance`]: `CT_Num`'s sequence is `abstractNumId,
/// lvlOverride{0,9}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberingInstanceContent {
    /// `w:abstractNumId` (`CT_DecimalNumber`) — the abstract definition this instance uses.
    /// `minOccurs="1"` in the schema, but read defensively (`Option`, never a panic) — a
    /// non-conformant file omitting it is [`crate::DocxError::MissingAbstractNumberingReference`]
    /// when resolved, not rejected on read.
    AbstractNumberingId(DecimalNumberValue),
    /// `w:lvlOverride` (repeatable, up to nine) — a per-level override for this instance.
    LevelOverride(NumberingLevelOverride),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Num` (`w:num`, "Numbering Definition Instance", §17.9.14) — one concrete numbering definition:
/// the abstract definition it uses (via [`NumberingInstance::abstract_numbering_id`]), plus any
/// per-level overrides. Two instances may name the *same* abstract definition — see the module's own
/// doc comment for why `numId` is a lookup key, never an index.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "numId", prefix = "w", codec = Number<DecimalNumber>, accessor = numbering_id, required))]
pub struct NumberingInstance {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "abstractNumId", variant = AbstractNumberingId, ty = DecimalNumberValue),
        child(local = "lvlOverride", variant = LevelOverride, ty = NumberingLevelOverride)
    )]
    content: Vec<NumberingInstanceContent>,
}

impl NumberingInstance {
    /// Builds a new `w:num` of `num_id`, naming `abstract_numbering_id` as its abstract definition
    /// (`CT_Num`'s own `abstractNumId` is `minOccurs="1"`, so this constructor always writes one),
    /// with no level overrides yet.
    #[must_use]
    pub fn new(interner: &mut Interner, num_id: i64, abstract_numbering_id: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, "num"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_numbering_id(interner, num_id);
        value.set_abstract_numbering_id(interner, Some(abstract_numbering_id));
        value
    }

    fn rank(item: &NumberingInstanceContent) -> Option<u16> {
        let local = match item {
            NumberingInstanceContent::AbstractNumberingId(_) => "abstractNumId",
            NumberingInstanceContent::LevelOverride(_) => "lvlOverride",
            NumberingInstanceContent::Raw(_) => return None,
        };
        NUMBERING_INSTANCE.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&NumberingInstanceContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: NumberingInstanceContent) {
        let at =
            NUMBERING_INSTANCE.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&NumberingInstanceContent) -> bool,
        value: Option<NumberingInstanceContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    decimal_number_property!(
        NumberingInstanceContent,
        abstract_numbering_id,
        set_abstract_numbering_id,
        AbstractNumberingId,
        "abstractNumId",
        "`w:abstractNumId` — the abstract definition this instance uses."
    );

    /// Every `w:lvlOverride` this instance carries, in document order.
    pub fn level_overrides(&self) -> impl Iterator<Item = &NumberingLevelOverride> {
        self.content.iter().filter_map(|item| match item {
            NumberingInstanceContent::LevelOverride(value) => Some(value),
            _ => None,
        })
    }

    /// The `w:lvlOverride` for `ilvl`, or `None` if this instance carries none for that level.
    ///
    /// # Errors
    /// Returns [`AttributeError`] if an override's own `ilvl` attribute is present but malformed.
    pub fn level_override(
        &self,
        ilvl: i64,
        interner: &Interner,
    ) -> Result<Option<&NumberingLevelOverride>, AttributeError> {
        for item in self.level_overrides() {
            if item.index(interner)? == ilvl {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }

    /// Appends `override_` as this instance's new last `w:lvlOverride`.
    pub fn push_level_override(&mut self, override_: NumberingLevelOverride) {
        self.insert(
            "lvlOverride",
            NumberingInstanceContent::LevelOverride(override_),
        );
    }
}

// -------------------------------------------------------------------------------------------
// CT_AbstractNum (w:abstractNum) — one abstract numbering definition: identity, style links, and
// up to nine levels.
// -------------------------------------------------------------------------------------------

/// One ordered child of an [`AbstractNumbering`]: `CT_AbstractNum`'s sequence, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractNumberingContent {
    /// `w:nsid` (`CT_LongHexNumber`) — a diagnostic identifier, not resolved against anything.
    NumberingSetId(HexIdentifier),
    /// `w:multiLevelType` — whether this is a single level, multilevel, or hybrid definition.
    MultiLevelType(MultiLevelKind),
    /// `w:tmpl` (`CT_LongHexNumber`) — the gallery template this definition originated from.
    Template(HexIdentifier),
    /// `w:name` (`CT_String`) — this definition's own display name.
    Name(StyleString),
    /// `w:styleLink` (`CT_String`) — informational: the numbering style this definition backs. See
    /// the module's own doc comment for why this is read, not actively resolved.
    StyleLink(StyleString),
    /// `w:numStyleLink` (`CT_String`) — the numbering style this definition delegates *to*. See the
    /// module's own doc comment and [`crate::Document::resolve_numbering`] for the redirect this
    /// implies.
    NumberingStyleLink(StyleString),
    /// `w:lvl` (repeatable, up to nine) — one of this definition's own levels.
    Level(NumberingLevel),
    /// Any other child — an unknown element (`w15:` extensions land here) — preserved verbatim.
    Raw(RawNode),
}

/// `CT_AbstractNum` (`w:abstractNum`, "Abstract Numbering Definition", §17.9.1) — one numbering
/// definition's actual formatting: up to nine [`NumberingLevel`]s, named by `abstractNumId` and
/// referenced by one or more [`NumberingInstance`]s (via [`NumberingInstance::abstract_numbering_id`]).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "abstractNumId", prefix = "w", codec = Number<DecimalNumber>, accessor = abstract_numbering_id, required))]
pub struct AbstractNumbering {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "nsid", variant = NumberingSetId, ty = HexIdentifier),
        child(local = "multiLevelType", variant = MultiLevelType, ty = MultiLevelKind),
        child(local = "tmpl", variant = Template, ty = HexIdentifier),
        child(local = "name", variant = Name, ty = StyleString),
        child(local = "styleLink", variant = StyleLink, ty = StyleString),
        child(local = "numStyleLink", variant = NumberingStyleLink, ty = StyleString),
        child(local = "lvl", variant = Level, ty = NumberingLevel)
    )]
    content: Vec<AbstractNumberingContent>,
}

impl AbstractNumbering {
    /// Builds a new `w:abstractNum` of `abstract_numbering_id`, with no levels or metadata yet.
    #[must_use]
    pub fn new(interner: &mut Interner, abstract_numbering_id: i64) -> Self {
        let mut value = Self {
            name: wml_name(interner, "abstractNum"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_abstract_numbering_id(interner, abstract_numbering_id);
        value
    }

    fn rank(item: &AbstractNumberingContent) -> Option<u16> {
        let local = match item {
            AbstractNumberingContent::NumberingSetId(_) => "nsid",
            AbstractNumberingContent::MultiLevelType(_) => "multiLevelType",
            AbstractNumberingContent::Template(_) => "tmpl",
            AbstractNumberingContent::Name(_) => "name",
            AbstractNumberingContent::StyleLink(_) => "styleLink",
            AbstractNumberingContent::NumberingStyleLink(_) => "numStyleLink",
            AbstractNumberingContent::Level(_) => "lvl",
            AbstractNumberingContent::Raw(_) => return None,
        };
        ABSTRACT_NUMBERING.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&AbstractNumberingContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: AbstractNumberingContent) {
        let at =
            ABSTRACT_NUMBERING.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&AbstractNumberingContent) -> bool,
        value: Option<AbstractNumberingContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    value_property!(
        AbstractNumberingContent,
        numbering_set_id,
        set_numbering_set_id,
        NumberingSetId,
        HexIdentifier,
        "nsid",
        "`w:nsid` — a diagnostic identifier, not resolved against anything."
    );
    value_property!(
        AbstractNumberingContent,
        multi_level_kind,
        set_multi_level_kind,
        MultiLevelType,
        MultiLevelKind,
        "multiLevelType",
        "`w:multiLevelType` — whether this is a single level, multilevel, or hybrid definition."
    );
    value_property!(
        AbstractNumberingContent,
        template,
        set_template,
        Template,
        HexIdentifier,
        "tmpl",
        "`w:tmpl` — the gallery template this definition originated from."
    );
    value_property!(
        AbstractNumberingContent,
        name,
        set_name,
        Name,
        StyleString,
        "name",
        "`w:name` — this definition's own display name."
    );
    value_property!(
        AbstractNumberingContent,
        style_link,
        set_style_link,
        StyleLink,
        StyleString,
        "styleLink",
        "`w:styleLink` — informational: the numbering style this definition backs. See the module's \
         own doc comment for why this is read, not actively resolved."
    );
    value_property!(
        AbstractNumberingContent,
        numbering_style_link,
        set_numbering_style_link,
        NumberingStyleLink,
        StyleString,
        "numStyleLink",
        "`w:numStyleLink` — the numbering style this definition delegates *to*. See the module's own \
         doc comment and [`crate::Document::resolve_numbering`] for the redirect this implies."
    );

    /// Every `w:lvl` this definition carries, in document order.
    pub fn levels(&self) -> impl Iterator<Item = &NumberingLevel> {
        self.content.iter().filter_map(|item| match item {
            AbstractNumberingContent::Level(value) => Some(value),
            _ => None,
        })
    }

    /// The `w:lvl` at `ilvl`, or `None` if this definition carries none for that level (legal — a
    /// numStyleLink-linked definition typically carries no levels of its own at all).
    ///
    /// # Errors
    /// Returns [`AttributeError`] if a level's own `ilvl` attribute is present but malformed.
    pub fn level(
        &self,
        ilvl: i64,
        interner: &Interner,
    ) -> Result<Option<&NumberingLevel>, AttributeError> {
        for item in self.levels() {
            if item.index(interner)? == ilvl {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }

    /// Appends `level` as this definition's new last `w:lvl`.
    pub fn push_level(&mut self, level: NumberingLevel) {
        self.insert("lvl", AbstractNumberingContent::Level(level));
    }
}

// -------------------------------------------------------------------------------------------
// CT_Numbering (w:numbering) — the numbering definitions part's own root.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`Numbering`]: `CT_Numbering`'s sequence is `numPicBullet*, abstractNum*,
/// num*, numIdMacAtCleanup?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberingContent {
    /// `w:numPicBullet` (repeatable).
    PictureBullet(NumberingPictureBullet),
    /// `w:abstractNum` (repeatable).
    AbstractNumbering(AbstractNumbering),
    /// `w:num` (repeatable).
    Instance(NumberingInstance),
    /// `w:numIdMacAtCleanup` (`CT_DecimalNumber`) — the highest `numId` Word's own cleanup pass
    /// observed; a Word-internal bookkeeping value, preserved for fidelity, not resolved against
    /// anything.
    HighestObservedId(DecimalNumberValue),
    /// Any other child — an unknown element (`w15:` extensions land here) — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Numbering` (`w:numbering`, the `word/numbering.xml` part's own root, §17.9.17) — every
/// picture bullet, abstract numbering definition and numbering instance this document defines. See
/// the module's own doc comment for the two-hop resolution from a paragraph's `w:numPr` to the
/// [`NumberingLevel`] it actually uses.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct Numbering {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "numPicBullet", variant = PictureBullet, ty = NumberingPictureBullet),
        child(local = "abstractNum", variant = AbstractNumbering, ty = AbstractNumbering),
        child(local = "num", variant = Instance, ty = NumberingInstance),
        child(local = "numIdMacAtCleanup", variant = HighestObservedId, ty = DecimalNumberValue)
    )]
    content: Vec<NumberingContent>,
}

impl Numbering {
    /// Builds a new, empty `w:numbering` — no picture bullets, abstract definitions or instances.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "numbering"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &NumberingContent) -> Option<u16> {
        let local = match item {
            NumberingContent::PictureBullet(_) => "numPicBullet",
            NumberingContent::AbstractNumbering(_) => "abstractNum",
            NumberingContent::Instance(_) => "num",
            NumberingContent::HighestObservedId(_) => "numIdMacAtCleanup",
            NumberingContent::Raw(_) => return None,
        };
        NUMBERING.rank_of(None, local)
    }

    fn insert(&mut self, local: &str, item: NumberingContent) {
        let at = NUMBERING.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Every `w:numPicBullet` this part carries, in document order.
    pub fn picture_bullets(&self) -> impl Iterator<Item = &NumberingPictureBullet> {
        self.content.iter().filter_map(|item| match item {
            NumberingContent::PictureBullet(value) => Some(value),
            _ => None,
        })
    }

    /// The `w:numPicBullet` whose `numPicBulletId` is `id`, or `None`.
    ///
    /// # Errors
    /// Returns [`AttributeError`] if a picture bullet's own id attribute is present but malformed.
    pub fn picture_bullet(
        &self,
        id: i64,
        interner: &Interner,
    ) -> Result<Option<&NumberingPictureBullet>, AttributeError> {
        for item in self.picture_bullets() {
            if item.picture_bullet_id(interner)? == id {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }

    /// Every `w:abstractNum` this part defines, in document order. Prefer [`NumberingIndex`] for a
    /// lookup by `abstractNumId` — this is a linear scan.
    pub fn abstract_numberings(&self) -> impl Iterator<Item = &AbstractNumbering> {
        self.content.iter().filter_map(|item| match item {
            NumberingContent::AbstractNumbering(value) => Some(value),
            _ => None,
        })
    }

    /// Every `w:num` this part defines, in document order. Prefer [`NumberingIndex`] for a lookup by
    /// `numId` — this is a linear scan.
    pub fn instances(&self) -> impl Iterator<Item = &NumberingInstance> {
        self.content.iter().filter_map(|item| match item {
            NumberingContent::Instance(value) => Some(value),
            _ => None,
        })
    }

    /// How many `w:abstractNum` definitions this part carries.
    #[must_use]
    pub fn abstract_numbering_count(&self) -> usize {
        self.abstract_numberings().count()
    }

    /// How many `w:num` instances this part carries.
    #[must_use]
    pub fn instance_count(&self) -> usize {
        self.instances().count()
    }

    /// Appends `bullet` as this part's new last `w:numPicBullet`.
    pub fn push_picture_bullet(&mut self, bullet: NumberingPictureBullet) {
        self.insert("numPicBullet", NumberingContent::PictureBullet(bullet));
    }

    /// Appends `definition` as this part's new last `w:abstractNum`.
    pub fn push_abstract_numbering(&mut self, definition: AbstractNumbering) {
        self.insert(
            "abstractNum",
            NumberingContent::AbstractNumbering(definition),
        );
    }

    /// Appends `instance` as this part's new last `w:num`.
    pub fn push_instance(&mut self, instance: NumberingInstance) {
        self.insert("num", NumberingContent::Instance(instance));
    }
}

// -------------------------------------------------------------------------------------------
// NumberingIndex — built once from a Numbering snapshot, then reused for every (numId, ilvl)
// resolution a caller needs. See the module's own doc comment for the full rationale.
// -------------------------------------------------------------------------------------------

/// The result of [`NumberingIndex::resolve`] (or [`crate::Document::resolve_numbering`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberingLookup<'a> {
    /// `numId = 0` — this paragraph explicitly carries no numbering. Not a lookup failure; see the
    /// module's own doc comment.
    None,
    /// Both hops resolved; overrides (if any) already applied.
    Resolved(NumberingResolution<'a>),
}

/// Both hops of a numbering resolution, with the `w:lvlOverride` layer already applied. See the
/// module's own doc comment for how [`NumberingResolution::effective_start`] and
/// [`NumberingResolution::level`] are derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberingResolution<'a> {
    instance: &'a NumberingInstance,
    abstract_definition: &'a AbstractNumbering,
    level: Option<&'a NumberingLevel>,
    effective_start: Option<i64>,
}

impl<'a> NumberingResolution<'a> {
    /// The `w:num` this resolution started from.
    #[must_use]
    pub fn instance(&self) -> &'a NumberingInstance {
        self.instance
    }

    /// The `w:abstractNum` this resolution's instance uses.
    #[must_use]
    pub fn abstract_definition(&self) -> &'a AbstractNumbering {
        self.abstract_definition
    }

    /// The effective level: the instance's own `w:lvlOverride/w:lvl` for this `ilvl` if it carries
    /// one, else the abstract definition's own `w:lvl` for this `ilvl`, else `None` (legal — an
    /// abstract definition need not define every one of the nine levels, and a `numStyleLink`-linked
    /// definition typically defines none of its own).
    #[must_use]
    pub fn level(&self) -> Option<&'a NumberingLevel> {
        self.level
    }

    /// The effective starting value: the instance's own `w:lvlOverride/w:startOverride` for this
    /// `ilvl` if it carries one (applying **regardless** of whether that same override also carries a
    /// replacement level), else [`NumberingResolution::level`]'s own `w:start`, else `None`.
    #[must_use]
    pub fn effective_start(&self) -> Option<i64> {
        self.effective_start
    }
}

/// The greatest number of `w:numStyleLink` redirect hops [`crate::Document::resolve_numbering`] walks
/// before treating the chain as broken rather than terminating normally — the same bounded-depth
/// cycle-safety design as [`super::styles::MAX_BASED_ON_CHAIN_DEPTH`], scaled down: a numStyleLink
/// redirect is at most one or two hops in every real file (a numbering style pointing at a numbering
/// definition that is not itself style-linked), so this bound is generous relative to any legitimate
/// document while still catching a malformed cycle promptly.
pub const MAX_NUM_STYLE_LINK_DEPTH: usize = 16;

/// A [`Numbering`] part's abstract definitions and instances, indexed by `abstractNumId`/`numId` and
/// built once from a `&`[`Numbering`] snapshot — see the module's own doc comment for why `numId` and
/// `abstractNumId` are lookup keys, never indices, and why this is not cached inside [`Numbering`]
/// itself (mirrors [`super::styles::StyleIndex`]'s own rationale exactly).
#[derive(Debug)]
pub struct NumberingIndex<'a> {
    abstract_entries: Vec<&'a AbstractNumbering>,
    abstract_by_id: HashMap<i64, usize>,
    num_entries: Vec<&'a NumberingInstance>,
    num_by_id: HashMap<i64, usize>,
}

impl<'a> NumberingIndex<'a> {
    /// Builds an index over every abstract definition and instance in `numbering`, resolving each
    /// one's own id once.
    ///
    /// A file naming two abstract definitions with the same `abstractNumId` (or two instances with
    /// the same `numId`) is not rejected here — the first one wins, matching
    /// [`super::styles::StyleIndex::build`]'s own policy for a duplicate `styleId`; later duplicates
    /// are simply unreachable through this index.
    ///
    /// # Errors
    /// Returns [`crate::DocxError`] if any definition's `abstractNumId` or any instance's `numId` is
    /// present but malformed.
    pub fn build(numbering: &'a Numbering, interner: &Interner) -> Result<Self, DocxError> {
        let abstract_entries: Vec<&AbstractNumbering> = numbering.abstract_numberings().collect();
        let mut abstract_by_id = HashMap::with_capacity(abstract_entries.len());
        for (index, item) in abstract_entries.iter().enumerate() {
            let id = item
                .abstract_numbering_id(interner)
                .map_err(FromXmlError::from)?;
            abstract_by_id.entry(id).or_insert(index);
        }

        let num_entries: Vec<&NumberingInstance> = numbering.instances().collect();
        let mut num_by_id = HashMap::with_capacity(num_entries.len());
        for (index, item) in num_entries.iter().enumerate() {
            let id = item.numbering_id(interner).map_err(FromXmlError::from)?;
            num_by_id.entry(id).or_insert(index);
        }

        Ok(Self {
            abstract_entries,
            abstract_by_id,
            num_entries,
            num_by_id,
        })
    }

    /// The abstract definition whose `abstractNumId` is exactly `id`, or `None`.
    #[must_use]
    pub fn abstract_numbering_by_id(&self, id: i64) -> Option<&'a AbstractNumbering> {
        self.abstract_by_id
            .get(&id)
            .map(|&index| self.abstract_entries[index])
    }

    /// The instance whose `numId` is exactly `id`, or `None`.
    #[must_use]
    pub fn numbering_instance_by_id(&self, id: i64) -> Option<&'a NumberingInstance> {
        self.num_by_id
            .get(&id)
            .map(|&index| self.num_entries[index])
    }

    /// Resolves `numbering_id`/`level` through both hops, applying the `w:lvlOverride` layer — see
    /// the module's own doc comment. Does **not** follow a `w:numStyleLink` redirect (that crosses
    /// into `word/styles.xml`, a different OPC part with its own [`Interner`]; see
    /// [`crate::Document::resolve_numbering`], which builds on this method).
    ///
    /// # Errors
    /// Returns [`crate::DocxError::UnknownNumberingId`] if `numbering_id` is not `0` and names no
    /// `w:num` in this part, [`crate::DocxError::MissingAbstractNumberingReference`] if the resolved
    /// `w:num` carries no `w:abstractNumId` at all (legal only for a non-conformant file — never
    /// rejected on read, only here, on resolution), [`crate::DocxError::UnknownAbstractNumberingId`]
    /// if that `abstractNumId` names no `w:abstractNum` in this part, or [`crate::DocxError`] if an
    /// attribute involved is present but malformed.
    pub fn resolve(
        &self,
        numbering_id: i64,
        level: i64,
        interner: &Interner,
    ) -> Result<NumberingLookup<'a>, DocxError> {
        if numbering_id == 0 {
            return Ok(NumberingLookup::None);
        }
        let instance = self
            .numbering_instance_by_id(numbering_id)
            .ok_or(DocxError::UnknownNumberingId(numbering_id))?;
        let abstract_id = instance
            .abstract_numbering_id(interner)
            .map_err(FromXmlError::from)?
            .ok_or(DocxError::MissingAbstractNumberingReference(numbering_id))?;
        let abstract_definition = self
            .abstract_numbering_by_id(abstract_id)
            .ok_or(DocxError::UnknownAbstractNumberingId(abstract_id))?;

        let overriding = instance
            .level_override(level, interner)
            .map_err(FromXmlError::from)?;
        let replacement_level = overriding.and_then(NumberingLevelOverride::replacement_level);
        let effective_level = replacement_level.or(abstract_definition
            .level(level, interner)
            .map_err(FromXmlError::from)?);

        let start_override = match overriding {
            Some(overriding) => overriding
                .start_override(interner)
                .map_err(FromXmlError::from)?,
            None => None,
        };
        let effective_start = match start_override {
            Some(value) => Some(value),
            None => match effective_level {
                Some(effective_level) => effective_level
                    .start(interner)
                    .map_err(FromXmlError::from)?,
                None => None,
            },
        };

        Ok(NumberingLookup::Resolved(NumberingResolution {
            instance,
            abstract_definition,
            level: effective_level,
            effective_start,
        }))
    }
}
