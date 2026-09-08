//! The stage every scene starts from, and the seven things enough scenes draw that drawing them
//! twice would be two drawings to keep in step.
//!
//! # ⚠ The stage is deliberately the *smallest* thing that can be called a document
//!
//! A backdrop and a page, and nothing else — two `FillPath` commands. That number is load-bearing:
//! `tests/every_entry_draws.rs` renders the bare stage, counts its commands, and requires every one
//! of the sixty-one entries to draw **more** than it. *"All 61 elements have a scene"* is satisfied
//! by sixty-one titled empty canvases, and this is what refuses them: an entry whose builder
//! returned after calling [`stage`] fails naming its own number.
//!
//! # Why a line of text is a grey bar
//!
//! See `crate::canvas`. In one sentence: a golden image containing glyph coverage is a golden image
//! of a rasteriser version and an installed face, and the elements this harness is about — the
//! caret, the selection fill, the squiggle, the bracket — are all *drawn beside* text rather than
//! *made of* it. [`text_line`] is what they sit beside.

use mjx_scene::DashPattern;

use crate::canvas::{
    self, dashed, filled, filled_and_stroked, pt, stroke, Canvas, NodeId, Pt, Rect, PAGE,
};

/// The content area inside the page — where a scene's own drawing goes.
pub const CONTENT: Rect = Rect {
    x: 34.0,
    y: 28.0,
    width: 232.0,
    height: 144.0,
};

/// How far apart two lines of text sit, in points.
pub const LINE_HEIGHT: f64 = 14.0;

/// How tall the bar standing in for a line of words is, in points.
pub const TEXT_BAR: f64 = 5.0;

/// The backdrop and the page. **Every scene begins here.**
pub fn stage(canvas: &mut Canvas) -> NodeId {
    let ink = canvas.ink().clone();
    canvas.rect(
        None,
        Rect::new(0.0, 0.0, canvas::STAGE.0, canvas::STAGE.1),
        filled(ink.backdrop()),
    );
    canvas.rect(None, PAGE, filled(ink.page()))
}

/// A document object — the thing the in-canvas UI is *about*.
///
/// Drawn in `theme.*.secondary-surface` with its own edge, which is deliberately a different family
/// of colour from anything the UI uses: an audit whose object and whose selection outline were the
/// same hue could not answer the question *"does this read as selection rather than as part of the
/// drawing?"*, which is entry 1's whole judgement.
pub fn object(canvas: &mut Canvas, parent: Option<NodeId>, rect: Rect) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    canvas.rect(
        parent,
        rect,
        filled_and_stroked(scale, ink.object(), ink.object_border(), 0.75),
    )
}

/// One line of words, as a bar at the metrics a line of text would have.
pub fn text_line(canvas: &mut Canvas, at: Pt, width: f64) -> Rect {
    let ink = canvas.ink().clone();
    let rect = Rect::new(at.x, at.y, width, TEXT_BAR);
    canvas.rect(None, rect, filled(canvas::with_alpha(ink.text(), 0x66)));
    rect
}

/// A paragraph: one bar per entry in `widths`, at [`LINE_HEIGHT`] apart, returning each line's
/// rectangle so a caller can put a caret or a selection on it.
pub fn paragraph(canvas: &mut Canvas, at: Pt, widths: &[f64]) -> Vec<Rect> {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            text_line(
                canvas,
                pt(at.x, at.y + (index as f64) * LINE_HEIGHT),
                *width,
            )
        })
        .collect()
}

/// A grid of cells: the lines, drawn in `document.*.grid-line`.
pub fn gridlines(canvas: &mut Canvas, area: Rect, columns: usize, rows: usize) {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let column_width = area.width / columns as f64;
    let row_height = area.height / rows as f64;
    for column in 0..=columns {
        let x = area.x + column as f64 * column_width;
        canvas.line(
            None,
            pt(x, area.y),
            pt(x, area.bottom()),
            "gridline",
            stroke(scale, ink.grid(), 0.5),
        );
    }
    for row in 0..=rows {
        let y = area.y + row as f64 * row_height;
        canvas.line(
            None,
            pt(area.x, y),
            pt(area.right(), y),
            "gridline",
            stroke(scale, ink.grid(), 0.5),
        );
    }
}

/// The rectangle of one cell of a grid laid out over `area`.
#[must_use]
pub fn cell(area: Rect, columns: usize, rows: usize, column: usize, row: usize) -> Rect {
    let column_width = area.width / columns as f64;
    let row_height = area.height / rows as f64;
    Rect::new(
        area.x + column as f64 * column_width,
        area.y + row as f64 * row_height,
        column_width,
        row_height,
    )
}

/// The rectangle spanning cells `(from_column, from_row)` to `(to_column, to_row)` inclusive.
#[must_use]
pub fn span(
    area: Rect,
    columns: usize,
    rows: usize,
    from: (usize, usize),
    to: (usize, usize),
) -> Rect {
    let first = cell(area, columns, rows, from.0, from.1);
    let last = cell(area, columns, rows, to.0, to.1);
    Rect::new(
        first.x,
        first.y,
        last.right() - first.x,
        last.bottom() - first.y,
    )
}

/// A selection outline around `rect` — no fill, a stroke of the accent colour at the width the
/// interaction state gives.
pub fn selection_outline(canvas: &mut Canvas, rect: Rect, resting_width: f64) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    canvas.rect(
        None,
        rect,
        canvas::stroked(scale, ink.accent(), ink.width(resting_width)),
    )
}

/// A dashed outline around `rect` — a preview, a boundary, a guide.
pub fn dashed_outline(
    canvas: &mut Canvas,
    rect: Rect,
    resting_width: f64,
    dash: DashPattern,
) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    canvas.rect(
        None,
        rect,
        dashed(scale, ink.accent(), ink.width(resting_width), dash),
    )
}

/// A square resize handle, and the grab region that goes with it.
///
/// The handle's own size is [`crate::canvas::Ink::affordance`] — the input device's number, grown
/// while engaged — and its grab region is that inflated by
/// [`crate::canvas::Ink::grab_padding`]. Both move when the input toggle moves, which is what makes
/// the touch axis judgeable rather than merely present.
pub fn handle(canvas: &mut Canvas, centre: Pt, label: &'static str) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let node = canvas.rect(
        None,
        Rect::centred(centre, ink.affordance()),
        filled_and_stroked(scale, ink.page(), ink.accent(), ink.width(1.0)),
    );
    canvas.grab(node, label);
    node
}

/// A handle that is *not* the one being pointed at — drawn at the resting size whatever the
/// interaction state says.
///
/// **The reason entry 3 can be judged at all.** If hover grew every handle at once, the toggle would
/// be indistinguishable from a scale change; growing exactly one is what shows a reviewer what the
/// pointer is doing.
pub fn resting_handle(canvas: &mut Canvas, centre: Pt, label: &'static str) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let side = ink.state.input.affordance_size();
    let node = canvas.rect(
        None,
        Rect::centred(centre, side),
        filled_and_stroked(scale, ink.page(), ink.accent(), 1.0),
    );
    canvas.grab(node, label);
    node
}

/// A diamond adjustment handle — the `avLst` control point, which must never be mistaken for a
/// resize handle.
pub fn adjustment_handle(canvas: &mut Canvas, centre: Pt, label: &'static str) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let radius = ink.affordance() / 2.0;
    let node = canvas.polygon(
        None,
        &[
            pt(centre.x, centre.y - radius),
            pt(centre.x + radius, centre.y),
            pt(centre.x, centre.y + radius),
            pt(centre.x - radius, centre.y),
        ],
        "adjustment handle",
        filled_and_stroked(scale, ink.warning(), ink.accent(), ink.width(0.9)),
    );
    canvas.grab(node, label);
    node
}

/// A round handle — a connection site, a vertex control point, a motion-path marker.
pub fn round_handle(canvas: &mut Canvas, centre: Pt, label: &'static str) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let node = canvas.circle(
        None,
        centre,
        ink.affordance() / 2.0,
        "round handle",
        filled_and_stroked(scale, ink.page(), ink.accent(), ink.width(1.0)),
    );
    canvas.grab(node, label);
    node
}

/// A readout badge: a raised plate with two bars standing in for the numbers on it.
///
/// The numbers are bars for the same reason a line of text is: a plate carrying real digits would be
/// a plate of a font. What a reviewer judges here is the badge's size, its contrast against what it
/// covers, and where it sits relative to what it measures.
pub fn badge(canvas: &mut Canvas, at: Pt, width: f64) -> NodeId {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let rect = Rect::new(at.x, at.y, width, 13.0);
    let node = canvas.rect(
        None,
        rect,
        filled_and_stroked(scale, ink.raised_surface(), ink.border(), 0.6),
    );
    canvas.rect(
        Some(node),
        Rect::new(rect.x + 4.0, rect.y + 4.0, width * 0.42, 4.0),
        filled(ink.text()),
    );
    canvas.rect(
        Some(node),
        Rect::new(rect.x + width * 0.5, rect.y + 4.0, width * 0.36, 4.0),
        filled(canvas::with_alpha(ink.text(), 0x99)),
    );
    node
}

/// A filled triangle, pointing in one of four directions — an arrow, a note indicator, a fill
/// preview marker.
pub fn triangle(
    canvas: &mut Canvas,
    corner: Pt,
    size: f64,
    towards: (f64, f64),
    label: &'static str,
    color: mjx_scene::Color,
) -> NodeId {
    let (dx, dy) = towards;
    canvas.polygon(
        None,
        &[
            corner,
            pt(corner.x + dx * size, corner.y),
            pt(corner.x, corner.y + dy * size),
        ],
        label,
        filled(color),
    )
}

/// The path of a rectangle with rounded corners, in points.
///
/// Written out rather than reached for from `mjx-geometry`: that crate is rank 2.5, this harness may
/// legally name it, and it would be the wrong dependency all the same. A preset shape table is
/// **DrawingML** — the thing the box model is above — and a harness that needed one could not run
/// with no document, which is the property that lets in-canvas design be settled before any format
/// renders. `crates/mjx-paint/tests/the_seam_holds.rs` refuses the same edge for the painter.
#[must_use]
pub fn rounded_rectangle(rect: Rect, radius: f64) -> Vec<mjx_scene::PathCommand> {
    use mjx_scene::{PathCommand, ScenePoint};
    let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0);
    let point = |x: f64, y: f64| ScenePoint::new(x as f32, y as f32);
    let (left, top, right, bottom) = (rect.x, rect.y, rect.right(), rect.bottom());
    vec![
        PathCommand::MoveTo(point(left + radius, top)),
        PathCommand::LineTo(point(right - radius, top)),
        PathCommand::QuadraticTo {
            control: point(right, top),
            end: point(right, top + radius),
        },
        PathCommand::LineTo(point(right, bottom - radius)),
        PathCommand::QuadraticTo {
            control: point(right, bottom),
            end: point(right - radius, bottom),
        },
        PathCommand::LineTo(point(left + radius, bottom)),
        PathCommand::QuadraticTo {
            control: point(left, bottom),
            end: point(left, bottom - radius),
        },
        PathCommand::LineTo(point(left, top + radius)),
        PathCommand::QuadraticTo {
            control: point(left, top),
            end: point(left + radius, top),
        },
        PathCommand::Close,
    ]
}

/// A squiggle: a run of quadratic humps along `width`, at `period` and `amplitude` points.
///
/// The three squiggles of entry 17 differ in exactly these two numbers and in colour, which is what
/// makes them tellable apart — a reviewer judging them is judging whether that is enough.
#[must_use]
pub fn squiggle(at: Pt, width: f64, period: f64, amplitude: f64) -> Vec<mjx_scene::PathCommand> {
    use mjx_scene::{PathCommand, ScenePoint};
    let point = |x: f64, y: f64| ScenePoint::new(x as f32, y as f32);
    let mut commands = vec![PathCommand::MoveTo(point(at.x, at.y))];
    let mut x = at.x;
    let mut up = true;
    while x < at.x + width {
        let next = (x + period).min(at.x + width);
        commands.push(PathCommand::QuadraticTo {
            control: point(
                (x + next) / 2.0,
                if up {
                    at.y - amplitude
                } else {
                    at.y + amplitude
                },
            ),
            end: point(next, at.y),
        });
        up = !up;
        x = next;
    }
    commands
}

/// A chevron — the arrowhead a connector, a dropdown or a direction indicator ends in.
///
/// The colour and the width are the caller's, not the palette's own accent. That is not a style
/// preference: a non-printing mark and a dropdown arrow are the same shape and belong to *different*
/// axes of the state matrix, and a helper that reached for `Ink::accent` would make every scene that
/// drew a chevron respond to the interaction toggle — including the six that are declared static,
/// whose declaration `tests/the_axes_are_not_identities.rs` would then refuse.
pub fn chevron(
    canvas: &mut Canvas,
    tip: Pt,
    size: f64,
    down: bool,
    label: &'static str,
    color: mjx_scene::Color,
    width: f64,
) -> NodeId {
    let scale = canvas.pixels_per_point();
    let sign = if down { -1.0 } else { 1.0 };
    canvas.polyline(
        None,
        &[
            pt(tip.x - size, tip.y + sign * size),
            tip,
            pt(tip.x + size, tip.y + sign * size),
        ],
        false,
        label,
        stroke(scale, color, width),
    )
}
