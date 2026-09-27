//! **Where pixels first appear in this programme**, and the assertions that cannot be satisfied by
//! skipping the work.
//!
//! MJX-STAND-IN: this crate may not name `mjx-geometry` — `tests/the_seam_holds.rs` forbids it — so
//! the stand-in is the only `GeometryProvider` a painter's own suite can construct, and the
//! placeholder-warning cases are about the stand-in itself.
//!
//! # What every case here asserts, and why it is not "it rendered"
//!
//! `wgpu` will hand back a different backend from the one asked for, a software adapter on a machine
//! with no GPU, and a one-sample pipeline where four were requested — silently, in all three cases.
//! *"The frame rendered without error"* stays true through every one of them, and through a run
//! where the whole suite was skipped for want of an adapter.
//!
//! So every case below does three things:
//!
//! 1. names **which painter and which backend** actually ran, out of the live device
//!    ([`common::announce`]);
//! 2. asserts something about the **pixels** — coverage, a specific colour at a specific place, or a
//!    difference between two renders — never merely that a call returned `Ok`;
//! 3. and when there is no device, prints a **named skip** and fails outright under
//!    `MJX_REQUIRE_GPU=1`, which is what continuous integration sets.
//!
//! # Proved by mutation
//!
//! * Forcing the adapter set to [`wgpu::Backends::empty()`] →
//!   `tests/a_missing_gpu_is_a_loud_skip.rs` fails loudly rather than passing quietly, which is the
//!   proof that clause three is real.
//! * Making `WgpuPainter::draw` return `Ok(DrawReport::default())` before doing anything → every
//!   case here fails on coverage, because none of them asks whether the call succeeded.
//! * Dropping the `Pop` handling for an opacity layer → the opacity case fails on the composited
//!   alpha, which is the only thing that distinguishes a group from an ungrouped draw.

mod common;

use mjx_paint::{NoGlyphs, NoImages, OffscreenSurface, Painter, Resources, Viewport, WgpuPainter};
use mjx_scene::{PlaceholderGeometry, SceneRect, SceneTransform};

/// Render one list on a fresh painter and answer its pixels and its report.
fn render(
    painter: &mut WgpuPainter,
    list: &mjx_scene::DisplayList,
    width: u32,
    height: u32,
    scale: f32,
) -> (mjx_paint::Pixels, mjx_paint::DrawReport) {
    let mut host = OffscreenSurface::new(width, height, scale);
    let viewport = Viewport::covering(&host);
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let geometry = PlaceholderGeometry::new();
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    let drawn = painter
        .draw(&frame, list, &mut resources)
        .expect("the list draws");
    let report = painter.end(frame).expect("the frame ends");
    assert_eq!(report.frame, frame_id_of(&report));
    let pixels = painter
        .read_pixels()
        .expect("the frame reads back")
        .expect("an offscreen painter has pixels");
    (pixels, drawn)
}

fn frame_id_of(report: &mjx_paint::FrameReport) -> u64 {
    report.frame
}

#[test]
fn a_solid_rectangle_lands_where_the_display_list_put_it() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "a_solid_rectangle_lands_where_the_display_list_put_it",
                &why,
            )
        }
    };
    common::announce(
        "a_solid_rectangle_lands_where_the_display_list_put_it",
        &painter,
    );
    // The claim under test is *where*, not *whether*: a painter that filled the whole target would
    // satisfy "something was drawn" and fail every one of these.
    let ink = common::rgb(0x20, 0x80, 0xc0);
    let list = common::one_rectangle(64.0, 48.0, SceneRect::new(16.0, 12.0, 48.0, 36.0), ink);
    let (pixels, drawn) = render(&mut painter, &list, 64, 48, 1.0);

    assert_eq!(pixels.width, 64);
    assert_eq!(pixels.height, 48);
    assert!(drawn.draw_calls > 0, "the walk issued no draw at all");
    assert!(drawn.triangles > 0, "the rectangle became no triangles");

    let inside = pixels.pixel(32, 24).expect("a pixel in the middle");
    assert_eq!(
        [inside[0], inside[1], inside[2], inside[3]],
        [ink.red, ink.green, ink.blue, 0xff],
        "the middle of the rectangle is not the colour the list asked for"
    );
    for (x, y) in [(2u32, 2u32), (61, 2), (2, 45), (61, 45)] {
        let corner = pixels.pixel(x, y).expect("a pixel in a corner");
        assert_eq!(
            corner[3], 0,
            "({x}, {y}) is outside the rectangle and was painted anyway"
        );
    }
    // And it covers about the area it should: 32 x 24 of 64 x 48 is a quarter of the target.
    let covered = pixels.covered();
    let expected = 32 * 24;
    assert!(
        covered.abs_diff(expected) < expected / 8,
        "the rectangle covers {covered} pixels where {expected} were expected"
    );
}

#[test]
fn every_command_kind_renders_and_the_result_is_not_flat() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "every_command_kind_renders_and_the_result_is_not_flat",
                &why,
            )
        }
    };
    common::announce(
        "every_command_kind_renders_and_the_result_is_not_flat",
        &painter,
    );

    let list = common::every_command(160.0, 120.0);
    // Every one of the nine opcodes is in this list, checked here rather than believed: a helper
    // that quietly stopped emitting one would make the claim in this test's name false and nothing
    // else would notice.
    let mut kinds = std::collections::BTreeSet::new();
    for command in list.commands() {
        kinds.insert(command.opcode());
    }
    assert_eq!(
        kinds.len(),
        9,
        "the fixture uses {} of the nine command kinds: {kinds:?}",
        kinds.len()
    );

    let mut host = OffscreenSurface::new(160, 120, 1.0);
    let viewport = Viewport::covering(&host);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    let drawn = painter
        .draw(&frame, &list, &mut resources)
        .expect("a page of every command draws");
    let report = painter.end(frame).expect("the frame ends");
    let pixels = painter
        .read_pixels()
        .expect("it reads back")
        .expect("offscreen pixels");

    assert_eq!(drawn.commands, list.commands().count());
    assert!(drawn.draw_calls >= 6, "only {} draws", drawn.draw_calls);
    assert!(drawn.glyphs >= 6, "only {} glyphs", drawn.glyphs);
    assert_eq!(drawn.images, 1);
    assert!(drawn.clips >= 1);
    assert!(
        drawn.layers >= 2,
        "an opacity group and an effect group are two layers; {} were opened",
        drawn.layers
    );
    assert!(
        drawn.atlas_bytes_uploaded >= glyphs.byte_len(),
        "the atlas page was not uploaded: {} bytes",
        drawn.atlas_bytes_uploaded
    );
    assert_eq!(
        report.sample_count,
        painter.backend().antialiasing.sample_count()
    );

    // **Not merely "no error".** A page of a background, a gradient, a hatch, a picture, a shadow
    // and a run of glyphs is many colours; a painter that drew one flat rectangle and returned
    // successfully would pass every count above and fail here.
    assert!(
        pixels.covered() > (160 * 120) / 2,
        "only {} of {} pixels were painted",
        pixels.covered(),
        160 * 120
    );
    let colours = pixels.distinct_colors();
    assert!(
        colours > 32,
        "the page has {colours} distinct colours in it, which is a flat fill rather than a page"
    );
}

#[test]
fn the_scale_factor_changes_the_target_and_where_an_edge_lands() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "the_scale_factor_changes_the_target_and_where_an_edge_lands",
                &why,
            )
        }
    };
    common::announce(
        "the_scale_factor_changes_the_target_and_where_an_edge_lands",
        &painter,
    );

    // The identity-value sweep, on `SurfaceHost::scale_factor`. If it is only ever `1.0`, nothing
    // notices its absence — so the same list is drawn at `1.0` and at `2.0`, and both the size of
    // the target and the pixel an edge lands on have to move.
    let ink = common::rgb(0xc0, 0x30, 0x30);
    let list = common::one_rectangle(40.0, 40.0, SceneRect::new(0.0, 0.0, 20.0, 20.0), ink);

    let (one, _) = render(&mut painter, &list, 40, 40, 1.0);
    let (two, _) = render(&mut painter, &list, 40, 40, 2.0);

    assert_eq!((one.width, one.height), (40, 40));
    assert_eq!(
        (two.width, two.height),
        (80, 80),
        "a 2.0 scale factor must allocate twice the framebuffer, or a Retina page is drawn at half \
         size and stretched"
    );
    // The rectangle is in *display-list* coordinates, so it stays 20x20 device pixels at both
    // scales — the framebuffer grows and the page does not. A painter that multiplied the geometry
    // by the scale factor as well would double it, which is the other half of getting this wrong.
    assert!(
        one.pixel(10, 10).is_some_and(|p| p[3] == 0xff),
        "the shape is missing at 1.0"
    );
    assert!(
        two.pixel(10, 10).is_some_and(|p| p[3] == 0xff),
        "the shape is missing at 2.0"
    );
    assert!(
        two.pixel(30, 30).is_some_and(|p| p[3] == 0),
        "at 2.0 the extra framebuffer beyond the page must be empty, not stretched into"
    );
}

#[test]
fn an_opacity_group_composites_and_the_identity_costs_nothing() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "an_opacity_group_composites_and_the_identity_costs_nothing",
                &why,
            )
        }
    };
    common::announce(
        "an_opacity_group_composites_and_the_identity_costs_nothing",
        &painter,
    );

    let ink = common::rgb(0xff, 0x00, 0x00);
    let rect = SceneRect::new(4.0, 4.0, 28.0, 28.0);
    let opaque = common::one_rectangle_at_opacity(32.0, 32.0, rect, ink, 1.0);
    let half = common::one_rectangle_at_opacity(32.0, 32.0, rect, ink, 0.5);

    let (opaque_pixels, opaque_report) = render(&mut painter, &opaque, 32, 32, 1.0);
    let (half_pixels, half_report) = render(&mut painter, &half, 32, 32, 1.0);

    // The identity opens no layer. That is the difference between a page costing one render target
    // and one per group, and a scene builder emits `PushOpacity` for every fragment that carries an
    // opacity at all.
    assert_eq!(
        opaque_report.layers, 0,
        "PushOpacity(1.0) opened {} offscreen layer(s); the identity must cost nothing",
        opaque_report.layers
    );
    assert_eq!(half_report.layers, 1, "PushOpacity(0.5) must open a layer");

    let solid = opaque_pixels.pixel(16, 16).expect("the middle");
    let faded = half_pixels.pixel(16, 16).expect("the middle");
    assert_eq!(solid[3], 0xff);
    assert!(
        (110..=145).contains(&faded[3]),
        "half of an opaque red should composite at about half alpha; it came out {}",
        faded[3]
    );
    assert!(
        faded[0] < solid[0],
        "premultiplied red at half alpha must be darker than at full: {} vs {}",
        faded[0],
        solid[0]
    );
}

#[test]
fn a_clip_stops_the_paint_at_its_own_edge() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => return common::skip("a_clip_stops_the_paint_at_its_own_edge", &why),
    };
    common::announce("a_clip_stops_the_paint_at_its_own_edge", &painter);

    let ink = common::rgb(0x00, 0x80, 0x00);
    // A shape that covers the whole page, clipped to a quarter of it. A painter with no clip at all
    // fills everything; one whose stencil is inverted fills nothing.
    let list = common::one_rectangle_clipped(
        40.0,
        40.0,
        SceneRect::new(0.0, 0.0, 40.0, 40.0),
        ink,
        SceneRect::new(0.0, 0.0, 20.0, 20.0),
    );
    let (pixels, report) = render(&mut painter, &list, 40, 40, 1.0);

    assert_eq!(report.clips, 1);
    assert!(
        pixels.pixel(10, 10).is_some_and(|p| p[3] == 0xff),
        "inside the clip is unpainted"
    );
    assert!(
        pixels.pixel(30, 10).is_some_and(|p| p[3] == 0),
        "outside the clip, to the right, was painted"
    );
    assert!(
        pixels.pixel(10, 30).is_some_and(|p| p[3] == 0),
        "outside the clip, below, was painted"
    );
    let covered = pixels.covered();
    assert!(
        (300..=460).contains(&covered),
        "a 20x20 clip on a 40x40 page should cover about 400 pixels; it covered {covered}"
    );
}

#[test]
fn nested_clips_intersect_and_a_pop_really_widens_the_clip_back() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "nested_clips_intersect_and_a_pop_really_widens_the_clip_back",
                &why,
            )
        }
    };
    common::announce(
        "nested_clips_intersect_and_a_pop_really_widens_the_clip_back",
        &painter,
    );

    // Two clips, and a second shape drawn over the whole outer clip after the inner one is popped.
    // The colour that survives *inside the inner clip* is the assertion:
    //
    //   * a correct pop puts that region back inside the outer clip, so the second shape covers it;
    //   * a pop that incremented instead of decrementing leaves it at a stencil value nothing tests
    //     against, so the first shape shows through;
    //   * a push and a pop that disagree by one paint nothing at all.
    //
    // The simpler version of this case — a second shape placed only *outside* the inner clip — was
    // a **green mutation**: that region happens to hold the right value either way.
    let before = common::rgb(0xc0, 0x20, 0x20);
    let after = common::rgb(0x20, 0x20, 0xc0);
    let outer = SceneRect::new(0.0, 0.0, 60.0, 60.0);
    let inner = SceneRect::new(0.0, 0.0, 30.0, 60.0);
    let list = common::two_clips_and_a_shape_drawn_after_the_inner_one_is_popped(
        80.0, 80.0, outer, inner, before, after,
    );
    let (pixels, report) = render(&mut painter, &list, 80, 80, 1.0);
    assert_eq!(report.clips, 2);

    let inside_both = pixels.pixel(15, 30).expect("a pixel inside both clips");
    assert_eq!(
        [inside_both[0], inside_both[1], inside_both[2]],
        [after.red, after.green, after.blue],
        "inside the inner clip the later shape must have covered the earlier one. It did not, which \
         means the `Pop` did not widen the clip back — and the region is still being tested against \
         the inner clip's own stencil value."
    );
    let outer_only = pixels
        .pixel(45, 30)
        .expect("a pixel inside the outer clip only");
    assert_eq!(
        [outer_only[0], outer_only[1], outer_only[2]],
        [after.red, after.green, after.blue],
        "the later shape is missing from the part of the outer clip the inner one never covered"
    );
    assert!(
        pixels.pixel(70, 30).is_some_and(|p| p[3] == 0),
        "paint escaped the outer clip"
    );
    assert!(
        pixels.pixel(30, 70).is_some_and(|p| p[3] == 0),
        "paint escaped the outer clip downward"
    );
    // And the first shape really was clipped when it was drawn: if the inner clip had done nothing,
    // the first fill would have covered the whole outer clip and the pixel counts would still come
    // out the same, so the *area* is asserted too.
    let covered = pixels.covered();
    assert!(
        (3_200..=3_900).contains(&covered),
        "a 60x60 outer clip should end up covering about 3600 pixels; it covered {covered}"
    );
}

#[test]
fn a_transform_moves_the_shape_and_the_identity_does_not() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "a_transform_moves_the_shape_and_the_identity_does_not",
                &why,
            )
        }
    };
    common::announce(
        "a_transform_moves_the_shape_and_the_identity_does_not",
        &painter,
    );

    // The identity-value sweep, on `PushTransform`. A painter that ignored the matrix entirely would
    // draw both of these the same way and pass any test that only looked at one.
    let ink = common::rgb(0x00, 0x00, 0xff);
    let rect = SceneRect::new(0.0, 0.0, 12.0, 12.0);
    let still = common::one_rectangle_transformed(48.0, 48.0, rect, ink, SceneTransform::IDENTITY);
    let moved = common::one_rectangle_transformed(
        48.0,
        48.0,
        rect,
        ink,
        SceneTransform {
            scale_x: 1.0,
            shear_y: 0.0,
            shear_x: 0.0,
            scale_y: 1.0,
            translate_x: 24.0,
            translate_y: 24.0,
        },
    );

    let (unmoved, _) = render(&mut painter, &still, 48, 48, 1.0);
    let (shifted, _) = render(&mut painter, &moved, 48, 48, 1.0);

    assert!(unmoved.pixel(6, 6).is_some_and(|p| p[3] == 0xff));
    assert!(unmoved.pixel(30, 30).is_some_and(|p| p[3] == 0));
    assert!(
        shifted.pixel(6, 6).is_some_and(|p| p[3] == 0),
        "the translated shape is still at the origin"
    );
    assert!(
        shifted.pixel(30, 30).is_some_and(|p| p[3] == 0xff),
        "the translated shape did not arrive where the matrix put it"
    );
    // The same ink covers the same area either way, so the difference is the position and not the
    // shape — which is what makes this a translation rather than a coincidence.
    assert_eq!(unmoved.covered(), shifted.covered());
}

#[test]
fn a_placeholder_shape_is_reported_and_is_painted_as_a_warning() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "a_placeholder_shape_is_reported_and_is_painted_as_a_warning",
                &why,
            )
        }
    };
    common::announce(
        "a_placeholder_shape_is_reported_and_is_painted_as_a_warning",
        &painter,
    );

    // The other half of MJXOFF-163's provenance fix: a painter can now tell a stand-in from the
    // document's own shape, and a caller can act on it. R10 asserts `placeholders == 0` before
    // calling a render a fidelity render. Since MJXOFF-206 a preset shape resolves to the
    // document's own geometry, so a stand-in is the honest answer for a handle nobody registered
    // rather than the ordinary case — which makes counting it more important, not less: it is now
    // a signal instead of a constant.
    let mut builder = mjx_scene::SceneBuilder::new(mjx_text::DeviceScale::UNZOOMED, 48.0, 48.0);
    let geometry = builder
        .add_geometry(&mjx_scene::Geometry::Unresolved {
            outline: 7,
            bounds: SceneRect::new(6.0, 6.0, 42.0, 42.0),
        })
        .expect("an unresolved shape");
    let paint = builder
        .add_paint(mjx_scene::Paint::Solid(common::rgb(0x00, 0x80, 0x00)))
        .expect("a paint");
    builder
        .push(mjx_scene::Command::FillPath { geometry, paint })
        .expect("a fill");
    let list = builder.finish().expect("well formed");

    let (pixels, drawn) = render(&mut painter, &list, 48, 48, 1.0);
    assert_eq!(
        drawn.placeholders, 1,
        "a shape resolved by the stand-in provider must be counted, or a page of placeholders can \
         be recorded as a fidelity render"
    );

    // And the stroked half of the same counter — two lines in `plan.rs`, and until MJXOFF-206 only
    // the fill was ever reached.
    let stroked = common::one_unresolved_stroked_shape(48.0, 48.0);
    let (_, stroked_report) = render(&mut painter, &stroked, 48, 48, 1.0);
    assert_eq!(
        stroked_report.placeholders, 1,
        "the GPU painter must count a *stroked* stand-in too: {stroked_report:?}"
    );

    // And it is visibly a warning rather than the green the document asked for.
    let mut found_warning = false;
    for y in 0..48 {
        for x in 0..48 {
            if let Some(pixel) = pixels.pixel(x, y) {
                if pixel[3] != 0 && pixel[0] > 0x80 && pixel[2] > 0x80 && pixel[1] < 0x40 {
                    found_warning = true;
                }
            }
        }
    }
    assert!(
        found_warning,
        "a stand-in shape must be painted in the warning colour so a reviewer can see it is one"
    );
}

#[test]
fn a_real_frame_actually_uses_the_texture_pool_and_a_small_budget_makes_it_evict() {
    // **The pool's own suite proves the pool works; this proves the painter uses it.** A budget
    // exercised only by a test against a stand-in texture would be a budget the renderer never
    // reaches — the same defect one layer up as a field that is written and never read, which is
    // what MJXOFF-163 had to fix in `mjx-scene`.
    let base = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "a_real_frame_actually_uses_the_texture_pool_and_a_small_budget_makes_it_evict",
                &why,
            )
        }
    };
    common::announce(
        "a_real_frame_actually_uses_the_texture_pool_and_a_small_budget_makes_it_evict",
        &base,
    );

    // A budget of two full-viewport targets, against a page whose opacity group, effect group and
    // blur intermediates need more than that at once.
    const SIDE: u32 = 96;
    let one_target = (SIDE * SIDE * 4) as usize;
    let mut painter = base.with_texture_budget(one_target * 2);
    let list = common::every_command(SIDE as f32, SIDE as f32);

    let mut host = OffscreenSurface::new(SIDE, SIDE, 1.0);
    let viewport = Viewport::covering(&host);
    let mut glyphs = common::ChequeredAtlas::new();
    let images = common::OnePicture::new();
    let geometry = PlaceholderGeometry::new();

    let mut last = None;
    // Two frames, because the second is where a *hit* becomes possible: a pool that created every
    // target afresh every frame would satisfy any single-frame assertion and be a pool in name only.
    for _ in 0..2 {
        let frame = painter.begin(&mut host, viewport).expect("a frame opens");
        let mut resources = Resources::new(&mut glyphs, &geometry, &images);
        painter
            .draw(&frame, &list, &mut resources)
            .expect("the page draws");
        last = Some(painter.end(frame).expect("the frame ends"));
    }
    let report = last.expect("two frames were drawn");
    let pool = report.pool;

    assert!(
        pool.misses > 0,
        "the painter acquired no render target at all, so the pool is not on the rendering path"
    );
    assert!(
        pool.hits > 0,
        "the second frame reused nothing: {pool:?}. A pool that creates every target afresh is a \
         pool in name only, and the byte budget then bounds something nobody allocates."
    );
    assert!(
        pool.evictions > 0 || pool.oversized > 0,
        "a budget of two targets against a page needing more did not force the pool to give \
         anything up: {pool:?}"
    );
    assert!(
        painter.retained_texture_bytes() <= one_target * 2,
        "the pool retains {} bytes against a budget of {}",
        painter.retained_texture_bytes(),
        one_target * 2
    );
    // And the page still rendered: a pool that met its budget by refusing every target would
    // satisfy every count above and draw nothing.
    let pixels = painter
        .read_pixels()
        .expect("it reads back")
        .expect("offscreen pixels");
    assert!(
        pixels.covered() > ((SIDE * SIDE) / 2) as usize,
        "only {} of {} pixels were painted under a tight texture budget",
        pixels.covered(),
        SIDE * SIDE
    );
    println!("pool after two frames: {pool:?}");
}

#[test]
fn a_frame_token_belongs_to_one_frame_and_one_painter() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip("a_frame_token_belongs_to_one_frame_and_one_painter", &why)
        }
    };
    common::announce(
        "a_frame_token_belongs_to_one_frame_and_one_painter",
        &painter,
    );

    let mut host = OffscreenSurface::new(16, 16, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");

    // A second `begin` while one is open is an error naming the frame that is open, not a silent
    // replacement that loses the first frame's work.
    match painter.begin(&mut host, viewport) {
        Err(mjx_paint::PaintError::FrameAlreadyOpen { open }) => assert_eq!(open, frame.id()),
        other => panic!("a second `begin` answered {other:?}"),
    }

    let report = painter.end(frame).expect("the frame ends");
    // And a token that has been ended is not accepted again.
    match painter.end(mjx_paint::Frame::new(report.frame, viewport)) {
        Err(mjx_paint::PaintError::WrongFrame { given, open }) => {
            assert_eq!(given, report.frame);
            assert_eq!(open, None);
        }
        other => panic!("ending a spent frame answered {other:?}"),
    }
}

#[test]
fn a_viewport_with_no_pixels_is_refused_before_anything_is_allocated() {
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "a_viewport_with_no_pixels_is_refused_before_anything_is_allocated",
                &why,
            )
        }
    };
    common::announce(
        "a_viewport_with_no_pixels_is_refused_before_anything_is_allocated",
        &painter,
    );

    // A minimised window and a zero-height flex row both produce this, and both are ordinary
    // runtime events rather than bugs.
    let mut host = OffscreenSurface::new(0, 40, 1.0);
    let viewport = Viewport::covering(&host);
    match painter.begin(&mut host, viewport) {
        Err(mjx_paint::PaintError::EmptyViewport { width, height }) => {
            assert_eq!((width, height), (0, 40));
        }
        other => panic!("an empty viewport answered {other:?}"),
    }
}
