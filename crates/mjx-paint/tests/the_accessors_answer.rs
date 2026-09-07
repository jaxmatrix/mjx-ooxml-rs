//! The accessors on this crate's public surface, each with a caller that **asserts what it says**.
//!
//! MJX-STAND-IN: this crate may not name `mjx-geometry` — `tests/the_seam_holds.rs` forbids it — so
//! the stand-in is the only `GeometryProvider` a painter's own suite can construct. Nothing here
//! asks what it draws; it is present because `Resources::new` requires a provider.
//!
//! # Why this file exists rather than a line of `let _ =`
//!
//! `the_public_surface_is_reachable.rs` found sixteen public items that nothing in this crate
//! named — eight of them R08's, and every one of them an accessor a shell or a suite will
//! eventually want. The cheap way to make a reachability gate green is to mention the names; that
//! turns the gate into a formality and leaves the accessors exactly as unproven as before.
//!
//! So each one below is asked a question whose answer would be **wrong** if the accessor were wired
//! to the wrong field. `pool_statistics` is checked against a frame that is known to have acquired
//! targets; `physical_x` against a viewport whose scale is known not to be one; `last_frame`
//! against the report `end` already returned. A getter that answered its neighbour's value fails
//! here and passes a mention.
//!
//! # What was found by writing it
//!
//! Every accessor in this file was reachable from nowhere before MJXOFF-164:
//!
//! * `Viewport::physical_x` and `physical_y` — the *scissor* half of the viewport's arithmetic. Its
//!   partners `physical_width`/`physical_height` were exercised at two scale factors by R08's own
//!   identity sweep; these two were never called at all, so a viewport that is not at the page's
//!   origin had never been converted to device pixels by anything.
//! * `DesktopWindow::requesting_redraws_through` — the whole of how a shell's event loop is reached
//!   from a lost surface. `SurfaceHost::request_redraw`'s two branches were half-covered:
//!   `OffscreenSurface` answers `false` and nothing had ever built a host that answers `true`.
//! * `TexturePool::resident_bytes` and `in_flight_len` — the two halves of the pool's own accounting
//!   that its byte-budget gate does not read.
//! * `WgpuPainter::graphics_device`, `pool_statistics` and `last_frame`.
//! * `PaintError::Scene`, which is constructed only by `#[from]` and so is never spelled anywhere.
//! * `CLIP_FILL_RULE`, which stated the rule a clip's own outline is filled with and was read by
//!   nothing, including the code that fills a clip's outline.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use mjx_paint::{
    DesktopWindow, OffscreenSurface, PaintError, Painter, Resources, SoftwarePainter, SurfaceHost,
    SurfaceTarget, TexturePool, TextureSize, Viewport,
};
use mjx_scene::PlaceholderGeometry;

#[test]
fn a_viewport_converts_its_origin_as_well_as_its_size() {
    // `physical_width` and `physical_height` were swept at two scale factors; `physical_x` and
    // `physical_y` were called by nothing. A frame drawn into part of a surface needs all four, and
    // a scissor rectangle placed with the logical origin is off by the scale factor.
    let viewport = Viewport {
        x: 10.5,
        y: 20.25,
        width: 100.0,
        height: 50.0,
        scale_factor: 2.0,
    };
    assert_eq!(viewport.physical_x(), 21, "10.5 logical at 2x is 21 device");
    assert_eq!(
        viewport.physical_y(),
        41,
        "20.25 at 2x is 40.5, which rounds to 41 — truncating would drop the row"
    );
    assert_eq!(viewport.physical_width(), 200);
    assert_eq!(viewport.physical_height(), 100);

    // And at the identity scale, which is the value that makes the conversion a no-op. Both must be
    // exercised or the multiply is unobservable.
    let unscaled = Viewport {
        scale_factor: 1.0,
        ..viewport
    };
    assert_eq!(unscaled.physical_x(), 11, "10.5 rounds up");
    assert_eq!(unscaled.physical_y(), 20);
    assert_ne!(
        unscaled.physical_x(),
        viewport.physical_x(),
        "the scale factor must reach the origin as well as the size"
    );
}

#[test]
fn a_host_that_can_ask_for_a_frame_says_so_and_one_that_cannot_says_so() {
    // Both branches of `request_redraw`. `OffscreenSurface` answers `false` and was covered; nothing
    // had ever built a host that answers `true`, so the branch a real shell takes was untested and
    // `requesting_redraws_through` — the only way to build one — was called by nobody.
    let offscreen = OffscreenSurface::new(80, 60, 1.0);
    assert!(
        !offscreen.request_redraw(),
        "there is no event loop behind an offscreen target, and saying otherwise would let a \
         painter report that it asked for a frame nothing will deliver"
    );
    assert!(matches!(offscreen.raw_handle(), SurfaceTarget::Offscreen));

    let asked = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&asked);
    let handles = mjx_paint::WindowHandles {
        // A handle that describes no window. It is never dereferenced here: this case exercises the
        // host's bookkeeping, and `DesktopWindow::new` is safe precisely because acting on the
        // promise is the painter's `unsafe` block and not the constructor's.
        window: wgpu::rwh::RawWindowHandle::Web(wgpu::rwh::WebWindowHandle::new(1)),
        display: None,
    };
    let window = DesktopWindow::new(handles, 800, 600, 2.0).requesting_redraws_through(Box::new(
        move || {
            counter.fetch_add(1, Ordering::Relaxed);
        },
    ));
    assert_eq!(window.size(), (800, 600));
    assert_eq!(window.scale_factor(), 2.0);
    assert!(matches!(window.raw_handle(), SurfaceTarget::Window(_)));
    assert!(
        window.request_redraw(),
        "a host with an event loop answers `true`"
    );
    assert_eq!(
        asked.load(Ordering::Relaxed),
        1,
        "and actually calls it — a host that answered `true` without asking would be the same lie \
         one that answered `false` after asking would be"
    );
    // `Debug` says whether it can, which is what a log line needs and what a shell debugging a
    // frozen canvas will read first.
    assert!(format!("{window:?}").contains("can_request_redraws: true"));
}

#[test]
fn the_pool_accounts_for_what_it_holds_and_what_is_out() {
    let mut pool: TexturePool<Vec<u8>> = TexturePool::with_budget(1 << 20);
    assert_eq!(pool.resident_bytes(), 0);
    assert_eq!(pool.in_flight_len(), 0);
    assert_eq!(pool.retained_len(), 0);

    let size = TextureSize::new(64, 64);
    let first = pool
        .acquire(size, |size| Ok(vec![0u8; size.byte_len()]))
        .expect("a target");
    let second = pool
        .acquire(size, |size| Ok(vec![0u8; size.byte_len()]))
        .expect("another");
    assert_eq!(
        pool.in_flight_len(),
        2,
        "two targets are out, and `in_flight_len` is the only accessor that says so"
    );
    assert_eq!(
        pool.resident_bytes(),
        size.byte_len() * 2,
        "and `resident_bytes` counts them — retained plus in flight, which is what a caller \
         watching a frame's peak needs and is not the same number as `retained_bytes`"
    );
    assert_eq!(
        pool.retained_bytes(),
        0,
        "nothing is retained while everything is out"
    );

    pool.release(first).expect("a live handle");
    pool.release(second).expect("a live handle");
    assert_eq!(pool.in_flight_len(), 0);
    assert_eq!(pool.retained_len(), 2);
    assert_eq!(pool.resident_bytes(), pool.retained_bytes());
}

#[test]
fn a_painter_keeps_its_last_report_for_a_caller_that_dropped_it() {
    let list = common::one_rectangle(
        50.0,
        50.0,
        mjx_scene::SceneRect::new(5.0, 5.0, 45.0, 45.0),
        common::rgb(0x30, 0x50, 0x70),
    );
    let mut painter = SoftwarePainter::new();
    assert!(
        painter.last_frame().is_none(),
        "a painter that has drawn nothing has no last frame"
    );
    let mut host = OffscreenSurface::new(50, 50, 1.0);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    painter
        .draw(&frame, &list, &mut resources)
        .expect("the page draws");
    let returned = painter.end(frame).expect("the frame finishes");
    assert_eq!(
        painter.last_frame(),
        Some(returned),
        "the kept report must be the one `end` answered with, not a fresh empty one"
    );
    // And the pool's statistics say the frame actually acquired something, which is what separates
    // a painter that rendered from one that returned a default.
    let statistics = painter.pool_statistics();
    assert!(
        statistics.misses > 0,
        "the frame's first layer had to be created: {statistics:?}"
    );
    assert!(
        statistics.peak_in_flight_bytes >= 50 * 50 * 4,
        "and it was at least a viewport: {statistics:?}"
    );
    assert!(
        painter.retained_texture_bytes() > 0,
        "and it was given back"
    );
}

#[test]
fn the_exporters_keep_their_reports_too() {
    let list = common::one_rectangle(
        40.0,
        40.0,
        mjx_scene::SceneRect::new(4.0, 4.0, 36.0, 36.0),
        common::rgb(0x80, 0x30, 0x30),
    );
    let library = common::liberation_library();
    let geometry = PlaceholderGeometry::new();
    let images = common::OnePicture::new();

    let mut svg = mjx_paint::SvgPainter::new();
    assert!(svg.last_frame().is_none());
    let mut host = OffscreenSurface::new(40, 40, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = svg.begin(&mut host, viewport).expect("a frame opens");
    let mut glyphs = common::ChequeredAtlas::new();
    let mut resources = Resources::new(&mut glyphs, &geometry, &images).with_fonts(&library);
    svg.draw(&frame, &list, &mut resources).expect("it draws");
    let returned = svg.end(frame).expect("it finishes");
    assert_eq!(svg.last_frame(), Some(returned));
    assert_eq!(returned.width, 40);

    let mut pdf = mjx_paint::PdfPainter::new();
    assert!(pdf.last_frame().is_none());
    let mut host = OffscreenSurface::new(40, 40, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = pdf.begin(&mut host, viewport).expect("a frame opens");
    let mut glyphs = common::ChequeredAtlas::new();
    let mut resources = Resources::new(&mut glyphs, &geometry, &images).with_fonts(&library);
    pdf.draw(&frame, &list, &mut resources).expect("it draws");
    let returned = pdf.end(frame).expect("it finishes");
    assert_eq!(pdf.last_frame(), Some(returned));
    assert_eq!(returned.drawn.draw_calls, 1);
}

#[test]
fn the_gpu_painter_hands_out_its_device_and_keeps_its_report() {
    const CASE: &str = "the GPU painter hands out its device and keeps its report";
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            common::skip(CASE, &why);
            return;
        }
    };
    common::announce(CASE, &painter);
    assert!(painter.last_frame().is_none());

    // The escape hatch a shell needs when it wants to ask the device something this surface does not
    // expose — the limits, the features, a second queue. Nothing had ever called it.
    let device = painter.graphics_device();
    assert!(
        device.max_texture_size() >= 2048,
        "every adapter in this platform's matrix makes a 2048-pixel texture: {}",
        device.max_texture_size()
    );
    assert_eq!(
        device.report().api,
        painter.backend().api,
        "the device the painter hands out must be the device it is drawing on"
    );

    let list = common::one_rectangle(
        60.0,
        60.0,
        mjx_scene::SceneRect::new(6.0, 6.0, 54.0, 54.0),
        common::rgb(0x20, 0x70, 0x40),
    );
    let mut host = OffscreenSurface::new(60, 60, 1.0);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    painter
        .draw(&frame, &list, &mut resources)
        .expect("it draws");
    let returned = painter.end(frame).expect("it finishes");
    assert_eq!(painter.last_frame(), Some(returned));
    assert!(painter.pool_statistics().misses > 0);
    assert!(painter.retained_texture_bytes() > 0);
}

#[test]
fn a_scene_error_reaches_a_caller_as_a_paint_error() {
    // `PaintError::Scene` is constructed only by `#[from]`, so the variant's own name appears
    // nowhere in the crate and a reachability gate cannot see it. It is nonetheless the arm every
    // tessellation failure and every malformed-list failure comes back through, which is what this
    // asserts by causing one.
    let error: PaintError = mjx_scene::SceneError::Tessellation { reason: "a probe" }.into();
    assert!(
        matches!(error, PaintError::Scene(_)),
        "a scene failure must reach a painter's caller as one: {error}"
    );
    assert!(
        error.to_string().contains("display list"),
        "and must say where it came from: {error}"
    );
}

#[test]
fn a_clip_is_filled_by_the_rule_the_crate_states() {
    // `CLIP_FILL_RULE` said what rule a clip's own outline is filled with and was read by nothing —
    // including the code that fills a clip's outline. It is the document's own answer for a clip
    // that carries no geometry of its own, and this is what ties the constant to it.
    let mut builder = mjx_scene::SceneBuilder::new(mjx_text::DeviceScale::UNZOOMED, 40.0, 40.0);
    let clip = builder
        .add_clip(mjx_scene::Clip::rectangle(mjx_scene::SceneRect::new(
            4.0, 4.0, 36.0, 36.0,
        )))
        .expect("a clip");
    let geometry = builder
        .add_geometry(&common::box_path(mjx_scene::SceneRect::new(
            0.0, 0.0, 40.0, 40.0,
        )))
        .expect("a shape");
    let paint = builder
        .add_paint(mjx_scene::Paint::Solid(common::rgb(0, 0, 0)))
        .expect("a paint");
    builder
        .push(mjx_scene::Command::PushClip(clip))
        .expect("a clip");
    builder
        .push(mjx_scene::Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(mjx_scene::Command::Pop).expect("it closes");
    let list = builder.finish().expect("a well-formed scene");

    let mut tessellator = mjx_scene::Tessellator::new();
    let plan = mjx_paint::plan_frame_with(
        &list,
        &PlaceholderGeometry::new(),
        &mut tessellator,
        mjx_paint::PlanOptions::for_vector(),
    )
    .expect("the list lowers");
    let rule = plan
        .layer(0)
        .expect("a root")
        .ops
        .iter()
        .find_map(|op| match op {
            mjx_paint::DrawOp::PushClip { outline, .. } => {
                outline.as_ref().map(|outline| outline.fill_rule)
            }
            _ => None,
        })
        .expect("a clip");
    assert_eq!(
        rule,
        mjx_paint::plan::CLIP_FILL_RULE,
        "a clip that carries no geometry is a rectangle, and a rectangle is wound once — which is \
         what the constant says and what nothing checked"
    );
}
