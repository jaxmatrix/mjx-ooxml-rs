//! The substitution table, and the tier-3 fetch policy.
//!
//! `docs/UI_PLATFORM_PLAN.md` §10 puts a substitution table *underneath* all three tiers: it is
//! consulted **before** any blind fallback, because a blind fallback that lands on a face with
//! different advances re-paginates the document, and one that lands on the metric-compatible clone
//! does not. This module is that table, plus the policy that decides which subset tier 3 would
//! fetch when no local face can serve a script at all.
//!
//! # Provenance
//!
//! The metric-compatible rows are not invented here. They are the pairings
//! `fontconfig`'s `30-metric-aliases.conf` declares — the file every Linux desktop already resolves
//! Office documents through — whose header tabulates them as:
//!
//! ```text
//! Microsoft fonts:  Liberation fonts:       Google CrOS core fonts:  StarOffice fonts:  AMT fonts:
//! Arial             Liberation Sans         Arimo                    Albany             Albany AMT
//! Arial Narrow      Liberation Sans Narrow
//! Times New Roman   Liberation Serif        Tinos                    Thorndale          Thorndale AMT
//! Courier New       Liberation Mono         Cousine                  Cumberland         Cumberland AMT
//! Cambria                                   Caladea
//! Calibri                                   Carlito
//! Symbol                                    SymbolNeu
//! Georgia           Gelasio
//! ```
//!
//! …together with the PostScript base-35 pairings from the same file (Helvetica → Nimbus Sans,
//! Times → Nimbus Roman, Courier → Nimbus Mono PS). Substitutes are listed in preference order.
//!
//! # What is deliberately *not* here
//!
//! No "looks a bit like it" row. Segoe UI, Consolas, Candara, Corbel and Constantia have no
//! metric-compatible clone, and inventing one would put a face with different advances behind a
//! table whose whole promise is that the advances match. A document naming one of those falls
//! through to the generic families below and the manifest records the substitution as
//! [`crate::MetricCompatibility::Unverified`], which is the truth.

/// One family, and the faces that may stand in for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SubstitutionRule {
    /// The family a document asks for.
    pub original: &'static str,
    /// What may be used instead, best first.
    pub substitutes: &'static [&'static str],
    /// Whether the substitutes were drawn to the original's advance widths. `false` means the
    /// substitute is a *stylistic* stand-in and text set in it will break into different lines.
    pub is_metric_compatible: bool,
}

/// Every substitution this crate knows, in the order [`substitution_for_family`] searches.
pub static SUBSTITUTION_TABLE: &[SubstitutionRule] = &[
    // --- Microsoft core fonts, metric-compatible clones. ---
    SubstitutionRule {
        original: "Arial",
        substitutes: &["Liberation Sans", "Arimo", "Albany", "Albany AMT"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Arial Narrow",
        substitutes: &["Liberation Sans Narrow"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Times New Roman",
        substitutes: &["Liberation Serif", "Tinos", "Thorndale", "Thorndale AMT"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Courier New",
        substitutes: &["Liberation Mono", "Cousine", "Cumberland", "Cumberland AMT"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Calibri",
        substitutes: &["Carlito"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Cambria",
        substitutes: &["Caladea"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Georgia",
        substitutes: &["Gelasio"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Symbol",
        substitutes: &["SymbolNeu"],
        is_metric_compatible: true,
    },
    // --- The PostScript base-35, whose clones are metric-compatible by construction. ---
    SubstitutionRule {
        original: "Helvetica",
        substitutes: &["Nimbus Sans", "TeX Gyre Heros", "Liberation Sans", "Arial"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Times",
        substitutes: &[
            "Nimbus Roman",
            "TeX Gyre Termes",
            "Liberation Serif",
            "Times New Roman",
        ],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Courier",
        substitutes: &[
            "Nimbus Mono PS",
            "TeX Gyre Cursor",
            "Liberation Mono",
            "Courier New",
        ],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "Palatino",
        substitutes: &["P052", "TeX Gyre Pagella", "Palatino Linotype"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "New Century Schoolbook",
        substitutes: &["C059", "TeX Gyre Schola", "Century Schoolbook"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "ITC Bookman",
        substitutes: &["URW Bookman", "TeX Gyre Bonum", "Bookman Old Style"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "ITC Avant Garde Gothic",
        substitutes: &["URW Gothic", "TeX Gyre Adventor"],
        is_metric_compatible: true,
    },
    SubstitutionRule {
        original: "ITC Zapf Chancery",
        substitutes: &["Z003", "TeX Gyre Chorus"],
        is_metric_compatible: true,
    },
];

/// The rule for `family`, matched case-insensitively.
#[must_use]
pub fn substitution_for_family(family: &str) -> Option<&'static SubstitutionRule> {
    SUBSTITUTION_TABLE
        .iter()
        .find(|rule| rule.original.eq_ignore_ascii_case(family))
}

/// The CSS generic families a document or a token stack can name in place of a real face.
///
/// A [`mjx_tokens::FontStack`] ends in one of these by construction — that is what a CSS font list
/// is for — so resolving one is not an edge case but the normal end of every stack.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GenericFamily {
    /// `sans-serif`, and `system-ui`/`ui-sans-serif` which resolve to the platform's own.
    SansSerif,
    /// `serif` and `ui-serif`.
    Serif,
    /// `monospace` and `ui-monospace`.
    Monospace,
    /// `cursive`.
    Cursive,
    /// `fantasy`.
    Fantasy,
}

impl GenericFamily {
    /// The generic a CSS keyword names, or `None` when the name is a real family.
    #[must_use]
    pub fn from_css_keyword(name: &str) -> Option<Self> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "sans-serif" | "system-ui" | "ui-sans-serif" | "ui-rounded" => Self::SansSerif,
            "serif" | "ui-serif" => Self::Serif,
            "monospace" | "ui-monospace" => Self::Monospace,
            "cursive" => Self::Cursive,
            "fantasy" => Self::Fantasy,
            _ => return None,
        })
    }

    /// Concrete families to try for this generic, best first. The list leads with the
    /// metric-compatible clones this crate bundles, so that a generic resolved on a machine with no
    /// system fonts at all still lands on a face whose numbers are known.
    #[must_use]
    pub fn candidate_families(self) -> &'static [&'static str] {
        match self {
            Self::SansSerif => &[
                "Liberation Sans",
                "Carlito",
                "Arial",
                "Helvetica",
                "DejaVu Sans",
                "Noto Sans",
            ],
            Self::Serif => &[
                "Liberation Serif",
                "Caladea",
                "Times New Roman",
                "DejaVu Serif",
                "Noto Serif",
            ],
            Self::Monospace => &[
                "Liberation Mono",
                "Courier New",
                "DejaVu Sans Mono",
                "Noto Sans Mono",
            ],
            Self::Cursive => &["Comic Neue", "URW Chancery L", "Z003"],
            Self::Fantasy => &["Impact", "Noto Sans Display"],
        }
    }
}

/// An inclusive span of code points.
///
/// A plain `RangeInclusive<char>` would do the job, but it is neither `Copy` nor `Eq`, so a table
/// built from it could be neither, and `clippy::single_range_in_vec_init` reads a one-element array
/// of them as a mistyped range. A two-field struct is smaller, comparable and unambiguous.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CharacterRange {
    /// The first code point in the span.
    pub first: char,
    /// The last code point in the span, included.
    pub last: char,
}

impl CharacterRange {
    /// A span from `first` to `last`, both included.
    #[must_use]
    pub const fn new(first: char, last: char) -> Self {
        Self { first, last }
    }

    /// Whether `character` falls in the span.
    #[must_use]
    pub fn contains(self, character: char) -> bool {
        self.first <= character && character <= self.last
    }
}

/// A font this platform would fetch on demand rather than bundle.
///
/// §10's third tier: full coverage of CJK, Indic and Arabic is more than a hundred megabytes, which
/// cannot ship in an application bundle on every platform. This table is the *policy* half of that
/// tier — which subset answers for which characters, and how large it is. **There is no transport
/// in this loop**, so a resolution that lands here produces a [`FetchPlan`] rather than a face, and
/// the caller decides what to do with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FetchableSubset {
    /// The family the fetched subset would provide.
    pub family: &'static str,
    /// The writing system it covers, named for a person rather than for a tag.
    pub writing_system: &'static str,
    /// The Unicode ranges it answers for.
    pub ranges: &'static [CharacterRange],
    /// Roughly how large the subset is, in bytes. This is why it is not bundled, and it is what a
    /// user interface shows before asking whether to download it.
    pub approximate_bytes: u32,
}

impl FetchableSubset {
    /// Whether this subset covers `character`.
    #[must_use]
    pub fn covers(&self, character: char) -> bool {
        self.ranges.iter().any(|range| range.contains(character))
    }
}

/// The subsets tier 3 can fetch, in the order [`fetchable_subset_for_character`] searches.
///
/// Sizes are the published sizes of the corresponding Noto releases, rounded; they are indicative
/// of the order of magnitude, which is the decision they inform.
pub static FETCHABLE_SUBSETS: &[FetchableSubset] = &[
    FetchableSubset {
        family: "Noto Sans SC",
        writing_system: "Han, Simplified Chinese",
        ranges: &[
            CharacterRange::new('\u{2E80}', '\u{2EFF}'), // CJK radicals supplement
            CharacterRange::new('\u{3000}', '\u{303F}'), // CJK symbols and punctuation
            CharacterRange::new('\u{3400}', '\u{4DBF}'), // CJK unified ideographs extension A
            CharacterRange::new('\u{4E00}', '\u{9FFF}'), // CJK unified ideographs
            CharacterRange::new('\u{F900}', '\u{FAFF}'), // CJK compatibility ideographs
            CharacterRange::new('\u{20000}', '\u{2A6DF}'), // extension B
        ],
        approximate_bytes: 10_000_000,
    },
    FetchableSubset {
        family: "Noto Sans JP",
        writing_system: "Japanese kana",
        ranges: &[
            CharacterRange::new('\u{3040}', '\u{309F}'), // hiragana
            CharacterRange::new('\u{30A0}', '\u{30FF}'), // katakana
            CharacterRange::new('\u{31F0}', '\u{31FF}'), // katakana phonetic extensions
            CharacterRange::new('\u{FF66}', '\u{FF9F}'), // halfwidth katakana
        ],
        approximate_bytes: 5_500_000,
    },
    FetchableSubset {
        family: "Noto Sans KR",
        writing_system: "Hangul",
        ranges: &[
            CharacterRange::new('\u{1100}', '\u{11FF}'), // hangul jamo
            CharacterRange::new('\u{3130}', '\u{318F}'), // hangul compatibility jamo
            CharacterRange::new('\u{A960}', '\u{A97F}'), // hangul jamo extended-A
            CharacterRange::new('\u{AC00}', '\u{D7AF}'), // hangul syllables
        ],
        approximate_bytes: 6_000_000,
    },
    FetchableSubset {
        family: "Noto Naskh Arabic",
        writing_system: "Arabic",
        ranges: &[
            CharacterRange::new('\u{0600}', '\u{06FF}'),
            CharacterRange::new('\u{0750}', '\u{077F}'),
            CharacterRange::new('\u{08A0}', '\u{08FF}'),
            CharacterRange::new('\u{FB50}', '\u{FDFF}'),
            CharacterRange::new('\u{FE70}', '\u{FEFF}'),
        ],
        approximate_bytes: 500_000,
    },
    FetchableSubset {
        family: "Noto Sans Hebrew",
        writing_system: "Hebrew",
        ranges: &[
            CharacterRange::new('\u{0590}', '\u{05FF}'),
            CharacterRange::new('\u{FB1D}', '\u{FB4F}'),
        ],
        approximate_bytes: 200_000,
    },
    FetchableSubset {
        family: "Noto Sans Devanagari",
        writing_system: "Devanagari",
        ranges: &[
            CharacterRange::new('\u{0900}', '\u{097F}'),
            CharacterRange::new('\u{A8E0}', '\u{A8FF}'),
        ],
        approximate_bytes: 300_000,
    },
    FetchableSubset {
        family: "Noto Sans Bengali",
        writing_system: "Bengali",
        ranges: &[CharacterRange::new('\u{0980}', '\u{09FF}')],
        approximate_bytes: 300_000,
    },
    FetchableSubset {
        family: "Noto Sans Tamil",
        writing_system: "Tamil",
        ranges: &[CharacterRange::new('\u{0B80}', '\u{0BFF}')],
        approximate_bytes: 250_000,
    },
    FetchableSubset {
        family: "Noto Sans Telugu",
        writing_system: "Telugu",
        ranges: &[CharacterRange::new('\u{0C00}', '\u{0C7F}')],
        approximate_bytes: 250_000,
    },
    FetchableSubset {
        family: "Noto Sans Thai",
        writing_system: "Thai",
        ranges: &[CharacterRange::new('\u{0E00}', '\u{0E7F}')],
        approximate_bytes: 200_000,
    },
    FetchableSubset {
        family: "Noto Color Emoji",
        writing_system: "Emoji",
        ranges: &[
            CharacterRange::new('\u{1F300}', '\u{1F5FF}'),
            CharacterRange::new('\u{1F600}', '\u{1F64F}'),
            CharacterRange::new('\u{1F680}', '\u{1F6FF}'),
            CharacterRange::new('\u{1F900}', '\u{1F9FF}'),
        ],
        approximate_bytes: 24_000_000,
    },
];

/// The subset that would cover `character`, or `None` when no fetchable subset does.
#[must_use]
pub fn fetchable_subset_for_character(character: char) -> Option<&'static FetchableSubset> {
    FETCHABLE_SUBSETS
        .iter()
        .find(|subset| subset.covers(character))
}

/// What tier 3 would fetch, and why. Produced by the resolver; **not** acted on, because this loop
/// ships no transport.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FetchPlan {
    /// The family the document asked for.
    pub requested_family: String,
    /// The subset that would cover the characters no local face could.
    pub subset: &'static FetchableSubset,
    /// The first character that drove the decision, so a message can name it.
    pub first_uncovered_character: char,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_pairs_the_plan_names_are_all_present_and_metric_compatible() {
        for (original, substitute) in [
            ("Calibri", "Carlito"),
            ("Cambria", "Caladea"),
            ("Arial", "Liberation Sans"),
            ("Times New Roman", "Liberation Serif"),
            ("Courier New", "Liberation Mono"),
        ] {
            let rule = substitution_for_family(original)
                .unwrap_or_else(|| panic!("no substitution rule for `{original}`"));
            assert!(
                rule.is_metric_compatible,
                "`{original}` is not marked metric-compatible"
            );
            assert_eq!(
                rule.substitutes.first().copied(),
                Some(substitute),
                "`{original}`'s first substitute should be `{substitute}`"
            );
        }
    }

    #[test]
    fn a_family_is_matched_however_it_is_capitalised() {
        assert!(substitution_for_family("CALIBRI").is_some());
        assert!(substitution_for_family("times new roman").is_some());
        assert!(substitution_for_family("Wingdings").is_none());
    }

    #[test]
    fn no_two_rules_claim_the_same_original() {
        for (position, rule) in SUBSTITUTION_TABLE.iter().enumerate() {
            for other in &SUBSTITUTION_TABLE[position + 1..] {
                assert!(
                    !rule.original.eq_ignore_ascii_case(other.original),
                    "`{}` has two rules, and only the first would ever be found",
                    rule.original
                );
            }
        }
    }

    #[test]
    fn no_rule_substitutes_a_family_for_itself() {
        for rule in SUBSTITUTION_TABLE {
            assert!(
                !rule
                    .substitutes
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(rule.original)),
                "`{}` lists itself as its own substitute, which would loop",
                rule.original
            );
        }
    }

    #[test]
    fn the_fetchable_subsets_do_not_overlap() {
        for (position, subset) in FETCHABLE_SUBSETS.iter().enumerate() {
            for other in &FETCHABLE_SUBSETS[position + 1..] {
                for range in subset.ranges {
                    for competing in other.ranges {
                        assert!(
                            range.last < competing.first || competing.last < range.first,
                            "`{}` and `{}` both claim {:?} — a lookup would be order-dependent",
                            subset.family,
                            other.family,
                            range
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_generic_keyword_is_recognised_and_a_real_family_is_not() {
        assert_eq!(
            GenericFamily::from_css_keyword("sans-serif"),
            Some(GenericFamily::SansSerif)
        );
        assert_eq!(
            GenericFamily::from_css_keyword(" UI-Monospace "),
            Some(GenericFamily::Monospace)
        );
        assert_eq!(GenericFamily::from_css_keyword("Nunito Sans"), None);
    }
}
