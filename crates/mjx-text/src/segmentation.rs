//! Grapheme clusters and word boundaries — the granularity a *user* moves through text at.
//!
//! A caret does not sit between bytes, or between `char`s. It sits between **extended grapheme
//! clusters**, as UAX #29 defines them: `é` written as `e` + `U+0301` is one place the caret can be
//! on either side of and nowhere in between, and so is a flag emoji built from two regional
//! indicators, and so is a Devanagari syllable with its matra.
//!
//! This is the granularity R11's caret and every selection extension in loop 2 will use, which is
//! why it is a named part of this crate's surface rather than a `unicode-segmentation` call at each
//! call site. A shaped run's clusters ([`crate::ShapedGlyph::cluster`]) are aligned to these, because
//! the shaper is asked for `MonotoneGraphemes`.
//!
//! # Word boundaries are not `split_whitespace`
//!
//! Double-clicking a word selects a UAX #29 *word*, which keeps `don't` together, keeps `3.14`
//! together, and puts a boundary between a Latin word and the Han ideograph beside it with no space
//! in between. Splitting on whitespace gets all three wrong.

use unicode_segmentation::UnicodeSegmentation;

/// The extended grapheme clusters of `text`, each with its byte offset.
pub fn grapheme_clusters(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.grapheme_indices(true)
}

/// How many extended grapheme clusters `text` holds — the number of places a caret can stop, less
/// one.
#[must_use]
pub fn grapheme_cluster_count(text: &str) -> usize {
    text.graphemes(true).count()
}

/// Every byte offset a caret may sit at, ascending, including `0` and `text.len()`.
///
/// An empty string yields a single boundary at `0`, because a caret can still be in it.
#[must_use]
pub fn grapheme_cluster_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::with_capacity(text.len() / 2 + 1);
    boundaries.push(0);
    for (offset, cluster) in text.grapheme_indices(true) {
        boundaries.push(offset + cluster.len());
    }
    boundaries
}

/// The next place a caret may stop after `from`, or `None` at the end of the text.
///
/// An offset that is not on a character boundary, or is past the end, answers `None` rather than
/// panicking: a caret position can arrive from a host that measured in the wrong units.
#[must_use]
pub fn next_grapheme_boundary(text: &str, from: usize) -> Option<usize> {
    if from >= text.len() || !text.is_char_boundary(from) {
        return None;
    }
    text[from..]
        .graphemes(true)
        .next()
        .map(|cluster| from + cluster.len())
}

/// The previous place a caret may stop before `from`, or `None` at the start of the text.
#[must_use]
pub fn previous_grapheme_boundary(text: &str, from: usize) -> Option<usize> {
    if from == 0 || from > text.len() || !text.is_char_boundary(from) {
        return None;
    }
    text[..from]
        .graphemes(true)
        .next_back()
        .map(|cluster| from - cluster.len())
}

/// The UAX #29 words of `text`, each with its byte offset. Punctuation and whitespace are words too;
/// [`words`] is the filtered form.
pub fn word_segments(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_word_bound_indices()
}

/// The UAX #29 words of `text` that hold a letter or a digit — what a double-click selects.
pub fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_word_bound_indices()
        .filter(|(_, word)| word.chars().any(char::is_alphanumeric))
}

/// The word `at` falls inside, as a byte range, or `None` when it falls in whitespace or
/// punctuation.
///
/// An offset resting immediately **after** a word belongs to that word: a double-click at the
/// right-hand edge of `select` selects `select`, and a caret that has just walked to the end of a
/// word is still in it.
#[must_use]
pub fn word_at(text: &str, at: usize) -> Option<std::ops::Range<usize>> {
    if at > text.len() || !text.is_char_boundary(at) {
        return None;
    }
    words(text)
        .map(|(offset, word)| offset..offset + word.len())
        .find(|range| range.contains(&at) || range.end == at)
}
