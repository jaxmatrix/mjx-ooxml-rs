//! List numbering: the counters nine levels keep, the restarts that reset them, and the
//! **composition of `w:lvlText`** — which is the part a gate can actually assert.
//!
//! # The identity-value trap, named before the code
//!
//! A list tested at one level, in one format, with no restart, exercises almost nothing. `%1` with a
//! decimal format and a start of 1 produces `1`, `2`, `3` for an implementation that composes the
//! template properly **and** for one that prints a running counter and ignores `w:lvlText`
//! altogether. Every plausible bug is the identity on that fixture.
//!
//! So the gate is written on the things that are not the identity, and this module is shaped around
//! being able to answer them:
//!
//! * **Multi-level composition.** `%1.%2.%3` at level two needs levels *zero and one*'s counters and
//!   *their own* number formats. An implementation that formatted every placeholder in the current
//!   level's format renders `I.B.3` as `III.II.III`.
//! * **`w:lvlRestart`.** A level resets when a *higher* level advances, and `w:lvlRestart="0"` means
//!   **never** — not "restart at level zero", which is the reading that turns a continuous numbered
//!   list into an endless run of ones.
//! * **`w:startOverride`.** A `w:num` may restart a level at an arbitrary value, and it outranks the
//!   abstract definition's own `w:start`.
//! * **`w:isLgl`.** Every placeholder is written in Arabic whatever the referenced level's format
//!   says, so `III.B.2` becomes `3.2.2`. It changes what a *higher* level renders as, which is what
//!   makes it a composition question rather than a formatting one.
//! * **A definition inherited through a style.** The `w:numPr` may be on the paragraph, or on its
//!   `w:pStyle`; `mjx-docx`'s ladder resolves both into one
//!   [`EffectiveNumberingReference`](mjx_docx::EffectiveNumberingReference), and a renderer reading
//!   only the direct one numbers nothing in a document built from Word's own Heading styles.
//!
//! # What a marker is made of, and where it goes
//!
//! A marker is three things: the composed text, the run formatting the level states for it (already
//! folded into the paragraph's own ladder by `mjx-docx`'s numbering tier), and a **suffix** —
//! `w:suff`, one of a tab, a space or nothing — that separates it from the paragraph's text.
//!
//! The tab is the interesting one, and it is why this module produces *text* rather than a
//! positioned box: `w:suff="tab"` emits a real `U+0009`, which [`crate::tabs`] then resolves against
//! the paragraph's own tab stops exactly as any other tab is. A hanging indent puts the marker at
//! `indent_start + first_line_offset` and the first tab stop at `indent_start`, so the text after
//! the number begins at the indent — which is what a hanging indent *is*, and it falls out of the
//! existing tab machinery rather than needing a second one.
//!
//! # ⚠ Provenance
//!
//! `SpecCode` for the members and their defaults (§17.9's own), `DocumentedBehaviour` for the
//! placeholder grammar (which `mjx_docx::LevelTextTemplate::segments` parses and this consumes), and
//! `EngineDerived` for two readings marked `GUESS:` at their sites: what a placeholder naming a
//! level the definition does not state renders as, and whether a bullet level's counter still
//! advances.

use mjx_docx::{
    DocumentFormatting, LevelTextSegment, NumberingDefinition, NumberingLevelFormatting,
};
use mjx_ooxml_types::wordprocessingml::{NumberFormat, NumberingLevelSuffix};

use crate::numbering::format_number;

/// The nine counters one `w:num` keeps.
///
/// One array rather than a map: `w:ilvl` is 0–8 by the schema, so the shape is known and a lookup is
/// an index.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Counters {
    numbering_id: i64,
    values: [i64; LEVELS],
    /// Whether each level has been reached at all — a level that has never been reached renders its
    /// `w:start`, and one that has renders its counter, and zero cannot tell those apart when a
    /// `w:start` is zero.
    started: [bool; LEVELS],
}

/// How many levels a `w:num` has. Nine, by §17.9.4's restriction on `w:ilvl`.
pub const LEVELS: usize = 9;

/// Every list in a document, counted in document order.
///
/// # Why this is a whole-document walk and not a per-paragraph function
///
/// A list's *n*th item is *n* because *n−1* items precede it, and which items precede it is a fact
/// about the document rather than about the paragraph. So the numbers are computed once, in one
/// pass, and looked up — the same shape `crate::model`'s footnote prefix sums already have, and for
/// the same reason: a per-paragraph answer would rewalk the document per paragraph.
///
/// The pass is over [`DocumentFormatting::paragraphs`]'s **top-level** entries in order. A cell's
/// paragraphs follow the body's in that list rather than being interleaved, so a numbered list
/// inside a table cell counts after the body's — which is wrong, and is stated here rather than
/// hidden: see [`ListNumbering::read`].
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct ListNumbering {
    /// The composed marker for each paragraph that has one, by paragraph index.
    markers: std::collections::BTreeMap<usize, Marker>,
}

/// One paragraph's list marker.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Marker {
    /// The composed text — what `w:lvlText`'s placeholders became.
    pub text: String,
    /// `w:suff` — what separates it from the paragraph's own text.
    pub suffix: NumberingLevelSuffix,
    /// Which level it is at, so a caller can say which counter produced it.
    pub level: i64,
    /// Whether every numeral in it could be written in the system the level asked for; see
    /// [`crate::numbering::is_written_exactly`].
    pub exact: bool,
    /// `w:lvlPicBulletId` — the picture bullet this level draws instead of a character, when it
    /// names one.
    ///
    /// **Reported and not drawn.** A picture bullet's payload is a VML shape or a DrawingML picture,
    /// `mjx-docx` keeps it unparsed on purpose, and this crate resolves no picture at all — the same
    /// line an inline drawing's *content* already sits on. What travels is that there is one, so a
    /// scene companion can draw it and a reader can be told why a bullet is missing.
    pub picture_bullet: Option<i64>,
}

impl Marker {
    /// The characters this marker contributes to the paragraph's layout text, suffix included.
    ///
    /// `w:suff="tab"` becomes a real `U+0009`, which is what makes the indent interaction fall out
    /// of [`crate::tabs`] rather than needing a second implementation; `"space"` becomes one space;
    /// `"nothing"` adds nothing at all.
    #[must_use]
    pub fn with_suffix(&self) -> String {
        let mut text = self.text.clone();
        match self.suffix {
            NumberingLevelSuffix::Tab => text.push('\t'),
            NumberingLevelSuffix::Space => text.push(' '),
            NumberingLevelSuffix::Nothing => {}
        }
        text
    }
}

impl ListNumbering {
    /// Walks `formatting`'s paragraphs once and composes every list marker in the document.
    ///
    /// # ⚠ A cell's list counts after the body's, and that is a known limit
    ///
    /// [`DocumentFormatting::paragraphs`] holds the body's top-level paragraphs first and every
    /// cell's paragraphs after them, which is the order MJXOFF-176 chose so that a `w:sectPr` span
    /// stays numbered against the body walk. A numbered list that runs *through* a table therefore
    /// has its in-cell items counted after the whole body rather than in place.
    ///
    /// It is visible only in a document whose list crosses a table boundary, and correcting it means
    /// walking [`DocumentFormatting::blocks`] recursively instead — which is the right fix and
    /// belongs with the child that also fixes what a `w:sectPr` span means inside a cell. Stated
    /// here so that nobody reads a green suite as evidence it does not happen.
    #[must_use]
    pub fn read(formatting: &DocumentFormatting) -> Self {
        let mut markers = std::collections::BTreeMap::new();
        let mut counters: Vec<Counters> = Vec::new();
        for (index, paragraph) in formatting.paragraphs().iter().enumerate() {
            let Some(reference) = paragraph.properties().numbering else {
                continue;
            };
            // `w:numId="0"` is not a list: §17.9.18 makes zero the explicit *removal* of an
            // inherited numbering reference, which is how a paragraph opts out of the list its style
            // puts it in. Counting it would number a paragraph that Word draws unnumbered.
            if reference.numbering_id == 0 {
                continue;
            }
            let Some(definition) = formatting.numbering_definition(reference.numbering_id) else {
                continue;
            };
            let level = reference.level.clamp(0, LEVELS as i64 - 1);
            let slot = match counters
                .iter()
                .position(|entry| entry.numbering_id == reference.numbering_id)
            {
                Some(slot) => slot,
                None => {
                    counters.push(Counters::new(definition));
                    counters.len() - 1
                }
            };
            counters[slot].advance(definition, level);
            if let Some(marker) = compose(definition, &counters[slot], level) {
                markers.insert(index, marker);
            }
        }
        Self { markers }
    }

    /// The marker for paragraph `index`, if it has one.
    #[must_use]
    pub fn marker(&self, index: usize) -> Option<&Marker> {
        self.markers.get(&index)
    }

    /// How many paragraphs carry a marker.
    #[must_use]
    pub fn len(&self) -> usize {
        self.markers.len()
    }

    /// Whether nothing in the document is numbered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }
}

impl Counters {
    /// The counters a definition starts at: every level at its own `w:start`, or its
    /// `w:startOverride` when the instance states one, and none of them yet reached.
    fn new(definition: &NumberingDefinition) -> Self {
        let mut values = [1_i64; LEVELS];
        for (index, value) in values.iter_mut().enumerate() {
            if let Some(level) = definition.level(i64::try_from(index).unwrap_or(i64::MAX)) {
                *value = level.start_override.unwrap_or(level.start);
            }
        }
        Self {
            numbering_id: definition.numbering_id,
            values,
            started: [false; LEVELS],
        }
    }

    /// One paragraph at `level` advances the counters.
    ///
    /// # The restart rule, and the value that is not a level
    ///
    /// §17.9.10: `w:lvlRestart` names the **one-based** level whose advance resets this one, and its
    /// own default is "the level immediately above" — so an ordinary nine-level outline restarts
    /// each level whenever any higher one moves, with no `w:lvlRestart` written anywhere.
    ///
    /// **`w:lvlRestart="0"` means never**, and it is the reading a renderer gets wrong: taken as a
    /// level index it would mean "restart when level zero advances", which is *almost* the default
    /// and therefore looks right on a two-level list and turns a legal-style continuous numbering
    /// into a sequence of ones.
    fn advance(&mut self, definition: &NumberingDefinition, level: i64) {
        let index = level.clamp(0, LEVELS as i64 - 1) as usize;
        if self.started[index] {
            self.values[index] = self.values[index].saturating_add(1);
        } else {
            self.started[index] = true;
        }
        // Every deeper level restarts, subject to its own `w:lvlRestart`.
        for deeper in (index + 1)..LEVELS {
            let restart_after = definition
                .level(deeper as i64)
                .and_then(|entry| entry.restart_after);
            let resets = match restart_after {
                // Never.
                Some(0) => false,
                // **The direction is the whole of this rule and it is easy to invert.**
                // `w:lvlRestart="1"` on level three means *restart when level **one** advances, and
                // not when level two does* — "renumber at each chapter, not at each section". So the
                // level that just advanced resets this one when **its** one-based number is at or
                // above the threshold in the outline, which is `index + 1 <= after`.
                //
                // The reverse reading (`after <= index + 1`) makes every stated `w:lvlRestart`
                // behave like the default, because the default is the case where the threshold is
                // this level's own number — so a two-level list looks right and a three-level one
                // renumbers at every section.
                Some(after) => i64::try_from(index).unwrap_or(i64::MAX).saturating_add(1) <= after,
                // The default: restart whenever any higher level advances, which is the same rule
                // with the threshold at this level's own one-based number.
                None => true,
            };
            if resets {
                self.started[deeper] = false;
                self.values[deeper] = definition
                    .level(deeper as i64)
                    .map_or(1, |entry| entry.start_override.unwrap_or(entry.start));
            }
        }
    }

    /// The value level `index` currently shows.
    fn value(&self, index: usize) -> i64 {
        self.values.get(index).copied().unwrap_or(1)
    }

    /// Whether level `index` has been reached at all.
    fn has_started(&self, index: usize) -> bool {
        self.started.get(index).copied().unwrap_or(false)
    }
}

/// Composes one marker from a level's `w:lvlText` and the counters.
fn compose(definition: &NumberingDefinition, counters: &Counters, level: i64) -> Option<Marker> {
    let entry = definition.level(level)?;
    let mut text = String::new();
    let mut exact = true;
    for segment in &entry.template {
        match segment {
            LevelTextSegment::Literal(literal) => text.push_str(literal),
            LevelTextSegment::Level(placeholder) => {
                // `%1` is level zero: the placeholders are one-based and `w:ilvl` is zero-based, and
                // conflating them renders a second-level marker with the first level's number.
                let referenced = usize::from(*placeholder).saturating_sub(1);
                let Some(source) = definition.level(referenced as i64) else {
                    // **GUESS:** a placeholder naming a level the definition does not state renders
                    // as nothing rather than as a zero or as the literal `%n`. §17.9.11 does not say.
                    // Nothing is the least wrong of the three: a `0` is a number a reader would take
                    // for the list's, and `%3` is markup on the page.
                    continue;
                };
                if !counters.has_started(referenced) && referenced > level as usize {
                    // A deeper level that has not been reached contributes nothing — a `%1.%2` at
                    // level zero is just `%1`, which is what makes one template serve every level.
                    continue;
                }
                let format = marker_format(entry, source);
                exact &= crate::numbering::is_written_exactly(format);
                text.push_str(&format_number(counters.value(referenced), format));
            }
        }
    }
    Some(Marker {
        text,
        // §17.9.29's own default is a tab, which is what makes an unstated `w:suff` behave like
        // every list Word writes.
        suffix: entry.suffix.unwrap_or(NumberingLevelSuffix::Tab),
        level,
        exact,
        picture_bullet: entry.picture_bullet,
    })
}

/// Which numeral system one placeholder is written in.
///
/// `w:isLgl` on the level being *rendered* forces Arabic for **every** placeholder in its template,
/// whatever each referenced level's own `w:numFmt` says — §17.9.11's whole purpose, and the reason
/// this takes both levels rather than only the source.
fn marker_format(
    rendered: &NumberingLevelFormatting,
    source: &NumberingLevelFormatting,
) -> NumberFormat {
    if rendered.legal {
        return NumberFormat::Decimal;
    }
    // A level that states no `w:numFmt` is decimal — §17.9.17's own default. A `bullet` level's
    // template is a literal character and holds no placeholder, so this is only reached for a
    // numbered one.
    source.format.unwrap_or(NumberFormat::Decimal)
}
