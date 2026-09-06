//! Where a word may be split across a line ending.
//!
//! Hyphenation is a break opportunity that is not in the text: UAX #14 offers a break after an
//! existing hyphen, but *inserting* one inside `hy-phen-a-tion` needs knowledge of the language.
//! Word models it as `w:autoHyphenation`, `w:hyphenationZone` and `w:suppressAutoHyphens`, and it is
//! off by default in every shipped template — so the interface is what matters now and the switch is
//! the reflow engine's.
//!
//! # Two implementations, both complete
//!
//! [`SoftHyphenHyphenator`] honours `U+00AD SOFT HYPHEN`, which is what a Word author inserts with
//! Ctrl+Hyphen and what a converted document carries. It needs no language data at all and it is
//! **not** a stand-in for the pattern algorithm: an author-placed soft hyphen outranks any pattern,
//! so this is a rule Word applies whether automatic hyphenation is on or off.
//!
//! [`PatternHyphenator`] is Frank Liang's algorithm — the one TeX uses, and the one every word
//! processor's automatic hyphenation descends from. It is complete here: the pattern trie, the
//! odd-value rule, the exception dictionary, and the left and right minima Word calls the
//! hyphenation zone's edges.
//!
//! **No language's patterns are shipped with it**, and that is a deliberate boundary rather than an
//! unfinished piece. A pattern set is a licensed data file — the TeX `hyph-*` collection carries a
//! different licence per language — and committing one is a repository-owner decision of the same
//! kind as the bundled font faces in `assets/fonts/`. [`HyphenationPatterns::from_tex_patterns`]
//! reads the format those files are written in, so supplying one is a data decision and not a code
//! change.

use std::collections::HashMap;
use std::fmt;

/// Something that can say where a word may be split.
///
/// The offsets are byte offsets **into the word**, at each of which a hyphen would be drawn and the
/// remainder moved to the next line. They are always strictly inside the word.
pub trait Hyphenator: fmt::Debug {
    /// Append every offset inside `word` at which it may be split, ascending, to `points`.
    ///
    /// Implementations must not clear `points`: a caller may be accumulating across a compound.
    fn hyphenation_points(&self, word: &str, points: &mut Vec<usize>);

    /// The same, into a fresh vector.
    fn hyphenation_points_of(&self, word: &str) -> Vec<usize> {
        let mut points = Vec::new();
        self.hyphenation_points(word, &mut points);
        points
    }
}

/// Never splits a word — `w:suppressAutoHyphens`, and the state every Word template ships in.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct NoHyphenation;

impl Hyphenator for NoHyphenation {
    fn hyphenation_points(&self, _word: &str, _points: &mut Vec<usize>) {}
}

/// Splits only where the author put a `U+00AD SOFT HYPHEN`.
///
/// The soft hyphen is invisible unless the line ends at it, which is exactly what a hyphenation
/// point is. Nothing about it is language-dependent, so this hyphenator is complete without any
/// data at all.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct SoftHyphenHyphenator;

/// `U+00AD SOFT HYPHEN` — the author's own hyphenation point.
pub const SOFT_HYPHEN: char = '\u{00AD}';

impl Hyphenator for SoftHyphenHyphenator {
    fn hyphenation_points(&self, word: &str, points: &mut Vec<usize>) {
        for (offset, character) in word.char_indices() {
            if character == SOFT_HYPHEN && offset > 0 && offset + character.len_utf8() < word.len()
            {
                points.push(offset);
            }
        }
    }
}

/// A set of Liang hyphenation patterns and exceptions.
///
/// A pattern is a string of letters with digits between them, `hy3ph`, read as: wherever `hyph`
/// occurs, the position between `y` and `p` scores 3. Odd totals are hyphenation points and even
/// totals are not, which is the whole of the rule. A `.` at either end anchors the pattern to the
/// start or the end of the word.
#[derive(Clone, Default, Debug)]
pub struct HyphenationPatterns {
    /// Pattern letters (with the anchoring dots kept) mapped to the score at each position; the
    /// scores vector is one longer than the letters, because a score sits *between* letters.
    patterns: HashMap<Box<str>, Box<[u8]>>,
    /// Words whose hyphenation the pattern set gets wrong, spelled with `-` at each point.
    exceptions: HashMap<Box<str>, Box<[usize]>>,
    longest_pattern: usize,
}

impl HyphenationPatterns {
    /// An empty set. A [`PatternHyphenator`] built on it never splits anything, which is the honest
    /// answer for a language whose patterns have not been supplied.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read patterns in the TeX `\patterns{...}` format: one pattern per whitespace-separated token,
    /// digits interleaved with letters, `.` anchoring to a word edge.
    ///
    /// Lines beginning with `%` are comments, as they are in a `.tex` file. Tokens that hold no
    /// letter at all are skipped rather than refused, because a pattern file may carry stray
    /// punctuation.
    #[must_use]
    pub fn from_tex_patterns(source: &str) -> Self {
        let mut set = Self::new();
        for line in source.lines() {
            let line = line.split('%').next().unwrap_or("");
            for token in line.split_whitespace() {
                set.add_pattern(token);
            }
        }
        set
    }

    /// Add one pattern, in the same notation.
    pub fn add_pattern(&mut self, pattern: &str) {
        let mut letters = String::with_capacity(pattern.len());
        let mut scores: Vec<u8> = vec![0];
        for character in pattern.chars() {
            if let Some(digit) = character.to_digit(10) {
                // `to_digit(10)` yields 0..=9, so the cast cannot lose anything.
                if let Some(last) = scores.last_mut() {
                    *last = digit as u8;
                }
            } else {
                letters.push(character);
                scores.push(0);
            }
        }
        if letters.is_empty() {
            return;
        }
        self.longest_pattern = self.longest_pattern.max(letters.chars().count());
        self.patterns
            .insert(Box::from(letters.as_str()), scores.into_boxed_slice());
    }

    /// Add an exception, spelled with `-` at each hyphenation point: `pre-sent`.
    pub fn add_exception(&mut self, spelling: &str) {
        let mut word = String::with_capacity(spelling.len());
        let mut points = Vec::new();
        for character in spelling.chars() {
            if character == '-' {
                points.push(word.len());
            } else {
                word.push(character);
            }
        }
        if word.is_empty() {
            return;
        }
        self.exceptions
            .insert(Box::from(word.as_str()), points.into_boxed_slice());
    }

    /// Read exceptions in the TeX `\hyphenation{...}` format: one hyphenated spelling per
    /// whitespace-separated token.
    pub fn add_tex_exceptions(&mut self, source: &str) {
        for line in source.lines() {
            let line = line.split('%').next().unwrap_or("");
            for token in line.split_whitespace() {
                self.add_exception(token);
            }
        }
    }

    /// How many patterns the set holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Whether the set holds no patterns at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// How many exceptions the set holds.
    #[must_use]
    pub fn exception_count(&self) -> usize {
        self.exceptions.len()
    }
}

/// Liang's algorithm over a [`HyphenationPatterns`].
///
/// `minimum_prefix` and `minimum_suffix` are the least number of characters that must stay on each
/// line — Word's own defaults for English are 2 and 3, and TeX's `\lefthyphenmin`/`\righthyphenmin`
/// are the same two numbers.
#[derive(Clone, Debug)]
pub struct PatternHyphenator {
    patterns: HyphenationPatterns,
    minimum_prefix: usize,
    minimum_suffix: usize,
}

impl PatternHyphenator {
    /// A hyphenator over `patterns`, keeping at least two characters before a break and three
    /// after.
    #[must_use]
    pub fn new(patterns: HyphenationPatterns) -> Self {
        Self {
            patterns,
            minimum_prefix: 2,
            minimum_suffix: 3,
        }
    }

    /// The same hyphenator with different minima. Both are clamped to at least one, because a break
    /// that leaves nothing on a line is not a break.
    #[must_use]
    pub fn with_minima(mut self, minimum_prefix: usize, minimum_suffix: usize) -> Self {
        self.minimum_prefix = minimum_prefix.max(1);
        self.minimum_suffix = minimum_suffix.max(1);
        self
    }

    /// The patterns it works from.
    #[must_use]
    pub fn patterns(&self) -> &HyphenationPatterns {
        &self.patterns
    }
}

impl Hyphenator for PatternHyphenator {
    fn hyphenation_points(&self, word: &str, points: &mut Vec<usize>) {
        if word.is_empty() {
            return;
        }
        // Liang's algorithm is defined over the lowercased word with a dot at each end, and the
        // scores are indexed by *character* position, not by byte.
        let lowercased: String = word.chars().flat_map(char::to_lowercase).collect();
        if let Some(exception) = self.patterns.exceptions.get(lowercased.as_str()) {
            // The exception's offsets are into the lowercased spelling. Lowercasing can change a
            // string's length (`İ` becomes two characters), so an exception is only usable when it
            // did not.
            if lowercased.len() == word.len() {
                points.extend(
                    exception
                        .iter()
                        .copied()
                        .filter(|offset| *offset > 0 && *offset < word.len()),
                );
            }
            return;
        }

        let anchored = format!(".{lowercased}.");
        let characters: Vec<char> = anchored.chars().collect();
        // One score between each pair of characters, plus the ends.
        let mut scores = vec![0_u8; characters.len() + 1];

        for start in 0..characters.len() {
            let mut fragment = String::new();
            for character in characters.iter().skip(start) {
                fragment.push(*character);
                if fragment.chars().count() > self.patterns.longest_pattern {
                    break;
                }
                let Some(pattern_scores) = self.patterns.patterns.get(fragment.as_str()) else {
                    continue;
                };
                for (offset, score) in pattern_scores.iter().enumerate() {
                    if let Some(existing) = scores.get_mut(start + offset) {
                        *existing = (*existing).max(*score);
                    }
                }
            }
        }

        // Character index `i` of `anchored` is character index `i - 1` of the word, because of the
        // leading dot. A score at `scores[i]` sits *before* `anchored`'s character `i`.
        let word_characters: Vec<(usize, char)> = word.char_indices().collect();
        let count = word_characters.len();
        for (index, (byte_offset, _)) in word_characters.iter().enumerate() {
            if index < self.minimum_prefix || count.saturating_sub(index) < self.minimum_suffix {
                continue;
            }
            // `scores[index + 1]` is the score before the word's character `index`.
            if scores.get(index + 1).is_some_and(|score| score % 2 == 1) {
                points.push(*byte_offset);
            }
        }
    }
}
