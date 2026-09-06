//! Where a line may end — UAX #14, and the East Asian rules Office applies on top of it.
//!
//! # UAX #14 is the floor, not the answer
//!
//! `unicode-linebreak` implements UAX #14 and gets the great majority of the world's text right.
//! Office does not stop there. Japanese typesetting has *kinsoku shori* (禁則処理, "rules for
//! prohibition"), standardised in **JIS X 4051** and exposed in Word as `w:kinsoku`,
//! `w:overflowPunct` and `w:topLinePunct`, and its two central rules are:
//!
//! * **行頭禁則** — characters that may not begin a line: the closing brackets and quotes, the
//!   ideographic comma and full stop, the small kana, the prolonged sound mark, the iteration marks,
//!   and the postfix signs `%`, `‰`, `℃`, `°`, `′`, `″`, `¢`.
//! * **行末禁則** — characters that may not end a line: the opening brackets and quotes, and the
//!   prefix currency signs.
//!
//! UAX #14 already covers most of that, because its `CL`, `CP`, `EX`, `IS` and `NS` classes were
//! written with kinsoku in mind. **It does not cover all of it**, and the gap is exactly where
//! rule LB18 — *break after a space, unconditionally* — meets a postfix sign or a small kana. In
//! `10 %`, UAX #14 offers a break after the space that would leave `%` at the head of the next line;
//! Word does not, and moves the number down with it. [`LineBreaker`] is where that difference lives,
//! and [`KinsokuRules`] is the set it is decided by.
//!
//! # Hanging punctuation
//!
//! `w:overflowPunct` lets a trailing comma or full stop extend past the measure rather than pushing
//! the character before it onto the next line. That is not a break *opportunity* at all — it is a
//! measurement rule — so it appears here as [`LineBreak::hanging`]: the tail of the line whose width
//! a caller does not count against the measure. Deciding what to do with that tail is the box
//! model's; deciding which characters may form it is this crate's.
//!
//! # What `LineBreakOptions::default()` is, and why it is plain UAX #14 (MJXOFF-160)
//!
//! Until MJXOFF-160 the `Default` turned **both** East Asian rules on, and
//! [`KinsokuRules::japanese_standard`]'s hangable set contains the ASCII comma and full stop as well
//! as the ideographic ones. The consequence was that a caller who said nothing got the Japanese
//! answer for an English paragraph: a line-final `.` did not count against the measure, so every
//! sentence-ending line was allowed to run a fraction of an em long.
//!
//! Two questions were tangled together there, and they have different kinds of answer.
//!
//! * **Should ASCII `,` and `.` be in the Japanese hangable set at all?** In a Japanese paragraph
//!   with `w:overflowPunct` on, ASCII punctuation is used and Word does hang it — but *exactly*
//!   which characters, and whether a Latin paragraph in the same document is treated the same way,
//!   is a measurement against Word rather than something the specification states. That question is
//!   **not answered here**; it is MJXOFF-155 §9's Windows/PowerPoint reference pass, and the set is
//!   left exactly as it was.
//! * **Should a caller who said nothing get those rules?** That one needs no measurement. Both flags
//!   are named for *document settings* — `w:kinsoku` and `w:overflowPunct` — and a document that
//!   carries neither has not asked for either. A `Default` that silently enables two settings the
//!   document did not write is not a default, it is a hidden policy, and it is one that changes
//!   where every English line breaks.
//!
//! So `Default` is now [`LineBreakOptions::unicode_only`], and the Japanese answer has a name of its
//! own: [`LineBreakOptions::japanese_typesetting`]. A box model reading a real document sets both
//! flags from what the document says and never relies on either.
//!
//! # This is line breaking, not layout
//!
//! [`LineBreaker::next_line`] chooses **where a line ends**, given a measure and a function that
//! measures a slice. It does not decide where the line *goes*, how tall it is, what it is aligned
//! to, or what floats around it — all of which are the box model's, from R05 onward.

use std::ops::Range;

use unicode_linebreak::{linebreaks, BreakOpportunity as UnicodeBreak};

/// Whether a break at this point is required or merely permitted.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BreakKind {
    /// A hard break: a paragraph separator, a line feed, `U+2028`. The line ends here whatever the
    /// measure says, and no rule may take it away — kinsoku included.
    Mandatory,
    /// A break the line may take if the measure calls for one.
    Allowed,
}

/// A point at which a line may end.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BreakOpportunity {
    /// The byte offset at which the **next** line would start. The line that ends here covers
    /// everything before it.
    pub at: usize,
    /// Whether the break is required.
    pub kind: BreakKind,
}

/// The kinsoku sets: which characters may not open a line, which may not close one, and which may
/// hang past the measure.
///
/// The default is JIS X 4051's standard (標準) set, which is also Word's default for Japanese.
/// A document that carries `w:noLineBreaksAfter` / `w:noLineBreaksBefore` declares its own, which is
/// what [`KinsokuRules::from_character_sets`] is for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KinsokuRules {
    prohibited_at_line_start: Box<str>,
    prohibited_at_line_end: Box<str>,
    hangable: Box<str>,
}

/// 行頭禁則文字 — may not begin a line. The JIS X 4051 standard set, as Word ships it.
const JAPANESE_STANDARD_PROHIBITED_AT_LINE_START: &str = concat!(
    // The ASCII and Latin-1 postfix and closing signs.
    "!%),.:;?]}¢°'\"",
    // The typographic postfix signs and closing quotes.
    "‰′″℃’”",
    // The CJK punctuation, brackets and iteration marks.
    "、。々〉》」』】〕〟〉",
    // Small hiragana.
    "ぁぃぅぇぉっゃゅょゎゕゖ",
    // Small katakana, the prolonged sound mark, the middle dot, and the sound marks.
    "ァィゥェォッャュョヮヵヶー・゛゜ゝゞヽヾ",
    // The fullwidth forms of the same.
    "！％），．：；？］｝｡｢｣､･ﾞﾟ",
);

/// 行末禁則文字 — may not end a line.
const JAPANESE_STANDARD_PROHIBITED_AT_LINE_END: &str =
    concat!("$([{£¥‘“", "〈《「『【〔〝", "＄（［｛￡￥",);

/// The characters `w:overflowPunct` lets extend past the measure: the ideographic and fullwidth
/// comma and full stop, their halfwidth forms, **and the ASCII comma and full stop**, which a
/// Japanese paragraph also uses.
///
/// The last two are why this set is reachable only through
/// [`LineBreakOptions::japanese_typesetting`] and never through `Default`: they are the characters
/// an English sentence ends with, and hanging them in a Latin paragraph nobody asked about is a
/// silent policy rather than a default. Whether Word hangs an ASCII full stop in a *Japanese*
/// paragraph, and whether it treats a Latin paragraph in the same document differently, is a
/// measurement against Word and not a reading of the specification — MJXOFF-155 §9's reference pass
/// — so the set itself is left exactly as JIS X 4051 and Word's own behaviour were transcribed.
const JAPANESE_HANGING_PUNCTUATION: &str = "、。，．｡､,.";

impl KinsokuRules {
    /// The JIS X 4051 standard set — Word's default for Japanese.
    #[must_use]
    pub fn japanese_standard() -> Self {
        Self {
            prohibited_at_line_start: Box::from(JAPANESE_STANDARD_PROHIBITED_AT_LINE_START),
            prohibited_at_line_end: Box::from(JAPANESE_STANDARD_PROHIBITED_AT_LINE_END),
            hangable: Box::from(JAPANESE_HANGING_PUNCTUATION),
        }
    }

    /// A set the document declared — `w:noLineBreaksBefore` and `w:noLineBreaksAfter`, whose values
    /// are exactly these two strings of characters.
    ///
    /// `hangable` is the set `w:overflowPunct` applies to; pass
    /// [`KinsokuRules::japanese_standard`]'s if the document does not narrow it.
    #[must_use]
    pub fn from_character_sets(
        prohibited_at_line_start: &str,
        prohibited_at_line_end: &str,
        hangable: &str,
    ) -> Self {
        Self {
            prohibited_at_line_start: Box::from(prohibited_at_line_start),
            prohibited_at_line_end: Box::from(prohibited_at_line_end),
            hangable: Box::from(hangable),
        }
    }

    /// Whether a line may not begin with `character`.
    #[must_use]
    pub fn prohibits_at_line_start(&self, character: char) -> bool {
        self.prohibited_at_line_start.contains(character)
    }

    /// Whether a line may not end with `character`.
    #[must_use]
    pub fn prohibits_at_line_end(&self, character: char) -> bool {
        self.prohibited_at_line_end.contains(character)
    }

    /// Whether `character` may hang past the measure when `w:overflowPunct` is on.
    #[must_use]
    pub fn hangs(&self, character: char) -> bool {
        self.hangable.contains(character)
    }

    /// The characters a line may not begin with.
    #[must_use]
    pub fn characters_prohibited_at_line_start(&self) -> &str {
        &self.prohibited_at_line_start
    }

    /// The characters a line may not end with.
    #[must_use]
    pub fn characters_prohibited_at_line_end(&self) -> &str {
        &self.prohibited_at_line_end
    }
}

impl Default for KinsokuRules {
    fn default() -> Self {
        Self::japanese_standard()
    }
}

/// What the document says about line breaking.
///
/// Both flags name a document setting, so both are **off** by default and a caller states what the
/// document said. See the module documentation for why that is the default rather than Word's
/// Japanese answer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LineBreakOptions {
    /// `w:kinsoku` — whether the East Asian prohibitions apply on top of UAX #14. Word turns this on
    /// for a Japanese document; a document that does not write the setting has not asked for it.
    pub east_asian_rules: bool,
    /// `w:overflowPunct` — whether trailing punctuation may extend past the measure.
    pub hanging_punctuation: bool,
    /// Which characters the two above are decided by. Consulted only when one of them is on, so a
    /// paragraph with both off is unaffected by what is in it.
    pub kinsoku: KinsokuRules,
}

impl Default for LineBreakOptions {
    /// Plain UAX #14 — [`LineBreakOptions::unicode_only`].
    fn default() -> Self {
        Self::unicode_only()
    }
}

impl LineBreakOptions {
    /// Plain UAX #14, with nothing layered on it — what a document that writes neither `w:kinsoku`
    /// nor `w:overflowPunct` asked for, and what `w:kinsoku` switched off means.
    #[must_use]
    pub fn unicode_only() -> Self {
        Self {
            east_asian_rules: false,
            hanging_punctuation: false,
            kinsoku: KinsokuRules::default(),
        }
    }

    /// Japanese typesetting: JIS X 4051's *kinsoku* prohibitions **and** hanging punctuation, over
    /// [`KinsokuRules::japanese_standard`] — what Word does for a document that writes `w:kinsoku`
    /// and `w:overflowPunct`.
    ///
    /// Named rather than defaulted, because these are two document settings and a caller that has
    /// not read a document has not seen them. See the module documentation.
    #[must_use]
    pub fn japanese_typesetting() -> Self {
        Self {
            east_asian_rules: true,
            hanging_punctuation: true,
            kinsoku: KinsokuRules::japanese_standard(),
        }
    }
}

/// Where a line ended, and why.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LineBreak {
    /// The byte offset the line ends at, which is where the next line starts.
    pub end: usize,
    /// Why it ended there.
    pub kind: LineBreakKind,
    /// The tail of the line whose width does not count against the measure, when
    /// `w:overflowPunct` is on. Empty otherwise, and empty whenever the line ends at a mandatory
    /// break.
    pub hanging: Range<usize>,
}

/// Why a line ended where it did.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LineBreakKind {
    /// A hard break in the text.
    Mandatory,
    /// The last opportunity that fitted the measure.
    Fitted,
    /// No opportunity fitted, so the first one past the measure was taken rather than producing a
    /// line of nothing. A word longer than the measure has to overflow *somewhere*, and Word
    /// overflows it rather than dropping it.
    Overflowing,
    /// The text ran out before the measure did.
    EndOfText,
}

/// The break opportunities in one paragraph, and the choice of where a line ends.
#[derive(Clone, Debug)]
pub struct LineBreaker<'a> {
    text: &'a str,
    options: LineBreakOptions,
    opportunities: Vec<BreakOpportunity>,
}

impl<'a> LineBreaker<'a> {
    /// Find the break opportunities in `text` under `options`.
    #[must_use]
    pub fn new(text: &'a str, options: LineBreakOptions) -> Self {
        let opportunities = break_opportunities(text, &options);
        Self {
            text,
            options,
            opportunities,
        }
    }

    /// The text being broken.
    #[must_use]
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// The options in force.
    #[must_use]
    pub fn options(&self) -> &LineBreakOptions {
        &self.options
    }

    /// Every opportunity, in order. The last one is always at `text.len()`.
    #[must_use]
    pub fn opportunities(&self) -> &[BreakOpportunity] {
        &self.opportunities
    }

    /// Where the line starting at `from` ends, if it must fit `measure`.
    ///
    /// `width_of` is asked for the width of `text[range]` in whatever unit `measure` is in — points,
    /// ems, font units, it does not matter as long as the two agree. It is a closure rather than a
    /// number per character because the width of a slice is a *shaped* width, and only the caller
    /// has the face and the cache to compute one.
    ///
    /// A **hard** break in the text — a line feed, `U+2028`, a paragraph separator — always wins,
    /// whatever the measure says. The end of the text is not one of those, even though UAX #14
    /// reports it as mandatory: it is measured like any other candidate, so a paragraph's last line
    /// obeys the same measure as every line before it. When nothing fits, the first opportunity past
    /// the measure is taken and the result says [`LineBreakKind::Overflowing`], because a line with
    /// no text on it would never terminate.
    pub fn next_line(
        &self,
        from: usize,
        measure: f64,
        width_of: &mut dyn FnMut(Range<usize>) -> f64,
    ) -> LineBreak {
        if from >= self.text.len() {
            return LineBreak {
                end: self.text.len(),
                kind: LineBreakKind::EndOfText,
                hanging: self.text.len()..self.text.len(),
            };
        }

        let mut best: Option<BreakOpportunity> = None;
        for opportunity in self
            .opportunities
            .iter()
            .copied()
            .filter(|opportunity| opportunity.at > from)
        {
            // UAX #14 reports the end of the text as a mandatory break, because there is nothing
            // after it to break before. That is not a *hard* break in the text, and it must **not**
            // be taken unconditionally: doing so was MJXOFF-158's one defect, found by MJXOFF-160.
            // A paragraph whose last word does not fit — `"a bbbbbbbbbbbbbbbbbbbb"` against a
            // measure of five, with an opportunity at byte 2 that fits — returned the whole
            // twenty-two characters as one line, because the loop reached the end sentinel and
            // returned before ever measuring it. Every paragraph's *last* line was exempt from its
            // own measure. The sentinel is therefore an ordinary candidate, and the `best` and
            // `None` arms below already give it the right [`LineBreakKind::EndOfText`].
            //
            // A real hard break — a line feed, `U+2028`, a paragraph separator — is still taken
            // whatever the measure says, because no rule may take one away.
            if opportunity.kind == BreakKind::Mandatory && opportunity.at < self.text.len() {
                return LineBreak {
                    end: opportunity.at,
                    kind: LineBreakKind::Mandatory,
                    hanging: self.hanging_tail(from..opportunity.at),
                };
            }
            let hanging = self.hanging_tail(from..opportunity.at);
            let measured = width_of(from..hanging.start);
            if measured <= measure {
                best = Some(opportunity);
            } else if best.is_some() {
                break;
            } else {
                // Nothing has fitted yet, so this is the shortest line the text allows. Taking it
                // is what stops a word longer than the measure from producing an empty line for
                // ever.
                return LineBreak {
                    end: opportunity.at,
                    kind: LineBreakKind::Overflowing,
                    hanging,
                };
            }
        }

        match best {
            Some(opportunity) => {
                let hanging = self.hanging_tail(from..opportunity.at);
                LineBreak {
                    end: opportunity.at,
                    kind: if opportunity.at == self.text.len() {
                        LineBreakKind::EndOfText
                    } else {
                        LineBreakKind::Fitted
                    },
                    hanging,
                }
            }
            None => LineBreak {
                end: self.text.len(),
                kind: LineBreakKind::EndOfText,
                hanging: self.hanging_tail(from..self.text.len()),
            },
        }
    }

    /// The trailing stretch of `line` that may hang past the measure.
    ///
    /// Empty unless `w:overflowPunct` is on. Never the whole line: a line that is nothing but
    /// hanging punctuation would measure zero and never advance.
    #[must_use]
    pub fn hanging_tail(&self, line: Range<usize>) -> Range<usize> {
        if !self.options.hanging_punctuation
            || line.start >= line.end
            || line.end > self.text.len()
            || !self.text.is_char_boundary(line.start)
            || !self.text.is_char_boundary(line.end)
        {
            return line.end..line.end;
        }
        let mut start = line.end;
        for (offset, character) in self.text[line.clone()].char_indices().rev() {
            if !self.options.kinsoku.hangs(character) {
                break;
            }
            start = line.start + offset;
        }
        if start <= line.start {
            return line.end..line.end;
        }
        start..line.end
    }
}

/// The break opportunities in `text` under `options`.
///
/// With [`LineBreakOptions::east_asian_rules`] off this is exactly UAX #14. With it on, an
/// opportunity is removed when the character that would begin the next line may not, or the
/// character that would end this one may not — **except** a mandatory break, which no rule may take
/// away: a line feed in the text ends the line whatever follows it.
#[must_use]
pub fn break_opportunities(text: &str, options: &LineBreakOptions) -> Vec<BreakOpportunity> {
    let mut opportunities: Vec<BreakOpportunity> = linebreaks(text)
        .map(|(at, kind)| BreakOpportunity {
            at,
            kind: match kind {
                UnicodeBreak::Mandatory => BreakKind::Mandatory,
                UnicodeBreak::Allowed => BreakKind::Allowed,
            },
        })
        .collect();

    if options.east_asian_rules {
        opportunities.retain(|opportunity| {
            opportunity.kind == BreakKind::Mandatory
                || permitted_by_kinsoku(text, *opportunity, &options.kinsoku)
        });
    }
    opportunities
}

fn permitted_by_kinsoku(text: &str, opportunity: BreakOpportunity, kinsoku: &KinsokuRules) -> bool {
    if opportunity.at >= text.len() {
        // The end of the text is not a line start, so nothing can be prohibited at it.
        return true;
    }
    if let Some(first_of_next_line) = text[opportunity.at..].chars().next() {
        if kinsoku.prohibits_at_line_start(first_of_next_line) {
            return false;
        }
    }
    if let Some(last_of_this_line) = text[..opportunity.at].chars().next_back() {
        if kinsoku.prohibits_at_line_end(last_of_this_line) {
            return false;
        }
    }
    true
}
