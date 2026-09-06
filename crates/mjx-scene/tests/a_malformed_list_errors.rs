//! A display list that is wrong is an error, never a panic and never an out-of-bounds read.
//!
//! # Why this suite is not optional
//!
//! A display list is **cacheable to disk** and is meant to cross a transport boundary. That makes
//! its bytes untrusted in exactly the sense the rest of this workspace means it, and the project's
//! standing rule follows: no `unwrap`, no `expect`, no `panic!` and no slice index on any path a
//! caller can reach with bytes it did not write.
//!
//! # The shape of the strongest test here
//!
//! [`every_truncation_of_a_real_list_is_an_error`] takes a scene that uses every command and every
//! table, and decodes **every prefix of it** — all several hundred. A single hand-picked truncation
//! proves one offset is checked; every prefix proves every offset is, including the ones a later
//! change adds. It is the same argument as a fuzz campaign and it is exhaustive rather than random,
//! because the input space here is one dimension long.
//!
//! # Proved by mutation
//!
//! * Replacing `read_u32`'s `slice_at` with an index (`&bytes[offset..offset + 4]`) turns
//!   [`every_truncation_of_a_real_list_is_an_error`] from a pass into a **panic**, at the first
//!   prefix that ends inside a record.
//! * Deleting the `end > available` check in `read_header` turns
//!   [`a_section_that_leaves_the_blob_is_refused`] green-to-red only if the section is also read;
//!   it is asserted directly instead, so the check has a caller of its own.
//! * Removing the `input.index() >= index` check in `validate_effects` makes
//!   [`an_effect_that_consumes_itself_is_a_cycle`] pass a list a painter would loop on for ever.

use mjx_scene::{
    Clip, Color, Command, DisplayList, Effect, EffectKind, Geometry, Paint, ResourceIndex,
    SceneBuilder, SceneError, SceneRect, SceneTransform, SectionKind,
};
use mjx_text::DeviceScale;

const INK: Color = Color {
    red: 0x11,
    green: 0x22,
    blue: 0x33,
    alpha: 0xff,
};

/// A scene that touches every table this version defines, so that a truncation sweep over it
/// reaches every offset the decoder reads.
fn a_scene_of_everything() -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::from_pixels_per_point(2.0), 320.0, 240.0);
    let transform = builder
        .add_transform(SceneTransform {
            scale_x: 0.5,
            ..SceneTransform::IDENTITY
        })
        .expect("a transform");
    let clip = builder
        .add_clip(Clip::rectangle(SceneRect::new(0.0, 0.0, 320.0, 240.0)))
        .expect("a clip");
    let paint = builder.add_paint(Paint::Solid(INK)).expect("a paint");
    let gradient = builder
        .add_gradient(&mjx_scene::Gradient::linear(
            vec![
                mjx_scene::GradientStop::new(0.0, INK),
                mjx_scene::GradientStop::new(1.0, INK),
            ],
            0.0,
        ))
        .expect("a gradient");
    let gradient_paint = builder
        .add_paint(Paint::Gradient(gradient))
        .expect("a gradient paint");
    let stroke = builder
        .add_stroke(mjx_scene::Stroke {
            paint,
            width: 1.0,
            cap: mjx_scene::LineCap::Flat,
            join: mjx_scene::LineJoin::Round,
            dash: mjx_scene::DashPattern::Solid,
            alignment: mjx_scene::StrokeAlignment::Centered,
            compound: mjx_scene::CompoundStroke::Single,
            head: mjx_scene::LineEnd::default(),
            tail: mjx_scene::LineEnd::default(),
        })
        .expect("a stroke");
    let effect = builder
        .add_effect(Effect {
            paint: Some(paint),
            radius: 2.0,
            ..Effect::new(EffectKind::Glow)
        })
        .expect("an effect");
    let rectangle = builder
        .add_geometry(&Geometry::Rectangle(SceneRect::new(0.0, 0.0, 10.0, 10.0)))
        .expect("a rectangle");
    let path = builder
        .add_geometry(&Geometry::path(
            vec![
                mjx_scene::PathCommand::MoveTo(mjx_scene::ScenePoint::new(0.0, 0.0)),
                mjx_scene::PathCommand::CubicTo {
                    first_control: mjx_scene::ScenePoint::new(1.0, 1.0),
                    second_control: mjx_scene::ScenePoint::new(2.0, 2.0),
                    end: mjx_scene::ScenePoint::new(3.0, 3.0),
                },
                mjx_scene::PathCommand::QuadraticTo {
                    control: mjx_scene::ScenePoint::new(4.0, 4.0),
                    end: mjx_scene::ScenePoint::new(5.0, 5.0),
                },
                mjx_scene::PathCommand::Close,
            ],
            mjx_scene::FillRule::NonZero,
        ))
        .expect("a path");
    let run = builder
        .add_glyph_run(&mjx_scene::SceneGlyphRun {
            face: 0,
            bucket_steps: 48,
            residual_scale: 1.0,
            origin: mjx_scene::ScenePoint::new(1.0, 2.0),
            direction: mjx_text::TextDirection::LeftToRight,
            hinting: mjx_text::Hinting::GridFitted,
            level: 0,
            glyphs: vec![mjx_scene::SceneGlyph {
                x: 0,
                y: 0,
                cluster: 0,
                glyph: 3,
                subpixel: 0,
                image: mjx_scene::GlyphImage::Outline(path),
            }],
        })
        .expect("a glyph run");
    let image = builder
        .add_image(mjx_scene::Image::stretched(9))
        .expect("an image");

    for command in [
        Command::PushTransform(transform),
        Command::PushClip(clip),
        Command::PushOpacity(0.75),
        Command::PushEffect(effect),
        Command::FillPath {
            geometry: rectangle,
            paint: gradient_paint,
        },
        Command::StrokePath {
            geometry: path,
            stroke,
        },
        Command::DrawGlyphs { run, paint },
        Command::DrawImage {
            image,
            destination: SceneRect::new(0.0, 0.0, 10.0, 10.0),
        },
        Command::Pop,
        Command::Pop,
        Command::Pop,
        Command::Pop,
    ] {
        builder.push(command).expect("every command is legal");
    }
    builder.finish().expect("the scene is well formed")
}

#[test]
fn the_scene_of_everything_really_does_touch_every_table() {
    let list = a_scene_of_everything();
    for section in SectionKind::ALL {
        assert!(
            !list.section_bytes(section).is_empty(),
            "the `{section}` section is empty, so the truncation sweep below never reads it and \
             proves nothing about its offsets"
        );
    }
}

#[test]
fn every_truncation_of_a_real_list_is_an_error() {
    let whole = a_scene_of_everything().into_bytes();
    assert!(
        whole.len() > 300,
        "the scene is only {} bytes, which cannot cover every table",
        whole.len()
    );
    for length in 0..whole.len() {
        let prefix = whole.get(..length).unwrap_or_default().to_vec();
        let outcome = DisplayList::from_bytes(prefix);
        assert!(
            outcome.is_err(),
            "the first {length} bytes of a {}-byte list decoded as a whole list",
            whole.len()
        );
    }
    // And the untruncated one still decodes, so the sweep above is not passing because nothing
    // decodes at all.
    assert!(DisplayList::from_bytes(whole).is_ok());
}

#[test]
fn a_prefix_of_the_magic_is_not_a_display_list() {
    for length in 0..4 {
        let bytes = b"MJXS".get(..length).unwrap_or_default().to_vec();
        assert!(matches!(
            DisplayList::from_bytes(bytes),
            Err(SceneError::Truncated { .. })
        ));
    }
    assert!(matches!(
        DisplayList::from_bytes(b"NOPE----------------------------".to_vec()),
        Err(SceneError::NotADisplayList {
            found: [b'N', b'O', b'P', b'E']
        })
    ));
}

#[test]
fn a_list_from_another_version_is_refused_before_anything_else_is_read() {
    let mut bytes = a_scene_of_everything().into_bytes();
    if let Some(field) = bytes.get_mut(4..6) {
        field.copy_from_slice(&2_u16.to_le_bytes());
    }
    // Wreck the section table as well, so that a decoder which read the sections *before* the
    // version would report something other than the version.
    if let Some(field) = bytes.get_mut(40..44) {
        field.copy_from_slice(&u32::MAX.to_le_bytes());
    }
    assert!(matches!(
        DisplayList::from_bytes(bytes),
        Err(SceneError::UnsupportedVersion {
            found: 2,
            supported: 1
        })
    ));
}

#[test]
fn a_section_that_leaves_the_blob_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    // The first section row begins at 32: kind, stride, offset, length. Grow its length past the
    // end of everything.
    if let Some(field) = bytes.get_mut(40..44) {
        field.copy_from_slice(&100_000_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("a section past the end is refused");
    assert!(
        matches!(
            error,
            SceneError::MalformedSection {
                section: SectionKind::Commands,
                ..
            }
        ),
        "expected a malformed-section error naming the commands, got {error}"
    );
}

#[test]
fn a_section_that_overlaps_the_one_before_it_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    // The second section row begins at 44 and its offset field at 48. Move that section back on
    // top of the header, which is before everything that precedes it.
    if let Some(field) = bytes.get_mut(48..52) {
        field.copy_from_slice(&0_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("overlapping sections are refused");
    assert!(
        matches!(error, SceneError::MalformedSection { .. }),
        "expected a malformed-section error, got {error}"
    );
}

#[test]
fn a_ragged_table_is_refused() {
    // Build a list with one transform, then shorten the transform section by four bytes and fix
    // the total length, so the table is no longer a whole number of records.
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 8.0, 8.0);
    builder
        .add_transform(SceneTransform {
            scale_x: 2.0,
            ..SceneTransform::IDENTITY
        })
        .expect("a transform");
    let mut bytes = builder.finish().expect("well formed").into_bytes();
    let last = bytes.len();
    bytes.truncate(last - 4);
    if let Some(field) = bytes.get_mut(24..28) {
        field.copy_from_slice(&u32::try_from(last - 4).unwrap_or_default().to_le_bytes());
    }
    // The section row's own length must be shortened too, or the section leaves the blob first.
    if let Some(field) = bytes.get_mut(40..44) {
        field.copy_from_slice(&20_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("a ragged table is refused");
    assert!(
        matches!(
            error,
            SceneError::RaggedSection {
                section: SectionKind::Transforms,
                length: 20,
                stride: 24
            }
        ),
        "expected a ragged-section error naming the transforms, got {error}"
    );
}

#[test]
fn a_command_that_names_a_record_the_list_does_not_hold_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let commands = section_offset(&bytes, SectionKind::Commands);
    // The first command is `PushTransform(0)`; point it at transform 99.
    if let Some(field) = bytes.get_mut(commands + 4..commands + 8) {
        field.copy_from_slice(&99_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("an out-of-range index is refused");
    assert!(
        matches!(
            error,
            SceneError::ResourceOutOfRange {
                section: SectionKind::Transforms,
                index: 99,
                count: 1
            }
        ),
        "expected an out-of-range error naming the transform table, got {error}"
    );
}

#[test]
fn an_unknown_opcode_is_refused_rather_than_skipped() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let commands = section_offset(&bytes, SectionKind::Commands);
    if let Some(byte) = bytes.get_mut(commands) {
        *byte = 200;
    }
    let error = DisplayList::from_bytes(bytes).expect_err("an unknown opcode is refused");
    assert!(
        matches!(
            error,
            SceneError::UnknownOpcode {
                index: 0,
                opcode: 200
            }
        ),
        "expected an unknown-opcode error, got {error}"
    );
}

#[test]
fn a_command_whose_length_does_not_match_its_opcode_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let commands = section_offset(&bytes, SectionKind::Commands);
    // `PushTransform` is eight bytes; claim twelve.
    if let Some(field) = bytes.get_mut(commands + 2..commands + 4) {
        field.copy_from_slice(&12_u16.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("a wrong length is refused");
    assert!(
        matches!(error, SceneError::MalformedCommand { index: 0, .. }),
        "expected a malformed-command error, got {error}"
    );
}

#[test]
fn a_command_of_zero_length_cannot_stall_the_walk() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let commands = section_offset(&bytes, SectionKind::Commands);
    if let Some(field) = bytes.get_mut(commands + 2..commands + 4) {
        field.copy_from_slice(&0_u16.to_le_bytes());
    }
    // A zero-length record would loop for ever in a walker that trusted the length. It is refused
    // before the loop can turn twice.
    let error = DisplayList::from_bytes(bytes).expect_err("a zero-length record is refused");
    assert!(
        matches!(
            error,
            SceneError::MalformedCommand {
                index: 0,
                length: 0,
                ..
            }
        ),
        "expected a malformed-command error, got {error}"
    );
}

#[test]
fn an_unbalanced_stream_is_refused_in_both_directions() {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 8.0, 8.0);
    let transform = builder
        .add_transform(SceneTransform {
            scale_x: 2.0,
            ..SceneTransform::IDENTITY
        })
        .expect("a transform");
    builder
        .push(Command::PushTransform(transform))
        .expect("a push");
    let error = builder.finish().expect_err("something is still pushed");
    assert!(matches!(error, SceneError::UnbalancedStack { .. }));

    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 8.0, 8.0);
    let error = builder
        .push(Command::Pop)
        .expect_err("a pop with nothing pushed");
    assert!(matches!(error, SceneError::UnbalancedStack { .. }));
}

#[test]
fn an_effect_that_consumes_itself_is_a_cycle() {
    // The builder refuses it up front …
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 8.0, 8.0);
    let error = builder
        .add_effect(Effect {
            input: Some(ResourceIndex::new(0)),
            ..Effect::new(EffectKind::Blur)
        })
        .expect_err("effect 0 cannot consume effect 0");
    assert!(matches!(
        error,
        SceneError::EffectCycle { index: 0, input: 0 }
    ));

    // … and the decoder refuses it in bytes that were not written by the builder.
    let mut bytes = a_scene_of_everything().into_bytes();
    let effects = section_offset(&bytes, SectionKind::Effects);
    if let Some(field) = bytes.get_mut(effects + 4..effects + 8) {
        field.copy_from_slice(&0_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("a self-consuming effect is refused");
    assert!(
        matches!(error, SceneError::EffectCycle { index: 0, input: 0 }),
        "expected an effect-cycle error, got {error}"
    );
}

#[test]
fn a_gradient_that_names_stops_it_does_not_have_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let gradients = section_offset(&bytes, SectionKind::Gradients);
    if let Some(field) = bytes.get_mut(gradients + 8..gradients + 12) {
        field.copy_from_slice(&500_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("a stop range past the table is refused");
    assert!(
        matches!(
            error,
            SceneError::ResourceOutOfRange {
                section: SectionKind::GradientStops,
                ..
            }
        ),
        "expected an out-of-range error naming the stop table, got {error}"
    );
}

#[test]
fn a_path_that_names_a_step_this_version_does_not_define_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let path_data = section_offset(&bytes, SectionKind::PathData);
    if let Some(field) = bytes.get_mut(path_data..path_data + 4) {
        field.copy_from_slice(&77_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("an unknown path step is refused");
    assert!(
        matches!(error, SceneError::MalformedPath { .. }),
        "expected a malformed-path error, got {error}"
    );
}

#[test]
fn a_paint_kind_this_version_does_not_define_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let paints = section_offset(&bytes, SectionKind::Paints);
    if let Some(field) = bytes.get_mut(paints..paints + 4) {
        field.copy_from_slice(&42_u32.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("an unknown paint kind is refused");
    assert!(
        matches!(
            error,
            SceneError::UnknownRecordKind {
                section: SectionKind::Paints,
                index: 0,
                ..
            }
        ),
        "expected an unknown-record-kind error naming the paints, got {error}"
    );
}

#[test]
fn a_section_kind_this_version_does_not_define_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    if let Some(field) = bytes.get_mut(32..34) {
        field.copy_from_slice(&999_u16.to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("an unknown section kind is refused");
    assert!(
        matches!(error, SceneError::MalformedHeader { .. }),
        "expected a malformed-header error, got {error}"
    );
}

#[test]
fn a_declared_length_that_is_not_the_real_one_is_refused() {
    let mut bytes = a_scene_of_everything().into_bytes();
    let real = bytes.len();
    if let Some(field) = bytes.get_mut(24..28) {
        field.copy_from_slice(&u32::try_from(real - 4).unwrap_or_default().to_le_bytes());
    }
    let error = DisplayList::from_bytes(bytes).expect_err("a wrong total length is refused");
    assert!(
        matches!(error, SceneError::MalformedHeader { .. }),
        "expected a malformed-header error, got {error}"
    );
}

/// Where a section begins, read out of the section table the way a reader would.
///
/// Written here rather than taken from the crate so that a defect in the crate's own section lookup
/// cannot make these tests target the wrong bytes and pass by accident.
fn section_offset(bytes: &[u8], wanted: SectionKind) -> usize {
    let count = bytes
        .get(20..22)
        .and_then(|field| <[u8; 2]>::try_from(field).ok())
        .map(u16::from_le_bytes)
        .unwrap_or_default();
    for row in 0..usize::from(count) {
        let at = 32 + row * 12;
        let kind = bytes
            .get(at..at + 2)
            .and_then(|field| <[u8; 2]>::try_from(field).ok())
            .map(u16::from_le_bytes)
            .unwrap_or_default();
        if kind == wanted.wire_value() {
            return bytes
                .get(at + 4..at + 8)
                .and_then(|field| <[u8; 4]>::try_from(field).ok())
                .map(u32::from_le_bytes)
                .unwrap_or_default() as usize;
        }
    }
    panic!("the scene of everything has no `{wanted}` section, so this test targets nothing")
}
