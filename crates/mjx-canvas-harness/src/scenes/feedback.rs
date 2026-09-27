//! §2.6 — state and feedback, entries 57 to 61. The five elements that say what the *renderer* is
//! doing, rather than what the document contains.

use mjx_scene::{DashPattern, Decoration, FillStyle, PatternPreset};

use crate::canvas::{
    self, dashed, dashed_stroke, filled, filled_and_stroked, pt, stroke, Canvas, Rect, PAGE, STAGE,
};
use crate::scenes::stage::{self, badge, gridlines, paragraph, CONTENT, LINE_HEIGHT};
use crate::state::Interaction;

/// 57 — Canvas focus ring.
pub fn canvas_focus_ring(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    paragraph(
        canvas,
        pt(CONTENT.x, CONTENT.y + 10.0),
        &[220.0, 208.0, 216.0, 178.0],
    );

    let focused = ink.state.interaction == Interaction::Focused;
    let frame = Rect::new(3.0, 3.0, STAGE.0 - 6.0, STAGE.1 - 6.0);
    if focused {
        // Two rings, the inner one in the page colour: a single accent ring vanishes against a dark
        // backdrop, and this is the pair that survives both schemes. The judgement the audit is for
        // is whether the offset is right.
        canvas.rect(
            None,
            frame.inflated(1.5),
            canvas::stroked(scale, ink.page(), 1.5),
        );
        canvas.rect(None, frame, canvas::stroked(scale, ink.accent(), 2.0));
    } else {
        // What it must **not** look like: the resting canvas edge, present so a reviewer is
        // comparing two things rather than remembering one.
        canvas.rect(
            None,
            frame,
            canvas::stroked(scale, ink.border_subtle(), 0.75),
        );
    }
}

/// 58 — Page placeholder for content not yet laid out, and its refinement tier.
pub fn page_placeholder(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let refined = ink.state.interaction == Interaction::Active;
    let skeleton = canvas::with_alpha(ink.muted(), if refined { 0x50 } else { 0x28 });

    // The skeleton: a title bar, a block of body bars, and a picture box. At the refined tier the
    // bars are ragged and the box carries its own frame, which is what "more of it is real" looks
    // like.
    canvas.rect(
        None,
        Rect::new(
            CONTENT.x,
            CONTENT.y + 4.0,
            if refined { 148.0 } else { 176.0 },
            10.0,
        ),
        filled(skeleton),
    );
    for index in 0..5 {
        let width = if refined {
            [206.0, 188.0, 214.0, 142.0, 198.0][index]
        } else {
            210.0
        };
        canvas.rect(
            None,
            Rect::new(
                CONTENT.x,
                CONTENT.y + 26.0 + f64::from(index as u32) * LINE_HEIGHT,
                width,
                6.0,
            ),
            filled(skeleton),
        );
    }
    let picture = Rect::new(CONTENT.x, CONTENT.y + 104.0, 96.0, 34.0);
    canvas.rect(None, picture, filled(skeleton));
    if refined {
        canvas.rect(None, picture, canvas::stroked(scale, ink.border(), 0.6));
    }

    // The tier marker: three pips, filled up to the tier reached.
    let reached = if refined { 2 } else { 1 };
    for index in 0..3 {
        let at = pt(
            CONTENT.right() - 26.0 + f64::from(index as u32) * 9.0,
            CONTENT.bottom() - 6.0,
        );
        canvas.circle(
            None,
            at,
            3.0,
            "refinement tier",
            if index < reached {
                filled(ink.accent())
            } else {
                canvas::stroked(scale, ink.border(), 0.75)
            },
        );
    }
}

/// 59 — Missing-resource placeholders.
///
/// **Drawn here, not by `mjx-scene`.** `PlaceholderGeometry` is the stand-in for a shape whose
/// *geometry* nobody registered, and every plate this harness takes asserts
/// `DrawReport::placeholders == 0`. A missing *resource* is a different thing entirely — the
/// geometry is known and the picture is not — so it is the document's own drawing and the counter
/// stays at zero.
pub fn missing_resource(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();

    let absent = Rect::new(CONTENT.x + 6.0, CONTENT.y + 22.0, 104.0, 78.0);
    let unsupported = Rect::new(CONTENT.x + 128.0, CONTENT.y + 22.0, 104.0, 78.0);

    // A real `a:pattFill` — one of the fifty-four presets — so the hatch is `mjx-paint`'s own mask
    // rather than a second table drawn here, and so the entry's `PatternFill` content kind is true.
    for (index, box_) in [absent, unsupported].into_iter().enumerate() {
        canvas.rect(
            None,
            box_,
            Decoration {
                fill: FillStyle::Pattern {
                    preset: if index == 0 {
                        PatternPreset::LightUpwardDiagonal
                    } else {
                        PatternPreset::LightDownwardDiagonal
                    },
                    foreground: canvas::with_alpha(ink.warning(), 0xaa),
                    background: canvas::with_alpha(ink.warning(), 0x18),
                },
                stroke: Some(dashed_stroke(
                    scale,
                    ink.warning(),
                    ink.width(1.0),
                    DashPattern::Dash,
                )),
                ..Decoration::none()
            },
        );
    }

    // The broken picture: a frame, a horizon and a sun, with the frame torn.
    let inner = absent.inflated(-22.0);
    canvas.rect(
        None,
        inner,
        canvas::stroked(scale, ink.warning(), ink.width(1.25)),
    );
    canvas.polyline(
        None,
        &[
            pt(inner.x, inner.bottom()),
            pt(inner.x + inner.width * 0.4, inner.y + inner.height * 0.35),
            pt(inner.right(), inner.bottom()),
        ],
        false,
        "broken picture",
        stroke(scale, ink.warning(), ink.width(1.0)),
    );
    canvas.circle(
        None,
        pt(inner.right() - 8.0, inner.y + 8.0),
        3.0,
        "broken picture sun",
        filled(ink.warning()),
    );

    // Unsupported media: a play triangle with a bar through it.
    let media = unsupported.centre();
    canvas.polygon(
        None,
        &[
            pt(media.x - 8.0, media.y - 10.0),
            pt(media.x + 10.0, media.y),
            pt(media.x - 8.0, media.y + 10.0),
        ],
        "unsupported media",
        filled(ink.warning()),
    );
    canvas.line(
        None,
        pt(media.x - 16.0, media.y + 14.0),
        pt(media.x + 16.0, media.y - 14.0),
        "unsupported bar",
        stroke(scale, ink.warning(), ink.width(1.75)),
    );

    if ink.state.interaction.is_engaged() {
        badge(canvas, pt(absent.x, absent.bottom() + 10.0), 108.0);
    }
}

/// 60 — Font-substitution badge.
pub fn font_substitution_badge(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let lines = paragraph(
        canvas,
        pt(CONTENT.x + 10.0, CONTENT.y + 20.0),
        &[212.0, 202.0, 196.0],
    );

    // The substituted run: underlined with a dotted rule, not recoloured. Recolouring the words
    // would change what the author's document looks like in order to report that it has changed,
    // which is the failure the badge exists to prevent.
    let substituted = Rect::new(lines[1].x + 44.0, lines[1].y, 92.0, 5.0);
    canvas.line(
        None,
        pt(substituted.x, substituted.bottom() + 2.0),
        pt(substituted.right(), substituted.bottom() + 2.0),
        "substitution underline",
        dashed_stroke(scale, ink.warning(), ink.width(0.75), DashPattern::Dot),
    );
    let node = badge(
        canvas,
        pt(substituted.right() + 6.0, substituted.y - 4.0),
        66.0,
    );
    canvas.grab(node, "substitution badge");
    canvas.circle(
        None,
        pt(substituted.right() + 12.0, substituted.y + 2.5),
        3.0,
        "substitution dot",
        filled(ink.warning()),
    );

    // The margin marker: this page carries a substitution, whether or not the run is on screen.
    stage::triangle(
        canvas,
        pt(PAGE.x + 3.0, lines[1].y - 4.0),
        6.0,
        (1.0, 1.0),
        "substitution marker",
        ink.warning(),
    );
}

/// 61 — Print-area boundary and zoom-dependent hairline rendering.
pub fn print_area_hairlines(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let sheet = Rect::new(CONTENT.x, CONTENT.y, 148.0, 132.0);
    gridlines(canvas, sheet, 4, 5);
    let print_area = stage::span(sheet, 4, 5, (0, 0), (2, 3));
    canvas.rect(
        None,
        print_area,
        dashed(scale, ink.muted(), 1.0, DashPattern::LargeDash),
    );

    // The hairline ladder: the same rule at four widths, labelled by a tick of its own width. This
    // is the scene the density toggle exists for — at 1× a quarter-point rule is a third of a
    // device pixel, and whether it should be snapped up to one is exactly the question the audit
    // has to answer.
    let ladder = pt(CONTENT.x + 172.0, CONTENT.y + 8.0);
    for (index, width) in [0.25_f64, 0.5, 0.75, 1.0].into_iter().enumerate() {
        let y = ladder.y + f64::from(index as u32) * 22.0;
        canvas.line(
            None,
            pt(ladder.x, y),
            pt(ladder.x + 56.0, y),
            "hairline",
            stroke(scale, ink.text(), width),
        );
        canvas.rect(
            None,
            Rect::new(ladder.x, y + 6.0, width * 8.0, 3.0),
            filled(canvas::with_alpha(ink.muted(), 0xaa)),
        );
    }
    canvas.rect(
        None,
        Rect::new(ladder.x - 8.0, ladder.y - 10.0, 76.0, 100.0),
        filled_and_stroked(
            scale,
            canvas::with_alpha(ink.page(), 0x00),
            ink.border_subtle(),
            0.5,
        ),
    );
}
