//! The same 186 plates, drawn **through the wired-in provider and a painter**, for a person to look
//! at.
//!
//! ```sh
//! cargo run -p mjx-geometry --example painted_gallery -- /tmp/painted
//! ```
//!
//! Writes `/tmp/painted.svg` (through `mjx_paint::SvgPainter`) and `/tmp/painted.png` (through
//! `mjx_paint::SoftwarePainter`, the pure-Rust rasteriser), and prints the frame's `DrawReport`.
//!
//! # How this differs from `plate_gallery`, which it does not replace
//!
//! `plate_gallery` writes its SVG **itself**, straight from `preset_outline`. It is the sheet to
//! compare against PowerPoint, because it captions every plate and draws each shape's box, text
//! rectangle and connection sites beside it.
//!
//! This one draws nothing itself. Every shape here is a `Geometry::Unresolved` — a bare handle and
//! a box — in a `DisplayList`, registered on a `PresetGeometryProvider` out of the shape's own
//! `a:prstGeom`, and resolved by a **painter** walking that list. So the claim it makes is a
//! different one: *this is what the renderer produces*, rather than *this is what the table says*.
//! A table that is right and a provider that is wired to nothing produce the same first sheet and
//! very different second ones.
//!
//! **The plates are in the same order and the same grid as `plate_gallery`'s**, deliberately, so
//! that sheet's captions serve as this one's index and neither has to grow a text renderer.
//!
//! # ⚠ Still not verification, and nothing gates on it
//!
//! MJXOFF-201 §6 is unchanged by the extra pipeline: *rendering all 186 and finding them plausible
//! is not evidence*, and an agent that looks at a picture and reports it correct has asserted
//! nothing. What this program does assert is the one thing a picture cannot show — it **refuses to
//! write a sheet whose `DrawReport::placeholders` is not zero**, because a sheet of framed, crossed
//! rounded rectangles that a reviewer mistook for shapes is worse than no sheet.
//!
//! The checks that are evidence live in `tests/` and run on every commit:
//! `every_preset_is_structurally_sound.rs`, `the_third_route_is_the_parser.rs`,
//! `every_adjustment_moves_its_shape.rs` and `the_provider_is_wired_in.rs`. **The authoritative
//! visual check is Microsoft PowerPoint on Windows, and it is the user's** (MJXOFF-155 §9 #5).

use mjx_dml::geometry::{GeometryGuideList, PresetGeometry};
use mjx_geometry::{seeded_shapes, PresetGeometryProvider, ShapeOutline, Size};
use mjx_ooxml_core::Interner;
use mjx_paint::{
    export::png, NoGlyphs, NoImages, OffscreenSurface, Painter, Resources, SoftwarePainter,
    SvgPainter, Viewport,
};
use mjx_scene::{
    Color, Command, DeviceScale, DisplayList, Geometry, SceneBuilder, SceneRect, StrokeStyle,
};

/// One plate, in device pixels. **The same numbers `plate_gallery` uses**, so the two sheets line
/// up plate for plate and that file's captions index this one.
const PLATE: (f32, f32) = (160.0, 150.0);
/// How much of a plate the shape's own box takes.
const SHAPE_BOX: (f32, f32) = (120.0, 90.0);
/// How many plates to a row.
const COLUMNS: usize = 10;
/// The strip at the top of the sheet `plate_gallery` puts its heading in.
const HEADING: f32 = 50.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stem = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "painted".to_owned());
    if let Some(directory) = std::path::Path::new(&stem).parent() {
        if !directory.as_os_str().is_empty() {
            std::fs::create_dir_all(directory)?;
        }
    }
    let shapes = seeded_shapes();
    let rows = shapes.len().div_ceil(COLUMNS);
    let (width, height) = (
        PLATE.0 * COLUMNS as f32,
        PLATE.1 * rows as f32 + HEADING - 10.0,
    );

    // One EMU per point, and a box whose aspect is not one: a square plate would hide an axis swap.
    let extents = Size::from_emu((SHAPE_BOX.0 as i64) * 12_700, (SHAPE_BOX.1 as i64) * 12_700);

    // The registry, built the way an application builds one: out of each shape's own `a:prstGeom`,
    // through the bridge, on a handle a box model would have minted.
    let mut interner = Interner::new();
    let mut provider = PresetGeometryProvider::new();
    for (index, definition) in shapes.iter().enumerate() {
        let token = definition.preset.to_wire();
        let empty_adjustments = GeometryGuideList::new(&mut interner, Vec::new());
        let mut document =
            PresetGeometry::new(&mut interner, definition.preset, Some(empty_adjustments));
        document.set_preset_token(&mut interner, token);
        let outline = ShapeOutline::from_preset_geometry(&document, &interner, extents)
            .ok_or_else(|| format!("`{token}` did not cross the document bridge"))?;
        provider.register(handle_of(index), outline);
    }

    let list = a_sheet(shapes.len(), width, height)?;

    let mut svg = SvgPainter::new();
    let drawn = draw(&mut svg, &list, &provider, width, height)?;
    println!("{drawn:?}");
    if drawn.placeholders != 0 {
        return Err(format!(
            "{} of the {} draws used stand-in geometry, so this sheet is not what the renderer \
             produces for a real deck and is not worth looking at",
            drawn.placeholders, drawn.draw_calls
        )
        .into());
    }
    // Two draws a shape, plus the one that paints the paper. Checked because a zero placeholder
    // count says nothing at all about a sheet that drew half its shapes — or none of them.
    let expected = shapes.len() * 2 + 1;
    if drawn.draw_calls != expected {
        return Err(format!(
            "{} draw calls where {} presets filled and stroked over a paper rectangle are {} — the \
             sheet is missing shapes, which a zero placeholder count says nothing about",
            drawn.draw_calls,
            shapes.len(),
            expected
        )
        .into());
    }
    let document = svg.document().ok_or("the SVG exporter wrote nothing")?;
    std::fs::write(format!("{stem}.svg"), document)?;

    let mut raster = SoftwarePainter::new();
    let _ = draw(&mut raster, &list, &provider, width, height)?;
    let pixels = raster
        .read_pixels()?
        .ok_or("the software painter kept no pixels")?;
    // Ink, not "it rendered". A sheet of 186 shapes in two colours on white paper has thousands of
    // distinct colours once the edges are antialiased; a sheet that drew nothing has one.
    println!(
        "raster: {} covered pixel(s), {} distinct colour(s)",
        pixels.covered(),
        pixels.distinct_colors()
    );
    if pixels.distinct_colors() < 100 {
        return Err(format!(
            "the raster sheet has {} distinct colour(s), which is not 186 antialiased outlines",
            pixels.distinct_colors()
        )
        .into());
    }
    std::fs::write(
        format!("{stem}.png"),
        png(pixels.width, pixels.height, &straight(&pixels.rgba)),
    )?;

    println!(
        "{} plates written to {stem}.svg and {stem}.png, through the provider and the painter — \
         `plate_gallery`'s captioned sheet is the index, and neither is a check anything gates on",
        shapes.len()
    );
    Ok(())
}

/// The handle the `index`-th plate is registered under. Not `index`, so a resolver that ignored the
/// registry and walked the table in order would draw the wrong sheet rather than the right one.
fn handle_of(index: usize) -> u64 {
    0x00A5_0000_0000_0000 | ((index as u64) * 7 + 3)
}

/// Where the `index`-th plate's box is.
fn box_of(index: usize) -> SceneRect {
    let (column, row) = (index % COLUMNS, index / COLUMNS);
    let left = column as f32 * PLATE.0 + (PLATE.0 - SHAPE_BOX.0) / 2.0;
    let top = row as f32 * PLATE.1 + HEADING;
    SceneRect::new(left, top, left + SHAPE_BOX.0, top + SHAPE_BOX.1)
}

/// A display list of `count` unresolved plates on a `width` × `height` page, each filled and
/// stroked — both, because the two are separate commands and a sheet that only filled would leave
/// every unfilled preset (a connector is one) invisible.
fn a_sheet(count: usize, width: f32, height: f32) -> Result<DisplayList, mjx_scene::SceneError> {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let paper = builder.add_geometry(&Geometry::Rectangle(SceneRect::new(
        0.0, 0.0, width, height,
    )))?;
    let white = builder.add_paint(mjx_scene::Paint::Solid(Color {
        red: 0xff,
        green: 0xff,
        blue: 0xff,
        alpha: 0xff,
    }))?;
    builder.push(Command::FillPath {
        geometry: paper,
        paint: white,
    })?;

    let fill = builder.add_paint(mjx_scene::Paint::Solid(Color {
        red: 0xdb,
        green: 0xe6,
        blue: 0xf5,
        alpha: 0xff,
    }))?;
    let stroke = builder
        .add_stroke_style(&StrokeStyle::solid(
            1.0,
            Color {
                red: 0x2b,
                green: 0x5a,
                blue: 0xa8,
                alpha: 0xff,
            },
        ))?
        .ok_or(mjx_scene::SceneError::UnresolvedOutline { outline: 0 })?;

    for index in 0..count {
        let geometry = builder.add_geometry(&Geometry::Unresolved {
            outline: handle_of(index),
            bounds: box_of(index),
        })?;
        builder.push(Command::FillPath {
            geometry,
            paint: fill,
        })?;
        builder.push(Command::StrokePath { geometry, stroke })?;
    }
    builder.finish()
}

/// One frame through `painter`, with `provider` resolving every handle.
fn draw(
    painter: &mut dyn Painter,
    list: &DisplayList,
    provider: &PresetGeometryProvider,
    width: f32,
    height: f32,
) -> Result<mjx_paint::DrawReport, mjx_paint::PaintError> {
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let mut host = OffscreenSurface::new(width as u32, height as u32, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport)?;
    let mut resources = Resources::new(&mut glyphs, provider, &images);
    let drawn = painter.draw(&frame, list, &mut resources)?;
    painter.end(frame)?;
    Ok(drawn)
}

/// Premultiplied `RGBA` as straight `RGBA`, which is what a PNG holds.
///
/// Every render target in this workspace is premultiplied — `Pixels::rgba` says so — and writing
/// the bytes through unchanged would darken every partly transparent edge pixel in the file.
fn straight(premultiplied: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(premultiplied.len());
    for pixel in premultiplied.as_chunks::<4>().0 {
        let alpha = pixel[3];
        if alpha == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        for channel in &pixel[..3] {
            out.push(
                ((u32::from(*channel) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255)
                    as u8,
            );
        }
        out.push(alpha);
    }
    out
}
