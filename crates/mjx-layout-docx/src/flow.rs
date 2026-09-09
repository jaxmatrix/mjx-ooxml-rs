//! One paragraph, laid out into lines: the measure per line, the spacing between them, and the two
//! document settings that decide whether a word may be split.
//!
//! # What this does not do
//!
//! It does not break lines. `mjx-text` finds the opportunities, `mjx-layout`'s
//! [`LineComposer`](mjx_layout::LineComposer) fits them against a measure, and this decides *what
//! the measure is* — which is the paragraph's column less its indents, and different for the first
//! line than for the rest, which is what a hanging indent is.
//!
//! It does not paginate either. A paragraph is laid out **once**, independently of where on the page
//! it lands, and [`crate::paginate`] then decides which of its lines are on which page. That
//! separation is what makes a checkpoint cheap: resuming inside a paragraph costs one paragraph's
//! layout and not a page's.
//!
//! # Hyphenation, and the two rules the composer cannot hold
//!
//! `w:consecutiveHyphenLimit` counts consecutive *lines* and `w:hyphenationZone` compares the
//! unhyphenated line against a distance from the margin. Both are facts about a sequence of lines
//! rather than about one line's break opportunities, so both live here:
//!
//! * the limit is a counter, and a line that would exceed it is composed again through
//!   [`LineComposer::next_line_without_hyphenation`](mjx_layout::LineComposer::next_line_without_hyphenation);
//! * the zone is a comparison, so a line that *did* hyphenate is composed a second time without,
//!   and the unhyphenated one is kept when it already comes within the zone of the margin. The
//!   second composition is paid for only by lines that actually hyphenated.
//!
//! **What this project can supply as a hyphenator is the soft-hyphen one**, and that is a real
//! limitation rather than a placeholder: [`PatternHyphenator`](mjx_text::PatternHyphenator) exists
//! and works, and the Liang patterns it needs are *language data* that nobody has committed to this
//! repository. So `w:autoHyphenation` on a document with no soft hyphens in it changes nothing here
//! and changes a great deal in Word. A caller with a pattern set hands one in through
//! [`crate::DocumentBoxModel::with_hyphenator`](crate::DocumentBoxModel::with_hyphenator).

use std::ops::Range;

use mjx_docx::{DocumentLayoutSettings, ParagraphFormatting};
use mjx_layout::{ComposedLine, LayoutRect};
use mjx_ooxml_core::measure::Emu;
use mjx_text::{BidiAnalysis, FaceId, FontError, Hyphenator, LineBreakOptions, ShapedRun};

use crate::justify::{place, points, LineContext, LinePlacement};
use crate::style::{LineHeight, ParagraphStyle, RunStyle};
use crate::tabs::TabRuler;
use crate::text::{
    composer, composer_runs, item_of, itemise_paragraph, paragraph_direction, resolve_face,
    shape_hyphen, CutPolicy, StyledItem, TextEngine,
};

/// One line of a paragraph, laid out but not yet placed on a page.
#[derive(Clone, PartialEq, Debug)]
pub struct LaidOutLine {
    /// The bytes of the paragraph it covers.
    pub range: Range<usize>,
    /// How tall its line box is, once `w:spacing/w:line` has had its say.
    pub height: Emu,
    /// Where the baseline sits, measured down from the top of the line box.
    pub baseline: Emu,
    /// How far above the baseline the tallest thing on it reaches.
    pub ascent: Emu,
    /// How far below.
    pub descent: Emu,
    /// Where every segment went.
    pub placement: LinePlacement,
    /// The shaped segments themselves, in draw order.
    pub composed: ComposedLine,
    /// Per segment, whether it is a tab — a tab is placed and emits no glyphs.
    pub is_tab: Vec<bool>,
    /// Whether the line ends with a hyphen this crate drew rather than the document wrote.
    pub hyphenated: bool,
}

/// One paragraph, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct ParagraphLayout {
    /// Its lines, in order. Never empty: an empty paragraph still occupies one line, because a
    /// reader must be able to put a caret in it.
    pub lines: Vec<LaidOutLine>,
    /// Its resolved style.
    pub style: ParagraphStyle,
    /// The space above it, before contextual spacing is applied.
    pub space_before: Emu,
    /// The space below it, likewise.
    pub space_after: Emu,
    /// The hyphen a hyphenated line draws: which face, the shaped glyph, and its advance.
    pub hyphen: Option<(FaceId, ShapedRun, Emu)>,
    /// Where every `bar` tab stop sits, which is drawn whatever the text does.
    pub bars: Vec<Emu>,
}

impl ParagraphLayout {
    /// How tall lines `from..to` are.
    #[must_use]
    pub fn height_of(&self, lines: Range<usize>) -> Emu {
        self.lines
            .get(lines)
            .map(|slice| {
                slice
                    .iter()
                    .fold(Emu::ZERO, |total, line| total + line.height)
            })
            .unwrap_or(Emu::ZERO)
    }
}

/// Everything a paragraph needs to be laid out that is not in the paragraph.
#[derive(Clone, Copy, Debug)]
pub struct FlowContext<'a> {
    /// The column the paragraph flows into.
    pub column: LayoutRect,
    /// The document's own settings.
    pub settings: &'a DocumentLayoutSettings,
    /// The hyphenator in force, or `None` when nothing may be split.
    pub hyphenator: Option<&'a dyn Hyphenator>,
}

/// Lays `paragraph` out into lines inside `context`'s column.
///
/// # Errors
/// [`FontError`] when a face will not shape.
pub fn lay_out(
    engine: &mut TextEngine<'_>,
    paragraph: &ParagraphFormatting,
    context: FlowContext<'_>,
) -> Result<ParagraphLayout, FontError> {
    let style = ParagraphStyle::of(paragraph.properties());
    let runs: Vec<RunStyle> = paragraph
        .runs()
        .iter()
        .map(|run| RunStyle::of(run.range.clone(), &run.properties))
        .collect();
    let tabs = TabRuler::new(
        &style.tab_stops,
        Emu::from_twips(context.settings.default_tab_stop_twips),
    );
    let bars: Vec<Emu> = tabs.bars().collect();

    let text = paragraph.text();
    let direction = paragraph_direction(style.direction == mjx_text::TextDirection::RightToLeft);
    let bidi = BidiAnalysis::resolve(text, direction);
    let policy = CutPolicy::of(style.alignment);
    let items = itemise_paragraph(engine, text, &runs, &bidi, policy)?;
    let composer_runs = composer_runs(engine.rasteriser, engine.features, &items);

    // The hyphen is shaped once per paragraph, in the first run's face at the first run's size,
    // because its *advance* is what every candidate measurement at a hyphenation point needs and a
    // per-line answer would mean re-measuring the candidates.
    //
    // GUESS: a hyphenated line's hyphen takes the paragraph's first run's face rather than the face
    // of the word it splits. The two differ only in a paragraph that changes font mid-word, which is
    // rare and which Word resolves the other way.
    let hyphenator = hyphenator_for(&style, context);
    // The glyph is resolved when *anything* on this paragraph could end a line with one: a
    // hyphenator that may split a word, or a soft hyphen the author already wrote.
    let may_hyphenate = hyphenator.is_some() || text.contains(mjx_docx::SOFT_HYPHEN);
    let hyphen = match (may_hyphenate, runs.first()) {
        (true, Some(first)) => match resolve_face(engine.fonts, engine.rasteriser, first)? {
            Some((face, face_id)) => {
                let (run, width) = shape_hyphen(engine.shaper, &face, first.size, engine.features)?;
                Some((face_id, run, points(width)))
            }
            None => None,
        },
        _ => None,
    };
    #[allow(clippy::option_if_let_else)]
    let hyphen_width = match &hyphen {
        Some((_, _, width)) => width.points(),
        None => 0.0,
    };

    let options = LineBreakOptions {
        east_asian_rules: style.east_asian_line_breaking,
        hanging_punctuation: style.overflow_punctuation,
        kinsoku: mjx_text::KinsokuRules::japanese_standard(),
    };
    let line_composer = composer(
        text,
        &composer_runs,
        &bidi,
        options,
        hyphenator.map(|hyphenator| (hyphenator, hyphen_width)),
    );

    let column_width = context.column.width();
    let mut lines: Vec<LaidOutLine> = Vec::new();
    let mut offset = 0_usize;
    let mut consecutive_hyphens = 0_i64;
    let limit = context.settings.consecutive_hyphen_limit;
    let zone = context
        .settings
        .hyphenation_zone_twips
        .map(Emu::from_twips)
        .unwrap_or(Emu::ZERO);

    while offset < text.len() {
        let measure = style.measure_of(lines.len(), column_width);
        let allow_hyphenation = limit.is_none_or(|limit| limit <= 0 || consecutive_hyphens < limit);
        let mut composed = if allow_hyphenation {
            line_composer.next_line(engine.shaper, offset, measure.points())?
        } else {
            line_composer.next_line_without_hyphenation(engine.shaper, offset, measure.points())?
        };
        if composed.hyphenated {
            // `w:hyphenationZone`: only hyphenate when the line would otherwise fall further than
            // the zone short of the margin. Composed a second time, and only for a line that did
            // hyphenate, so an unhyphenated document pays nothing for the rule.
            let plain = line_composer.next_line_without_hyphenation(
                engine.shaper,
                offset,
                measure.points(),
            )?;
            if measure - points(plain.width_in_points) <= zone && plain.range.end > offset {
                composed = plain;
            }
        }
        consecutive_hyphens = if composed.hyphenated {
            consecutive_hyphens.saturating_add(1)
        } else {
            0
        };

        let end = composed.range.end;
        let laid_out = lay_out_line(
            text,
            &items,
            composed,
            &style,
            &tabs,
            measure,
            lines.len(),
            end >= text.len(),
        );
        lines.push(laid_out);
        if end <= offset {
            // The composer could not advance — a measure narrower than one glyph. Taking the rest of
            // the paragraph as one line is what terminates the loop, and it is what a reader sees in
            // Word too: text that overflows a column too narrow to hold it.
            if let Some(last) = lines.last_mut() {
                last.range = offset..text.len();
            }
            break;
        }
        offset = end;
    }

    if lines.is_empty() {
        lines.push(empty_line(engine, &runs, &style)?);
    }

    Ok(ParagraphLayout {
        lines,
        space_before: style.space_before,
        space_after: style.space_after,
        style,
        hyphen,
        bars,
    })
}

/// Which hyphenator a paragraph gets, and why a soft hyphen does not need one.
///
/// **UAX #14 already breaks at a `U+00AD SOFT HYPHEN`** — its line-breaking class is `BA`, break
/// after — so an authored hyphenation point needs no hyphenator at all and works in a document
/// whose `w:autoHyphenation` is off, which is what *manual* hyphenation means. What a hyphenator
/// adds is the points the author did **not** write, which is `w:autoHyphenation`'s subject; and
/// `w:suppressAutoHyphens` on a paragraph switches that off for that paragraph alone while leaving
/// its soft hyphens working.
///
/// Returning a [`SoftHyphenHyphenator`](mjx_text::SoftHyphenHyphenator) here would be a second
/// implementation of a rule `mjx-text` already holds, and its points would be deduplicated against
/// the ordinary opportunities anyway — one pass of the paragraph for nothing.
fn hyphenator_for<'a>(
    style: &ParagraphStyle,
    context: FlowContext<'a>,
) -> Option<&'a dyn Hyphenator> {
    if style.suppress_auto_hyphens || !context.settings.auto_hyphenation {
        return None;
    }
    context.hyphenator
}

#[allow(clippy::too_many_arguments)]
fn lay_out_line(
    text: &str,
    items: &[StyledItem],
    composed: ComposedLine,
    style: &ParagraphStyle,
    tabs: &TabRuler,
    measure: Emu,
    index: usize,
    is_last: bool,
) -> LaidOutLine {
    let is_tab: Vec<bool> = composed
        .segments
        .iter()
        .map(|segment| item_of(items, &segment.range).is_some_and(|item| items[item].is_tab))
        .collect();
    let placement = place(
        text,
        &composed.segments,
        &is_tab,
        LineContext {
            leading_indent: style.leading_indent_of(index),
            measure,
            alignment: style.alignment,
            is_last,
            tabs,
        },
    );
    // A line that ends at a soft hyphen draws one, and the composer cannot say so: the break came
    // from UAX #14's own opportunity list rather than from a hyphenator, so `ComposedLine` reports
    // it as an ordinary fitted break. Whether a *hyphen is drawn* is a rendering rule about the
    // text and belongs here.
    let hyphenated = composed.hyphenated
        || text
            .get(composed.range.clone())
            .is_some_and(|slice| slice.ends_with(mjx_docx::SOFT_HYPHEN));
    let ascent = points(composed.ascent_in_points);
    let descent = points(composed.descent_in_points);
    let natural = ascent + descent;
    let height = style.line_height.applied_to(natural);
    // Where the baseline sits inside a line box the paragraph resized. Under `exact` the box may be
    // shorter than the text, and the text is then clipped from the **top**, which is what Word does:
    // the descender stays on the baseline and the ascender is what runs into the line above.
    //
    // GUESS: for `atLeast` and for a multiple, the extra leading goes **above** the baseline. Word
    // distributes it that way for `atLeast`; whether it does for a multiple is a question for the
    // sitting, and putting it below would move every baseline on the page.
    let baseline = match style.line_height {
        LineHeight::Exact(_) => height - descent,
        LineHeight::Multiple(_) | LineHeight::AtLeast(_) => height - descent,
    };
    LaidOutLine {
        range: composed.range.clone(),
        height,
        baseline: baseline.maximum(Emu::ZERO),
        ascent,
        descent,
        placement,
        hyphenated,
        is_tab,
        composed,
    }
}

/// The one line an empty paragraph still occupies.
///
/// There are no glyphs to take an ascent from, so the face's own metrics answer — which is why this
/// resolves a face for a paragraph that will draw nothing.
fn empty_line(
    engine: &mut TextEngine<'_>,
    runs: &[RunStyle],
    style: &ParagraphStyle,
) -> Result<LaidOutLine, FontError> {
    let probe = runs.first().cloned().unwrap_or_else(|| RunStyle {
        range: 0..0,
        family: crate::style::UNNAMED_FAMILY.to_owned(),
        size: mjx_text::FontSize::from_points(crate::style::ASSUMED_FONT_SIZE_POINTS),
        weight: mjx_text::FontWeight::REGULAR,
        slant: mjx_text::FontSlant::Upright,
        language: None,
        hidden: false,
    });
    let (ascent, descent) = match resolve_face(engine.fonts, engine.rasteriser, &probe)? {
        Some((face, _)) => {
            let metrics = face.metrics();
            let units = f64::from(metrics.units_per_em.max(1));
            let size = probe.size.in_points();
            (
                Emu::from_points(f64::from(metrics.ascender) * size / units),
                Emu::from_points(f64::from(metrics.descender).abs() * size / units),
            )
        }
        None => (Emu::ZERO, Emu::ZERO),
    };
    let natural = ascent + descent;
    let height = style.line_height.applied_to(natural);
    Ok(LaidOutLine {
        range: 0..0,
        height,
        baseline: (height - descent).maximum(Emu::ZERO),
        ascent,
        descent,
        placement: LinePlacement {
            segments: Vec::new(),
            leaders: Vec::new(),
            natural_width: Emu::ZERO,
            expansion_points: 0,
            expansion_each: Emu::ZERO,
        },
        composed: ComposedLine {
            range: 0..0,
            hanging: 0..0,
            kind: mjx_text::LineBreakKind::EndOfText,
            segments: Vec::new(),
            width_in_points: 0.0,
            hanging_width_in_points: 0.0,
            ascent_in_points: ascent.points(),
            descent_in_points: descent.points(),
            hyphenated: false,
            measure_probes: 0,
        },
        is_tab: Vec::new(),
        hyphenated: false,
    })
}
