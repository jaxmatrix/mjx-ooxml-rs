//! One cell, laid out: where its text sits in its box, and how far out of that box it is allowed to
//! go.
//!
//! # The order the rules apply in
//!
//! It matters, and getting it wrong produces a sheet that is subtly not Excel:
//!
//! 1. **Resolve `general`** against the cell's *type*, because every later rule reads the resolved
//!    alignment and not the stated one — a number in a `general` cell is right-aligned, so it
//!    overflows leftward.
//! 2. **`fill`** short-circuits: the text repeats inside the cell and nothing else applies.
//! 3. **Wrap** decides the measure. A wrapped cell composes at its own inner width; an unwrapped one
//!    composes at an effectively infinite measure and breaks only at a hard break.
//! 4. **Shrink-to-fit** runs only on an unwrapped cell that does not fit, and it runs *before*
//!    overflow, because a cell that shrank has nothing left to overflow with.
//! 5. **Overflow** is asked last, of an unwrapped, unshrunk cell whose line is wider than its box.
//!
//! # ⚠ Three numbers here are readings rather than specification
//!
//! GUESS: the cell's internal padding. Excel leaves a small gap between a cell's border and its
//! text — the figure used here is two pixels a side at 96 dpi, which matches Excel's own rendering
//! closely enough that a column auto-fitted here and in Excel agree — but ECMA-376 states no such
//! quantity anywhere.
//!
//! GUESS: `alignment@indent`'s unit. §18.8.1 says *"an increment of 1 represents 3 spaces"*, and
//! this reads a space as the maximum digit width, so one indent level is three times
//! [`MaximumDigitWidth`]. Whether Excel uses the space glyph's
//! own advance instead has not been observed on Windows.
//!
//! GUESS: what a rotation rotates *about*. This rotates about the centre of the cell's box, which
//! keeps rotated text inside its column at every angle. Excel appears to pivot about the alignment
//! anchor instead — a bottom-left-aligned rotated label rises from the bottom-left corner — and the
//! two differ visibly at 45°.

use mjx_layout::{ComposedLine, ComposedSegment, LayoutPoint, LayoutRect, Transform};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_ooxml_types::spreadsheetml::{CellType, HorizontalAlignment, VerticalAlignment};
use mjx_text::{BidiAnalysis, LineBreakOptions, TextDirection};

use crate::geometry::{MaximumDigitWidth, EMU_PER_PIXEL};
use crate::overflow::{self, Overflow, OverflowDirection};
use crate::text::{self, CellRunStyle, TextEngine};

/// The gap between a cell's edge and its text, in pixels a side at 96 dpi.
pub const CELL_PADDING_PIXELS: i64 = 2;

/// How many maximum-digit-widths one `alignment@indent` level is worth.
pub const INDENT_DIGITS: i64 = 3;

/// `alignment@textRotation`'s sentinel for vertically stacked text, which is not a rotation at all.
pub const STACKED_TEXT_ROTATION: u64 = 255;

/// The measure an unwrapped cell composes at — wide enough that only a hard break ends a line.
const UNWRAPPED_MEASURE_POINTS: f64 = 1.0e9;

/// How far a `centerContinuous` run is allowed to reach when the row is empty beside it.
///
/// GUESS: a guard rather than a rule. `centerContinuous` centres across the contiguous run of cells
/// carrying that alignment, and on a sheet whose whole `col` block states it the run would be the
/// full 16,384 columns. Sixty-four is far past any label a person centres across a report and far
/// short of a walk.
pub const MAX_CENTER_CONTINUOUS_SPAN: u16 = 64;

/// How many times `shrinkToFit` halves the interval it is searching.
///
/// Twelve bisections resolve the scale to better than one part in four thousand, which is finer than
/// a font size is quoted in.
const SHRINK_BISECTIONS: u32 = 12;

/// The smallest scale shrink-to-fit will apply.
///
/// GUESS: Excel does not shrink below a legible size, and a cell one pixel wide would otherwise ask
/// for a scale of nearly zero.
pub const MINIMUM_SHRINK_SCALE: f64 = 0.1;

/// How a cell's content sits in its box, resolved.
#[derive(Clone, PartialEq, Debug)]
pub struct CellStyle {
    /// The horizontal alignment, with `general` already resolved against the cell's type.
    pub horizontal: HorizontalAlignment,
    /// The vertical alignment.
    pub vertical: VerticalAlignment,
    /// `alignment@wrapText`.
    pub wrap: bool,
    /// `alignment@shrinkToFit`.
    pub shrink: bool,
    /// `alignment@indent`, in levels of [`INDENT_DIGITS`] digit widths each.
    pub indent: u32,
    /// `alignment@textRotation` — `0..=180` degrees, or [`STACKED_TEXT_ROTATION`].
    pub rotation: u64,
    /// `alignment@readingOrder`.
    pub reading_order: Option<u32>,
    /// The face, size, weight and slant the cell's text is set in.
    pub run: CellRunStyle,
}

impl CellStyle {
    /// The style stated by a resolved alignment and font, with `general` resolved against
    /// `cell_type`.
    #[must_use]
    pub fn resolve(
        alignment: Option<&mjx_sml::CellAlignment>,
        interner: &mjx_ooxml_core::Interner,
        font: Option<&mjx_sml::FontProperties>,
        cell_type: CellType,
    ) -> Self {
        let stated = alignment
            .and_then(|alignment| alignment.horizontal_alignment(interner).ok().flatten())
            .unwrap_or(HorizontalAlignment::General);
        let horizontal = if stated == HorizontalAlignment::General {
            overflow::resolve_general(cell_type)
        } else {
            stated
        };
        Self {
            horizontal,
            vertical: alignment
                .and_then(|alignment| alignment.vertical_alignment(interner).ok())
                .unwrap_or(VerticalAlignment::Bottom),
            wrap: alignment
                .and_then(|alignment| alignment.wraps_text(interner).ok().flatten())
                .unwrap_or(false),
            shrink: alignment
                .and_then(|alignment| alignment.shrinks_to_fit(interner).ok().flatten())
                .unwrap_or(false),
            indent: alignment
                .and_then(|alignment| alignment.indent(interner).ok().flatten())
                .unwrap_or(0),
            rotation: alignment
                .and_then(|alignment| alignment.text_rotation(interner).ok().flatten())
                .unwrap_or(0),
            reading_order: alignment
                .and_then(|alignment| alignment.reading_order(interner).ok().flatten()),
            run: CellRunStyle::from_font(font),
        }
    }

    /// Whether the text is stacked one character per line rather than rotated.
    #[must_use]
    pub fn is_stacked(&self) -> bool {
        self.rotation == STACKED_TEXT_ROTATION
    }

    /// The rotation as an angle on the page, **clockwise**, or `None` for upright or stacked text.
    ///
    /// §18.8.1: `0..=90` is anticlockwise from horizontal, and `91..=180` means
    /// `value - 90` degrees *clockwise*. The screen's `y` grows downward, so an anticlockwise
    /// document rotation is a negative clockwise page rotation.
    #[must_use]
    pub fn rotation_angle(&self) -> Option<Angle> {
        match self.rotation {
            0 | STACKED_TEXT_ROTATION => None,
            degrees @ 1..=90 =>
            {
                #[allow(clippy::cast_precision_loss)]
                Some(Angle::from_degrees(-(degrees as f64)))
            }
            degrees @ 91..=180 =>
            {
                #[allow(clippy::cast_precision_loss)]
                Some(Angle::from_degrees((degrees - 90) as f64))
            }
            // Anything else is a value the schema does not allow. Drawing it upright is better than
            // refusing the sheet.
            _ => None,
        }
    }
}

/// One line of a cell's text, placed.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedLine {
    /// The line's box, in sheet coordinates.
    pub rect: LayoutRect,
    /// Where the baseline sits, measured down from the top of `rect`.
    pub baseline: Emu,
    /// How far above the baseline the line reaches.
    pub ascent: Emu,
    /// How far below.
    pub descent: Emu,
    /// Which way the line reads.
    pub direction: TextDirection,
    /// Where the pen starts, in sheet coordinates.
    pub origin: LayoutPoint,
    /// The bytes of the cell's display text it covers.
    pub range: std::ops::Range<usize>,
    /// The shaped stretches, in visual order.
    pub segments: Vec<ComposedSegment>,
}

/// A cell's text, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedText {
    /// Every line, top to bottom.
    pub lines: Vec<PlacedLine>,
    /// What happened at the cell's edges.
    pub overflow: Overflow,
    /// What shrink-to-fit multiplied the font size by; `1.0` when it did not run.
    pub scale: f64,
    /// The rectangle the text is allowed to be drawn in — the cell's box, widened by whatever
    /// overflow permitted.
    pub text_box: LayoutRect,
    /// The clip the text is drawn under, or `None` when nothing clips it.
    pub clip: Option<LayoutRect>,
    /// The transform its subtree is drawn in, or `None` for upright text.
    pub transform: Option<Transform>,
}

impl PlacedText {
    /// A cell with nothing in it.
    #[must_use]
    pub fn empty(rect: LayoutRect) -> Self {
        Self {
            lines: Vec::new(),
            overflow: Overflow::Fits,
            scale: 1.0,
            text_box: rect,
            clip: None,
            transform: None,
        }
    }

    /// How tall the text is in total.
    #[must_use]
    pub fn height(&self) -> Emu {
        self.lines
            .iter()
            .map(|line| line.rect.height())
            .fold(Emu::ZERO, |total, height| total + height)
    }
}

/// Everything about the cell's surroundings the placement needs, so that this module never holds a
/// [`SheetGrid`](crate::SheetGrid) and can be driven from a plain closure in a test.
///
/// Two of its fields are `&mut dyn FnMut`, which is why [`Debug`] is written out below rather than
/// derived: a closure has no representation to print, and the two that matter — the cell's own
/// rectangle and the probes that decide its overflow — do.
pub struct CellContext<'a> {
    /// The cell's own box in sheet coordinates — the merged union's, for a merge anchor.
    pub rect: LayoutRect,
    /// Which column the cell is in.
    pub column: u16,
    /// The digit width every column on this sheet is quoted in.
    pub digit: MaximumDigitWidth,
    /// How wide a neighbouring column is.
    pub column_width: &'a mut dyn FnMut(u16) -> Emu,
    /// Where a neighbouring column's left edge is.
    pub column_left: &'a mut dyn FnMut(u16) -> Emu,
    /// The nearest populated column strictly to the right, if any.
    pub occupied_right: Option<u16>,
    /// The nearest populated column strictly to the left, if any.
    pub occupied_left: Option<u16>,
    /// Whether the cell's alignment centres across a run of columns.
    pub center_continuous_span: Option<(u16, u16)>,
    /// Whether the cell is a merge anchor, which suppresses overflow entirely.
    pub merged: bool,
}

impl std::fmt::Debug for CellContext<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CellContext")
            .field("rect", &self.rect)
            .field("column", &self.column)
            .field("digit", &self.digit)
            .field("occupied_right", &self.occupied_right)
            .field("occupied_left", &self.occupied_left)
            .field("center_continuous_span", &self.center_continuous_span)
            .field("merged", &self.merged)
            .finish_non_exhaustive()
    }
}

/// Lays out one cell's `text` in `context` under `style`.
///
/// # Errors
/// [`mjx_text::FontError`] when a face will not shape.
pub fn place(
    engine: &mut TextEngine<'_>,
    text: &str,
    style: &CellStyle,
    context: &mut CellContext<'_>,
) -> Result<PlacedText, mjx_text::FontError> {
    if text.is_empty() {
        return Ok(PlacedText::empty(context.rect));
    }
    if style.is_stacked() {
        return place_stacked(engine, text, style, context);
    }

    let padding = Emu::from_emu(CELL_PADDING_PIXELS * EMU_PER_PIXEL);
    let indent = Emu::from_emu(
        i64::from(style.indent) * INDENT_DIGITS * context.digit.pixels() * EMU_PER_PIXEL,
    );
    let inner = (context.rect.width() - padding.times(2) - indent).maximum(Emu::ZERO);

    let direction = text::reading_order(style.reading_order);
    let bidi = BidiAnalysis::resolve(text, direction);
    let options = LineBreakOptions::default();

    // `fill` never leaves the cell and never wraps: the text repeats until the box is full.
    if style.horizontal == HorizontalAlignment::Fill {
        return place_filled(
            engine, text, style, context, &bidi, &options, inner, padding,
        );
    }

    let mut scale = 1.0_f64;
    let mut composed = compose_at(
        engine, text, style, &bidi, &options, inner, style.wrap, scale,
    )?;

    if style.shrink && !style.wrap && inner > Emu::ZERO {
        let widest = widest_line(&composed);
        if widest > inner {
            scale = shrink_scale(engine, text, style, &bidi, &options, inner)?;
            composed = compose_at(engine, text, style, &bidi, &options, inner, false, scale)?;
        }
    }

    let widest = widest_line(&composed);
    let overflow = if style.wrap {
        Overflow::SuppressedByWrap
    } else if context.merged {
        // A merged region already spans its columns; letting it spill further would draw over cells
        // the merge deliberately does not cover.
        Overflow::Fits
    } else {
        resolve_overflow(style, context, widest, inner)
    };

    let text_box = text_box_for(context, &overflow, style);
    let clip = clip_for(context, &overflow, style, &text_box);
    let lines = stack_lines(&composed, style, context, &text_box, padding, indent, scale);
    let transform = style
        .rotation_angle()
        .map(|angle| Transform::rotation_about(angle, centre_of(context.rect)));

    Ok(PlacedText {
        lines,
        overflow,
        scale,
        text_box,
        clip,
        transform,
    })
}

/// The centre of a rectangle, which is what a rotation pivots about.
fn centre_of(rect: LayoutRect) -> LayoutPoint {
    LayoutPoint {
        x: rect.left + rect.width().divided_by(2),
        y: rect.top + rect.height().divided_by(2),
    }
}

/// Composes the cell's text at `scale`, wrapping at `inner` or not at all.
#[allow(clippy::too_many_arguments)]
fn compose_at(
    engine: &mut TextEngine<'_>,
    text: &str,
    style: &CellStyle,
    bidi: &BidiAnalysis,
    options: &LineBreakOptions,
    inner: Emu,
    wrap: bool,
    scale: f64,
) -> Result<Vec<ComposedLine>, mjx_text::FontError> {
    let scaled = style.run.scaled(scale);
    let items = text::itemise_cell(engine, text, &scaled, bidi)?;
    let runs = text::composer_runs(engine.rasteriser, engine.features, &items, scaled.size);
    let measure = if wrap {
        inner.points().max(1.0)
    } else {
        UNWRAPPED_MEASURE_POINTS
    };
    text::compose_cell(engine.shaper, text, &runs, bidi, options.clone(), measure)
}

/// The widest composed line, in EMU.
fn widest_line(lines: &[ComposedLine]) -> Emu {
    lines
        .iter()
        .map(|line| Emu::from_points(line.width_in_points))
        .fold(Emu::ZERO, Emu::maximum)
}

/// The largest scale at which the text fits `inner`, by bisection.
///
/// Bisection rather than a closed form because a glyph's advance is not linear in the point size:
/// hinting and rounding make a run at 9 pt slightly wider than nine tenths of the same run at 10 pt,
/// so solving for the scale directly would give an answer that does not fit.
fn shrink_scale(
    engine: &mut TextEngine<'_>,
    text: &str,
    style: &CellStyle,
    bidi: &BidiAnalysis,
    options: &LineBreakOptions,
    inner: Emu,
) -> Result<f64, mjx_text::FontError> {
    let mut low = MINIMUM_SHRINK_SCALE;
    let mut high = 1.0_f64;
    for _ in 0..SHRINK_BISECTIONS {
        let middle = f64::midpoint(low, high);
        let composed = compose_at(engine, text, style, bidi, options, inner, false, middle)?;
        if widest_line(&composed) <= inner {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok(low)
}

/// `horizontal="fill"` — the text repeated until the box is full, clipped to it.
#[allow(clippy::too_many_arguments)]
fn place_filled(
    engine: &mut TextEngine<'_>,
    text: &str,
    style: &CellStyle,
    context: &mut CellContext<'_>,
    bidi: &BidiAnalysis,
    options: &LineBreakOptions,
    inner: Emu,
    padding: Emu,
) -> Result<PlacedText, mjx_text::FontError> {
    let once = compose_at(engine, text, style, bidi, options, inner, false, 1.0)?;
    let width = widest_line(&once);
    let repeats = if width <= Emu::ZERO {
        1
    } else {
        // One more than fits, so the last repeat is clipped rather than leaving a gap.
        usize::try_from(inner.emu() / width.emu().max(1))
            .unwrap_or(1)
            .saturating_add(1)
            .clamp(1, 4096)
    };
    let repeated = text.repeat(repeats);
    let bidi = BidiAnalysis::resolve(&repeated, text::reading_order(style.reading_order));
    let composed = compose_at(engine, &repeated, style, &bidi, options, inner, false, 1.0)?;
    let text_box = context.rect;
    let filled = CellStyle {
        horizontal: HorizontalAlignment::Left,
        ..style.clone()
    };
    let lines = stack_lines(
        &composed,
        &filled,
        context,
        &text_box,
        padding,
        Emu::ZERO,
        1.0,
    );
    Ok(PlacedText {
        lines,
        overflow: Overflow::SuppressedByFill,
        scale: 1.0,
        text_box,
        clip: Some(context.rect),
        transform: None,
    })
}

/// `textRotation="255"` — one character per line, stacked downward and centred.
fn place_stacked(
    engine: &mut TextEngine<'_>,
    text: &str,
    style: &CellStyle,
    context: &mut CellContext<'_>,
) -> Result<PlacedText, mjx_text::FontError> {
    let options = LineBreakOptions::default();
    let mut lines: Vec<PlacedLine> = Vec::new();
    let mut cursor = context.rect.top;
    let centre = context.rect.left + context.rect.width().divided_by(2);
    for (offset, character) in text.char_indices() {
        let piece = &text[offset..offset + character.len_utf8()];
        let bidi = BidiAnalysis::resolve(piece, text::reading_order(style.reading_order));
        let composed = compose_at(
            engine,
            piece,
            style,
            &bidi,
            &options,
            context.rect.width(),
            false,
            1.0,
        )?;
        let Some(line) = composed.into_iter().next() else {
            continue;
        };
        let ascent = Emu::from_points(line.ascent_in_points);
        let descent = Emu::from_points(line.descent_in_points);
        let height = ascent + descent;
        let width = Emu::from_points(line.width_in_points);
        let left = centre - width.divided_by(2);
        lines.push(PlacedLine {
            rect: LayoutRect::from_edges(left, cursor, left + width, cursor + height),
            baseline: ascent,
            ascent,
            descent,
            direction: TextDirection::LeftToRight,
            origin: LayoutPoint {
                x: left,
                y: cursor + ascent,
            },
            range: offset..offset + character.len_utf8(),
            segments: line.segments,
        });
        cursor += height;
    }
    Ok(PlacedText {
        lines,
        overflow: Overflow::Fits,
        scale: 1.0,
        text_box: context.rect,
        clip: Some(context.rect),
        transform: None,
    })
}

/// Asks [`crate::overflow`] how far this cell's text may spread.
///
/// `occupied` is answered from **one** probe each way, which the caller made through the packed
/// store's own index: the first populated column to the right of this cell is the only column to the
/// right that can stop the text, so comparing against it is exactly as correct as asking each column
/// in turn and is `O(1)` instead of `O(columns)`.
fn resolve_overflow(
    style: &CellStyle,
    context: &mut CellContext<'_>,
    widest: Emu,
    inner: Emu,
) -> Overflow {
    if let Some((first, last)) = context.center_continuous_span {
        return Overflow::Spills {
            columns: (first, last),
            direction: OverflowDirection::Both,
            stopped_left_by: None,
            stopped_right_by: None,
        };
    }
    let Some(direction) = OverflowDirection::of(style.horizontal) else {
        return Overflow::Fits;
    };
    let needed = widest - inner;
    let occupied_left = context.occupied_left;
    let occupied_right = context.occupied_right;
    overflow::resolve(
        context.column,
        needed,
        direction,
        |column| (context.column_width)(column),
        |column| Some(column) == occupied_left || Some(column) == occupied_right,
    )
}

/// The rectangle the text may be drawn in, once overflow has said how far it reaches.
fn text_box_for(
    context: &mut CellContext<'_>,
    overflow: &Overflow,
    _style: &CellStyle,
) -> LayoutRect {
    let (first, last) = overflow.columns(context.column);
    if first == context.column && last == context.column {
        return context.rect;
    }
    let left = (context.column_left)(first);
    let right = (context.column_left)(last) + (context.column_width)(last);
    LayoutRect::from_edges(
        left.minimum(context.rect.left),
        context.rect.top,
        right.maximum(context.rect.right),
        context.rect.bottom,
    )
}

/// The clip the text is drawn under.
///
/// **Always the text box**, and that is the rule rather than an optimisation. The text box is the
/// cell's own rectangle when nothing spilled, the widened union when overflow allowed one, and the
/// cell's own rectangle again when a populated neighbour stopped it — so one clip expresses all
/// three states, and each of them is a state a reader can see:
///
/// * a label that fits is not visibly clipped, because it does not reach the edge;
/// * a label that spilled is clipped at the last empty column it was allowed to cross;
/// * a label a neighbour stopped is clipped at its own edge, which is what "cut off" looks like.
///
/// A line's `rect` stays the text's **natural** extent, unclipped, because that is what a hit test
/// and a selection need: clicking past the visible end of a truncated label still lands in the cell
/// whose text it is.
fn clip_for(
    context: &mut CellContext<'_>,
    _overflow: &Overflow,
    _style: &CellStyle,
    text_box: &LayoutRect,
) -> Option<LayoutRect> {
    let _ = context;
    Some(*text_box)
}

/// Stacks the composed lines inside `text_box` under the cell's two alignments.
fn stack_lines(
    composed: &[ComposedLine],
    style: &CellStyle,
    context: &mut CellContext<'_>,
    text_box: &LayoutRect,
    padding: Emu,
    indent: Emu,
    _scale: f64,
) -> Vec<PlacedLine> {
    let total = composed
        .iter()
        .map(|line| Emu::from_points(line.height_in_points()))
        .fold(Emu::ZERO, |sum, height| sum + height);
    let box_height = context.rect.height();
    let slack = (box_height - total).maximum(Emu::ZERO);
    let mut cursor = context.rect.top
        + match style.vertical {
            VerticalAlignment::Top => Emu::ZERO,
            VerticalAlignment::Center | VerticalAlignment::Distributed => slack.divided_by(2),
            // GUESS: `justify` and `distributed` should spread the slack *between* the lines rather
            // than centring the block. Doing that needs the per-line gap to be a property of the
            // line rather than of the stack, which is a change to `PlacedLine`; centring is the
            // closer of the two available answers and is marked so the sitting can settle it.
            VerticalAlignment::Bottom | VerticalAlignment::Justify => slack,
        };

    let mut placed = Vec::with_capacity(composed.len());
    for line in composed {
        let ascent = Emu::from_points(line.ascent_in_points);
        let descent = Emu::from_points(line.descent_in_points);
        let height = Emu::from_points(line.height_in_points());
        let width = Emu::from_points(line.width_in_points);
        let usable = (text_box.width() - padding.times(2) - indent).maximum(Emu::ZERO);
        let offset = match style.horizontal {
            HorizontalAlignment::Right => (usable - width).maximum(Emu::ZERO),
            HorizontalAlignment::Center | HorizontalAlignment::CenterContinuous => {
                (usable - width).maximum(Emu::ZERO).divided_by(2)
            }
            _ => Emu::ZERO,
        };
        let left = text_box.left + padding + indent + offset;
        placed.push(PlacedLine {
            rect: LayoutRect::from_edges(left, cursor, left + width, cursor + height),
            baseline: ascent,
            ascent,
            descent,
            direction: direction_of(line),
            origin: LayoutPoint {
                x: left,
                y: cursor + ascent,
            },
            range: line.range.clone(),
            segments: line.segments.clone(),
        });
        cursor += height;
    }
    placed
}

/// The direction a composed line reads as a whole — its first segment's, and left to right for a
/// line with no segments at all.
fn direction_of(line: &ComposedLine) -> TextDirection {
    line.segments
        .first()
        .map_or(TextDirection::LeftToRight, |segment| segment.direction)
}

/// How far a `centerContinuous` cell centres across.
///
/// The run is the contiguous stretch of empty columns beside this one, bounded by the first
/// populated cell on each side and by [`MAX_CENTER_CONTINUOUS_SPAN`].
#[must_use]
pub fn center_continuous_span(
    column: u16,
    occupied_left: Option<u16>,
    occupied_right: Option<u16>,
) -> (u16, u16) {
    let first = occupied_left
        .map_or(column.saturating_sub(MAX_CENTER_CONTINUOUS_SPAN), |stop| {
            stop.saturating_add(1)
        })
        .max(column.saturating_sub(MAX_CENTER_CONTINUOUS_SPAN));
    let last = occupied_right
        .map_or(column.saturating_add(MAX_CENTER_CONTINUOUS_SPAN), |stop| {
            stop.saturating_sub(1)
        })
        .min(column.saturating_add(MAX_CENTER_CONTINUOUS_SPAN));
    (first.min(column), last.max(column))
}
