//! Where the pieces of one line go across the measure: the five alignments, the tab stops, and the
//! two kinds of expansion.
//!
//! # Justification is where an identity value hides
//!
//! Four of the five alignments move a line *rigidly* — start, end and centre translate every segment
//! by the same amount, and a start-aligned line is not translated at all. Only justification changes
//! the distance *between* segments, and it is therefore the only one where the arithmetic can be
//! wrong without a left-aligned fixture noticing. `tests/a_justified_line_has_positions.rs` asserts
//! segment positions rather than a line width for exactly that reason: a line width is identical
//! under a correct justifier and under one that does nothing but pad the end.
//!
//! # The East Asian difference, and where it comes from
//!
//! Latin justification widens the gaps **between words**, and a line with no spaces on it cannot be
//! justified at all. East Asian justification widens the gaps **between characters**, because a
//! Japanese line has no spaces and is expected to fill the measure anyway.
//!
//! Word does both, in one paragraph, decided per line: a run of ideographs is expanded between its
//! characters and a run of Latin words between its spaces. So [`expansion_points`] counts **two**
//! kinds of opportunity and the difference is visible immediately — a CJK line under a Latin-only
//! justifier is left-aligned with a ragged right edge, which is what an unjustified line looks like.

use mjx_layout::ComposedSegment;
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::TabStopLeader;

use crate::style::Alignment;
use crate::tabs::{TabKind, TabRuler, DECIMAL_SEPARATOR};
use crate::text::is_expansion_space;

/// One segment of a line, placed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlacedSegment {
    /// Which segment of [`ComposedLine::segments`](mjx_layout::ComposedLine) it is.
    pub segment: usize,
    /// Where its leading edge sits, from the column's leading edge.
    pub x: Emu,
    /// How wide it is.
    pub width: Emu,
}

/// A gap a tab opened, and what fills it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlacedLeader {
    /// Where the gap starts.
    pub from: Emu,
    /// Where it ends.
    pub to: Emu,
    /// What is drawn across it.
    pub leader: TabStopLeader,
}

/// One line, placed across the measure.
#[derive(Clone, PartialEq, Debug)]
pub struct LinePlacement {
    /// Every segment, in the order they are drawn.
    pub segments: Vec<PlacedSegment>,
    /// Every tab gap with a leader in it.
    pub leaders: Vec<PlacedLeader>,
    /// How wide the line is before any alignment moved it.
    pub natural_width: Emu,
    /// How many expansion opportunities justification found. Zero on a line that could not be
    /// justified at all, which is what a single unbroken word is.
    pub expansion_points: usize,
    /// How much each opportunity was widened by. Zero for every alignment but the two justifying
    /// ones, and zero on a justified line that already filled its measure — **the identity value**,
    /// which is why it is reported rather than inferred.
    pub expansion_each: Emu,
}

/// What the caller knows about a line that this module cannot see for itself.
#[derive(Clone, Copy, Debug)]
pub struct LineContext<'a> {
    /// Where the line starts, from the column's leading edge — the paragraph's indent for this line.
    pub leading_indent: Emu,
    /// How wide the line may be.
    pub measure: Emu,
    /// How it is aligned.
    pub alignment: Alignment,
    /// Whether it is the last line of its paragraph, which is what stops `w:jc="both"` from
    /// stretching a two-word final line across the page.
    pub is_last: bool,
    /// The paragraph's tab ruler.
    pub tabs: &'a TabRuler,
}

/// Places `segments` across the measure.
///
/// `is_tab` says, per segment, whether it is exactly a tab character; `text` is the paragraph's own
/// text, from which a decimal tab finds its separator.
#[must_use]
pub fn place(
    text: &str,
    segments: &[ComposedSegment],
    is_tab: &[bool],
    context: LineContext<'_>,
) -> LinePlacement {
    let mut placed: Vec<PlacedSegment> = Vec::with_capacity(segments.len());
    let mut leaders = Vec::new();
    let mut pen = context.leading_indent;
    let mut saw_tab = false;

    let mut index = 0_usize;
    while index < segments.len() {
        let width = points(segments[index].width_in_points);
        if is_tab.get(index).copied().unwrap_or(false) {
            saw_tab = true;
            let stop = context.tabs.next_after(pen);
            let target = match stop.kind {
                TabKind::Leading | TabKind::Bar => stop.position,
                TabKind::Centred => {
                    let chunk = chunk_width(segments, is_tab, index + 1);
                    stop.position - chunk.divided_by(2)
                }
                TabKind::Trailing => stop.position - chunk_width(segments, is_tab, index + 1),
                TabKind::Decimal => {
                    stop.position - width_before_separator(text, segments, is_tab, index + 1)
                }
            };
            // A tab never moves the pen backwards: a centred or trailing tab whose chunk is wider
            // than the space in front of it would otherwise draw the text on top of what came
            // before. Word clamps in the same direction, and a clamp is what stops the pen from
            // walking off the leading edge.
            let target = target.maximum(pen);
            if stop.leader != TabStopLeader::None && target > pen {
                leaders.push(PlacedLeader {
                    from: pen,
                    to: target,
                    leader: stop.leader,
                });
            }
            // The tab itself is a segment with an advance of its own, which is *not* its width here:
            // its width is the distance to the stop. It emits no glyph run (see `crate::flow`), so
            // recording it keeps every segment index addressable.
            placed.push(PlacedSegment {
                segment: index,
                x: pen,
                width: target - pen,
            });
            pen = target;
            index += 1;
            continue;
        }
        placed.push(PlacedSegment {
            segment: index,
            x: pen,
            width,
        });
        pen += width;
        index += 1;
    }

    let natural_width = pen - context.leading_indent;
    let mut placement = LinePlacement {
        segments: placed,
        leaders,
        natural_width,
        expansion_points: 0,
        expansion_each: Emu::ZERO,
    };

    if saw_tab {
        // GUESS: a line with a tab on it is not aligned any further. A tab stop is an absolute
        // position, so translating the line would move the text off the stop it was placed on —
        // which is the whole point of a tab. Word behaves this way for a justified line; whether it
        // does for a centred one with a leading tab is a question for the sitting.
        return placement;
    }

    let slack = context.measure - natural_width;
    match context.alignment {
        Alignment::Start => {}
        Alignment::Centre => translate(&mut placement, slack.divided_by(2)),
        Alignment::End => translate(&mut placement, slack),
        Alignment::Justified | Alignment::Distributed => {
            let stretches = context.alignment.stretches_the_last_line() || !context.is_last;
            if !stretches || slack <= Emu::ZERO {
                return placement;
            }
            let ranges: Vec<std::ops::Range<usize>> = segments
                .iter()
                .map(|segment| segment.range.clone())
                .collect();
            let points = expansion_points(text, &ranges, context.alignment);
            placement.expansion_points = points.len();
            if points.is_empty() {
                return placement;
            }
            #[allow(clippy::cast_possible_wrap)]
            let each = slack.divided_by(points.len() as i64);
            placement.expansion_each = each;
            // A cursor rather than a `contains`: `points` is ascending and so are the segments, so
            // a lookup per segment would make a distributed line — one segment per character —
            // quadratic in its own length.
            let mut widened = 0_i64;
            let mut cursor = points.iter().peekable();
            for placed in &mut placement.segments {
                placed.x += each.times(widened);
                if cursor.peek().is_some_and(|point| **point == placed.segment) {
                    cursor.next();
                    widened += 1;
                }
            }
        }
    }
    placement
}

/// Which segment *boundaries* justification may widen — named by the index of the segment **after**
/// which the gap sits.
///
/// Two kinds, and a line may hold both:
///
/// * a segment that ends with an expansion space, which is a Latin word gap;
/// * a boundary between two East Asian characters, which is the gap Japanese and Chinese
///   justification widens and Latin justification has no equivalent of.
///
/// Under [`Alignment::Distributed`] **every** boundary is an opportunity, which is what
/// *distributed* means: the text is spread evenly whatever it is made of.
///
/// It takes the segments' **byte ranges** rather than the segments, and that is deliberate: the
/// decision reads nothing but the text, so a suite can assert it without a shaped run — and this
/// repository commits no CJK face, so an end-to-end assertion about a Japanese line is not
/// available. See `tests/a_justified_line_has_positions.rs`.
#[must_use]
pub fn expansion_points(
    text: &str,
    ranges: &[std::ops::Range<usize>],
    alignment: Alignment,
) -> Vec<usize> {
    let mut points = Vec::new();
    for (index, range) in ranges.iter().enumerate() {
        let Some(next) = ranges.get(index + 1) else {
            // The gap after the last segment is the ragged edge, not an opportunity: widening it
            // would push the line past its own measure.
            break;
        };
        let opportunity = match alignment {
            Alignment::Distributed => true,
            Alignment::Justified => {
                ends_with(text, range, is_expansion_space)
                    || (ends_with(text, range, is_east_asian)
                        && starts_with(text, next, is_east_asian))
            }
            Alignment::Start | Alignment::Centre | Alignment::End => false,
        };
        if opportunity {
            points.push(index);
        }
    }
    points
}

fn translate(placement: &mut LinePlacement, by: Emu) {
    if by == Emu::ZERO {
        return;
    }
    for placed in &mut placement.segments {
        placed.x += by;
    }
    for leader in &mut placement.leaders {
        leader.from += by;
        leader.to += by;
    }
}

/// How wide the run of segments starting at `from` is, up to the next tab or the end of the line.
fn chunk_width(segments: &[ComposedSegment], is_tab: &[bool], from: usize) -> Emu {
    let mut total = Emu::ZERO;
    for (index, segment) in segments.iter().enumerate().skip(from) {
        if is_tab.get(index).copied().unwrap_or(false) {
            break;
        }
        total += points(segment.width_in_points);
    }
    total
}

/// How wide the text between `from` and the first [`DECIMAL_SEPARATOR`] after it is.
///
/// The whole chunk when there is no separator, which places a decimal tab exactly where a trailing
/// tab would — the reading that lines up a column of whole numbers with a column of decimals.
fn width_before_separator(
    text: &str,
    segments: &[ComposedSegment],
    is_tab: &[bool],
    from: usize,
) -> Emu {
    let mut total = Emu::ZERO;
    for (index, segment) in segments.iter().enumerate().skip(from) {
        if is_tab.get(index).copied().unwrap_or(false) {
            break;
        }
        let Some(slice) = text.get(segment.range.clone()) else {
            total += points(segment.width_in_points);
            continue;
        };
        if let Some(offset) = slice.find(DECIMAL_SEPARATOR) {
            // The separator is inside this segment. Its width is not divisible without re-shaping,
            // so the fraction of the segment before it is taken as a fraction of its advance —
            // exact for a monospaced digit run, which is what a decimal-tabbed column is.
            #[allow(clippy::cast_precision_loss)]
            let fraction = if slice.is_empty() {
                0.0
            } else {
                offset as f64 / slice.len() as f64
            };
            return total + points(segment.width_in_points * fraction);
        }
        total += points(segment.width_in_points);
    }
    total
}

fn ends_with(text: &str, range: &std::ops::Range<usize>, is: fn(char) -> bool) -> bool {
    text.get(range.clone())
        .and_then(|slice| slice.chars().next_back())
        .is_some_and(is)
}

fn starts_with(text: &str, range: &std::ops::Range<usize>, is: fn(char) -> bool) -> bool {
    text.get(range.clone())
        .and_then(|slice| slice.chars().next())
        .is_some_and(is)
}

/// Whether `character` is one an East Asian line may be stretched around.
///
/// The blocks that are set solid without spaces: the CJK ideographs and their extensions, the
/// kana, Hangul, the CJK symbols and punctuation, and the fullwidth forms. **GUESS:** the set is
/// read off Unicode's own block boundaries rather than from JIS X 4051, which defines the classes
/// for *prohibition* and not for *expansion*; `mjx-text`'s `KinsokuRules` already holds the
/// prohibition sets and this is deliberately not one of them.
#[must_use]
pub fn is_east_asian(character: char) -> bool {
    matches!(character as u32,
        0x1100..=0x11FF      // Hangul Jamo
        | 0x2E80..=0x2EFF    // CJK Radicals Supplement
        | 0x3000..=0x303F    // CJK Symbols and Punctuation
        | 0x3040..=0x309F    // Hiragana
        | 0x30A0..=0x30FF    // Katakana
        | 0x3130..=0x318F    // Hangul Compatibility Jamo
        | 0x31F0..=0x31FF    // Katakana Phonetic Extensions
        | 0x3400..=0x4DBF    // CJK Unified Ideographs Extension A
        | 0x4E00..=0x9FFF    // CJK Unified Ideographs
        | 0xA960..=0xA97F    // Hangul Jamo Extended-A
        | 0xAC00..=0xD7AF    // Hangul Syllables
        | 0xF900..=0xFAFF    // CJK Compatibility Ideographs
        | 0xFF00..=0xFF60    // Fullwidth Forms
        | 0x20000..=0x2FA1F  // CJK Unified Ideographs Extensions B onward
    )
}

/// A width in typographic points as a length.
#[must_use]
pub fn points(value: f64) -> Emu {
    if !value.is_finite() {
        return Emu::ZERO;
    }
    Emu::from_points(value)
}
