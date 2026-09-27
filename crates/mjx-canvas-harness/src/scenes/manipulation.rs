//! §2.4 — manipulation, entries 31 to 43. Everything that only exists while something is being
//! moved, and therefore the family least well served by a still picture: the harness is where these
//! are judged, and the plates only lock them once they are.

use mjx_scene::{DashPattern, FillRule};

use crate::canvas::{
    self, dashed, dashed_stroke, filled, filled_and_stroked, pt, stroke, Canvas, Pt, Rect, PAGE,
};
use crate::scenes::stage::{
    self, badge, cell, gridlines, handle, object, resting_handle, round_handle, selection_outline,
    span, CONTENT,
};
use crate::state::Interaction;

/// 31 — Drag ghost / translucent preview.
pub fn drag_ghost(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let origin = Rect::new(56.0, 70.0, 92.0, 58.0);
    object(canvas, None, origin);
    canvas.rect(
        None,
        origin,
        dashed(scale, ink.muted(), 0.75, DashPattern::Dash),
    );
    let carried = origin.offset(96.0, 26.0);
    let ghost = canvas.group(
        None,
        carried,
        if ink.state.interaction == Interaction::Active {
            0.55
        } else {
            0.4
        },
        None,
    );
    object(canvas, Some(ghost), carried);
    selection_outline(canvas, carried.inflated(2.0), 1.0);
}

/// 32 — Smart alignment guides: edge, centre and equal spacing.
pub fn alignment_guides(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let bodies = [
        Rect::new(44.0, 52.0, 52.0, 40.0),
        Rect::new(124.0, 52.0, 52.0, 40.0),
        Rect::new(204.0, 52.0, 52.0, 40.0),
    ];
    for body in bodies {
        object(canvas, None, body);
    }
    let dragged = Rect::new(124.0, 116.0, 52.0, 40.0);
    object(canvas, None, dragged);
    selection_outline(canvas, dragged.inflated(2.0), 1.0);

    // The edge guide: the left edges line up.
    canvas.line(
        None,
        pt(dragged.x, PAGE.y + 4.0),
        pt(dragged.x, PAGE.bottom() - 4.0),
        "edge guide",
        stroke(scale, ink.guide(), ink.width(0.75)),
    );
    // The centre guide: the dragged object's centre matches the middle one's.
    canvas.line(
        None,
        pt(PAGE.x + 4.0, dragged.centre().y),
        pt(PAGE.right() - 4.0, dragged.centre().y),
        "centre guide",
        stroke(scale, ink.guide(), ink.width(0.75)),
    );
    // The equal-spacing marks: two gaps, called out as the same.
    for gap in [
        (bodies[0].right(), bodies[1].x),
        (bodies[1].right(), bodies[2].x),
    ] {
        let y = bodies[0].bottom() + 8.0;
        canvas.line(
            None,
            pt(gap.0, y),
            pt(gap.1, y),
            "equal spacing",
            stroke(scale, ink.guide(), ink.width(0.75)),
        );
        for x in [gap.0, gap.1] {
            canvas.line(
                None,
                pt(x, y - 3.5),
                pt(x, y + 3.5),
                "spacing tick",
                stroke(scale, ink.guide(), ink.width(0.75)),
            );
        }
    }
}

/// 33 — Snap indicators: to grid, to guide, to object.
pub fn snap_indicators(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let sheet = Rect::new(CONTENT.x, CONTENT.y, 224.0, 132.0);
    // The grid the first kind snaps to.
    for column in 0..=14 {
        for row in 0..=8 {
            let at = pt(
                sheet.x + f64::from(column) * 16.0,
                sheet.y + f64::from(row) * 16.0,
            );
            canvas.rect(
                None,
                Rect::centred(at, 1.0),
                filled(canvas::with_alpha(ink.grid(), 0xcc)),
            );
        }
    }
    // The guide the second kind snaps to.
    let guide_x = sheet.x + 128.0;
    canvas.line(
        None,
        pt(guide_x, sheet.y),
        pt(guide_x, sheet.bottom()),
        "slide guide",
        dashed_stroke(scale, ink.guide(), 0.6, DashPattern::Dash),
    );
    let neighbour = Rect::new(sheet.x + 158.0, sheet.y + 24.0, 54.0, 40.0);
    object(canvas, None, neighbour);

    let dragged = Rect::new(sheet.x + 48.0, sheet.y + 56.0, 64.0, 44.0);
    object(canvas, None, dragged);
    selection_outline(canvas, dragged.inflated(2.0), 1.0);

    // Three markers, one per kind, deliberately three different shapes: a cross for the grid, a
    // bar for the guide, a ring for the object. A reviewer who cannot say which snap engaged is
    // looking at a design defect rather than at a rendering one.
    let cross = pt(dragged.x, dragged.y);
    canvas.line(
        None,
        pt(cross.x - 5.0, cross.y),
        pt(cross.x + 5.0, cross.y),
        "grid snap",
        stroke(scale, ink.accent(), ink.width(1.0)),
    );
    canvas.line(
        None,
        pt(cross.x, cross.y - 5.0),
        pt(cross.x, cross.y + 5.0),
        "grid snap",
        stroke(scale, ink.accent(), ink.width(1.0)),
    );
    canvas.rect(
        None,
        Rect::new(guide_x - 1.5, dragged.centre().y - 9.0, 3.0, 18.0),
        filled(ink.accent()),
    );
    canvas.circle(
        None,
        pt(neighbour.x, neighbour.bottom()),
        5.0,
        "object snap",
        canvas::stroked(scale, ink.accent(), ink.width(1.25)),
    );
}

/// 34 — Live dimension and position readout during a drag.
pub fn drag_readout(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(122.0, 84.0, 88.0, 58.0);
    object(canvas, None, body);
    selection_outline(canvas, body.inflated(2.0), 1.0);
    // The leaders to the edges the numbers are measured from.
    canvas.line(
        None,
        pt(PAGE.x, body.centre().y),
        pt(body.x, body.centre().y),
        "left leader",
        dashed_stroke(scale, ink.guide(), 0.6, DashPattern::Dot),
    );
    canvas.line(
        None,
        pt(body.centre().x, PAGE.y),
        pt(body.centre().x, body.y),
        "top leader",
        dashed_stroke(scale, ink.guide(), 0.6, DashPattern::Dot),
    );
    badge(canvas, pt(body.right() + 8.0, body.y - 4.0), 70.0);
    badge(canvas, pt(body.right() + 8.0, body.y + 12.0), 70.0);
    if ink.state.interaction == Interaction::Active {
        badge(canvas, pt(body.right() + 8.0, body.y + 28.0), 70.0);
    }
}

/// 35 — Resize ghost with its dimension readout and aspect-lock state.
pub fn resize_ghost(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let original = Rect::new(62.0, 62.0, 88.0, 58.0);
    object(canvas, None, original);
    canvas.rect(
        None,
        original,
        dashed(scale, ink.muted(), 0.75, DashPattern::Dash),
    );
    let target = Rect::new(original.x, original.y, 132.0, 87.0);
    let ghost = canvas.group(None, target, 0.42, None);
    object(canvas, Some(ghost), target);
    selection_outline(canvas, target, 1.0);
    for (index, centre) in target.handles().into_iter().enumerate() {
        if index == 4 {
            handle(canvas, centre, "south-east");
        } else {
            resting_handle(canvas, centre, "corner");
        }
    }
    badge(
        canvas,
        pt(target.right() + 8.0, target.bottom() - 12.0),
        62.0,
    );
    // The aspect lock: two links, joined.
    let lock = pt(target.right() + 8.0, target.bottom() + 8.0);
    for offset in [0.0, 6.0] {
        canvas.path(
            None,
            Rect::new(lock.x + offset, lock.y, 9.0, 7.0),
            stage::rounded_rectangle(Rect::new(lock.x + offset, lock.y, 9.0, 7.0), 3.0),
            FillRule::NonZero,
            "aspect lock",
            canvas::stroked(scale, ink.accent(), ink.width(0.9)),
        );
    }
}

/// 36 — Crop handles and the out-of-crop darkening overlay.
pub fn crop_handles(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let picture = Rect::new(52.0, 46.0, 196.0, 108.0);
    object(canvas, None, picture);
    // Something inside the picture, so the darkening has something to darken.
    for index in 0..5 {
        canvas.rect(
            None,
            Rect::new(
                picture.x + 10.0 + f64::from(index) * 36.0,
                picture.y + 14.0,
                26.0,
                80.0 - f64::from(index) * 12.0,
            ),
            filled(canvas::with_alpha(ink.object_border(), 0x77)),
        );
    }
    let crop = Rect::new(picture.x + 34.0, picture.y + 18.0, 124.0, 70.0);
    // The darkening: one group, four rectangles, so the discarded area is dimmed exactly once.
    let dim = canvas.group(None, picture, 0.55, None);
    let shade = canvas::with_alpha(ink.text(), 0xff);
    for band in [
        Rect::new(picture.x, picture.y, picture.width, crop.y - picture.y),
        Rect::new(
            picture.x,
            crop.bottom(),
            picture.width,
            picture.bottom() - crop.bottom(),
        ),
        Rect::new(picture.x, crop.y, crop.x - picture.x, crop.height),
        Rect::new(
            crop.right(),
            crop.y,
            picture.right() - crop.right(),
            crop.height,
        ),
    ] {
        canvas.rect(Some(dim), band, filled(shade));
    }
    canvas.rect(
        None,
        crop,
        canvas::stroked(scale, ink.page(), ink.width(1.0)),
    );
    // The eight crop handles, each an L rather than a square: a crop handle that looked like a
    // resize handle would be a crop that resizes the picture.
    let arm = ink.affordance();
    for (index, corner) in crop.handles().into_iter().enumerate() {
        let (dx, dy) = match index {
            0 => (1.0, 1.0),
            1 => (0.0, 1.0),
            2 => (-1.0, 1.0),
            3 => (-1.0, 0.0),
            4 => (-1.0, -1.0),
            5 => (0.0, -1.0),
            6 => (1.0, -1.0),
            _ => (1.0, 0.0),
        };
        let node = canvas.polyline(
            None,
            &[
                pt(corner.x + dx * arm, corner.y),
                corner,
                pt(corner.x, corner.y + dy * arm),
            ],
            false,
            "crop handle",
            stroke(scale, ink.page(), ink.width(2.2)),
        );
        canvas.grab(node, "crop handle");
    }
}

/// 37 — Table row and column resize affordance with its preview line.
pub fn table_resize(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let table = Rect::new(CONTENT.x, CONTENT.y + 8.0, 226.0, 112.0);
    gridlines(canvas, table, 4, 4);
    let boundary = cell(table, 4, 4, 1, 0).right();
    // The affordance: a band on the boundary, sized for the input device.
    let band = Rect::new(
        boundary - ink.affordance() / 2.0,
        table.y,
        ink.affordance(),
        table.height,
    );
    let node = canvas.rect(None, band, filled(canvas::with_alpha(ink.accent(), 0x33)));
    canvas.grab(node, "column boundary");
    canvas.line(
        None,
        pt(boundary, table.y - 6.0),
        pt(boundary, table.bottom() + 6.0),
        "resize preview",
        stroke(scale, ink.accent(), ink.width(1.25)),
    );
    // The double-bar grip, which is what says *resize* rather than *select*.
    for offset in [-2.0_f64, 2.0] {
        canvas.line(
            None,
            pt(boundary + offset, table.y - 12.0),
            pt(boundary + offset, table.y - 5.0),
            "resize grip",
            stroke(scale, ink.accent(), ink.width(1.0)),
        );
    }
}

/// 38 — Table insert affordance, the between-rows/columns control.
pub fn table_insert(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let table = Rect::new(CONTENT.x, CONTENT.y + 20.0, 226.0, 100.0);
    gridlines(canvas, table, 4, 3);
    let boundary = cell(table, 4, 3, 1, 0).right();
    let centre = pt(boundary, table.y - 10.0);
    let radius = ink.affordance() * 0.62;
    let node = canvas.circle(
        None,
        centre,
        radius,
        "insert control",
        filled_and_stroked(scale, ink.page(), ink.accent(), ink.width(1.0)),
    );
    canvas.grab(node, "insert column");
    canvas.line(
        None,
        pt(centre.x - radius * 0.5, centre.y),
        pt(centre.x + radius * 0.5, centre.y),
        "plus",
        stroke(scale, ink.accent(), ink.width(1.1)),
    );
    canvas.line(
        None,
        pt(centre.x, centre.y - radius * 0.5),
        pt(centre.x, centre.y + radius * 0.5),
        "plus",
        stroke(scale, ink.accent(), ink.width(1.1)),
    );
    // The line the insertion would land on.
    canvas.line(
        None,
        pt(boundary, table.y),
        pt(boundary, table.bottom()),
        "insertion line",
        stroke(scale, ink.accent(), ink.width(1.6)),
    );
}

/// 39 — Custom-geometry vertex editing.
pub fn vertex_editing(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let anchors = [
        pt(56.0, 130.0),
        pt(126.0, 58.0),
        pt(200.0, 132.0),
        pt(252.0, 74.0),
    ];
    let controls = [pt(88.0, 76.0), pt(160.0, 60.0), pt(230.0, 138.0)];
    let bounds = Rect::new(44.0, 44.0, 224.0, 108.0);
    let point = |at: Pt| mjx_scene::ScenePoint::new(at.x as f32, at.y as f32);
    canvas.path(
        None,
        bounds,
        vec![
            mjx_scene::PathCommand::MoveTo(point(anchors[0])),
            mjx_scene::PathCommand::QuadraticTo {
                control: point(controls[0]),
                end: point(anchors[1]),
            },
            mjx_scene::PathCommand::QuadraticTo {
                control: point(controls[1]),
                end: point(anchors[2]),
            },
            mjx_scene::PathCommand::QuadraticTo {
                control: point(controls[2]),
                end: point(anchors[3]),
            },
        ],
        FillRule::NonZero,
        "custom geometry",
        canvas::stroked(scale, ink.object_border(), 1.25),
    );
    // The tangents: what a control point belongs to.
    for (index, control) in controls.into_iter().enumerate() {
        for anchor in [anchors[index], anchors[index + 1]] {
            canvas.line(
                None,
                anchor,
                control,
                "tangent",
                stroke(scale, canvas::with_alpha(ink.guide(), 0xcc), 0.5),
            );
        }
    }
    // Square vertices, round control points — the two must never be the same shape.
    for (index, anchor) in anchors.into_iter().enumerate() {
        if index == 1 {
            handle(canvas, anchor, "selected vertex");
        } else {
            resting_handle(canvas, anchor, "vertex");
        }
    }
    for control in controls {
        round_handle(canvas, control, "control point");
    }
}

/// 40 — Motion-path editing handles and the path preview.
pub fn motion_path_editing(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let start = Rect::new(52.0, 108.0, 58.0, 40.0);
    object(canvas, None, start);
    let finish = start.offset(148.0, -58.0);
    let ghost = canvas.group(None, finish, 0.38, None);
    object(canvas, Some(ghost), finish);

    let from = start.centre();
    let to = finish.centre();
    let point = |at: Pt| mjx_scene::ScenePoint::new(at.x as f32, at.y as f32);
    canvas.path(
        None,
        Rect::new(
            from.x - 6.0,
            to.y - 30.0,
            to.x - from.x + 12.0,
            from.y - to.y + 40.0,
        ),
        vec![
            mjx_scene::PathCommand::MoveTo(point(from)),
            mjx_scene::PathCommand::CubicTo {
                first_control: point(pt(from.x + 40.0, from.y - 60.0)),
                second_control: point(pt(to.x - 40.0, to.y + 60.0)),
                end: point(to),
            },
        ],
        FillRule::NonZero,
        "motion path",
        canvas::dashed(scale, ink.accent(), ink.width(1.0), DashPattern::Dash),
    );
    // A green triangle at the start and a red square at the end, which is PowerPoint's own
    // vocabulary and the one a reviewer will already know.
    canvas.polygon(
        None,
        &[
            pt(from.x - 5.0, from.y - 5.0),
            pt(from.x + 5.0, from.y),
            pt(from.x - 5.0, from.y + 5.0),
        ],
        "path start",
        filled(ink.insertion()),
    );
    let node = canvas.rect(
        None,
        Rect::centred(to, ink.affordance()),
        filled_and_stroked(scale, ink.deletion(), ink.page(), 0.75),
    );
    canvas.grab(node, "path end");
}

/// 41 — Marching ants, and the phase the animation steps through.
pub fn marching_ants(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let sheet = Rect::new(CONTENT.x, CONTENT.y + 8.0, 226.0, 116.0);
    gridlines(canvas, sheet, 5, 4);
    let cut = span(sheet, 5, 4, (1, 1), (3, 2));

    // **The ants are drawn one at a time rather than as a dash pattern**, because a dash pattern
    // has no phase and a phase is the whole of what a cadence is. The interaction toggle steps it,
    // which is how a still plate can say anything about an animation at all.
    let phase = match ink.state.interaction {
        Interaction::Default => 0.0,
        Interaction::Hover => 1.5,
        Interaction::Active => 3.0,
        Interaction::Focused => 4.5,
        Interaction::Disabled => 6.0,
    };
    let ant = 4.0_f64;
    let period = 8.0_f64;
    let width = ink.width(1.25);
    let along = |from: Pt, to: Pt, canvas: &mut Canvas| {
        let length = ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt();
        let (ux, uy) = ((to.x - from.x) / length, (to.y - from.y) / length);
        let mut at = phase % period;
        while at < length {
            let end = (at + ant).min(length);
            canvas.rect(
                None,
                Rect::new(
                    from.x + ux * at - if uy.abs() > 0.5 { width / 2.0 } else { 0.0 },
                    from.y + uy * at - if ux.abs() > 0.5 { width / 2.0 } else { 0.0 },
                    if ux.abs() > 0.5 { end - at } else { width },
                    if uy.abs() > 0.5 { end - at } else { width },
                ),
                filled(ink.accent()),
            );
            at += period;
        }
    };
    let corners = [
        (pt(cut.x, cut.y), pt(cut.right(), cut.y)),
        (pt(cut.right(), cut.y), pt(cut.right(), cut.bottom())),
        (pt(cut.right(), cut.bottom()), pt(cut.x, cut.bottom())),
        (pt(cut.x, cut.bottom()), pt(cut.x, cut.y)),
    ];
    for (from, to) in corners {
        along(from, to, canvas);
    }
}

/// 42 — Drop indicator: between paragraphs, between slides, into a cell.
pub fn drop_indicator(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();

    // Between paragraphs: a caret bar with serifs, across the text measure.
    let lines = stage::paragraph(canvas, pt(CONTENT.x, CONTENT.y), &[120.0, 104.0, 116.0]);
    let between = lines[1].bottom() + 4.0;
    canvas.rect(
        None,
        Rect::new(lines[0].x, between - ink.width(1.0), 120.0, ink.width(2.0)),
        filled(ink.accent()),
    );
    for x in [lines[0].x, lines[0].x + 120.0] {
        canvas.rect(
            None,
            Rect::new(x - 1.0, between - 4.0, 2.0, 8.0),
            filled(ink.accent()),
        );
    }

    // Between slides: a taller bar beside two thumbnails.
    for (index, at) in [166.0_f64, 224.0].into_iter().enumerate() {
        let thumbnail = Rect::new(at, CONTENT.y, 46.0, 32.0);
        canvas.rect(
            None,
            thumbnail,
            filled_and_stroked(scale, ink.page(), ink.border(), 0.6),
        );
        if index == 0 {
            canvas.rect(
                None,
                Rect::new(
                    thumbnail.right() + 4.0,
                    thumbnail.y - 3.0,
                    ink.width(2.5),
                    thumbnail.height + 6.0,
                ),
                filled(ink.accent()),
            );
        }
    }

    // Into a cell: a ring around the target rather than a bar, because a cell is dropped *into*.
    let sheet = Rect::new(CONTENT.x, CONTENT.y + 66.0, 226.0, 62.0);
    gridlines(canvas, sheet, 4, 2);
    let target = cell(sheet, 4, 2, 2, 1);
    canvas.rect(
        None,
        target.inflated(-1.0),
        canvas::stroked(scale, ink.accent(), ink.width(2.0)),
    );
}

/// 43 — Multi-touch transform feedback: pinch scale and two-finger rotate.
pub fn multi_touch_transform(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(96.0, 66.0, 108.0, 70.0);
    let turned = object(canvas, None, body);
    canvas.rotate(
        turned,
        if ink.state.interaction == Interaction::Active {
            18.0
        } else {
            6.0
        },
    );
    selection_outline(canvas, body.inflated(3.0), 1.0);

    // The two contacts. A finger's contact patch is about eleven millimetres — roughly thirty-one
    // points — and drawing it at that size rather than as a dot is the only honest way to judge
    // whether anything under it can be seen.
    let contacts = [
        pt(body.x + 14.0, body.y + 16.0),
        pt(body.right() - 14.0, body.bottom() - 16.0),
    ];
    let patch = if ink.is_touch() { 31.0 } else { 12.0 };
    for contact in contacts {
        let node = canvas.circle(
            None,
            contact,
            patch / 2.0,
            "touch contact",
            filled(canvas::with_alpha(ink.accent(), 0x38)),
        );
        canvas.grab(node, "touch contact");
        canvas.circle(
            None,
            contact,
            patch / 2.0,
            "touch contact edge",
            canvas::stroked(scale, ink.accent(), ink.width(0.9)),
        );
    }
    // The pinch axis.
    canvas.line(
        None,
        contacts[0],
        contacts[1],
        "pinch axis",
        dashed_stroke(scale, ink.accent(), ink.width(0.9), DashPattern::Dash),
    );
    badge(canvas, pt(body.right() + 8.0, body.y), 58.0);
}
