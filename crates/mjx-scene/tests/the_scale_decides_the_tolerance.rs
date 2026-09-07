//! The scale a path will be *drawn* at decides how finely it is flattened — asserted on the
//! consequence, not on the getter.
//!
//! # The trap this file exists to defeat, and how it was found
//!
//! `TessellationOptions::for_glyph_run` divides the tolerance by the run's residual scale, and
//! **deleting that division was a green mutation**: `cargo test -p mjx-scene` came back clean.
//!
//! It was not merely untested — it was *unobservable*. A probe that aborted whenever a residual
//! other than `1.0` reached the function **never fired**, because no test anywhere supplied one. The
//! line executed constantly and hid the defect perfectly, since `tolerance / 1.0` is `tolerance`.
//! That is the difference between *reachability* and *observability*, and only the second is
//! evidence.
//!
//! It matters more than its size suggests. The outline route exists **because** a glyph is too large
//! for the atlas — [`OUTLINE_PIXELS_PER_EM_THRESHOLD`](mjx_text::OUTLINE_PIXELS_PER_EM_THRESHOLD) is
//! the whole point — so the residual factor is non-trivial exactly when this code runs in a real
//! document and never in a test that takes the default. Get it wrong and large glyphs go visibly
//! polygonal at zoom: a fidelity defect R10's golden images would eventually catch, at a far higher
//! cost than a test here.
//!
//! # So every assertion below is a *relationship*, and most reach the triangles
//!
//! A test that read `options.tolerance()` back and compared it to a formula would restate the
//! implementation and pin nothing. What is asserted instead is that a run drawn larger is flattened
//! **finer**, that finer flattening yields **more vertices for the same curve**, and that the clamps
//! hold at both extremes. Those are the claims the module documentation makes in prose.
//!
//! # Proved by mutation
//!
//! * `for_glyph_run` ignoring `residual_scale` → the ordering tests fail on both the tolerance and
//!   the vertex counts.
//! * `page_bucket` ignoring the device scale → a page at two zooms buckets identically.
//! * `clamp_tolerance` not clamping → the extremes fail, and an unclamped `1e-30` is an unbounded
//!   flattening rather than a fine one.

use mjx_scene::{
    page_bucket, DeviceScale, FillRule, Geometry, GlyphImage, Hinting, PathCommand,
    PlaceholderGeometry, ScaleBucket, SceneGlyph, SceneGlyphRun, ScenePoint, TessellationOptions,
    Tessellator, TextDirection, MAXIMUM_TOLERANCE, MINIMUM_TOLERANCE, REFERENCE_EM_POINTS,
    TOLERANCE_DEVICE_PIXELS,
};

/// A glyph run carrying nothing but the two numbers this file is about.
///
/// Built by hand rather than laid out, because the point is to supply a residual scale that **is
/// not one** — which is exactly what no test in this crate did before, and why the defect this file
/// gates was invisible.
fn a_run_at(bucket_steps: u32, residual_scale: f32) -> SceneGlyphRun {
    SceneGlyphRun {
        face: 0,
        bucket_steps,
        residual_scale,
        origin: ScenePoint::ORIGIN,
        direction: TextDirection::LeftToRight,
        hinting: Hinting::GridFitted,
        level: 0,
        glyphs: vec![SceneGlyph {
            x: 0,
            y: 0,
            cluster: 0,
            glyph: 3,
            subpixel: 0,
            image: GlyphImage::Blank,
        }],
    }
}

/// One curve, large enough that how finely it is flattened is visible in the vertex count.
fn a_curve() -> Geometry {
    Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(0.0, 0.0)),
            PathCommand::CubicTo {
                first_control: ScenePoint::new(0.0, 120.0),
                second_control: ScenePoint::new(180.0, 120.0),
                end: ScenePoint::new(180.0, 0.0),
            },
            PathCommand::Close,
        ],
        FillRule::NonZero,
    )
}

/// How many vertices that curve becomes at `options`.
fn vertices_at(options: TessellationOptions) -> usize {
    Tessellator::new()
        .fill(&a_curve(), &PlaceholderGeometry::new(), options)
        .expect("a curve tessellates")
        .vertex_count()
}

// -------------------------------------------------------------------------------------------
// A run drawn larger is flattened finer
// -------------------------------------------------------------------------------------------

#[test]
fn a_run_a_painter_will_magnify_is_flattened_finer_and_a_shrunk_one_coarser() {
    // The outline is in the bucket's own pixels and a painter multiplies the whole run by
    // `residual_scale` before drawing it. So a quarter of a device pixel *at the destination* is a
    // quarter divided by the residual *at the source*, and the flattening has to be that much
    // finer — or the magnification magnifies the error with it.
    let unmagnified = TessellationOptions::for_glyph_run(&a_run_at(48, 1.0));
    let magnified = TessellationOptions::for_glyph_run(&a_run_at(48, 4.0));
    let shrunk = TessellationOptions::for_glyph_run(&a_run_at(48, 0.25));

    assert_eq!(unmagnified.tolerance(), TOLERANCE_DEVICE_PIXELS);
    assert!(
        magnified.tolerance() < unmagnified.tolerance(),
        "a run drawn four times larger asks for a tolerance of {}, which is not finer than {}",
        magnified.tolerance(),
        unmagnified.tolerance()
    );
    assert!(
        shrunk.tolerance() > unmagnified.tolerance(),
        "a run drawn a quarter the size asks for {}, which is not coarser than {}",
        shrunk.tolerance(),
        unmagnified.tolerance()
    );
    // Not merely ordered — *proportional*. Four times larger is four times finer, which is the only
    // ratio that keeps the error at the destination constant.
    assert!(
        (magnified.tolerance() * 4.0 - unmagnified.tolerance()).abs() < 1.0e-6,
        "four times the magnification bought {}× the precision, not four",
        unmagnified.tolerance() / magnified.tolerance()
    );

    // And it reaches the triangles, which is what a getter comparison would not have shown.
    let coarse = vertices_at(shrunk);
    let ordinary = vertices_at(unmagnified);
    let fine = vertices_at(magnified);
    assert!(
        coarse < ordinary && ordinary < fine,
        "the same curve became {coarse}, {ordinary} and {fine} vertices at three tolerances that \
         differ by a factor of sixteen; the flattening is not responding to the residual scale"
    );
    assert!(
        fine > coarse * 2,
        "a sixteen-fold finer tolerance produced {fine} vertices against {coarse}"
    );
}

#[test]
fn the_bucket_a_run_names_is_the_bucket_it_is_tessellated_at() {
    // The scale bucket is MJXOFF-161's, already in the record. Reading it rather than inventing a
    // second bucketing scheme is hand-off 3, and this is what holds it.
    for steps in [1_u32, 48, 120, 4096] {
        assert_eq!(
            TessellationOptions::for_glyph_run(&a_run_at(steps, 1.0)).bucket(),
            ScaleBucket::from_steps(steps)
        );
    }
}

#[test]
fn a_residual_scale_a_display_list_should_not_carry_still_yields_a_usable_tolerance() {
    // A display list may have come off a disk, so `residual_scale` may be anything at all — and a
    // tolerance of zero, of infinity or of a `NaN` is not a flattening, it is a hang or a crash.
    for impossible in [0.0_f32, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let options = TessellationOptions::for_glyph_run(&a_run_at(48, impossible));
        assert_eq!(
            options.tolerance(),
            TOLERANCE_DEVICE_PIXELS,
            "a residual scale of {impossible} did not fall back"
        );
    }

    // A residual a document could produce but nothing should draw: the clamp holds, and the mesh is
    // a mesh rather than a hundred million triangles.
    let enormous = TessellationOptions::for_glyph_run(&a_run_at(48, 1.0e9));
    assert_eq!(enormous.tolerance(), MINIMUM_TOLERANCE);
    let mesh = Tessellator::new()
        .fill(&a_curve(), &PlaceholderGeometry::new(), enormous)
        .expect("even the finest tolerance tessellates");
    assert!(
        !mesh.is_empty() && mesh.triangle_count() < 100_000,
        "the finest tolerance produced {} triangles, which is an unbounded flattening rather than \
         a fine one",
        mesh.triangle_count()
    );

    let minute = TessellationOptions::for_glyph_run(&a_run_at(48, 1.0e-9));
    assert_eq!(minute.tolerance(), MAXIMUM_TOLERANCE);
}

// -------------------------------------------------------------------------------------------
// The tolerance a caller chooses, and its bounds
// -------------------------------------------------------------------------------------------

#[test]
fn a_chosen_tolerance_is_honoured_between_its_bounds_and_clamped_outside_them() {
    let bucket = ScaleBucket::from_steps(48);

    // Honoured, unchanged, in the range a caller has any business asking for.
    for wanted in [
        MINIMUM_TOLERANCE,
        0.01_f32,
        TOLERANCE_DEVICE_PIXELS,
        8.0,
        MAXIMUM_TOLERANCE,
    ] {
        assert_eq!(
            TessellationOptions::with_tolerance(bucket, wanted).tolerance(),
            wanted
        );
    }

    // Clamped outside it — in both directions, and to the stated bound rather than to whatever the
    // arithmetic produced.
    assert_eq!(
        TessellationOptions::with_tolerance(bucket, 1.0e-30).tolerance(),
        MINIMUM_TOLERANCE
    );
    assert_eq!(
        TessellationOptions::with_tolerance(bucket, 1.0e30).tolerance(),
        MAXIMUM_TOLERANCE
    );
    // Zero and the non-finite numbers are not "as fine as possible"; they are a caller that has no
    // opinion, and they take the default.
    for nonsense in [0.0_f32, -4.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            TessellationOptions::with_tolerance(bucket, nonsense).tolerance(),
            TOLERANCE_DEVICE_PIXELS,
            "a tolerance of {nonsense}"
        );
    }

    // And the clamp is load-bearing rather than decorative: an unclamped `1e-30` on a curve is an
    // unbounded flattening, and this is the bound that stops it.
    let mesh = Tessellator::new()
        .fill(
            &a_curve(),
            &PlaceholderGeometry::new(),
            TessellationOptions::with_tolerance(bucket, 1.0e-30),
        )
        .expect("an absurd tolerance still tessellates");
    assert!(
        mesh.triangle_count() < 100_000,
        "an unclamped tolerance produced {} triangles",
        mesh.triangle_count()
    );
}

// -------------------------------------------------------------------------------------------
// The page's own bucket
// -------------------------------------------------------------------------------------------

#[test]
fn a_page_at_two_zooms_is_two_buckets_and_the_larger_zoom_is_the_larger_bucket() {
    let unzoomed = page_bucket(DeviceScale::UNZOOMED);
    let doubled = page_bucket(DeviceScale::from_pixels_per_point(
        DeviceScale::UNZOOMED.pixels_per_point() * 2.0,
    ));
    let halved = page_bucket(DeviceScale::from_pixels_per_point(
        DeviceScale::UNZOOMED.pixels_per_point() / 2.0,
    ));

    assert!(
        halved < unzoomed && unzoomed < doubled,
        "three zoom levels bucketed to {halved:?}, {unzoomed:?} and {doubled:?}; the device scale \
         is not reaching the bucket, so a page cached at one zoom would be served at another"
    );
    assert!(!unzoomed.is_empty());

    // The quantiser is `mjx-text`'s, applied to a nominal em — one scheme for the page and the
    // glyphs on it, not two. Asserted as a bound rather than as the formula restated: a twelve-point
    // em at the unzoomed scale is sixteen pixels, so the bucket has to be able to hold it.
    assert!(
        unzoomed.pixels_per_em() >= DeviceScale::UNZOOMED.pixels_per_point() * REFERENCE_EM_POINTS,
        "the bucket for the unzoomed page is {:?}, smaller than the em it is meant to enclose",
        unzoomed
    );

    // A scale a document or a wire could produce, and must not become a panic or an empty bucket
    // that quietly serves every zoom one cache entry.
    for impossible in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let _ = page_bucket(DeviceScale::from_pixels_per_point(impossible));
    }
}
