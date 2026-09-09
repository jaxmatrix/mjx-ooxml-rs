//! Where the page ends: the four constraints that move content between pages, and the termination
//! argument for each of them.
//!
//! # The dangerous gate
//!
//! *"The document paginates"* is green for an implementation that breaks every page at a fixed line
//! count and honours nothing. Every constraint below is therefore stated with **the page assignment
//! it changes**, and `tests/each_constraint_moves_a_paragraph.rs` asserts, for each of the four,
//! that a named paragraph lands on a different page when the constraint is switched off.
//!
//! # The four, and why each terminates
//!
//! * **`w:pageBreakBefore`** — the paragraph starts a page. It cannot loop: the break is taken only
//!   when something is already on the page, so taking it always leaves a non-empty page behind.
//! * **`w:keepLines`** — the paragraph is not split. A paragraph taller than a whole page could
//!   otherwise be pushed forward for ever, so it is placed **anyway** when the page it is pushed on
//!   to is already empty. That is the same page Word puts it on, overflowing.
//! * **`w:widowControl`** — no single line of a paragraph is left alone at the foot of a page or
//!   carried alone to the top of the next. It can only ever move lines *forward*, and it is switched
//!   off entirely for a paragraph that has nowhere forward to go, so the count of lines placed is
//!   monotone.
//! * **`w:keepNext`** — the paragraph shares a page with the one after it. This is the classic
//!   infinite loop: a chain longer than a page has no satisfying assignment, so the chain is broken
//!   at the point where breaking it would leave the page **empty**, and the first paragraph of an
//!   unsatisfiable chain is placed where it does not fit. `tests/termination.rs` runs a
//!   thousand-paragraph chain and asserts it produces pages.
//!
//! Above all of them sits the one guarantee the whole engine rests on: **every page places at least
//! one line.** A page that placed nothing would produce a next position equal to its own start, and
//! the page after it would be identical, for ever. Every constraint above is allowed to refuse
//! content only while something else is already on the page.

use std::collections::BTreeMap;

use mjx_docx::ParagraphFormatting;
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::BreakType;

use crate::flow::ParagraphLayout;

/// Where in the document a page starts.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct FlowPosition {
    /// Which paragraph, counted from zero across the body.
    pub paragraph: u32,
    /// Which of its lines, counted from zero. Zero is the start of the paragraph.
    pub line: u32,
}

impl FlowPosition {
    /// The start of the document.
    pub const START: Self = Self {
        paragraph: 0,
        line: 0,
    };
}

/// One paragraph, or the part of one, placed on a page.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacedParagraph {
    /// Which paragraph.
    pub paragraph: usize,
    /// Which of its lines are on this page.
    pub lines: std::ops::Range<usize>,
    /// Where the first of them sits, from the top of the column.
    pub top: Emu,
    /// How much empty space was left above it.
    pub space_before: Emu,
    /// Whether it began on an earlier page.
    pub continued: bool,
    /// Whether it carries on onto a later one.
    pub continues: bool,
}

/// One page, assembled.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PageAssembly {
    /// What is on it, in document order.
    pub blocks: Vec<PlacedParagraph>,
    /// Where the next page starts, or `None` when the document ended here.
    pub next: Option<FlowPosition>,
    /// How many paragraphs this page's assembly had to look at.
    ///
    /// **The instrument the checkpoint gate rests on**, and it counts *work* rather than output: a
    /// page assembled from a checkpoint looks at the paragraphs on it, and a page assembled by
    /// walking from the beginning looks at every paragraph before it too. A gate on the *fragments*
    /// cannot tell the two apart, because they produce the same page — which is exactly why a
    /// checkpoint that is never used still passes every output assertion.
    pub paragraphs_visited: u32,
}

/// What the paginator needs to know about the page it is filling.
#[derive(Clone, Copy, Debug)]
pub struct PageShape {
    /// How tall the column is.
    pub height: Emu,
}

/// Assembles the page that starts at `from`.
///
/// `layout_of` is asked for a paragraph's lines and must answer the same lines for the same
/// paragraph however many times it is asked — that equivalence is what makes page *N* alone and
/// pages 1..=*N* in order agree, and it is why paragraph layout takes no argument that depends on
/// which page the paragraph is on.
///
/// # Errors
/// Whatever `layout_of` fails with.
pub fn assemble<E>(
    paragraphs: &[ParagraphFormatting],
    from: FlowPosition,
    shape: PageShape,
    layout_of: &mut dyn FnMut(usize) -> Result<ParagraphLayout, E>,
) -> Result<(PageAssembly, BTreeMap<usize, ParagraphLayout>), E> {
    let mut layouts: BTreeMap<usize, ParagraphLayout> = BTreeMap::new();
    let mut blocks: Vec<PlacedParagraph> = Vec::new();
    let mut visited = 0_u32;
    let mut y = Emu::ZERO;
    let mut index = from.paragraph as usize;
    let mut line = from.line as usize;
    let mut next: Option<FlowPosition> = None;

    while index < paragraphs.len() {
        if let std::collections::btree_map::Entry::Vacant(slot) = layouts.entry(index) {
            slot.insert(layout_of(index)?);
            visited = visited.saturating_add(1);
        }
        // Inserted immediately above when absent.
        let Some(layout) = layouts.get(&index) else {
            break;
        };
        let total = layout.lines.len();
        if line >= total {
            index += 1;
            line = 0;
            continue;
        }

        // `w:pageBreakBefore` — but never at the very start of the document, where it would open the
        // file with a blank page. **GUESS:** Word ignores it on the first paragraph; ECMA-376 does
        // not say, and honouring it would be a visible extra page in every document whose first
        // heading style carries the flag.
        if layout.style.page_break_before && line == 0 && index > 0 && !blocks.is_empty() {
            next = Some(FlowPosition {
                paragraph: index as u32,
                line: 0,
            });
            break;
        }

        // GUESS: the space above a paragraph is suppressed at the top of a page. Word does this, and
        // the alternative — a band of white space above the first line of every page — is
        // immediately visible.
        //
        // `w:contextualSpacing` is the document's own suppression: *don't add space between
        // paragraphs of the same style*, which is what every bulleted list in every document relies
        // on. It is a question about two paragraphs' **identity** and not about their resolved
        // values, which is why `ParagraphFormatting::style_id` exists at all.
        let suppressed = line > 0
            || blocks.is_empty()
            || (layout.style.contextual_spacing
                && same_style(paragraphs, index, index.wrapping_sub(1)));
        let space_before = if suppressed {
            Emu::ZERO
        } else {
            layout.space_before
        };

        let available = shape.height - y - space_before;
        let mut fitted = lines_that_fit(layout, line, available);
        let remaining = total - line;

        if layout.style.keep_lines_together && fitted < remaining {
            // Not splittable. Push it whole — unless the page it would be pushed on to is this one,
            // which is the case that does not terminate.
            fitted = if blocks.is_empty() { remaining } else { 0 };
        }

        if layout.style.widow_control {
            fitted = widow_control(fitted, remaining);
        }

        if fitted == 0 {
            if blocks.is_empty() {
                // Nothing else is on the page, so refusing again would produce a page with nothing
                // on it and a next position identical to this one. One line is placed, overflowing.
                fitted = 1;
            } else {
                next = Some(FlowPosition {
                    paragraph: index as u32,
                    line: line as u32,
                });
                break;
            }
        }

        // A `w:br@type="page"` or `="column"` inside the paragraph ends the page at the line it
        // sits on, whatever else would have fitted.
        let hard = hard_break_within(&paragraphs[index], layout, line, fitted);
        let (fitted, hard_ended) = match hard {
            Some(at_line) => (at_line - line + 1, true),
            None => (fitted, false),
        };

        blocks.push(PlacedParagraph {
            paragraph: index,
            lines: line..line + fitted,
            top: y + space_before,
            space_before,
            continued: line > 0,
            continues: line + fitted < total,
        });
        y = y + space_before + layout.height_of(line..line + fitted);

        if hard_ended || line + fitted < total {
            let resume_line = line + fitted;
            next = Some(if resume_line >= total {
                FlowPosition {
                    paragraph: (index + 1) as u32,
                    line: 0,
                }
            } else {
                FlowPosition {
                    paragraph: index as u32,
                    line: resume_line as u32,
                }
            });
            break;
        }

        if !(layout.style.contextual_spacing && same_style(paragraphs, index, index + 1)) {
            y += layout.space_after;
        }
        index += 1;
        line = 0;
    }

    // `w:keepNext`, applied after the page is full because it is a statement about the *boundary*
    // and the boundary is not known until then.
    if next.is_some() {
        apply_keep_with_next(&mut blocks, &layouts, &mut next);
    }

    Ok((
        PageAssembly {
            blocks,
            next,
            paragraphs_visited: visited,
        },
        layouts,
    ))
}

/// Whether the paragraphs at `one` and `other` name the same `w:pStyle`.
///
/// Two paragraphs that name **no** style are the same style: that is the document's default
/// paragraph style, which is what an unstyled body is made of, and reading `None` as *different*
/// would turn `w:contextualSpacing` off for exactly the documents that use it most.
fn same_style(paragraphs: &[ParagraphFormatting], one: usize, other: usize) -> bool {
    match (paragraphs.get(one), paragraphs.get(other)) {
        (Some(left), Some(right)) => left.style_id() == right.style_id(),
        // A paragraph with no neighbour has nothing to share a style with, so the space stands.
        _ => false,
    }
}

/// How many of `layout`'s lines from `line` onward fit in `available`.
fn lines_that_fit(layout: &ParagraphLayout, line: usize, available: Emu) -> usize {
    if available <= Emu::ZERO {
        return 0;
    }
    let mut used = Emu::ZERO;
    let mut fitted = 0_usize;
    for candidate in layout.lines.iter().skip(line) {
        let next = used + candidate.height;
        if next > available {
            break;
        }
        used = next;
        fitted += 1;
    }
    fitted
}

/// `w:widowControl`, both halves.
///
/// An **orphan** is a single line left behind at the foot of a page; a **widow** is a single line
/// carried alone to the top of the next. Both are refused by moving lines *forward*, which is the
/// only direction that terminates: the number of lines placed never grows.
#[must_use]
pub fn widow_control(fitted: usize, remaining: usize) -> usize {
    if remaining < 2 || fitted == 0 || fitted >= remaining {
        // Nothing to protect: a one-line paragraph has no widow, and a paragraph that fits entirely
        // has no boundary inside it.
        return fitted;
    }
    let mut fitted = fitted;
    if remaining - fitted == 1 && fitted >= 2 {
        // A widow: one line would go over alone. Send a second with it.
        fitted -= 1;
    }
    if fitted == 1 {
        // An orphan: one line would stay behind alone. Send the paragraph whole.
        fitted = 0;
    }
    fitted
}

/// The line a hard break inside the paragraph ends the page at, if one falls inside the lines about
/// to be placed.
fn hard_break_within(
    paragraph: &ParagraphFormatting,
    layout: &ParagraphLayout,
    from: usize,
    count: usize,
) -> Option<usize> {
    for break_at in paragraph.hard_breaks() {
        // A column break with one column is a page break, which is what it means: the column *is*
        // the page. Multi-column flow within a page is MJXOFF-175 (R20), and until it exists a
        // column break that behaved differently would be a break nothing acted on.
        if !matches!(break_at.kind, BreakType::Page | BreakType::Column) {
            continue;
        }
        for offset in 0..count {
            let index = from + offset;
            let Some(line) = layout.lines.get(index) else {
                break;
            };
            if break_at.at >= line.range.start && break_at.at < line.range.end {
                return Some(index);
            }
        }
    }
    None
}

/// Moves a trailing `w:keepNext` chain on to the next page.
///
/// Only a paragraph placed **in full** can be moved: a paragraph split across the boundary already
/// has its last line on the same page as what follows it, which is what `w:keepNext` asks for.
fn apply_keep_with_next(
    blocks: &mut Vec<PlacedParagraph>,
    layouts: &BTreeMap<usize, ParagraphLayout>,
    next: &mut Option<FlowPosition>,
) {
    loop {
        // Never empty the page. This is the termination argument for an unsatisfiable chain: a
        // chain of `w:keepNext` paragraphs longer than a page has no assignment that satisfies it,
        // so the chain is broken here and the reader sees it broken rather than seeing nothing.
        if blocks.len() < 2 {
            return;
        }
        let Some(last) = blocks.last() else {
            return;
        };
        let Some(layout) = layouts.get(&last.paragraph) else {
            return;
        };
        if !layout.style.keep_with_next || last.lines.end != layout.lines.len() {
            return;
        }
        let moved = FlowPosition {
            paragraph: last.paragraph as u32,
            line: last.lines.start as u32,
        };
        // Only ever move backwards in the document; a `next` already earlier than this block would
        // mean the page was cut before it, which cannot happen.
        if next.is_some_and(|position| position <= moved) {
            return;
        }
        *next = Some(moved);
        blocks.pop();
    }
}
