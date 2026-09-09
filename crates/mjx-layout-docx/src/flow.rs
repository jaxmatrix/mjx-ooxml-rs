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

use crate::generated::Composition;
use crate::justify::{place, points, LineContext, LinePlacement};
use crate::style::{LineHeight, ParagraphStyle, RunStyle};
use crate::tabs::TabRuler;
use crate::text::{
    composer, composer_runs, item_of, itemise_paragraph, paragraph_direction, resolve_face,
    shape_hyphen, CutPolicy, StyledItem, TextEngine,
};
use crate::wrap::{free_runs, next_clear_edge, run_for, Exclusion};

/// One line of a paragraph, laid out but not yet placed on a page.
#[derive(Clone, PartialEq, Debug)]
pub struct LaidOutLine {
    /// The bytes of the paragraph it covers.
    pub range: Range<usize>,
    /// Where the line's own measure starts, from the column's left edge.
    ///
    /// The paragraph's leading indent, except beside a float that pushed the line to its right — in
    /// which case it is the left edge of the free run the line was composed into.
    pub left: Emu,
    /// How wide that measure was.
    ///
    /// **This is the number a wrapping assertion reads.** A line beside a triangle is narrower near
    /// the triangle's base than near its apex, and nothing else in a `FragmentTree` says so: the
    /// glyphs only show where the text happened to break.
    pub measure: Emu,
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
    /// The string this paragraph was laid out from, and the map back to the document's own offsets.
    ///
    /// **Carried rather than recomputed**, because every line's `range` is in the *layout* string's
    /// offsets and a caller turning one into a [`mjx_layout::SourceRef`] needs the map. Recomposing
    /// it at emission time would mean composing every paragraph twice and would let the two
    /// compositions disagree — which is the failure that produces a caret in the wrong place with no
    /// error anywhere.
    pub composition: Composition,
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
    /// Where in its column the paragraph starts, which is what decides which floats its lines meet.
    ///
    /// Ignored entirely when `exclusions` is empty, which is every document with no drawing in it.
    pub top: Emu,
    /// What the paragraph's lines have to flow around, in the column's own coordinates.
    pub exclusions: &'a [Exclusion],
}

impl<'a> FlowContext<'a> {
    /// A context with nothing floating in it — the shape every caller had before MJXOFF-176.
    #[must_use]
    pub fn plain(
        column: LayoutRect,
        settings: &'a DocumentLayoutSettings,
        hyphenator: Option<&'a dyn Hyphenator>,
    ) -> Self {
        Self {
            column,
            settings,
            hyphenator,
            top: Emu::ZERO,
            exclusions: &[],
        }
    }
}

/// How many times one line may be composed before its band is accepted.
///
/// # The cycle, and why it is two rather than unbounded
///
/// A line's height decides which vertical band it occupies; the band decides how wide the line is;
/// and the width can change the line's height, because a narrower line takes fewer runs and may
/// therefore be shorter. That is a real cycle and an engine that iterated it to a fixed point could
/// oscillate between two heights for ever.
///
/// It is cut at **two**: the line is composed once against the band its predecessor's height
/// predicts, and once more against the band its own first composition produced. The second
/// composition is accepted whatever it says. The error that leaves is bounded by one line's height
/// — the second band is the right one unless the *second* composition changed height again — and it
/// is bounded in a direction that is visible rather than silent, because
/// [`LaidOutLine::measure`] reports the width the line was actually fitted against.
///
/// **`GUESS:`** Word's own number is not documented. Two is the smallest that makes the common case
/// (a line beside a float whose height is the paragraph's usual one) exactly right.
pub const BAND_COMPOSITIONS: usize = 2;

/// Lays `paragraph` out into lines inside `context`'s column.
///
/// # Errors
/// [`FontError`] when a face will not shape.
pub fn lay_out(
    engine: &mut TextEngine<'_>,
    paragraph: &ParagraphFormatting,
    context: FlowContext<'_>,
) -> Result<ParagraphLayout, FontError> {
    lay_out_composed(engine, &Composition::plain(paragraph), context)
}

/// Lays a **composed** paragraph out into lines inside `context`'s column.
///
/// The one this crate actually calls. [`lay_out`] is the same thing over a paragraph with nothing
/// generated — no list marker, no field value, no note mark, no inline object — which is what a
/// caller measuring one paragraph in isolation wants and what every suite written before
/// MJXOFF-177 asks for.
///
/// # Errors
/// [`FontError`] when a face will not shape.
pub fn lay_out_composed(
    engine: &mut TextEngine<'_>,
    composition: &Composition,
    context: FlowContext<'_>,
) -> Result<ParagraphLayout, FontError> {
    let style = composition.style().clone();
    let runs: Vec<RunStyle> = composition.runs().to_vec();
    let tabs = TabRuler::new(
        &style.tab_stops,
        Emu::from_twips(context.settings.default_tab_stop_twips),
    );
    let bars: Vec<Emu> = tabs.bars().collect();

    let text = composition.text();
    let direction = paragraph_direction(style.direction == mjx_text::TextDirection::RightToLeft);
    let bidi = BidiAnalysis::resolve(text, direction);
    let policy = CutPolicy::of(style.alignment);
    let items = itemise_paragraph(engine, text, &runs, &bidi, policy, composition.objects())?;
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

    // Where the paragraph's next line sits in its column, which only matters when something floats
    // beside it. `Emu::ZERO` and an empty exclusion list make every band query below a no-op, so a
    // document with no drawings in it runs the loop MJXOFF-174 wrote.
    let mut band_top = context.top;
    let mut previous_height: Option<Emu> = None;
    // What a line is assumed to be as tall as before it has been composed — the first band query of
    // the paragraph has nothing else to go on. Every later line uses its predecessor's real height.
    let nominal_height = style.line_height.applied_to(Emu::from_points(
        runs.first()
            .map_or(crate::style::ASSUMED_FONT_SIZE_POINTS, |run| {
                run.size.in_points()
            })
            * 1.2,
    ));

    while offset < text.len() {
        let full_measure = style.measure_of(lines.len(), column_width);
        let indent = style.leading_indent_of(lines.len());
        let allow_hyphenation = limit.is_none_or(|limit| limit <= 0 || consecutive_hyphens < limit);

        // The band this line occupies, and the free run inside it. Both are settled before the line
        // is composed, because the run *is* the measure.
        let mut skipped = Emu::ZERO;
        let mut left = indent;
        let mut measure = full_measure;
        let mut composed;
        let mut attempt = 0_usize;
        let mut guess = previous_height.unwrap_or(nominal_height);
        loop {
            if !context.exclusions.is_empty() {
                let (start, span, cleared) =
                    band_of(&context, indent, full_measure, band_top, guess);
                skipped = cleared;
                left = start;
                measure = span;
            }
            composed = if allow_hyphenation {
                line_composer.next_line(engine.shaper, offset, measure.points())?
            } else {
                line_composer.next_line_without_hyphenation(
                    engine.shaper,
                    offset,
                    measure.points(),
                )?
            };
            if composed.hyphenated {
                // `w:hyphenationZone`: only hyphenate when the line would otherwise fall further
                // than the zone short of the margin. Composed a second time, and only for a line
                // that did hyphenate, so an unhyphenated document pays nothing for the rule.
                let plain = line_composer.next_line_without_hyphenation(
                    engine.shaper,
                    offset,
                    measure.points(),
                )?;
                if measure - points(plain.width_in_points) <= zone && plain.range.end > offset {
                    composed = plain;
                }
            }
            attempt += 1;
            if context.exclusions.is_empty() || attempt >= BAND_COMPOSITIONS {
                break;
            }
            // Compose again only when the height this line actually came out at would have selected
            // a different band. See [`BAND_COMPOSITIONS`] for why there is no third attempt.
            let actual = style
                .line_height
                .applied_to(points(composed.ascent_in_points) + points(composed.descent_in_points));
            if actual == guess {
                break;
            }
            guess = actual;
        }
        consecutive_hyphens = if composed.hyphenated {
            consecutive_hyphens.saturating_add(1)
        } else {
            0
        };

        let end = composed.range.end;
        // **An inline object raises the line it sits on**, exactly as a very tall run does — which
        // is the other half of MJXOFF-176's declared gap. `crate::float::inline_height` computed
        // this and was never called; the composition now carries each object's own extent, so the
        // line it lands on is as tall as it is.
        let (object_ascent, object_descent) = composition.objects().iter().fold(
            (Emu::ZERO, Emu::ZERO),
            |(ascent, descent), object| {
                if object.at >= offset && object.at < end.max(offset + 1) {
                    (
                        ascent.maximum(object.ascent),
                        descent.maximum(object.descent),
                    )
                } else {
                    (ascent, descent)
                }
            },
        );
        let mut laid_out = lay_out_line(
            text,
            &items,
            composed,
            &style,
            &tabs,
            measure,
            left,
            end >= text.len(),
            object_ascent,
            object_descent,
        );
        // A band this line could not enter at all is skipped by growing the line box above the text
        // rather than by inserting a block of nothing: `ParagraphLayout::height_of` then reports the
        // paragraph's real extent with no further arithmetic, and a `wp:wrapTopAndBottom` object
        // therefore pushes the text below it without the paginator having to know it exists.
        if skipped > Emu::ZERO {
            laid_out.height += skipped;
            laid_out.baseline += skipped;
        }
        previous_height = Some(laid_out.height - skipped);
        band_top += laid_out.height;
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
        composition: composition.clone(),
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

/// The free run one line may use in the band starting at `top`, and how far the line had to move
/// down to find one.
///
/// Returns `(left, measure, skipped)`. **Termination:** every iteration moves `top` to the bottom
/// edge of an exclusion that blocked it, which is strictly greater than the `top` it came in with,
/// and there are finitely many exclusions — so the loop visits each at most once. The cap is
/// belt-and-braces against a malformed file whose object has a bottom above its top.
fn band_of(
    context: &FlowContext<'_>,
    indent: Emu,
    measure: Emu,
    top: Emu,
    height: Emu,
) -> (Emu, Emu, Emu) {
    let mut at = top;
    let mut skipped = Emu::ZERO;
    for _ in 0..=context.exclusions.len() {
        let runs = free_runs(
            indent,
            indent + measure,
            at,
            at + height,
            context.exclusions,
        );
        if let Some(run) = run_for(&runs, measure) {
            return (run.0, run.1 - run.0, skipped);
        }
        let Some(edge) = next_clear_edge(at, at + height, context.exclusions) else {
            break;
        };
        skipped += edge - at;
        at = edge;
    }
    (indent, measure, skipped)
}

#[allow(clippy::too_many_arguments)]
fn lay_out_line(
    text: &str,
    items: &[StyledItem],
    composed: ComposedLine,
    style: &ParagraphStyle,
    tabs: &TabRuler,
    measure: Emu,
    left: Emu,
    is_last: bool,
    object_ascent: Emu,
    object_descent: Emu,
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
            leading_indent: left,
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
    let ascent = points(composed.ascent_in_points).maximum(object_ascent);
    let descent = points(composed.descent_in_points).maximum(object_descent);
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
        left,
        measure,
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
        left: style.leading_indent_of(0),
        measure: Emu::ZERO,
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
