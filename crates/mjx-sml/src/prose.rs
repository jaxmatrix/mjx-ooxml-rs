//! Holding this crate's own documentation to what its code does — test-only.
//!
//! # Why a module rather than a paragraph in each test
//!
//! Two content models in this crate divide their slots into *modelled* and *held*, and both write
//! that division down in prose beside the code that makes it: `CT_Worksheet`'s thirty-nine slots in
//! [`crate::worksheet`], and the three sheet kinds' in [`crate::sheets`]. **Every one of those
//! figures has been wrong at least once.** `CT_Worksheet`'s was written as 25/14, then 31/8, then
//! 34/5, then 35/4 across four children — MJXOFF-88 §9 B2 — and at 0.0.138 `dialogsheet.rs` said
//! *eleven* of its sixteen slots were modelled where the reader types ten, and *five held* directly
//! above a list of six.
//!
//! A number in prose is a claim, and a claim a test cannot read is a claim that expires silently.
//! So each of those files derives its split from its own read path and then hands the prose here to
//! be checked against it. What lives in this module is only the reading of English: the derivation
//! stays beside the type it describes, because that is what has to change when the type does.
//!
//! Nothing here is compiled into the library.

/// The English for `0..=39`, so a count can be compared against prose that spells it out.
///
/// As far as the widest sequence in the schema (`CT_Worksheet`) and no further: a number this table
/// cannot spell is a number this crate's documentation has no business writing down.
pub(crate) const NUMBER_WORDS: [&str; 40] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "twenty-one",
    "twenty-two",
    "twenty-three",
    "twenty-four",
    "twenty-five",
    "twenty-six",
    "twenty-seven",
    "twenty-eight",
    "twenty-nine",
    "thirty",
    "thirty-one",
    "thirty-two",
    "thirty-three",
    "thirty-four",
    "thirty-five",
    "thirty-six",
    "thirty-seven",
    "thirty-eight",
    "thirty-nine",
];

/// The leading run of `//!` lines in `source` — a file's module documentation and nothing else.
///
/// One file may document more than one content model: `sheets/chartsheet.rs` carries
/// `CT_Chartsheet`'s fourteen slots in its header and `CT_CustomChartsheetView`'s three on an item
/// halfway down, and a whole-file scan reads the second as a claim about the first. The header is
/// where a part's own split is stated, so that is what is scanned.
pub(crate) fn module_documentation(source: &str) -> String {
    source
        .lines()
        .take_while(|line| line.starts_with("//!") || line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `text` contains `phrase` as whole words.
///
/// A plain `contains` cannot be used for a number word: the spelling of 9 is a tail of the spelling
/// of 39, and the spelling of 5 a tail of the spelling of 25, so a hyphenated word would match
/// inside itself and report the wrong figure. A match therefore has to begin and end at a boundary,
/// and a hyphen counts as part of the word rather than as one.
pub(crate) fn contains_phrase(text: &str, phrase: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(offset) = text[from..].find(phrase) {
        let at = from + offset;
        let before_is_boundary =
            at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'-');
        let end = at + phrase.len();
        let after_is_boundary =
            end == bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-');
        if before_is_boundary && after_is_boundary {
            return true;
        }
        from = at + 1;
    }
    false
}

/// `text` as one lower-case run of prose, with the comment markers and the emphasis removed.
///
/// Three things would otherwise let a claim slip past [`contains_phrase`], and all three are how
/// this crate's documentation is actually written: a count is usually **bold**, so the phrase is
/// `**four** held` rather than `four held`; an element name is in backticks; and a sentence wraps
/// across `//!` lines wherever it reaches the column limit. Normalising once is cheaper than
/// teaching the matcher about markdown.
pub(crate) fn normalised_prose(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_start();
        let line = line
            .strip_prefix("//!")
            .or_else(|| line.strip_prefix("///"))
            .or_else(|| line.strip_prefix("//"))
            .unwrap_or(line);
        for character in line.chars() {
            match character {
                '*' | '`' => {}
                character if character.is_whitespace() => {
                    if !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                character => out.extend(character.to_lowercase()),
            }
        }
        if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out
}

/// Checks every spelled-out count in `text` that stands in front of one of `claims`' nouns, and
/// answers how many claims it read.
///
/// A number word in front of a noun this crate uses for a count is a claim about the model. The
/// scan is over *every* spelling, not over the expected one, which is what makes a **wrong** number
/// fail rather than merely a missing one: a file that says `eleven modelled` where the reader types
/// ten fails here, and a file that says nothing at all is caught by the caller's floor instead.
///
/// # Panics
/// With the file, the phrase and the derived figure, when the two disagree.
pub(crate) fn check_counts(file: &str, text: &str, claims: &[(&str, usize)]) -> usize {
    let prose = normalised_prose(text);
    let mut checked = 0usize;
    for (noun, right) in claims {
        for (value, word) in NUMBER_WORDS.iter().enumerate() {
            let phrase = format!("{word} {noun}");
            if !contains_phrase(&prose, &phrase) {
                continue;
            }
            checked += 1;
            assert_eq!(
                value, *right,
                "{file} says \"{phrase}\", and the read path says {right}"
            );
        }
    }
    checked
}
