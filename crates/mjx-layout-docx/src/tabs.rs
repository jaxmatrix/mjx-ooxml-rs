//! Tab stops: the five kinds that place text, the sixth that draws a rule, the leader characters,
//! and the implicit grid past the last stated stop.
//!
//! # The resolution order is already done
//!
//! `w:tabs` is a `CT_PPrBase` member like any other, so `mjx-docx`'s ladder has already merged the
//! paragraph's own stops with its style chain's and `w:docDefaults`'. What is left is
//! WordprocessingML's own rule that the ladder cannot express: **`w:val="clear"` removes the stop at
//! that position** (§17.3.1.37) rather than adding one. A `clear` that survived into the stop list
//! would be a tab stop at a position the document deliberately emptied.
//!
//! # Where `w:pos` is measured from
//!
//! **GUESS:** from the leading edge of the column, before the paragraph's own indents. §17.3.1.37
//! says "relative to the current text margin" and does not define whether an indent moves the
//! margin. The two readings differ visibly for any indented paragraph with a tab in it, and this one
//! is the reading under which a tab stop at 4,320 twips lines up down a page whose paragraphs are
//! indented differently — which is what a table of contents is.

use mjx_docx::EffectiveTabStop;
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::{TabStopLeader, TabStopType};

use crate::measure::signed_twips_measure;

/// What a tab does when the pen reaches it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabKind {
    /// The text after the tab **begins** at the stop. `start`, `left`, and the implicit grid.
    Leading,
    /// The text after the tab is **centred** on the stop.
    Centred,
    /// The text after the tab **ends** at the stop. `end`, `right`.
    Trailing,
    /// The first decimal separator after the tab sits **on** the stop.
    Decimal,
    /// A vertical rule at the stop. The pen does **not** move: a `bar` tab is a rule drawn at a
    /// position, not a tab that advances anything, which is the one member of `ST_TabJc` that is
    /// not a tab at all.
    Bar,
}

impl TabKind {
    /// What `ST_TabJc` means. `clear` is not here — see [`TabRuler::new`], which removes rather than
    /// maps it.
    #[must_use]
    fn of(value: TabStopType) -> Option<Self> {
        Some(match value {
            TabStopType::Clear => return None,
            TabStopType::Start | TabStopType::Left => Self::Leading,
            TabStopType::Center => Self::Centred,
            TabStopType::End | TabStopType::Right => Self::Trailing,
            TabStopType::Decimal => Self::Decimal,
            TabStopType::Bar => Self::Bar,
            // GUESS: `num` aligns text to the list's own tab position, which is numbering and so
            // MJXOFF-178 (R22). Until a list has a number, its tab has no position, and a leading
            // tab is what the paragraph looks like without one.
            TabStopType::List => Self::Leading,
        })
    }
}

/// One resolved stop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TabStop {
    /// Where it is, from the column's leading edge.
    pub position: Emu,
    /// What it does.
    pub kind: TabKind,
    /// What fills the gap in front of it.
    pub leader: TabStopLeader,
}

/// A paragraph's tab stops, and the grid past the last of them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TabRuler {
    stops: Vec<TabStop>,
    interval: Emu,
}

impl TabRuler {
    /// `w:defaultTabStop`'s fallback when the document states none, or states one of zero.
    ///
    /// Half an inch, which is what `mjx_docx::DocumentLayoutSettings` already reports; the second
    /// copy here is the guard for the *zero* case, which that reader cannot make — a document may
    /// legally write `w:defaultTabStop="0"`, and a grid of zero-width steps is an infinite loop.
    pub const FALLBACK_INTERVAL: Emu = Emu::from_twips(720);

    /// The ruler a paragraph with these stops and this default interval has.
    ///
    /// `w:val="clear"` removes any stop already at its position and adds none — see the module's own
    /// documentation. A stop whose `w:pos` will not parse is dropped rather than placed at zero,
    /// because a stop at zero is a stop a reader can see and a dropped one falls through to the
    /// grid, which is what the paragraph would have had anyway.
    #[must_use]
    pub fn new(stated: &[EffectiveTabStop], interval: Emu) -> Self {
        let mut stops: Vec<TabStop> = Vec::with_capacity(stated.len());
        for stop in stated {
            let Some(position) = signed_twips_measure(&stop.position) else {
                continue;
            };
            stops.retain(|existing| existing.position != position);
            let Some(kind) = TabKind::of(stop.alignment) else {
                continue;
            };
            stops.push(TabStop {
                position,
                kind,
                leader: stop.leader.unwrap_or(TabStopLeader::None),
            });
        }
        stops.sort_by_key(|stop| stop.position.emu());
        Self {
            stops,
            interval: if interval > Emu::ZERO {
                interval
            } else {
                Self::FALLBACK_INTERVAL
            },
        }
    }

    /// The stops it states, in order.
    #[must_use]
    pub fn stated(&self) -> &[TabStop] {
        &self.stops
    }

    /// The grid interval past the last stated stop.
    #[must_use]
    pub fn interval(&self) -> Emu {
        self.interval
    }

    /// Every `bar` stop, which draws a rule whatever the text does.
    ///
    /// A bar tab is the one stop that has an effect on a line that never reaches it: the rule is
    /// drawn down the paragraph at that position regardless of where the text ends.
    pub fn bars(&self) -> impl Iterator<Item = Emu> + '_ {
        self.stops
            .iter()
            .filter(|stop| stop.kind == TabKind::Bar)
            .map(|stop| stop.position)
    }

    /// The stop a tab at `position` lands on.
    ///
    /// The first stated stop strictly past `position` wins, skipping `bar` stops, which do not
    /// advance a pen. Past the last stated stop the implicit grid takes over, at the next multiple
    /// of [`TabRuler::interval`] — and a tab that would not move at all still moves one whole step,
    /// because a tab that advanced nothing would draw a paragraph of tabs on top of itself.
    #[must_use]
    pub fn next_after(&self, position: Emu) -> TabStop {
        if let Some(stop) = self
            .stops
            .iter()
            .find(|stop| stop.kind != TabKind::Bar && stop.position > position)
        {
            return *stop;
        }
        let step = self.interval.emu().max(1);
        let past = self.stops.last().map_or(0, |stop| stop.position.emu());
        let from = position.emu().max(past);
        let multiples = from.div_euclid(step).saturating_add(1);
        TabStop {
            position: Emu::from_emu(multiples.saturating_mul(step)),
            kind: TabKind::Leading,
            leader: TabStopLeader::None,
        }
    }
}

/// The character a leader is drawn with, or `None` for a gap.
///
/// **GUESS: the five characters.** ECMA-376 names the leader styles and not the glyphs; these are
/// what Word draws, read off its own output, and `heavy` is an underscore in a bolder weight rather
/// than a different character — which this cannot express, so it uses the same one and is visibly
/// lighter than Word's.
#[must_use]
pub fn leader_character(leader: TabStopLeader) -> Option<char> {
    match leader {
        TabStopLeader::None => None,
        TabStopLeader::Dot => Some('.'),
        TabStopLeader::Hyphen => Some('-'),
        TabStopLeader::Underscore | TabStopLeader::Heavy => Some('_'),
        TabStopLeader::MiddleDot => Some('\u{00B7}'),
    }
}

/// Which character a decimal tab aligns on.
///
/// **GUESS: the full stop, always.** The separator is a locale's, not a document's: `w:listSeparator`
/// in `word/settings.xml` states the *list* separator and no element states the decimal one, so
/// there is nothing in a `.docx` to read. A German document aligning on a comma will align on
/// nothing here and fall back to a leading tab, which is visible and is the honest failure.
pub const DECIMAL_SEPARATOR: char = '.';
