//! Our own side of a plate comparison: the same page, built as a display list and exported.
//!
//! # Why this is not "render the .pptx"
//!
//! Nothing in this workspace renders a `.pptx` yet — that is Phase R's layout work, from R14 onward.
//! What exists is the seam beneath it: a [`DisplayList`] of geometry handles, a
//! [`GeometryProvider`](mjx_scene::GeometryProvider) that resolves them, and four painters.
//!
//! So our side of the comparison is built from **the same plate table the deck was authored from**,
//! through the same `mjx-geometry` provider a renderer would use, on the same grid. That is a
//! narrower claim than *"our renderer draws this deck"* and it is the honest one: it compares the
//! **geometry** PowerPoint drew against the geometry our provider resolves, which is exactly what
//! MJXOFF-201 is about, and it does not pretend to compare a layout engine that does not exist.
//!
//! The claim is written into the names: [`our_page`] builds a page from the plate table, and every
//! test that consumes it says in its own name what it proves.
//!
//! # The count, beside the zero
//!
//! [`PageRender::report`] carries `DrawReport`. A caller asserts `placeholders == 0` — but a zero
//! is also true of a page that drew nothing at all, so `draw_calls` is asserted beside it. Both
//! halves are needed and MJXOFF-206 is why: half of that counter had never executed anywhere,
//! because every stand-in scene in the workspace was a fill and the *stroke* branch was unreachable
//! from any test.

use mjx_dml::geometry::{GeometryGuideList, PresetGeometry};
use mjx_geometry::{PresetGeometryProvider, ShapeOutline};
use mjx_ooxml_core::Interner;
use mjx_paint::{
    DrawReport, NoGlyphs, NoImages, OffscreenSurface, PaintError, Painter, PdfPainter, Pixels,
    Resources, SoftwarePainter, Viewport,
};
use mjx_scene::{
    Color, Command, DeviceScale, DisplayList, Geometry, SceneBuilder, SceneError, SceneRect,
    StrokeStyle,
};

use crate::deck::{PLATE_FILL, PLATE_STROKE, PLATE_STROKE_POINTS};
use crate::layout::{PLATES_PER_SLIDE, SLIDE};
use crate::plates::{plate_extents, Plate};

/// What one page of our side came out as.
#[derive(Debug)]
pub struct PageRender {
    /// The page's PDF, single-page, `/MediaBox [0 0 960 540]`.
    pub pdf: Vec<u8>,
    /// What the draw did — `placeholders`, `draw_calls`, the lot.
    pub report: DrawReport,
    /// How many plates on this page have an outline at all.
    pub drawn_plates: usize,
}

/// The provider for one page, registered the way an application registers one: out of each shape's
/// own `a:prstGeom`, through the document bridge, on a handle a box model would have minted.
///
/// A plate with nothing to draw — `upArrow`, or a shape at a singular point — is **not registered**,
/// and its handle is never referenced by the display list either. The alternative is a stand-in,
/// and a stand-in would put a framed crossed rectangle where PowerPoint drew a shape and count it as
/// a draw. Naming the plate excluded is the honest answer; drawing something instead is the
/// dishonest one.
///
/// # Errors
///
/// [`SceneError::UnresolvedOutline`] naming the plate whose `a:prstGeom` did not cross the bridge,
/// which would mean the generated table and `mjx-dml`'s reader disagree about a shape.
pub fn provider_for(plates: &[Plate]) -> Result<PresetGeometryProvider, SceneError> {
    let mut interner = Interner::new();
    let mut provider = PresetGeometryProvider::new();
    for plate in plates {
        if !plate.kind.draws() {
            continue;
        }
        let empty = GeometryGuideList::new(&mut interner, Vec::new());
        let mut document = PresetGeometry::new(&mut interner, plate.preset, Some(empty));
        document.set_preset_token(&mut interner, plate.token);
        for (name, value) in &plate.adjustments {
            document.set_adjustment(&mut interner, name, *value);
        }
        let outline = ShapeOutline::from_preset_geometry(&document, &interner, plate_extents())
            .ok_or(SceneError::UnresolvedOutline {
                outline: handle_of(plate.index),
            })?;
        provider.register(handle_of(plate.index), outline);
    }
    Ok(provider)
}

/// The handle the plate at `index` is registered under.
///
/// Not `index`. A resolver that ignored the registry and walked its own table in order would then
/// draw a plausible sheet in the wrong order, and nothing would notice; with a scattered handle it
/// draws nothing and the placeholder count says so.
#[must_use]
pub const fn handle_of(index: usize) -> u64 {
    0x00A5_0000_0000_0000 | ((index as u64) * 7 + 3)
}

/// The display list for one page of `plates` — the paper, then every plate that draws, filled and
/// stroked.
///
/// # Errors
///
/// [`SceneError`] from the builder, which for a fixed page of constants means the builder's own
/// limits rather than anything a caller passed.
pub fn our_page(plates: &[Plate], page: usize) -> Result<(DisplayList, usize), SceneError> {
    #[allow(
        clippy::cast_precision_loss,
        reason = "the page is 960 x 540 points, both exact in f32"
    )]
    let (width, height) = (SLIDE.0 as f32, SLIDE.1 as f32);
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);

    let paper = builder.add_geometry(&Geometry::Rectangle(SceneRect::new(
        0.0, 0.0, width, height,
    )))?;
    let white = builder.add_paint(mjx_scene::Paint::Solid(rgb("FFFFFF")))?;
    builder.push(Command::FillPath {
        geometry: paper,
        paint: white,
    })?;

    let fill = builder.add_paint(mjx_scene::Paint::Solid(rgb(PLATE_FILL)))?;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a stroke width stated in this crate as 1.0"
    )]
    let stroke = builder
        .add_stroke_style(&StrokeStyle::solid(
            PLATE_STROKE_POINTS as f32,
            rgb(PLATE_STROKE),
        ))?
        .ok_or(SceneError::UnresolvedOutline { outline: 0 })?;

    let mut drawn = 0usize;
    for plate in plates
        .iter()
        .skip(page * PLATES_PER_SLIDE)
        .take(PLATES_PER_SLIDE)
    {
        if !plate.kind.draws() {
            continue;
        }
        let geometry = builder.add_geometry(&Geometry::Unresolved {
            outline: handle_of(plate.index),
            bounds: plate.geometry().shape_box().to_scene_rect(),
        })?;
        builder.push(Command::FillPath {
            geometry,
            paint: fill,
        })?;
        builder.push(Command::StrokePath { geometry, stroke })?;
        drawn += 1;
    }
    Ok((builder.finish()?, drawn))
}

/// One page of our side, exported as a PDF a reader this workspace did not write can open.
///
/// # Errors
///
/// [`PaintError`] from the exporter, and [`PaintError::NoPixels`] if it finished with no document.
pub fn our_page_as_pdf(plates: &[Plate], page: usize) -> Result<PageRender, PaintError> {
    let (list, drawn_plates) = our_page(plates, page).map_err(PaintError::from)?;
    let provider = provider_for(plates).map_err(PaintError::from)?;
    let mut painter = PdfPainter::new();
    let report = draw(&mut painter, &list, &provider)?;
    let pdf = painter
        .document()
        .ok_or(PaintError::NoPixels { name: "pdf" })?
        .to_vec();
    Ok(PageRender {
        pdf,
        report,
        drawn_plates,
    })
}

/// The same page through the pure-Rust rasteriser, for a caller that wants pixels without a PDF
/// reader in the loop.
///
/// # Errors
///
/// [`PaintError`] from the painter, and [`PaintError::NoPixels`] if it kept none.
pub fn our_page_as_pixels(
    plates: &[Plate],
    page: usize,
) -> Result<(Pixels, DrawReport), PaintError> {
    let (list, _) = our_page(plates, page).map_err(PaintError::from)?;
    let provider = provider_for(plates).map_err(PaintError::from)?;
    let mut painter = SoftwarePainter::new();
    let report = draw(&mut painter, &list, &provider)?;
    let pixels = painter
        .read_pixels()?
        .ok_or(PaintError::NoPixels { name: "tiny-skia" })?;
    Ok((pixels, report))
}

/// One frame through `painter`, with `provider` resolving every handle.
fn draw(
    painter: &mut dyn Painter,
    list: &DisplayList,
    provider: &PresetGeometryProvider,
) -> Result<DrawReport, PaintError> {
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the page is 960 x 540 points and both are positive constants"
    )]
    let mut host = OffscreenSurface::new(SLIDE.0 as u32, SLIDE.1 as u32, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport)?;
    let mut resources = Resources::new(&mut glyphs, provider, &images);
    let report = painter.draw(&frame, list, &mut resources)?;
    painter.end(frame)?;
    Ok(report)
}

/// A six-digit `RRGGBB` as a scene colour.
///
/// # Panics
///
/// On anything that is not six hexadecimal digits. Every caller passes one of this crate's own
/// constants, so a failure is a typo in a `const` and not an input.
#[must_use]
pub fn rgb(hex: &str) -> Color {
    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16).unwrap_or_else(|_| panic!("`{hex}` is not RRGGBB"))
    };
    assert_eq!(hex.len(), 6, "`{hex}` is not six hexadecimal digits");
    Color {
        red: parse(0..2),
        green: parse(2..4),
        blue: parse(4..6),
        alpha: 0xff,
    }
}
