//! Composing one line of text — the box model's side of the text engine.
//!
//! # What this is and is not
//!
//! `mjx-text` says where a line **may** end, what glyphs a run becomes, and which way each stretch
//! reads. It never says where a line goes, how tall it is, or what flows around it. This module is
//! the join: given a paragraph, the runs it is made of and a measure, it produces the shaped
//! segments of one line, in the order they are drawn, with the widths a box model positions them by.
//! Everything above that — where the line sits, what indents it, whether it fits on the page — is
//! the box model's.
//!
//! Nothing here re-implements measurement. Every width comes from a [`ShapedRun`] the shaper
//! produced.
//!
//! # Shape in logical order, place in visual order
//!
//! This is `mjx-text`'s rule and it is easy to get backwards, because getting it backwards produces
//! text that **measures correctly and draws in the wrong order** — a bug that survives every width
//! assertion and is visible only in a picture.
//!
//! A shaper is handed logical text and told which way it runs; UAX #9 rule L2 then says which order
//! the resulting runs are drawn in. So [`LineComposer`] shapes each stretch in
//! [`BidiAnalysis::logical_runs`] order and emits [`ComposedLine::segments`] in
//! [`BidiAnalysis::visual_runs`] order — and within a right-to-left level run that is split by a
//! font change, the *items* are reversed too, because the logically last one is the visually
//! leftmost.
//!
//! # `unsafe_to_break`, and why measuring by slicing is a correctness question
//!
//! Fitting a line means asking *how wide is `text[from..candidate]`?* many times, once per candidate
//! break. Doing that by re-shaping every candidate is quadratic in the number of break opportunities
//! on a line, which is unaffordable; doing it by summing the advances of an already-shaped run is
//! linear, and is what every fast text stack does.
//!
//! It is also **wrong at some boundaries**, and HarfBuzz says exactly which. Shaping is contextual:
//! `f` followed by `i` may become one `ﬁ` glyph, and `A` followed by `V` is kerned closer together
//! than either is alone. At such a boundary, shaping the two halves separately does not give the
//! same answer as slicing the whole, and [`ShapedGlyph::unsafe_to_break`](mjx_text::ShapedGlyph) is
//! the shaper's flag for it. A line breaker that slices anyway loses a ligature or a kern **at every
//! line end**, and the error is a fraction of an em per line — small enough to survive a
//! screenshot, large enough to move where the next line breaks, and therefore large enough to
//! diverge from Word's pagination within a page or two.
//!
//! So [`slice_width`] returns `None` at an unsafe boundary and the composer re-shapes instead. The
//! flag decides which of the two paths runs, and `tests/text_composition.rs` proves it by measuring
//! both answers on a fixture where they differ.
//!
//! Note what this does *not* say: the segments a composed line finally carries are **always**
//! re-shaped from the line's own text, never sliced, because a [`ShapedRun`] is what a fragment
//! carries and a subrange of one is not a `ShapedRun`. The safe-slicing path is a *measurement*
//! optimisation, and the flag is what keeps that measurement honest.

use std::ops::Range;
use std::sync::Arc;

use mjx_text::{
    AdvanceWidth, BidiAnalysis, BidiLevel, FaceId, FeatureSet, FontError, FontFace, FontSize,
    Hyphenator, LineBreakKind, LineBreakOptions, LineBreaker, ShapedGlyph, ShapedRun, Shaper,
    ShapingRequest, TextDirection, TextScript,
};

/// One stretch of a paragraph that a shaper can take: one direction, one script, one face, one size.
///
/// This is what `mjx-text`'s [`itemise`](mjx_text::itemise) produces, plus the formatting the
/// document put on the run — which is the box model's to supply, because only it has read the
/// document.
#[derive(Clone, Debug)]
pub struct TextRun<'a> {
    /// The bytes it covers, in the paragraph's own offsets.
    pub range: Range<usize>,
    /// The face to shape it in.
    pub face: &'a Arc<FontFace>,
    /// That face's identity in the rasteriser that will draw it, which is what a fragment carries.
    pub face_id: FaceId,
    /// The one script it is in.
    pub script: TextScript,
    /// Which way it is shaped.
    pub direction: TextDirection,
    /// The UAX #9 level it was resolved at.
    pub level: BidiLevel,
    /// The size it is set at.
    pub size: FontSize,
    /// The OpenType features in force.
    pub features: &'a FeatureSet,
    /// Its language, as a BCP 47 tag, or `None`.
    pub language: Option<&'a str>,
    /// A width in points that **replaces the shaper's answer** for this run, in which case the run
    /// is not shaped at all and carries no glyphs.
    ///
    /// # What this is for, and why it is on the run rather than beside it
    ///
    /// A line is not always made only of text. A `.docx` puts a picture inline in a paragraph, an
    /// equation inline in a sentence, and a `.pptx` will put a field's placeholder there; each is an
    /// **atomic inline box** — a thing that occupies a character's position on the line, has a width
    /// nothing in a font knows, and must not be broken inside. UAX #14 has a class for exactly this
    /// (`CB`, contingent break, whose representative character is `U+FFFC OBJECT REPLACEMENT
    /// CHARACTER`), so the line breaker already knows what to do with one; what it did **not** have
    /// until MJXOFF-177 was a way to be told how wide it is.
    ///
    /// Without it, the only honest thing a box model could do was leave the object out of the
    /// measure, and a line carrying one was then measured as if it were not there — one object too
    /// long, breaking in the wrong place, ending the page in the wrong place. Two children in a row
    /// declared that gap rather than closing it (MJXOFF-175's footnote mark and MJXOFF-176's inline
    /// drawing), because closing it is a change to the contract **all three** box models share, and
    /// that is a decision to take once and deliberately rather than to smuggle into a format crate.
    ///
    /// It is on [`TextRun`] rather than in a parallel list because the composer's fitting loop
    /// measures *ranges*, and the only structure it has for "which formatting applies to these
    /// bytes" is this one. A second list would have to be intersected with the runs on every probe.
    ///
    /// **A fixed-advance run is all-or-nothing.** Its advance is counted whenever a candidate range
    /// touches it at all, because an object has no interior to measure a prefix of; a box model
    /// therefore gives each object a run of exactly one `U+FFFC`, which is what makes "touches it"
    /// and "contains it" the same question.
    pub advance: Option<f64>,
}

impl TextRun<'_> {
    fn shaping_request<'text>(&self, text: &'text str) -> ShapingRequest<'text>
    where
        Self: 'text,
    {
        let mut request = ShapingRequest::new(text, self.script, self.size, self.features)
            .in_direction(self.direction);
        if let Some(language) = self.language {
            request = request.in_language(language);
        }
        request
    }
}

/// One shaped stretch of a composed line, ready to become a
/// [`GlyphRunFragment`](crate::GlyphRunFragment).
#[derive(Clone, PartialEq, Debug)]
pub struct ComposedSegment {
    /// The bytes it covers, in the paragraph's own **logical** offsets.
    pub range: Range<usize>,
    /// Which face, as the rasteriser numbered it.
    pub face: FaceId,
    /// Which way it reads.
    pub direction: TextDirection,
    /// The level it was resolved at.
    pub level: BidiLevel,
    /// The glyphs, in draw order.
    pub run: ShapedRun,
    /// How wide it is, in typographic points at its own size.
    pub width_in_points: f64,
}

/// One line, composed: where it ends, what is on it, and how big it is.
#[derive(Clone, PartialEq, Debug)]
pub struct ComposedLine {
    /// The bytes the line covers, in the paragraph's own offsets. The next line starts at `end`.
    pub range: Range<usize>,
    /// The trailing bytes allowed to hang past the measure — `w:overflowPunct`. Empty when hanging
    /// punctuation is off or the line ends at a hard break.
    pub hanging: Range<usize>,
    /// Why the line ended where it did.
    pub kind: LineBreakKind,
    /// The shaped stretches, in **visual** order — left to right on the page, whatever the
    /// paragraph's direction. Their `range`s stay logical, so a caret still walks them in document
    /// order.
    pub segments: Vec<ComposedSegment>,
    /// How wide the line is in points, hanging tail included — the width it is actually drawn at.
    pub width_in_points: f64,
    /// How much of that width hangs past the measure and was not counted when the line was fitted.
    pub hanging_width_in_points: f64,
    /// How far the line reaches above its baseline, in points: the largest ascent on it.
    pub ascent_in_points: f64,
    /// How far it reaches below, in points.
    pub descent_in_points: f64,
    /// Whether the line ends at a hyphenation point, and so draws a hyphen the text does not
    /// contain.
    ///
    /// The box model appends the glyph — this crate never invents text — but it is decided here,
    /// because only the composer knows which opportunity the fitting loop took. `width_in_points`
    /// **includes** the hyphen's advance, because that is the width the line was fitted against and
    /// the width it is drawn at.
    pub hyphenated: bool,
    /// How many times the breaker asked for the width of a candidate slice.
    ///
    /// Not decoration: it is the observable difference between the fast path for a paragraph that
    /// cannot be broken and the general one, and `tests/text_composition.rs` asserts it is zero for
    /// the first. A counter nothing reads would be exactly the dead field this crate is written
    /// against, so it is public and it is asserted on.
    pub measure_probes: u32,
}

impl ComposedLine {
    /// Whether the line ran to the end of the paragraph.
    #[must_use]
    pub fn is_last(&self) -> bool {
        matches!(self.kind, LineBreakKind::EndOfText)
    }

    /// How tall the line is — ascent plus descent, in points.
    #[must_use]
    pub fn height_in_points(&self) -> f64 {
        self.ascent_in_points + self.descent_in_points
    }
}

/// Composes the lines of one paragraph.
///
/// Built once per paragraph, because [`LineBreaker`] finds the paragraph's break opportunities once
/// and every line after the first reuses them. Building one per line — which the free function
/// [`break_opportunities`](mjx_text::break_opportunities) would invite — would re-run UAX #14 over
/// the whole paragraph for every line of it.
#[derive(Debug)]
pub struct LineComposer<'a> {
    text: &'a str,
    runs: &'a [TextRun<'a>],
    bidi: &'a BidiAnalysis,
    breaker: LineBreaker<'a>,
    /// The advance of the hyphen a hyphenated line ends with, or `None` when this composer does not
    /// hyphenate at all.
    hyphen: Option<f64>,
}

impl<'a> LineComposer<'a> {
    /// A composer for `text`, made of `runs`, resolved by `bidi`, broken under `options`.
    ///
    /// `runs` must cover `text` in order and must not overlap. A gap is text that will not be drawn
    /// and an overlap is text drawn twice; neither is refused here, because both come from a
    /// document and refusing the paragraph would lose text a reader can see.
    #[must_use]
    pub fn new(
        text: &'a str,
        runs: &'a [TextRun<'a>],
        bidi: &'a BidiAnalysis,
        options: LineBreakOptions,
    ) -> Self {
        Self {
            text,
            runs,
            bidi,
            breaker: LineBreaker::new(text, options),
            hyphen: None,
        }
    }

    /// The same, hyphenating: `hyphenator` is asked where each word may be split, and a line may end
    /// inside one.
    ///
    /// `hyphen_width_in_points` is the advance of the hyphen the box model will draw, at the size it
    /// will draw it. It is a number rather than a face because the composer must **add it to every
    /// candidate it measures** — a line fitted without it overruns the measure by a third of an em
    /// on every hyphenated line, which is a defect no width assertion on the *slice* can see.
    ///
    /// Word's `w:consecutiveHyphenLimit` is not expressible here and is not meant to be: it is a
    /// property of the *page*, not of a paragraph, so the box model counts and calls
    /// [`LineComposer::next_line_without_hyphenation`] when the limit is reached.
    #[must_use]
    pub fn hyphenating(
        text: &'a str,
        runs: &'a [TextRun<'a>],
        bidi: &'a BidiAnalysis,
        options: LineBreakOptions,
        hyphenator: &dyn Hyphenator,
        hyphen_width_in_points: f64,
    ) -> Self {
        Self {
            text,
            runs,
            bidi,
            breaker: LineBreaker::with_hyphenation(text, options, hyphenator),
            hyphen: Some(hyphen_width_in_points.max(0.0)),
        }
    }

    /// The paragraph being composed.
    #[must_use]
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// Whether any line starting at `from` could end before the paragraph does.
    ///
    /// False for a paragraph with no interior break opportunity at all — one long word, a single
    /// CJK-free token, an empty cell, a title. That case is extremely common and it is the reason
    /// [`LineBreaker::opportunities`] is read here rather than the free function
    /// [`break_opportunities`](mjx_text::break_opportunities) being called: knowing the whole
    /// paragraph's opportunities up front lets [`LineComposer::next_line`] skip the measuring loop
    /// entirely, which is the difference between shaping the line once and shaping it once per
    /// candidate.
    #[must_use]
    pub fn can_break_before_the_end(&self, from: usize) -> bool {
        self.breaker
            .opportunities()
            .iter()
            .any(|opportunity| opportunity.at > from && opportunity.at < self.text.len())
    }

    /// Compose the line starting at `from`, fitting `measure` points.
    ///
    /// Returns a line covering nothing when `from` is at or past the end of the paragraph, which is
    /// how a caller's loop terminates.
    ///
    /// # Errors
    ///
    /// [`FontError`] if a face will not shape — the only thing that can go wrong here, because every
    /// other decision is arithmetic over what the shaper returned.
    pub fn next_line(
        &self,
        shaper: &mut Shaper,
        from: usize,
        measure: f64,
    ) -> Result<ComposedLine, FontError> {
        self.compose_line(shaper, from, measure, true)
    }

    /// The same, refusing to end the line inside a word even though this composer hyphenates.
    ///
    /// What `w:consecutiveHyphenLimit` needs: the limit counts *lines*, so it is the box model that
    /// knows the count and this is how it says so. On a composer built by [`LineComposer::new`] it
    /// is identical to [`LineComposer::next_line`].
    ///
    /// # Errors
    ///
    /// As [`LineComposer::next_line`].
    pub fn next_line_without_hyphenation(
        &self,
        shaper: &mut Shaper,
        from: usize,
        measure: f64,
    ) -> Result<ComposedLine, FontError> {
        self.compose_line(shaper, from, measure, false)
    }

    fn compose_line(
        &self,
        shaper: &mut Shaper,
        from: usize,
        measure: f64,
        hyphenate: bool,
    ) -> Result<ComposedLine, FontError> {
        let from = from.min(self.text.len());
        if from >= self.text.len() {
            return Ok(self.empty_line(from));
        }

        let (break_end, hanging, kind, probes) = if self.can_break_before_the_end(from) {
            self.fit(shaper, from, measure, hyphenate)?
        } else {
            // No interior opportunity, so the line runs to the end of the paragraph whatever the
            // measure says. Measuring candidates would ask a question with one possible answer.
            (
                self.text.len(),
                self.breaker.hanging_tail(from..self.text.len()),
                LineBreakKind::EndOfText,
                0,
            )
        };

        let mut line = self.compose(shaper, from..break_end)?;
        line.hanging = hanging.clone();
        line.kind = kind;
        line.measure_probes = probes;
        line.hanging_width_in_points = if hanging.start >= hanging.end {
            0.0
        } else {
            self.width_of(shaper, hanging)?
        };
        if hyphenate && self.breaker.is_hyphenation_point(break_end) {
            line.hyphenated = true;
            // The width the line is drawn at includes the glyph the box model is about to append,
            // and it is the same number the fitting loop measured this candidate by. Reporting the
            // slice's width instead would make every consumer of `width_in_points` — justification
            // above all — spread the hyphen's advance across the words.
            line.width_in_points += self.hyphen.unwrap_or(0.0);
        }
        Ok(line)
    }

    /// Where the line ends, by asking the breaker and measuring candidates.
    fn fit(
        &self,
        shaper: &mut Shaper,
        from: usize,
        measure: f64,
        hyphenate: bool,
    ) -> Result<(usize, Range<usize>, LineBreakKind, u32), FontError> {
        // One shaping of each item's tail, reused by every candidate. Shaping the tail *from this
        // line's start* rather than from the item's start is itself the re-shaping rule: the
        // previous line ended here, so the text after the break is shaped as its own run.
        let mut tails: Vec<Option<ItemShaping>> = vec![None; self.runs.len()];
        let mut failure: Option<FontError> = None;
        let mut probes = 0_u32;

        let broken = {
            let mut width_of = |range: Range<usize>| -> f64 {
                probes = probes.saturating_add(1);
                let candidate_is_hyphenated = self.breaker.is_hyphenation_point(range.end);
                match self.measure(shaper, &mut tails, range) {
                    Ok(width) => {
                        if candidate_is_hyphenated {
                            width + self.hyphen.unwrap_or(0.0)
                        } else {
                            width
                        }
                    }
                    Err(error) => {
                        failure.get_or_insert(error);
                        // Nothing fits, so the breaker takes the first opportunity and terminates
                        // rather than looping; the error is returned below regardless.
                        f64::INFINITY
                    }
                }
            };
            self.breaker
                .next_line_with(from, measure, hyphenate, &mut width_of)
        };

        if let Some(error) = failure {
            return Err(error);
        }
        Ok((broken.end, broken.hanging, broken.kind, probes))
    }

    /// The width of `range` in points, shaping each item's tail once and slicing it where the shaper
    /// says slicing is safe.
    fn measure(
        &self,
        shaper: &mut Shaper,
        tails: &mut [Option<ItemShaping>],
        range: Range<usize>,
    ) -> Result<f64, FontError> {
        let mut total = 0.0_f64;
        for (index, run) in self.runs.iter().enumerate() {
            let Some(overlap) = intersect(&run.range, &range) else {
                continue;
            };
            // An atomic inline box has no interior, so no tail is shaped for it and no slice is
            // taken: its advance is counted whole the moment a candidate touches it.
            if let Some(advance) = run.advance {
                total += advance;
                continue;
            }
            let tail_start = run.range.start.max(range.start);
            let tail = match tails.get_mut(index) {
                None => None,
                Some(slot) => {
                    if slot
                        .as_ref()
                        .is_none_or(|shaping| shaping.range.start != tail_start)
                    {
                        let tail_range = tail_start..run.range.end;
                        let shaped = self.shape(shaper, run, tail_range.clone())?;
                        *slot = Some(ItemShaping {
                            range: tail_range,
                            run: shaped,
                        });
                    }
                    slot.as_ref()
                }
            };

            total += match tail {
                Some(shaping) => match slice_width(&shaping.run, &shaping.range, &overlap) {
                    Some(width) => width,
                    None => self.shape(shaper, run, overlap)?.advance_in_points(),
                },
                None => self.shape(shaper, run, overlap)?.advance_in_points(),
            };
        }
        Ok(total)
    }

    /// The width of `range` in points, shaped fresh. Used for the hanging tail, which is measured
    /// once per line and never probed.
    fn width_of(&self, shaper: &mut Shaper, range: Range<usize>) -> Result<f64, FontError> {
        let mut total = 0.0_f64;
        for run in self.runs {
            let Some(overlap) = intersect(&run.range, &range) else {
                continue;
            };
            let shaped = self.shape(shaper, run, overlap)?;
            total += Self::advance_of(run, &shaped);
        }
        Ok(total)
    }

    /// Shape one stretch of one item.
    ///
    /// A run with a [`TextRun::advance`] is **not shaped**: it is an atomic inline box whose glyphs
    /// are not in any font, and shaping its `U+FFFC` placeholder would put a `.notdef` box on the
    /// line that a painter would draw. It shapes the empty string instead, so the segment carries no
    /// glyphs and its width comes from [`Self::advance_of`].
    fn shape(
        &self,
        shaper: &mut Shaper,
        run: &TextRun<'_>,
        range: Range<usize>,
    ) -> Result<ShapedRun, FontError> {
        let slice = if run.advance.is_some() {
            ""
        } else {
            self.text.get(range).unwrap_or("")
        };
        shaper.shape(run.face, &run.shaping_request(slice))
    }

    /// How wide `shaped` is: the run's own fixed advance when it declares one, and the shaper's
    /// answer otherwise.
    ///
    /// One function rather than three call sites doing the same `match`, because the three are the
    /// fitting probe, the hanging tail and the final composition — and a fixed advance that reached
    /// two of them would produce a line that was measured one way and drawn another.
    fn advance_of(run: &TextRun<'_>, shaped: &ShapedRun) -> f64 {
        run.advance.unwrap_or_else(|| shaped.advance_in_points())
    }

    /// Shape every item of `line` and put the results in visual order.
    fn compose(&self, shaper: &mut Shaper, line: Range<usize>) -> Result<ComposedLine, FontError> {
        // Shape in logical order. `logical_runs` is not consulted for the *shaping* order — the
        // items already are in logical order and each carries its own resolved direction — but the
        // visual arrangement below is exactly `visual_runs`, which is the half that matters.
        let mut shaped: Vec<ComposedSegment> = Vec::new();
        for run in self.runs {
            let Some(overlap) = intersect(&run.range, &line) else {
                continue;
            };
            let piece = self.shape(shaper, run, overlap.clone())?;
            let width = Self::advance_of(run, &piece);
            shaped.push(ComposedSegment {
                range: overlap,
                face: run.face_id,
                direction: run.direction,
                level: run.level,
                run: piece,
                width_in_points: width,
            });
        }

        let segments = self.in_visual_order(line.clone(), shaped);
        let width_in_points = segments
            .iter()
            .map(|segment| segment.width_in_points)
            .sum::<f64>();
        let (ascent_in_points, descent_in_points) = self.vertical_metrics(&line);

        Ok(ComposedLine {
            range: line,
            hanging: 0..0,
            kind: LineBreakKind::EndOfText,
            segments,
            width_in_points,
            hanging_width_in_points: 0.0,
            ascent_in_points,
            descent_in_points,
            hyphenated: false,
            measure_probes: 0,
        })
    }

    /// Rearrange logically ordered segments into the order they are drawn — UAX #9 rule L2, applied
    /// to whole segments.
    ///
    /// [`BidiAnalysis::visual_runs`] gives the order of the *level runs*; a level run split by a
    /// font change contains several segments, and inside a right-to-left one those are reversed
    /// relative to each other, because the logically last is the visually leftmost.
    fn in_visual_order(
        &self,
        line: Range<usize>,
        mut logical: Vec<ComposedSegment>,
    ) -> Vec<ComposedSegment> {
        let level_runs = self.bidi.visual_runs(line);
        if level_runs.len() <= 1
            && !level_runs
                .iter()
                .any(|run| run.direction().is_right_to_left())
        {
            return logical;
        }

        let mut visual: Vec<ComposedSegment> = Vec::with_capacity(logical.len());
        for level_run in &level_runs {
            let mut inside: Vec<ComposedSegment> = Vec::new();
            logical.retain(|segment| {
                if intersect(&segment.range, &level_run.range).is_some() {
                    inside.push(segment.clone());
                    false
                } else {
                    true
                }
            });
            if level_run.direction().is_right_to_left() {
                inside.reverse();
            }
            visual.append(&mut inside);
        }
        // Anything the level runs did not claim — which can only happen if `runs` reached outside
        // the resolved text — keeps its logical position at the end rather than being dropped.
        visual.append(&mut logical);
        visual
    }

    /// The tallest ascent and deepest descent of the items on `line`.
    ///
    /// From the faces' `hhea` ascent and descent, which is the metric Word takes a line's height
    /// from. A paragraph's declared line spacing multiplies or replaces this, and that is the box
    /// model's — this is the floor below which a line cannot be drawn without clipping.
    fn vertical_metrics(&self, line: &Range<usize>) -> (f64, f64) {
        let mut ascent = 0.0_f64;
        let mut descent = 0.0_f64;
        for run in self.runs {
            if intersect(&run.range, line).is_none() {
                continue;
            }
            let metrics = run.face.metrics();
            let em = f64::from(metrics.units_per_em.max(1));
            let size = run.size.in_points();
            ascent = ascent.max(f64::from(metrics.ascender) / em * size);
            // A descender is negative in the face's own tables and a descent is a positive depth.
            descent = descent.max(f64::from(-metrics.descender) / em * size);
        }
        (ascent, descent)
    }

    fn empty_line(&self, at: usize) -> ComposedLine {
        ComposedLine {
            range: at..at,
            hanging: at..at,
            kind: LineBreakKind::EndOfText,
            segments: Vec::new(),
            width_in_points: 0.0,
            hanging_width_in_points: 0.0,
            ascent_in_points: 0.0,
            descent_in_points: 0.0,
            hyphenated: false,
            measure_probes: 0,
        }
    }
}

/// One item's tail, shaped once and measured many times.
#[derive(Clone, Debug)]
struct ItemShaping {
    range: Range<usize>,
    run: ShapedRun,
}

/// The width of `sub` in points, taken from an already-shaped `run` — or `None` when the shaper
/// says a boundary of `sub` is unsafe to break at.
///
/// `covering` is the byte range of the paragraph that `run` was shaped from, so `covering.start` is
/// what cluster 0 means and `covering.len()` is one past the last valid boundary.
///
/// This is the function [`ShapedGlyph::unsafe_to_break`] exists for, and the `None` is the whole of
/// its meaning: at such a boundary the glyphs on either side are not the glyphs the two halves would
/// produce on their own, so no arithmetic over this run's advances is the answer and the caller must
/// shape the halves.
///
/// A boundary is safe when a glyph starts exactly there — an offset inside a ligature's cluster has
/// no glyph of its own and can never be sliced at — **and** that glyph is not flagged.
#[must_use]
pub fn slice_width(run: &ShapedRun, covering: &Range<usize>, sub: &Range<usize>) -> Option<f64> {
    let glyphs = run.glyphs();
    let text_len = covering.end.checked_sub(covering.start)?;
    let start = sub.start.checked_sub(covering.start)?;
    let end = sub.end.checked_sub(covering.start)?;
    if start > end || end > text_len {
        return None;
    }

    let (first, last) = if run.direction().is_right_to_left() {
        // Glyphs are in visual order, so clusters descend and a logical prefix is a glyph suffix.
        (
            right_to_left_split(glyphs, end, text_len)?,
            right_to_left_split(glyphs, start, text_len)?,
        )
    } else {
        (
            left_to_right_split(glyphs, start, text_len)?,
            left_to_right_split(glyphs, end, text_len)?,
        )
    };

    let slice = glyphs.get(first..last)?;
    let mut font_units = 0_i64;
    for glyph in slice {
        font_units += i64::from(glyph.x_advance);
    }
    let advance = AdvanceWidth {
        // A line a million ems wide saturates rather than wrapping; it will not fit any measure,
        // which is the right answer for it.
        font_units: i32::try_from(font_units).unwrap_or(i32::MAX),
        units_per_em: run.units_per_em(),
    };
    Some(advance.at_size(run.size().in_points()))
}

/// The glyph index at which a left-to-right run splits at byte `offset`, or `None` when it may not
/// be split there.
fn left_to_right_split(glyphs: &[ShapedGlyph], offset: usize, text_len: usize) -> Option<usize> {
    if offset == 0 {
        return Some(0);
    }
    if offset >= text_len {
        return Some(glyphs.len());
    }
    let offset = u32::try_from(offset).ok()?;
    let split = glyphs
        .iter()
        .take_while(|glyph| glyph.cluster < offset)
        .count();
    let boundary = glyphs.get(split)?;
    // A glyph must begin exactly here — otherwise `offset` is inside a cluster, which is what a
    // ligature makes of two characters — and the shaper must not have flagged it.
    if boundary.cluster != offset || boundary.unsafe_to_break {
        return None;
    }
    Some(split)
}

/// The same for a right-to-left run, whose glyphs are in visual order and whose clusters therefore
/// descend: a logical prefix `[0, offset)` is the glyph **suffix** `[split, len)`.
fn right_to_left_split(glyphs: &[ShapedGlyph], offset: usize, text_len: usize) -> Option<usize> {
    if offset == 0 {
        return Some(glyphs.len());
    }
    if offset >= text_len {
        return Some(0);
    }
    let offset = u32::try_from(offset).ok()?;
    let split = glyphs
        .iter()
        .take_while(|glyph| glyph.cluster >= offset)
        .count();
    // The glyph that begins the logically later part sits immediately before the split.
    let boundary = glyphs.get(split.checked_sub(1)?)?;
    if boundary.cluster != offset || boundary.unsafe_to_break {
        return None;
    }
    Some(split)
}

/// The overlap of two byte ranges, or `None` when they do not meet.
fn intersect(left: &Range<usize>, right: &Range<usize>) -> Option<Range<usize>> {
    let start = left.start.max(right.start);
    let end = left.end.min(right.end);
    if start >= end {
        return None;
    }
    Some(start..end)
}
