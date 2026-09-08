//! Bullets: what a paragraph's marker says, how big it is, and what font draws it.
//!
//! # Nine levels, three kinds, one counter set
//!
//! DrawingML gives a text body nine indent levels, and every level has its own bullet — which is why
//! demoting a line changes its marker without anything being written to the paragraph. That
//! resolution is `mjx-pptx`'s: [`effective_paragraph_properties`] answers with the level's bullet
//! already selected, and this module never looks at a list style.
//!
//! What is here is the part that is *not* a property: an automatic number is a **sequence**, so the
//! marker a paragraph draws depends on every paragraph before it. [`AutoNumberCounters`] is that
//! state, and its one rule is the one that is easy to get wrong: **a level restarts when a shallower
//! level intervenes.** A body of `1. 2. a) b) 3.` numbers its third top-level item `3.`, not `1.`,
//! and its next nested item `a)` again.
//!
//! # `a:buBlip` is R15
//!
//! A picture bullet needs an image handle and a resource table, which is R15's work. A paragraph
//! with one is laid out with its indents intact and no marker drawn, so the text sits where it will
//! sit once the picture arrives rather than moving when it does.
//!
//! [`effective_paragraph_properties`]: mjx_pptx::Presentation::effective_paragraph_properties

use mjx_dml::{
    AutoNumberBullet, AutonumberScheme, Bullet, BulletSize, BulletTypeface, ParagraphPropertiesSpec,
};
// `mjx-dml` has a `FontSize` too — the `a:rPr@sz` on the wire, in hundredths of a point. This is the
// *font engine's*, which is what a run is shaped at, and the two are converted through points at the
// one place a bullet size is read.
use mjx_text::FontSize;

use crate::text::RunStyle;

/// The nine levels a text body has.
pub const LEVELS: usize = 9;

/// The running numbers of each indent level.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AutoNumberCounters {
    counts: [u32; LEVELS],
}

impl AutoNumberCounters {
    /// All nine sequences, unstarted.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number the next automatic bullet at `level` draws, advancing that level's sequence and
    /// **restarting every deeper one**.
    ///
    /// The restart is the whole of the rule: a nested list that resumes under a new parent starts
    /// at its own `startAt` again rather than continuing where the previous nest left off.
    pub fn next(&mut self, level: usize, start_at: u32) -> u32 {
        let level = level.min(LEVELS - 1);
        for deeper in self.counts.iter_mut().skip(level + 1) {
            *deeper = 0;
        }
        let Some(slot) = self.counts.get_mut(level) else {
            return start_at;
        };
        *slot = slot.saturating_add(1);
        // The first item of a sequence is `startAt`; every later one counts up from it.
        start_at.saturating_add(slot.saturating_sub(1))
    }

    /// Ends any open sequence at `level` and deeper, without drawing anything.
    ///
    /// What a paragraph with a non-automatic bullet — a character, none at all — does to the
    /// numbering around it: it interrupts the sequences below it but not the one it sits in.
    pub fn interrupt(&mut self, level: usize) {
        let level = level.min(LEVELS - 1);
        for deeper in self.counts.iter_mut().skip(level + 1) {
            *deeper = 0;
        }
    }

    /// How many items level `level` has drawn so far.
    #[must_use]
    pub fn drawn_at(&self, level: usize) -> u32 {
        self.counts.get(level.min(LEVELS - 1)).copied().unwrap_or(0)
    }
}

/// A paragraph's marker, ready to be drawn.
#[derive(Clone, PartialEq, Debug)]
pub struct Marker {
    /// The characters to draw.
    pub text: String,
    /// The style to draw them in — the run style of the paragraph's first run, with the bullet's own
    /// size and typeface substituted where it states them.
    pub style: RunStyle,
}

/// The marker `properties` gives a paragraph at `level`, or `None` when it draws none.
///
/// `text_style` is the style the paragraph's first run renders at, because `a:buSzTx` and
/// `a:buFontTx` both mean *whatever the text uses* and there is no other way to know what that is.
#[must_use]
pub fn marker_for(
    properties: &ParagraphPropertiesSpec,
    level: usize,
    text_style: &RunStyle,
    counters: &mut AutoNumberCounters,
) -> Option<Marker> {
    let bullet = properties.bullet()?;
    let text = match bullet {
        Bullet::None => {
            counters.interrupt(level);
            return None;
        }
        // R15: a picture bullet needs an image handle and a resource table. The indents still
        // apply, so the text sits where it will sit once the picture arrives.
        Bullet::Picture(_) => {
            counters.interrupt(level);
            return None;
        }
        Bullet::Character(character) => {
            counters.interrupt(level);
            character.character.clone()
        }
        Bullet::AutoNumber(auto) => auto_number_text(*auto, counters.next(level, auto.start_at)),
    };
    if text.is_empty() {
        return None;
    }

    let mut style = text_style.clone();
    if let Some(size) = properties.bullet_size() {
        style.size = bullet_size(size, text_style.size);
    }
    if let Some(BulletTypeface::Explicit(font)) = properties.bullet_typeface() {
        style.family.clone_from(&font.typeface);
    }
    Some(Marker { text, style })
}

/// The size a bullet draws at, given the size its text draws at.
#[must_use]
pub fn bullet_size(size: BulletSize, text_size: FontSize) -> FontSize {
    match size {
        BulletSize::FollowText => text_size,
        BulletSize::Percentage(fraction) => {
            let ratio = fraction.ratio();
            if ratio.is_finite() && ratio > 0.0 {
                FontSize::from_points(text_size.in_points() * ratio)
            } else {
                text_size
            }
        }
        // `a:buSzPts` is on the wire in hundredths of a point; the font engine's size is in
        // thousandths. Converting through points is the one place the two units meet.
        BulletSize::Points(points) => FontSize::from_points(points.points()),
    }
}

/// The characters an automatic bullet draws for item `number`.
///
/// The Latin schemes — Arabic numerals, Latin letters and Roman numerals, each with the period and
/// parenthesis punctuations — are spelled out. **Every other scheme falls back to plain Arabic
/// numerals**, and that is a stated limitation rather than an oversight: the East Asian, Thai,
/// Hebrew and Devanagari sequences each need their own numeral system, and a wrong glyph in the
/// right place is worse than a legible stand-in. The scheme is still carried on the paragraph, so
/// nothing is lost from the document.
#[must_use]
pub fn auto_number_text(bullet: AutoNumberBullet, number: u32) -> String {
    use AutonumberScheme as Scheme;
    let numeral = match bullet.scheme {
        Scheme::LowercaseLetterParenthesesBoth
        | Scheme::LowercaseLetterParenthesisRight
        | Scheme::LowercaseLetterPeriod => latin_letter(number, false),
        Scheme::UppercaseLetterParenthesesBoth
        | Scheme::UppercaseLetterParenthesisRight
        | Scheme::UppercaseLetterPeriod => latin_letter(number, true),
        Scheme::LowercaseRomanParenthesesBoth
        | Scheme::LowercaseRomanParenthesisRight
        | Scheme::LowercaseRomanPeriod => roman(number, false),
        Scheme::UppercaseRomanParenthesesBoth
        | Scheme::UppercaseRomanParenthesisRight
        | Scheme::UppercaseRomanPeriod => roman(number, true),
        _ => number.to_string(),
    };
    match bullet.scheme {
        Scheme::LowercaseLetterParenthesesBoth
        | Scheme::UppercaseLetterParenthesesBoth
        | Scheme::LowercaseRomanParenthesesBoth
        | Scheme::UppercaseRomanParenthesesBoth
        | Scheme::ArabicParenthesesBoth
        | Scheme::ThaiLetterParenthesesBoth
        | Scheme::ThaiNumberParenthesesBoth => format!("({numeral})"),
        Scheme::LowercaseLetterParenthesisRight
        | Scheme::UppercaseLetterParenthesisRight
        | Scheme::LowercaseRomanParenthesisRight
        | Scheme::UppercaseRomanParenthesisRight
        | Scheme::ArabicParenthesisRight
        | Scheme::ThaiLetterParenthesisRight
        | Scheme::ThaiNumberParenthesisRight
        | Scheme::HindiNumberParenthesisRight => format!("{numeral})"),
        Scheme::LowercaseLetterPeriod
        | Scheme::UppercaseLetterPeriod
        | Scheme::LowercaseRomanPeriod
        | Scheme::UppercaseRomanPeriod
        | Scheme::ArabicPeriod
        | Scheme::DoubleByteArabicPeriod
        | Scheme::SimplifiedChinesePeriod
        | Scheme::TraditionalChinesePeriod
        | Scheme::JapaneseDoubleBytePeriod
        | Scheme::JapaneseKoreanPeriod
        | Scheme::ThaiLetterPeriod
        | Scheme::ThaiNumberPeriod
        | Scheme::HindiVowelPeriod
        | Scheme::HindiNumberPeriod
        | Scheme::HindiConsonantPeriod => format!("{numeral}."),
        Scheme::BidirectionalArabicAlphabeticMinus
        | Scheme::BidirectionalArabicAbjadMinus
        | Scheme::BidirectionalHebrewMinus => format!("{numeral}-"),
        _ => numeral,
    }
}

/// `1 → a`, `26 → z`, `27 → aa`, the spreadsheet-column sequence Office numbers letters with.
fn latin_letter(number: u32, uppercase: bool) -> String {
    if number == 0 {
        return String::new();
    }
    let base = if uppercase { b'A' } else { b'a' };
    let mut letters = Vec::new();
    let mut remaining = number;
    while remaining > 0 {
        let index = (remaining - 1) % 26;
        // `index` is 0..=25, so the sum is within one uppercase or lowercase letter.
        letters.push(base + index as u8);
        remaining = (remaining - 1) / 26;
    }
    letters.reverse();
    String::from_utf8(letters).unwrap_or_default()
}

/// A Roman numeral, in the subtractive form Office writes.
///
/// Above 3999 there is no numeral, and the Arabic number is drawn instead — which is what a reader
/// would rather see than `MMMM…` repeated a thousand times.
fn roman(number: u32, uppercase: bool) -> String {
    const VALUES: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    if number == 0 || number > 3999 {
        return number.to_string();
    }
    let mut text = String::new();
    let mut remaining = number;
    for (value, numeral) in VALUES {
        while remaining >= value {
            text.push_str(numeral);
            remaining -= value;
        }
    }
    if uppercase {
        text.to_uppercase()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sequence_counts_up_from_its_start() {
        let mut counters = AutoNumberCounters::new();
        assert_eq!(counters.next(0, 1), 1);
        assert_eq!(counters.next(0, 1), 2);
        assert_eq!(counters.next(0, 1), 3);
        assert_eq!(counters.drawn_at(0), 3);
    }

    #[test]
    fn a_sequence_that_starts_at_five_draws_five_first() {
        let mut counters = AutoNumberCounters::new();
        assert_eq!(counters.next(2, 5), 5);
        assert_eq!(counters.next(2, 5), 6);
    }

    #[test]
    fn a_deeper_level_restarts_when_a_shallower_one_intervenes() {
        let mut counters = AutoNumberCounters::new();
        assert_eq!(counters.next(0, 1), 1);
        assert_eq!(counters.next(1, 1), 1);
        assert_eq!(counters.next(1, 1), 2);
        // Back out to the top level: the nested sequence ends.
        assert_eq!(counters.next(0, 1), 2);
        assert_eq!(
            counters.next(1, 1),
            1,
            "the nest restarts under its new parent"
        );
    }

    #[test]
    fn a_character_bullet_interrupts_the_nests_below_it_and_not_its_own() {
        let mut counters = AutoNumberCounters::new();
        assert_eq!(counters.next(0, 1), 1);
        assert_eq!(counters.next(1, 1), 1);
        counters.interrupt(0);
        assert_eq!(counters.next(1, 1), 1, "the nest restarted");
        assert_eq!(counters.next(0, 1), 2, "its own level did not");
    }

    #[test]
    fn the_latin_letter_sequence_carries_past_z() {
        assert_eq!(latin_letter(1, false), "a");
        assert_eq!(latin_letter(26, false), "z");
        assert_eq!(latin_letter(27, false), "aa");
        assert_eq!(latin_letter(28, true), "AB");
        assert_eq!(latin_letter(702, false), "zz");
        assert_eq!(latin_letter(703, false), "aaa");
    }

    #[test]
    fn roman_numerals_are_subtractive_and_give_up_above_the_range() {
        assert_eq!(roman(4, false), "iv");
        assert_eq!(roman(9, true), "IX");
        assert_eq!(roman(1994, true), "MCMXCIV");
        assert_eq!(roman(3999, false), "mmmcmxcix");
        assert_eq!(roman(4000, false), "4000");
    }

    #[test]
    fn each_punctuation_is_applied_to_its_numeral() {
        let cases = [
            (AutonumberScheme::ArabicPeriod, 3, "3."),
            (AutonumberScheme::ArabicParenthesisRight, 3, "3)"),
            (AutonumberScheme::ArabicParenthesesBoth, 3, "(3)"),
            (AutonumberScheme::ArabicPlain, 3, "3"),
            (AutonumberScheme::LowercaseLetterPeriod, 2, "b."),
            (AutonumberScheme::UppercaseLetterParenthesisRight, 2, "B)"),
            (AutonumberScheme::LowercaseRomanPeriod, 7, "vii."),
            (AutonumberScheme::UppercaseRomanParenthesesBoth, 7, "(VII)"),
            (AutonumberScheme::BidirectionalHebrewMinus, 4, "4-"),
        ];
        for (scheme, number, expected) in cases {
            assert_eq!(
                auto_number_text(AutoNumberBullet::new(scheme), number),
                expected,
                "for {scheme:?}"
            );
        }
    }

    #[test]
    fn an_unimplemented_scheme_falls_back_to_arabic_numerals() {
        assert_eq!(
            auto_number_text(
                AutoNumberBullet::new(AutonumberScheme::SimplifiedChinesePlain),
                12
            ),
            "12"
        );
    }

    #[test]
    fn a_bullet_size_is_a_proportion_of_the_text_or_a_size_of_its_own() {
        let text = FontSize::from_points(20.0);
        assert_eq!(bullet_size(BulletSize::FollowText, text), text);
        assert_eq!(
            bullet_size(BulletSize::percentage(0.5), text),
            FontSize::from_points(10.0)
        );
        assert_eq!(
            bullet_size(
                BulletSize::Points(mjx_dml::FontSize::from_points(8.0)),
                text
            ),
            FontSize::from_points(8.0)
        );
        // A file may state a nonsensical percentage; the text's own size is the honest answer.
        assert_eq!(bullet_size(BulletSize::percentage(0.0), text), text);
    }
}
