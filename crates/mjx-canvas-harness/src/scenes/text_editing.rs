//! §2.2 — text selection and editing, entries 12 to 21.
//!
//! Every scene here draws its own text as bars rather than as glyphs. `crate::canvas` gives the
//! reason at length; the short form is that these ten elements are drawn **beside** text, not made
//! of it, and a plate of a caret in a line of real glyphs is a plate of the font stack.

use mjx_scene::{DashPattern, Decoration, FillRule, FillStyle, Gradient, GradientStop};

use crate::canvas::{self, dashed, filled, pt, stroke, Canvas, Rect};
use crate::scenes::stage::{self, badge, paragraph, text_line, CONTENT, LINE_HEIGHT, TEXT_BAR};
use crate::state::Interaction;

/// 12 — Caret: blink cadence and hairline thickness across DPI.
pub fn caret_hairline(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let lines = paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 6.0),
        &[188.0, 204.0, 152.0],
    );
    let colour = match ink.state.interaction {
        Interaction::Disabled => canvas::with_alpha(ink.muted(), 0x60),
        Interaction::Default => ink.text(),
        _ => ink.accent(),
    };
    // The caret in the second line, at the width the platform actually draws.
    let line = lines[1];
    canvas.rect(
        None,
        Rect::new(line.x + 96.0, line.y - 3.0, 1.0, TEXT_BAR + 6.0),
        filled(colour),
    );
    // The ladder: the same caret at four widths, so a reviewer can say which one survives at each
    // density rather than only whether the platform's choice does.
    for (index, width) in [0.5_f64, 0.75, 1.0, 1.5].into_iter().enumerate() {
        let x = CONTENT.x + 12.0 + (index as f64) * 46.0;
        canvas.rect(
            None,
            Rect::new(x, CONTENT.y + 78.0, width, 26.0),
            filled(colour),
        );
        canvas.rect(
            None,
            Rect::new(x - 6.0, CONTENT.y + 110.0, 12.0, 3.0),
            filled(canvas::with_alpha(ink.muted(), 0x88)),
        );
    }
}

/// 13 — Caret: the bidi split form and the direction indicator.
pub fn caret_bidi(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let base = pt(CONTENT.x, CONTENT.y + 30.0);
    text_line(canvas, base, 92.0);
    // The right-to-left run, in a different tint so the boundary is where the eye already is.
    canvas.rect(
        None,
        Rect::new(base.x + 100.0, base.y, 78.0, TEXT_BAR),
        filled(canvas::with_alpha(ink.object_border(), 0x99)),
    );
    text_line(canvas, pt(base.x + 186.0, base.y), 40.0);

    let colour = if ink.state.interaction == Interaction::Disabled {
        canvas::with_alpha(ink.muted(), 0x60)
    } else {
        ink.accent()
    };
    // The split caret: an upper half on the side the next left-to-right character goes, a lower
    // half on the side the next right-to-left one does.
    canvas.rect(
        None,
        Rect::new(base.x + 96.0, base.y - 4.0, 1.25, 7.0),
        filled(colour),
    );
    canvas.rect(
        None,
        Rect::new(base.x + 178.0, base.y + 2.0, 1.25, 7.0),
        filled(colour),
    );
    // The direction flag, on the half that is active.
    canvas.polygon(
        None,
        &[
            pt(base.x + 97.25, base.y - 4.0),
            pt(base.x + 103.0, base.y - 4.0),
            pt(base.x + 97.25, base.y - 0.5),
        ],
        "direction flag",
        filled(colour),
    );
    canvas.rect(
        None,
        Rect::new(CONTENT.x, CONTENT.y + 74.0, 210.0, TEXT_BAR),
        filled(canvas::with_alpha(ink.text(), 0x44)),
    );
}

/// 14 — Selection fill: single line, multi-line, ragged trailing edge.
pub fn selection_fill_runs(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let lines = paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 8.0),
        &[206.0, 214.0, 198.0, 128.0, 178.0],
    );
    let fill = canvas.group(None, CONTENT, ink.selection_opacity(), None);
    let colour = ink.selection_colour();
    let mut band = |rect: Rect| {
        canvas.rect(Some(fill), rect, filled(colour));
    };
    // Within one line.
    band(Rect::new(
        lines[0].x + 44.0,
        lines[0].y - 3.0,
        74.0,
        TEXT_BAR + 6.0,
    ));
    // Across three, with a ragged last line.
    band(Rect::new(
        lines[2].x + 88.0,
        lines[2].y - 3.0,
        110.0,
        TEXT_BAR + 6.0,
    ));
    band(Rect::new(
        lines[3].x,
        lines[3].y - 3.0,
        128.0,
        TEXT_BAR + 6.0,
    ));
    band(Rect::new(
        lines[4].x,
        lines[4].y - 3.0,
        62.0,
        TEXT_BAR + 6.0,
    ));
    if ink.state.interaction.is_engaged() {
        // The extension the pointer is dragging out.
        band(Rect::new(
            lines[4].x + 62.0,
            lines[4].y - 3.0,
            34.0,
            TEXT_BAR + 6.0,
        ));
    }
}

/// 15 — Selection fill across columns, pages and a table's cells.
pub fn selection_fill_across(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let left = Rect::new(CONTENT.x, CONTENT.y, 104.0, 62.0);
    let right = Rect::new(CONTENT.x + 122.0, CONTENT.y, 104.0, 62.0);
    let first = paragraph(canvas, pt(left.x, left.y + 6.0), &[100.0, 96.0, 88.0]);
    let second = paragraph(canvas, pt(right.x, right.y + 6.0), &[102.0, 92.0, 60.0]);
    canvas.line(
        None,
        pt(CONTENT.x + 113.0, CONTENT.y),
        pt(CONTENT.x + 113.0, CONTENT.y + 62.0),
        "column boundary",
        stroke(scale, canvas::with_alpha(ink.grid(), 0xcc), 0.5),
    );

    let table = Rect::new(CONTENT.x, CONTENT.y + 84.0, 226.0, 46.0);
    stage::gridlines(canvas, table, 4, 2);

    let fill = canvas.group(None, CONTENT, ink.selection_opacity(), None);
    let colour = ink.selection_colour();
    let mut band = |rect: Rect| {
        canvas.rect(Some(fill), rect, filled(colour));
    };
    band(Rect::new(
        first[1].x + 40.0,
        first[1].y - 3.0,
        60.0,
        TEXT_BAR + 6.0,
    ));
    band(Rect::new(
        first[2].x,
        first[2].y - 3.0,
        88.0,
        TEXT_BAR + 6.0,
    ));
    band(Rect::new(
        second[0].x,
        second[0].y - 3.0,
        102.0,
        TEXT_BAR + 6.0,
    ));
    band(stage::span(table, 4, 2, (1, 0), (2, 1)));
}

/// 16 — IME composition underline and the candidate-window anchor.
pub fn ime_composition(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let base = pt(CONTENT.x, CONTENT.y + 30.0);
    text_line(canvas, base, 58.0);
    let composition = Rect::new(base.x + 66.0, base.y, 138.0, TEXT_BAR);
    canvas.rect(
        None,
        composition,
        filled(canvas::with_alpha(ink.accent(), 0x44)),
    );
    // Two weights: the clause being converted is heavier than the rest of the composition.
    canvas.rect(
        None,
        Rect::new(composition.x, composition.bottom() + 3.0, 54.0, 0.75),
        filled(ink.accent()),
    );
    canvas.rect(
        None,
        Rect::new(composition.x + 54.0, composition.bottom() + 2.0, 44.0, 2.0),
        filled(ink.accent()),
    );
    canvas.rect(
        None,
        Rect::new(composition.x + 98.0, composition.bottom() + 3.0, 40.0, 0.75),
        filled(ink.accent()),
    );
    // The rectangle the candidate window anchors to.
    canvas.rect(
        None,
        Rect::new(
            composition.x + 50.0,
            composition.y - 4.0,
            52.0,
            TEXT_BAR + 10.0,
        ),
        dashed(scale, ink.accent(), ink.width(0.75), DashPattern::Dot),
    );
    if ink.state.interaction.is_engaged() {
        badge(
            canvas,
            pt(composition.x + 50.0, composition.bottom() + 12.0),
            78.0,
        );
    }
}

/// 17 — Spelling, grammar and style squiggles.
pub fn squiggles(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let lines = paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 18.0),
        &[212.0, 212.0, 212.0],
    );
    let kinds: [(mjx_scene::Color, f64, f64, f64); 3] = [
        (ink.deletion(), 3.0, 1.4, 1.0),
        (ink.insertion(), 4.5, 1.1, 0.8),
        (ink.warning(), 6.0, 0.9, 0.6),
    ];
    for (index, (colour, period, amplitude, width)) in kinds.into_iter().enumerate() {
        let line = lines[index];
        let start = pt(line.x + 24.0, line.bottom() + 3.5);
        canvas.path(
            None,
            Rect::new(
                start.x - 2.0,
                start.y - amplitude - 2.0,
                108.0,
                amplitude * 2.0 + 4.0,
            ),
            stage::squiggle(start, 104.0, period, amplitude),
            FillRule::NonZero,
            "squiggle",
            canvas::stroked(scale, colour, ink.width(width)),
        );
    }
}

/// 18 — Hyperlink hover affordance.
pub fn hyperlink_hover(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let lines = paragraph(canvas, pt(CONTENT.x, CONTENT.y + 22.0), &[210.0, 196.0]);

    let quiet = Rect::new(lines[0].x + 40.0, lines[0].y, 64.0, TEXT_BAR);
    canvas.rect(None, quiet, filled(ink.theme.accent));
    canvas.rect(
        None,
        Rect::new(quiet.x, quiet.bottom() + 2.0, quiet.width, 0.5),
        filled(canvas::with_alpha(ink.theme.accent, 0x88)),
    );

    let live = Rect::new(lines[1].x + 24.0, lines[1].y, 82.0, TEXT_BAR);
    let node = canvas.rect(None, live, filled(ink.accent()));
    canvas.grab(node, "hyperlink");
    canvas.rect(
        None,
        Rect::new(live.x, live.bottom() + 2.0, live.width, ink.width(0.75)),
        filled(ink.accent()),
    );
    if ink.state.interaction.is_engaged() {
        canvas.rect(
            None,
            Rect::new(live.x - 2.0, live.y - 2.0, live.width + 4.0, TEXT_BAR + 4.0),
            canvas::stroked(scale, ink.accent(), 0.75),
        );
        badge(canvas, pt(live.x, live.bottom() + 10.0), 96.0);
    }
}

/// 19 — Text-overflow and autofit indicators inside a shape.
pub fn text_overflow(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let shape = Rect::new(CONTENT.x + 8.0, CONTENT.y + 10.0, 148.0, 74.0);
    stage::object(canvas, None, shape);
    // The text is clipped by the shape, which is what "does not fit" means.
    let clipped = canvas.group(None, shape, 1.0, Some(shape));
    for index in 0..7 {
        let y = shape.y + 8.0 + (index as f64) * LINE_HEIGHT;
        canvas.rect(
            Some(clipped),
            Rect::new(shape.x + 8.0, y, 128.0, TEXT_BAR),
            filled(canvas::with_alpha(ink.text(), 0x66)),
        );
    }
    // The overflow marker on the bottom edge.
    for index in 0..3 {
        canvas.rect(
            None,
            Rect::new(
                shape.centre().x - 9.0 + (index as f64) * 7.0,
                shape.bottom() - 4.0,
                4.0,
                2.0,
            ),
            filled(ink.warning()),
        );
    }
    canvas.rect(
        None,
        shape,
        canvas::stroked(scale, ink.warning(), ink.width(1.0)),
    );
    // The autofit badge: the text was shrunk rather than clipped.
    badge(canvas, pt(shape.right() + 12.0, shape.y + 6.0), 62.0);
    canvas.polygon(
        None,
        &[
            pt(shape.right() + 12.0, shape.y + 34.0),
            pt(shape.right() + 22.0, shape.y + 34.0),
            pt(shape.right() + 17.0, shape.y + 42.0),
        ],
        "autofit arrow",
        filled(ink.warning()),
    );
}

/// 20 — Placeholder prompt text on an empty placeholder.
pub fn placeholder_prompt(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let title = Rect::new(CONTENT.x + 4.0, CONTENT.y + 6.0, 224.0, 34.0);
    let body = Rect::new(CONTENT.x + 4.0, CONTENT.y + 52.0, 224.0, 76.0);
    for (index, placeholder) in [title, body].into_iter().enumerate() {
        let focused = ink.state.interaction == Interaction::Focused && index == 1;
        canvas.rect(
            None,
            placeholder,
            dashed(
                scale,
                if focused { ink.accent() } else { ink.border() },
                if focused { ink.width(1.25) } else { 0.75 },
                DashPattern::Dash,
            ),
        );
    }
    canvas.rect(
        None,
        Rect::new(title.x + 10.0, title.centre().y - 3.0, 116.0, 6.0),
        filled(canvas::with_alpha(ink.muted(), 0x88)),
    );
    for index in 0..3 {
        canvas.rect(
            None,
            Rect::new(
                body.x + 10.0,
                body.y + 12.0 + (index as f64) * LINE_HEIGHT,
                150.0 - (index as f64) * 22.0,
                TEXT_BAR,
            ),
            filled(canvas::with_alpha(ink.muted(), 0x66)),
        );
    }
}

/// 21 — Drag-to-select autoscroll edge indicator.
pub fn autoscroll_edge(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 8.0),
        &[214.0, 208.0, 216.0, 190.0, 212.0, 176.0],
    );
    let band = Rect::new(
        crate::canvas::PAGE.right() - 34.0,
        crate::canvas::PAGE.y,
        34.0,
        crate::canvas::PAGE.height,
    );
    // A real `a:gradFill`, which is why this entry's content kind is `GradientFill` and why a
    // LibreOffice reference is excluded from it **by the provider** rather than by a list here.
    canvas.rect(
        None,
        band,
        Decoration::filled(FillStyle::Gradient(Gradient::linear(
            vec![
                GradientStop::new(0.0, canvas::with_alpha(ink.accent(), 0x00)),
                GradientStop::new(1.0, canvas::with_alpha(ink.accent(), 0x88)),
            ],
            0.0,
        ))),
    );
    let arrow = pt(band.right() - 10.0, band.centre().y);
    canvas.polygon(
        None,
        &[
            pt(arrow.x - 6.0, arrow.y - 8.0),
            pt(arrow.x + 4.0, arrow.y),
            pt(arrow.x - 6.0, arrow.y + 8.0),
        ],
        "autoscroll arrow",
        filled(ink.accent()),
    );
}
