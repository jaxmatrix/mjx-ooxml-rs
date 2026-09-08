//! §2.3 — grid selection, entries 22 to 30. Excel's own in-canvas UI.

use mjx_scene::{DashPattern, FillRule};

use crate::canvas::{self, dashed, filled, filled_and_stroked, pt, stroke, Canvas, Rect};
use crate::scenes::stage::{self, badge, cell, gridlines, span};
use crate::state::Interaction;

/// The sheet every scene in this family is drawn on.
const SHEET: Rect = Rect {
    x: 40.0,
    y: 34.0,
    width: 220.0,
    height: 132.0,
};

/// How many columns and rows it has.
const COLUMNS: usize = 5;
/// Rows.
const ROWS: usize = 5;

/// 22 — Active-cell border, distinct from the range border.
pub fn active_cell_border(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    gridlines(canvas, SHEET, COLUMNS, ROWS);
    let range = span(SHEET, COLUMNS, ROWS, (1, 1), (3, 3));
    let fill = canvas.group(None, range, ink.selection_opacity(), None);
    canvas.rect(Some(fill), range, filled(ink.selection_colour()));
    canvas.rect(
        None,
        range,
        canvas::stroked(scale, ink.accent(), ink.width(1.0)),
    );
    // The active cell: the same colour, a heavier weight, and no fill — which is how Excel says
    // *"typing goes here"* without saying *"this is a second selection"*.
    let active = cell(SHEET, COLUMNS, ROWS, 2, 2);
    canvas.rect(None, active, filled(ink.page()));
    canvas.rect(
        None,
        active,
        canvas::stroked(scale, ink.accent(), ink.width(2.0)),
    );
}

/// 23 — Range selection fill and border.
pub fn range_selection(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    gridlines(canvas, SHEET, COLUMNS, ROWS);
    let range = span(SHEET, COLUMNS, ROWS, (1, 1), (3, 3));
    let fill = canvas.group(None, range, ink.selection_opacity(), None);
    canvas.rect(Some(fill), range, filled(ink.selection_colour()));
    canvas.rect(
        None,
        range,
        canvas::stroked(scale, ink.accent(), ink.width(1.25)),
    );
    // The gridlines again, over the fill, because a selection that hides the grid it selects is a
    // selection you cannot count cells in.
    gridlines(canvas, range, 3, 3);
}

/// 24 — Multi-range (discontiguous) selection.
pub fn multi_range_selection(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    gridlines(canvas, SHEET, COLUMNS, ROWS);
    let ranges = [
        span(SHEET, COLUMNS, ROWS, (0, 0), (1, 1)),
        // Adjacent to the first, which is the case the judgement is about.
        span(SHEET, COLUMNS, ROWS, (2, 0), (2, 1)),
        span(SHEET, COLUMNS, ROWS, (3, 3), (4, 4)),
    ];
    let opacity = ink.selection_opacity();
    let colour = ink.selection_colour();
    for range in ranges {
        let fill = canvas.group(None, range, opacity, None);
        canvas.rect(Some(fill), range, filled(colour));
    }
    for range in ranges {
        canvas.rect(
            None,
            range,
            canvas::stroked(scale, ink.accent(), ink.width(1.25)),
        );
    }
}

/// 25 — Fill handle, and the fill preview with its series tooltip.
pub fn fill_handle(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    gridlines(canvas, SHEET, COLUMNS, ROWS);
    let range = span(SHEET, COLUMNS, ROWS, (1, 1), (2, 2));
    let fill = canvas.group(None, range, ink.selection_opacity(), None);
    canvas.rect(Some(fill), range, filled(ink.selection_colour()));
    canvas.rect(
        None,
        range,
        canvas::stroked(scale, ink.accent(), ink.width(1.25)),
    );

    // The smallest grab target in the inventory: Excel draws it at about three points square,
    // whatever the pointer is. This harness sizes it from the input device instead, which is the
    // proposal the audit is meant to accept or reject.
    let grip = ink.affordance() * 0.55;
    let node = canvas.rect(
        None,
        Rect::centred(pt(range.right(), range.bottom()), grip),
        filled_and_stroked(scale, ink.accent(), ink.page(), 0.6),
    );
    canvas.grab(node, "fill handle");

    if ink.state.interaction == Interaction::Active {
        let preview = span(SHEET, COLUMNS, ROWS, (1, 3), (2, 4));
        canvas.rect(
            None,
            preview,
            dashed(scale, ink.accent(), ink.width(1.0), DashPattern::Dash),
        );
        badge(canvas, pt(preview.right() + 6.0, preview.y), 52.0);
    }
}

/// 26 — Row and column header highlight for the current selection.
pub fn header_highlight(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(
        SHEET.x + 22.0,
        SHEET.y + 14.0,
        SHEET.width - 22.0,
        SHEET.height - 14.0,
    );
    let column_header = Rect::new(body.x, SHEET.y, body.width, 14.0);
    let row_header = Rect::new(SHEET.x, body.y, 22.0, body.height);
    canvas.rect(None, column_header, filled(ink.theme.surface));
    canvas.rect(None, row_header, filled(ink.theme.surface));
    gridlines(canvas, body, 4, 4);
    gridlines(canvas, column_header, 4, 1);
    gridlines(canvas, row_header, 1, 4);

    let selected_columns = span(column_header, 4, 1, (1, 0), (2, 0));
    let selected_rows = span(row_header, 1, 4, (0, 1), (0, 2));
    for header in [selected_columns, selected_rows] {
        canvas.rect(None, header, filled(ink.accent_surface()));
        canvas.rect(
            None,
            header,
            canvas::stroked(scale, ink.accent(), ink.width(0.75)),
        );
    }
    let range = span(body, 4, 4, (1, 1), (2, 2));
    let fill = canvas.group(None, range, ink.selection_opacity(), None);
    canvas.rect(Some(fill), range, filled(ink.selection_colour()));
    canvas.rect(
        None,
        range,
        canvas::stroked(scale, ink.accent(), ink.width(1.25)),
    );
}

/// 27 — Frozen and split pane divider lines.
pub fn pane_dividers(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    gridlines(canvas, SHEET, COLUMNS, ROWS);

    // The frozen divider: one dark rule, and nothing to grab — a frozen pane is moved from the
    // ribbon, never by dragging.
    let frozen = SHEET.x + SHEET.width / f64::from(COLUMNS as u32) * 2.0;
    canvas.line(
        None,
        pt(frozen, SHEET.y),
        pt(frozen, SHEET.bottom()),
        "frozen divider",
        stroke(scale, ink.text(), 1.0),
    );

    // The split divider: a wider band, and it *is* draggable, which is why it has a grab region and
    // the frozen one does not.
    let split = SHEET.y + SHEET.height / f64::from(ROWS as u32) * 3.0;
    let band = Rect::new(
        SHEET.x,
        split - ink.affordance() / 4.0,
        SHEET.width,
        ink.affordance() / 2.0,
    );
    let node = canvas.rect(
        None,
        band,
        filled_and_stroked(scale, ink.theme.surface, ink.border(), 0.5),
    );
    canvas.grab(node, "split divider");
    canvas.line(
        None,
        pt(band.x + band.width * 0.42, band.centre().y),
        pt(band.x + band.width * 0.58, band.centre().y),
        "split grip",
        stroke(scale, ink.muted(), 1.0),
    );
}

/// 28 — Merged-cell selection behaviour.
pub fn merged_cell_selection(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    gridlines(canvas, SHEET, COLUMNS, ROWS);
    let merged = span(SHEET, COLUMNS, ROWS, (1, 1), (2, 2));
    // The merge has no interior gridlines: it is one cell, so it is painted over.
    canvas.rect(None, merged, filled(ink.page()));
    canvas.rect(None, merged, canvas::stroked(scale, ink.grid(), 0.5));

    let range = span(SHEET, COLUMNS, ROWS, (1, 1), (3, 3));
    let fill = canvas.group(None, range, ink.selection_opacity(), None);
    canvas.rect(Some(fill), range, filled(ink.selection_colour()));
    canvas.rect(
        None,
        range,
        canvas::stroked(scale, ink.accent(), ink.width(1.25)),
    );
    // The active cell inside a range containing a merge is the merge as a whole.
    canvas.rect(
        None,
        merged,
        canvas::stroked(scale, ink.accent(), ink.width(2.0)),
    );
}

/// 29 — AutoFilter dropdown affordance in a header cell.
pub fn autofilter_dropdown(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let header = Rect::new(SHEET.x, SHEET.y, SHEET.width, 22.0);
    canvas.rect(None, header, filled(ink.theme.surface));
    gridlines(canvas, header, 3, 1);
    gridlines(
        canvas,
        Rect::new(SHEET.x, header.bottom(), SHEET.width, SHEET.height - 22.0),
        3,
        4,
    );

    for column in 0..2 {
        let host = cell(header, 3, 1, column, 0);
        let side = ink.affordance();
        let button = Rect::new(
            host.right() - side - 3.0,
            host.centre().y - side / 2.0,
            side,
            side,
        );
        let node = canvas.path(
            None,
            button,
            stage::rounded_rectangle(button, 2.5),
            FillRule::NonZero,
            "dropdown button",
            filled_and_stroked(
                scale,
                if column == 1 {
                    ink.accent_surface()
                } else {
                    ink.page()
                },
                ink.border(),
                0.6,
            ),
        );
        canvas.grab(node, "autofilter dropdown");
        if column == 1 {
            // The funnel: a filtered column says so on its own face.
            canvas.polygon(
                None,
                &[
                    pt(button.x + 2.0, button.y + 2.5),
                    pt(button.right() - 2.0, button.y + 2.5),
                    pt(button.centre().x + 1.0, button.centre().y + 0.5),
                    pt(button.centre().x + 1.0, button.bottom() - 2.0),
                    pt(button.centre().x - 1.0, button.bottom() - 3.0),
                    pt(button.centre().x - 1.0, button.centre().y + 0.5),
                ],
                "filter funnel",
                filled(ink.accent()),
            );
        } else {
            stage::chevron(
                canvas,
                pt(button.centre().x, button.centre().y + 1.5),
                2.5,
                true,
                "dropdown chevron",
                ink.accent(),
                ink.width(1.1),
            );
        }
    }
}

/// 30 — Comment / note indicator triangle.
pub fn comment_indicator(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    gridlines(canvas, SHEET, COLUMNS, ROWS);
    let noted = cell(SHEET, COLUMNS, ROWS, 1, 1);
    let side = if ink.is_touch() { 6.0 } else { 4.0 };
    stage::triangle(
        canvas,
        pt(noted.right() - side, noted.y),
        side,
        (1.0, 1.0),
        "note indicator",
        ink.comment(),
    );
    // The ladder: the same triangle at three sizes, so that a reviewer at 1× can say where it stops
    // being a triangle rather than only whether the platform's size works.
    for (index, size) in [3.0_f64, 4.0, 6.0].into_iter().enumerate() {
        let host = cell(SHEET, COLUMNS, ROWS, 3, index + 1);
        stage::triangle(
            canvas,
            pt(host.right() - size, host.y),
            size,
            (1.0, 1.0),
            "note indicator",
            ink.comment(),
        );
    }
}
