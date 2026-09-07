//! The identity-value sweep: for every parameter this crate takes, *is it ever supplied at a value
//! other than the one that makes it a no-op* — and **does the value ever reach anyone**?
//!
//! # Why both questions, and why the second was added
//!
//! R07 found nine defects in one file by asking the first question, including a `signed_area` that
//! lost its sign and a swapped `ThickThin`/`ThinThick` pair. Both would have shipped. MJXOFF-163
//! names this crate's candidates: `scale_factor`, `PushOpacity(1.0)`, `BlendMode`, the identity
//! transform, and the atlas delta — *which needs two frames to be observable at all*.
//!
//! The audit that reviewed R07 added the second question, and it is the sharper one: R07's sweep and
//! the `provenance` field that was written once and read zero times failed the same way — they asked
//! *"is this ever non-identity?"* and never asked *"does this value ever reach anyone?"*. A parameter
//! can be varied, be read, change an internal number, and reach nobody. So each case below gates on
//! **a relationship plus a downstream consequence**: not a getter answering what was set, but a
//! second observable that has to move with it.
//!
//! # What runs where
//!
//! Most of this is [`plan_frame`], which has no graphics API in it at all and therefore runs on a
//! machine with no GPU. The atlas-delta case and the blend-mode case need a device and skip loudly
//! without one.

mod common;

use mjx_paint::{
    plan_frame, DrawOp, LayerKind, NoGlyphs, NoImages, OffscreenSurface, PaintProgram, Painter,
    Resources, Viewport,
};
use mjx_scene::{
    Color, Command, Effect, EffectKind, Gradient, GradientStop, Paint, PlaceholderGeometry,
    SceneBuilder, SceneRect, SceneTransform, Tessellator,
};
use mjx_text::DeviceScale;

fn plan(list: &mjx_scene::DisplayList) -> mjx_paint::FramePlan {
    let mut tessellator = Tessellator::new();
    plan_frame(list, &PlaceholderGeometry::new(), &mut tessellator).expect("the list lowers")
}

#[test]
fn push_opacity_at_one_costs_no_layer_and_below_one_costs_exactly_one() {
    // The candidate MJXOFF-163 names first, and the most expensive to get wrong: a scene builder
    // emits `PushOpacity` for *every* fragment that carries an opacity, and the overwhelmingly
    // common value is `1.0`. A painter that opened a full-viewport render target for each would
    // cost one target per group on a page that needs none.
    let rect = SceneRect::new(2.0, 2.0, 20.0, 20.0);
    let ink = common::rgb(0x30, 0x60, 0x90);

    // The two non-finite rows are worth their own note, and they are a **finding rather than an
    // expectation**: a `NaN` or an infinite opacity reaches the painter as **zero**, not as itself.
    // `SceneBuilder` sanitises it on the way into the display list, exactly as it sanitises every
    // other number a document can make up, and zero is a real opacity that opens a real (and
    // entirely transparent) layer. So the painter's own non-finite guard is **unreachable through a
    // well-formed list**. It is kept because a hand-built list is a thing R09's painters, a
    // transport boundary and this very suite all produce — but it is dead code as far as
    // `mjx-scene` is concerned, and that is recorded here rather than assumed either way.
    for (factor, expected) in [
        (1.0f32, 0usize),
        (1.5, 0),
        (f32::INFINITY, 1),
        (0.999, 1),
        (0.5, 1),
        (0.0, 1),
        (f32::NAN, 1),
    ] {
        let list = common::one_rectangle_at_opacity(32.0, 32.0, rect, ink, factor);
        let plan = plan(&list);
        assert_eq!(
            plan.layers().len(),
            expected + 1,
            "PushOpacity({factor}) produced {} layer(s) beyond the root; {expected} was expected",
            plan.layers().len() - 1
        );
        // The downstream consequence: the root layer's operations differ. With no layer the fill is
        // *in* the root; with one the root holds a composite and the fill is elsewhere.
        let root = plan.layer(0).expect("a root");
        let composites = root
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::Composite { .. }))
            .count();
        assert_eq!(composites, expected);
        if expected == 1 {
            let child = plan.layer(1).expect("the group");
            let wanted = if factor.is_finite() {
                factor.max(0.0)
            } else {
                0.0
            };
            assert!(
                matches!(child.kind, LayerKind::Opacity(alpha) if (alpha - wanted).abs() < 1e-6),
                "the layer does not carry the factor it was opened for: {:?}",
                child.kind
            );
        }
    }
}

#[test]
fn the_identity_transform_composes_away_and_a_real_one_does_not() {
    // A transform that is only ever the identity is a transform nothing would notice the absence
    // of. The relationship: composing is associative and the identity is neutral; the consequence:
    // the operation's transform is the *product*, so a nested pair multiplies rather than replaces.
    let outer = SceneTransform {
        scale_x: 2.0,
        shear_y: 0.0,
        shear_x: 0.0,
        scale_y: 2.0,
        translate_x: 10.0,
        translate_y: 0.0,
    };
    let inner = SceneTransform {
        scale_x: 1.0,
        shear_y: 0.0,
        shear_x: 0.0,
        scale_y: 1.0,
        translate_x: 5.0,
        translate_y: 3.0,
    };
    assert_eq!(
        mjx_paint::plan::compose(SceneTransform::IDENTITY, inner),
        inner
    );
    assert_eq!(
        mjx_paint::plan::compose(outer, SceneTransform::IDENTITY),
        outer
    );

    // A `PushTransform` is **relative**, so the inner translation is scaled by the outer one.
    let composed = mjx_paint::plan::compose(outer, inner);
    assert_eq!(composed.scale_x, 2.0);
    assert_eq!(
        (composed.translate_x, composed.translate_y),
        (20.0, 6.0),
        "the inner translation must be put through the outer map, or a group inside a group lands \
         in the wrong place"
    );
    // And the point it moves.
    assert_eq!(mjx_paint::plan::apply(composed, 0.0, 0.0), (20.0, 6.0));
    assert_eq!(mjx_paint::plan::apply(composed, 1.0, 1.0), (22.0, 8.0));

    // The consequence in a plan: nested transform groups reach the draw as one matrix.
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 64.0, 64.0);
    let outer_slot = builder.add_transform(outer).expect("outer");
    let inner_slot = builder.add_transform(inner).expect("inner");
    let geometry = builder
        .add_geometry(&common::box_path(SceneRect::new(0.0, 0.0, 4.0, 4.0)))
        .expect("a shape");
    let paint = builder
        .add_paint(Paint::Solid(common::rgb(1, 2, 3)))
        .expect("a paint");
    builder
        .push(Command::PushTransform(outer_slot))
        .expect("push");
    builder
        .push(Command::PushTransform(inner_slot))
        .expect("push");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(Command::Pop).expect("pop");
    builder.push(Command::Pop).expect("pop");
    let list = builder.finish().expect("well formed");

    let plan = plan(&list);
    let root = plan.layer(0).expect("a root");
    let found = root.ops.iter().find_map(|op| match op {
        DrawOp::Mesh { transform, .. } => Some(*transform),
        _ => None,
    });
    assert_eq!(
        found,
        Some(composed),
        "the draw did not receive the product of the two transforms"
    );
}

#[test]
fn a_gradient_mapping_moves_across_the_shape_and_is_not_a_constant() {
    // `GradientMapping` is resolved on the processor and read by three lines of shader that no test
    // can execute. `GradientMapping::at` is those three lines in Rust, and this is what stops the
    // mapping being a constant nobody would notice: `t` has to be zero at one end of the shape, one
    // at the other, and monotonic across it.
    let stops = vec![
        GradientStop::new(0.0, common::rgb(0, 0, 0)),
        GradientStop::new(1.0, common::rgb(0xff, 0xff, 0xff)),
    ];
    let bounds = SceneRect::new(10.0, 20.0, 110.0, 60.0);
    let horizontal = Gradient::linear(stops.clone(), 0.0);
    let mapping = mjx_paint::plan::GradientMapping::resolve(&horizontal, bounds);

    let left = mapping.at(bounds.left, 40.0);
    let middle = mapping.at((bounds.left + bounds.right) / 2.0, 40.0);
    let right = mapping.at(bounds.right, 40.0);
    assert!(left < 0.01, "t at the left edge is {left}");
    assert!((middle - 0.5).abs() < 0.02, "t in the middle is {middle}");
    assert!(right > 0.99, "t at the right edge is {right}");

    // Turned a quarter, the same gradient runs the other way — which is the identity-value question
    // for `Gradient::angle`.
    let vertical = Gradient::linear(stops.clone(), std::f32::consts::FRAC_PI_2);
    let turned = mjx_paint::plan::GradientMapping::resolve(&vertical, bounds);
    assert!(turned.at(60.0, bounds.top) < 0.01);
    assert!(turned.at(60.0, bounds.bottom) > 0.99);
    assert!(
        (turned.at(bounds.left, 40.0) - turned.at(bounds.right, 40.0)).abs() < 0.01,
        "a vertical ramp must not vary along x"
    );

    // And `angle_is_scaled` is not decorative: on a shape that is not square it changes the
    // direction. This one is two and a half times as wide as it is tall.
    let mut scaled = Gradient::linear(stops, std::f32::consts::FRAC_PI_4);
    scaled.angle_is_scaled = true;
    let scaled_mapping = mjx_paint::plan::GradientMapping::resolve(&scaled, bounds);
    let plain = mjx_paint::plan::GradientMapping::resolve(
        &Gradient::linear(
            vec![
                GradientStop::new(0.0, common::rgb(0, 0, 0)),
                GradientStop::new(1.0, common::rgb(0xff, 0xff, 0xff)),
            ],
            std::f32::consts::FRAC_PI_4,
        ),
        bounds,
    );
    assert_ne!(
        scaled_mapping.axis, plain.axis,
        "`a:lin@scaled` measures the angle in the shape's own aspect ratio; on a shape this far \
         from square the two must differ, or the flag is being read and discarded"
    );

    // A radial gradient is a different mapping entirely, not the linear one with a flag set.
    let radial = Gradient {
        kind: mjx_scene::GradientKind::Radial,
        path_shade: mjx_scene::PathShade::Circle,
        ..Gradient::linear(
            vec![
                GradientStop::new(0.0, common::rgb(0, 0, 0)),
                GradientStop::new(1.0, common::rgb(0xff, 0xff, 0xff)),
            ],
            0.0,
        )
    };
    let radial_mapping = mjx_paint::plan::GradientMapping::resolve(&radial, bounds);
    assert!(radial_mapping.radial);
    let centre = (
        (bounds.left + bounds.right) / 2.0,
        (bounds.top + bounds.bottom) / 2.0,
    );
    assert!(radial_mapping.at(centre.0, centre.1) < 0.01);
    assert!(radial_mapping.at(bounds.right, centre.1) > 0.99);
}

#[test]
fn an_effect_dag_reaches_the_plan_with_its_numbers_and_its_chain() {
    // Every effect field is a candidate: a painter that read `kind` and nothing else would draw
    // seven identical blurs. The relationship checked here is that the node's numbers survive the
    // display list unchanged, and that a two-node chain reaches the plan as a chain.
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 64.0, 64.0);
    let colour = builder
        .add_paint(Paint::Solid(Color {
            red: 0x12,
            green: 0x34,
            blue: 0x56,
            alpha: 0x78,
        }))
        .expect("a colour");
    let first = builder
        .add_effect(Effect {
            paint: Some(colour),
            radius: 6.5,
            distance: 4.25,
            direction: 1.25,
            ..Effect::new(EffectKind::OuterShadow)
        })
        .expect("a shadow");
    let second = builder
        .add_effect(Effect {
            input: Some(first),
            radius: 2.5,
            ..Effect::new(EffectKind::Blur)
        })
        .expect("a blur over it");
    let geometry = builder
        .add_geometry(&common::box_path(SceneRect::new(8.0, 8.0, 40.0, 40.0)))
        .expect("a shape");
    let paint = builder
        .add_paint(Paint::Solid(common::rgb(0, 0x80, 0)))
        .expect("a paint");
    builder
        .push(Command::PushEffect(second))
        .expect("an effect");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(Command::Pop).expect("pop");
    let list = builder.finish().expect("well formed");

    let plan = plan(&list);
    let layer = plan.layer(1).expect("the effect layer");
    let LayerKind::Effect(nodes) = &layer.kind else {
        panic!("the effect group is a {:?}", layer.kind);
    };
    assert_eq!(nodes.len(), 2, "the whole DAG must reach the painter");

    let shadow = nodes.first().expect("the first node");
    assert_eq!(shadow.effect.kind, EffectKind::OuterShadow);
    assert!((shadow.effect.radius - 6.5).abs() < 1e-6);
    assert!((shadow.effect.distance - 4.25).abs() < 1e-6);
    assert!((shadow.effect.direction - 1.25).abs() < 1e-6);
    assert_eq!(
        shadow.color,
        Color {
            red: 0x12,
            green: 0x34,
            blue: 0x56,
            alpha: 0x78
        },
        "the shadow's colour must be resolved out of the paint table, not invented"
    );
    assert_eq!(shadow.input, None, "the first node consumes the subtree");

    let blur = nodes.get(1).expect("the second node");
    assert_eq!(blur.effect.kind, EffectKind::Blur);
    assert_eq!(
        blur.input,
        Some(0),
        "the chain must survive: a blur *of the shadow* is not a blur beside it"
    );

    // And behind-ness is a property of the kind, stated once so four painters agree.
    assert!(mjx_paint::plan::draws_behind(EffectKind::OuterShadow));
    assert!(mjx_paint::plan::draws_behind(EffectKind::Glow));
    assert!(mjx_paint::plan::draws_behind(EffectKind::Reflection));
    assert!(!mjx_paint::plan::draws_behind(EffectKind::Blur));
    assert!(!mjx_paint::plan::draws_behind(EffectKind::SoftEdge));
    assert!(!mjx_paint::plan::draws_behind(EffectKind::InnerShadow));
    assert!(!mjx_paint::plan::draws_behind(EffectKind::FillOverlay));
}

#[test]
fn a_clip_open_around_a_group_is_replayed_inside_it() {
    // A clip applied only when the group is composited is right for an opacity and **wrong for every
    // effect**: a blur would drag the clipped-away part of the subtree back across the boundary. The
    // consequence checked here is structural — the child layer's own operations begin with the clip.
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 64.0, 64.0);
    let clip = builder
        .add_clip(mjx_scene::Clip::rectangle(SceneRect::new(
            0.0, 0.0, 32.0, 32.0,
        )))
        .expect("a clip");
    let geometry = builder
        .add_geometry(&common::box_path(SceneRect::new(0.0, 0.0, 64.0, 64.0)))
        .expect("a shape");
    let paint = builder
        .add_paint(Paint::Solid(common::rgb(0, 0, 0)))
        .expect("a paint");
    builder.push(Command::PushClip(clip)).expect("a clip");
    builder.push(Command::PushOpacity(0.5)).expect("a group");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(Command::Pop).expect("pop");
    builder.push(Command::Pop).expect("pop");
    let list = builder.finish().expect("well formed");

    let plan = plan(&list);
    let child = plan.layer(1).expect("the group");
    assert!(
        matches!(child.ops.first(), Some(DrawOp::PushClip { .. })),
        "the clip open around the group was not re-emitted into it: {:?}",
        child.ops.first()
    );
    // And the parent still has its own copy, so the composite is clipped too.
    let root = plan.layer(0).expect("a root");
    assert!(root
        .ops
        .iter()
        .any(|op| matches!(op, DrawOp::PushClip { .. })));
    assert!(root
        .ops
        .iter()
        .any(|op| matches!(op, DrawOp::PopClip { .. })));
}

#[test]
fn a_run_of_glyphs_on_one_page_is_one_draw_and_the_quads_land_where_the_record_says() {
    // The parameter here is `SceneGlyph::x`: a painter that ignored it would stack every glyph at
    // the run's origin, and a page of text would still have pixels on it. The consequence is that
    // the quads are *spread*, and that they are one batch rather than six.
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 128.0, 64.0);
    let ink = builder
        .add_paint(Paint::Solid(common::rgb(0, 0, 0)))
        .expect("an ink");
    let run = builder
        .add_glyph_run(&common::stand_in_run(128.0))
        .expect("a run");
    builder
        .push(Command::DrawGlyphs { run, paint: ink })
        .expect("a run");
    let list = builder.finish().expect("well formed");

    let plan = plan(&list);
    let root = plan.layer(0).expect("a root");
    let batches: Vec<_> = root
        .ops
        .iter()
        .filter_map(|op| match op {
            DrawOp::Glyphs { quads, paint, .. } => Some((quads.clone(), paint.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        batches.len(),
        1,
        "six glyphs on one atlas page must be one draw call, not {}",
        batches.len()
    );
    let (quads, paint) = batches.first().expect("the batch");
    assert_eq!(quads.len(), 6);
    assert!(
        matches!(paint, PaintProgram::Glyphs { page, .. } if *page == common::ATLAS_PAGE),
        "the batch names the wrong atlas page"
    );

    // Spread, not stacked. Nine pixels apart is what the record says.
    for pair in quads.windows(2) {
        let (Some(left), Some(right)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        assert!(
            (right.x - left.x - 9.0).abs() < 1e-3,
            "glyphs must advance by what the run recorded: {} then {}",
            left.x,
            right.x
        );
    }
    // And the texture coordinates advance with them, or every glyph would draw the same letter.
    for pair in quads.windows(2) {
        let (Some(left), Some(right)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        assert!(right.u > left.u, "every quad samples the same texels");
    }
}

#[test]
fn the_atlas_delta_is_uploaded_once_and_the_second_frame_uploads_nothing() {
    // **The candidate that needs two frames to be observable at all.** A painter that re-uploaded
    // the whole atlas every frame would satisfy every single-frame assertion in this crate, and
    // would cost a megabyte a frame on a page of text. One frame cannot tell the difference; two
    // can, and only if the second draws the same words.
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "the_atlas_delta_is_uploaded_once_and_the_second_frame_uploads_nothing",
                &why,
            )
        }
    };
    common::announce(
        "the_atlas_delta_is_uploaded_once_and_the_second_frame_uploads_nothing",
        &painter,
    );

    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 128.0, 64.0);
    let ink = builder
        .add_paint(Paint::Solid(common::rgb(0, 0, 0)))
        .expect("an ink");
    let run = builder
        .add_glyph_run(&common::stand_in_run(128.0))
        .expect("a run");
    builder
        .push(Command::DrawGlyphs { run, paint: ink })
        .expect("a run");
    let list = builder.finish().expect("well formed");

    let mut glyphs = common::ChequeredAtlas::new();
    let images = NoImages;
    let geometry = PlaceholderGeometry::new();
    let mut host = OffscreenSurface::new(128, 64, 1.0);
    let viewport = Viewport::covering(&host);

    let first = {
        let frame = painter.begin(&mut host, viewport).expect("frame one");
        let mut resources = Resources::new(&mut glyphs, &geometry, &images);
        let drawn = painter
            .draw(&frame, &list, &mut resources)
            .expect("draw one");
        painter.end(frame).expect("end one");
        drawn
    };
    let first_pixels = painter
        .read_pixels()
        .expect("readback")
        .expect("offscreen")
        .covered();

    let second = {
        let frame = painter.begin(&mut host, viewport).expect("frame two");
        let mut resources = Resources::new(&mut glyphs, &geometry, &images);
        let drawn = painter
            .draw(&frame, &list, &mut resources)
            .expect("draw two");
        painter.end(frame).expect("end two");
        drawn
    };
    let second_pixels = painter
        .read_pixels()
        .expect("readback")
        .expect("offscreen")
        .covered();

    assert_eq!(
        first.atlas_bytes_uploaded,
        glyphs.byte_len(),
        "the first frame must upload the whole page the atlas reported"
    );
    assert_eq!(first.atlas_pages_created, 1);
    assert_eq!(
        second.atlas_bytes_uploaded, 0,
        "the second frame drew the same words and uploaded {} bytes. Re-uploading the atlas per \
         frame is the obvious wrong implementation, and it is invisible to any assertion a single \
         frame can make.",
        second.atlas_bytes_uploaded
    );
    assert_eq!(second.atlas_pages_created, 0);

    // And the consequence: the second frame still draws the text. An "upload nothing" that also
    // drew nothing would satisfy the assertion above perfectly.
    assert!(first_pixels > 0, "the first frame drew no glyphs");
    assert_eq!(
        first_pixels, second_pixels,
        "the second frame drew a different amount of ink from the first, so the texture it drew \
         from is not the one the first frame uploaded"
    );
}

#[test]
fn every_blend_mode_composites_differently_from_source_over() {
    // `BlendMode` is a five-valued parameter whose identity value is `Over`, and a painter that
    // mapped all five onto `Over` would draw every page in this workspace correctly today, because
    // nothing but an effect's `a:fillOverlay` sets anything else. So each is driven through a fill
    // overlay over a known background and the result has to differ.
    let mut painter = match common::painter() {
        Ok(painter) => painter,
        Err(why) => {
            return common::skip(
                "every_blend_mode_composites_differently_from_source_over",
                &why,
            )
        }
    };
    common::announce(
        "every_blend_mode_composites_differently_from_source_over",
        &painter,
    );

    let mut seen: Vec<(mjx_scene::BlendMode, [u8; 4])> = Vec::new();
    for mode in common::BLEND_MODES {
        let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 32.0, 32.0);
        let overlay_colour = builder
            .add_paint(Paint::Solid(common::rgb(0x40, 0xa0, 0xff)))
            .expect("an overlay colour");
        let effect = builder
            .add_effect(Effect {
                paint: Some(overlay_colour),
                blend: mode,
                ..Effect::new(EffectKind::FillOverlay)
            })
            .expect("a fill overlay");
        let geometry = builder
            .add_geometry(&common::box_path(SceneRect::new(4.0, 4.0, 28.0, 28.0)))
            .expect("a shape");
        let base = builder
            .add_paint(Paint::Solid(common::rgb(0xc0, 0x60, 0x20)))
            .expect("a base colour");
        builder
            .push(Command::PushEffect(effect))
            .expect("an effect");
        builder
            .push(Command::FillPath {
                geometry,
                paint: base,
            })
            .expect("a fill");
        builder.push(Command::Pop).expect("pop");
        let list = builder.finish().expect("well formed");

        let mut host = OffscreenSurface::new(32, 32, 1.0);
        let viewport = Viewport::covering(&host);
        let mut glyphs = NoGlyphs;
        let images = NoImages;
        let geometry_provider = PlaceholderGeometry::new();
        let frame = painter.begin(&mut host, viewport).expect("a frame");
        let mut resources = Resources::new(&mut glyphs, &geometry_provider, &images);
        painter
            .draw(&frame, &list, &mut resources)
            .expect("it draws");
        painter.end(frame).expect("it ends");
        let pixels = painter.read_pixels().expect("readback").expect("offscreen");
        let middle = pixels.pixel(16, 16).expect("the middle");
        seen.push((mode, middle));
    }

    // `Over` is the identity. Every other mode has to produce a different pixel from it, or the
    // parameter is read and discarded.
    let over = seen
        .iter()
        .find(|(mode, _)| *mode == mjx_scene::BlendMode::Over)
        .map(|(_, pixel)| *pixel)
        .expect("Over was rendered");
    for (mode, pixel) in &seen {
        if *mode == mjx_scene::BlendMode::Over {
            continue;
        }
        assert_ne!(
            *pixel, over,
            "{mode:?} composited to exactly what `Over` did, so the blend mode reached the pipeline \
             and changed nothing. All five are fixed-function blend states and none of them is \
             `Over` except `Over`."
        );
    }
    println!("blend modes: {seen:?}");
}
