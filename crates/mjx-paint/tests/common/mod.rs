//! Shared scaffolding: the display lists the suites draw, and **the loud, named skip**.
//!
//! # Why a skip has to be loud
//!
//! MJXOFF-155 §8 names this project's signature defect: *a gate that is green precisely when the
//! work is skipped*. A painter is the sharpest instance of it in the whole programme, because a
//! machine with no graphics stack produces no adapter, and a suite that treated that as "nothing to
//! do" would report success for a render that never happened.
//!
//! So there is no silent skip anywhere in this crate's tests. [`painter`] answers either a painter
//! or the reason there is none, [`skip`] **prints the reason with the case's own name**, and when
//! `MJX_REQUIRE_GPU=1` is set it fails instead — the same shape `MJX_REQUIRE_SCHEMA` and
//! `MJX_REQUIRE_SOFFICE` already use in this repository, and what continuous integration sets so
//! that an absence there can never pass.

#![allow(
    dead_code,
    unreachable_pub,
    reason = "each suite uses a different part of this scaffolding"
)]

use mjx_paint::{PaintError, WgpuPainter};
use mjx_scene::{
    AtlasPlacement, BitmapFormat, BlendMode, Clip, Color, Command, DisplayList, EffectKind,
    FillRule, Geometry, GlyphImage, GradientStop, Hinting, Image, Paint, PathCommand, SceneBuilder,
    SceneGlyph, SceneGlyphRun, ScenePoint, SceneRect, SceneTransform, StrokeStyle, TextDirection,
};
use mjx_text::DeviceScale;

/// The environment variable that turns a missing GPU into a failure.
pub const REQUIRE: &str = "MJX_REQUIRE_GPU";

/// A painter, or the reason there is not one.
///
/// # Errors
///
/// The message a skip would print.
pub fn painter() -> Result<WgpuPainter, String> {
    match WgpuPainter::offscreen() {
        Ok(painter) => Ok(painter),
        Err(PaintError::NoAdapter { looked_for, detail }) => Err(format!(
            "no graphics adapter (looked for {looked_for}): {detail}"
        )),
        Err(PaintError::Device(detail)) => Err(format!(
            "an adapter exists but would not open a device: {detail}"
        )),
        Err(other) => Err(format!("the painter could not be built: {other}")),
    }
}

/// Announce that `case` did not run, and why — or fail, when this run is not allowed to skip.
pub fn skip(case: &str, why: &str) {
    let message = format!(
        "SKIPPED {case}: {why}. This case needs a graphics device; set {REQUIRE}=1 to make its \
         absence a failure instead."
    );
    assert!(
        std::env::var(REQUIRE).is_err(),
        "{REQUIRE} is set, so a missing graphics device is a failure and not a skip. {message}"
    );
    println!("{message}");
}

/// Announce which painter and which backend a case actually ran on.
///
/// Printed by every rendering case, because *"it rendered"* is true of a software fallback, of the
/// wrong backend and of a frame that drew nothing, and the only way to tell is to say which.
pub fn announce(case: &str, painter: &WgpuPainter) {
    use mjx_paint::Painter;
    println!(
        "{case}: painter `{}` on {}",
        painter.name(),
        painter.backend()
    );
}

/// An opaque colour.
pub fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color {
        red,
        green,
        blue,
        alpha: 0xff,
    }
}

/// A rectangle, as a path rather than a `Geometry::Rectangle`, so the tessellator does real work.
pub fn box_path(rect: SceneRect) -> Geometry {
    Geometry::path(
        vec![
            PathCommand::MoveTo(ScenePoint::new(rect.left, rect.top)),
            PathCommand::LineTo(ScenePoint::new(rect.right, rect.top)),
            PathCommand::LineTo(ScenePoint::new(rect.right, rect.bottom)),
            PathCommand::LineTo(ScenePoint::new(rect.left, rect.bottom)),
            PathCommand::Close,
        ],
        FillRule::NonZero,
    )
}

/// A page with one filled rectangle in `ink`, at `rect`.
pub fn one_rectangle(width: f32, height: f32, rect: SceneRect, ink: Color) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&box_path(rect))
        .expect("one rectangle");
    let paint = builder.add_paint(Paint::Solid(ink)).expect("one paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.finish().expect("the scene is well formed")
}

/// A page whose one rectangle is wrapped in `PushOpacity(factor)`.
pub fn one_rectangle_at_opacity(
    width: f32,
    height: f32,
    rect: SceneRect,
    ink: Color,
    factor: f32,
) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&box_path(rect))
        .expect("one rectangle");
    let paint = builder.add_paint(Paint::Solid(ink)).expect("one paint");
    builder
        .push(Command::PushOpacity(factor))
        .expect("an opacity group");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(Command::Pop).expect("the group closes");
    builder.finish().expect("the scene is well formed")
}

/// A page whose one rectangle is drawn under `transform`.
pub fn one_rectangle_transformed(
    width: f32,
    height: f32,
    rect: SceneRect,
    ink: Color,
    transform: SceneTransform,
) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&box_path(rect))
        .expect("one rectangle");
    let paint = builder.add_paint(Paint::Solid(ink)).expect("one paint");
    let slot = builder.add_transform(transform).expect("one transform");
    builder
        .push(Command::PushTransform(slot))
        .expect("a transform group");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(Command::Pop).expect("the group closes");
    builder.finish().expect("the scene is well formed")
}

/// A page whose one rectangle is clipped to `clip`.
pub fn one_rectangle_clipped(
    width: f32,
    height: f32,
    rect: SceneRect,
    ink: Color,
    clip: SceneRect,
) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&box_path(rect))
        .expect("one rectangle");
    let paint = builder.add_paint(Paint::Solid(ink)).expect("one paint");
    let region = builder.add_clip(Clip::rectangle(clip)).expect("one clip");
    builder.push(Command::PushClip(region)).expect("a clip");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.push(Command::Pop).expect("the clip closes");
    builder.finish().expect("the scene is well formed")
}

/// A page that uses **every one of the nine commands**.
///
/// The list MJXOFF-163's *"a display list containing every command kind renders to an offscreen
/// target"* clause is about, and the one the coverage gate counts opcodes in — so that "every kind"
/// is a fact the suite checks rather than a claim its author made.
pub fn every_command(width: f32, height: f32) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);

    let background = builder
        .add_geometry(&box_path(SceneRect::new(0.0, 0.0, width, height)))
        .expect("a background");
    let paper = builder
        .add_paint(Paint::Solid(rgb(0xf5, 0xf5, 0xf0)))
        .expect("a paper colour");
    builder
        .push(Command::FillPath {
            geometry: background,
            paint: paper,
        })
        .expect("the background");

    // A transform group, and a clip inside it.
    let turned = builder
        .add_transform(SceneTransform {
            scale_x: 1.0,
            shear_y: 0.15,
            shear_x: 0.0,
            scale_y: 1.0,
            translate_x: 4.0,
            translate_y: 2.0,
        })
        .expect("a transform");
    builder
        .push(Command::PushTransform(turned))
        .expect("a transform group");

    let region = builder
        .add_clip(Clip::rectangle(SceneRect::new(
            8.0,
            8.0,
            width - 8.0,
            height - 8.0,
        )))
        .expect("a clip");
    builder.push(Command::PushClip(region)).expect("a clip");

    // An opacity group with a gradient-filled shape in it.
    builder
        .push(Command::PushOpacity(0.6))
        .expect("an opacity group");
    let gradient = builder
        .add_gradient(&mjx_scene::Gradient::linear(
            vec![
                GradientStop::new(0.0, rgb(0x20, 0x60, 0xc0)),
                GradientStop::new(1.0, rgb(0xc0, 0x30, 0x20)),
            ],
            0.0,
        ))
        .expect("a gradient");
    let gradient_paint = builder
        .add_paint(Paint::Gradient(gradient))
        .expect("a gradient paint");
    let shape = builder
        .add_geometry(&box_path(SceneRect::new(
            16.0,
            16.0,
            width * 0.6,
            height * 0.5,
        )))
        .expect("a shape");
    builder
        .push(Command::FillPath {
            geometry: shape,
            paint: gradient_paint,
        })
        .expect("a gradient fill");
    builder.push(Command::Pop).expect("the opacity closes");

    // An effect group with a hatched shape under a drop shadow.
    let shadow_colour = builder
        .add_paint(Paint::Solid(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0xa0,
        }))
        .expect("a shadow colour");
    let effect = builder
        .add_effect(mjx_scene::Effect {
            paint: Some(shadow_colour),
            radius: 4.0,
            distance: 3.0,
            // A shadow to the lower right, which is where every document's default one falls.
            direction: std::f32::consts::FRAC_PI_4,
            ..mjx_scene::Effect::new(EffectKind::OuterShadow)
        })
        .expect("an effect");
    builder
        .push(Command::PushEffect(effect))
        .expect("an effect group");
    let hatch = builder
        .add_paint(Paint::Pattern {
            preset: mjx_scene::PatternPreset::DiagonalCross,
            foreground: rgb(0x10, 0x40, 0x10),
            background: rgb(0xd0, 0xe8, 0xd0),
        })
        .expect("a hatch");
    let hatched = builder
        .add_geometry(&box_path(SceneRect::new(
            width * 0.35,
            height * 0.4,
            width * 0.8,
            height * 0.75,
        )))
        .expect("a hatched shape");
    builder
        .push(Command::FillPath {
            geometry: hatched,
            paint: hatch,
        })
        .expect("a hatched fill");
    let stroke = builder
        .add_stroke_style(&StrokeStyle::solid(3.0, rgb(0x00, 0x00, 0x00)))
        .expect("a stroke interns")
        .expect("a visible stroke");
    builder
        .push(Command::StrokePath {
            geometry: hatched,
            stroke,
        })
        .expect("a stroke");
    builder.push(Command::Pop).expect("the effect closes");

    // A picture.
    let picture = builder
        .add_image(Image::stretched(PICTURE_HANDLE))
        .expect("a picture");
    builder
        .push(Command::DrawImage {
            image: picture,
            destination: SceneRect::new(width * 0.05, height * 0.7, width * 0.3, height * 0.95),
        })
        .expect("a picture");

    // A run of glyphs out of a stand-in atlas page.
    let ink = builder
        .add_paint(Paint::Solid(rgb(0x00, 0x00, 0x60)))
        .expect("an ink");
    let run = builder
        .add_glyph_run(&stand_in_run(width))
        .expect("a glyph run");
    builder
        .push(Command::DrawGlyphs { run, paint: ink })
        .expect("a run");

    builder.push(Command::Pop).expect("the clip closes");
    builder.push(Command::Pop).expect("the transform closes");
    builder.finish().expect("the scene is well formed")
}

/// The handle [`every_command`] draws its picture with.
pub const PICTURE_HANDLE: u64 = 0x00c0_ffee;

/// The atlas page [`stand_in_run`] draws from.
pub const ATLAS_PAGE: u32 = 0;

/// How wide and tall the stand-in atlas page is.
pub const ATLAS_SIDE: u16 = 64;

/// A run of six square glyphs out of a stand-in atlas page.
///
/// Squares rather than letters because this crate must not depend on a font: the seam gate refuses
/// `mjx-text` in every file of `src/` but `glyph_atlas.rs`, and a suite that shaped real text would
/// be testing the font engine rather than the painter. What is being checked here is that a run
/// becomes **one** batched draw call, that its quads land where the record says, and that the
/// atlas's delta reaches the texture.
pub fn stand_in_run(width: f32) -> SceneGlyphRun {
    let mut glyphs = Vec::new();
    for index in 0..6u16 {
        glyphs.push(SceneGlyph {
            x: i32::from(index) * 9,
            y: 0,
            cluster: u32::from(index),
            glyph: index,
            subpixel: 0,
            image: GlyphImage::Atlas(AtlasPlacement {
                page: ATLAS_PAGE,
                format: BitmapFormat::Coverage,
                x: index * 8,
                y: 0,
                width: 8,
                height: 8,
                offset_from_origin_x: 0,
                offset_from_origin_y: -8,
            }),
        });
    }
    SceneGlyphRun {
        face: 0,
        bucket_steps: 8,
        residual_scale: 1.0,
        origin: ScenePoint::new(width * 0.08, 26.0),
        direction: TextDirection::LeftToRight,
        hinting: Hinting::GridFitted,
        level: 0,
        glyphs,
    }
}

/// A picture source with one solid orange picture behind [`PICTURE_HANDLE`].
pub struct OnePicture {
    pixels: Vec<u8>,
}

impl Default for OnePicture {
    fn default() -> Self {
        Self::new()
    }
}

impl OnePicture {
    /// A 4x4 orange picture.
    #[must_use]
    pub fn new() -> Self {
        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for index in 0..16 {
            // Not flat: a picture drawn as one colour and a picture drawn correctly both have
            // pixels, and only a picture with a pattern in it can tell them apart.
            let bright = if index % 3 == 0 { 0xff } else { 0xa0 };
            pixels.extend_from_slice(&[bright, 0x80, 0x20, 0xff]);
        }
        Self { pixels }
    }
}

impl mjx_paint::ImageSource for OnePicture {
    fn pixels(&self, handle: u64) -> Option<mjx_paint::ImagePixels<'_>> {
        if handle != PICTURE_HANDLE {
            return None;
        }
        Some(mjx_paint::ImagePixels {
            width: 4,
            height: 4,
            rgba: &self.pixels,
        })
    }
}

/// An atlas source that reports one page and a chequered coverage image the first time it is asked,
/// and nothing after that.
///
/// **Nothing after that is the point.** A delta that reported everything again on every frame would
/// satisfy any assertion a single frame could make; the second frame is what separates an
/// incremental upload from a wholesale one.
pub struct ChequeredAtlas {
    delivered: bool,
    pixels: Vec<u8>,
}

impl Default for ChequeredAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl ChequeredAtlas {
    /// An atlas with one page of chequered coverage.
    #[must_use]
    pub fn new() -> Self {
        let side = usize::from(ATLAS_SIDE);
        let mut pixels = vec![0u8; side * side];
        for y in 0..side {
            for x in 0..side {
                if let Some(slot) = pixels.get_mut(y * side + x) {
                    *slot = if (x / 2 + y / 2) % 2 == 0 { 0xff } else { 0x40 };
                }
            }
        }
        Self {
            delivered: false,
            pixels,
        }
    }

    /// How many bytes its one page carries.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.pixels.len()
    }
}

impl mjx_paint::AtlasSource for ChequeredAtlas {
    fn take_changes(
        &mut self,
        visitor: &mut dyn mjx_paint::AtlasVisitor,
    ) -> Result<(), PaintError> {
        if self.delivered {
            return Ok(());
        }
        self.delivered = true;
        visitor.page_created(mjx_paint::AtlasPage {
            page: ATLAS_PAGE,
            format: BitmapFormat::Coverage,
            size: ATLAS_SIDE,
        })?;
        visitor.write(mjx_paint::AtlasWrite {
            page: ATLAS_PAGE,
            format: BitmapFormat::Coverage,
            x: 0,
            y: 0,
            width: ATLAS_SIDE,
            height: ATLAS_SIDE,
            pixels: &self.pixels,
        })?;
        Ok(())
    }
}

/// Which blend modes a page could ask for, so a sweep names them rather than assuming.
pub const BLEND_MODES: [BlendMode; 5] = [
    BlendMode::Over,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Darken,
    BlendMode::Lighten,
];

/// Every effect kind, for the same reason.
pub const EFFECT_KINDS: [EffectKind; 7] = EffectKind::ALL;
