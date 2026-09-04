//! The effective-properties ladder: what a run or paragraph actually renders as, not merely what
//! `word/document.xml` states about it.
//!
//! See [the effective-properties guide](crate::effective_properties) for the full account (the ladder
//! order as verified against ECMA-376 Part 1 prose, the toggle rule, the theme-colour mapping, the
//! cache design, and where this reader stops). This module carries the implementation; the guide
//! carries the explanation a caller should read first.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use mjx_dml::{resolve_color, ColorMap, FontSchemeSlot, ResolvedColor, SchemeColors};
use mjx_ooxml_core::{AttributeError, FromXml, FromXmlError, Interner};
use mjx_ooxml_types::shared::{
    RelativeHorizontalAlignment, RelativeVerticalAlignment, VerticalTextPosition,
};
use mjx_ooxml_types::wordprocessingml::{
    BorderStyle, CombineBrackets, DropCap, EighthPointMeasure, EmphasisMark, FontTypeHint,
    HalfPointMeasure, HeightRule, HexadecimalColor, HighlightColor, HorizontalAnchor,
    Justification, ShadingPattern, SignedHalfPointMeasure, SignedTwipsMeasure, TabStopLeader,
    TabStopType, TextBoxTightWrap, TextEffect as TextEffectKind, TextFlowDirection,
    TextFrameWrapping, TextScale, ThemeColor, ThemeFont, Underline as UnderlineKind,
    VerticalAnchor, VerticalTextAlignment,
};

use crate::address::{BlockPath, RunPath};
use crate::error::DocxError;

use super::numbering::{NumberingLevel, NumberingLookup};
use super::paragraph_properties::{
    ConditionalFormatting, FrameProperties, Indentation, LineSpacing, NumberingProperties,
    ParagraphBorders, ParagraphProperties, Spacing, TabStop, TabStops,
};
use super::run_properties::{
    Border, Color as WmlColor, EastAsianLayout, Fonts, Languages, ManualRunWidth, RunProperties,
    Shading, Underline,
};
use super::styles::{StyleDefinition, StyleIndex, StyleParagraphProperties};
use super::{Document, MainDocument};

// -------------------------------------------------------------------------------------------
// Small interner-free mirrors of the leaf types that carry more than one attribute. Each is
// extracted once, while the winning rung's own interner is in scope, exactly as every other
// effective reader in this workspace extracts before crossing a part boundary — see
// `mjx_pptx::presentation::effective`'s own doc comment for the pattern this one repeats.
// -------------------------------------------------------------------------------------------

/// A resolved colour: either `auto` (let the renderer choose) or a concrete `RRGGBB`, already
/// resolved through any `themeColor` reference. `themeTint`/`themeShade`, when present, are **not**
/// baked in — see [the guide](crate::effective_properties) for why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveColor {
    /// The wire value `"auto"` — the file's own way of leaving the choice to the renderer.
    Auto,
    /// A concrete `RRGGBB` hex value, uppercase, no leading `#`.
    Hex(String),
}

/// A resolved `w:rFonts` — each of the four script slots independently resolved: a literal typeface
/// wins over that slot's own theme reference (`asciiTheme` etc.), which is baked to a concrete font
/// name through `mjx-dml`'s font scheme; a slot naming neither is `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveFonts {
    /// The Latin/ASCII-range typeface.
    pub ascii: Option<String>,
    /// The Latin "High ANSI" typeface.
    pub high_ansi: Option<String>,
    /// The East Asian typeface.
    pub east_asian: Option<String>,
    /// The complex-script typeface.
    pub complex_script: Option<String>,
    /// `w:rFonts@hint` — which script wins when a character could come from more than one.
    pub hint: Option<FontTypeHint>,
}

/// A resolved `w:bdr` / `w:top` / … (`CT_Border`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBorder {
    /// `w:val` — the border's line style.
    pub style: BorderStyle,
    /// `w:color`(+theme), resolved.
    pub color: EffectiveColor,
    /// `w:sz` — width in eighths of a point, if stated.
    pub width_eighths_of_a_point: Option<EighthPointMeasure>,
    /// `w:space` — spacing in points (defaults to `0` when the element is present).
    pub spacing_points: mjx_ooxml_types::wordprocessingml::PointMeasure,
    /// `w:shadow`.
    pub shadow: Option<bool>,
    /// `w:frame`.
    pub frame: Option<bool>,
}

/// A resolved `w:shd` (`CT_Shd`) — a shading pattern plus its two independent colours (the pattern
/// colour and the fill), each already resolved through any theme reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveShading {
    /// `w:val` — the shading pattern.
    pub pattern: ShadingPattern,
    /// `w:color`(+theme), resolved — the pattern's own colour.
    pub pattern_color: Option<EffectiveColor>,
    /// `w:fill`(+`themeFill`), resolved — the background colour the pattern draws over.
    pub fill: Option<EffectiveColor>,
}

/// A resolved `w:u` (`CT_Underline`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveUnderline {
    /// `w:val` — the underline style (`none` is a real, distinct value from the element being
    /// absent).
    pub style: Option<UnderlineKind>,
    /// `w:color`(+theme), resolved.
    pub color: EffectiveColor,
}

/// A resolved `w:fitText` (`CT_FitText`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveManualRunWidth {
    /// `w:val` — the manually-fitted width.
    pub width: mjx_ooxml_types::shared::TwipsMeasure,
    /// `w:id` — links the runs of one manually-fitted span together.
    pub id: Option<i64>,
}

/// A resolved `w:lang` (`CT_Language`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveLanguages {
    /// `w:val` — the Latin-text language.
    pub latin: Option<mjx_ooxml_types::shared::LanguageTag>,
    /// `w:eastAsia` — the East Asian-text language.
    pub east_asian: Option<mjx_ooxml_types::shared::LanguageTag>,
    /// `w:bidi` — the complex-script-text language.
    pub complex_script: Option<mjx_ooxml_types::shared::LanguageTag>,
}

/// A resolved `w:eastAsianLayout` (`CT_EastAsianLayout`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveEastAsianLayout {
    /// `w:id`.
    pub id: Option<i64>,
    /// `w:combine` — two-lines-in-one.
    pub combine_two_lines: Option<bool>,
    /// `w:combineBrackets`.
    pub combine_brackets: Option<CombineBrackets>,
    /// `w:vert` — vertical text.
    pub vertical: Option<bool>,
    /// `w:vertCompress`.
    pub vertical_compressed: Option<bool>,
}

/// A resolved `w:framePr` (`CT_FramePr`) — kept as one atomic value (see the module's own doc comment
/// for why the merge unit stops at the child-element level, not below it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveFrameProperties {
    /// `w:dropCap`.
    pub drop_cap: Option<DropCap>,
    /// `w:lines` — drop-cap line count.
    pub drop_cap_lines: Option<i64>,
    /// `w:w` — frame width, in twips.
    pub width: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:h` — frame height, in twips.
    pub height: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:vSpace`.
    pub vertical_spacing: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:hSpace`.
    pub horizontal_spacing: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:wrap`.
    pub wrap: Option<TextFrameWrapping>,
    /// `w:hAnchor`.
    pub horizontal_anchor: Option<HorizontalAnchor>,
    /// `w:vAnchor`.
    pub vertical_anchor: Option<VerticalAnchor>,
    /// `w:x`.
    pub x: Option<SignedTwipsMeasure>,
    /// `w:xAlign`.
    pub x_alignment: Option<RelativeHorizontalAlignment>,
    /// `w:y`.
    pub y: Option<SignedTwipsMeasure>,
    /// `w:yAlign`.
    pub y_alignment: Option<RelativeVerticalAlignment>,
    /// `w:hRule`.
    pub height_rule: Option<HeightRule>,
    /// `w:anchorLock`.
    pub anchor_lock: Option<bool>,
}

/// A resolved `w:ind` (`CT_Ind`) — every attribute independently, exactly as
/// [`Indentation`] itself hands them back (this reader does not additionally collapse the
/// logical/physical precedence — see that type's own doc comment).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveIndentation {
    /// `w:start`.
    pub start: Option<SignedTwipsMeasure>,
    /// `w:startChars`.
    pub start_chars: Option<i64>,
    /// `w:end`.
    pub end: Option<SignedTwipsMeasure>,
    /// `w:endChars`.
    pub end_chars: Option<i64>,
    /// `w:left`.
    pub left: Option<SignedTwipsMeasure>,
    /// `w:leftChars`.
    pub left_chars: Option<i64>,
    /// `w:right`.
    pub right: Option<SignedTwipsMeasure>,
    /// `w:rightChars`.
    pub right_chars: Option<i64>,
    /// `w:hanging`.
    pub hanging: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:hangingChars`.
    pub hanging_chars: Option<i64>,
    /// `w:firstLine`.
    pub first_line: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:firstLineChars`.
    pub first_line_chars: Option<i64>,
}

/// A resolved `w:spacing` (`CT_Spacing`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveSpacing {
    /// `w:before`.
    pub before: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:beforeLines`.
    pub before_lines: Option<i64>,
    /// `w:beforeAutospacing`.
    pub before_autospacing: Option<bool>,
    /// `w:after`.
    pub after: Option<mjx_ooxml_types::shared::TwipsMeasure>,
    /// `w:afterLines`.
    pub after_lines: Option<i64>,
    /// `w:afterAutospacing`.
    pub after_autospacing: Option<bool>,
    /// `w:line` paired with `w:lineRule`, together (see [`LineSpacing`]).
    pub line: Option<LineSpacing>,
}

/// One resolved `w:tab` (`CT_TabStop`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveTabStop {
    /// `w:val`.
    pub alignment: TabStopType,
    /// `w:leader`.
    pub leader: Option<TabStopLeader>,
    /// `w:pos`.
    pub position: SignedTwipsMeasure,
}

/// A resolved `w:cnfStyle` (`CT_Cnf`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveConditionalFormatting {
    /// `w:val` — the twelve-bit mask, as its own wire string.
    pub bitmask: Option<String>,
    /// `w:firstRow`.
    pub first_row: Option<bool>,
    /// `w:lastRow`.
    pub last_row: Option<bool>,
    /// `w:firstColumn`.
    pub first_column: Option<bool>,
    /// `w:lastColumn`.
    pub last_column: Option<bool>,
    /// `w:oddVBand`.
    pub odd_vertical_band: Option<bool>,
    /// `w:evenVBand`.
    pub even_vertical_band: Option<bool>,
    /// `w:oddHBand`.
    pub odd_horizontal_band: Option<bool>,
    /// `w:evenHBand`.
    pub even_horizontal_band: Option<bool>,
    /// `w:firstRowFirstColumn`.
    pub first_row_first_column: Option<bool>,
    /// `w:firstRowLastColumn`.
    pub first_row_last_column: Option<bool>,
    /// `w:lastRowFirstColumn`.
    pub last_row_first_column: Option<bool>,
    /// `w:lastRowLastColumn`.
    pub last_row_last_column: Option<bool>,
}

/// A resolved `w:pBdr` (`CT_PBdr`) — the six borders, each independently resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveParagraphBorders {
    /// `w:top`.
    pub top: Option<EffectiveBorder>,
    /// `w:left`.
    pub left: Option<EffectiveBorder>,
    /// `w:bottom`.
    pub bottom: Option<EffectiveBorder>,
    /// `w:right`.
    pub right: Option<EffectiveBorder>,
    /// `w:between`.
    pub between: Option<EffectiveBorder>,
    /// `w:bar`.
    pub bar: Option<EffectiveBorder>,
}

/// Which numbering definition instance and level a paragraph's `w:numPr` (its own, or the first its
/// paragraph-style chain states) names. Resolving *what that renders as* is
/// [`Document::resolve_numbering`] — this is only the reference, echoed back the same way every other
/// `EG_RPrBase`/`CT_PPrBase` member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveNumberingReference {
    /// `w:numId`.
    pub numbering_id: i64,
    /// `w:ilvl`, defaulting to `0` when the file states a `numId` without one (`CT_NumPr`'s `ilvl` is
    /// `minOccurs="0"`).
    pub level: i64,
}

/// Turns an [`AttributeError`] into a [`DocxError`] the same way every hand-written accessor
/// elsewhere in this crate does (`.map_err(FromXmlError::from)?`), once, so the 38+32 field
/// extractions below read as plain field assignments.
fn attr<T>(result: Result<T, AttributeError>) -> Result<T, DocxError> {
    result.map_err(|error| DocxError::from(FromXmlError::from(error)))
}

// -------------------------------------------------------------------------------------------
// Theme resolution — the WordprocessingML side of `w:themeColor`/`w:rFonts`'s theme attributes.
//
// Word's `ST_ThemeColor` names a *logical* slot (`background1`, `text1`, …) that a document-level
// `w:clrSchemeMapping` (`word/settings.xml`, not modelled by any child yet) can remap; absent that
// element, ECMA-376 Part 1 §17.15.1.20 states each attribute's own default explicitly: `bg1` defaults
// to `light1`, `bg2` to `light2`, and every `accentN`/`hyperlink`/`followedHyperlink` defaults to
// itself. `t1`/`t2`'s own prose ("if this attribute is omitted, then the t1/t2 theme color shall be
// used") does not literally name `dark1`/`dark2` the way `bg1`/`bg2`'s prose names `light1`/`light2`
// — but §17.18.103's `ST_WmlColorSchemeIndex` restricts `clrSchemeMapping`'s own attribute *values*
// to the ten physical/hyperlink slots, so `t1`/`t2` cannot even name themselves literally; taken with
// the universal `bg1→lt1`/`tx1→dk1`/`bg2→lt2`/`tx2→dk2` convention DrawingML's own default `p:clrMap`
// already uses ([`mjx_dml::ColorMap::identity`]'s own doc comment states exactly this pairing), the
// only sound reading is that `t1`/`t2` default to `dark1`/`dark2`, symmetric to `bg1`/`bg2`. This
// module therefore reuses [`mjx_dml::ColorMap::identity`] directly for the `background1`/`text1`/
// `background2`/`text2` half of the mapping, rather than restating it — see
// [the guide](crate::effective_properties) for the full account and the citation to double-check.
// -------------------------------------------------------------------------------------------

/// Word's `ST_ThemeColor` mapped onto DrawingML's `a:schemeClr` vocabulary — the wire tokens mostly
/// coincide (`accent1`…`accent6`, `hyperlink`, `followedHyperlink`); `dark1`/`light1`/`dark2`/`light2`
/// spell what `a:schemeClr` calls `dk1`/`lt1`/`dk2`/`lt2`, and `background1`/`text1`/`background2`/
/// `text2` spell what `a:schemeClr` calls `bg1`/`tx1`/`bg2`/`tx2` — see the module's own doc comment
/// for the default `bg1`/`tx1`/`bg2`/`tx2` mapping this then goes through via
/// [`mjx_dml::ColorMap::identity`]. `none` has no DrawingML counterpart at all.
fn word_theme_color_to_scheme_color(color: ThemeColor) -> Option<mjx_dml::SchemeColor> {
    use mjx_dml::SchemeColor as S;
    Some(match color {
        ThemeColor::Dark1 => S::Dark1,
        ThemeColor::Light1 => S::Light1,
        ThemeColor::Dark2 => S::Dark2,
        ThemeColor::Light2 => S::Light2,
        ThemeColor::Accent1 => S::Accent1,
        ThemeColor::Accent2 => S::Accent2,
        ThemeColor::Accent3 => S::Accent3,
        ThemeColor::Accent4 => S::Accent4,
        ThemeColor::Accent5 => S::Accent5,
        ThemeColor::Accent6 => S::Accent6,
        ThemeColor::Hyperlink => S::Hyperlink,
        ThemeColor::FollowedHyperlink => S::FollowedHyperlink,
        ThemeColor::Background1 => S::Background1,
        ThemeColor::Background2 => S::Background2,
        ThemeColor::Text1 => S::Text1,
        ThemeColor::Text2 => S::Text2,
        ThemeColor::None => return None,
    })
}

/// Word's `ST_Theme` (a `w:rFonts` theme attribute's own value) mapped onto which font-scheme
/// collection and script slot it names. `majorAscii`/`majorHAnsi` (and their minor counterparts) both
/// name the theme's single Latin typeface — DrawingML's font scheme has no separate "High ANSI" slot,
/// because that distinction (which codepoints are "ASCII" versus "High ANSI") is WordprocessingML's
/// own, not the theme's.
fn word_theme_font_slot(font: ThemeFont) -> (FontSchemeSlot, mjx_dml::FontSlot) {
    use mjx_dml::FontSlot as F;
    use FontSchemeSlot as C;
    match font {
        ThemeFont::MajorAscii | ThemeFont::MajorHighAnsi => (C::Major, F::Latin),
        ThemeFont::MajorEastAsia => (C::Major, F::EastAsian),
        ThemeFont::MajorComplexScript => (C::Major, F::ComplexScript),
        ThemeFont::MinorAscii | ThemeFont::MinorHighAnsi => (C::Minor, F::Latin),
        ThemeFont::MinorEastAsia => (C::Minor, F::EastAsian),
        ThemeFont::MinorComplexScript => (C::Minor, F::ComplexScript),
    }
}

/// The theme this document's `theme1.xml` carries, resolved once per `effective_*` call and kept
/// interner-free from that point on: the colour scheme (as [`SchemeColors`], sysClr-aware) and the
/// font scheme. `None` (both fields) when the document relates to no `word/theme/themeN.xml` at all —
/// every theme-bearing field then simply keeps its unresolved `themeColor`/theme-font form (see
/// [`EffectiveColor`]/[`EffectiveFonts`]'s own doc comments: there is nothing dishonest about that,
/// the file points somewhere the document does not go).
///
/// [`ThemeContext::resolve`] builds a throwaway `a:schemeClr` element to hand to
/// [`mjx_dml::resolve_color`] — genuine reuse of that resolver's own sysClr/tint/shade-aware
/// machinery rather than a second implementation — using a dedicated scratch [`Interner`] that never
/// meets `word/document.xml`'s, `styles.xml`'s or `numbering.xml`'s own (a scheme-colour wire token
/// like `"dk1"` is plain ASCII, so which interner it is momentarily stored in is immaterial).
struct ThemeContext {
    colors: Option<SchemeColors>,
    fonts: Option<mjx_dml::FontScheme>,
    scratch: RefCell<Interner>,
}

impl ThemeContext {
    fn empty() -> Self {
        Self {
            colors: None,
            fonts: None,
            scratch: RefCell::new(Interner::new()),
        }
    }

    /// Resolves a `w:themeColor` reference to a concrete colour, or `None` when the theme does not
    /// define that slot (or this document has no theme at all).
    fn resolve(&self, color: ThemeColor) -> Option<ResolvedColor> {
        let scheme_colors = self.colors.as_ref()?;
        let scheme_color = word_theme_color_to_scheme_color(color)?;
        let mut scratch = self.scratch.borrow_mut();
        let synthetic = mjx_dml::Color::scheme(&mut scratch, scheme_color);
        resolve_color(
            &synthetic,
            scheme_colors,
            &ColorMap::identity(),
            None,
            &scratch,
        )
    }

    /// Resolves a `w:rFonts` theme attribute (`asciiTheme`, …) to a concrete typeface name, or `None`
    /// when the scheme leaves that slot undefined (or this document has no theme at all).
    fn resolve_font(&self, font: ThemeFont) -> Option<String> {
        let scheme = self.fonts.as_ref()?;
        let (collection, slot) = word_theme_font_slot(font);
        let typeface = &scheme.collection(collection).font(slot)?.typeface;
        (!typeface.is_empty()).then(|| typeface.clone())
    }
}

/// Resolves a `w:color`/`w:u`/`w:bdr`-shaped colour (a required or `auto`-defaulted hex plus the
/// theme triple) against `theme`.
fn resolve_wml_color(
    hex_value: &HexadecimalColor,
    theme_color: Option<ThemeColor>,
    theme: &ThemeContext,
) -> EffectiveColor {
    if let Some(resolved) = theme_color.and_then(|color| theme.resolve(color)) {
        return EffectiveColor::Hex(resolved.to_hex());
    }
    if hex_value.to_wire().eq_ignore_ascii_case("auto") {
        EffectiveColor::Auto
    } else {
        EffectiveColor::Hex(hex_value.to_wire().to_uppercase())
    }
}

// -------------------------------------------------------------------------------------------
// EffectiveCharacterProperties — EG_RPrBase's 39 members minus `rStyle` (which selects the
// character-style rung rather than contributing a resolved value), 38 fields.
// -------------------------------------------------------------------------------------------

/// The **effective** character formatting a run renders with — every `EG_RPrBase` member resolved
/// across the ladder, colours baked to concrete `RRGGBB` where the source names a theme reference.
///
/// See [the guide](crate::effective_properties) for the ladder order, the toggle rule, and where this
/// reader stops.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectiveCharacterProperties {
    // The twelve formal toggle properties (ECMA-376 Part 1 §17.7.3) — combined by XOR across the
    // style hierarchy, not by plain override. See [`combine_toggle`].
    /// `w:b` — bold. A toggle property.
    pub bold: Option<bool>,
    /// `w:bCs` — bold (complex script). A toggle property.
    pub bold_complex_script: Option<bool>,
    /// `w:i` — italic. A toggle property.
    pub italic: Option<bool>,
    /// `w:iCs` — italic (complex script). A toggle property.
    pub italic_complex_script: Option<bool>,
    /// `w:caps` — all capitals. A toggle property.
    pub all_capitals: Option<bool>,
    /// `w:smallCaps`. A toggle property.
    pub small_caps: Option<bool>,
    /// `w:strike` — single strikethrough. A toggle property.
    pub strikethrough: Option<bool>,
    /// `w:outline` — character outline. A toggle property.
    pub outline: Option<bool>,
    /// `w:shadow`. A toggle property.
    pub shadow: Option<bool>,
    /// `w:emboss`. A toggle property.
    pub embossing: Option<bool>,
    /// `w:imprint`. A toggle property.
    pub imprinting: Option<bool>,
    /// `w:vanish` — hidden text. A toggle property.
    pub hidden: Option<bool>,

    // The eight remaining `CT_OnOff`-shaped members — **not** toggle properties per §17.7.3's own
    // list, despite the identical wire shape; plain override like every other field below.
    /// `w:dstrike` — double strikethrough. Plain override, **not** a toggle property.
    pub double_strikethrough: Option<bool>,
    /// `w:noProof`. Plain override.
    pub proofing_exempt: Option<bool>,
    /// `w:snapToGrid` (inter-character spacing). Plain override.
    pub snap_to_grid: Option<bool>,
    /// `w:webHidden`. Plain override.
    pub web_hidden: Option<bool>,
    /// `w:rtl`. Plain override.
    pub right_to_left: Option<bool>,
    /// `w:cs`. Plain override.
    pub complex_script: Option<bool>,
    /// `w:specVanish`. Plain override.
    pub always_hidden: Option<bool>,
    /// `w:oMath`. Plain override.
    pub math: Option<bool>,

    // The eighteen value-shaped members, plain override (whichever rung states the member first,
    // walking direct → character-style chain → paragraph-style chain → numbering level →
    // docDefaults, wins outright — see the guide for why this granularity is correct).
    /// `w:rFonts`, each script slot resolved through the theme where it names one.
    pub fonts: Option<EffectiveFonts>,
    /// `w:color`, resolved through the theme where it names one.
    pub color: Option<EffectiveColor>,
    /// `w:spacing` — character spacing, in twentieths of a point.
    pub character_spacing: Option<SignedTwipsMeasure>,
    /// `w:w` — horizontal character scale.
    pub character_scale: Option<TextScale>,
    /// `w:kern` — the font-size threshold above which kerning applies.
    pub kerning: Option<HalfPointMeasure>,
    /// `w:position` — baseline offset, in half-points.
    pub vertical_offset: Option<SignedHalfPointMeasure>,
    /// `w:sz` — font size, in half-points.
    pub font_size: Option<HalfPointMeasure>,
    /// `w:szCs` — complex-script font size, in half-points.
    pub complex_script_font_size: Option<HalfPointMeasure>,
    /// `w:highlight`.
    pub highlight: Option<HighlightColor>,
    /// `w:u`, resolved.
    pub underline: Option<EffectiveUnderline>,
    /// `w:effect`.
    pub text_effect: Option<TextEffectKind>,
    /// `w:bdr`, resolved.
    pub border: Option<EffectiveBorder>,
    /// `w:shd`, resolved.
    pub shading: Option<EffectiveShading>,
    /// `w:fitText`.
    pub manual_run_width: Option<EffectiveManualRunWidth>,
    /// `w:vertAlign` — subscript/superscript.
    pub vertical_alignment: Option<VerticalTextPosition>,
    /// `w:em` — emphasis mark.
    pub emphasis_mark: Option<EmphasisMark>,
    /// `w:lang`.
    pub languages: Option<EffectiveLanguages>,
    /// `w:eastAsianLayout`.
    pub east_asian_layout: Option<EffectiveEastAsianLayout>,
}

/// Builds a field-by-field plain-fallback merge (`self`'s own values win; `other`'s fill in whatever
/// `self` leaves unset) for a struct all of whose fields are `Option<T: Clone>` — the one merge
/// primitive both [`EffectiveCharacterProperties::merge_under`] and
/// [`EffectiveParagraphProperties::merge_under`] build on.
macro_rules! merge_under_fields {
    ($ty:ident, $self:expr, $other:expr, [$($field:ident),+ $(,)?]) => {
        $ty {
            $($field: $self.$field.clone().or_else(|| $other.$field.clone())),+
        }
    };
}

impl EffectiveCharacterProperties {
    /// `self`'s own values win; `other`'s fill in whatever `self` leaves unset. The correct
    /// combination **within** one style's `w:basedOn` chain for every field (ECMA-376 Part 1 §17.7.1's
    /// "attempt to read the value in the style; if it does not exist … repeat" — plain fallback, even
    /// for the twelve toggle properties), and the correct combination **across** ladder tiers for the
    /// 26 non-toggle fields. The twelve toggle fields need a second pass after folding across tiers —
    /// see `combine_toggle` (this module) — because §17.7.3's cross-tier rule is XOR, not fallback.
    #[must_use]
    pub fn merge_under(&self, other: &Self) -> Self {
        merge_under_fields!(
            Self,
            self,
            other,
            [
                bold,
                bold_complex_script,
                italic,
                italic_complex_script,
                all_capitals,
                small_caps,
                strikethrough,
                outline,
                shadow,
                embossing,
                imprinting,
                hidden,
                double_strikethrough,
                proofing_exempt,
                snap_to_grid,
                web_hidden,
                right_to_left,
                complex_script,
                always_hidden,
                math,
                fonts,
                color,
                character_spacing,
                character_scale,
                kerning,
                vertical_offset,
                font_size,
                complex_script_font_size,
                highlight,
                underline,
                text_effect,
                border,
                shading,
                manual_run_width,
                vertical_alignment,
                emphasis_mark,
                languages,
                east_asian_layout,
            ]
        )
    }
}

/// Combines one toggle property's five per-tier values per ECMA-376 Part 1 §17.7.3: a direct value
/// wins outright; otherwise a `true` at `doc_defaults` wins outright; otherwise the remaining
/// tiers — numbering, the (already within-chain-resolved) paragraph-style tier, and the
/// (already within-chain-resolved) character-style tier — combine by Boolean XOR, a tier with no
/// stated value simply not contributing (XOR's identity element is exactly "no opinion"). Proved by
/// mutation in `tests/effective.rs`: replacing this with plain fallback (last non-`None` wins) turns
/// the toggle test red.
fn combine_toggle(
    direct: Option<bool>,
    doc_defaults: Option<bool>,
    numbering: Option<bool>,
    paragraph_tier: Option<bool>,
    character_tier: Option<bool>,
) -> Option<bool> {
    if let Some(direct) = direct {
        return Some(direct);
    }
    if doc_defaults == Some(true) {
        return Some(true);
    }
    let terms = [numbering, paragraph_tier, character_tier];
    if terms.iter().all(Option::is_none) {
        return doc_defaults;
    }
    Some(
        terms
            .into_iter()
            .flatten()
            .fold(false, |acc, value| acc ^ value),
    )
}

/// Recombines every toggle field of `merged` (already plain-fallback-merged across tiers, which is
/// wrong for these twelve) using [`combine_toggle`] against the five tiers' own already-extracted,
/// already within-chain-resolved values.
fn recombine_toggles(
    merged: &mut EffectiveCharacterProperties,
    direct: &EffectiveCharacterProperties,
    doc_defaults: &EffectiveCharacterProperties,
    numbering: &EffectiveCharacterProperties,
    paragraph_tier: &EffectiveCharacterProperties,
    character_tier: &EffectiveCharacterProperties,
) {
    macro_rules! recombine {
        ([$($field:ident),+ $(,)?]) => {
            $(
                merged.$field = combine_toggle(
                    direct.$field,
                    doc_defaults.$field,
                    numbering.$field,
                    paragraph_tier.$field,
                    character_tier.$field,
                );
            )+
        };
    }
    // The twelve formal toggle-property fields alone (ECMA-376 Part 1 §17.7.3) — must stay in sync
    // with the module doc comment's own list; the mutation test in `tests/effective.rs` (replacing
    // this whole recombination with a no-op) is what proves the sync actually matters.
    recombine!([
        bold,
        bold_complex_script,
        italic,
        italic_complex_script,
        all_capitals,
        small_caps,
        strikethrough,
        outline,
        shadow,
        embossing,
        imprinting,
        hidden,
    ]);
}

/// Extracts every `EG_RPrBase` field `rpr` states, resolving colours/fonts through `theme` — the one
/// function every rung (direct, each style's own `w:rPr`, a numbering level's, `w:rPrDefault`) is
/// read through, so a member added here is added for every rung at once. `interner` is whichever
/// part `rpr` itself was parsed from (`word/document.xml`, `styles.xml` or `numbering.xml`) — `theme`
/// is already interner-free and never touches it.
fn extract_run_properties(
    rpr: &RunProperties,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveCharacterProperties, DocxError> {
    let fonts = rpr
        .fonts()
        .map(|fonts| extract_fonts(fonts, theme, interner))
        .transpose()?;
    let color = rpr
        .color()
        .map(|color| extract_wml_color(color, theme, interner))
        .transpose()?;
    let underline = rpr
        .underline()
        .map(|underline| extract_underline(underline, theme, interner))
        .transpose()?;
    let border = rpr
        .border()
        .map(|border| extract_border(border, theme, interner))
        .transpose()?;
    let shading = rpr
        .shading()
        .map(|shading| extract_shading(shading, theme, interner))
        .transpose()?;
    let manual_run_width = rpr
        .manual_run_width()
        .map(|value| extract_manual_run_width(value, interner))
        .transpose()?;
    let languages = rpr
        .languages()
        .map(|value| extract_languages(value, interner))
        .transpose()?;
    let east_asian_layout = rpr
        .east_asian_layout()
        .map(|value| extract_east_asian_layout(value, interner))
        .transpose()?;

    Ok(EffectiveCharacterProperties {
        bold: attr(rpr.bold(interner))?,
        bold_complex_script: attr(rpr.bold_complex_script(interner))?,
        italic: attr(rpr.italic(interner))?,
        italic_complex_script: attr(rpr.italic_complex_script(interner))?,
        all_capitals: attr(rpr.all_capitals(interner))?,
        small_caps: attr(rpr.small_caps(interner))?,
        strikethrough: attr(rpr.strikethrough(interner))?,
        outline: attr(rpr.outline(interner))?,
        shadow: attr(rpr.shadow(interner))?,
        embossing: attr(rpr.embossing(interner))?,
        imprinting: attr(rpr.imprinting(interner))?,
        hidden: attr(rpr.hidden(interner))?,
        double_strikethrough: attr(rpr.double_strikethrough(interner))?,
        proofing_exempt: attr(rpr.proofing_exempt(interner))?,
        snap_to_grid: attr(rpr.snap_to_grid(interner))?,
        web_hidden: attr(rpr.web_hidden(interner))?,
        right_to_left: attr(rpr.right_to_left(interner))?,
        complex_script: attr(rpr.complex_script(interner))?,
        always_hidden: attr(rpr.always_hidden(interner))?,
        math: attr(rpr.math(interner))?,
        fonts,
        color,
        character_spacing: rpr
            .character_spacing()
            .map(|value| attr(value.twentieths_of_a_point(interner)))
            .transpose()?,
        character_scale: rpr
            .character_scale()
            .map(|value| attr(value.percentage(interner)))
            .transpose()?
            .flatten(),
        kerning: attr(rpr.kerning(interner))?,
        vertical_offset: rpr
            .vertical_offset()
            .map(|value| attr(value.half_points(interner)))
            .transpose()?,
        font_size: attr(rpr.font_size(interner))?,
        complex_script_font_size: attr(rpr.complex_script_font_size(interner))?,
        highlight: rpr
            .highlight()
            .map(|value| attr(value.color(interner)))
            .transpose()?,
        underline,
        text_effect: rpr
            .text_effect()
            .map(|value| attr(value.kind(interner)))
            .transpose()?,
        border,
        shading,
        manual_run_width,
        vertical_alignment: rpr
            .vertical_alignment()
            .map(|value| attr(value.position(interner)))
            .transpose()?,
        emphasis_mark: rpr
            .emphasis_mark()
            .map(|value| attr(value.mark(interner)))
            .transpose()?,
        languages,
        east_asian_layout,
    })
}

fn extract_fonts(
    fonts: &Fonts,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveFonts, DocxError> {
    let ascii_literal = attr(fonts.ascii_font(interner))?.map(Cow::into_owned);
    let ascii_theme = attr(fonts.ascii_theme_font(interner))?;
    let high_ansi_literal = attr(fonts.high_ansi_font(interner))?.map(Cow::into_owned);
    let high_ansi_theme = attr(fonts.high_ansi_theme_font(interner))?;
    let east_asian_literal = attr(fonts.east_asian_font(interner))?.map(Cow::into_owned);
    let east_asian_theme = attr(fonts.east_asian_theme_font(interner))?;
    let complex_script_literal = attr(fonts.complex_script_font(interner))?.map(Cow::into_owned);
    let complex_script_theme = attr(fonts.complex_script_theme_font(interner))?;
    Ok(EffectiveFonts {
        ascii: ascii_literal.or_else(|| ascii_theme.and_then(|font| theme.resolve_font(font))),
        high_ansi: high_ansi_literal
            .or_else(|| high_ansi_theme.and_then(|font| theme.resolve_font(font))),
        east_asian: east_asian_literal
            .or_else(|| east_asian_theme.and_then(|font| theme.resolve_font(font))),
        complex_script: complex_script_literal
            .or_else(|| complex_script_theme.and_then(|font| theme.resolve_font(font))),
        hint: attr(fonts.hint(interner))?,
    })
}

fn extract_wml_color(
    color: &WmlColor,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveColor, DocxError> {
    let hex_value = attr(color.hex_value(interner))?;
    let theme_color = attr(color.theme_color(interner))?;
    Ok(resolve_wml_color(&hex_value, theme_color, theme))
}

fn extract_underline(
    underline: &Underline,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveUnderline, DocxError> {
    let style = attr(underline.style(interner))?;
    let color = attr(underline.color(interner))?;
    let theme_color = attr(underline.theme_color(interner))?;
    Ok(EffectiveUnderline {
        style,
        color: resolve_wml_color(&color, theme_color, theme),
    })
}

fn extract_border(
    border: &Border,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveBorder, DocxError> {
    let style = attr(border.style(interner))?;
    let color = attr(border.color(interner))?;
    let theme_color = attr(border.theme_color(interner))?;
    Ok(EffectiveBorder {
        style,
        color: resolve_wml_color(&color, theme_color, theme),
        width_eighths_of_a_point: attr(border.width_eighths_of_a_point(interner))?,
        spacing_points: attr(border.spacing_points(interner))?,
        shadow: attr(border.shadow(interner))?,
        frame: attr(border.frame(interner))?,
    })
}

fn extract_shading(
    shading: &Shading,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveShading, DocxError> {
    let pattern = attr(shading.pattern(interner))?;
    let pattern_color = attr(shading.color(interner))?;
    let pattern_theme_color = attr(shading.theme_color(interner))?;
    let fill_color = attr(shading.fill_color(interner))?;
    let fill_theme_color = attr(shading.theme_fill_color(interner))?;
    Ok(EffectiveShading {
        pattern,
        pattern_color: pattern_color.map(|hex| resolve_wml_color(&hex, pattern_theme_color, theme)),
        fill: fill_color.map(|hex| resolve_wml_color(&hex, fill_theme_color, theme)),
    })
}

fn extract_manual_run_width(
    value: &ManualRunWidth,
    interner: &Interner,
) -> Result<EffectiveManualRunWidth, DocxError> {
    Ok(EffectiveManualRunWidth {
        width: attr(value.width(interner))?,
        id: attr(value.id(interner))?,
    })
}

fn extract_languages(
    value: &Languages,
    interner: &Interner,
) -> Result<EffectiveLanguages, DocxError> {
    Ok(EffectiveLanguages {
        latin: attr(value.latin(interner))?,
        east_asian: attr(value.east_asian(interner))?,
        complex_script: attr(value.complex_script(interner))?,
    })
}

fn extract_east_asian_layout(
    value: &EastAsianLayout,
    interner: &Interner,
) -> Result<EffectiveEastAsianLayout, DocxError> {
    Ok(EffectiveEastAsianLayout {
        id: attr(value.id(interner))?,
        combine_two_lines: attr(value.combine_two_lines(interner))?,
        combine_brackets: attr(value.combine_brackets(interner))?,
        vertical: attr(value.vertical(interner))?,
        vertical_compressed: attr(value.vertical_compressed(interner))?,
    })
}

// -------------------------------------------------------------------------------------------
// EffectiveParagraphProperties — CT_PPrBase's 33 members minus `pStyle` (the tier selector), 32
// fields. None of the eighteen `CT_OnOff` members here is a toggle property (ECMA-376 Part 1
// §17.7.3's list names only run-level, §17.3.2.x properties) — every field below is plain override,
// no XOR pass needed.
// -------------------------------------------------------------------------------------------

/// The **effective** paragraph formatting a paragraph renders with — every `CT_PPrBase` member
/// resolved across the ladder.
///
/// See [the guide](crate::effective_properties) for the ladder order and where this reader stops.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectiveParagraphProperties {
    /// `w:keepNext`.
    pub keep_with_next: Option<bool>,
    /// `w:keepLines`.
    pub keep_lines_together: Option<bool>,
    /// `w:pageBreakBefore`.
    pub page_break_before: Option<bool>,
    /// `w:widowControl`.
    pub widow_control: Option<bool>,
    /// `w:suppressLineNumbers`.
    pub suppress_line_numbers: Option<bool>,
    /// `w:suppressAutoHyphens`.
    pub suppress_auto_hyphens: Option<bool>,
    /// `w:kinsoku`.
    pub east_asian_line_breaking_rules: Option<bool>,
    /// `w:wordWrap`.
    pub word_wrap: Option<bool>,
    /// `w:overflowPunct`.
    pub overflow_punctuation: Option<bool>,
    /// `w:topLinePunct`.
    pub compress_punctuation_at_line_start: Option<bool>,
    /// `w:autoSpaceDE`.
    pub auto_space_latin_and_east_asian: Option<bool>,
    /// `w:autoSpaceDN`.
    pub auto_space_east_asian_and_numbers: Option<bool>,
    /// `w:bidi`.
    pub right_to_left_layout: Option<bool>,
    /// `w:adjustRightInd`.
    pub adjust_right_indent_for_document_grid: Option<bool>,
    /// `w:snapToGrid` (inter-line spacing).
    pub snap_to_grid: Option<bool>,
    /// `w:contextualSpacing`.
    pub contextual_spacing: Option<bool>,
    /// `w:mirrorIndents`.
    pub mirror_indents: Option<bool>,
    /// `w:suppressOverlap`.
    pub suppress_overlap: Option<bool>,

    /// `w:framePr`.
    pub frame: Option<EffectiveFrameProperties>,
    /// `w:numPr` — the reference alone; see [`EffectiveNumberingReference`]'s own doc comment.
    pub numbering: Option<EffectiveNumberingReference>,
    /// `w:pBdr`, each of its six borders resolved.
    pub borders: Option<EffectiveParagraphBorders>,
    /// `w:shd`, resolved.
    pub shading: Option<EffectiveShading>,
    /// `w:tabs`.
    pub tab_stops: Option<Vec<EffectiveTabStop>>,
    /// `w:spacing`.
    pub spacing: Option<EffectiveSpacing>,
    /// `w:ind`.
    pub indentation: Option<EffectiveIndentation>,
    /// `w:jc`.
    pub alignment: Option<Justification>,
    /// `w:textDirection`.
    pub text_direction: Option<TextFlowDirection>,
    /// `w:textAlignment`.
    pub vertical_character_alignment: Option<VerticalTextAlignment>,
    /// `w:textboxTightWrap`.
    pub text_box_tight_wrap: Option<TextBoxTightWrap>,
    /// `w:outlineLvl`.
    pub outline_level: Option<i64>,
    /// `w:divId`.
    pub associated_html_div_id: Option<i64>,
    /// `w:cnfStyle`.
    pub conditional_formatting: Option<EffectiveConditionalFormatting>,
}

impl EffectiveParagraphProperties {
    /// `self`'s own values win; `other`'s fill in whatever `self` leaves unset — used both within a
    /// style's own `w:basedOn` chain and across the ladder's tiers (no toggle-property special case
    /// is needed here; see the module's own doc comment).
    #[must_use]
    pub fn merge_under(&self, other: &Self) -> Self {
        merge_under_fields!(
            Self,
            self,
            other,
            [
                keep_with_next,
                keep_lines_together,
                page_break_before,
                widow_control,
                suppress_line_numbers,
                suppress_auto_hyphens,
                east_asian_line_breaking_rules,
                word_wrap,
                overflow_punctuation,
                compress_punctuation_at_line_start,
                auto_space_latin_and_east_asian,
                auto_space_east_asian_and_numbers,
                right_to_left_layout,
                adjust_right_indent_for_document_grid,
                snap_to_grid,
                contextual_spacing,
                mirror_indents,
                suppress_overlap,
                frame,
                numbering,
                borders,
                shading,
                tab_stops,
                spacing,
                indentation,
                alignment,
                text_direction,
                vertical_character_alignment,
                text_box_tight_wrap,
                outline_level,
                associated_html_div_id,
                conditional_formatting,
            ]
        )
    }
}

/// Extracts every `CT_PPrBase` field `ppr` states, resolving colours through `theme` — for a style
/// definition's, `w:pPrDefault`'s, or a numbering level's own `w:pPr` (`CT_PPrGeneral` /
/// [`StyleParagraphProperties`]).
fn extract_style_paragraph_properties(
    ppr: &StyleParagraphProperties,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveParagraphProperties, DocxError> {
    let numbering = ppr
        .numbering()
        .map(|value| extract_numbering_reference(value, interner))
        .transpose()?
        .flatten();
    let borders = ppr
        .borders()
        .map(|value| extract_paragraph_borders(value, theme, interner))
        .transpose()?;
    let shading = ppr
        .shading()
        .map(|value| extract_shading(value, theme, interner))
        .transpose()?;
    let tab_stops = ppr
        .tab_stops()
        .map(|value| extract_tab_stops(value, interner))
        .transpose()?;
    let spacing = ppr
        .spacing()
        .map(|value| extract_spacing(value, interner))
        .transpose()?;
    let indentation = ppr
        .indentation()
        .map(|value| extract_indentation(value, interner))
        .transpose()?;
    let frame = ppr
        .frame()
        .map(|value| extract_frame_properties(value, interner))
        .transpose()?;
    let conditional_formatting = ppr
        .conditional_formatting()
        .map(|value| extract_conditional_formatting(value, interner))
        .transpose()?;

    Ok(EffectiveParagraphProperties {
        keep_with_next: attr(ppr.keep_with_next(interner))?,
        keep_lines_together: attr(ppr.keep_lines_together(interner))?,
        page_break_before: attr(ppr.page_break_before(interner))?,
        widow_control: attr(ppr.widow_control(interner))?,
        suppress_line_numbers: attr(ppr.suppress_line_numbers(interner))?,
        suppress_auto_hyphens: attr(ppr.suppress_auto_hyphens(interner))?,
        east_asian_line_breaking_rules: attr(ppr.east_asian_line_breaking_rules(interner))?,
        word_wrap: attr(ppr.word_wrap(interner))?,
        overflow_punctuation: attr(ppr.overflow_punctuation(interner))?,
        compress_punctuation_at_line_start: attr(ppr.compress_punctuation_at_line_start(interner))?,
        auto_space_latin_and_east_asian: attr(ppr.auto_space_latin_and_east_asian(interner))?,
        auto_space_east_asian_and_numbers: attr(ppr.auto_space_east_asian_and_numbers(interner))?,
        right_to_left_layout: attr(ppr.right_to_left_layout(interner))?,
        adjust_right_indent_for_document_grid: attr(
            ppr.adjust_right_indent_for_document_grid(interner),
        )?,
        snap_to_grid: attr(ppr.snap_to_grid(interner))?,
        contextual_spacing: attr(ppr.contextual_spacing(interner))?,
        mirror_indents: attr(ppr.mirror_indents(interner))?,
        suppress_overlap: attr(ppr.suppress_overlap(interner))?,
        frame,
        numbering,
        borders,
        shading,
        tab_stops,
        spacing,
        indentation,
        alignment: ppr
            .alignment()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        text_direction: ppr
            .text_direction()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        vertical_character_alignment: ppr
            .vertical_character_alignment()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        text_box_tight_wrap: ppr
            .text_box_tight_wrap()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        outline_level: attr(ppr.outline_level(interner))?,
        associated_html_div_id: attr(ppr.associated_html_div_id(interner))?,
        conditional_formatting,
    })
}

/// The same extraction as [`extract_style_paragraph_properties`], for a live paragraph's own `w:pPr`
/// (`CT_PPr`/[`ParagraphProperties`]) rather than a style's `CT_PPrGeneral` — the two types expose the
/// identical 32 accessor names (see `paragraph_properties.rs`'s and `styles.rs`'s own doc comments for
/// why they are two Rust types at all), so the field-by-field body is intentionally the same shape.
fn extract_paragraph_properties(
    ppr: &ParagraphProperties,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveParagraphProperties, DocxError> {
    let numbering = ppr
        .numbering()
        .map(|value| extract_numbering_reference(value, interner))
        .transpose()?
        .flatten();
    let borders = ppr
        .borders()
        .map(|value| extract_paragraph_borders(value, theme, interner))
        .transpose()?;
    let shading = ppr
        .shading()
        .map(|value| extract_shading(value, theme, interner))
        .transpose()?;
    let tab_stops = ppr
        .tab_stops()
        .map(|value| extract_tab_stops(value, interner))
        .transpose()?;
    let spacing = ppr
        .spacing()
        .map(|value| extract_spacing(value, interner))
        .transpose()?;
    let indentation = ppr
        .indentation()
        .map(|value| extract_indentation(value, interner))
        .transpose()?;
    let frame = ppr
        .frame()
        .map(|value| extract_frame_properties(value, interner))
        .transpose()?;
    let conditional_formatting = ppr
        .conditional_formatting()
        .map(|value| extract_conditional_formatting(value, interner))
        .transpose()?;

    Ok(EffectiveParagraphProperties {
        keep_with_next: attr(ppr.keep_with_next(interner))?,
        keep_lines_together: attr(ppr.keep_lines_together(interner))?,
        page_break_before: attr(ppr.page_break_before(interner))?,
        widow_control: attr(ppr.widow_control(interner))?,
        suppress_line_numbers: attr(ppr.suppress_line_numbers(interner))?,
        suppress_auto_hyphens: attr(ppr.suppress_auto_hyphens(interner))?,
        east_asian_line_breaking_rules: attr(ppr.east_asian_line_breaking_rules(interner))?,
        word_wrap: attr(ppr.word_wrap(interner))?,
        overflow_punctuation: attr(ppr.overflow_punctuation(interner))?,
        compress_punctuation_at_line_start: attr(ppr.compress_punctuation_at_line_start(interner))?,
        auto_space_latin_and_east_asian: attr(ppr.auto_space_latin_and_east_asian(interner))?,
        auto_space_east_asian_and_numbers: attr(ppr.auto_space_east_asian_and_numbers(interner))?,
        right_to_left_layout: attr(ppr.right_to_left_layout(interner))?,
        adjust_right_indent_for_document_grid: attr(
            ppr.adjust_right_indent_for_document_grid(interner),
        )?,
        snap_to_grid: attr(ppr.snap_to_grid(interner))?,
        contextual_spacing: attr(ppr.contextual_spacing(interner))?,
        mirror_indents: attr(ppr.mirror_indents(interner))?,
        suppress_overlap: attr(ppr.suppress_overlap(interner))?,
        frame,
        numbering,
        borders,
        shading,
        tab_stops,
        spacing,
        indentation,
        alignment: ppr
            .alignment()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        text_direction: ppr
            .text_direction()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        vertical_character_alignment: ppr
            .vertical_character_alignment()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        text_box_tight_wrap: ppr
            .text_box_tight_wrap()
            .map(|value| attr(value.value(interner)))
            .transpose()?,
        outline_level: attr(ppr.outline_level(interner))?,
        associated_html_div_id: attr(ppr.associated_html_div_id(interner))?,
        conditional_formatting,
    })
}

fn extract_numbering_reference(
    value: &NumberingProperties,
    interner: &Interner,
) -> Result<Option<EffectiveNumberingReference>, DocxError> {
    let Some(numbering_id) = attr(value.numbering_id(interner))? else {
        return Ok(None);
    };
    let level = attr(value.level(interner))?.unwrap_or(0);
    Ok(Some(EffectiveNumberingReference {
        numbering_id,
        level,
    }))
}

fn extract_paragraph_borders(
    value: &ParagraphBorders,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveParagraphBorders, DocxError> {
    Ok(EffectiveParagraphBorders {
        top: value
            .top()
            .map(|b| extract_border(b, theme, interner))
            .transpose()?,
        left: value
            .left()
            .map(|b| extract_border(b, theme, interner))
            .transpose()?,
        bottom: value
            .bottom()
            .map(|b| extract_border(b, theme, interner))
            .transpose()?,
        right: value
            .right()
            .map(|b| extract_border(b, theme, interner))
            .transpose()?,
        between: value
            .between()
            .map(|b| extract_border(b, theme, interner))
            .transpose()?,
        bar: value
            .bar()
            .map(|b| extract_border(b, theme, interner))
            .transpose()?,
    })
}

fn extract_tab_stops(
    value: &TabStops,
    interner: &Interner,
) -> Result<Vec<EffectiveTabStop>, DocxError> {
    fn extract_one(tab: &TabStop, interner: &Interner) -> Result<EffectiveTabStop, DocxError> {
        Ok(EffectiveTabStop {
            alignment: attr(tab.alignment(interner))?,
            leader: attr(tab.leader(interner))?,
            position: attr(tab.position(interner))?,
        })
    }
    value.tabs().map(|tab| extract_one(tab, interner)).collect()
}

fn extract_spacing(value: &Spacing, interner: &Interner) -> Result<EffectiveSpacing, DocxError> {
    Ok(EffectiveSpacing {
        before: attr(value.before(interner))?,
        before_lines: attr(value.before_lines(interner))?,
        before_autospacing: attr(value.before_autospacing(interner))?,
        after: attr(value.after(interner))?,
        after_lines: attr(value.after_lines(interner))?,
        after_autospacing: attr(value.after_autospacing(interner))?,
        line: attr(value.line_spacing(interner))?,
    })
}

fn extract_indentation(
    value: &Indentation,
    interner: &Interner,
) -> Result<EffectiveIndentation, DocxError> {
    Ok(EffectiveIndentation {
        start: attr(value.start(interner))?,
        start_chars: attr(value.start_chars(interner))?,
        end: attr(value.end(interner))?,
        end_chars: attr(value.end_chars(interner))?,
        left: attr(value.left(interner))?,
        left_chars: attr(value.left_chars(interner))?,
        right: attr(value.right(interner))?,
        right_chars: attr(value.right_chars(interner))?,
        hanging: attr(value.hanging(interner))?,
        hanging_chars: attr(value.hanging_chars(interner))?,
        first_line: attr(value.first_line(interner))?,
        first_line_chars: attr(value.first_line_chars(interner))?,
    })
}

fn extract_frame_properties(
    value: &FrameProperties,
    interner: &Interner,
) -> Result<EffectiveFrameProperties, DocxError> {
    Ok(EffectiveFrameProperties {
        drop_cap: attr(value.drop_cap(interner))?,
        drop_cap_lines: attr(value.drop_cap_lines(interner))?,
        width: attr(value.width(interner))?,
        height: attr(value.height(interner))?,
        vertical_spacing: attr(value.vertical_spacing(interner))?,
        horizontal_spacing: attr(value.horizontal_spacing(interner))?,
        wrap: attr(value.wrap(interner))?,
        horizontal_anchor: attr(value.horizontal_anchor(interner))?,
        vertical_anchor: attr(value.vertical_anchor(interner))?,
        x: attr(value.x(interner))?,
        x_alignment: attr(value.x_alignment(interner))?,
        y: attr(value.y(interner))?,
        y_alignment: attr(value.y_alignment(interner))?,
        height_rule: attr(value.height_rule(interner))?,
        anchor_lock: attr(value.anchor_lock(interner))?,
    })
}

fn extract_conditional_formatting(
    value: &ConditionalFormatting,
    interner: &Interner,
) -> Result<EffectiveConditionalFormatting, DocxError> {
    Ok(EffectiveConditionalFormatting {
        bitmask: attr(value.bitmask(interner))?.map(|mask| mask.to_wire().to_owned()),
        first_row: attr(value.first_row(interner))?,
        last_row: attr(value.last_row(interner))?,
        first_column: attr(value.first_column(interner))?,
        last_column: attr(value.last_column(interner))?,
        odd_vertical_band: attr(value.odd_vertical_band(interner))?,
        even_vertical_band: attr(value.even_vertical_band(interner))?,
        odd_horizontal_band: attr(value.odd_horizontal_band(interner))?,
        even_horizontal_band: attr(value.even_horizontal_band(interner))?,
        first_row_first_column: attr(value.first_row_first_column(interner))?,
        first_row_last_column: attr(value.first_row_last_column(interner))?,
        last_row_first_column: attr(value.last_row_first_column(interner))?,
        last_row_last_column: attr(value.last_row_last_column(interner))?,
    })
}

// -------------------------------------------------------------------------------------------
// The per-call chain cache — see the guide's "Cost and caching" section for the full account of
// what this does and does not persist, and what would invalidate it if it did.
// -------------------------------------------------------------------------------------------

/// Memoizes [`StyleIndex::based_on_chain`] by `styleId` for the lifetime of one `effective_*` call —
/// so that resolving all 38 (or 32) fields of one ladder walks each relevant chain exactly once, never
/// once per field. See [the guide](crate::effective_properties)'s own "Cost and caching" section for
/// why this cache does not (and, given [`Interner`] is not [`Clone`], cannot without a larger change
/// to [`Document`]'s own re-parse-per-call architecture) survive across separate `effective_*` calls,
/// and what a caller who wants that would do instead.
struct ChainCache<'a> {
    style_index: &'a StyleIndex<'a>,
    interner: &'a Interner,
    chains: RefCell<HashMap<String, Vec<&'a StyleDefinition>>>,
}

impl<'a> ChainCache<'a> {
    fn new(style_index: &'a StyleIndex<'a>, interner: &'a Interner) -> Self {
        Self {
            style_index,
            interner,
            chains: RefCell::new(HashMap::new()),
        }
    }

    /// The `w:basedOn` chain starting at `style_id`, memoized. `Ok(&[])` — not an error — when
    /// `style_id` itself is not in this style sheet, mirroring how an absent `w:pStyle`/`w:rStyle`
    /// already contributes an empty tier: a dangling style reference degrades this one tier to "says
    /// nothing" rather than failing the whole ladder read.
    fn chain(&self, style_id: &str) -> Result<Vec<&'a StyleDefinition>, DocxError> {
        if let Some(hit) = self.chains.borrow().get(style_id) {
            return Ok(hit.clone());
        }
        let chain = match self.style_index.based_on_chain(style_id, self.interner) {
            Ok(chain) => chain,
            Err(DocxError::UnknownStyleId(_)) => Vec::new(),
            Err(other) => return Err(other),
        };
        self.chains
            .borrow_mut()
            .insert(style_id.to_owned(), chain.clone());
        Ok(chain)
    }
}

/// Folds `extract` over `chain` (index `0` = the leaf style, highest priority within the chain) via
/// [`EffectiveCharacterProperties::merge_under`] — leaf wins per field, falling back down the chain
/// exactly as ECMA-376 Part 1 §17.7.1 describes. An empty chain (no style stated, or a dangling
/// reference) contributes the all-`None` default, i.e. nothing.
fn merge_character_chain(
    chain: &[&StyleDefinition],
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveCharacterProperties, DocxError> {
    let mut result = EffectiveCharacterProperties::default();
    for style in chain {
        let Some(rpr) = style.run_properties() else {
            continue;
        };
        let contribution = extract_run_properties(rpr, theme, interner)?;
        result = result.merge_under(&contribution);
    }
    Ok(result)
}

fn merge_paragraph_chain(
    chain: &[&StyleDefinition],
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveParagraphProperties, DocxError> {
    let mut result = EffectiveParagraphProperties::default();
    for style in chain {
        let Some(ppr) = style.paragraph_properties() else {
            continue;
        };
        let contribution = extract_style_paragraph_properties(ppr, theme, interner)?;
        result = result.merge_under(&contribution);
    }
    Ok(result)
}

/// The first `w:numPr` a paragraph-style chain states (leaf first), or `None` if none of them do.
fn numbering_reference_from_chain(
    chain: &[&StyleDefinition],
    interner: &Interner,
) -> Result<Option<EffectiveNumberingReference>, DocxError> {
    for style in chain {
        let Some(ppr) = style.paragraph_properties() else {
            continue;
        };
        let Some(numbering) = ppr.numbering() else {
            continue;
        };
        if let Some(reference) = extract_numbering_reference(numbering, interner)? {
            return Ok(Some(reference));
        }
    }
    Ok(None)
}

// -------------------------------------------------------------------------------------------
// Document::effective_run_properties / effective_paragraph_properties
// -------------------------------------------------------------------------------------------

/// What `word/document.xml` alone states about one run: its own direct `w:rPr` (already resolved,
/// since document.xml's own interner never needs to leave this block), the character/paragraph
/// style ids it names (plain owned strings — interner-independent once extracted, so they may
/// freely cross into `styles.xml`'s own interner afterwards), and its paragraph's own `w:numPr`.
struct DirectRunContext {
    direct: EffectiveCharacterProperties,
    character_style_id: Option<String>,
    paragraph_style_id: Option<String>,
    own_numbering: Option<EffectiveNumberingReference>,
}

impl Document {
    /// The theme this document relates to, resolved once, interner-free — `None` (both fields) if it
    /// relates to no `word/theme/themeN.xml` at all.
    fn load_theme_context(&mut self) -> Result<ThemeContext, DocxError> {
        let Some(theme_part) = self.parts.theme.clone() else {
            return Ok(ThemeContext::empty());
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = mjx_dml::Theme::from_xml(&doc.root, &doc.interner)?;
        let colors = theme
            .color_scheme()
            .map(|scheme| SchemeColors::from_scheme(scheme, &doc.interner));
        let fonts = theme.font_scheme().cloned();
        Ok(ThemeContext {
            colors,
            fonts,
            scratch: RefCell::new(Interner::new()),
        })
    }

    /// `word/document.xml` alone: the run's direct `w:rPr`, the character/paragraph style ids its
    /// direct formatting names, and its paragraph's own `w:numPr`.
    fn direct_run_context(
        &mut self,
        paragraph_path: &BlockPath,
        run_path: &RunPath,
        theme: &ThemeContext,
    ) -> Result<DirectRunContext, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let paragraph = body.paragraph(paragraph_path.clone()).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        let run = paragraph
            .run(run_path.clone())
            .ok_or_else(|| DocxError::AddressNotFound(format!("no run at {run_path}")))?;

        let direct = match run.run_properties() {
            Some(rpr) => extract_run_properties(rpr, theme, &doc.interner)?,
            None => EffectiveCharacterProperties::default(),
        };
        let character_style_id = run
            .run_properties()
            .and_then(RunProperties::character_style)
            .map(|reference| attr(reference.style_id(&doc.interner)))
            .transpose()?
            .map(Cow::into_owned);
        let paragraph_style_id = paragraph
            .properties()
            .and_then(ParagraphProperties::style)
            .map(|reference| attr(reference.style_id(&doc.interner)))
            .transpose()?
            .map(Cow::into_owned);
        let own_numbering = paragraph
            .properties()
            .and_then(ParagraphProperties::numbering)
            .map(|value| extract_numbering_reference(value, &doc.interner))
            .transpose()?
            .flatten();

        Ok(DirectRunContext {
            direct,
            character_style_id,
            paragraph_style_id,
            own_numbering,
        })
    }

    /// The **effective** character formatting of the run at `run` within the paragraph at
    /// `paragraph` — every `EG_RPrBase` member resolved across the full ladder: `w:docDefaults` →
    /// the numbering level's own `w:rPr` (when the paragraph is in a list) → the paragraph style's
    /// `w:basedOn` chain (root to leaf) → the character style's `w:basedOn` chain (root to leaf) →
    /// this run's own direct `w:rPr`. See [the guide](crate::effective_properties) for why this order
    /// — verified against ECMA-376 Part 1 §17.7.2 — corrects the order this ticket's own brief
    /// assumed (numbering before the paragraph-style chain, not after it).
    ///
    /// Colours are baked to concrete `RRGGBB` through this document's theme where the source names a
    /// `themeColor`/theme-font reference; a reference the theme does not define keeps its unresolved
    /// form (see [`EffectiveColor`]/[`EffectiveFonts`]).
    ///
    /// Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, [`DocxError::AddressNotFound`]
    /// if either address does not resolve, [`DocxError::BasedOnChainTooDeep`] if a style chain does
    /// not terminate, or another [`DocxError`] if a related part cannot be read.
    pub fn effective_run_properties(
        &mut self,
        paragraph: impl Into<BlockPath>,
        run: impl Into<RunPath>,
    ) -> Result<EffectiveCharacterProperties, DocxError> {
        let paragraph_path = paragraph.into();
        let run_path = run.into();

        let theme = self.load_theme_context()?;
        let direct_context = self.direct_run_context(&paragraph_path, &run_path, &theme)?;

        let paragraph_style_id = direct_context.paragraph_style_id.clone();
        let character_style_id = direct_context.character_style_id.clone();

        let style_results = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let style_index = StyleIndex::build(sheet, interner)?;
            let cache = ChainCache::new(&style_index, interner);

            let doc_defaults = sheet
                .document_defaults()
                .and_then(super::styles::DocumentDefaults::run_properties_default)
                .and_then(super::styles::DefaultRunProperties::run_properties)
                .map(|rpr| extract_run_properties(rpr, &theme, interner))
                .transpose()?
                .unwrap_or_default();

            let paragraph_chain = match &paragraph_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };
            let character_chain = match &character_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };

            let paragraph_tier = merge_character_chain(&paragraph_chain, &theme, interner)?;
            let character_tier = merge_character_chain(&character_chain, &theme, interner)?;
            let numbering_reference = numbering_reference_from_chain(&paragraph_chain, interner)?;

            Ok((
                doc_defaults,
                paragraph_tier,
                character_tier,
                numbering_reference,
            ))
        })?;
        let (doc_defaults, paragraph_tier, character_tier, style_numbering) = match style_results {
            Some(result) => result?,
            None => (
                EffectiveCharacterProperties::default(),
                EffectiveCharacterProperties::default(),
                EffectiveCharacterProperties::default(),
                None,
            ),
        };

        let numbering_reference = direct_context.own_numbering.or(style_numbering);
        let numbering_effective = match numbering_reference {
            Some(reference) => self
                .resolve_numbering(
                    reference.numbering_id,
                    reference.level,
                    |lookup, interner| -> Result<_, DocxError> {
                        match lookup {
                            NumberingLookup::Resolved(resolution) => resolution
                                .level()
                                .and_then(NumberingLevel::run_properties)
                                .map(|rpr| extract_run_properties(rpr, &theme, interner))
                                .transpose(),
                            NumberingLookup::None => Ok(None),
                        }
                    },
                )??
                .unwrap_or_default(),
            None => EffectiveCharacterProperties::default(),
        };

        let mut merged = direct_context
            .direct
            .merge_under(&character_tier)
            .merge_under(&paragraph_tier)
            .merge_under(&numbering_effective)
            .merge_under(&doc_defaults);
        recombine_toggles(
            &mut merged,
            &direct_context.direct,
            &doc_defaults,
            &numbering_effective,
            &paragraph_tier,
            &character_tier,
        );
        Ok(merged)
    }

    /// The **effective** paragraph formatting of the paragraph at `paragraph` — every `CT_PPrBase`
    /// member resolved across the ladder: `w:docDefaults` → the numbering level's own `w:pPr` (when
    /// the paragraph is in a list) → the paragraph style's `w:basedOn` chain (root to leaf) → this
    /// paragraph's own direct `w:pPr`. There is no character-style tier here — `w:rStyle` affects a
    /// *run's* character formatting, never a paragraph's own layout.
    ///
    /// See [the guide](crate::effective_properties) for the full ladder order, verified against
    /// ECMA-376 Part 1 §17.7.2, and where this reader stops (computed list numbers, among others).
    ///
    /// Reading does not dirty any part.
    ///
    /// # Errors
    /// As [`effective_run_properties`](Self::effective_run_properties).
    pub fn effective_paragraph_properties(
        &mut self,
        paragraph: impl Into<BlockPath>,
    ) -> Result<EffectiveParagraphProperties, DocxError> {
        let paragraph_path = paragraph.into();

        let theme = self.load_theme_context()?;

        let (direct, paragraph_style_id, own_numbering) = {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            let paragraph = body.paragraph(paragraph_path.clone()).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
            let direct = match paragraph.properties() {
                Some(ppr) => extract_paragraph_properties(ppr, &theme, &doc.interner)?,
                None => EffectiveParagraphProperties::default(),
            };
            let paragraph_style_id = paragraph
                .properties()
                .and_then(ParagraphProperties::style)
                .map(|reference| attr(reference.style_id(&doc.interner)))
                .transpose()?
                .map(Cow::into_owned);
            let own_numbering = paragraph
                .properties()
                .and_then(ParagraphProperties::numbering)
                .map(|value| extract_numbering_reference(value, &doc.interner))
                .transpose()?
                .flatten();
            (direct, paragraph_style_id, own_numbering)
        };

        let style_results = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let style_index = StyleIndex::build(sheet, interner)?;
            let cache = ChainCache::new(&style_index, interner);

            let doc_defaults = sheet
                .document_defaults()
                .and_then(super::styles::DocumentDefaults::paragraph_properties_default)
                .and_then(super::styles::DefaultParagraphProperties::paragraph_properties)
                .map(|ppr| extract_style_paragraph_properties(ppr, &theme, interner))
                .transpose()?
                .unwrap_or_default();

            let paragraph_chain = match &paragraph_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };
            let paragraph_tier = merge_paragraph_chain(&paragraph_chain, &theme, interner)?;
            let numbering_reference = numbering_reference_from_chain(&paragraph_chain, interner)?;

            Ok((doc_defaults, paragraph_tier, numbering_reference))
        })?;
        let (doc_defaults, paragraph_tier, style_numbering) = match style_results {
            Some(result) => result?,
            None => (
                EffectiveParagraphProperties::default(),
                EffectiveParagraphProperties::default(),
                None,
            ),
        };

        let numbering_reference = own_numbering.or(style_numbering);
        let numbering_effective = match numbering_reference {
            Some(reference) => self
                .resolve_numbering(
                    reference.numbering_id,
                    reference.level,
                    |lookup, interner| -> Result<_, DocxError> {
                        match lookup {
                            NumberingLookup::Resolved(resolution) => resolution
                                .level()
                                .and_then(NumberingLevel::paragraph_properties)
                                .map(|ppr| {
                                    extract_style_paragraph_properties(ppr, &theme, interner)
                                })
                                .transpose(),
                            NumberingLookup::None => Ok(None),
                        }
                    },
                )??
                .unwrap_or_default(),
            None => EffectiveParagraphProperties::default(),
        };

        Ok(direct
            .merge_under(&paragraph_tier)
            .merge_under(&numbering_effective)
            .merge_under(&doc_defaults))
    }
}
