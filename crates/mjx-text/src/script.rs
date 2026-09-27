//! Which writing system a character belongs to, and where one run of text stops being one.
//!
//! A shaping call sees **one script**. HarfBuzz — and therefore `rustybuzz` — selects a whole
//! shaping engine from the script: Arabic joining, Indic reordering, Khmer and Myanmar cluster
//! rules, the Hangul jamo composer, and the default engine for everything else. Handing it a string
//! that is half Latin and half Devanagari does not produce half of each; it produces whichever
//! engine the buffer's script happened to name, applied to all of it.
//!
//! So a paragraph is cut into runs before anything is shaped: by embedding level
//! ([`crate::BidiAnalysis`]), then by script (here), then by face
//! ([`crate::itemise`]). This module is the middle cut.
//!
//! # `Common` and `Inherited` extend, they do not split
//!
//! A space, a full stop, a digit and a combining mark have no script of their own — ISO 15924 gives
//! them `Zyyy` (Common) or `Zinh` (Inherited). Splitting a run at every space would defeat the point
//! of shaping (kerning across the space would be lost, and so would any contextual substitution
//! spanning it), so those characters **join the run they are next to**, which is the rule UAX #24
//! §5.1 describes and every shaping engine implements.

use std::ops::Range;

/// A writing system, named by its ISO 15924 four-letter code.
///
/// The code is stored rather than an enumeration of the 160-odd registered scripts: an enumeration
/// would have to grow with every Unicode release, and this crate never switches on the script
/// itself — it passes it to the shaper and asks it two questions, [`TextScript::code`] and
/// [`TextScript::is_right_to_left`].
///
/// Codes are in ISO 15924's own casing — an initial capital and three lowercase letters, `Latn`,
/// `Arab`, `Deva` — which is the form `rustybuzz` reads.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextScript([u8; 4]);

impl TextScript {
    /// `Zyyy` — a character used by many scripts: a space, a digit, most punctuation.
    pub const COMMON: Self = Self(*b"Zyyy");
    /// `Zinh` — a character that takes the script of what it is attached to: a combining mark.
    pub const INHERITED: Self = Self(*b"Zinh");
    /// `Zzzz` — an unassigned or private-use code point.
    pub const UNKNOWN: Self = Self(*b"Zzzz");
    /// `Latn`.
    pub const LATIN: Self = Self(*b"Latn");
    /// `Grek`.
    pub const GREEK: Self = Self(*b"Grek");
    /// `Cyrl`.
    pub const CYRILLIC: Self = Self(*b"Cyrl");
    /// `Arab` — right-to-left, and the script whose joining forms make positional shaping visible.
    pub const ARABIC: Self = Self(*b"Arab");
    /// `Hebr` — right-to-left.
    pub const HEBREW: Self = Self(*b"Hebr");
    /// `Deva` — Devanagari, whose pre-base vowel signs are drawn before the consonant they follow.
    pub const DEVANAGARI: Self = Self(*b"Deva");
    /// `Beng`.
    pub const BENGALI: Self = Self(*b"Beng");
    /// `Thai`.
    pub const THAI: Self = Self(*b"Thai");
    /// `Hani` — the Han ideographs, shared by Chinese, Japanese and Korean.
    pub const HAN: Self = Self(*b"Hani");
    /// `Hira` — Japanese hiragana.
    pub const HIRAGANA: Self = Self(*b"Hira");
    /// `Kana` — Japanese katakana.
    pub const KATAKANA: Self = Self(*b"Kana");
    /// `Hang` — Korean hangul.
    pub const HANGUL: Self = Self(*b"Hang");

    /// The script named by an ISO 15924 code.
    ///
    /// Any four bytes are accepted: the register grows, and refusing a code this build has not heard
    /// of would make a document unreadable rather than merely unshaped.
    #[must_use]
    pub const fn from_iso_15924_code(code: [u8; 4]) -> Self {
        Self(code)
    }

    /// The ISO 15924 code, as text.
    #[must_use]
    pub fn code(&self) -> &str {
        // Every construction path produces four ASCII letters: the constants above are literals,
        // and `of_character` takes its bytes from `unicode-script`'s own table of ISO 15924 codes.
        // A caller that hands `from_iso_15924_code` something else gets the replacement character
        // rather than a panic, because this is the untrusted-input path.
        std::str::from_utf8(&self.0).unwrap_or("Zzzz")
    }

    /// The script the character belongs to, by the Unicode `Script` property.
    #[must_use]
    pub fn of_character(character: char) -> Self {
        let name = unicode_script::Script::from(character).short_name();
        let bytes = name.as_bytes();
        match bytes {
            [a, b, c, d] => Self([*a, *b, *c, *d]),
            // Every ISO 15924 code is four letters, so this is unreachable in practice; answering
            // `Zzzz` rather than asserting keeps the promise that nothing here panics on text.
            _ => Self::UNKNOWN,
        }
    }

    /// Whether the script is written right to left.
    ///
    /// The list is UAX #9's: every script whose letters have the `R` bidirectional character type
    /// (as opposed to `AL`, which is the Arabic-like subset). It is used to pick a shaping direction
    /// for a run whose embedding level is even but whose script is right-to-left — which happens
    /// whenever a right-to-left word is quoted inside a left-to-right paragraph with the
    /// bidirectional algorithm switched off.
    #[must_use]
    pub fn is_right_to_left(&self) -> bool {
        matches!(
            &self.0,
            b"Adlm"
                | b"Arab"
                | b"Armi"
                | b"Avst"
                | b"Chrs"
                | b"Cprt"
                | b"Elym"
                | b"Hatr"
                | b"Hebr"
                | b"Hung"
                | b"Khar"
                | b"Lydi"
                | b"Mand"
                | b"Mani"
                | b"Medf"
                | b"Mend"
                | b"Merc"
                | b"Mero"
                | b"Narb"
                | b"Nbat"
                | b"Nkoo"
                | b"Orkh"
                | b"Ougr"
                | b"Palm"
                | b"Phli"
                | b"Phlp"
                | b"Phnx"
                | b"Prti"
                | b"Rohg"
                | b"Samr"
                | b"Sarb"
                | b"Sogd"
                | b"Sogo"
                | b"Syrc"
                | b"Thaa"
                | b"Ugar"
                | b"Yezi"
        )
    }

    /// Whether the script is one that carries no direction of its own — `Zyyy`, `Zinh` or `Zzzz`.
    #[must_use]
    pub fn is_undetermined(&self) -> bool {
        *self == Self::COMMON || *self == Self::INHERITED || *self == Self::UNKNOWN
    }
}

impl std::fmt::Debug for TextScript {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "TextScript({})", self.code())
    }
}

impl std::fmt::Display for TextScript {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A stretch of text in one script.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScriptRun {
    /// The bytes it covers.
    pub range: Range<usize>,
    /// The script every character in it belongs to, once `Zyyy`/`Zinh` have been resolved.
    pub script: TextScript,
}

/// Cut `text` into runs of one script each.
///
/// `Zyyy` (Common) and `Zinh` (Inherited) characters extend whichever run they touch: they join the
/// run before them, or — when they open the text — the run that first follows. Text that is nothing
/// but undetermined characters comes back as one [`TextScript::COMMON`] run, which is the right
/// answer for a paragraph of digits and spaces.
///
/// The returned runs partition `text` exactly: they are in order, adjacent, and cover every byte.
#[must_use]
pub fn itemise_by_script(text: &str) -> Vec<ScriptRun> {
    let mut runs: Vec<ScriptRun> = Vec::new();
    if text.is_empty() {
        return runs;
    }

    let mut current: Option<(usize, TextScript)> = None;
    // Bytes seen before any determined script, waiting to be given to the first real run.
    let mut leading_undetermined: Option<usize> = None;

    for (offset, character) in text.char_indices() {
        let script = TextScript::of_character(character);
        if script.is_undetermined() {
            if current.is_none() && leading_undetermined.is_none() {
                leading_undetermined = Some(offset);
            }
            continue;
        }
        match current {
            Some((_, open)) if open == script => {}
            Some((start, open)) => {
                runs.push(ScriptRun {
                    range: start..offset,
                    script: open,
                });
                current = Some((offset, script));
            }
            None => {
                current = Some((leading_undetermined.take().unwrap_or(offset), script));
            }
        }
    }

    match current {
        Some((start, script)) => runs.push(ScriptRun {
            range: start..text.len(),
            script,
        }),
        None => runs.push(ScriptRun {
            range: leading_undetermined.unwrap_or(0)..text.len(),
            script: TextScript::COMMON,
        }),
    }
    runs
}

/// Cut `range` of `text` into runs of one script each, in the same way [`itemise_by_script`] cuts a
/// whole string.
///
/// An out-of-range or non-boundary `range` produces no runs rather than a panic.
#[must_use]
pub fn itemise_range_by_script(text: &str, range: Range<usize>) -> Vec<ScriptRun> {
    if range.start > range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Vec::new();
    }
    itemise_by_script(&text[range.clone()])
        .into_iter()
        .map(|run| ScriptRun {
            range: range.start + run.range.start..range.start + run.range.end,
            script: run.script,
        })
        .collect()
}
