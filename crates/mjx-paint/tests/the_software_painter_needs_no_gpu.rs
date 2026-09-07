//! **A page becomes pixels on a machine with no graphics stack at all**, and every effect that page
//! can carry lands where the effect means.
//!
//! # Why this file may not mention `WgpuPainter`
//!
//! It is the headless guarantee. R10's fidelity oracle, R11's plate regression and every golden
//! image after them are taken through this painter, on continuous-integration machines that have no
//! GPU and no display server — and it is half of what made `CLAUDE.md`'s amended pure-Rust rule a
//! boundary rather than a concession.
//!
//! A suite that *could* fall back to a GPU would not prove that. So this one does not name the type:
//! there is no adapter request on any path it reaches, and if `SoftwarePainter::new` ever grew one
//! this file would be the thing that stopped compiling on a machine without it. `two_painters_agree`
//! is where the GPU is compared against; this is where it is absent.
//!
//! # Effects, asserted on where the ink lands
//!
//! MJXOFF-164 asks for *"visibly correct output, asserted on pixel statistics rather than on 'did
//! not error'"*, and the coordinator's audit sharpened it: **a shadow's pixels must land outside the
//! shape's own coverage and an inner shadow's inside it.** That is the assertion that catches the
//! defect [`mjx_paint::plan::draws_behind`] was supposed to prevent and did not — until MJXOFF-164
//! the function was tested, documented and read by nobody, and swapping the two pushes in the
//! shadow arm painted every shadow **on top of its shape** with the whole suite green.
//!
//! So each of the seven kinds is rendered twice — with the effect and without — and the two are
//! compared **by region**:
//!
//! | kind | where it must change something | where it must not |
//! |---|---|---|
//! | `OuterShadow` | outside the shape, toward the effect's own direction | — |
//! | `Glow` | outside the shape, on every side | — |
//! | `Reflection` | below the shape | above it |
//! | `InnerShadow` | **inside** the shape | outside it |
//! | `FillOverlay` | inside the shape | outside it |
//! | `Blur` | outside the shape, where its edge spread to | — |
//! | `SoftEdge` | at the shape's own edge | — |
//!
//! An assertion that only counted covered pixels would pass on a shadow drawn in the wrong place, in
//! the wrong colour, and on top of its shape. These do not.
//!
//! # Proved by mutation
//!
//! * `draws_behind` answering `false` for `OuterShadow` → the shape's own interior changes and
//!   `a_shadow_lands_behind_its_shape` fails on the *unchanged* half, which is the half a naive
//!   assertion leaves out.
//! * `replaces_subtree` answering `false` for `Blur` → the sharp shape is drawn over the blurred one
//!   and the interior stops matching a blurred render.
//! * The `Tint` step reading `source.rgb` instead of `source.a` → the shadow takes the shape's own
//!   colour and `a_shadow_is_drawn_in_the_effects_colour` fails.

mod common;

use mjx_paint::{
    OffscreenSurface, PaintError, Painter, Pixels, Resources, SoftwarePainter, Viewport,
};
use mjx_scene::{EffectKind, PlaceholderGeometry, SceneRect};

/// The page every effect case is drawn on.
const WIDTH: u32 = 120;
/// How tall.
const HEIGHT: u32 = 100;

/// Where `common::one_shape_under` puts its shape, at this size.
const SHAPE: SceneRect = SceneRect {
    left: 36.0,
    top: 30.0,
    right: 84.0,
    bottom: 60.0,
};

/// Draw a list through the software painter and bring its pixels back.
fn render(list: &mjx_scene::DisplayList) -> Result<(Pixels, mjx_paint::DrawReport), PaintError> {
    let mut painter = SoftwarePainter::new();
    let mut host = OffscreenSurface::new(WIDTH, HEIGHT, 1.0);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport)?;
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    let drawn = painter.draw(&frame, list, &mut resources)?;
    painter.end(frame)?;
    let pixels = painter
        .read_pixels()?
        .ok_or(PaintError::NoPixels { name: "tiny-skia" })?;
    Ok((pixels, drawn))
}

/// How many pixels of a rectangle differ between two renders, and by how much in total.
fn difference_in(left: &Pixels, right: &Pixels, region: SceneRect) -> (usize, u64) {
    let mut differing = 0usize;
    let mut total = 0u64;
    for y in region.top.max(0.0) as u32..(region.bottom.max(0.0) as u32).min(HEIGHT) {
        for x in region.left.max(0.0) as u32..(region.right.max(0.0) as u32).min(WIDTH) {
            let (Some(a), Some(b)) = (left.pixel(x, y), right.pixel(x, y)) else {
                continue;
            };
            let mut worst = 0u8;
            for channel in 0..4 {
                let step = a[channel].abs_diff(b[channel]);
                total += u64::from(step);
                worst = worst.max(step);
            }
            if worst > 4 {
                differing += 1;
            }
        }
    }
    (differing, total)
}

/// How many pixels of a region carry any ink at all.
fn covered_in(pixels: &Pixels, region: SceneRect) -> usize {
    let mut covered = 0usize;
    for y in region.top.max(0.0) as u32..(region.bottom.max(0.0) as u32).min(HEIGHT) {
        for x in region.left.max(0.0) as u32..(region.right.max(0.0) as u32).min(WIDTH) {
            if pixels.pixel(x, y).is_some_and(|pixel| pixel[3] > 8) {
                covered += 1;
            }
        }
    }
    covered
}

/// A rectangle inside the shape, clear of its own antialiased edge.
fn inside() -> SceneRect {
    SceneRect::new(
        SHAPE.left + 4.0,
        SHAPE.top + 4.0,
        SHAPE.right - 4.0,
        SHAPE.bottom - 4.0,
    )
}

/// A rectangle below and to the right of the shape, where a shadow at 45° falls.
fn lower_right() -> SceneRect {
    SceneRect::new(
        SHAPE.right + 1.0,
        SHAPE.bottom + 1.0,
        SHAPE.right + 14.0,
        SHAPE.bottom + 14.0,
    )
}

/// A rectangle above the shape, where nothing but a glow should reach.
fn above() -> SceneRect {
    SceneRect::new(SHAPE.left, SHAPE.top - 14.0, SHAPE.right, SHAPE.top - 2.0)
}

/// A rectangle well below the shape, where only a reflection reaches.
fn below() -> SceneRect {
    SceneRect::new(
        SHAPE.left,
        SHAPE.bottom + 4.0,
        SHAPE.right,
        SHAPE.bottom + 24.0,
    )
}

#[test]
fn a_page_becomes_pixels_with_no_graphics_stack() {
    // The whole of `every_command`, on a painter that has never asked for an adapter. Nothing in
    // this test's reachable code constructs a graphics device, and that is the guarantee.
    let list = common::every_command(200.0, 150.0);
    let mut painter = SoftwarePainter::new();
    let mut host = OffscreenSurface::new(200, 150, 1.0);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    let drawn = painter
        .draw(&frame, &list, &mut resources)
        .expect("a page draws");
    let report = painter.end(frame).expect("the frame finishes");
    let pixels = painter
        .read_pixels()
        .expect("readback")
        .expect("a software painter draws into memory and always has pixels");

    println!(
        "software: {} on {}, {drawn:?}",
        painter.name(),
        painter.backend()
    );

    // The report says what is actually running. Never "did it work": a painter that reported a GPU
    // here would be lying about the one thing this file exists to establish.
    assert_eq!(painter.backend().api, mjx_paint::GraphicsApi::None);
    assert_eq!(painter.backend().adapter, mjx_paint::AdapterKind::Cpu);
    assert!(!painter.backend().adapter.is_hardware());
    assert_eq!(report.sample_count, 1, "analytic coverage is one sample");

    assert_eq!(drawn.commands, 14, "every command of the page was walked");
    assert!(drawn.draw_calls >= 6, "and each of them drew: {drawn:?}");
    assert!(
        drawn.layers >= 2,
        "an opacity group and an effect: {drawn:?}"
    );
    assert!(drawn.glyphs >= 6, "the run's glyphs: {drawn:?}");
    assert_eq!(drawn.images, 1, "the picture: {drawn:?}");

    // Pixels, not "it did not error". A page rendered as one flat rectangle has pixels too; it does
    // not have hundreds of colours.
    assert_eq!(pixels.covered(), 200 * 150, "the page has a background");
    assert!(
        pixels.distinct_colors() > 100,
        "a page with a gradient, a hatch, a shadow and a picture on it is not {} colours",
        pixels.distinct_colors()
    );
}

#[test]
fn a_shadow_lands_behind_its_shape() {
    let plain = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::Blur,
    ))
    .expect("a blurred page draws");
    let _ = plain;
    let bare = common::one_rectangle(
        WIDTH as f32,
        HEIGHT as f32,
        SHAPE,
        common::rgb(0x20, 0x80, 0x40),
    );
    let (without, _) = render(&bare).expect("the shape alone draws");
    let (with, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::OuterShadow,
    ))
    .expect("the shadowed page draws");

    // **Outside the shape, toward the shadow's own direction, something appeared.**
    let (outside, _) = difference_in(&without, &with, lower_right());
    assert!(
        outside > 40,
        "a drop shadow at 45° must put ink below and right of its shape; only {outside} pixels of \
         that region changed"
    );

    // **And the shape's own interior did not.** This is the half that catches the ordering: a
    // shadow drawn *over* its shape covers the interior in shadow colour, and an assertion that
    // only looked outside would pass on it.
    let (interior, _) = difference_in(&without, &with, inside());
    assert_eq!(
        interior, 0,
        "the shadow reached inside the shape, which means it was drawn over the subtree rather \
         than behind it — the exact defect `draws_behind` names and, until MJXOFF-164, decided \
         nothing about"
    );
}

#[test]
fn a_shadow_is_drawn_in_the_effects_colour() {
    let (with, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::OuterShadow,
    ))
    .expect("the shadowed page draws");
    // The effect's colour is red and the shape's is green. A tint that read the input's RGB rather
    // than only its alpha would produce a green shadow, which is a shadow of the picture rather than
    // of its outline.
    let mut reddest = [0u8; 4];
    for y in SHAPE.bottom as u32 + 2..(SHAPE.bottom as u32 + 12).min(HEIGHT) {
        for x in SHAPE.right as u32 + 2..(SHAPE.right as u32 + 12).min(WIDTH) {
            let Some(pixel) = with.pixel(x, y) else {
                continue;
            };
            if pixel[3] > reddest[3] {
                reddest = pixel;
            }
        }
    }
    assert!(
        reddest[3] > 16,
        "there is no shadow in the shadow's own region at all: {reddest:02x?}"
    );
    assert!(
        reddest[0] > reddest[1] && reddest[0] > reddest[2],
        "the shadow is not the effect's red: {reddest:02x?}. A tint that read the subtree's colour \
         rather than its alpha would draw the shape's green here."
    );
}

#[test]
fn an_inner_shadow_lands_inside_its_shape() {
    let bare = common::one_rectangle(
        WIDTH as f32,
        HEIGHT as f32,
        SHAPE,
        common::rgb(0x20, 0x80, 0x40),
    );
    let (without, _) = render(&bare).expect("the shape alone draws");
    let (with, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::InnerShadow,
    ))
    .expect("the page draws");

    // Inside the shape's own edge, where an inner shadow lives.
    let rim = SceneRect::new(SHAPE.left, SHAPE.top, SHAPE.right, SHAPE.top + 8.0);
    let (inside_changed, _) = difference_in(&without, &with, rim);
    assert!(
        inside_changed > 40,
        "an inner shadow must darken the inside of its shape's edge; only {inside_changed} pixels \
         changed there"
    );

    // And **outside** it, nothing: an inner shadow that leaked outside would be an outer one.
    let (outside_changed, _) = difference_in(&without, &with, lower_right());
    assert_eq!(
        outside_changed, 0,
        "an inner shadow reached outside its shape, which is what an *outer* shadow does"
    );
}

#[test]
fn a_glow_reaches_every_side_and_a_reflection_only_downward() {
    let bare = common::one_rectangle(
        WIDTH as f32,
        HEIGHT as f32,
        SHAPE,
        common::rgb(0x20, 0x80, 0x40),
    );
    let (without, _) = render(&bare).expect("the shape alone draws");

    let (glowing, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::Glow,
    ))
    .expect("the glowing page draws");
    // A glow is undirected: it must reach above the shape as well as below it, which is exactly what
    // separates it from a drop shadow.
    let (glow_above, _) = difference_in(&without, &glowing, above());
    assert!(
        glow_above > 40,
        "a glow must reach above its shape; only {glow_above} pixels changed there. A glow that \
         took the shadow's offset would be a drop shadow with a different name."
    );
    // And it goes **behind**. Without this half the case is satisfied by a glow drawn over its own
    // shape: the region above the shape is empty either way, so the outward half alone does not see
    // the ordering. Flipping `draws_behind` for `Glow` fails here and nowhere else.
    let (glow_interior, _) = difference_in(&without, &glowing, inside());
    assert_eq!(
        glow_interior, 0,
        "the glow reached inside the shape, which means it was drawn over the subtree rather than \
         behind it"
    );

    let (reflected, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::Reflection,
    ))
    .expect("the reflected page draws");
    let below_before = covered_in(&without, below());
    let below_after = covered_in(&reflected, below());
    assert!(
        below_after > below_before + 100,
        "a reflection must put a copy of the shape below it: {below_before} covered pixels before \
         and {below_after} after"
    );
    let (reflection_above, _) = difference_in(&without, &reflected, above());
    assert_eq!(
        reflection_above, 0,
        "a reflection reached *above* its shape, which is the flip axis being the viewport's own \
         edge rather than the subtree's"
    );
}

#[test]
fn a_blur_softens_the_edge_and_a_soft_edge_only_the_edge() {
    let bare = common::one_rectangle(
        WIDTH as f32,
        HEIGHT as f32,
        SHAPE,
        common::rgb(0x20, 0x80, 0x40),
    );
    let (without, _) = render(&bare).expect("the shape alone draws");

    let (blurred, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::Blur,
    ))
    .expect("the blurred page draws");
    // A blur spreads the shape's ink outside its own outline. **And it replaces the subtree**: a
    // painter that drew the sharp shape as well would leave the interior untouched *and* spread —
    // so the interior is asserted to have changed too, which is what `replaces_subtree` decides.
    let outside = SceneRect::new(
        SHAPE.left,
        SHAPE.bottom + 1.0,
        SHAPE.right,
        SHAPE.bottom + 6.0,
    );
    let (spread, _) = difference_in(&without, &blurred, outside);
    assert!(
        spread > 40,
        "a blur must spread its shape's ink past the outline; {spread} pixels changed"
    );
    let edge = SceneRect::new(SHAPE.left, SHAPE.top, SHAPE.right, SHAPE.top + 3.0);
    let (softened, _) = difference_in(&without, &blurred, edge);
    assert!(
        softened > 40,
        "a blur must soften the shape's own edge; a sharp copy drawn over the blurred one would \
         leave it exactly as it was, and {softened} pixels changed"
    );

    let (soft, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::SoftEdge,
    ))
    .expect("the soft-edged page draws");
    // A soft edge feathers **inward**: it multiplies the subtree's alpha by a blurred copy of that
    // alpha, so the interior stays and the rim fades. It must not spread outside at all.
    let (soft_outside, _) = difference_in(&without, &soft, outside);
    assert_eq!(
        soft_outside, 0,
        "a soft edge reached outside its shape, which is a blur rather than a soft edge"
    );
    let (soft_edge, _) = difference_in(&without, &soft, edge);
    assert!(
        soft_edge > 40,
        "a soft edge must fade its shape's own rim; {soft_edge} pixels changed"
    );
}

#[test]
fn a_fill_overlay_recolours_the_shape_and_nothing_else() {
    let bare = common::one_rectangle(
        WIDTH as f32,
        HEIGHT as f32,
        SHAPE,
        common::rgb(0x20, 0x80, 0x40),
    );
    let (without, _) = render(&bare).expect("the shape alone draws");
    let (overlaid, _) = render(&common::one_shape_under(
        WIDTH as f32,
        HEIGHT as f32,
        EffectKind::FillOverlay,
    ))
    .expect("the overlaid page draws");

    let (interior, _) = difference_in(&without, &overlaid, inside());
    assert!(
        interior > 400,
        "a fill overlay must recolour the shape it is on: {interior} pixels changed"
    );
    let (outside, _) = difference_in(&without, &overlaid, lower_right());
    assert_eq!(
        outside, 0,
        "a fill overlay reached outside its shape, which no overlay does"
    );
}

#[test]
fn text_filled_with_a_gradient_is_not_one_colour() {
    // Hand-off 9 from R08: `a:textFill` with a gradient in it was reduced to one representative
    // colour rather than drawn wrongly, and the real answer was handed to this child. A run drawn in
    // one colour has one colour in it; a run drawn through a gradient does not.
    let list = common::gradient_text_page(240.0, 120.0);
    let mut painter = SoftwarePainter::new();
    let mut host = OffscreenSurface::new(240, 120, 1.0);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    let drawn = painter
        .draw(&frame, &list, &mut resources)
        .expect("the page draws");
    painter.end(frame).expect("the frame finishes");
    let pixels = painter.read_pixels().expect("readback").expect("pixels");

    assert!(
        drawn.layers >= 1,
        "a gradient-filled run must open a mask layer: {drawn:?}"
    );
    // The run runs left to right through a red-to-blue ramp, so its left end must be redder than its
    // right. A representative colour would make the two equal, which is precisely the behaviour this
    // replaces.
    let mut left_red = 0u32;
    let mut left_blue = 0u32;
    let mut right_red = 0u32;
    let mut right_blue = 0u32;
    for y in 40..80 {
        for x in 10..40 {
            if let Some(pixel) = pixels.pixel(x, y) {
                left_red += u32::from(pixel[0]);
                left_blue += u32::from(pixel[2]);
            }
        }
        for x in 100..130 {
            if let Some(pixel) = pixels.pixel(x, y) {
                right_red += u32::from(pixel[0]);
                right_blue += u32::from(pixel[2]);
            }
        }
    }
    println!("left r={left_red} b={left_blue}, right r={right_red} b={right_blue}");
    assert!(
        left_red > left_blue,
        "the left end of a red-to-blue run must be red: r={left_red} b={left_blue}"
    );
    assert!(
        right_blue > right_red,
        "and its right end blue: r={right_red} b={right_blue}. Equal ends would mean the run was \
         reduced to one representative colour, which is what R08 did and handed on."
    );
}
