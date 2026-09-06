//! Which way the text runs — UAX #9, with the document's own declaration ahead of the content.
//!
//! # The base direction is declared, not guessed
//!
//! UAX #9 rule P2/P3 says a paragraph whose direction is not otherwise known takes the direction of
//! its **first strong character**. That is the *fallback*, not the rule, and a renderer that always
//! applies it gets Word wrong: `w:pPr/w:bidi` and `w:sectPr/w:bidi` state the paragraph's base
//! direction outright, and DrawingML's `a:pPr/@rtl` does the same for a shape's paragraph. A
//! right-to-left paragraph that happens to open with a Latin word is still right-to-left, and
//! inferring from content would left-align it and put its punctuation on the wrong side.
//!
//! So [`ParagraphDirection`] has three values, and only one of them looks at the text.
//!
//! # A run's own direction is an embedding, not an override
//!
//! `w:rPr/w:rtl` marks a run as right-to-left. In UAX #9 terms that is an **embedding** — the run's
//! contents are still resolved by the algorithm, at a level raised above the paragraph's — and not
//! an *override*, which would force every character to the run's direction regardless of what it is.
//! A number inside an `w:rtl` run still reads left to right, which is exactly what an embedding
//! gives and an override does not.
//!
//! [`BidiAnalysis::resolve_with_run_directions`] therefore expresses each declared run as the
//! explicit embedding controls UAX #9 §2.1 defines — `U+202A LEFT-TO-RIGHT EMBEDDING`,
//! `U+202B RIGHT-TO-LEFT EMBEDDING` and `U+202C POP DIRECTIONAL FORMATTING` — around a scratch copy
//! of the text, resolves that, and maps the levels back onto the original byte offsets. The
//! document's own text is never modified, and the controls never reach a shaper. This is the same
//! construction CSS `unicode-bidi: embed` is defined by.

use std::ops::Range;

use unicode_bidi::{BidiInfo, Level};

/// Which way a sequence of characters is written.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum TextDirection {
    /// Latin, Cyrillic, Greek, the Brahmic scripts, Han — the majority case.
    #[default]
    LeftToRight,
    /// Arabic, Hebrew, Syriac, Thaana, N'Ko and the rest of UAX #9's right-to-left set.
    RightToLeft,
}

impl TextDirection {
    /// Whether this is the right-to-left direction.
    #[must_use]
    pub fn is_right_to_left(self) -> bool {
        matches!(self, Self::RightToLeft)
    }

    /// The direction the other way round.
    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::LeftToRight => Self::RightToLeft,
            Self::RightToLeft => Self::LeftToRight,
        }
    }
}

/// What a paragraph declares about its own direction.
///
/// The first two come from the document — `w:bidi`, `a:pPr/@rtl` — and are obeyed. The third is
/// UAX #9's P2/P3 fallback, for a paragraph that declares nothing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum ParagraphDirection {
    /// The document says the paragraph is left-to-right.
    #[default]
    LeftToRight,
    /// The document says the paragraph is right-to-left — `w:bidi`, or `a:pPr/@rtl`.
    RightToLeft,
    /// The document says nothing, so UAX #9 rules P2 and P3 apply: the paragraph takes the
    /// direction of its first strong character, and is left-to-right if it has none.
    FromFirstStrongCharacter,
}

/// A UAX #9 embedding level: even is left-to-right, odd is right-to-left.
///
/// Levels run 0 to 125; the value is exactly the `L` of the specification, so a caller that knows
/// UAX #9 can reason about it directly, and one that does not has [`BidiLevel::direction`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct BidiLevel(pub u8);

impl BidiLevel {
    /// The level of a plain left-to-right paragraph.
    pub const LEFT_TO_RIGHT: Self = Self(0);
    /// The level of a plain right-to-left paragraph.
    pub const RIGHT_TO_LEFT: Self = Self(1);

    /// Which way text at this level runs — even levels left-to-right, odd right-to-left.
    #[must_use]
    pub fn direction(self) -> TextDirection {
        if self.0.is_multiple_of(2) {
            TextDirection::LeftToRight
        } else {
            TextDirection::RightToLeft
        }
    }
}

/// A stretch of the paragraph the document declared a direction for — one `w:rtl` run, or one
/// DrawingML run whose `a:rPr` says the same.
///
/// `range` is in bytes, must lie on character boundaries, and must not overlap another. Ranges that
/// break either rule are **skipped** rather than panicked on; see [`BidiAnalysis::resolve_with_run_directions`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeclaredRunDirection {
    /// The bytes the declaration covers.
    pub range: Range<usize>,
    /// The direction the document declared for them.
    pub direction: TextDirection,
}

/// One stretch of text at a single embedding level.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DirectionalRun {
    /// The bytes it covers.
    pub range: Range<usize>,
    /// The level UAX #9 resolved for them.
    pub level: BidiLevel,
}

impl DirectionalRun {
    /// Which way this run is written.
    #[must_use]
    pub fn direction(&self) -> TextDirection {
        self.level.direction()
    }
}

/// The resolved bidirectional structure of one paragraph.
///
/// Holds one level per **byte** of the paragraph — the same shape `unicode-bidi` produces, so that
/// slicing a level run and slicing the text use the same offsets and neither needs a translation
/// table. Every byte of a multi-byte character carries that character's level.
#[derive(Clone, Debug)]
pub struct BidiAnalysis {
    base: TextDirection,
    levels: Vec<BidiLevel>,
}

impl BidiAnalysis {
    /// Resolve `text` with the paragraph direction the document declared.
    #[must_use]
    pub fn resolve(text: &str, paragraph: ParagraphDirection) -> Self {
        let info = BidiInfo::new(text, paragraph_level(paragraph));
        Self::from_bidi_info(&info, text.len(), paragraph)
    }

    /// Resolve `text` with the paragraph direction the document declared, honouring each run whose
    /// own direction the document declared as a UAX #9 embedding.
    ///
    /// `runs` must be sorted by `range.start`, non-overlapping, and on character boundaries. A run
    /// that is not is **skipped**: these ranges are derived from a document, and refusing the whole
    /// paragraph because one run's offsets disagree would lose text a reader can see. Skipping one
    /// costs that run its declared direction and nothing else, and the rest of the paragraph
    /// resolves normally.
    #[must_use]
    pub fn resolve_with_run_directions(
        text: &str,
        paragraph: ParagraphDirection,
        runs: &[DeclaredRunDirection],
    ) -> Self {
        // `U+202A`/`U+202B` open an embedding and `U+202C` closes it; each is three bytes in UTF-8.
        const LEFT_TO_RIGHT_EMBEDDING: char = '\u{202A}';
        const RIGHT_TO_LEFT_EMBEDDING: char = '\u{202B}';
        const POP_DIRECTIONAL_FORMATTING: char = '\u{202C}';

        let usable: Vec<&DeclaredRunDirection> = usable_runs(text, runs).collect();
        if usable.is_empty() {
            return Self::resolve(text, paragraph);
        }

        // Segments map the scratch copy back onto the original: `(scratch_start, original_start,
        // length)`, one per stretch of real text between inserted controls. There is at most one
        // more segment than there are declared runs, so this is O(runs) rather than O(bytes).
        let mut scratch = String::with_capacity(text.len() + usable.len() * 8);
        let mut segments: Vec<(usize, usize, usize)> = Vec::with_capacity(usable.len() * 2 + 1);
        let mut cursor = 0_usize;

        let copy = |scratch: &mut String,
                    segments: &mut Vec<(usize, usize, usize)>,
                    from: usize,
                    to: usize| {
            if to > from {
                segments.push((scratch.len(), from, to - from));
                scratch.push_str(&text[from..to]);
            }
        };

        for run in &usable {
            copy(&mut scratch, &mut segments, cursor, run.range.start);
            scratch.push(match run.direction {
                TextDirection::LeftToRight => LEFT_TO_RIGHT_EMBEDDING,
                TextDirection::RightToLeft => RIGHT_TO_LEFT_EMBEDDING,
            });
            copy(&mut scratch, &mut segments, run.range.start, run.range.end);
            scratch.push(POP_DIRECTIONAL_FORMATTING);
            cursor = run.range.end;
        }
        copy(&mut scratch, &mut segments, cursor, text.len());

        let info = BidiInfo::new(&scratch, paragraph_level(paragraph));
        let scratch_levels = Self::from_bidi_info(&info, scratch.len(), paragraph);

        let mut levels = vec![BidiLevel(0); text.len()];
        for (scratch_start, original_start, length) in segments {
            for offset in 0..length {
                // Both indices are in range by construction: `segments` was built from the same
                // two strings whose lengths bound these vectors.
                if let (Some(source), Some(target)) = (
                    scratch_levels.levels.get(scratch_start + offset),
                    levels.get_mut(original_start + offset),
                ) {
                    *target = *source;
                }
            }
        }

        Self {
            base: scratch_levels.base,
            levels,
        }
    }

    fn from_bidi_info(info: &BidiInfo<'_>, length: usize, declared: ParagraphDirection) -> Self {
        // An empty string has no paragraph for `unicode-bidi` to report a level for. Falling back
        // to left-to-right there would lose the declaration on an empty right-to-left paragraph —
        // which still has a caret, still has a margin to start at, and is what an author sees after
        // pressing Return in a Hebrew document.
        let declared_base = match declared {
            ParagraphDirection::RightToLeft => TextDirection::RightToLeft,
            ParagraphDirection::LeftToRight | ParagraphDirection::FromFirstStrongCharacter => {
                TextDirection::LeftToRight
            }
        };
        let base = info.paragraphs.first().map_or(declared_base, |paragraph| {
            BidiLevel(paragraph.level.number()).direction()
        });
        let mut levels = Vec::with_capacity(length);
        levels.extend(info.levels.iter().map(|level| BidiLevel(level.number())));
        levels.resize(length, BidiLevel(0));
        Self { base, levels }
    }

    /// The paragraph's own direction, after P2/P3 if the document left it to the content.
    ///
    /// This is what decides which margin the first line starts at and where an odd-length last line
    /// hangs — not the direction of any particular run.
    #[must_use]
    pub fn base_direction(&self) -> TextDirection {
        self.base
    }

    /// How many bytes of text were resolved.
    #[must_use]
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// Whether the paragraph was empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// The embedding level at `byte`, or the base level for an offset past the end.
    #[must_use]
    pub fn level_at(&self, byte: usize) -> BidiLevel {
        self.levels.get(byte).copied().unwrap_or(match self.base {
            TextDirection::LeftToRight => BidiLevel::LEFT_TO_RIGHT,
            TextDirection::RightToLeft => BidiLevel::RIGHT_TO_LEFT,
        })
    }

    /// The level runs of `line`, in **logical** order — the order the bytes appear in the string.
    ///
    /// This is the order to shape in: each run is one direction, and a shaper is handed logical
    /// text and told which way it goes.
    #[must_use]
    pub fn logical_runs(&self, line: Range<usize>) -> Vec<DirectionalRun> {
        let line = self.clamp(line);
        let mut runs: Vec<DirectionalRun> = Vec::new();
        let mut start = line.start;
        while start < line.end {
            let level = self.level_at(start);
            let mut end = start + 1;
            while end < line.end && self.level_at(end) == level {
                end += 1;
            }
            runs.push(DirectionalRun {
                range: start..end,
                level,
            });
            start = end;
        }
        runs
    }

    /// The level runs of `line`, in **visual** order — left to right on the page, whatever the
    /// paragraph's direction.
    ///
    /// This is UAX #9 rule L2: reverse each maximal stretch at or above every level down to the
    /// lowest odd level. The runs' `range`s stay in logical coordinates, so a caller can still slice
    /// the original text with them; it is their *order* in the returned vector that is visual.
    #[must_use]
    pub fn visual_runs(&self, line: Range<usize>) -> Vec<DirectionalRun> {
        let mut runs = self.logical_runs(line);
        if runs.is_empty() {
            return runs;
        }

        let highest = runs.iter().map(|run| run.level.0).max().unwrap_or(0);
        let lowest_odd = runs
            .iter()
            .map(|run| run.level.0)
            .filter(|level| level % 2 == 1)
            .min();
        let Some(lowest_odd) = lowest_odd else {
            // Every run is even, so nothing is reversed and logical order is visual order.
            return runs;
        };

        let mut level = highest;
        while level >= lowest_odd {
            let mut index = 0;
            while index < runs.len() {
                if runs[index].level.0 >= level {
                    let start = index;
                    while index < runs.len() && runs[index].level.0 >= level {
                        index += 1;
                    }
                    runs[start..index].reverse();
                } else {
                    index += 1;
                }
            }
            // `lowest_odd` is at least 1, so this cannot wrap.
            level -= 1;
        }
        runs
    }

    fn clamp(&self, line: Range<usize>) -> Range<usize> {
        let start = line.start.min(self.levels.len());
        let end = line.end.clamp(start, self.levels.len());
        start..end
    }
}

fn paragraph_level(paragraph: ParagraphDirection) -> Option<Level> {
    match paragraph {
        ParagraphDirection::LeftToRight => Some(Level::ltr()),
        ParagraphDirection::RightToLeft => Some(Level::rtl()),
        ParagraphDirection::FromFirstStrongCharacter => None,
    }
}

/// The declared runs that can be honoured: in order, non-overlapping, non-empty, and on character
/// boundaries.
fn usable_runs<'a>(
    text: &'a str,
    runs: &'a [DeclaredRunDirection],
) -> impl Iterator<Item = &'a DeclaredRunDirection> {
    let mut previous_end = 0_usize;
    runs.iter().filter(move |run| {
        let usable = run.range.start >= previous_end
            && run.range.end > run.range.start
            && run.range.end <= text.len()
            && text.is_char_boundary(run.range.start)
            && text.is_char_boundary(run.range.end);
        if usable {
            previous_end = run.range.end;
        }
        usable
    })
}
