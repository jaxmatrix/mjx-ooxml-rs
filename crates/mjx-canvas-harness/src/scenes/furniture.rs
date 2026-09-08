//! §2.5 — document furniture, entries 44 to 56. The chrome that belongs to the *document* rather
//! than to the selection: it is on the page whether or not anything is selected, and most of it is
//! wrong when it is louder than the words.

use mjx_scene::{BlendMode, DashPattern, Decoration, EffectKind, EffectStyle, FillRule, FillStyle};

use crate::canvas::{
    self, dashed, dashed_stroke, filled, filled_and_stroked, pt, stroke, Canvas, Rect, PAGE,
};
use crate::scenes::stage::{
    self, badge, object, paragraph, text_line, CONTENT, LINE_HEIGHT, TEXT_BAR,
};
use crate::state::Interaction;

/// 44 — Canvas backdrop, page fill, page border and page shadow.
pub fn page_and_shadow(canvas: &mut Canvas) {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    canvas.rect(
        None,
        Rect::new(0.0, 0.0, canvas::STAGE.0, canvas::STAGE.1),
        filled(ink.backdrop()),
    );
    // **A real `a:outerShdw` through the effect pipeline**, at the numbers the
    // `document.*.page-shadow` token states, rather than a grey rectangle drawn behind the page. A
    // shadow faked as a rectangle is a shadow that cannot be judged: it has no blur, so the only
    // question the audit could ask about it — *"is the falloff right?"* — would have no answer.
    let shadow = &ink.document.page_shadow;
    let to_device = |css_pixels: f32| css_pixels * 0.75 * scale;
    let (offset_x, offset_y) = (
        to_device(shadow.offset_x.to_pixels(16.0)),
        to_device(shadow.offset_y.to_pixels(16.0)),
    );
    let page = Rect::new(
        PAGE.x + 6.0,
        PAGE.y + 6.0,
        PAGE.width - 12.0,
        PAGE.height - 18.0,
    );
    let decoration = Decoration {
        fill: FillStyle::Solid(ink.page()),
        stroke: Some(stroke(scale, ink.page_border(), 0.75)),
        opacity: 1.0,
        effects: vec![EffectStyle {
            fill: FillStyle::Solid(shadow.color),
            radius: to_device(shadow.blur.to_pixels(16.0)),
            distance: offset_x.hypot(offset_y),
            direction: offset_y.atan2(offset_x),
            grow: true,
            blend: BlendMode::Over,
            ..EffectStyle::new(EffectKind::OuterShadow)
        }],
    };
    canvas.rect(None, page, decoration);
    // A page with nothing on it is a page nobody can judge a shadow against.
    paragraph(
        canvas,
        pt(page.x + 24.0, page.y + 26.0),
        &[188.0, 196.0, 172.0, 190.0],
    );
}

/// 45 — Page gap and the page-break indicator.
pub fn page_gap_and_break(canvas: &mut Canvas) {
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    canvas.rect(
        None,
        Rect::new(0.0, 0.0, canvas::STAGE.0, canvas::STAGE.1),
        filled(ink.backdrop()),
    );
    let first = Rect::new(PAGE.x + 20.0, 8.0, PAGE.width - 40.0, 86.0);
    let second = Rect::new(first.x, first.bottom() + 14.0, first.width, 86.0);
    for page in [first, second] {
        canvas.rect(
            None,
            page,
            filled_and_stroked(scale, ink.page(), ink.page_border(), 0.75),
        );
    }
    paragraph(
        canvas,
        pt(first.x + 18.0, first.y + 20.0),
        &[172.0, 180.0, 158.0, 176.0],
    );
    paragraph(
        canvas,
        pt(second.x + 18.0, second.y + 46.0),
        &[176.0, 148.0],
    );

    // The explicit page break, in the second page's own body: a rule with a gap in the middle.
    let rule = second.y + 30.0;
    for segment in [
        (second.x + 18.0, second.centre().x - 26.0),
        (second.centre().x + 26.0, second.right() - 18.0),
    ] {
        canvas.line(
            None,
            pt(segment.0, rule),
            pt(segment.1, rule),
            "page break",
            dashed_stroke(scale, ink.muted(), 0.6, DashPattern::Dash),
        );
    }
    // The collapsed form, which is what the same break looks like with marks turned off.
    canvas.line(
        None,
        pt(first.x + 18.0, first.bottom() - 12.0),
        pt(first.right() - 18.0, first.bottom() - 12.0),
        "collapsed break",
        stroke(scale, canvas::with_alpha(ink.muted(), 0x55), 0.5),
    );
}

/// 46 — Header and footer dimmed regions with their boundaries.
pub fn header_footer_regions(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let header = Rect::new(PAGE.x, PAGE.y, PAGE.width, 34.0);
    let footer = Rect::new(PAGE.x, PAGE.bottom() - 30.0, PAGE.width, 30.0);
    let body = Rect::new(
        PAGE.x,
        header.bottom(),
        PAGE.width,
        footer.y - header.bottom(),
    );

    text_line(canvas, pt(header.x + 22.0, header.y + 14.0), 90.0);
    paragraph(
        canvas,
        pt(body.x + 22.0, body.y + 14.0),
        &[218.0, 210.0, 224.0, 176.0],
    );
    text_line(canvas, pt(footer.x + 22.0, footer.y + 12.0), 62.0);

    // Editing the header inverts which region is dimmed — that is what the focused state *is*
    // here, and the two must be equally readable.
    let editing_header = ink.state.interaction == Interaction::Focused;
    let dimmed: &[Rect] = if editing_header {
        &[body]
    } else {
        &[header, footer]
    };
    let dim = canvas.group(None, PAGE, 0.45, None);
    for region in dimmed {
        canvas.rect(Some(dim), *region, filled(ink.page()));
    }
    for boundary in [header.bottom(), footer.y] {
        canvas.line(
            None,
            pt(PAGE.x + 8.0, boundary),
            pt(PAGE.right() - 8.0, boundary),
            "region boundary",
            dashed_stroke(scale, ink.muted(), ink.width(0.6), DashPattern::Dash),
        );
    }
}

/// 47 — Text-wrap boundary preview around a floating object.
pub fn wrap_boundary(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    // A ragged column of text, wrapped around the object rather than under it.
    let widths = [216.0, 132.0, 126.0, 128.0, 130.0, 212.0];
    let lines = paragraph(canvas, pt(CONTENT.x, CONTENT.y + 6.0), &widths);
    let floating = Rect::new(CONTENT.x + 142.0, lines[1].y - 6.0, 82.0, 58.0);
    object(canvas, None, floating);
    // The square wrap boundary — the object's own box plus the distance-from-text.
    canvas.rect(
        None,
        floating.inflated(7.0),
        dashed(scale, ink.guide(), ink.width(0.75), DashPattern::Dash),
    );
    // The tight boundary, which follows the shape rather than the box: this is what "tight" means
    // and it is the only way to see that the two are different things.
    canvas.polygon(
        None,
        &[
            pt(floating.x + 12.0, floating.y - 4.0),
            pt(floating.right() + 4.0, floating.y + 6.0),
            pt(floating.right() + 4.0, floating.bottom() - 8.0),
            pt(floating.centre().x, floating.bottom() + 4.0),
            pt(floating.x - 4.0, floating.bottom() - 18.0),
        ],
        "tight wrap boundary",
        canvas::dashed(scale, ink.accent(), ink.width(0.75), DashPattern::Dot),
    );
}

/// 48 — Column boundaries and the balanced-column indicator.
pub fn column_boundaries(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let area = Rect::new(CONTENT.x, CONTENT.y, 226.0, 130.0);
    let column_width = 66.0;
    let gutter = 14.0;
    for index in 0..3 {
        let x = area.x + f64::from(index) * (column_width + gutter);
        let count = if index == 2 { 5 } else { 8 };
        let widths: Vec<f64> = (0..count)
            .map(|line| {
                if line == count - 1 {
                    column_width * 0.62
                } else {
                    column_width
                }
            })
            .collect();
        paragraph(canvas, pt(x, area.y + 6.0), &widths);
        if index < 2 {
            let boundary = x + column_width + gutter / 2.0;
            canvas.line(
                None,
                pt(boundary, area.y),
                pt(boundary, area.bottom()),
                "column boundary",
                dashed_stroke(
                    scale,
                    canvas::with_alpha(ink.grid(), 0xdd),
                    0.5,
                    DashPattern::Dot,
                ),
            );
        }
    }
    // The balance mark at the foot of the last column: the rule that says the columns end level.
    let last = area.x + 2.0 * (column_width + gutter);
    canvas.line(
        None,
        pt(last, area.y + 5.0 * LINE_HEIGHT + 8.0),
        pt(last + column_width, area.y + 5.0 * LINE_HEIGHT + 8.0),
        "balance mark",
        stroke(scale, ink.guide(), 0.75),
    );
}

/// 49 — Section-break markers.
pub fn section_break_markers(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    paragraph(canvas, pt(CONTENT.x, CONTENT.y), &[220.0, 212.0, 168.0]);
    paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 70.0),
        &[218.0, 194.0, 206.0],
    );

    for (index, y) in [CONTENT.y + 52.0_f64, CONTENT.y + 126.0]
        .into_iter()
        .enumerate()
    {
        canvas.line(
            None,
            pt(CONTENT.x, y),
            pt(CONTENT.x + 226.0, y),
            "section break",
            dashed_stroke(scale, ink.muted(), 0.6, DashPattern::LargeDashDot),
        );
        // The label plate: a continuous break and a next-page break must be tellable apart, and the
        // rule alone cannot do it.
        let plate = Rect::new(
            CONTENT.x + 74.0,
            y - 6.0,
            if index == 0 { 62.0 } else { 78.0 },
            12.0,
        );
        canvas.rect(
            None,
            plate,
            filled_and_stroked(scale, ink.page(), ink.muted(), 0.5),
        );
        canvas.rect(
            None,
            Rect::new(
                plate.x + 5.0,
                plate.centre().y - 2.0,
                plate.width - 10.0,
                4.0,
            ),
            filled(canvas::with_alpha(ink.muted(), 0xaa)),
        );
    }
}

/// 50 — Footnote separator and continuation notice.
pub fn footnote_separator(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y),
        &[224.0, 216.0, 210.0, 182.0],
    );

    // The ordinary separator: a short rule, a third of the measure.
    let first = CONTENT.y + 70.0;
    canvas.line(
        None,
        pt(CONTENT.x, first),
        pt(CONTENT.x + 74.0, first),
        "footnote separator",
        stroke(scale, ink.muted(), 0.75),
    );
    paragraph(canvas, pt(CONTENT.x, first + 8.0), &[190.0, 148.0]);

    // The continuation separator: the full measure, which is how Word says *"this note began on the
    // previous page"* without a word.
    let second = CONTENT.y + 108.0;
    canvas.line(
        None,
        pt(CONTENT.x, second),
        pt(CONTENT.x + 226.0, second),
        "continuation separator",
        stroke(scale, ink.muted(), 0.75),
    );
    paragraph(canvas, pt(CONTENT.x, second + 8.0), &[210.0, 96.0]);
}

/// 51 — Non-printing marks.
pub fn non_printing_marks(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let mark = canvas::with_alpha(ink.muted(), 0xcc);
    let base = pt(CONTENT.x, CONTENT.y + 24.0);
    text_line(canvas, base, 48.0);
    text_line(canvas, pt(base.x + 60.0, base.y), 40.0);
    text_line(canvas, pt(base.x + 132.0, base.y), 56.0);

    // The space dot, between the first two words.
    canvas.rect(
        None,
        Rect::centred(pt(base.x + 54.0, base.y + TEXT_BAR / 2.0), 1.6),
        filled(mark),
    );
    // The tab arrow.
    canvas.line(
        None,
        pt(base.x + 104.0, base.y + TEXT_BAR / 2.0),
        pt(base.x + 128.0, base.y + TEXT_BAR / 2.0),
        "tab",
        stroke(scale, mark, 0.6),
    );
    stage::chevron(
        canvas,
        pt(base.x + 128.0, base.y + TEXT_BAR / 2.0),
        3.0,
        false,
        "tab arrow",
        mark,
        0.6,
    );
    // The pilcrow, at the end of the line: a bowl, a stem and a second stem.
    let pilcrow = pt(base.x + 196.0, base.y - 4.0);
    canvas.path(
        None,
        Rect::new(pilcrow.x - 1.0, pilcrow.y - 1.0, 10.0, 15.0),
        stage::rounded_rectangle(Rect::new(pilcrow.x, pilcrow.y, 5.5, 7.0), 2.75),
        FillRule::NonZero,
        "pilcrow bowl",
        filled(mark),
    );
    for offset in [4.0_f64, 6.5] {
        canvas.rect(
            None,
            Rect::new(pilcrow.x + offset, pilcrow.y, 1.0, 13.0),
            filled(mark),
        );
    }
    // The line break, on the second line.
    let second = pt(base.x, base.y + LINE_HEIGHT * 2.0);
    text_line(canvas, second, 118.0);
    canvas.polyline(
        None,
        &[
            pt(second.x + 128.0, second.y - 3.0),
            pt(second.x + 128.0, second.y + 6.0),
            pt(second.x + 122.0, second.y + 6.0),
        ],
        false,
        "line break",
        stroke(scale, mark, 0.7),
    );
    stage::chevron(
        canvas,
        pt(second.x + 122.0, second.y + 6.0),
        3.0,
        false,
        "break arrow",
        mark,
        0.7,
    );
    // The optional hyphen, on the third line.
    let third = pt(base.x, base.y + LINE_HEIGHT * 4.0);
    text_line(canvas, third, 96.0);
    canvas.rect(
        None,
        Rect::new(third.x + 100.0, third.y + TEXT_BAR / 2.0 - 0.4, 5.0, 0.8),
        filled(mark),
    );
    canvas.rect(
        None,
        Rect::new(third.x + 104.0, third.y - 2.0, 0.8, 4.0),
        filled(mark),
    );
}

/// 52 — Field shading and the field-selection state.
pub fn field_shading(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let lines = paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 18.0),
        &[222.0, 214.0, 190.0],
    );

    // The shading is a group at the token's own alpha, for the reason `Ink::selection_colour`
    // gives: two abutting fields at the same alpha must not composite into a darker seam.
    let shade = canvas.group(None, CONTENT, 0.16, None);
    let quiet = canvas::with_alpha(ink.muted(), 0xff);
    for (line, from, width) in [
        (0_usize, 56.0_f64, 48.0_f64),
        (1, 24.0, 62.0),
        (2, 96.0, 54.0),
    ] {
        canvas.rect(
            Some(shade),
            Rect::new(
                lines[line].x + from,
                lines[line].y - 2.5,
                width,
                TEXT_BAR + 5.0,
            ),
            filled(quiet),
        );
    }
    // The hovered field, and the selected one: three states of the same element, side by side, so
    // the judgement — *"is the shading visible without reading as a selection?"* — can be made.
    let hovered = Rect::new(lines[1].x + 24.0, lines[1].y - 2.5, 62.0, TEXT_BAR + 5.0);
    let selected = Rect::new(lines[2].x + 96.0, lines[2].y - 2.5, 54.0, TEXT_BAR + 5.0);
    let hover_group = canvas.group(None, CONTENT, 0.28, None);
    canvas.rect(Some(hover_group), hovered, filled(quiet));
    let selection = canvas.group(None, CONTENT, ink.selection_opacity() * 2.0, None);
    canvas.rect(Some(selection), selected, filled(ink.selection_colour()));
}

/// 53 — Bookmark brackets.
pub fn bookmark_brackets(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let lines = paragraph(canvas, pt(CONTENT.x, CONTENT.y + 24.0), &[218.0, 202.0]);
    let colour = canvas::with_alpha(ink.muted(), 0xdd);
    let height = TEXT_BAR + 6.0;
    // The opening and closing brackets around a run.
    for (x, opening) in [(lines[0].x + 48.0, true), (lines[0].x + 148.0, false)] {
        let arm = if opening { 3.0 } else { -3.0 };
        canvas.polyline(
            None,
            &[
                pt(x + arm, lines[0].y - 3.0),
                pt(x, lines[0].y - 3.0),
                pt(x, lines[0].y - 3.0 + height),
                pt(x + arm, lines[0].y - 3.0 + height),
            ],
            false,
            "bookmark bracket",
            stroke(scale, colour, 0.75),
        );
    }
    // An empty bookmark, which collapses into an I-beam.
    let empty = lines[1].x + 108.0;
    canvas.polyline(
        None,
        &[
            pt(empty - 3.0, lines[1].y - 3.0),
            pt(empty, lines[1].y - 3.0),
            pt(empty, lines[1].y - 3.0 + height),
            pt(empty - 3.0, lines[1].y - 3.0 + height),
        ],
        false,
        "empty bookmark",
        stroke(scale, colour, 0.75),
    );
    canvas.polyline(
        None,
        &[
            pt(empty + 3.0, lines[1].y - 3.0),
            pt(empty, lines[1].y - 3.0),
            pt(empty, lines[1].y - 3.0 + height),
            pt(empty + 3.0, lines[1].y - 3.0 + height),
        ],
        false,
        "empty bookmark",
        stroke(scale, colour, 0.75),
    );
}

/// 54 — Comment anchor and its connector line to the margin card.
pub fn comment_anchor(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(PAGE.x + 14.0, PAGE.y + 16.0, 152.0, 140.0);
    let lines = paragraph(
        canvas,
        pt(body.x, body.y),
        &[148.0, 140.0, 152.0, 118.0, 146.0],
    );

    let anchored = Rect::new(lines[1].x + 40.0, lines[1].y - 2.0, 74.0, TEXT_BAR + 4.0);
    canvas.rect(
        None,
        anchored,
        filled(canvas::with_alpha(ink.comment(), 0x3a)),
    );
    canvas.rect(
        None,
        Rect::new(
            anchored.x,
            anchored.bottom(),
            anchored.width,
            ink.width(1.0),
        ),
        filled(ink.comment()),
    );

    let card = Rect::new(PAGE.right() - 84.0, lines[1].y - 8.0, 74.0, 44.0);
    canvas.rect(
        None,
        card,
        filled_and_stroked(scale, ink.raised_surface(), ink.border(), 0.6),
    );
    paragraph(canvas, pt(card.x + 6.0, card.y + 8.0), &[58.0, 62.0, 40.0]);
    // The connector, elbowed rather than straight, so it does not cross the words at an angle.
    canvas.polyline(
        None,
        &[
            pt(anchored.right(), anchored.centre().y),
            pt(body.right() + 8.0, anchored.centre().y),
            pt(body.right() + 8.0, card.centre().y),
            pt(card.x, card.centre().y),
        ],
        false,
        "comment connector",
        stroke(scale, ink.comment(), ink.width(0.75)),
    );
    canvas.circle(
        None,
        pt(anchored.right(), anchored.centre().y),
        2.0,
        "anchor dot",
        filled(ink.comment()),
    );
}

/// 55 — Tracked-change bars and inline insert/delete rendering.
pub fn tracked_changes(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let lines = paragraph(
        canvas,
        pt(CONTENT.x + 12.0, CONTENT.y + 18.0),
        &[214.0, 206.0, 198.0, 180.0],
    );

    // An insertion: an underline in the insert colour. **Its own glyphs stay at the text colour** —
    // the `document.*.tracked-change-insert` token is tagged `fill-only` precisely because it does
    // not reach 4.5 : 1 against the page, and a scene that coloured the words would contradict the
    // contrast rule the token generator enforces.
    let inserted = Rect::new(lines[0].x + 62.0, lines[0].y, 68.0, TEXT_BAR);
    canvas.rect(
        None,
        Rect::new(inserted.x, inserted.bottom() + 1.5, inserted.width, 1.0),
        filled(ink.insertion()),
    );

    // A deletion: a strike-through in the delete colour, over words that are still there.
    let deleted = Rect::new(lines[2].x + 34.0, lines[2].y, 82.0, TEXT_BAR);
    canvas.rect(
        None,
        Rect::new(deleted.x, deleted.centre().y - 0.5, deleted.width, 1.0),
        filled(ink.deletion()),
    );

    // The change bars in the margin: one per changed line, and nothing beside the unchanged ones.
    for line in [0_usize, 2] {
        canvas.line(
            None,
            pt(CONTENT.x, lines[line].y - 4.0),
            pt(CONTENT.x, lines[line].bottom() + 4.0),
            "change bar",
            stroke(
                scale,
                if line == 0 {
                    ink.insertion()
                } else {
                    ink.deletion()
                },
                1.5,
            ),
        );
    }
}

/// 56 — Slide guides, drawing-canvas boundary, and the safe area.
pub fn guides_and_safe_area(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    object(canvas, None, Rect::new(96.0, 74.0, 108.0, 60.0));

    // Three dashed rules, deliberately three different dashes: a reviewer who cannot tell a slide
    // guide from a canvas boundary from the safe area is looking at a design defect.
    let centre_x = PAGE.centre().x;
    let centre_y = PAGE.centre().y;
    let vertical = canvas.line(
        None,
        pt(centre_x, PAGE.y),
        pt(centre_x, PAGE.bottom()),
        "vertical guide",
        dashed_stroke(scale, ink.guide(), ink.width(0.75), DashPattern::Dash),
    );
    canvas.grab(vertical, "vertical guide");
    let horizontal = canvas.line(
        None,
        pt(PAGE.x, centre_y),
        pt(PAGE.right(), centre_y),
        "horizontal guide",
        dashed_stroke(scale, ink.guide(), ink.width(0.75), DashPattern::Dash),
    );
    canvas.grab(horizontal, "horizontal guide");
    // The grips at each guide's end, sized by the input device. A guide is a hairline, and a
    // hairline is not something a finger can find: without a grip, this element would be one a
    // touch user could see and could not move — and the input toggle would move no pixel, which is
    // the same defect wearing a different hat.
    let grip = ink.affordance();
    canvas.rect(
        None,
        Rect::new(centre_x - grip / 2.0, PAGE.y, grip, grip * 0.55),
        filled_and_stroked(scale, ink.page(), ink.guide(), 0.75),
    );
    canvas.rect(
        None,
        Rect::new(PAGE.x, centre_y - grip / 2.0, grip * 0.55, grip),
        filled_and_stroked(scale, ink.page(), ink.guide(), 0.75),
    );
    canvas.rect(
        None,
        Rect::new(
            PAGE.x + 30.0,
            PAGE.y + 22.0,
            PAGE.width - 60.0,
            PAGE.height - 44.0,
        ),
        dashed(scale, ink.muted(), 0.6, DashPattern::Dot),
    );
    canvas.rect(
        None,
        PAGE.inflated(-8.0),
        dashed(
            scale,
            canvas::with_alpha(ink.warning(), 0xcc),
            0.75,
            DashPattern::LargeDashDotDot,
        ),
    );
    if ink.state.interaction.is_engaged() {
        badge(canvas, pt(centre_x + 6.0, PAGE.y + 8.0), 44.0);
    }
}
