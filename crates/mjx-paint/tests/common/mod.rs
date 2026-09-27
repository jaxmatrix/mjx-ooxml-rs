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

/// A page drawn under **two** nested clips, with a second shape drawn after the inner one is popped.
///
/// The single-clip case cannot see the stencil's depth arithmetic: with one clip, "test against the
/// current depth" and "test against zero" are the same assertion.
///
/// **And the obvious two-clip scene cannot see a broken `Pop` either** — that was a green mutation.
/// Replacing `PopClip`'s stencil operation with `Push` (incrementing where it should decrement)
/// leaves the region *outside* the inner clip at exactly the value the next draw tests against, so a
/// second shape placed there paints correctly by coincidence. What separates them is a second shape
/// that covers the **inner** region as well: after a correct pop that region is back inside the
/// outer clip and the later shape covers it, and after a broken one it is at a value nothing tests
/// against and the earlier shape shows through.
///
/// So `after` is drawn over the whole outer clip in its own colour, and the case asserts which
/// colour survives *inside the inner clip*.
pub fn two_clips_and_a_shape_drawn_after_the_inner_one_is_popped(
    width: f32,
    height: f32,
    outer: SceneRect,
    inner: SceneRect,
    before: Color,
    after: Color,
) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let whole = builder
        .add_geometry(&box_path(SceneRect::new(0.0, 0.0, width, height)))
        .expect("a shape covering the page");
    let first = builder.add_paint(Paint::Solid(before)).expect("a colour");
    let second = builder.add_paint(Paint::Solid(after)).expect("another");
    let outer_slot = builder
        .add_clip(Clip::rectangle(outer))
        .expect("the outer clip");
    let inner_slot = builder
        .add_clip(Clip::rectangle(inner))
        .expect("the inner clip");

    builder
        .push(Command::PushClip(outer_slot))
        .expect("the outer clip");
    builder
        .push(Command::PushClip(inner_slot))
        .expect("the inner clip");
    builder
        .push(Command::FillPath {
            geometry: whole,
            paint: first,
        })
        .expect("the first fill");
    builder.push(Command::Pop).expect("the inner clip closes");
    builder
        .push(Command::FillPath {
            geometry: whole,
            paint: second,
        })
        .expect("the second fill");
    builder.push(Command::Pop).expect("the outer clip closes");
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

// ---------------------------------------------------------------------------------------------
// Real faces, for the two exporters
// ---------------------------------------------------------------------------------------------

/// The word a document exporter has to be able to put in a file and get back out of it.
///
/// Eight letters, all of them in Latin-1, none of them repeated in a way that would let a broken
/// `/ToUnicode` map look right by coincidence — `pdftotext` finding `Fidelity` when the map is wrong
/// would need eight independent mistakes to agree.
pub const KNOWN_TEXT: &str = "Fidelity";

/// The bundled face this crate's export suites draw with.
///
/// Reached by a path relative to this crate rather than through `mjx-text`'s own test support,
/// because an integration test is its own crate and cannot see another crate's `tests/`. The face is
/// committed, licensed and recorded in `crates/mjx-text/assets/fonts/README.md`.
pub fn liberation_face() -> std::sync::Arc<mjx_text::FontFace> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mjx-text/assets/fonts/LiberationSans-Regular.ttf");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    std::sync::Arc::new(
        mjx_text::FontFace::parse(std::sync::Arc::from(bytes.as_slice()), 0)
            .expect("the committed face parses"),
    )
}

/// A font source holding that face as face zero, which is what [`text_page`] names.
pub fn liberation_library() -> mjx_paint::FaceLibrary {
    let mut library = mjx_paint::FaceLibrary::new();
    library.insert(0, liberation_face());
    library
}

/// A page with one filled rectangle and one run of [`KNOWN_TEXT`] in real glyphs.
///
/// # Why the glyph ids are real and the atlas rectangles are not
///
/// A rasteriser needs the atlas rectangles and does not care what glyph a quad is; an **exporter**
/// needs the glyph ids and cannot use an atlas rectangle at all. So this page carries real ids, from
/// the real face's own `cmap`, placed at the face's own advances — which is what makes
/// `pdftotext` able to read the word back — and stand-in atlas placements, which is what lets the
/// same page render through both rasterisers.
pub fn text_page(width: f32, height: f32) -> DisplayList {
    let face = liberation_face();
    let reader = face.reader().expect("the face reads");
    let size = 24.0f32;
    let units = f32::from(reader.units_per_em().max(1));

    let mut glyphs = Vec::new();
    let mut pen = 0.0f32;
    for (index, character) in KNOWN_TEXT.chars().enumerate() {
        let Some(glyph) = reader.glyph_for_character(character) else {
            continue;
        };
        let advance = reader.advance(glyph).map_or(size * 0.5, |advance| {
            advance.font_units as f32 * size / units
        });
        glyphs.push(SceneGlyph {
            x: pen.round() as i32,
            y: 0,
            cluster: index as u32,
            glyph: glyph.0,
            subpixel: 0,
            image: GlyphImage::Atlas(AtlasPlacement {
                page: ATLAS_PAGE,
                format: BitmapFormat::Coverage,
                x: (index as u16 % 4) * 8,
                y: 0,
                width: 8,
                height: 8,
                offset_from_origin_x: 0,
                offset_from_origin_y: -8,
            }),
        });
        pen += advance;
    }

    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let background = builder
        .add_geometry(&box_path(SceneRect::new(0.0, 0.0, width, height)))
        .expect("a background");
    let paper = builder
        .add_paint(Paint::Solid(rgb(0xff, 0xff, 0xff)))
        .expect("a paper colour");
    builder
        .push(Command::FillPath {
            geometry: background,
            paint: paper,
        })
        .expect("the background");
    let ink = builder
        .add_paint(Paint::Solid(rgb(0x10, 0x10, 0x30)))
        .expect("an ink");
    let run = builder
        .add_glyph_run(&SceneGlyphRun {
            face: 0,
            bucket_steps: mjx_scene::ScaleBucket::enclosing(size).steps(),
            residual_scale: 1.0,
            origin: ScenePoint::new(12.0, 60.0),
            direction: TextDirection::LeftToRight,
            hinting: Hinting::GridFitted,
            level: 0,
            glyphs,
        })
        .expect("a glyph run");
    builder
        .push(Command::DrawGlyphs { run, paint: ink })
        .expect("a run");
    builder.finish().expect("the scene is well formed")
}

/// The same page, with its text filled by a gradient rather than by one colour.
///
/// What `a:textFill` is, and the case R08 reduced to a representative colour and handed on. It is a
/// separate fixture rather than a flag because the two go through different arms of the plan — a
/// solid run draws into its parent and a filled one opens a mask layer — and a suite that could not
/// name them apart could not assert that.
pub fn gradient_text_page(width: f32, height: f32) -> DisplayList {
    let face = liberation_face();
    let reader = face.reader().expect("the face reads");
    let size = 32.0f32;
    let units = f32::from(reader.units_per_em().max(1));
    let mut glyphs = Vec::new();
    let mut pen = 0.0f32;
    for (index, character) in KNOWN_TEXT.chars().enumerate() {
        let Some(glyph) = reader.glyph_for_character(character) else {
            continue;
        };
        let advance = reader.advance(glyph).map_or(size * 0.5, |advance| {
            advance.font_units as f32 * size / units
        });
        glyphs.push(SceneGlyph {
            x: pen.round() as i32,
            y: 0,
            cluster: index as u32,
            glyph: glyph.0,
            subpixel: 0,
            image: GlyphImage::Atlas(AtlasPlacement {
                page: ATLAS_PAGE,
                format: BitmapFormat::Coverage,
                x: 0,
                y: 0,
                width: 24,
                height: 24,
                offset_from_origin_x: 0,
                offset_from_origin_y: -24,
            }),
        });
        pen += advance;
    }

    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let gradient = builder
        .add_gradient(&mjx_scene::Gradient::linear(
            vec![
                GradientStop::new(0.0, rgb(0xd0, 0x20, 0x20)),
                GradientStop::new(1.0, rgb(0x20, 0x20, 0xd0)),
            ],
            0.0,
        ))
        .expect("a gradient");
    let fill = builder
        .add_paint(Paint::Gradient(gradient))
        .expect("a gradient paint");
    let run = builder
        .add_glyph_run(&SceneGlyphRun {
            face: 0,
            bucket_steps: mjx_scene::ScaleBucket::enclosing(size).steps(),
            residual_scale: 1.0,
            origin: ScenePoint::new(10.0, 70.0),
            direction: TextDirection::LeftToRight,
            hinting: Hinting::GridFitted,
            level: 0,
            glyphs,
        })
        .expect("a glyph run");
    builder
        .push(Command::DrawGlyphs { run, paint: fill })
        .expect("a run");
    builder.finish().expect("the scene is well formed")
}

/// A page with one shape under one effect, so an effect arm can be exercised at all.
///
/// R08's own suite constructed only `OuterShadow` and `Blur`; `Glow`, `Reflection`, `SoftEdge` and
/// `InnerShadow` had never executed anywhere when this was written.
pub fn one_shape_under(width: f32, height: f32, kind: EffectKind) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let shape = builder
        .add_geometry(&box_path(SceneRect::new(
            width * 0.3,
            height * 0.3,
            width * 0.7,
            height * 0.6,
        )))
        .expect("a shape");
    let ink = builder
        .add_paint(Paint::Solid(rgb(0x20, 0x80, 0x40)))
        .expect("an ink");
    let effect_colour = builder
        .add_paint(Paint::Solid(Color {
            red: 0xc0,
            green: 0x00,
            blue: 0x00,
            alpha: 0xd0,
        }))
        .expect("an effect colour");
    let effect = builder
        .add_effect(mjx_scene::Effect {
            paint: Some(effect_colour),
            radius: 5.0,
            distance: 6.0,
            direction: std::f32::consts::FRAC_PI_4,
            // **`Effect::new` leaves both fade alphas at zero**, and a reflection that fades from
            // nothing to nothing is invisible. Every effect fixture in this crate before MJXOFF-164
            // used the defaults, so the `Reflection` arm of both painters had never produced a
            // single pixel — and a cross-painter comparison of two blank reflections agrees
            // perfectly. Set here for every kind, because the two fields mean nothing to the other
            // six and everything to this one.
            start_alpha: 0.6,
            end_alpha: 0.0,
            start_position: 0.0,
            end_position: 1.0,
            ..mjx_scene::Effect::new(kind)
        })
        .expect("an effect");
    builder
        .push(Command::PushEffect(effect))
        .expect("an effect group");
    builder
        .push(Command::FillPath {
            geometry: shape,
            paint: ink,
        })
        .expect("a fill");
    builder.push(Command::Pop).expect("the effect closes");
    builder.finish().expect("the scene is well formed")
}

/// A page whose one shape is a handle **no geometry table has been supplied for**, drawn as a
/// **fill**.
///
/// Since MJXOFF-206 a preset shape resolves to the document's own geometry through
/// `mjx-geometry`'s `PresetGeometryProvider`, so this is no longer what an ordinary page is made
/// of — it is the honest answer for a handle nobody registered, a preset ECMA-376 defines no
/// geometry for, or a shape whose own formulas are singular at the adjustments in force. R10's
/// fidelity rule is that a golden image may not be taken against one, and
/// `DrawReport::placeholders` is the number it refuses on — so **every** painter has to count it,
/// not only the one that was written first.
///
/// Its companion is [`one_unresolved_stroked_shape`], and the pair is not a convenience: the count
/// is incremented in **two** places in `plan.rs`, once under `Command::FillPath` and once under
/// `Command::StrokePath`, so a page that is only ever filled leaves half the counter unproved. It
/// did, until MJXOFF-206: replacing the stroke increment with `std::process::abort()` left
/// `mjx-paint`, `mjx-scene` and `mjx-geometry` **green**.
pub fn one_unresolved_shape(width: f32, height: f32) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&Geometry::Unresolved {
            outline: 7,
            bounds: SceneRect::new(width * 0.15, height * 0.15, width * 0.85, height * 0.85),
        })
        .expect("an unresolved shape");
    let paint = builder
        .add_paint(Paint::Solid(rgb(0x00, 0x80, 0x00)))
        .expect("a paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.finish().expect("the scene is well formed")
}

/// The same handle nobody registered, drawn as a **stroke** instead of a fill.
///
/// The other half of [`one_unresolved_shape`], and the reason it exists is arithmetic rather than
/// taste: `plan.rs` increments `DrawReport::placeholders` under `Command::FillPath` **and** under
/// `Command::StrokePath`, and those are two lines. A suite whose every stand-in is filled proves
/// one of them. Outlined shapes are not exotic — a `straightConnector1` has no interior at all and
/// sixty-three of the presets end a contour without an `a:close` — so the stroked stand-in is a case
/// a real deck reaches.
pub fn one_unresolved_stroked_shape(width: f32, height: f32) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let geometry = builder
        .add_geometry(&Geometry::Unresolved {
            outline: 7,
            bounds: SceneRect::new(width * 0.15, height * 0.15, width * 0.85, height * 0.85),
        })
        .expect("an unresolved shape");
    let stroke = builder
        .add_stroke_style(&StrokeStyle::solid(3.0, rgb(0x00, 0x80, 0x00)))
        .expect("a stroke interns")
        .expect("a visible stroke");
    builder
        .push(Command::StrokePath { geometry, stroke })
        .expect("a stroke");
    builder.finish().expect("the scene is well formed")
}

/// A page with one **dashed** stroke on it.
///
/// A rasteriser gets its dashes from the tessellator, which cuts the path; an exporter writes a dash
/// array and needs the lengths. `mjx_scene::dash_lengths` was private until MJXOFF-164, so an
/// exporter had no way to know what `lgDashDot` means and could only have written a solid line — or
/// invented an eleventh interpretation of a preset ECMA-376 names and does not measure.
pub fn dashed_page(width: f32, height: f32) -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let line = builder
        .add_geometry(&Geometry::path(
            vec![
                PathCommand::MoveTo(ScenePoint::new(8.0, height / 2.0)),
                PathCommand::LineTo(ScenePoint::new(width - 8.0, height / 2.0)),
            ],
            FillRule::NonZero,
        ))
        .expect("a line");
    let stroke = builder
        .add_stroke_style(&StrokeStyle {
            dash: mjx_scene::DashPattern::LargeDashDot,
            ..StrokeStyle::solid(3.0, rgb(0x00, 0x00, 0x00))
        })
        .expect("a stroke interns")
        .expect("a visible stroke");
    builder
        .push(Command::StrokePath {
            geometry: line,
            stroke,
        })
        .expect("a stroke");
    builder.finish().expect("the scene is well formed")
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
