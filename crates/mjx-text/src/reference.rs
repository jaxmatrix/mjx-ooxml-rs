//! Published metrics for the *original* fonts a substitution replaces.
//!
//! # Why this table exists, and why it may not be measured from our own faces
//!
//! A substitute is metric-compatible when its advance widths are the same fraction of an em as the
//! original's. That is the property pagination rests on: equal advances mean equal line breaks, and
//! equal line breaks mean a document that ends on the page Office ends it on.
//!
//! The trap in checking it is that **a face always agrees with itself**. Reading Carlito's advances
//! with this crate's own reader and comparing them against a table this crate derived from Carlito
//! is green whatever either says. So every number below is transcribed from a source *outside this
//! repository*, and each carries the citation it was transcribed from and a
//! [`ReferenceAuthority`] saying how far that source can be trusted.
//!
//! # Where the numbers come from
//!
//! * **Arial, Times New Roman and Courier New.** Their advance widths are the Adobe base-35
//!   metrics for Helvetica, Times-Roman and Courier: the three Microsoft faces were drawn to those
//!   widths, which is the entire reason a Helvetica-metric clone can stand in for Arial. The
//!   literals are transcribed from the AFM files of the (URW)++ base-35 set, which Artifex
//!   publishes at <https://github.com/ArtifexSoftware/urw-base35-fonts> (`fonts/NimbusSans-Regular.afm`,
//!   `fonts/NimbusRoman-Regular.afm`, `fonts/NimbusMonoPS-Regular.afm`, retrieved 2026-09-06),
//!   in the AFM's own 1000-unit em.
//! * **Their vertical metrics** come from `@capsizecss/metrics` 4.2.0
//!   (<https://github.com/seek-oss/capsize>, MIT), whose entries are measured from the *Microsoft*
//!   faces themselves rather than from a clone — `entireMetricsCollection/arial/regular`,
//!   `…/timesNewRoman/regular`, `…/courierNew/regular`.
//! * **Calibri.** Microsoft publishes no width table for it. What *is* published is the number
//!   ECMA-376 Part 1 §18.3.1.13 (`col/@width`) depends on: a column width is expressed in Maximum
//!   Digit Widths of the workbook's normal font, and Calibri's maximum digit advance is **1038 font
//!   units at 2048 units per em**. Every Calibri digit shares that advance, so it pins ten
//!   characters, and it is enough to catch a Carlito that has drifted.
//! * **Cambria.** Nothing about it is published: no width table, and no vertical metrics in any
//!   collection surveyed for this table. Its entry is therefore
//!   [`ReferenceAuthority::Unverified`], and the resolver reports Cambria substitutions as
//!   unverified rather than pretending otherwise. Filling it in needs one measurement of the real
//!   face on a licensed Windows machine — the same offline pass MJXOFF-155 §9 already plans for the
//!   fidelity oracle's reference renderer. **Advance widths are facts about a font, not the font
//!   program, so what that pass would commit here is numbers, not a redistributable binary.**
//!
//! # What "agrees" means
//!
//! [`ADVANCE_TOLERANCE_PER_MILLE`]. A 2048-unit em cannot express a 1000-unit em's value exactly,
//! so a face drawn to a 1000-unit specification lands within half a 2048th of it — 0.245 thousandths
//! of an em at worst. The tolerance is 0.5, which passes that quantisation and fails a
//! disagreement of a single 1000-unit step.

use crate::face::AdvanceWidth;

/// How far a substitute's advance may sit from the published original's, in thousandths of an em.
///
/// See the module documentation: this is twice the worst rounding a 2048-unit em can introduce when
/// realising a 1000-unit design, and half the smallest disagreement a 1000-unit table can express.
pub const ADVANCE_TOLERANCE_PER_MILLE: f64 = 0.5;

/// How much weight a reference value carries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReferenceAuthority {
    /// Transcribed from a source outside this repository that measured, or specified, the original
    /// face. A comparison against it is evidence.
    Published,
    /// Transcribed from a source that describes a *related* face rather than the original — kept so
    /// the shape of the data is honest, but never treated as proof.
    Provisional,
    /// No source was found. The entry names the family so that the gap is visible and queryable
    /// rather than silently absent, and carries no numbers at all.
    Unverified,
}

impl ReferenceAuthority {
    /// Whether a comparison against a value of this authority proves anything.
    #[must_use]
    pub fn is_evidence(self) -> bool {
        matches!(self, Self::Published)
    }
}

/// The published vertical metrics of an original face, in that face's own units.
///
/// # Which of these the compatibility contract covers
///
/// The em square and the `hhea` triple do: together they fix the baseline-to-baseline distance, so
/// a substitute that differs puts a different number of lines on a page. **Cap height and x-height
/// do not.** They are design metrics — they place small capitals and scale a drop cap — and a
/// metric-compatible clone is under no obligation to match them, nor does it: Arial's published
/// `sCapHeight` is 1467 against Liberation Sans's 1409, and Arial's x-height 1062 against 1082.
/// They are recorded because R03 and R04 want them, not because anything is asserted about them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReferenceVerticalMetrics {
    /// The em square the four values below are measured against.
    pub units_per_em: u16,
    /// `hhea.ascender`.
    pub ascender: i16,
    /// `hhea.descender`, negative below the baseline.
    pub descender: i16,
    /// `hhea.lineGap`.
    pub line_gap: i16,
    /// `OS/2.sCapHeight`.
    pub cap_height: i16,
    /// `OS/2.sxHeight`.
    pub x_height: i16,
    /// Where these five numbers were transcribed from.
    pub provenance: &'static str,
    /// How much weight they carry.
    pub authority: ReferenceAuthority,
}

/// The published metrics of one original font family.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReferenceMetrics {
    /// The family as a document names it — `Times New Roman`, not `Nimbus Roman`.
    pub family: &'static str,
    /// The em square [`ReferenceMetrics::advances`] is measured against.
    pub advance_units_per_em: u16,
    /// Advance widths, sorted by character so a lookup can binary-search.
    pub advances: &'static [(char, i32)],
    /// Where the advances were transcribed from.
    pub advance_provenance: &'static str,
    /// How much weight the advances carry.
    pub advance_authority: ReferenceAuthority,
    /// The published vertical metrics, where a source for them was found.
    pub vertical: Option<ReferenceVerticalMetrics>,
}

impl ReferenceMetrics {
    /// The published advance for `character`, or `None` when the table does not cover it.
    #[must_use]
    pub fn advance_for_character(&self, character: char) -> Option<AdvanceWidth> {
        self.advances
            .binary_search_by_key(&character, |(key, _)| *key)
            .ok()
            .map(|position| AdvanceWidth {
                font_units: self.advances[position].1,
                units_per_em: self.advance_units_per_em,
            })
    }

    /// How many characters the table covers.
    #[must_use]
    pub fn covered_character_count(&self) -> usize {
        self.advances.len()
    }
}

/// The published reference metrics for `family`, matched case-insensitively, or `None` when this
/// table has no entry for it at all.
///
/// An entry whose authority is [`ReferenceAuthority::Unverified`] is still returned: "we know this
/// family and have no numbers for it" is a different answer from "we have never heard of it", and
/// the substitution manifest reports the two differently.
#[must_use]
pub fn reference_for_family(family: &str) -> Option<&'static ReferenceMetrics> {
    REFERENCE_METRICS
        .iter()
        .find(|entry| entry.family.eq_ignore_ascii_case(family))
}

/// Every family this table has published metrics for.
pub static REFERENCE_METRICS: &[ReferenceMetrics] = &[
    ReferenceMetrics {
        family: "Arial",
        advance_units_per_em: 1000,
        advances: ARIAL_ADVANCES,
        advance_provenance: "(URW)++ base-35 `NimbusSans-Regular.afm`, the published Helvetica \
                             metric set Arial was drawn to; Artifex `urw-base35-fonts`, retrieved \
                             2026-09-06",
        advance_authority: ReferenceAuthority::Published,
        vertical: Some(ReferenceVerticalMetrics {
            units_per_em: 2048,
            ascender: 1854,
            descender: -434,
            line_gap: 67,
            cap_height: 1467,
            x_height: 1062,
            provenance: "@capsizecss/metrics 4.2.0, `entireMetricsCollection/arial/regular`, \
                         measured from Microsoft's own Arial",
            authority: ReferenceAuthority::Published,
        }),
    },
    ReferenceMetrics {
        family: "Times New Roman",
        advance_units_per_em: 1000,
        advances: TIMES_NEW_ROMAN_ADVANCES,
        advance_provenance: "(URW)++ base-35 `NimbusRoman-Regular.afm`, the published Times-Roman \
                             metric set Times New Roman was drawn to; Artifex `urw-base35-fonts`, \
                             retrieved 2026-09-06",
        advance_authority: ReferenceAuthority::Published,
        vertical: Some(ReferenceVerticalMetrics {
            units_per_em: 2048,
            ascender: 1825,
            descender: -443,
            line_gap: 87,
            cap_height: 1356,
            x_height: 916,
            provenance: "@capsizecss/metrics 4.2.0, \
                         `entireMetricsCollection/timesNewRoman/regular`, measured from \
                         Microsoft's own Times New Roman",
            authority: ReferenceAuthority::Published,
        }),
    },
    ReferenceMetrics {
        family: "Courier New",
        advance_units_per_em: 1000,
        advances: COURIER_NEW_ADVANCES,
        advance_provenance: "(URW)++ base-35 `NimbusMonoPS-Regular.afm`, the published Courier \
                             metric set Courier New was drawn to; Artifex `urw-base35-fonts`, \
                             retrieved 2026-09-06",
        advance_authority: ReferenceAuthority::Published,
        vertical: Some(ReferenceVerticalMetrics {
            units_per_em: 2048,
            ascender: 1705,
            descender: -615,
            line_gap: 0,
            cap_height: 1170,
            x_height: 866,
            provenance: "@capsizecss/metrics 4.2.0, \
                         `entireMetricsCollection/courierNew/regular`, measured from Microsoft's \
                         own Courier New",
            authority: ReferenceAuthority::Published,
        }),
    },
    ReferenceMetrics {
        family: "Calibri",
        advance_units_per_em: 2048,
        advances: CALIBRI_ADVANCES,
        advance_provenance: "ECMA-376 Part 1 §18.3.1.13 expresses a column width in Maximum Digit \
                             Widths of the workbook's normal font; Calibri's maximum digit advance \
                             is 1038 units at 2048 units per em, and every Calibri digit carries \
                             it (tabular figures)",
        advance_authority: ReferenceAuthority::Published,
        // No published source measures Calibri's `hhea` values. The digit advance above is enough
        // to catch a drifted substitute horizontally; line height is left unclaimed rather than
        // guessed.
        vertical: None,
    },
    ReferenceMetrics {
        family: "Cambria",
        advance_units_per_em: 1000,
        advances: &[],
        advance_provenance: "no published metric table for Cambria was found: Microsoft publishes \
                             none, and it is absent from the metric collections that carry Arial, \
                             Times New Roman and Courier New. Measuring the real face on a \
                             licensed Windows machine is what fills this in — see MJXOFF-155 §9",
        advance_authority: ReferenceAuthority::Unverified,
        vertical: None,
    },
];

/// Arial's advance widths, at 1000 units per em.
///
/// Transcribed from `NimbusSans-Regular.afm`'s `C … ; WX … ; N …` records, which are the published
/// Helvetica metrics. `'` is `quotesingle` (U+0027), **not** the AFM's `quoteright` at code 39: the
/// AFM's default encoding is AdobeStandardEncoding, where code 39 is U+2019, and a TrueType face
/// maps U+0027 to a narrower glyph.
static ARIAL_ADVANCES: &[(char, i32)] = &[
    (' ', 278),  // space
    ('!', 278),  // exclam
    ('"', 355),  // quotedbl
    ('#', 556),  // numbersign
    ('$', 556),  // dollar
    ('%', 889),  // percent
    ('&', 667),  // ampersand
    ('\'', 191), // quotesingle
    ('(', 333),  // parenleft
    (')', 333),  // parenright
    ('*', 389),  // asterisk
    ('+', 584),  // plus
    (',', 278),  // comma
    ('-', 333),  // hyphen
    ('.', 278),  // period
    ('/', 278),  // slash
    ('0', 556),  // zero
    ('1', 556),  // one
    ('2', 556),  // two
    ('3', 556),  // three
    ('4', 556),  // four
    ('5', 556),  // five
    ('6', 556),  // six
    ('7', 556),  // seven
    ('8', 556),  // eight
    ('9', 556),  // nine
    (':', 278),  // colon
    (';', 278),  // semicolon
    ('<', 584),  // less
    ('=', 584),  // equal
    ('>', 584),  // greater
    ('?', 556),  // question
    ('@', 1015), // at
    ('A', 667),  // A
    ('B', 667),  // B
    ('C', 722),  // C
    ('D', 722),  // D
    ('E', 667),  // E
    ('F', 611),  // F
    ('G', 778),  // G
    ('H', 722),  // H
    ('I', 278),  // I
    ('J', 500),  // J
    ('K', 667),  // K
    ('L', 556),  // L
    ('M', 833),  // M
    ('N', 722),  // N
    ('O', 778),  // O
    ('P', 667),  // P
    ('Q', 778),  // Q
    ('R', 722),  // R
    ('S', 667),  // S
    ('T', 611),  // T
    ('U', 722),  // U
    ('V', 667),  // V
    ('W', 944),  // W
    ('X', 667),  // X
    ('Y', 667),  // Y
    ('Z', 611),  // Z
    ('[', 278),  // bracketleft
    ('\\', 278), // backslash
    (']', 278),  // bracketright
    ('_', 556),  // underscore
    ('a', 556),  // a
    ('b', 556),  // b
    ('c', 500),  // c
    ('d', 556),  // d
    ('e', 556),  // e
    ('f', 278),  // f
    ('g', 556),  // g
    ('h', 556),  // h
    ('i', 222),  // i
    ('j', 222),  // j
    ('k', 500),  // k
    ('l', 222),  // l
    ('m', 833),  // m
    ('n', 556),  // n
    ('o', 556),  // o
    ('p', 556),  // p
    ('q', 556),  // q
    ('r', 333),  // r
    ('s', 500),  // s
    ('t', 278),  // t
    ('u', 556),  // u
    ('v', 500),  // v
    ('w', 722),  // w
    ('x', 500),  // x
    ('y', 500),  // y
    ('z', 500),  // z
    ('{', 334),  // braceleft
    ('|', 260),  // bar
    ('}', 334),  // braceright
];

/// Times New Roman's advance widths, at 1000 units per em, from `NimbusRoman-Regular.afm`.
static TIMES_NEW_ROMAN_ADVANCES: &[(char, i32)] = &[
    (' ', 250),  // space
    ('!', 333),  // exclam
    ('"', 408),  // quotedbl
    ('#', 500),  // numbersign
    ('$', 500),  // dollar
    ('%', 833),  // percent
    ('&', 778),  // ampersand
    ('\'', 180), // quotesingle
    ('(', 333),  // parenleft
    (')', 333),  // parenright
    ('*', 500),  // asterisk
    ('+', 564),  // plus
    (',', 250),  // comma
    ('-', 333),  // hyphen
    ('.', 250),  // period
    ('/', 278),  // slash
    ('0', 500),  // zero
    ('1', 500),  // one
    ('2', 500),  // two
    ('3', 500),  // three
    ('4', 500),  // four
    ('5', 500),  // five
    ('6', 500),  // six
    ('7', 500),  // seven
    ('8', 500),  // eight
    ('9', 500),  // nine
    (':', 278),  // colon
    (';', 278),  // semicolon
    ('<', 564),  // less
    ('=', 564),  // equal
    ('>', 564),  // greater
    ('?', 444),  // question
    ('@', 921),  // at
    ('A', 722),  // A
    ('B', 667),  // B
    ('C', 667),  // C
    ('D', 722),  // D
    ('E', 611),  // E
    ('F', 556),  // F
    ('G', 722),  // G
    ('H', 722),  // H
    ('I', 333),  // I
    ('J', 389),  // J
    ('K', 722),  // K
    ('L', 611),  // L
    ('M', 889),  // M
    ('N', 722),  // N
    ('O', 722),  // O
    ('P', 556),  // P
    ('Q', 722),  // Q
    ('R', 667),  // R
    ('S', 556),  // S
    ('T', 611),  // T
    ('U', 722),  // U
    ('V', 722),  // V
    ('W', 944),  // W
    ('X', 722),  // X
    ('Y', 722),  // Y
    ('Z', 611),  // Z
    ('[', 333),  // bracketleft
    ('\\', 278), // backslash
    (']', 333),  // bracketright
    ('_', 500),  // underscore
    ('a', 444),  // a
    ('b', 500),  // b
    ('c', 444),  // c
    ('d', 500),  // d
    ('e', 444),  // e
    ('f', 333),  // f
    ('g', 500),  // g
    ('h', 500),  // h
    ('i', 278),  // i
    ('j', 278),  // j
    ('k', 500),  // k
    ('l', 278),  // l
    ('m', 778),  // m
    ('n', 500),  // n
    ('o', 500),  // o
    ('p', 500),  // p
    ('q', 500),  // q
    ('r', 333),  // r
    ('s', 389),  // s
    ('t', 278),  // t
    ('u', 500),  // u
    ('v', 500),  // v
    ('w', 722),  // w
    ('x', 500),  // x
    ('y', 500),  // y
    ('z', 444),  // z
    ('{', 480),  // braceleft
    ('|', 200),  // bar
    ('}', 480),  // braceright
];

/// Courier New's advance widths, at 1000 units per em, from `NimbusMonoPS-Regular.afm`. Every entry
/// is 600 because the face is monospaced; the table is written out in full anyway, so that a
/// substitute that is *not* uniformly 600 fails on the character that differs rather than on a
/// summary.
static COURIER_NEW_ADVANCES: &[(char, i32)] = &[
    (' ', 600),  // space
    ('!', 600),  // exclam
    ('"', 600),  // quotedbl
    ('#', 600),  // numbersign
    ('$', 600),  // dollar
    ('%', 600),  // percent
    ('&', 600),  // ampersand
    ('\'', 600), // quotesingle
    ('(', 600),  // parenleft
    (')', 600),  // parenright
    ('*', 600),  // asterisk
    ('+', 600),  // plus
    (',', 600),  // comma
    ('-', 600),  // hyphen
    ('.', 600),  // period
    ('/', 600),  // slash
    ('0', 600),  // zero
    ('1', 600),  // one
    ('2', 600),  // two
    ('3', 600),  // three
    ('4', 600),  // four
    ('5', 600),  // five
    ('6', 600),  // six
    ('7', 600),  // seven
    ('8', 600),  // eight
    ('9', 600),  // nine
    (':', 600),  // colon
    (';', 600),  // semicolon
    ('<', 600),  // less
    ('=', 600),  // equal
    ('>', 600),  // greater
    ('?', 600),  // question
    ('@', 600),  // at
    ('A', 600),  // A
    ('B', 600),  // B
    ('C', 600),  // C
    ('D', 600),  // D
    ('E', 600),  // E
    ('F', 600),  // F
    ('G', 600),  // G
    ('H', 600),  // H
    ('I', 600),  // I
    ('J', 600),  // J
    ('K', 600),  // K
    ('L', 600),  // L
    ('M', 600),  // M
    ('N', 600),  // N
    ('O', 600),  // O
    ('P', 600),  // P
    ('Q', 600),  // Q
    ('R', 600),  // R
    ('S', 600),  // S
    ('T', 600),  // T
    ('U', 600),  // U
    ('V', 600),  // V
    ('W', 600),  // W
    ('X', 600),  // X
    ('Y', 600),  // Y
    ('Z', 600),  // Z
    ('[', 600),  // bracketleft
    ('\\', 600), // backslash
    (']', 600),  // bracketright
    ('_', 600),  // underscore
    ('a', 600),  // a
    ('b', 600),  // b
    ('c', 600),  // c
    ('d', 600),  // d
    ('e', 600),  // e
    ('f', 600),  // f
    ('g', 600),  // g
    ('h', 600),  // h
    ('i', 600),  // i
    ('j', 600),  // j
    ('k', 600),  // k
    ('l', 600),  // l
    ('m', 600),  // m
    ('n', 600),  // n
    ('o', 600),  // o
    ('p', 600),  // p
    ('q', 600),  // q
    ('r', 600),  // r
    ('s', 600),  // s
    ('t', 600),  // t
    ('u', 600),  // u
    ('v', 600),  // v
    ('w', 600),  // w
    ('x', 600),  // x
    ('y', 600),  // y
    ('z', 600),  // z
    ('{', 600),  // braceleft
    ('|', 600),  // bar
    ('}', 600),  // braceright
];

/// Calibri's digit advance, at 2048 units per em, from the ECMA-376 Maximum Digit Width above.
///
/// Only the digits: they are the characters the published number pins, and a table that guessed at
/// the letters would be worse than a short one.
static CALIBRI_ADVANCES: &[(char, i32)] = &[
    ('0', 1038),
    ('1', 1038),
    ('2', 1038),
    ('3', 1038),
    ('4', 1038),
    ('5', 1038),
    ('6', 1038),
    ('7', 1038),
    ('8', 1038),
    ('9', 1038),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every table must be sorted, because `advance_for_character` binary-searches it, and a table
    /// that is out of order would return `None` for a character it covers.
    #[test]
    fn every_advance_table_is_sorted_and_unique() {
        for entry in REFERENCE_METRICS {
            for pair in entry.advances.windows(2) {
                assert!(
                    pair[0].0 < pair[1].0,
                    "`{}`'s advance table is not strictly sorted: {:?} is not before {:?}",
                    entry.family,
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// An entry that claims to be evidence must carry numbers, and one that carries none must not
    /// claim to be.
    #[test]
    fn authority_and_content_agree() {
        for entry in REFERENCE_METRICS {
            assert!(
                !entry.advance_provenance.is_empty(),
                "`{}` cites no source for its advances",
                entry.family
            );
            if entry.advance_authority.is_evidence() {
                assert!(
                    !entry.advances.is_empty(),
                    "`{}` claims published advances and carries none",
                    entry.family
                );
            } else {
                assert!(
                    entry.advances.is_empty(),
                    "`{}` carries advances but does not claim they are published — a number \
                     nothing stands behind is worse than no number",
                    entry.family
                );
            }
        }
    }

    #[test]
    fn a_family_is_matched_however_it_is_capitalised() {
        assert!(reference_for_family("times new roman").is_some());
        assert!(reference_for_family("TIMES NEW ROMAN").is_some());
        assert!(reference_for_family("Helvetica").is_none());
    }
}
