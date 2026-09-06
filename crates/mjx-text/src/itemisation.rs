//! Cutting a paragraph into the runs a shaper can actually take.
//!
//! A shaping call sees **one direction, one script and one face**. A paragraph is none of those
//! things, so it is cut three times, in this order:
//!
//! 1. **By embedding level**, because the direction decides which way the glyphs come out and which
//!    Arabic joining forms are chosen. [`crate::BidiAnalysis`] produced these.
//! 2. **By script**, because the script selects the whole shaping engine. [`crate::itemise_by_script`]
//!    produced these.
//! 3. **By face**, here, because a face that has no glyph for a character cannot draw it and the
//!    fallback chain has to be asked for one that can.
//!
//! The order matters. Cutting by face first would ask the resolver to cover a run that spans two
//! scripts, and no single face would; cutting by script first means every coverage question is
//! asked about one writing system, which is the question the resolver's tier 3 was built to answer.
//!
//! # Where R02's fallback chain is actually exercised
//!
//! [`crate::FontResolver::resolve`] takes a [`crate::FontRequest`], and
//! [`crate::FontRequest::requiring`] is what makes the third tier answerable: a face can match the
//! family and still have nothing to draw the run with. This module is the caller that fills that in
//! — one required character per fallback decision, taken from the first character the current face
//! cannot cover.
//!
//! # Characters that do not force a new item
//!
//! Whitespace and the default-ignorable code points — the soft hyphen, the zero-width space and
//! joiners, the bidirectional controls, the variation selectors, the byte-order mark — extend
//! whatever item they land in rather than starting a new one. A face need not have a glyph for them:
//! a shaper drops them or draws nothing, and splitting a word in half because its face lacks a
//! zero-width joiner would break every ligature across the split.

use std::ops::Range;
use std::sync::Arc;

use crate::direction::{BidiAnalysis, BidiLevel, TextDirection};
use crate::error::FontError;
use crate::face::FontFace;
use crate::index::FontRequest;
use crate::resolver::{FontResolution, FontResolver};
use crate::script::{itemise_range_by_script, TextScript};

/// One run: a single direction, a single script, and a single face.
#[derive(Clone, Debug)]
pub struct TextItem {
    /// The bytes it covers, in the paragraph's own offsets.
    pub range: Range<usize>,
    /// The script every character in it belongs to.
    pub script: TextScript,
    /// The UAX #9 level it was resolved at.
    pub level: BidiLevel,
    /// Which way it is shaped.
    pub direction: TextDirection,
    /// What the resolver made of the face request.
    ///
    /// [`FontResolution::Resolved`] carries the face and which tier answered.
    /// [`FontResolution::FetchRequired`] and [`FontResolution::Unresolvable`] are both drawable —
    /// as `.notdef` — and both are worth showing a reader, so they are items rather than omissions.
    pub font: FontResolution,
}

impl TextItem {
    /// The face this item is drawn in, when one was found.
    #[must_use]
    pub fn face(&self) -> Option<&Arc<FontFace>> {
        self.font.face()
    }
}

/// Cut `range` of `text` into items, resolving a face for each.
///
/// `template` supplies the family, weight, width and slant the document asked for; its
/// `required_characters` are replaced per item, because the whole point is to ask the resolver about
/// the characters this item actually holds.
///
/// # Errors
///
/// Whatever [`FontResolver::resolve`] returns — a face that was indexed and will not parse. A family
/// that is simply absent is a substitution, not an error, and is in the resolver's manifest.
pub fn itemise(
    text: &str,
    range: Range<usize>,
    bidi: &BidiAnalysis,
    resolver: &mut FontResolver,
    template: &FontRequest<'_>,
) -> Result<Vec<TextItem>, FontError> {
    let mut items = Vec::new();
    if range.start >= range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Ok(items);
    }

    for level_run in bidi.logical_runs(range) {
        for script_run in itemise_range_by_script(text, level_run.range.clone()) {
            // A right-to-left script at an even level happens whenever the bidirectional algorithm
            // has been switched off above this layer, or the run was declared left-to-right and
            // holds Arabic anyway; shaping it left-to-right would choose the wrong joining forms.
            let direction = if level_run.level.direction() == TextDirection::RightToLeft
                || script_run.script.is_right_to_left()
            {
                TextDirection::RightToLeft
            } else {
                TextDirection::LeftToRight
            };

            let mut cursor = script_run.range.start;
            while cursor < script_run.range.end {
                let remaining = &text[cursor..script_run.range.end];
                let Some(first) = remaining.chars().next() else {
                    break;
                };
                let required = [first];
                let request = template.for_family(template.family).requiring(&required);
                let resolution = resolver.resolve(&request)?;
                let end = extent_of(&resolution, remaining, cursor)?;
                items.push(TextItem {
                    range: cursor..end,
                    script: script_run.script,
                    level: level_run.level,
                    direction,
                    font: resolution,
                });
                cursor = end;
            }
        }
    }
    Ok(items)
}

/// How far the item that starts at `start` reaches, given the face that answered for its first
/// character.
fn extent_of(
    resolution: &FontResolution,
    remaining: &str,
    start: usize,
) -> Result<usize, FontError> {
    let Some(face) = resolution.face() else {
        // No face, so no coverage question to ask; keep every character the resolver could not
        // answer for in one item rather than one item per character, which is what a reader sees as
        // a single run of missing glyphs.
        return Ok(start + remaining.len());
    };
    // One parse for the whole item — the seam R02 built `FaceReader` for. Opening one per character
    // would walk the table directory once per glyph.
    let reader = face.reader()?;
    let mut end = start;
    for (offset, character) in remaining.char_indices() {
        if !extends_the_current_item(character) && !reader.covers(character) {
            if offset == 0 {
                // Not even the first character is covered — the resolver answered with a blind
                // fallback, and there is nothing better to try. Take the character so the walk
                // makes progress instead of looping.
                return Ok(start + character.len_utf8());
            }
            break;
        }
        end = start + offset + character.len_utf8();
    }
    Ok(end)
}

/// Whether a character joins whatever item it lands in rather than deciding one.
fn extends_the_current_item(character: char) -> bool {
    character.is_whitespace() || is_default_ignorable(character)
}

/// The Unicode `Default_Ignorable_Code_Point` property, as the ranges it is defined over.
///
/// Written out rather than pulled from a table crate: it is eleven ranges, it changes about once a
/// decade, and the alternative is a dependency whose whole content is this list.
fn is_default_ignorable(character: char) -> bool {
    matches!(
        character as u32,
        0x00AD                       // SOFT HYPHEN
            | 0x034F                 // COMBINING GRAPHEME JOINER
            | 0x061C                 // ARABIC LETTER MARK
            | 0x115F..=0x1160        // HANGUL CHOSEONG/JUNGSEONG FILLER
            | 0x17B4..=0x17B5        // KHMER VOWEL INHERENT AQ/AA
            | 0x180B..=0x180F        // MONGOLIAN variation selectors and separator
            | 0x200B..=0x200F        // ZERO WIDTH SPACE through RIGHT-TO-LEFT MARK
            | 0x202A..=0x202E        // the embedding, override and pop controls
            | 0x2060..=0x206F        // WORD JOINER through the deprecated formatting controls
            | 0x3164                 // HANGUL FILLER
            | 0xFE00..=0xFE0F        // VARIATION SELECTOR-1..16
            | 0xFEFF                 // ZERO WIDTH NO-BREAK SPACE
            | 0xFFA0                 // HALFWIDTH HANGUL FILLER
            | 0xFFF0..=0xFFF8        // unassigned but reserved as ignorable
            | 0x1BCA0..=0x1BCA3      // SHORTHAND FORMAT controls
            | 0x1D173..=0x1D17A      // MUSICAL SYMBOL beams and slurs
            | 0xE0000..=0xE0FFF // TAG characters and VARIATION SELECTOR-17..256
    )
}
