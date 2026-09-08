//! Laying out one shape's text: insets, columns, indents, bullets, line spacing, anchoring and the
//! autofit search that ties them together.
//!
//! # The order the pieces come in
//!
//! ```text
//! shape rectangle
//!   ├─ less the four insets            → the content box
//!   ├─ divided by `numCol`/`spcCol`    → the columns
//!   ├─ each paragraph broken to a column's measure, less its own margins and indent
//!   ├─ the lines poured into the columns until each is full
//!   ├─ each column's block anchored inside the content box
//!   └─ each line aligned inside its own measure
//! ```
//!
//! Autofit wraps the whole of that: the pour is attempted at a font scale, and if the last column
//! overflowed, the next scale down is attempted. That is why the composition is a function of a
//! scale rather than a mutation — a search needs to be able to throw an attempt away.
//!
//! # What is a reading of PowerPoint and what is a reading of the specification
//!
//! ECMA-376 says what the attributes *are* and is nearly silent on what a renderer does with them.
//! Every place below where behaviour was chosen rather than read is marked `GUESS:` at the site, so
//! the Windows sitting has a list rather than a diff. The four that matter most:
//!
//! * how `anchor="just"` and `anchor="dist"` distribute their slack;
//! * whether `lnSpcReduction` applies to a `a:spcPts` line spacing as well as to a `a:spcPct` one;
//! * where a `spAutoFit` shape grows from;
//! * and the autofit ladder itself, which lives in [`crate::autofit`].

use mjx_dml::{
    Fraction, ParagraphPropertiesSpec, TextAlignment, TextAnchoring, TextBodyPropertiesSpec,
    TextDirection, TextSpacing, TextWrapping,
};
use mjx_layout::{ComposedLine, LayoutPoint, LayoutRect, Transform};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_text::{BidiAnalysis, FaceId, LineBreakOptions, ShapedRun, TextDirection as RunDirection};

use crate::autofit::{self, AutofitOutcome, AutofitPolicy};
use crate::bullet::{self, AutoNumberCounters};
use crate::deck::TextBody;
use crate::error::SlideLayoutError;
use crate::text::{self, RunStyle, TabStops, TextEngine};

/// The measure a body with `wrap="none"` is broken to, in points.
///
/// Not infinity: [`LineComposer`](mjx_layout::LineComposer) fits by comparing widths, and an
/// infinite measure would make every comparison meaningless rather than trivially true. A million
/// points is eleven miles of text, which no line reaches and no arithmetic overflows.
pub const UNWRAPPED_MEASURE_POINTS: f64 = 1_000_000.0;

/// The largest column count a body is laid out with.
///
/// `ST_TextColumnCount` is `1..=16`; a file may state more, and dividing a shape into sixty thousand
/// columns would produce sixty thousand empty rectangles. The schema's own ceiling is the clamp.
pub const MAXIMUM_COLUMNS: u16 = 16;

/// One shaped piece of a line, already positioned.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedPiece {
    /// Which of the paragraph's runs it came from.
    pub run: usize,
    /// The bytes of that run's own text it covers.
    pub range_in_run: std::ops::Range<usize>,
    /// Which face draws it.
    pub face: FaceId,
    /// The glyphs.
    pub shaped: ShapedRun,
    /// Where the pen starts — on the baseline, at the piece's leading edge in visual order.
    pub origin: LayoutPoint,
    /// How wide it is.
    pub width: Emu,
    /// Which way it is written.
    pub direction: RunDirection,
    /// The UAX #9 level it was resolved at.
    pub level: mjx_text::BidiLevel,
}

/// A paragraph's marker, positioned.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedMarker {
    /// Which face draws it.
    pub face: FaceId,
    /// The glyphs.
    pub shaped: ShapedRun,
    /// Where the pen starts.
    pub origin: LayoutPoint,
    /// How wide it is.
    pub width: Emu,
}

/// One line, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedLine {
    /// Which paragraph it belongs to.
    pub paragraph: usize,
    /// The bytes of that paragraph's text it covers.
    pub range: std::ops::Range<usize>,
    /// Its box, in slide coordinates.
    pub rect: LayoutRect,
    /// Where its baseline sits, measured down from the top of its box.
    pub baseline: Emu,
    /// How far above the baseline it reaches.
    pub ascent: Emu,
    /// How far below it reaches.
    pub descent: Emu,
    /// Which way it reads as a whole.
    pub base_direction: RunDirection,
    /// The trailing width allowed to hang past the measure.
    pub hanging_width: Emu,
    /// Its glyph pieces, in visual order.
    pub pieces: Vec<PlacedPiece>,
    /// Its paragraph's marker, when it is the paragraph's first line and the paragraph has one.
    pub marker: Option<PlacedMarker>,
}

/// One column of a text body, with the lines that landed in it.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedColumn {
    /// The column's own rectangle inside the content box.
    pub rect: LayoutRect,
    /// The lines in it, in reading order.
    pub lines: Vec<PlacedLine>,
}

/// A whole text body, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedBody {
    /// The content box — the shape's rectangle less its four insets.
    pub content: LayoutRect,
    /// Its columns, left to right (or right to left when `@rtlCol` says so).
    pub columns: Vec<PlacedColumn>,
    /// What autofit did.
    pub outcome: AutofitOutcome,
    /// Whether the text still did not fit after autofit had done what it could.
    pub overflows: bool,
}

impl PlacedBody {
    /// Every line of every column.
    pub fn lines(&self) -> impl Iterator<Item = &PlacedLine> {
        self.columns.iter().flat_map(|column| column.lines.iter())
    }

    /// How many lines there are in total.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.columns.iter().map(|column| column.lines.len()).sum()
    }
}

/// How a vertical text body is laid out: in which rectangle, and turned by how much afterwards.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct VerticalLayout {
    /// The rectangle the body is laid out in — the shape's own for horizontal text, and the shape's
    /// **transposed** about its centre for vertical text.
    pub rect: LayoutRect,
    /// The turn applied to the finished fragments, or `None` for horizontal text.
    pub turn: Option<Transform>,
}

/// Where a body's text is laid out and how it is turned, for `a:bodyPr@vert`.
///
/// # Why transpose and turn rather than stack lines downward
///
/// `vert` and `vert270` are, in PowerPoint, a **rotation of the whole text block** rather than a
/// different line-stacking discipline: the glyphs keep their own orientation relative to the line,
/// the line runs down the shape, and a reader tilts their head. Laying the text out in the shape's
/// transposed rectangle and turning the result reproduces exactly that, and — the part that matters
/// — it makes every *line-breaking* decision against the measure the text really has, which is the
/// shape's height. A renderer that broke lines to the shape's width and then turned them would wrap
/// in the wrong places, which is the visible half of getting this wrong.
///
/// # What is deliberately not implemented
///
/// `eaVert`, `mongolianVert`, `wordArtVert` and `wordArtVertRtl` are **not** rotations. They stack
/// upright glyphs down a column, which needs per-glyph vertical metrics, vertical substitution
/// features (`vert`/`vrt2`) and a line-stacking direction the font engine has to be told about.
/// That is a font-engine question as much as a layout one, and it is not this child's. They are laid
/// out horizontally, which is legible and wrong, rather than turned, which would be illegible and
/// wrong.
///
/// # ⚠ A seam finding, not a preference
///
/// [`Constraints::writing_mode`](mjx_layout::Constraints::writing_mode) exists and would be the
/// right place for this — except that it is on `Constraints`, which is what the **caller** decides
/// about the page. `a:bodyPr@vert` is a property of one *shape*, and two shapes on one slide may
/// disagree, so the contract as it stands cannot carry the document's answer. That is reported to
/// R05 rather than worked around: the turn below is this crate's own, and the day the seam grows a
/// per-fragment writing mode it should move there.
#[must_use]
pub fn vertical_layout(shape: LayoutRect, geometry: &TextBodyPropertiesSpec) -> VerticalLayout {
    let vertical = geometry
        .vertical()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_VERTICAL);
    let degrees = match vertical {
        // `vert` reads top to bottom with the glyphs turned a quarter turn clockwise.
        TextDirection::Vertical => 90.0,
        // `vert270` reads bottom to top — the same turn the other way.
        TextDirection::Vertical270 => -90.0,
        _ => {
            return VerticalLayout {
                rect: shape,
                turn: None,
            }
        }
    };
    let centre = LayoutPoint::new(
        shape.left + shape.width().divided_by(2),
        shape.top + shape.height().divided_by(2),
    );
    let half_width = shape.height().divided_by(2);
    let half_height = shape.width().divided_by(2);
    VerticalLayout {
        rect: LayoutRect::from_edges(
            centre.x - half_width,
            centre.y - half_height,
            centre.x + half_width,
            centre.y + half_height,
        ),
        turn: Some(Transform::rotation_about(
            Angle::from_degrees(degrees),
            centre,
        )),
    }
}

/// The content box of a shape whose rectangle is `shape` and whose body geometry is `geometry`.
///
/// Never wider or taller than the shape: insets that overlap collapse the box to nothing rather than
/// inverting it, because an inverted content box would place text outside the shape it belongs to.
#[must_use]
pub fn content_box(shape: LayoutRect, geometry: &TextBodyPropertiesSpec) -> LayoutRect {
    let left = geometry
        .left_inset()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_LEFT_INSET);
    let top = geometry
        .top_inset()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_TOP_INSET);
    let right = geometry
        .right_inset()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_RIGHT_INSET);
    let bottom = geometry
        .bottom_inset()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_BOTTOM_INSET);
    LayoutRect {
        left: shape.left + left,
        top: shape.top + top,
        right: (shape.right - right).maximum(shape.left + left),
        bottom: (shape.bottom - bottom).maximum(shape.top + top),
    }
}

/// The column rectangles of a content box.
///
/// `@rtlCol` reverses which rectangle is column zero, not the rectangles themselves: a
/// right-to-left body's first column is the rightmost one.
#[must_use]
pub fn columns_of(content: LayoutRect, geometry: &TextBodyPropertiesSpec) -> Vec<LayoutRect> {
    let count = geometry
        .columns()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_COLUMNS)
        .clamp(1, MAXIMUM_COLUMNS);
    let gap = geometry
        .column_space()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_COLUMN_SPACE)
        .maximum(Emu::ZERO);
    let gaps = gap.times(i64::from(count) - 1);
    let usable = content.width() - gaps;
    if usable <= Emu::ZERO {
        return vec![content];
    }
    let width = usable.divided_by(i64::from(count));
    if width <= Emu::ZERO {
        return vec![content];
    }
    let mut rects: Vec<LayoutRect> = (0..count)
        .map(|index| {
            let left = content.left + (width + gap).times(i64::from(index));
            LayoutRect::from_edges(left, content.top, left + width, content.bottom)
        })
        .collect();
    if geometry.has_right_to_left_columns().unwrap_or(false) {
        rects.reverse();
    }
    rects
}

/// Lays out `body` inside `shape`.
///
/// # Errors
/// [`SlideLayoutError::Text`] when a face will not shape. Nothing else here fails: a malformed
/// paragraph produces a bad-looking body, never a refusal.
pub fn lay_out(
    engine: &mut TextEngine<'_>,
    shape: LayoutRect,
    body: &TextBody,
    policy: AutofitPolicy,
) -> Result<PlacedBody, SlideLayoutError> {
    let content = content_box(shape, &body.geometry);
    let columns = columns_of(content, &body.geometry);
    let autofit_choice = body.geometry.autofit();

    let stored = match policy {
        AutofitPolicy::Disabled => AutofitOutcome::unscaled(),
        AutofitPolicy::HonourOnly | AutofitPolicy::HonourAndRecompute => {
            autofit::stored_scale(autofit_choice).unwrap_or_else(AutofitOutcome::unscaled)
        }
    };

    let first = compose(engine, body, &columns, stored)?;
    let searches = policy == AutofitPolicy::HonourAndRecompute
        && autofit::shrinks_the_text(autofit_choice)
        && first.overflows;
    let mut placed = first;

    if searches {
        placed = search(engine, body, &columns, stored, placed)?;
    }

    if autofit::grows_the_shape(autofit_choice) {
        // GUESS: `a:spAutoFit` grows the shape downward from its stated top. PowerPoint writes the
        // grown `a:ext@cy` into the file, so a deck read from disk already fits and this only
        // matters once something above has edited the text — which is exactly when there is nothing
        // to compare against. The height is reported rather than applied so the caller decides.
        placed.outcome.grown_height = Some(required_height(&placed, &body.geometry, shape));
    }
    Ok(placed)
}

/// Searches the autofit ladder for the largest pair at which the body fits.
///
/// # The scale is a rung, never a product
///
/// A recomputed scale is **absolute**: it is one of the values in [`autofit::FONT_SCALES`], and a
/// stored scale only says where the search starts. Multiplying the stored scale by a rung — which is
/// the obvious thing to write, and what this did first — produces numbers PowerPoint never writes
/// (`0.85 × 0.70 = 0.595`), so every shape with a stored scale would then disagree with Office by
/// construction. `an_autofit_that_recomputes_lands_on_a_rung_of_the_ladder` is what holds that.
///
/// # One probe, then a bisection
///
/// The reductions are tried in order, and each is *rejected in one composition* — if the smallest
/// font does not fit even with that reduction, nothing at that reduction will. When one does fit,
/// [`autofit::largest_fitting_rung`] finds the largest font that still does in about four probes
/// instead of fourteen. See that function for why the bisection is sound.
fn search(
    engine: &mut TextEngine<'_>,
    body: &TextBody,
    columns: &[LayoutRect],
    stored: AutofitOutcome,
    fallback: PlacedBody,
) -> Result<PlacedBody, SlideLayoutError> {
    let Some(last) = autofit::FONT_SCALES.len().checked_sub(1) else {
        return Ok(fallback);
    };
    let from = autofit::rung_of(stored.font_scale);
    let mut tightest = fallback;

    for &reduction in autofit::LINE_SPACE_REDUCTIONS {
        // A stored reduction is PowerPoint's own last answer too, and the text overflowed at it, so
        // a smaller one cannot help.
        if reduction < stored.line_space_reduction {
            continue;
        }
        let smallest = compose_at(engine, body, columns, last, reduction)?;
        let fits = !smallest.overflows;
        tightest = smallest;
        if !fits {
            continue;
        }
        let mut probe = |rung: usize| -> Result<bool, SlideLayoutError> {
            Ok(!compose_at(engine, body, columns, rung, reduction)?.overflows)
        };
        let rung = autofit::largest_fitting_rung(from, last, &mut probe)?;
        if rung >= last {
            return Ok(tightest);
        }
        return compose_at(engine, body, columns, rung, reduction);
    }
    // Nothing on the ladder fits. The smallest pair is still the best answer there is: the text
    // overflows either way, and it overflows by less.
    Ok(tightest)
}

/// Composes the body at one rung of the ladder.
fn compose_at(
    engine: &mut TextEngine<'_>,
    body: &TextBody,
    columns: &[LayoutRect],
    rung: usize,
    reduction: f64,
) -> Result<PlacedBody, SlideLayoutError> {
    let font_scale = autofit::FONT_SCALES.get(rung).copied().unwrap_or(1.0);
    compose(
        engine,
        body,
        columns,
        AutofitOutcome {
            font_scale,
            line_space_reduction: reduction,
            recomputed: true,
            grown_height: None,
        },
    )
}

/// The shape height a `spAutoFit` body's text needs: its tallest column plus the vertical insets.
fn required_height(
    placed: &PlacedBody,
    geometry: &TextBodyPropertiesSpec,
    shape: LayoutRect,
) -> Emu {
    let top = geometry
        .top_inset()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_TOP_INSET);
    let bottom = geometry
        .bottom_inset()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_BOTTOM_INSET);
    let used = placed
        .columns
        .iter()
        .map(|column| {
            column
                .lines
                .iter()
                .fold(Emu::ZERO, |tallest, line| tallest.maximum(line.rect.bottom))
                - placed.content.top
        })
        .fold(Emu::ZERO, Emu::maximum);
    (used + top + bottom).maximum(shape.height().minimum(Emu::ZERO))
}

/// One attempt at composing the body at a given autofit outcome.
fn compose(
    engine: &mut TextEngine<'_>,
    body: &TextBody,
    columns: &[LayoutRect],
    outcome: AutofitOutcome,
) -> Result<PlacedBody, SlideLayoutError> {
    let content = columns
        .iter()
        .fold(LayoutRect::ZERO, |union, column| union.union(*column));
    let wrap = body
        .geometry
        .wrap()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_WRAP);
    let space_first_last = body
        .geometry
        .spaces_first_and_last_paragraph()
        .unwrap_or(false);

    let mut counters = AutoNumberCounters::new();
    let mut pending: Vec<PendingLine> = Vec::new();
    let mut pending_columns: Vec<Vec<PendingLine>> = vec![Vec::new(); columns.len().max(1)];

    // One flat stream of lines, measured against the first column's width. Every column of a text
    // body is the same width, so a line that fits column 0 fits column 3 — which is what makes a
    // single composition pass and a separate pour correct rather than an approximation.
    let measure_column = columns.first().copied().unwrap_or(content);
    for (index, paragraph) in body.paragraphs.iter().enumerate() {
        let composed = compose_paragraph(
            engine,
            paragraph,
            index,
            measure_column,
            wrap,
            outcome,
            &mut counters,
            index == 0,
            index + 1 == body.paragraphs.len(),
            space_first_last,
        )?;
        pending.extend(composed);
    }

    // Pour: fill each column to its height, then move on. The last column keeps whatever is left,
    // which is what makes overflow visible instead of silently dropping text.
    let mut column_index = 0_usize;
    let mut pen = content.top;
    for line in pending {
        let is_last_column = column_index + 1 >= pending_columns.len();
        let bottom = columns.get(column_index).copied().unwrap_or(content).bottom;
        let occupied = pen + line.space_before + line.height;
        let column_has_something = pending_columns
            .get(column_index)
            .is_some_and(|lines| !lines.is_empty());
        if !is_last_column && occupied > bottom && column_has_something {
            column_index += 1;
            pen = content.top;
        }
        if let Some(slot) = pending_columns.get_mut(column_index) {
            slot.push(line.clone());
        }
        pen += line.space_before + line.height + line.space_after;
    }

    let mut placed_columns = Vec::with_capacity(pending_columns.len());
    let mut overflows = false;
    for (index, lines) in pending_columns.into_iter().enumerate() {
        let rect = columns.get(index).copied().unwrap_or(content);
        let (placed, spilled) = place_column(rect, lines, &body.geometry, wrap);
        overflows |= spilled;
        placed_columns.push(PlacedColumn {
            rect,
            lines: placed,
        });
    }

    Ok(PlacedBody {
        content,
        columns: placed_columns,
        outcome,
        overflows,
    })
}

/// A line that has been composed but not yet given a position.
#[derive(Clone, PartialEq, Debug)]
struct PendingLine {
    paragraph: usize,
    range: std::ops::Range<usize>,
    /// The line's own height, after line spacing and any reduction.
    height: Emu,
    ascent: Emu,
    descent: Emu,
    space_before: Emu,
    space_after: Emu,
    base_direction: RunDirection,
    hanging_width: Emu,
    alignment: TextAlignment,
    /// Where the line's text starts, relative to the column's left edge.
    text_offset: Emu,
    /// How wide the line was allowed to be.
    measure: Emu,
    /// The line's own drawn width.
    width: Emu,
    /// Where a tab in this line advances to, which is its paragraph's own resolution.
    tab_stops: TabStops,
    segments: Vec<PendingPiece>,
    marker: Option<PendingMarker>,
}

#[derive(Clone, PartialEq, Debug)]
struct PendingPiece {
    run: usize,
    range_in_run: std::ops::Range<usize>,
    face: FaceId,
    shaped: ShapedRun,
    width: Emu,
    direction: RunDirection,
    level: mjx_text::BidiLevel,
    /// Whether the piece is a tab, which advances the pen instead of drawing.
    is_tab: bool,
}

#[derive(Clone, PartialEq, Debug)]
struct PendingMarker {
    face: FaceId,
    shaped: ShapedRun,
    width: Emu,
    /// Where the marker starts, relative to the column's left edge.
    offset: Emu,
}

/// Composes every line of one paragraph, at the scale `outcome` states.
#[allow(clippy::too_many_arguments)]
fn compose_paragraph(
    engine: &mut TextEngine<'_>,
    paragraph: &crate::deck::Paragraph,
    index: usize,
    column: LayoutRect,
    wrap: TextWrapping,
    outcome: AutofitOutcome,
    counters: &mut AutoNumberCounters,
    is_first: bool,
    is_last: bool,
    space_first_last: bool,
) -> Result<Vec<PendingLine>, SlideLayoutError> {
    let properties = &paragraph.properties;
    let level = properties.level().map_or(0, |level| level.value() as usize);
    let left_margin = points_to_emu(properties.left_margin_points());
    let right_margin = points_to_emu(properties.right_margin_points());
    let indent = points_to_emu(properties.indent_points());
    let alignment = properties.alignment().unwrap_or(TextAlignment::Left);

    let styles: Vec<RunStyle> = paragraph
        .runs
        .iter()
        .map(|run| {
            RunStyle::from_properties(run.range.clone(), &run.properties).scaled(outcome.font_scale)
        })
        .collect();

    // The style the paragraph's marker follows when it says "whatever the text uses".
    let leading_style = styles.first().cloned().unwrap_or_else(|| {
        RunStyle::from_properties(0..0, &Default::default()).scaled(outcome.font_scale)
    });
    let marker = bullet::marker_for(properties, level, &leading_style, counters);

    let marker_piece = match &marker {
        None => None,
        Some(marker) => shape_marker(engine, marker)?,
    };
    let marker_width = marker_piece.as_ref().map_or(Emu::ZERO, |piece| piece.1);

    // Where the first line's text starts, and where every later line's does. A negative `indent`
    // hangs the marker out to the left of the text; a marker wider than the hang pushes the text
    // along, which is what stops a long automatic number from drawing over its own paragraph.
    let hang_start = left_margin + indent;
    let first_text_offset = if marker_piece.is_some() {
        (hang_start + marker_width).maximum(left_margin)
    } else {
        hang_start
    };
    let later_text_offset = left_margin;

    let usable = column.width() - right_margin;
    let measure_of_line = |line: usize| -> Emu {
        let offset = if line == 0 {
            first_text_offset
        } else {
            later_text_offset
        };
        (usable - offset).maximum(Emu::ZERO)
    };

    let space_before = if is_first && !space_first_last {
        Emu::ZERO
    } else {
        spacing_to_emu(properties.space_before(), &leading_style)
    };
    let space_after = if is_last && !space_first_last {
        Emu::ZERO
    } else {
        spacing_to_emu(properties.space_after(), &leading_style)
    };

    let direction = text::paragraph_direction(properties.is_right_to_left());
    let bidi = BidiAnalysis::resolve(&paragraph.text, direction);
    let tab_stops = TabStops::new(
        properties.tab_stops().to_vec(),
        properties
            .default_tab_size_points()
            .map_or(TabStops::DEFAULT_SIZE, Emu::from_points),
    );

    // An empty paragraph still occupies a line, so that a reader can put a caret in it.
    if paragraph.text.is_empty() {
        let height = empty_height(engine, &leading_style, properties, outcome)?;
        return Ok(vec![PendingLine {
            paragraph: index,
            range: 0..0,
            height: height.0,
            ascent: height.1,
            descent: height.0 - height.1,
            space_before,
            space_after,
            base_direction: base_direction_of(&bidi),
            hanging_width: Emu::ZERO,
            alignment,
            text_offset: first_text_offset,
            measure: measure_of_line(0),
            width: Emu::ZERO,
            tab_stops,
            segments: Vec::new(),
            marker: marker_piece.map(|(piece, width)| PendingMarker {
                face: piece.0,
                shaped: piece.1,
                width,
                offset: hang_start,
            }),
        }]);
    }

    let items = text::itemise_paragraph(engine, &paragraph.text, &styles, &bidi)?;
    let runs = text::composer_runs(engine.rasteriser, engine.features, &items);
    let options = LineBreakOptions::default();
    let composed = text::compose_paragraph(
        engine.shaper,
        &paragraph.text,
        &runs,
        &bidi,
        options,
        |line| match wrap {
            TextWrapping::None => UNWRAPPED_MEASURE_POINTS,
            TextWrapping::Square => measure_of_line(line).points(),
        },
    )?;

    let factor = line_spacing_factor(properties, outcome);
    let fixed = fixed_line_height(properties);
    let mut lines = Vec::with_capacity(composed.len());
    for (number, line) in composed.into_iter().enumerate() {
        let ascent = Emu::from_points(line.ascent_in_points);
        let descent = Emu::from_points(line.descent_in_points);
        let natural = Emu::from_points(line.height_in_points());
        let height = match fixed {
            Some(height) => height,
            None => Emu::from_emu((natural.emu() as f64 * factor).round() as i64),
        };
        let segments = pending_pieces(&items, &paragraph.runs, &line);
        lines.push(PendingLine {
            paragraph: index,
            range: line.range.clone(),
            height,
            ascent,
            descent,
            space_before: if number == 0 { space_before } else { Emu::ZERO },
            space_after: Emu::ZERO,
            base_direction: base_direction_of(&bidi),
            hanging_width: Emu::from_points(line.hanging_width_in_points),
            alignment,
            text_offset: if number == 0 {
                first_text_offset
            } else {
                later_text_offset
            },
            measure: measure_of_line(number),
            width: Emu::from_points(line.width_in_points),
            tab_stops: tab_stops.clone(),
            segments,
            marker: if number == 0 {
                marker_piece.clone().map(|(piece, width)| PendingMarker {
                    face: piece.0,
                    shaped: piece.1,
                    width,
                    offset: hang_start,
                })
            } else {
                None
            },
        });
    }
    if let Some(last) = lines.last_mut() {
        last.space_after = space_after;
    }
    Ok(lines)
}

/// The pieces one composed line becomes, mapped back onto the paragraph's own runs.
fn pending_pieces(
    items: &[text::StyledItem],
    runs: &[crate::deck::Run],
    line: &ComposedLine,
) -> Vec<PendingPiece> {
    line.segments
        .iter()
        .filter_map(|segment| {
            let index = text::item_of(items, &segment.range)?;
            let styled = items.get(index)?;
            let run = runs.get(styled.run)?;
            Some(PendingPiece {
                run: styled.run,
                range_in_run: segment.range.start.saturating_sub(run.range.start)
                    ..segment.range.end.saturating_sub(run.range.start),
                face: segment.face,
                shaped: segment.run.clone(),
                width: Emu::from_points(segment.width_in_points),
                direction: segment.direction,
                level: segment.level,
                is_tab: styled.is_tab,
            })
        })
        .collect()
}

/// Shapes a marker into one run, or `None` when no face will draw it.
type ShapedMarker = ((FaceId, ShapedRun), Emu);

fn shape_marker(
    engine: &mut TextEngine<'_>,
    marker: &bullet::Marker,
) -> Result<Option<ShapedMarker>, SlideLayoutError> {
    let Some((face, face_id)) = text::resolve_face(engine.fonts, engine.rasteriser, &marker.style)?
    else {
        return Ok(None);
    };
    let bidi = BidiAnalysis::resolve(&marker.text, mjx_text::ParagraphDirection::LeftToRight);
    let items = text::itemise_paragraph(
        engine,
        &marker.text,
        std::slice::from_ref(&RunStyle {
            range: 0..marker.text.len(),
            ..marker.style.clone()
        }),
        &bidi,
    )?;
    let runs = text::composer_runs(engine.rasteriser, engine.features, &items);
    let composed = text::compose_paragraph(
        engine.shaper,
        &marker.text,
        &runs,
        &bidi,
        LineBreakOptions::default(),
        |_| UNWRAPPED_MEASURE_POINTS,
    )?;
    let Some(line) = composed.into_iter().next() else {
        // No face resolved for the marker's characters. The indents still apply; nothing is drawn.
        let _ = face;
        return Ok(None);
    };
    let width = Emu::from_points(line.width_in_points);
    let Some(segment) = line.segments.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(((face_id, segment.run), width)))
}

/// The height and ascent of an empty paragraph's single line.
fn empty_height(
    engine: &mut TextEngine<'_>,
    style: &RunStyle,
    properties: &ParagraphPropertiesSpec,
    outcome: AutofitOutcome,
) -> Result<(Emu, Emu), SlideLayoutError> {
    let Some((face, _)) = text::resolve_face(engine.fonts, engine.rasteriser, style)? else {
        return Ok((Emu::ZERO, Emu::ZERO));
    };
    let natural = text::empty_line_height(&face, style.size);
    let ascent = text::empty_line_ascent(&face, style.size);
    let height = match fixed_line_height(properties) {
        Some(height) => height,
        None => {
            let factor = line_spacing_factor(properties, outcome);
            Emu::from_emu((natural.emu() as f64 * factor).round() as i64)
        }
    };
    Ok((height, ascent))
}

/// The proportion a `a:spcPct` line spacing multiplies a line's natural height by, with the autofit
/// reduction taken off.
///
/// GUESS: `lnSpcReduction` is subtracted from the percentage rather than applied to the result,
/// which is what makes a 100 % spacing with a 20 % reduction come out at 80 % rather than at 80 % of
/// something already reduced. The two agree at 100 % spacing — which is nearly every paragraph — and
/// differ at 150 %.
fn line_spacing_factor(properties: &ParagraphPropertiesSpec, outcome: AutofitOutcome) -> f64 {
    let stated = match properties.line_spacing() {
        Some(TextSpacing::Percentage(fraction)) => finite(fraction, 1.0),
        // A fixed spacing is not a factor; `fixed_line_height` handles it.
        Some(TextSpacing::Points(_)) | None => 1.0,
    };
    (stated - outcome.line_space_reduction).max(0.0)
}

/// The fixed height a `a:spcPts` line spacing states, or `None` for a proportional one.
///
/// GUESS: `lnSpcReduction` is **not** applied to a fixed spacing. A point value is an author saying
/// *exactly this many points*, and PowerPoint's own autofit reduces the percentage rather than the
/// number — but this has not been checked against Office.
fn fixed_line_height(properties: &ParagraphPropertiesSpec) -> Option<Emu> {
    match properties.line_spacing() {
        Some(TextSpacing::Points(points)) => {
            let value = Emu::from_points(points.points());
            (value > Emu::ZERO).then_some(value)
        }
        _ => None,
    }
}

/// The space a `a:spcBef` / `a:spcAft` states, in EMU.
fn spacing_to_emu(spacing: Option<TextSpacing>, style: &RunStyle) -> Emu {
    match spacing {
        None => Emu::ZERO,
        Some(TextSpacing::Points(points)) => Emu::from_points(points.points().max(0.0)),
        // GUESS: a percentage space-before is a proportion of the font size rather than of the
        // measured line height. PowerPoint's own UI writes these as percentages of the text size,
        // and taking it from the line height would make the same document space differently
        // depending on which face was substituted.
        Some(TextSpacing::Percentage(fraction)) => {
            Emu::from_points(style.size.in_points() * finite(fraction, 0.0).max(0.0))
        }
    }
}

fn finite(fraction: Fraction, absent: f64) -> f64 {
    let ratio = fraction.ratio();
    if ratio.is_finite() {
        ratio
    } else {
        absent
    }
}

fn points_to_emu(points: Option<f64>) -> Emu {
    points.map_or(Emu::ZERO, Emu::from_points)
}

fn base_direction_of(bidi: &BidiAnalysis) -> RunDirection {
    bidi.base_direction()
}

/// Places one column's lines: anchors the block vertically, aligns each line horizontally, and lays
/// the glyph pieces out along each baseline.
///
/// Returns the lines and whether the block was taller than the column.
fn place_column(
    column: LayoutRect,
    lines: Vec<PendingLine>,
    geometry: &TextBodyPropertiesSpec,
    wrap: TextWrapping,
) -> (Vec<PlacedLine>, bool) {
    if lines.is_empty() {
        return (Vec::new(), false);
    }

    let used: Emu = lines.iter().fold(Emu::ZERO, |total, line| {
        total + line.space_before + line.height + line.space_after
    });
    let slack = column.height() - used;
    let overflows = slack < Emu::ZERO;
    let anchor = geometry
        .anchor()
        .unwrap_or(TextBodyPropertiesSpec::DEFAULT_ANCHOR);

    // Where the block starts, and how much extra room goes between each pair of lines.
    let (mut pen, leading) = match anchor {
        TextAnchoring::Top => (column.top, Emu::ZERO),
        TextAnchoring::Center => (
            column.top + slack.divided_by(2).maximum(Emu::ZERO),
            Emu::ZERO,
        ),
        TextAnchoring::Bottom => (column.top + slack.maximum(Emu::ZERO), Emu::ZERO),
        // GUESS: `just` spreads the slack between the lines and `dist` spreads it around them as
        // well, which is how a justified and a distributed *paragraph* differ and the closest
        // reading of what the two anchors mean vertically. Neither is written down.
        TextAnchoring::Justified => (column.top, spread(slack, lines.len().saturating_sub(1))),
        TextAnchoring::Distributed => {
            let gaps = lines.len().saturating_add(1);
            let each = spread(slack, gaps);
            (column.top + each, each)
        }
    };

    // `@anchorCtr` centres the *block* horizontally: every line moves by the same amount, which is
    // what makes it different from centring each line.
    let block_shift = if geometry.is_anchor_centered().unwrap_or(false) {
        let widest = lines.iter().fold(Emu::ZERO, |widest, line| {
            widest.maximum(line.text_offset + line.width)
        });
        (column.width() - widest).divided_by(2).maximum(Emu::ZERO)
    } else {
        Emu::ZERO
    };

    let mut placed = Vec::with_capacity(lines.len());
    for line in lines {
        pen += line.space_before;
        let top = pen;
        let bottom = top + line.height;
        // The baseline sits an ascent below the top of the line's own box, with the leading the
        // spacing added shared above it. A line whose height was reduced below its ascent draws its
        // baseline at the bottom rather than outside the box.
        let extra = (line.height - (line.ascent + line.descent)).maximum(Emu::ZERO);
        let baseline = (extra.divided_by(2) + line.ascent).minimum(line.height);

        let aligned = align_offset(&line, wrap) + block_shift;
        let left = column.left + aligned;
        let mut pieces = Vec::with_capacity(line.segments.len());
        let mut cursor = left;
        for piece in &line.segments {
            if piece.is_tab {
                // A tab advances to its stop rather than by its glyph's advance, and draws nothing.
                // The stops are measured from the column's own left edge, which is where a
                // paragraph's `a:tabLst` positions are measured from.
                let from = cursor - column.left;
                cursor = column.left + line.tab_stops.next_after(from);
                continue;
            }
            pieces.push(PlacedPiece {
                run: piece.run,
                range_in_run: piece.range_in_run.clone(),
                face: piece.face,
                shaped: piece.shaped.clone(),
                origin: LayoutPoint::new(cursor, top + baseline),
                width: piece.width,
                direction: piece.direction,
                level: piece.level,
            });
            cursor += piece.width;
        }

        let marker = line.marker.as_ref().map(|marker| PlacedMarker {
            face: marker.face,
            shaped: marker.shaped.clone(),
            origin: LayoutPoint::new(column.left + marker.offset + block_shift, top + baseline),
            width: marker.width,
        });

        placed.push(PlacedLine {
            paragraph: line.paragraph,
            range: line.range.clone(),
            rect: LayoutRect::from_edges(left, top, (left + line.width).maximum(left), bottom),
            baseline,
            ascent: line.ascent,
            descent: line.descent,
            base_direction: line.base_direction,
            hanging_width: line.hanging_width,
            pieces,
            marker,
        });
        pen = bottom + line.space_after + leading;
    }
    (placed, overflows)
}

/// How far right of the column's left edge a line's text starts, after alignment.
fn align_offset(line: &PendingLine, wrap: TextWrapping) -> Emu {
    let slack = (line.measure - line.width).maximum(Emu::ZERO);
    // A body that does not wrap has no measure to align against — every line is as long as it is —
    // so alignment other than the leading edge would move text by an arbitrary amount.
    if wrap == TextWrapping::None {
        return line.text_offset;
    }
    match line.alignment {
        TextAlignment::Center => line.text_offset + slack.divided_by(2),
        TextAlignment::Right => line.text_offset + slack,
        // GUESS: `just`, `justLow`, `dist` and `thaiDist` are laid out flush left. Real
        // justification expands the spaces inside a line, which needs a second pass over the shaped
        // glyphs that this box model does not make; a flush-left line is where the text starts
        // either way, so nothing moves *backwards* when justification arrives.
        TextAlignment::Left
        | TextAlignment::Justified
        | TextAlignment::JustifiedLow
        | TextAlignment::Distributed
        | TextAlignment::ThaiDistributed => line.text_offset,
    }
}

/// `total` shared between `gaps` gaps, or nothing when there are none or it is negative.
fn spread(total: Emu, gaps: usize) -> Emu {
    if gaps == 0 || total <= Emu::ZERO {
        return Emu::ZERO;
    }
    total.divided_by(i64::try_from(gaps).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> LayoutRect {
        LayoutRect::from_edges(
            Emu::from_emu(0),
            Emu::from_emu(0),
            Emu::from_emu(4_000_000),
            Emu::from_emu(2_000_000),
        )
    }

    #[test]
    fn the_content_box_is_the_shape_less_the_schema_default_insets() {
        let content = content_box(shape(), &TextBodyPropertiesSpec::new());
        assert_eq!(content.left, Emu::from_emu(91_440));
        assert_eq!(content.top, Emu::from_emu(45_720));
        assert_eq!(content.right, Emu::from_emu(4_000_000 - 91_440));
        assert_eq!(content.bottom, Emu::from_emu(2_000_000 - 45_720));
    }

    #[test]
    fn a_stated_inset_replaces_the_default_and_a_stated_zero_is_zero() {
        let geometry = TextBodyPropertiesSpec::new()
            .with_left_inset(Emu::ZERO)
            .with_right_inset(Emu::from_emu(500_000));
        let content = content_box(shape(), &geometry);
        assert_eq!(content.left, Emu::ZERO);
        assert_eq!(content.right, Emu::from_emu(3_500_000));
        assert_eq!(
            content.top,
            Emu::from_emu(45_720),
            "the others still default"
        );
    }

    #[test]
    fn insets_that_overlap_collapse_rather_than_inverting() {
        let geometry = TextBodyPropertiesSpec::new()
            .with_left_inset(Emu::from_emu(3_000_000))
            .with_right_inset(Emu::from_emu(3_000_000));
        let content = content_box(shape(), &geometry);
        assert!(!content.is_empty() || content.width() == Emu::ZERO);
        assert!(content.right >= content.left, "never inverted");
    }

    #[test]
    fn one_column_is_the_whole_content_box() {
        let content = content_box(shape(), &TextBodyPropertiesSpec::new());
        let columns = columns_of(content, &TextBodyPropertiesSpec::new());
        assert_eq!(columns, vec![content]);
    }

    #[test]
    fn three_columns_share_the_width_and_leave_two_gaps() {
        let geometry = TextBodyPropertiesSpec::new()
            .with_columns(3)
            .with_column_space(Emu::from_emu(100_000));
        let content = LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_emu(1_100_000),
            Emu::from_emu(500_000),
        );
        let columns = columns_of(content, &geometry);
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].width(), Emu::from_emu(300_000));
        assert_eq!(columns[1].left, Emu::from_emu(400_000));
        assert_eq!(columns[2].left, Emu::from_emu(800_000));
        assert_eq!(columns[2].right, Emu::from_emu(1_100_000));
    }

    #[test]
    fn right_to_left_columns_reverse_which_rectangle_is_first() {
        let content = LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_emu(1_000_000),
            Emu::from_emu(500_000),
        );
        let left_to_right = columns_of(content, &TextBodyPropertiesSpec::new().with_columns(2));
        let right_to_left = columns_of(
            content,
            &TextBodyPropertiesSpec::new()
                .with_columns(2)
                .with_right_to_left_columns(true),
        );
        assert_eq!(left_to_right[0], right_to_left[1]);
        assert_eq!(left_to_right[1], right_to_left[0]);
    }

    #[test]
    fn a_column_count_past_the_schemas_ceiling_is_clamped_rather_than_believed() {
        let content = LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_emu(16_000_000),
            Emu::from_emu(500_000),
        );
        let columns = columns_of(content, &TextBodyPropertiesSpec::new().with_columns(60_000));
        assert_eq!(columns.len(), usize::from(MAXIMUM_COLUMNS));
    }

    #[test]
    fn a_gap_wider_than_the_content_falls_back_to_one_column() {
        let content = LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_emu(100_000),
            Emu::from_emu(500_000),
        );
        let geometry = TextBodyPropertiesSpec::new()
            .with_columns(4)
            .with_column_space(Emu::from_emu(1_000_000));
        assert_eq!(columns_of(content, &geometry), vec![content]);
    }
}
