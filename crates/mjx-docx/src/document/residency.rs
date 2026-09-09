//! [`DocumentFormatting`] — the whole document read **once**, so that laying it out is not
//! quadratic.
//!
//! # The measurement that made this exist
//!
//! [`Document::effective_paragraph_properties`] and [`Document::effective_run_properties`] are
//! correct and are shaped for a caller asking about *one* paragraph. Each call re-parses
//! `word/document.xml`, re-parses `word/styles.xml`, rebuilds the whole [`StyleIndex`], rebuilds a
//! chain cache it then throws away, and re-parses `word/theme/theme1.xml`. That is stated in the
//! guide's own "Cost and caching" section, and it is the right trade for a caller that asks once.
//!
//! A **flow layout engine** asks once per paragraph, for every paragraph, and a two-hundred-page
//! document holds thousands of them. The cost of laying one out would then be the number of
//! paragraphs times the size of the document — and the crate above it (`mjx-layout-docx`,
//! MJXOFF-174) would have had no way out except to re-derive the ladder itself, which is the one
//! thing it must not do.
//!
//! So this is the same answer `mjx-xlsx` already gives for a worksheet (`SheetFormatting`: the
//! styles part and one sheet, parsed once, resolved many times): **read once, resolve many**. Every
//! part is parsed once, the style index and the chain cache are built once, each distinct
//! `(w:numId, w:ilvl)` pair is resolved once however many paragraphs share it, and the theme is
//! loaded once.
//!
//! # This adds no resolution of its own
//!
//! The ladder's *order* is stated in exactly one place — `combine_paragraph_tiers` and
//! `combine_run_tiers` in `effective.rs` — and both orchestrations call it. The extraction of a
//! single rung is `extract_paragraph_properties` / `extract_run_properties`, which every rung
//! already shared before this file existed. `crates/mjx-docx/tests/residency.rs` asserts the two
//! orchestrations agree paragraph by paragraph and run by run over the committed corpus, which is
//! what makes "one implementation" a checked claim rather than a comment.
//!
//! # What the text is, and why it is not [`Paragraph::text`]
//!
//! [`Paragraph::text`] concatenates `w:t` and nothing else, which is right for a caller reading what
//! a paragraph *says*. A renderer needs what a paragraph *is*: a tab occupies horizontal space, a
//! `w:br` ends a line, a soft hyphen is an authored hyphenation point and a non-breaking hyphen is a
//! hyphen that may not be broken at. So [`ParagraphFormatting::text`] carries
//!
//! | markup | character |
//! |---|---|
//! | `w:t` | its own text |
//! | `w:tab` | `U+0009 TAB` |
//! | `w:br` (any `@type`) | `U+000A LINE FEED` |
//! | `w:cr` | `U+000A LINE FEED` |
//! | `w:softHyphen` | `U+00AD SOFT HYPHEN` |
//! | `w:noBreakHyphen` | `U+2011 NON-BREAKING HYPHEN` |
//!
//! and nothing else. The two hyphens are not an interpretation: UAX #14 gives `U+00AD` class `BA`
//! (break after) and `U+2011` class `GL` (non-breaking), which is exactly what the two elements
//! mean, so the line breaker gets them right without anything here teaching it.
//!
//! A `w:br@type="page"` or `="column"` is *also* recorded as a [`HardBreak`], because ending a line
//! and ending a page are different facts and the line feed carries only the first.
//!
//! **`w:sym` contributes nothing**, and that is reported rather than guessed at. A symbol is a
//! character code in a *named font* — the pair is the content — so a renderer that dropped the font
//! would draw a different glyph while looking entirely plausible.

use std::collections::HashMap;
use std::ops::Range;

use mjx_ooxml_core::{FromXml, Interner};
use mjx_ooxml_types::wordprocessingml::BreakType;

use super::body::{Paragraph, ParagraphContent, Run, RunInnerContent};
use super::effective::{
    attr, combine_paragraph_tiers, combine_run_tiers, extract_numbering_reference,
    extract_paragraph_properties, extract_run_properties, extract_style_paragraph_properties,
    merge_character_chain, merge_paragraph_chain, numbering_reference_from_chain, ChainCache,
    EffectiveCharacterProperties, EffectiveNumberingReference, EffectiveParagraphProperties,
    ThemeContext,
};
use super::numbering::NumberingLookup;
use super::paragraph_properties::ParagraphProperties;
use super::run_properties::RunProperties;
use super::sections::SectionProperties;
use super::styles::{
    DefaultParagraphProperties, DefaultRunProperties, DocumentDefaults, StyleSheet,
};
use super::{Document, MainDocument, StyleIndex};
use crate::address::RunPath;
use crate::error::DocxError;
use crate::page::{PageMargins, PageSize};

/// `U+00AD SOFT HYPHEN` — what `w:softHyphen` is, in one character.
pub const SOFT_HYPHEN: char = '\u{00AD}';

/// `U+2011 NON-BREAKING HYPHEN` — what `w:noBreakHyphen` is, in one character.
pub const NON_BREAKING_HYPHEN: char = '\u{2011}';

/// One run of a paragraph, with its formatting already resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct RunFormatting {
    /// The bytes of [`ParagraphFormatting::text`] it covers.
    pub range: Range<usize>,
    /// Where it is in the paragraph, when [`Paragraph::run`] can address it.
    ///
    /// `None` for a run inside a `w:fldSimple`: [`Paragraph::text`] descends into one and
    /// [`Paragraph::run_count`]'s own slot walk does not, so such a run has text but no address.
    /// Reported rather than papered over — a path invented here would resolve to a different run.
    pub path: Option<RunPath>,
    /// Every `EG_RPrBase` member, resolved across the whole ladder.
    pub properties: EffectiveCharacterProperties,
}

/// A `w:br` that ends more than a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardBreak {
    /// The byte of [`ParagraphFormatting::text`] the break's own line feed sits at.
    pub at: usize,
    /// `w:br@type` — [`BreakType::Page`] or [`BreakType::Column`]. A `textWrapping` break is only a
    /// line feed and is not recorded here.
    pub kind: BreakType,
}

/// One paragraph, with everything a box model needs and nothing it would have to re-derive.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphFormatting {
    text: String,
    runs: Vec<RunFormatting>,
    hard_breaks: Vec<HardBreak>,
    properties: EffectiveParagraphProperties,
    style_id: Option<String>,
}

impl ParagraphFormatting {
    /// What the paragraph reads as. See this module's own documentation for the six markup elements
    /// that contribute a character.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its runs, in document order, covering [`ParagraphFormatting::text`] without gaps or overlaps.
    #[must_use]
    pub fn runs(&self) -> &[RunFormatting] {
        &self.runs
    }

    /// Every page or column break inside it, in order.
    #[must_use]
    pub fn hard_breaks(&self) -> &[HardBreak] {
        &self.hard_breaks
    }

    /// Every `CT_PPrBase` member, resolved across the whole ladder.
    #[must_use]
    pub fn properties(&self) -> &EffectiveParagraphProperties {
        &self.properties
    }

    /// The `w:pStyle` it names, if any — the **id**, not the resolved values.
    ///
    /// The ladder resolves a style into values and then has no further use for its name, so this is
    /// the one thing about a style that survives resolution. `w:contextualSpacing` is why it has
    /// to: *"don't add space between paragraphs of the same style"* is a question about two
    /// paragraphs' **identity**, which no amount of resolved values can answer — two paragraphs of
    /// different styles that happen to resolve alike are not the same style.
    #[must_use]
    pub fn style_id(&self) -> Option<&str> {
        self.style_id.as_deref()
    }
}

/// One section's page geometry, in plain numbers.
///
/// The `w:sectPr` itself is [`SectionProperties`] and needs an [`Interner`] to read; this is what it
/// resolves to, so a caller holding a [`DocumentFormatting`] does not have to hold an interner
/// beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionFormatting {
    /// The first paragraph index this section governs.
    pub first_paragraph: usize,
    /// The last, inclusive, or `None` when it governs no paragraph.
    pub last_paragraph: Option<usize>,
    /// `w:pgSz`, or `None` when the section states none.
    pub page_size: Option<PageSize>,
    /// `w:pgMar`, or `None` when the section states none.
    pub page_margins: Option<PageMargins>,
}

/// The document settings a layout engine reads: hyphenation, and the default tab interval.
///
/// Every field is the value **in force**, with the schema default already applied, so a consumer
/// never has to know which of them default to on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentLayoutSettings {
    /// `w:autoHyphenation` — off unless the document says otherwise.
    pub auto_hyphenation: bool,
    /// `w:doNotHyphenateCaps`.
    pub do_not_hyphenate_capitals: bool,
    /// `w:consecutiveHyphenLimit` — how many consecutive lines may end in a hyphen. `None` is "no
    /// limit", which is what an absent element means.
    pub consecutive_hyphen_limit: Option<i64>,
    /// `w:hyphenationZone` in twips — how close to the right margin a line must come before
    /// hyphenation is attempted at all. `None` when the document states none.
    pub hyphenation_zone_twips: Option<i64>,
    /// `w:defaultTabStop` in twips — the interval of the implicit tab grid past the last stated
    /// stop.
    pub default_tab_stop_twips: i64,
}

impl DocumentLayoutSettings {
    /// `w:defaultTabStop`'s value when the document states none.
    ///
    /// Half an inch. ECMA-376 gives `w:defaultTabStop` **no** schema default, and this is the value
    /// Word's own `Normal.dotm` writes into every document it creates — a fact about Word rather
    /// than about the specification, which is why it is named here rather than buried in an
    /// `unwrap_or`.
    pub const DEFAULT_TAB_STOP_TWIPS: i64 = 720;
}

impl Default for DocumentLayoutSettings {
    fn default() -> Self {
        Self {
            auto_hyphenation: false,
            do_not_hyphenate_capitals: false,
            consecutive_hyphen_limit: None,
            hyphenation_zone_twips: None,
            default_tab_stop_twips: Self::DEFAULT_TAB_STOP_TWIPS,
        }
    }
}

/// A whole document, parsed once and resolved once: what a box model lays out.
///
/// Owns nothing borrowed from the [`Document`] it came from, so a caller may keep one while the
/// document is edited elsewhere — and it can never write, so keeping one cannot make the package
/// stale in the other direction. It is a **snapshot**: an edit to the document does not reach it,
/// and the caller re-reads.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentFormatting {
    paragraphs: Vec<ParagraphFormatting>,
    sections: Vec<SectionFormatting>,
    settings: DocumentLayoutSettings,
}

impl DocumentFormatting {
    /// Every paragraph of the body, in document order.
    ///
    /// Paragraphs inside a table are **not** here: a table is a different layout discipline and its
    /// own child (MJXOFF-176, R21). [`super::body::Body::paragraphs`] is what this walks, and that
    /// walks the body's top level.
    #[must_use]
    pub fn paragraphs(&self) -> &[ParagraphFormatting] {
        &self.paragraphs
    }

    /// Every section, in document order.
    #[must_use]
    pub fn sections(&self) -> &[SectionFormatting] {
        &self.sections
    }

    /// The settings in force.
    #[must_use]
    pub fn settings(&self) -> &DocumentLayoutSettings {
        &self.settings
    }

    /// Which section governs the paragraph at `paragraph`, or `None` when none does.
    #[must_use]
    pub fn section_of(&self, paragraph: usize) -> Option<&SectionFormatting> {
        self.sections.iter().find(|section| {
            section
                .last_paragraph
                .is_some_and(|last| paragraph >= section.first_paragraph && paragraph <= last)
        })
    }
}

/// What `word/document.xml` alone states about one paragraph, before any style is consulted.
struct DirectParagraph {
    text: String,
    runs: Vec<DirectRun>,
    hard_breaks: Vec<HardBreak>,
    direct: EffectiveParagraphProperties,
    style_id: Option<String>,
    own_numbering: Option<EffectiveNumberingReference>,
}

/// The same for one run.
struct DirectRun {
    range: Range<usize>,
    path: Option<RunPath>,
    direct: EffectiveCharacterProperties,
    character_style_id: Option<String>,
}

impl Document {
    /// Reads the whole document once and resolves every paragraph's and run's effective formatting.
    ///
    /// This is the reader a **layout engine** uses; [`Document::effective_paragraph_properties`] is
    /// the reader a caller asking one question uses. They compute the same thing (see this module's
    /// own documentation, and `tests/residency.rs`, which asserts it); they differ only in what
    /// they re-parse.
    ///
    /// Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::BasedOnChainTooDeep`] if a style chain does not terminate, or another
    /// [`DocxError`] if a related part cannot be read.
    pub fn formatting(&mut self) -> Result<DocumentFormatting, DocxError> {
        let theme = self.load_theme_context()?;
        let (direct_paragraphs, sections) = self.read_direct(&theme)?;
        let settings = self.read_layout_settings()?;

        let resolved = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let index = StyleIndex::build(sheet, interner)?;
            let cache = ChainCache::new(&index, interner);
            let defaults = LadderDefaults::read(sheet, &theme, interner)?;
            let mut tiers = Vec::with_capacity(direct_paragraphs.len());
            for paragraph in &direct_paragraphs {
                tiers.push(ParagraphTiers::read(paragraph, &cache, &theme, interner)?);
            }
            Ok((defaults, tiers))
        })?;
        let (defaults, tiers) = match resolved {
            Some(result) => result?,
            None => (
                LadderDefaults::default(),
                direct_paragraphs
                    .iter()
                    .map(|paragraph| ParagraphTiers::empty(paragraph.runs.len()))
                    .collect(),
            ),
        };

        // Every distinct list the document reaches, resolved once each rather than once per
        // paragraph: a hundred bullets in one list is one resolution, not a hundred.
        let mut numbering_effects: HashMap<(i64, i64), NumberingTier> = HashMap::new();
        for (paragraph, tier) in direct_paragraphs.iter().zip(&tiers) {
            let Some(reference) = paragraph.own_numbering.or(tier.style_numbering) else {
                continue;
            };
            let key = (reference.numbering_id, reference.level);
            if numbering_effects.contains_key(&key) {
                continue;
            }
            let resolved = self.numbering_tier(&theme, reference)?;
            numbering_effects.insert(key, resolved);
        }

        let mut paragraphs = Vec::with_capacity(direct_paragraphs.len());
        for (paragraph, tier) in direct_paragraphs.into_iter().zip(tiers) {
            let numbering = paragraph
                .own_numbering
                .or(tier.style_numbering)
                .and_then(|reference| {
                    numbering_effects.get(&(reference.numbering_id, reference.level))
                })
                .cloned()
                .unwrap_or_default();
            let properties = combine_paragraph_tiers(
                &paragraph.direct,
                &tier.paragraph,
                &numbering.paragraph,
                &defaults.paragraph,
            );
            let runs = paragraph
                .runs
                .into_iter()
                .zip(tier.characters)
                .map(|(run, character_tier)| RunFormatting {
                    range: run.range,
                    path: run.path,
                    properties: combine_run_tiers(
                        &run.direct,
                        &character_tier,
                        &tier.character_from_paragraph_style,
                        &numbering.character,
                        &defaults.character,
                        // No table style applies outside a table cell; a table's own paragraphs are
                        // R21's, and this reader does not descend into one.
                        &EffectiveCharacterProperties::default(),
                    ),
                })
                .collect();
            paragraphs.push(ParagraphFormatting {
                text: paragraph.text,
                runs,
                hard_breaks: paragraph.hard_breaks,
                properties,
                style_id: paragraph.style_id,
            });
        }

        Ok(DocumentFormatting {
            paragraphs,
            sections,
            settings,
        })
    }

    /// One parse of `word/document.xml`: every paragraph's text, runs, direct formatting and style
    /// references, and the section spans, resolved to plain numbers.
    fn read_direct(
        &mut self,
        theme: &ThemeContext,
    ) -> Result<(Vec<DirectParagraph>, Vec<SectionFormatting>), DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let interner = &doc.interner;

        let mut paragraphs = Vec::new();
        for paragraph in body.paragraphs() {
            paragraphs.push(read_direct_paragraph(paragraph, theme, interner)?);
        }

        let mut sections = Vec::new();
        for span in super::sections::sections_in(body) {
            let (page_size, page_margins) = match span.properties.as_ref() {
                Some(properties) => (
                    read_page_size(properties, interner)?,
                    read_page_margins(properties, interner)?,
                ),
                None => (None, None),
            };
            sections.push(SectionFormatting {
                first_paragraph: span.first_paragraph,
                last_paragraph: span.last_paragraph,
                page_size,
                page_margins,
            });
        }
        Ok((paragraphs, sections))
    }

    /// `word/settings.xml`'s layout-relevant half, with every schema default applied.
    fn read_layout_settings(&mut self) -> Result<DocumentLayoutSettings, DocxError> {
        let read = self.document_settings(|settings, interner| -> Result<_, DocxError> {
            Ok(DocumentLayoutSettings {
                auto_hyphenation: attr(settings.auto_hyphenation(interner))?.unwrap_or(false),
                do_not_hyphenate_capitals: attr(settings.do_not_hyphenate_caps(interner))?
                    .unwrap_or(false),
                consecutive_hyphen_limit: settings
                    .consecutive_hyphen_limit()
                    .map(|value| attr(value.value(interner)))
                    .transpose()?,
                hyphenation_zone_twips: settings
                    .hyphenation_zone()
                    .map(|value| attr(value.twips(interner)))
                    .transpose()?
                    .and_then(|measure| {
                        mjx_ooxml_types::support::universal_measure::twips_from_wire(
                            measure.to_wire(),
                        )
                    }),
                default_tab_stop_twips: attr(settings.default_tab_stop_twips(interner))?
                    .and_then(|measure| {
                        mjx_ooxml_types::support::universal_measure::twips_from_wire(
                            measure.to_wire(),
                        )
                    })
                    .unwrap_or(DocumentLayoutSettings::DEFAULT_TAB_STOP_TWIPS),
            })
        })?;
        match read {
            Some(result) => result,
            None => Ok(DocumentLayoutSettings::default()),
        }
    }

    /// The numbering level `reference` names, as the two tiers it contributes to the ladder.
    fn numbering_tier(
        &mut self,
        theme: &ThemeContext,
        reference: EffectiveNumberingReference,
    ) -> Result<NumberingTier, DocxError> {
        let resolved = self.resolve_numbering(
            reference.numbering_id,
            reference.level,
            |lookup, interner| -> Result<NumberingTier, DocxError> {
                let NumberingLookup::Resolved(resolution) = lookup else {
                    return Ok(NumberingTier::default());
                };
                let Some(level) = resolution.level() else {
                    return Ok(NumberingTier::default());
                };
                Ok(NumberingTier {
                    paragraph: level
                        .paragraph_properties()
                        .map(|ppr| extract_style_paragraph_properties(ppr, theme, interner))
                        .transpose()?
                        .unwrap_or_default(),
                    character: level
                        .run_properties()
                        .map(|rpr| extract_run_properties(rpr, theme, interner))
                        .transpose()?
                        .unwrap_or_default(),
                })
            },
        );
        match resolved {
            Ok(inner) => inner,
            // A `w:numPr` naming a list the document does not define is a defect in the document,
            // not a reason to refuse to lay the document out: the paragraph loses its list
            // formatting and keeps its text. `effective_paragraph_properties` propagates the error
            // because its caller asked about that one paragraph; a whole-document read cannot.
            Err(DocxError::UnknownNumberingId(_)) => Ok(NumberingTier::default()),
            Err(other) => Err(other),
        }
    }
}

/// `w:docDefaults`, both halves, read once.
#[derive(Default)]
struct LadderDefaults {
    paragraph: EffectiveParagraphProperties,
    character: EffectiveCharacterProperties,
}

impl LadderDefaults {
    fn read(
        sheet: &StyleSheet,
        theme: &ThemeContext,
        interner: &Interner,
    ) -> Result<Self, DocxError> {
        Ok(Self {
            paragraph: sheet
                .document_defaults()
                .and_then(DocumentDefaults::paragraph_properties_default)
                .and_then(DefaultParagraphProperties::paragraph_properties)
                .map(|ppr| extract_style_paragraph_properties(ppr, theme, interner))
                .transpose()?
                .unwrap_or_default(),
            character: sheet
                .document_defaults()
                .and_then(DocumentDefaults::run_properties_default)
                .and_then(DefaultRunProperties::run_properties)
                .map(|rpr| extract_run_properties(rpr, theme, interner))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

/// What a numbering level contributes to the two ladders.
#[derive(Default, Clone)]
struct NumberingTier {
    paragraph: EffectiveParagraphProperties,
    character: EffectiveCharacterProperties,
}

/// One paragraph's style tiers, resolved against the one style index this read builds.
struct ParagraphTiers {
    paragraph: EffectiveParagraphProperties,
    character_from_paragraph_style: EffectiveCharacterProperties,
    characters: Vec<EffectiveCharacterProperties>,
    style_numbering: Option<EffectiveNumberingReference>,
}

impl ParagraphTiers {
    /// What a document with no `word/styles.xml` contributes: nothing, once per run.
    fn empty(runs: usize) -> Self {
        Self {
            paragraph: EffectiveParagraphProperties::default(),
            character_from_paragraph_style: EffectiveCharacterProperties::default(),
            characters: vec![EffectiveCharacterProperties::default(); runs],
            style_numbering: None,
        }
    }

    fn read(
        paragraph: &DirectParagraph,
        cache: &ChainCache<'_>,
        theme: &ThemeContext,
        interner: &Interner,
    ) -> Result<Self, DocxError> {
        let chain = match &paragraph.style_id {
            Some(id) => cache.chain(id)?,
            None => Vec::new(),
        };
        let mut characters = Vec::with_capacity(paragraph.runs.len());
        for run in &paragraph.runs {
            let character_chain = match &run.character_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };
            characters.push(merge_character_chain(&character_chain, theme, interner)?);
        }
        Ok(Self {
            paragraph: merge_paragraph_chain(&chain, theme, interner)?,
            character_from_paragraph_style: merge_character_chain(&chain, theme, interner)?,
            characters,
            style_numbering: numbering_reference_from_chain(&chain, interner)?,
        })
    }
}

/// One paragraph of `word/document.xml`, read.
fn read_direct_paragraph(
    paragraph: &Paragraph,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<DirectParagraph, DocxError> {
    let mut collected = Collected::default();
    walk_paragraph_content(
        paragraph.content(),
        &mut Vec::new(),
        true,
        theme,
        interner,
        &mut collected,
    )?;

    let direct = match paragraph.properties() {
        Some(ppr) => extract_paragraph_properties(ppr, theme, interner)?,
        None => EffectiveParagraphProperties::default(),
    };
    let style_id = paragraph
        .properties()
        .and_then(ParagraphProperties::style)
        .map(|reference| attr(reference.style_id(interner)))
        .transpose()?
        .map(std::borrow::Cow::into_owned);
    let own_numbering = paragraph
        .properties()
        .and_then(ParagraphProperties::numbering)
        .map(|value| extract_numbering_reference(value, interner))
        .transpose()?
        .flatten();

    Ok(DirectParagraph {
        text: collected.text,
        runs: collected.runs,
        hard_breaks: collected.hard_breaks,
        direct,
        style_id,
        own_numbering,
    })
}

/// What one walk of a paragraph's content accumulates.
#[derive(Default)]
struct Collected {
    text: String,
    runs: Vec<DirectRun>,
    hard_breaks: Vec<HardBreak>,
}

/// Walks a paragraph's content the way `paragraph_content_text` does — descending into every
/// wrapper, so no text is lost — while numbering the slots the way [`Paragraph::run_count`] does, so
/// that the [`RunPath`]s produced actually resolve.
///
/// The two walks differ in exactly one place, and it is reported rather than reconciled: a
/// `w:fldSimple` contributes text and is **not** a run slot, so a run inside one gets `None` for its
/// path (`addressable` is how that travels down). Adding it to the slot walk would renumber every
/// existing address in the crate.
fn walk_paragraph_content(
    content: &[ParagraphContent],
    prefix: &mut Vec<usize>,
    addressable: bool,
    theme: &ThemeContext,
    interner: &Interner,
    collected: &mut Collected,
) -> Result<(), DocxError> {
    let mut slot = 0_usize;
    let descend = |inner: &[ParagraphContent],
                   prefix: &mut Vec<usize>,
                   slot: &mut usize,
                   collected: &mut Collected|
     -> Result<(), DocxError> {
        prefix.push(*slot);
        *slot += 1;
        let result = walk_paragraph_content(inner, prefix, addressable, theme, interner, collected);
        prefix.pop();
        result
    };
    for item in content {
        match item {
            ParagraphContent::Run(run) => {
                let path = if addressable {
                    prefix.push(slot);
                    let path = RunPath::from(prefix.clone());
                    prefix.pop();
                    Some(path)
                } else {
                    None
                };
                slot += 1;
                read_run(run, path, theme, interner, collected)?;
            }
            ParagraphContent::Hyperlink(hyperlink) => {
                descend(hyperlink.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::CustomXml(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::SmartTag(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::StructuredDocumentTag(control) => {
                let inner = control.content_run().map_or(&[][..], |run| run.content());
                descend(inner, prefix, &mut slot, collected)?;
            }
            ParagraphContent::BidirectionalEmbedding(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::BidirectionalOverride(wrapper) => {
                descend(wrapper.content(), prefix, &mut slot, collected)?;
            }
            ParagraphContent::SimpleField(field) => {
                // Not a run slot; see this function's own doc comment. Its runs keep their text and
                // lose their address, and `slot` deliberately does not advance — a `w:fldSimple` is
                // invisible to `Paragraph::run_count`, so counting it here would shift every
                // address after it.
                walk_paragraph_content(
                    field.content(),
                    &mut Vec::new(),
                    false,
                    theme,
                    interner,
                    collected,
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// One `w:r`: the characters it contributes and the formatting it states.
fn read_run(
    run: &Run,
    path: Option<RunPath>,
    theme: &ThemeContext,
    interner: &Interner,
    collected: &mut Collected,
) -> Result<(), DocxError> {
    let start = collected.text.len();
    for item in run.content() {
        match item {
            RunInnerContent::Text(value) => collected.text.push_str(value.text()),
            RunInnerContent::TabCharacter(_) => collected.text.push('\t'),
            RunInnerContent::CarriageReturn(_) => collected.text.push('\n'),
            RunInnerContent::OptionalHyphen(_) => collected.text.push(SOFT_HYPHEN),
            RunInnerContent::NonBreakingHyphen(_) => collected.text.push(NON_BREAKING_HYPHEN),
            RunInnerContent::Break(value) => {
                if let Some(kind @ (BreakType::Page | BreakType::Column)) =
                    attr(value.kind(interner))?
                {
                    collected.hard_breaks.push(HardBreak {
                        at: collected.text.len(),
                        kind,
                    });
                }
                collected.text.push('\n');
            }
            _ => {}
        }
    }
    let direct = match run.run_properties() {
        Some(rpr) => extract_run_properties(rpr, theme, interner)?,
        None => EffectiveCharacterProperties::default(),
    };
    let character_style_id = run
        .run_properties()
        .and_then(RunProperties::character_style)
        .map(|reference| attr(reference.style_id(interner)))
        .transpose()?
        .map(std::borrow::Cow::into_owned);
    collected.runs.push(DirectRun {
        range: start..collected.text.len(),
        path,
        direct,
        character_style_id,
    });
    Ok(())
}

fn read_page_size(
    properties: &SectionProperties,
    interner: &Interner,
) -> Result<Option<PageSize>, DocxError> {
    attr(properties.page_size(interner))
}

fn read_page_margins(
    properties: &SectionProperties,
    interner: &Interner,
) -> Result<Option<PageMargins>, DocxError> {
    attr(properties.page_margins(interner))
}
