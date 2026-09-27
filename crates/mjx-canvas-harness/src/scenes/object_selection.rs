//! §2.1 — object selection, entries 1 to 11.

use mjx_scene::{DashPattern, FillRule};

use crate::canvas::{self, dashed, filled, filled_and_stroked, pt, stroke, Canvas, Rect, PAGE};
use crate::scenes::stage::{
    self, adjustment_handle, badge, dashed_outline, handle, object, resting_handle, round_handle,
    selection_outline, CONTENT,
};
use crate::state::Interaction;

/// 1 — Selection outline, single object.
pub fn selection_outline_single(canvas: &mut Canvas) {
    stage::stage(canvas);
    let body = Rect::new(96.0, 62.0, 108.0, 72.0);
    object(canvas, None, body);
    selection_outline(canvas, body.inflated(3.0), 1.0);
}

/// 2 — Selection outline, multiple objects, with the union bounds.
pub fn selection_outline_multiple(canvas: &mut Canvas) {
    stage::stage(canvas);
    let bodies = [
        Rect::new(50.0, 44.0, 62.0, 44.0),
        Rect::new(132.0, 78.0, 74.0, 50.0),
        Rect::new(196.0, 40.0, 52.0, 62.0),
    ];
    let mut union = bodies[0];
    for body in bodies {
        object(canvas, None, body);
        union = Rect::new(
            union.x.min(body.x),
            union.y.min(body.y),
            union.right().max(body.right()) - union.x.min(body.x),
            union.bottom().max(body.bottom()) - union.y.min(body.y),
        );
    }
    for body in bodies {
        selection_outline(canvas, body.inflated(2.0), 0.75);
    }
    dashed_outline(canvas, union.inflated(7.0), 1.25, DashPattern::Dash);
}

/// 3 — Resize handles: four corner, four edge, with hover and active states.
pub fn resize_handles(canvas: &mut Canvas) {
    stage::stage(canvas);
    let body = Rect::new(88.0, 58.0, 124.0, 80.0);
    object(canvas, None, body);
    selection_outline(canvas, body.inflated(3.0), 1.0);
    let names = [
        "north-west",
        "north",
        "north-east",
        "east",
        "south-east",
        "south",
        "south-west",
        "west",
    ];
    for (index, centre) in body.inflated(3.0).handles().into_iter().enumerate() {
        // **Exactly one handle answers the interaction toggle.** If every one grew under hover the
        // toggle would be indistinguishable from a zoom, and a reviewer could not see what the
        // pointer is doing to the element.
        if index == 3 {
            handle(canvas, centre, names[index]);
        } else {
            resting_handle(canvas, centre, names[index]);
        }
    }
}

/// 4 — Rotation handle, its tether, and the live angle readout.
pub fn rotation_handle(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(102.0, 74.0, 96.0, 62.0);
    let turned = object(canvas, None, body);
    canvas.rotate(
        turned,
        if ink.state.interaction == Interaction::Active {
            -22.0
        } else {
            -8.0
        },
    );
    selection_outline(canvas, body.inflated(3.0), 1.0);
    let top = pt(body.centre().x, body.y - 3.0);
    let grip = pt(top.x, top.y - 22.0);
    canvas.line(
        None,
        top,
        grip,
        "rotation tether",
        stroke(scale, ink.accent(), 1.0),
    );
    round_handle(canvas, grip, "rotation grip");
    if ink.state.interaction.is_engaged() {
        badge(canvas, pt(grip.x + 12.0, grip.y - 8.0), 46.0);
    }
}

/// 5 — Rotation snap indicator, at fifteen-degree increments.
pub fn rotation_snap(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(114.0, 76.0, 72.0, 48.0);
    object(canvas, None, body);
    let centre = body.centre();
    let (inner, outer) = (44.0_f64, 52.0_f64);
    canvas.circle(
        None,
        centre,
        outer,
        "snap dial",
        dashed(
            scale,
            canvas::with_alpha(ink.guide(), 0xa0),
            0.5,
            DashPattern::Dot,
        ),
    );
    for step in 0..24 {
        let degrees = f64::from(step) * 15.0;
        let radians = degrees.to_radians();
        let (sine, cosine) = radians.sin_cos();
        let engaged = step == 3;
        let length = if engaged { outer + 8.0 } else { outer };
        canvas.line(
            None,
            pt(centre.x + cosine * inner, centre.y + sine * inner),
            pt(centre.x + cosine * length, centre.y + sine * length),
            "snap mark",
            stroke(
                scale,
                if engaged { ink.accent() } else { ink.guide() },
                if engaged { ink.width(1.6) } else { 0.6 },
            ),
        );
    }
    // The arm, rotated, so the dial has a needle rather than only marks — and so that this scene
    // carries a real `PushTransform` rather than declaring one it does not have.
    let arm = canvas.rect(
        None,
        Rect::new(centre.x, centre.y - 1.0, outer, 2.0),
        filled(ink.accent()),
    );
    canvas.rotate(
        arm,
        if ink.state.interaction == Interaction::Active {
            45.0
        } else {
            15.0
        },
    );
}

/// 6 — Shape adjustment handles, the `avLst` control points.
pub fn adjustment_handles(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();

    let rounded = Rect::new(48.0, 60.0, 88.0, 62.0);
    canvas.path(
        None,
        rounded,
        stage::rounded_rectangle(rounded, 16.0),
        FillRule::NonZero,
        "rounded rectangle",
        filled_and_stroked(scale, ink.object(), ink.object_border(), 0.75),
    );
    selection_outline(canvas, rounded.inflated(3.0), 0.75);
    adjustment_handle(canvas, pt(rounded.x + 16.0, rounded.y), "corner radius");

    let arrow = Rect::new(166.0, 60.0, 92.0, 62.0);
    canvas.polygon(
        None,
        &[
            pt(arrow.x, arrow.y + 18.0),
            pt(arrow.x + 54.0, arrow.y + 18.0),
            pt(arrow.x + 54.0, arrow.y),
            pt(arrow.right(), arrow.centre().y),
            pt(arrow.x + 54.0, arrow.bottom()),
            pt(arrow.x + 54.0, arrow.bottom() - 18.0),
            pt(arrow.x, arrow.bottom() - 18.0),
        ],
        "arrow",
        filled_and_stroked(scale, ink.object(), ink.object_border(), 0.75),
    );
    selection_outline(canvas, arrow.inflated(3.0), 0.75);
    adjustment_handle(canvas, pt(arrow.x + 54.0, arrow.y + 18.0), "arrow neck");
    adjustment_handle(canvas, pt(arrow.x + 54.0, arrow.centre().y), "head length");
}

/// 7 — Connection sites, shown while a connector is being drawn.
pub fn connection_sites(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let left = Rect::new(48.0, 66.0, 76.0, 52.0);
    let right = Rect::new(176.0, 82.0, 76.0, 52.0);
    object(canvas, None, left);
    object(canvas, None, right);
    for (index, site) in sites(left).into_iter().enumerate() {
        round_handle(canvas, site, ["north", "east", "south", "west"][index]);
    }
    for (index, site) in sites(right).into_iter().enumerate() {
        round_handle(canvas, site, ["north", "east", "south", "west"][index]);
    }
    // The connector being drawn — which is *why* the sites are shown at all.
    canvas.line(
        None,
        pt(left.right(), left.centre().y),
        pt(right.x - 10.0, right.centre().y - 6.0),
        "connector being drawn",
        stroke(scale, ink.accent(), ink.width(1.25)),
    );
}

/// The four connection sites of a rectangle.
fn sites(rect: crate::canvas::Rect) -> [crate::canvas::Pt; 4] {
    let centre = rect.centre();
    [
        pt(centre.x, rect.y),
        pt(rect.right(), centre.y),
        pt(centre.x, rect.bottom()),
        pt(rect.x, centre.y),
    ]
}

/// 8 — Connector endpoints, hover targets, and the reroute preview.
pub fn connector_endpoints(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let left = Rect::new(44.0, 52.0, 70.0, 46.0);
    let right = Rect::new(186.0, 112.0, 70.0, 46.0);
    object(canvas, None, left);
    object(canvas, None, right);
    let start = pt(left.right(), left.centre().y);
    let finish = pt(right.x, right.centre().y);
    let elbow = pt((start.x + finish.x) / 2.0, start.y);
    let knee = pt(elbow.x, finish.y);
    canvas.polyline(
        None,
        &[start, elbow, knee, finish],
        false,
        "connector",
        stroke(scale, ink.object_border(), 1.25),
    );
    // The route the connector would take if the drag were released here.
    canvas.polyline(
        None,
        &[
            start,
            pt(start.x + 24.0, start.y),
            pt(start.x + 24.0, finish.y + 26.0),
            pt(finish.x, finish.y + 26.0),
            finish,
        ],
        false,
        "reroute preview",
        {
            let mut style = stroke(scale, ink.accent(), ink.width(1.0));
            style.dash = DashPattern::Dash;
            style
        },
    );
    round_handle(canvas, start, "start endpoint");
    round_handle(canvas, finish, "end endpoint");
}

/// 9 — Group selection outline versus a selected child inside a group.
pub fn group_versus_child(canvas: &mut Canvas) {
    stage::stage(canvas);
    let bounds = Rect::new(64.0, 50.0, 168.0, 96.0);
    // A real clip, so the group is a group rather than three objects that happen to be near each
    // other — and so the scene carries the `PushClip` its entry declares.
    let group = canvas.group(None, bounds, 1.0, Some(bounds));
    let children = [
        Rect::new(76.0, 62.0, 56.0, 34.0),
        Rect::new(146.0, 62.0, 72.0, 34.0),
        Rect::new(76.0, 106.0, 142.0, 28.0),
    ];
    for child in children {
        object(canvas, Some(group), child);
    }
    dashed_outline(canvas, bounds.inflated(5.0), 1.0, DashPattern::Dash);
    selection_outline(canvas, children[1].inflated(2.0), 1.25);
    for centre in children[1].inflated(2.0).handles() {
        resting_handle(canvas, centre, "child handle");
    }
}

/// 10 — Locked-object indicator.
pub fn locked_object(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();

    let free = Rect::new(46.0, 66.0, 92.0, 60.0);
    object(canvas, None, free);
    selection_outline(canvas, free.inflated(3.0), 1.0);
    for centre in free.inflated(3.0).handles() {
        resting_handle(canvas, centre, "free handle");
    }

    let locked = Rect::new(166.0, 66.0, 92.0, 60.0);
    object(canvas, None, locked);
    canvas.rect(
        None,
        locked.inflated(3.0),
        dashed(scale, ink.muted(), 1.0, DashPattern::SystemDot),
    );
    // An inert handle at every position, in the muted colour: a locked object that showed no
    // handles at all would be indistinguishable from one that is not selected.
    for centre in locked.inflated(3.0).handles() {
        canvas.rect(
            None,
            Rect::centred(centre, ink.state.input.affordance_size() * 0.8),
            filled_and_stroked(scale, ink.page(), ink.muted(), 0.75),
        );
    }
    // The padlock, as a shackle and a body.
    let lock = pt(locked.centre().x, locked.y - 14.0);
    canvas.rect(
        None,
        Rect::new(lock.x - 5.0, lock.y - 1.0, 10.0, 9.0),
        filled_and_stroked(scale, ink.warning(), ink.muted(), 0.6),
    );
    canvas.polyline(
        None,
        &[
            pt(lock.x - 3.0, lock.y - 1.0),
            pt(lock.x - 3.0, lock.y - 5.0),
            pt(lock.x + 3.0, lock.y - 5.0),
            pt(lock.x + 3.0, lock.y - 1.0),
        ],
        false,
        "shackle",
        stroke(scale, ink.muted(), 1.1),
    );
}

/// 11 — Off-canvas / partially-off-slide indicator.
pub fn off_canvas_indicator(canvas: &mut Canvas) {
    stage::stage(canvas);
    let ink = canvas.ink().clone();
    let scale = canvas.pixels_per_point();
    let body = Rect::new(216.0, 58.0, 104.0, 66.0);

    // The whole object, dimmed, outside the page.
    let ghost = canvas.group(None, body, 0.32, None);
    object(canvas, Some(ghost), body);

    // The same object again, clipped to the page: the part that will actually print.
    let inside = canvas.group(None, body, 1.0, Some(PAGE));
    object(canvas, Some(inside), body);

    selection_outline(canvas, body.inflated(3.0), 1.0);
    // The marker on the page boundary that says something continues past it.
    canvas.line(
        None,
        pt(PAGE.right(), body.y - 4.0),
        pt(PAGE.right(), body.bottom() + 4.0),
        "page edge",
        stroke(scale, ink.warning(), ink.width(2.0)),
    );
    for offset in [0.0, 1.0, 2.0] {
        stage::chevron(
            canvas,
            pt(PAGE.right() - 6.0 - offset * 5.0, body.centre().y),
            4.0,
            false,
            "off-slide marker",
            ink.warning(),
            ink.width(1.1),
        );
    }
    canvas.rect(
        None,
        Rect::new(CONTENT.x, CONTENT.bottom() - 10.0, 96.0, 10.0),
        filled(canvas::with_alpha(ink.warning(), 0x40)),
    );
}
