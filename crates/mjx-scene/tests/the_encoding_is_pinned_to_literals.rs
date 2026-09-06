//! The bytes are pinned to a hand-written specification, not to the code that produces them.
//!
//! # The trap this file exists to defeat
//!
//! *"A display list round-trips through the encoding"* is satisfied by an encoder and a decoder that
//! are **wrong in the same way**. Move a field four bytes in both and every round-trip still passes;
//! the format has changed, no test has noticed, and the first thing to find out is a painter reading
//! a cache written by yesterday's build.
//!
//! So nothing below round-trips anything. Every assertion compares the produced bytes against a byte
//! string written out here **by hand**, from the layout in `crates/mjx-scene/src/encoding.rs` — a
//! second, independent statement of the same specification. The encoder and the decoder can only
//! agree with each other by agreeing with this file.
//!
//! # Proved by mutation
//!
//! Each of these was proved to fail before it was trusted, by moving one field's offset in the
//! encoder and watching this file — not the round-trip — go red:
//!
//! * swapping the pattern paint's foreground and background words (`add_paint`) →
//!   `the_paint_table_is_a_kind_word_and_three_payload_words` fails, naming both byte strings;
//! * writing the geometry's bounds at offset 12 and its flags at 28 instead of the other way round
//!   (`add_geometry`) → `the_geometry_table_holds_a_kind_two_words_flags_and_a_box` fails on byte
//!   12;
//! * writing `1` rather than `0` into a command record's reserved byte (`push`) →
//!   `every_command_is_pinned_to_its_bytes` **and**
//!   `the_smallest_scene_is_one_hundred_and_twenty_eight_bytes_of_specification` both fail, on the
//!   record's second byte and on byte 69 of the blob.
//!
//! Two of those three leave a list that still decodes, which is the point: they change the format
//! without changing what the code can read back, and only a second statement of the specification
//! catches them. The third — swapping the *kind* word for the colour word — was tried first and is
//! not listed above, because the validator refuses the result before the byte comparison is reached;
//! it is a weaker mutation for this file even though it is a louder failure.
//!
//! # Why the numbers are written as hexadecimal literals
//!
//! `640.0_f32.to_le_bytes()` would be the encoder's own arithmetic restated, which pins nothing.
//! `0x4420_0000` is the IEEE-754 bit pattern of 640.0 written down independently, which is what a
//! specification says.

use mjx_scene::encoding::{
    CLIP_STRIDE, EFFECT_STRIDE, GEOMETRY_STRIDE, GLYPH_RUN_STRIDE, GLYPH_STRIDE,
    GRADIENT_STOP_STRIDE, GRADIENT_STRIDE, IMAGE_STRIDE, PAINT_STRIDE, STROKE_STRIDE,
    TRANSFORM_STRIDE,
};
use mjx_scene::{
    Clip, Color, Command, CompoundStroke, DashPattern, DisplayList, Effect, EffectKind, FillRule,
    Geometry, GlyphImage, Gradient, GradientStop, Image, ImageAdjustments, ImageFillMode, LineCap,
    LineEnd, LineEndShape, LineEndSize, LineJoin, Paint, PathCommand, PathShade, PatternPreset,
    RectangleAnchor, ResourceIndex, SceneBuilder, SceneGlyph, SceneGlyphRun, ScenePoint, SceneRect,
    SceneTransform, SectionKind, Stroke, StrokeAlignment, TileFlip,
};
use mjx_text::{BitmapFormat, DeviceScale, Hinting, TextDirection};

/// A readable failure message: two byte strings, side by side, with the first difference named.
#[track_caller]
fn assert_bytes(what: &str, produced: &[u8], expected: &[u8]) {
    if produced == expected {
        return;
    }
    let first = produced
        .iter()
        .zip(expected.iter())
        .position(|(a, b)| a != b)
        .map_or_else(
            || {
                format!(
                    "the lengths differ: {} against {}",
                    produced.len(),
                    expected.len()
                )
            },
            |at| {
                format!(
                    "byte {at} is {:#04x} and the specification says {:#04x}",
                    produced.get(at).copied().unwrap_or_default(),
                    expected.get(at).copied().unwrap_or_default()
                )
            },
        );
    panic!(
        "{what} does not match the byte layout written down in this file — {first}.\n  \
         produced: {}\n  expected: {}",
        hex(produced),
        hex(expected)
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A scale of exactly two pixels per point, so the header's `f32` is a round bit pattern.
fn two_pixels_per_point() -> DeviceScale {
    DeviceScale::from_pixels_per_point(2.0)
}

/// `#112233ff`, whose packed word is `0xff332211` and is therefore unmistakable in a byte string.
const INK: Color = Color {
    red: 0x11,
    green: 0x22,
    blue: 0x33,
    alpha: 0xff,
};

// -------------------------------------------------------------------------------------------
// The whole blob of the smallest useful scene
// -------------------------------------------------------------------------------------------

#[test]
fn the_smallest_scene_is_one_hundred_and_twenty_eight_bytes_of_specification() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 640.0, 480.0);
    let geometry = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 64.0, 32.0)))
        .expect("the geometry table takes one rectangle");
    let paint = builder
        .add_paint(Paint::Solid(INK))
        .expect("the paint table takes one colour");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill is a legal command");
    let list = builder.finish().expect("the scene is well formed");

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        // --- header, 32 bytes ---
        b'M', b'J', b'X', b'S',   // magic
        0x01, 0x00,               // version 1
        0x20, 0x00,               // header length 32
        0x00, 0x00, 0x00, 0x40,   // device scale 2.0        (0x4000_0000)
        0x00, 0x00, 0x20, 0x44,   // page width 640.0        (0x4420_0000)
        0x00, 0x00, 0xf0, 0x43,   // page height 480.0       (0x43f0_0000)
        0x03, 0x00,               // three sections
        0x00, 0x00,               // no flags
        0x80, 0x00, 0x00, 0x00,   // 128 bytes in total
        0x00, 0x00, 0x00, 0x00,   // reserved
        // --- section table, 12 bytes a row, ascending by kind ---
        0x01, 0x00, 0x00, 0x00,   // commands, variable stride
        0x44, 0x00, 0x00, 0x00,   //   at 68
        0x0c, 0x00, 0x00, 0x00,   //   for 12 bytes
        0x04, 0x00, 0x10, 0x00,   // paints, 16-byte records
        0x50, 0x00, 0x00, 0x00,   //   at 80
        0x10, 0x00, 0x00, 0x00,   //   for 16 bytes
        0x09, 0x00, 0x20, 0x00,   // geometries, 32-byte records
        0x60, 0x00, 0x00, 0x00,   //   at 96
        0x20, 0x00, 0x00, 0x00,   //   for 32 bytes
        // --- commands ---
        0x06, 0x00, 0x0c, 0x00,   // FillPath, 12 bytes
        0x00, 0x00, 0x00, 0x00,   //   geometry 0
        0x00, 0x00, 0x00, 0x00,   //   paint 0
        // --- paints ---
        0x00, 0x00, 0x00, 0x00,   // solid
        0x11, 0x22, 0x33, 0xff,   //   #112233ff
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // --- geometries ---
        0x00, 0x00, 0x00, 0x00,   // rectangle
        0x00, 0x00, 0x00, 0x00,   //   no path offset
        0x00, 0x00, 0x00, 0x00,   //   no path length
        0x00, 0x00, 0x00, 0x00,   //   non-zero fill rule
        0x00, 0x00, 0x00, 0x00,   //   left 0.0
        0x00, 0x00, 0x00, 0x00,   //   top 0.0
        0x00, 0x00, 0x80, 0x42,   //   right 64.0            (0x4280_0000)
        0x00, 0x00, 0x00, 0x42,   //   bottom 32.0           (0x4200_0000)
    ];

    assert_bytes("the smallest scene", list.as_bytes(), &expected);
    assert_eq!(list.byte_len(), 128);
}

// -------------------------------------------------------------------------------------------
// One scene of every command kind, pinned command by command
// -------------------------------------------------------------------------------------------

#[test]
fn every_command_is_pinned_to_its_bytes() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    let transform = builder
        .add_transform(SceneTransform::IDENTITY)
        .expect("a transform");
    let clip = builder
        .add_clip(Clip::rectangle(SceneRect::new(0.0, 0.0, 8.0, 8.0)))
        .expect("a clip");
    let effect = builder
        .add_effect(Effect::new(EffectKind::Blur))
        .expect("an effect");
    let geometry = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 8.0, 8.0)))
        .expect("a geometry");
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    let second_paint = builder
        .add_paint(Paint::Solid(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0xff,
        }))
        .expect("a second paint");
    let stroke = builder
        .add_stroke(Stroke {
            paint,
            width: 1.0,
            cap: LineCap::Flat,
            join: LineJoin::Round,
            dash: DashPattern::Solid,
            alignment: StrokeAlignment::Centered,
            compound: CompoundStroke::Single,
            head: LineEnd::default(),
            tail: LineEnd::default(),
        })
        .expect("a stroke");
    let run = builder
        .add_glyph_run(&SceneGlyphRun {
            face: 0,
            bucket_steps: 0,
            residual_scale: 1.0,
            origin: ScenePoint::ORIGIN,
            direction: TextDirection::LeftToRight,
            hinting: Hinting::GridFitted,
            level: 0,
            glyphs: Vec::new(),
        })
        .expect("a glyph run");
    let image = builder.add_image(Image::stretched(0)).expect("an image");

    for command in [
        Command::PushTransform(transform),
        Command::PushClip(clip),
        Command::PushOpacity(0.5),
        Command::PushEffect(effect),
        Command::FillPath { geometry, paint },
        Command::StrokePath { geometry, stroke },
        Command::DrawGlyphs {
            run,
            paint: second_paint,
        },
        Command::DrawImage {
            image,
            destination: SceneRect::new(1.0, 2.0, 3.0, 4.0),
        },
        Command::Pop,
        Command::Pop,
        Command::Pop,
        Command::Pop,
    ] {
        builder.push(command).expect("every command is legal here");
    }
    let list = builder.finish().expect("the scene is well formed");

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x01, 0x00, 0x08, 0x00,   // PushTransform, 8 bytes
        0x00, 0x00, 0x00, 0x00,   //   transform 0
        0x02, 0x00, 0x08, 0x00,   // PushClip, 8 bytes
        0x00, 0x00, 0x00, 0x00,   //   clip 0
        0x03, 0x00, 0x08, 0x00,   // PushOpacity, 8 bytes
        0x00, 0x00, 0x00, 0x3f,   //   0.5                   (0x3f00_0000)
        0x04, 0x00, 0x08, 0x00,   // PushEffect, 8 bytes
        0x00, 0x00, 0x00, 0x00,   //   effect 0
        0x06, 0x00, 0x0c, 0x00,   // FillPath, 12 bytes
        0x00, 0x00, 0x00, 0x00,   //   geometry 0
        0x00, 0x00, 0x00, 0x00,   //   paint 0
        0x07, 0x00, 0x0c, 0x00,   // StrokePath, 12 bytes
        0x00, 0x00, 0x00, 0x00,   //   geometry 0
        0x00, 0x00, 0x00, 0x00,   //   stroke 0
        0x08, 0x00, 0x0c, 0x00,   // DrawGlyphs, 12 bytes
        0x00, 0x00, 0x00, 0x00,   //   run 0
        0x01, 0x00, 0x00, 0x00,   //   paint 1
        0x09, 0x00, 0x18, 0x00,   // DrawImage, 24 bytes
        0x00, 0x00, 0x00, 0x00,   //   image 0
        0x00, 0x00, 0x80, 0x3f,   //   left 1.0              (0x3f80_0000)
        0x00, 0x00, 0x00, 0x40,   //   top 2.0               (0x4000_0000)
        0x00, 0x00, 0x40, 0x40,   //   right 3.0             (0x4040_0000)
        0x00, 0x00, 0x80, 0x40,   //   bottom 4.0            (0x4080_0000)
        0x05, 0x00, 0x04, 0x00,   // Pop
        0x05, 0x00, 0x04, 0x00,   // Pop
        0x05, 0x00, 0x04, 0x00,   // Pop
        0x05, 0x00, 0x04, 0x00,   // Pop
    ];

    assert_bytes(
        "the command stream",
        list.section_bytes(SectionKind::Commands),
        &expected,
    );
    assert_eq!(list.record_count(SectionKind::Commands), 12);
}

// -------------------------------------------------------------------------------------------
// One record of every table
// -------------------------------------------------------------------------------------------

#[test]
fn the_transform_table_is_six_floats_in_matrix_order() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    builder
        .add_transform(SceneTransform {
            scale_x: 1.0,
            shear_y: 2.0,
            shear_x: 3.0,
            scale_y: 4.0,
            translate_x: 5.0,
            translate_y: 6.0,
        })
        .expect("a transform");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x00, 0x00, 0x80, 0x3f,   // scale_x 1.0
        0x00, 0x00, 0x00, 0x40,   // shear_y 2.0
        0x00, 0x00, 0x40, 0x40,   // shear_x 3.0
        0x00, 0x00, 0x80, 0x40,   // scale_y 4.0
        0x00, 0x00, 0xa0, 0x40,   // translate_x 5.0         (0x40a0_0000)
        0x00, 0x00, 0xc0, 0x40,   // translate_y 6.0         (0x40c0_0000)
    ];
    assert_eq!(expected.len(), TRANSFORM_STRIDE);
    assert_bytes(
        "a transform record",
        list.section_bytes(SectionKind::Transforms),
        &expected,
    );
}

#[test]
fn the_clip_table_says_whether_it_follows_a_path_and_always_carries_a_box() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    builder
        .add_clip(Clip::rectangle(SceneRect::new(1.0, 2.0, 3.0, 4.0)))
        .expect("a rectangular clip");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00,   // a rectangle, not a path
        0xff, 0xff, 0xff, 0xff,   // and therefore no geometry
        0x00, 0x00, 0x80, 0x3f,   // left 1.0
        0x00, 0x00, 0x00, 0x40,   // top 2.0
        0x00, 0x00, 0x40, 0x40,   // right 3.0
        0x00, 0x00, 0x80, 0x40,   // bottom 4.0
    ];
    assert_eq!(expected.len(), CLIP_STRIDE);
    assert_bytes(
        "a rectangular clip record",
        list.section_bytes(SectionKind::Clips),
        &expected,
    );
}

#[test]
fn the_paint_table_is_a_kind_word_and_three_payload_words() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    builder.add_paint(Paint::Solid(INK)).expect("a solid");
    builder
        .add_paint(Paint::Pattern {
            preset: PatternPreset::Weave,
            foreground: INK,
            background: Color {
                red: 0x44,
                green: 0x55,
                blue: 0x66,
                alpha: 0x77,
            },
        })
        .expect("a pattern");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        // solid
        0x00, 0x00, 0x00, 0x00,
        0x11, 0x22, 0x33, 0xff,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // pattern
        0x02, 0x00, 0x00, 0x00,   // kind 2
        0x30, 0x00, 0x00, 0x00,   // `Weave` is the 49th preset, so wire value 48
        0x11, 0x22, 0x33, 0xff,   // foreground
        0x44, 0x55, 0x66, 0x77,   // background
    ];
    assert_eq!(expected.len(), PAINT_STRIDE * 2);
    assert_bytes(
        "the paint table",
        list.section_bytes(SectionKind::Paints),
        &expected,
    );
    // The preset's position is a wire value, so it is stated here as well as read out of the byte
    // string above: a table reordered without a version bump would change the meaning of 49.
    assert_eq!(PatternPreset::Weave.wire_value(), 48);
}

#[test]
fn a_gradient_names_a_run_of_the_stop_table() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    let mut gradient = Gradient::linear(
        vec![
            GradientStop::new(0.0, INK),
            GradientStop::new(
                1.0,
                Color {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 0,
                },
            ),
        ],
        0.0,
    );
    gradient.flip = TileFlip::Both;
    gradient.path_shade = PathShade::Circle;
    builder.add_gradient(&gradient).expect("a gradient");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00,   // linear
        0x00, 0x00, 0x00, 0x00,   // first stop 0
        0x02, 0x00, 0x00, 0x00,   // two stops
        0x07, 0x00, 0x00, 0x00,   // rotate-with-shape (bit 0) + flip `xy` (bits 1-2 = 3)
        0x00, 0x00, 0x00, 0x00,   // angle 0.0
        0x01, 0x00, 0x00, 0x00,   // path shade `circle`
        0x00, 0x00, 0x00, 0x00,   // focus left 0.0
        0x00, 0x00, 0x00, 0x00,   // focus top 0.0
        0x00, 0x00, 0x80, 0x3f,   // focus right 1.0
        0x00, 0x00, 0x80, 0x3f,   // focus bottom 1.0
        0x00, 0x00, 0x00, 0x00,   // tile left 0.0
        0x00, 0x00, 0x00, 0x00,   // tile top 0.0
        0x00, 0x00, 0x80, 0x3f,   // tile right 1.0
        0x00, 0x00, 0x80, 0x3f,   // tile bottom 1.0
    ];
    assert_eq!(expected.len(), GRADIENT_STRIDE);
    assert_bytes(
        "a gradient record",
        list.section_bytes(SectionKind::Gradients),
        &expected,
    );

    #[rustfmt::skip]
    let stops: Vec<u8> = vec![
        0x00, 0x00,               // position 0 ten-thousandths
        0x00, 0x00,               // padding
        0x11, 0x22, 0x33, 0xff,   // #112233ff
        0x10, 0x27,               // position 10 000 ten-thousandths
        0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,   // fully transparent
    ];
    assert_eq!(stops.len(), GRADIENT_STOP_STRIDE * 2);
    assert_bytes(
        "the gradient stop table",
        list.section_bytes(SectionKind::GradientStops),
        &stops,
    );
}

#[test]
fn a_stroke_packs_its_seven_small_enumerations_into_three_words() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    builder
        .add_stroke(Stroke {
            paint,
            width: 2.0,
            cap: LineCap::Square,
            join: LineJoin::Miter { limit: 4.0 },
            dash: DashPattern::LargeDashDot,
            alignment: StrokeAlignment::Inset,
            compound: CompoundStroke::ThickThin,
            head: LineEnd {
                shape: LineEndShape::Stealth,
                width: LineEndSize::Large,
                length: LineEndSize::Small,
            },
            tail: LineEnd {
                shape: LineEndShape::Oval,
                width: LineEndSize::Medium,
                length: LineEndSize::Medium,
            },
        })
        .expect("a stroke");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00,   // paint 0
        0x00, 0x00, 0x00, 0x40,   // width 2.0
        0x00, 0x00, 0x80, 0x40,   // miter limit 4.0
        0x02,                     // cap: square
        0x02,                     // join: miter
        0x05,                     // dash: lgDashDot
        0x01,                     // alignment: inset
        0x02,                     // compound: thickThin
        0x02,                     // head shape: stealth
        0x02,                     // head width: large
        0x00,                     // head length: small
        0x04,                     // tail shape: oval
        0x01,                     // tail width: medium
        0x01,                     // tail length: medium
        0x00,                     // reserved
    ];
    assert_eq!(expected.len(), STROKE_STRIDE);
    assert_bytes(
        "a stroke record",
        list.section_bytes(SectionKind::Strokes),
        &expected,
    );
}

#[test]
fn an_effect_names_its_input_and_carries_every_field_of_every_kind() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    let blur = builder
        .add_effect(Effect {
            radius: 1.0,
            ..Effect::new(EffectKind::Blur)
        })
        .expect("a blur");
    builder
        .add_effect(Effect {
            input: Some(blur),
            paint: Some(paint),
            distance: 2.0,
            anchor: RectangleAnchor::Center,
            rotate_with_shape: true,
            ..Effect::new(EffectKind::OuterShadow)
        })
        .expect("a shadow of the blur");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let shadow: Vec<u8> = vec![
        0x02, 0x00, 0x00, 0x00,   // outer shadow
        0x00, 0x00, 0x00, 0x00,   // input: effect 0, the blur
        0x00, 0x00, 0x00, 0x00,   // paint 0
        0x00, 0x00, 0x00, 0x00,   // radius 0.0
        0x00, 0x00, 0x00, 0x40,   // distance 2.0
        0x00, 0x00, 0x00, 0x00,   // direction 0.0
        0x00, 0x00, 0x80, 0x3f,   // scale_x 1.0
        0x00, 0x00, 0x80, 0x3f,   // scale_y 1.0
        0x00, 0x00, 0x00, 0x00,   // skew_x
        0x00, 0x00, 0x00, 0x00,   // skew_y
        0x00, 0x00, 0x00, 0x00,   // start_alpha
        0x00, 0x00, 0x00, 0x00,   // start_position
        0x00, 0x00, 0x00, 0x00,   // end_alpha
        0x00, 0x00, 0x00, 0x00,   // end_position
        0x00, 0x00, 0x00, 0x00,   // fade_direction
        0x43, 0x00, 0x00, 0x00,   // grow (bit 0) + rotate (bit 1) + anchor 4 at bit 4 = 0x43
    ];
    assert_eq!(shadow.len(), EFFECT_STRIDE);
    let table = list.section_bytes(SectionKind::Effects);
    assert_bytes(
        "the second effect record",
        table.get(EFFECT_STRIDE..).unwrap_or_default(),
        &shadow,
    );
}

#[test]
fn the_geometry_table_holds_a_kind_two_words_flags_and_a_box() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    builder
        .add_geometry(&Geometry::path(
            vec![
                PathCommand::MoveTo(ScenePoint::new(1.0, 2.0)),
                PathCommand::LineTo(ScenePoint::new(3.0, 4.0)),
                PathCommand::Close,
            ],
            FillRule::EvenOdd,
        ))
        .expect("a path");
    builder
        .add_geometry(&Geometry::Unresolved {
            outline: 0x0000_0002_0000_0001,
            bounds: SceneRect::new(0.0, 0.0, 1.0, 1.0),
        })
        .expect("an unresolved outline");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        // the path
        0x01, 0x00, 0x00, 0x00,   // a path
        0x00, 0x00, 0x00, 0x00,   //   at byte 0 of the path data
        0x1c, 0x00, 0x00, 0x00,   //   for 28 bytes: 12 + 12 + 4
        0x01, 0x00, 0x00, 0x00,   //   even-odd (bit 0)
        0x00, 0x00, 0x80, 0x3f,   //   bounds left 1.0
        0x00, 0x00, 0x00, 0x40,   //   bounds top 2.0
        0x00, 0x00, 0x40, 0x40,   //   bounds right 3.0
        0x00, 0x00, 0x80, 0x40,   //   bounds bottom 4.0
        // the unresolved outline
        0x02, 0x00, 0x00, 0x00,   // unresolved
        0x01, 0x00, 0x00, 0x00,   //   handle, low word
        0x02, 0x00, 0x00, 0x00,   //   handle, high word
        0x00, 0x00, 0x00, 0x00,   //   no flags
        0x00, 0x00, 0x00, 0x00,   //   bounds left 0.0
        0x00, 0x00, 0x00, 0x00,   //   bounds top 0.0
        0x00, 0x00, 0x80, 0x3f,   //   bounds right 1.0
        0x00, 0x00, 0x80, 0x3f,   //   bounds bottom 1.0
    ];
    assert_eq!(expected.len(), GEOMETRY_STRIDE * 2);
    assert_bytes(
        "the geometry table",
        list.section_bytes(SectionKind::Geometries),
        &expected,
    );

    #[rustfmt::skip]
    let path: Vec<u8> = vec![
        0x01, 0x00, 0x00, 0x00,   // MoveTo
        0x00, 0x00, 0x80, 0x3f,   //   1.0
        0x00, 0x00, 0x00, 0x40,   //   2.0
        0x02, 0x00, 0x00, 0x00,   // LineTo
        0x00, 0x00, 0x40, 0x40,   //   3.0
        0x00, 0x00, 0x80, 0x40,   //   4.0
        0x05, 0x00, 0x00, 0x00,   // Close
    ];
    assert_bytes(
        "the path data",
        list.section_bytes(SectionKind::PathData),
        &path,
    );
}

#[test]
fn a_glyph_run_names_a_range_of_the_glyph_table() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    builder
        .add_glyph_run(&SceneGlyphRun {
            face: 1,
            bucket_steps: 64,
            residual_scale: 1.0,
            origin: ScenePoint::new(2.0, 3.0),
            direction: TextDirection::RightToLeft,
            hinting: Hinting::Unhinted,
            level: 1,
            glyphs: vec![SceneGlyph {
                x: 4,
                y: -5,
                cluster: 6,
                glyph: 7,
                subpixel: 2,
                image: GlyphImage::Atlas(mjx_scene::AtlasPlacement {
                    page: 1,
                    format: BitmapFormat::Coverage,
                    x: 8,
                    y: 9,
                    width: 10,
                    height: 11,
                    offset_from_origin_x: 1,
                    offset_from_origin_y: -12,
                }),
            }],
        })
        .expect("a glyph run");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let run: Vec<u8> = vec![
        0x01, 0x00, 0x00, 0x00,   // face 1
        0x00, 0x00, 0x00, 0x00,   // first glyph 0
        0x01, 0x00, 0x00, 0x00,   // one glyph
        0x40, 0x00, 0x00, 0x00,   // bucket 64 steps
        0x00, 0x00, 0x80, 0x3f,   // residual scale 1.0
        0x00, 0x00, 0x00, 0x40,   // origin x 2.0
        0x00, 0x00, 0x40, 0x40,   // origin y 3.0
        0x03, 0x01, 0x00, 0x00,   // rtl (bit 0) + unhinted (bit 1) + level 1 at bit 8
    ];
    assert_eq!(run.len(), GLYPH_RUN_STRIDE);
    assert_bytes(
        "a glyph run record",
        list.section_bytes(SectionKind::GlyphRuns),
        &run,
    );

    #[rustfmt::skip]
    let glyph: Vec<u8> = vec![
        0x04, 0x00, 0x00, 0x00,   // x 4
        0xfb, 0xff, 0xff, 0xff,   // y -5
        0x06, 0x00, 0x00, 0x00,   // cluster 6
        0x07, 0x00,               // glyph 7
        0x02,                     // subpixel phase 2
        0x00,                     // image kind: atlas
        0x01, 0x00, 0x00, 0x00,   // atlas page 1
        0x08, 0x00, 0x09, 0x00,   // x 8, y 9
        0x0a, 0x00, 0x0b, 0x00,   // width 10, height 11
        0x01, 0x00, 0xf4, 0xff,   // offsets 1 and -12
        0x00, 0x00, 0x00, 0x00,   // coverage format
    ];
    assert_eq!(glyph.len(), GLYPH_STRIDE);
    assert_bytes(
        "a glyph record",
        list.section_bytes(SectionKind::Glyphs),
        &glyph,
    );
}

#[test]
fn an_image_record_carries_its_handle_its_crop_its_tiling_and_its_adjustments() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    builder
        .add_image(Image {
            handle: 0x0000_0009_0000_0007,
            fill_mode: ImageFillMode::Tile,
            crop: SceneRect::new(0.0, 0.0, 1.0, 1.0),
            tile_offset: ScenePoint::new(1.0, 2.0),
            tile_scale_x: 1.0,
            tile_scale_y: 1.0,
            flip: TileFlip::Horizontal,
            anchor: RectangleAnchor::BottomRight,
            rotate_with_shape: false,
            adjustments: ImageAdjustments {
                grayscale: true,
                ..ImageAdjustments::none()
            },
        })
        .expect("an image");
    let list = finish_with_a_pop(builder);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x07, 0x00, 0x00, 0x00,   // handle, low word
        0x09, 0x00, 0x00, 0x00,   // handle, high word
        0x13, 0x08, 0x00, 0x00,   // tile (bit 0) + flip `x` (bit 1) + grayscale (bit 4)
                                  //   + anchor 8 at bit 8 = 0x0813
        0x00, 0x00, 0x00, 0x00,   // crop left 0.0
        0x00, 0x00, 0x00, 0x00,   // crop top 0.0
        0x00, 0x00, 0x80, 0x3f,   // crop right 1.0
        0x00, 0x00, 0x80, 0x3f,   // crop bottom 1.0
        0x00, 0x00, 0x80, 0x3f,   // tile offset x 1.0
        0x00, 0x00, 0x00, 0x40,   // tile offset y 2.0
        0x00, 0x00, 0x80, 0x3f,   // tile scale x 1.0
        0x00, 0x00, 0x80, 0x3f,   // tile scale y 1.0
        0x10, 0x27,               // alpha 10 000 ten-thousandths
        0x00, 0x00,               // padding
        0x00, 0x00,               // brightness 0
        0x00, 0x00,               // contrast 0
        0x00, 0x00, 0x00, 0x00,   // reserved
        0x00, 0x00, 0x00, 0x00,   // duotone shadow: absent, written transparent
        0x00, 0x00, 0x00, 0x00,   // duotone highlight
        0x00, 0x00, 0x00, 0x00,   // colour change from
        0x00, 0x00, 0x00, 0x00,   // colour change to
    ];
    assert_eq!(expected.len(), IMAGE_STRIDE);
    assert_bytes(
        "an image record",
        list.section_bytes(SectionKind::Images),
        &expected,
    );
}

// -------------------------------------------------------------------------------------------
// The properties the byte strings above rest on
// -------------------------------------------------------------------------------------------

#[test]
fn a_repeated_fill_costs_an_integer_and_not_a_record() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    let geometry = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 1.0, 1.0)))
        .expect("a geometry");
    for _ in 0..400 {
        let paint = builder
            .add_paint(Paint::Solid(INK))
            .expect("the same paint");
        builder
            .push(Command::FillPath { geometry, paint })
            .expect("a fill");
    }
    let list = builder.finish().expect("the scene is well formed");

    assert_eq!(
        list.record_count(SectionKind::Paints),
        1,
        "four hundred fills of one colour must intern to one paint record, or the interner is not \
         doing the one thing it is for"
    );
    assert_eq!(list.record_count(SectionKind::Geometries), 1);
    assert_eq!(list.record_count(SectionKind::Commands), 400);
}

#[test]
fn an_empty_table_costs_no_bytes_at_all() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    let geometry = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 1.0, 1.0)))
        .expect("a geometry");
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    let list = builder.finish().expect("the scene is well formed");

    for empty in [
        SectionKind::Transforms,
        SectionKind::Clips,
        SectionKind::Gradients,
        SectionKind::GradientStops,
        SectionKind::Strokes,
        SectionKind::Effects,
        SectionKind::PathData,
        SectionKind::GlyphRuns,
        SectionKind::Glyphs,
        SectionKind::Images,
    ] {
        assert!(
            list.section_bytes(empty).is_empty(),
            "the `{empty}` section was written for a scene that has none"
        );
        assert_eq!(list.record_count(empty), 0);
    }
}

#[test]
fn the_header_states_the_version_this_build_writes() {
    let list = finish_with_a_pop(SceneBuilder::new(two_pixels_per_point(), 1.0, 1.0));
    assert_eq!(DisplayList::VERSION, 1);
    assert_eq!(&DisplayList::MAGIC, b"MJXS");
    assert_eq!(
        list.as_bytes().get(4..6),
        Some(&[0x01, 0x00][..]),
        "the version is the fifth and sixth bytes, before anything a wrong version could misread"
    );
    assert!((list.device_scale().pixels_per_point() - 2.0).abs() < f32::EPSILON);
    assert_eq!(list.page_size(), (1.0, 1.0));
}

#[test]
fn a_resource_index_is_addressable_without_walking_the_table() {
    let mut builder = SceneBuilder::new(two_pixels_per_point(), 8.0, 8.0);
    for step in 0..8_u8 {
        builder
            .add_paint(Paint::Solid(Color {
                red: step,
                green: 0,
                blue: 0,
                alpha: 0xff,
            }))
            .expect("eight distinct paints");
    }
    let list = finish_with_a_pop(builder);
    for step in 0..8_u8 {
        assert_eq!(
            list.paint(ResourceIndex::new(u32::from(step))),
            Some(Paint::Solid(Color {
                red: step,
                green: 0,
                blue: 0,
                alpha: 0xff,
            })),
            "entry {step} of the paint table is not where its stride says it is"
        );
    }
    assert_eq!(list.paint(ResourceIndex::new(8)), None);
}

/// Finish a builder that pushed no commands.
///
/// A scene with resources and no commands is legal — it is what a builder that has interned a page's
/// palette and not yet walked it looks like — and it is what lets these tests pin one table at a
/// time without a command stream in the way.
fn finish_with_a_pop(builder: SceneBuilder) -> DisplayList {
    builder.finish().expect("the scene is well formed")
}
