//! Footnotes: the second flow, the space it takes from the first, and **the fixed point that
//! resolves the circularity between them — with its termination argument written out**.
//!
//! # The circularity, stated plainly
//!
//! A footnote's height reduces the body's. The body's content decides which footnotes are on the
//! page. So:
//!
//! ```text
//! more notes  ->  less body space  ->  fewer lines placed  ->  fewer notes
//! fewer notes ->  more body space  ->  more lines placed    ->  more notes
//! ```
//!
//! Every naive loop over that oscillates: reserve nothing, discover a note, reserve for it, lose the
//! line that referenced it, reserve nothing, discover the note again. An engine that laid the notes
//! out *after* pagination was settled would never oscillate and would be **wrong**, and wrong in the
//! way that is hardest to see — the page looks right, the note is on the right page, and the
//! paragraph three pages later is on the wrong one. `tests/a_footnote_moves_the_body.rs` asserts on
//! exactly that paragraph.
//!
//! # The resolution: a monotone reservation, and why it converges in two assemblies
//!
//! Let *R* be the height reserved for notes, and *N(R)* the height the notes actually referenced by
//! a body laid out under *R* would need. Two facts:
//!
//! 1. **The body content placed is non-increasing in *R*.** More reserved is less body space, and
//!    every pagination rule in [`crate::paginate`] places a *prefix* of what it would have placed
//!    with more room. (The one rule that could place *more* — "a column that is still empty takes a
//!    line anyway" — fires only on the page's first line, which is fixed by where the page starts
//!    and not by *R*.)
//! 2. **Therefore *N* is non-increasing in *R*.** Fewer lines reference a subset of the notes, and a
//!    subset is not taller.
//!
//! The iteration is *R*₀ = the height already committed to a note carried in from the previous page,
//! and *R*ₖ₊₁ = *N*(*R*ₖ), taken only while *N*(*R*ₖ) > *R*ₖ. It **terminates after at most two body
//! assemblies**, and not merely eventually:
//!
//! * if *N*(*R*₀) ≤ *R*₀ the first assembly stands;
//! * otherwise *R*₁ = *N*(*R*₀) > *R*₀, and because *N* is non-increasing, *N*(*R*₁) ≤ *N*(*R*₀) =
//!   *R*₁ — so the second assembly satisfies the stopping condition unconditionally.
//!
//! **The body is not then re-expanded to take up the slack**, and that is the decision that makes
//! the loop finite rather than merely convergent: expanding it could only re-admit the line that was
//! just excluded, which re-adds the note, which re-shrinks it. A few EMU of white space above the
//! footnote rule is what a stable answer costs, and Word leaves the same gap.
//!
//! # A footnote taller than the page
//!
//! That is a real document and not a hypothetical, and it is where an unbounded reservation stops
//! terminating: *N* would exceed the whole body area, the body would place nothing, the page's next
//! position would equal its own, and the page after it would be identical for ever.
//!
//! So the reservation is **capped**: the note area may never take the space the body's first line
//! needs. The cap is a constant for the page — it depends on where the page starts, not on *R* — so
//! the two-assembly argument above is unaffected, and the note that does not fit is **split**: as
//! many of its lines as the area holds are placed, and the rest is carried to the next page under a
//! continuation separator. Progress is then guaranteed from the other side too, because
//! [`StreamLayout::lines_that_fit`](crate::stream::StreamLayout::lines_that_fit) never places zero
//! lines while there are lines left: every page consumes at least one line of the carried note *and*
//! at least one line of the body, so both flows strictly advance. `tests/termination.rs` runs a note
//! forty times the height of its page.

use std::ops::Range;

use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::{FootnoteEndnoteType, FootnotePosition};

use crate::stream::StreamLayout;

/// One note a page's body refers to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DemandedNote {
    /// Which entry of `word/footnotes.xml` it is.
    pub note: usize,
    /// The number its mark carries, from the section's own numbering rules.
    pub number: i64,
}

/// The tail of a note that did not fit, carried to the next page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NoteCarry {
    /// Which entry.
    pub note: usize,
    /// Its number, so the continuation is not renumbered.
    pub number: i64,
    /// The first of its lines that has not been placed.
    pub line: u32,
}

/// One note, or part of one, placed in a page's note area.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacedNote {
    /// Which entry.
    pub note: usize,
    /// Its number.
    pub number: i64,
    /// Which of its lines are here.
    pub lines: Range<usize>,
    /// Where the first of them sits, from the top of the note area.
    pub top: Emu,
    /// Whether it began on an earlier page.
    pub continued: bool,
    /// Whether it carries on to the next.
    pub continues: bool,
}

/// A page's note area.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NoteArea {
    /// Which of Word's two rules is drawn above the notes, or `None` when the part defines neither.
    ///
    /// [`FootnoteEndnoteType::ContinuationSeparator`] on a page whose area opens with a note carried
    /// from the page before, [`FootnoteEndnoteType::Separator`] otherwise — which is exactly what
    /// the two entries are for, and the difference a reader sees is the length of the rule.
    pub separator: Option<FootnoteEndnoteType>,
    /// How tall the separator is, including the gap above the notes.
    pub separator_height: Emu,
    /// The notes, in order.
    pub notes: Vec<PlacedNote>,
    /// How tall the whole area is.
    pub height: Emu,
    /// What did not fit.
    pub carry: Option<NoteCarry>,
    /// Where a `continuationNotice` was placed, from the top of the area, or `None` when the page
    /// carries nothing forward or the part defines no such entry.
    ///
    /// The notice is **furniture placed in the room that is left**, and it never enlarges the
    /// demand: including its height in *N*(*R*) would mean reserving space on every page for a
    /// notice most of them do not need, and reserving it *conditionally* would make *N* depend on
    /// whether a carry happens — which is decided by *N*. Leaving it out of the fixed point keeps
    /// the two-assembly bound exactly as `crate::notes`'s own argument states it.
    pub continuation_notice: Option<Emu>,
    /// Where on the page the area sits.
    pub position: FootnotePosition,
}

impl NoteArea {
    /// A page with no notes on it at all.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            separator: None,
            separator_height: Emu::ZERO,
            notes: Vec::new(),
            height: Emu::ZERO,
            carry: None,
            continuation_notice: None,
            position: FootnotePosition::PageBottom,
        }
    }

    /// Whether it takes no space.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
}

/// What [`build`] needs that is not a measurement: the laid-out notes and Word's two rules.
///
/// The layouts arrive as a **map rather than a closure**, and that is deliberate: a closure would
/// have to be built where the notes are laid out and read where the area is fitted, and the two are
/// on either side of a `&mut self` borrow of the box model — the shape that tempts an implementation
/// into leaking a boxed closure to make the lifetimes work. A borrow of the map crosses that seam
/// with nothing allocated.
#[derive(Debug)]
pub struct NoteContent<'a> {
    /// Every note this page laid out, by entry index.
    pub layouts: &'a std::collections::BTreeMap<usize, StreamLayout>,
    /// `w:separator`'s own content, laid out.
    pub separator: Option<&'a StreamLayout>,
    /// `w:continuationSeparator`'s.
    pub continuation_separator: Option<&'a StreamLayout>,
    /// `w:continuationNotice`'s — the line Word prints at the foot of a page whose note carries on.
    pub continuation_notice: Option<&'a StreamLayout>,
    /// Where a footnote sits on the page.
    pub position: FootnotePosition,
}

/// How tall a note area holding `carry` and `demand` **wants** to be, before any cap.
///
/// This is *N*(*R*) of the module's own argument, and it deliberately takes no available height: it
/// is the demand, and [`build`] is what fits the demand into the space there is.
///
/// **Zero when there is nothing to place**, and that is load-bearing rather than tidy: the separator
/// is furniture *for* the notes, so counting it on a page that has none would reserve a rule's
/// height out of the body of every page of every document that merely relates a footnotes part —
/// and, worse, would make the first iteration of the fixed point always disagree with `R = 0`, so
/// every page would be assembled twice. Neither is visible in a fixture that compares two documents
/// which both carry the part, because the loss cancels; `an_empty_note_area_costs_the_body_nothing`
/// compares against a document with **no** part at all.
#[must_use]
pub fn demanded_height(
    content: &NoteContent<'_>,
    carry: Option<NoteCarry>,
    demand: &[DemandedNote],
) -> Emu {
    if carry.is_none() && demand.is_empty() {
        return Emu::ZERO;
    }
    let mut total = separator_height(content, carry.is_some());
    if let Some(carry) = carry {
        if let Some(layout) = content.layouts.get(&carry.note) {
            total += layout.height_of(carry.line as usize..layout.line_count());
        }
    }
    for note in demand {
        if let Some(layout) = content.layouts.get(&note.note) {
            total += layout.height();
        }
    }
    total
}

/// Fills a note area of at most `available` with `carry` and then `demand`.
///
/// A note that does not fit is split at a line and the remainder is carried; **at least one line is
/// always placed** while there is anything to place, which is what makes the carry strictly shorter
/// on every page and the whole arrangement terminate. See this module's own documentation.
#[must_use]
pub fn build(
    content: &NoteContent<'_>,
    carry: Option<NoteCarry>,
    demand: &[DemandedNote],
    available: Emu,
) -> NoteArea {
    if carry.is_none() && demand.is_empty() {
        return NoteArea::empty();
    }
    let separator = separator_kind(content, carry.is_some());
    let separator_height = separator_height(content, carry.is_some());
    let mut area = NoteArea {
        separator,
        separator_height,
        notes: Vec::new(),
        height: separator_height,
        carry: None,
        continuation_notice: None,
        position: content.position,
    };
    let mut y = separator_height;
    let mut room = available - separator_height;

    // The carried note first: it is already on this page's conscience and nothing new may push it
    // off, which is what "carried" means.
    if let Some(carry) = carry {
        if let Some(layout) = content.layouts.get(&carry.note) {
            let from = carry.line as usize;
            let (fitted, height) = layout.lines_that_fit(from, room.maximum(Emu::ZERO));
            let end = from + fitted;
            area.notes.push(PlacedNote {
                note: carry.note,
                number: carry.number,
                lines: from..end,
                top: y,
                continued: true,
                continues: end < layout.line_count(),
            });
            y += height;
            room -= height;
            if end < layout.line_count() {
                area.carry = Some(NoteCarry {
                    note: carry.note,
                    number: carry.number,
                    line: u32::try_from(end).unwrap_or(u32::MAX),
                });
                area.height = y;
                place_notice(content, &mut area, y, room);
                return area;
            }
        }
    }

    for note in demand {
        let Some(layout) = content.layouts.get(&note.note) else {
            continue;
        };
        if layout.line_count() == 0 {
            continue;
        }
        let (fitted, height) = layout.lines_that_fit(0, room.maximum(Emu::ZERO));
        let continues = fitted < layout.line_count();
        area.notes.push(PlacedNote {
            note: note.note,
            number: note.number,
            lines: 0..fitted,
            top: y,
            continued: false,
            continues,
        });
        y += height;
        room -= height;
        if continues {
            area.carry = Some(NoteCarry {
                note: note.note,
                number: note.number,
                line: u32::try_from(fitted).unwrap_or(u32::MAX),
            });
            place_notice(content, &mut area, y, room);
            break;
        }
    }
    area.height = area.height.maximum(y);
    area
}

/// Puts the `continuationNotice` at the foot of the area, if the part defines one and there is room.
///
/// If there is not, it is **dropped rather than made room for**: it is a courtesy line, and taking
/// space from the note it announces would be a strange trade. See
/// [`NoteArea::continuation_notice`] for why it stays outside the fixed point.
fn place_notice(content: &NoteContent<'_>, area: &mut NoteArea, y: Emu, room: Emu) {
    area.height = area.height.maximum(y);
    let Some(notice) = content.continuation_notice else {
        return;
    };
    let height = notice.height();
    if height <= Emu::ZERO || height > room {
        return;
    }
    area.continuation_notice = Some(y);
    area.height = y + height;
}

/// Which rule is drawn above a page's notes.
fn separator_kind(content: &NoteContent<'_>, carried: bool) -> Option<FootnoteEndnoteType> {
    if carried {
        content
            .continuation_separator
            .map(|_| FootnoteEndnoteType::ContinuationSeparator)
            .or_else(|| content.separator.map(|_| FootnoteEndnoteType::Separator))
    } else {
        content.separator.map(|_| FootnoteEndnoteType::Separator)
    }
}

/// How tall that rule is.
fn separator_height(content: &NoteContent<'_>, carried: bool) -> Emu {
    let stream = if carried {
        content.continuation_separator.or(content.separator)
    } else {
        content.separator
    };
    stream.map_or(Emu::ZERO, StreamLayout::height)
}
