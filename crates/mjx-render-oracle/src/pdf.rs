//! The PDF comparison pipeline: **the layout tier and the pixel tier**, both working today against
//! our own exports.
//!
//! # Why "pixel perfect against PowerPoint" is only coherent through PDF
//!
//! Comparing our screen render against PowerPoint's can never be pixel-identical: different
//! rasterisers, different hinting, different antialiasing, different subpixel positioning, and all
//! four differ even when the layout is *exactly* right. Framed that way the gate fails forever and
//! teaches nobody anything.
//!
//! Route both sides through PDF and the phrase becomes achievable:
//!
//! * **The layout tier — the strong one.** `pdftotext -bbox-layout` gives **word-level bounding
//!   boxes** from both documents. Exact, rasteriser-independent, and *diagnosable*: a line-break
//!   divergence names the word rather than smearing grey over a diff image. **No provider exclusion
//!   touches it**, because a word's box says nothing whatever about how the shape behind it is
//!   filled — which is exactly why it stays meaningful on the content whose pixel tier is
//!   compromised.
//! * **The pixel tier.** `pdftoppm` rasterises **both** PDFs at the same DPI with **one
//!   rasteriser**, so antialiasing cancels and a remaining difference is real. This is the tier the
//!   gradient exclusion applies to.
//!
//! # Proved before the Windows sitting, not during it
//!
//! MJXOFF-165 asks for this to work *today*, and the way it is proved is that both sides are our
//! own exports: `tests/the_pdf_tiers_work_on_our_own_exports.rs` exports the same page twice with
//! one word moved eight points, and asserts that the layout tier **names that word**. A pipeline
//! first exercised on the morning the Office artefacts arrive is a pipeline debugged on the one day
//! it is expensive to debug.
//!
//! # The text here, and why it is not in a golden image
//!
//! The layout tier needs words, and words need a face. This module draws with the committed
//! Liberation Sans in `crates/mjx-text/assets/fonts/`, and that is safe **because nothing here is
//! committed**: both sides are generated in the same run, so a change to the glyph rasteriser moves
//! both and the comparison still measures what it claims. A *golden image* containing glyphs would
//! be a different matter, and [`crate::specimen`] says why there are none.

use std::path::Path;

use mjx_paint::{
    FaceLibrary, NoGlyphs, NoImages, OffscreenSurface, Painter, PdfPainter, Resources, Viewport,
};
use mjx_scene::{
    AtlasPlacement, BitmapFormat, Command, DeviceScale, DisplayList, GlyphImage, Hinting, Paint,
    ScaleBucket, SceneBuilder, SceneGlyph, SceneGlyphRun, ScenePoint, SceneRect, TextDirection,
};

use crate::perceptual::{compare, Difference, Tolerance};
use crate::png::Image;
use crate::tools::{rasterise, word_boxes, Raster, WordBox};

/// The page the layout tier draws on, in points.
pub const PAGE: (f32, f32) = (360.0, 120.0);

/// The words, drawn as separate runs so that `pdftotext` cannot join two of them into one box.
///
/// Five words, none a prefix of another and none repeated: a comparison that matched the wrong pair
/// would have to get five independent things wrong before it looked right.
pub const WORDS: [&str; 5] = ["Fidelity", "opens", "every", "single", "package"];

/// Which word [`text_page`]'s `shift` moves. The middle one, so a divergence has words on both sides
/// of it and a comparison that silently dropped the first or the last would still be caught.
pub const MOVED_WORD: usize = 2;

/// The committed face this module draws with.
///
/// # Errors
///
/// A sentence naming the path that could not be read or parsed.
pub fn face() -> Result<std::sync::Arc<mjx_text::FontFace>, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mjx-text/assets/fonts/LiberationSans-Regular.ttf");
    let bytes =
        std::fs::read(&path).map_err(|error| format!("reading {}: {error}", path.display()))?;
    mjx_text::FontFace::parse(std::sync::Arc::from(bytes.as_slice()), 0)
        .map(std::sync::Arc::new)
        .map_err(|error| format!("parsing {}: {error}", path.display()))
}

/// A page of [`WORDS`], with the word at [`MOVED_WORD`] displaced by `shift` points.
///
/// # Errors
///
/// A sentence, when the face will not read or the builder refuses the scene.
pub fn text_page(shift: f32) -> Result<DisplayList, String> {
    let face = face()?;
    let reader = face
        .reader()
        .map_err(|error| format!("reading the committed face: {error}"))?;
    let size = 18.0f32;
    let units = f32::from(reader.units_per_em().max(1));

    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, PAGE.0, PAGE.1);
    let background = builder
        .add_geometry(&mjx_scene::Geometry::Rectangle(SceneRect::new(
            0.0, 0.0, PAGE.0, PAGE.1,
        )))
        .map_err(|error| format!("the background: {error}"))?;
    let paper = builder
        .add_paint(Paint::Solid(mjx_scene::Color {
            red: 0xff,
            green: 0xff,
            blue: 0xff,
            alpha: 0xff,
        }))
        .map_err(|error| format!("the paper: {error}"))?;
    builder
        .push(Command::FillPath {
            geometry: background,
            paint: paper,
        })
        .map_err(|error| format!("the background fill: {error}"))?;
    let ink = builder
        .add_paint(Paint::Solid(mjx_scene::Color {
            red: 0x10,
            green: 0x10,
            blue: 0x30,
            alpha: 0xff,
        }))
        .map_err(|error| format!("the ink: {error}"))?;

    let mut pen_x = 16.0f32;
    for (index, word) in WORDS.iter().enumerate() {
        let mut glyphs = Vec::with_capacity(word.len());
        let mut advance_total = 0.0f32;
        for (position, character) in word.chars().enumerate() {
            let Some(glyph) = reader.glyph_for_character(character) else {
                continue;
            };
            let advance = reader.advance(glyph).map_or(size * 0.5, |advance| {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "an advance in font units is at most a few thousand"
                )]
                {
                    advance.font_units as f32 * size / units
                }
            });
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a pen position on a 360-point page"
            )]
            glyphs.push(SceneGlyph {
                x: advance_total.round() as i32,
                y: 0,
                cluster: position as u32,
                glyph: glyph.0,
                subpixel: 0,
                // A stand-in atlas rectangle: an exporter needs the glyph **id**, which is real
                // above, and never opens the atlas. See `mjx-paint`'s own text fixtures, which say
                // the same thing at more length.
                image: GlyphImage::Atlas(AtlasPlacement {
                    page: 0,
                    format: BitmapFormat::Coverage,
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 8,
                    offset_from_origin_x: 0,
                    offset_from_origin_y: -8,
                }),
            });
            advance_total += advance;
        }
        let offset = if index == MOVED_WORD { shift } else { 0.0 };
        let run = builder
            .add_glyph_run(&SceneGlyphRun {
                face: 0,
                bucket_steps: ScaleBucket::enclosing(size).steps(),
                residual_scale: 1.0,
                origin: ScenePoint::new(pen_x + offset, 70.0),
                direction: TextDirection::LeftToRight,
                hinting: Hinting::GridFitted,
                level: 0,
                glyphs,
            })
            .map_err(|error| format!("the run for `{word}`: {error}"))?;
        builder
            .push(Command::DrawGlyphs { run, paint: ink })
            .map_err(|error| format!("drawing `{word}`: {error}"))?;
        // A full em of clear space between words, so poppler splits them rather than joining two
        // into one box. A gap that was merely "wide enough" would be a gap that stops being wide
        // enough the first time a word gets longer.
        pen_x += advance_total + size;
    }
    builder
        .finish()
        .map_err(|error| format!("the text page: {error}"))
}

/// Export `list` as a PDF, with the committed face embedded.
///
/// # Errors
///
/// A sentence, from the painter or from a face that will not read.
pub fn export(list: &DisplayList) -> Result<Vec<u8>, String> {
    let face = face()?;
    let mut library = FaceLibrary::new();
    library.insert(0, face);
    let mut painter = PdfPainter::new();
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let provider = mjx_geometry::PresetGeometryProvider::new();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the page is 360 by 120 points and both are positive constants"
    )]
    let mut host = OffscreenSurface::new(PAGE.0 as u32, PAGE.1 as u32, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter
        .begin(&mut host, viewport)
        .map_err(|error| format!("beginning the export: {error}"))?;
    let mut resources = Resources::new(&mut glyphs, &provider, &images).with_fonts(&library);
    painter
        .draw(&frame, list, &mut resources)
        .map_err(|error| format!("drawing the export: {error}"))?;
    painter
        .end(frame)
        .map_err(|error| format!("finishing the export: {error}"))?;
    painter
        .document()
        .map(<[u8]>::to_vec)
        .ok_or_else(|| "the PDF exporter finished with no document".to_owned())
}

/// What the layout tier concluded about two documents.
#[derive(Clone, PartialEq, Debug)]
pub struct WordComparison {
    /// How many words each side had.
    pub counts: (usize, usize),
    /// How far apart two boxes may be, in points, and still be the same box.
    pub allowed_points: f64,
    /// The first divergence, **naming the word**, or `None` when the two agree.
    pub first_difference: Option<String>,
    /// How many words moved further than the tolerance.
    pub moved: usize,
}

impl WordComparison {
    /// Whether the two documents put every word in the same place.
    #[must_use]
    pub const fn agreed(&self) -> bool {
        self.first_difference.is_none()
    }
}

/// How far apart two word boxes may be, in points, and still count as the same place.
///
/// A quarter of a point. Not zero: two PDF producers place a run by writing a text matrix, and the
/// same position written by two producers can round differently in the last digit poppler parses.
/// Not a whole point either: a point is a fifth of the smallest interword gap on any page this
/// crate draws, so a tolerance of one would begin to absorb real movement.
pub const WORD_TOLERANCE_POINTS: f64 = 0.25;

/// Compare two documents' word boxes.
///
/// Words are matched **in reading order** — top to bottom, then left to right — which is what a
/// person comparing two pages does and what makes a divergence nameable. Matching by text would
/// hide the one failure this tier exists for: a word that moved to another line still has the same
/// text.
#[must_use]
pub fn compare_words(left: &[WordBox], right: &[WordBox], allowed_points: f64) -> WordComparison {
    let ordered = |words: &[WordBox]| -> Vec<WordBox> {
        let mut sorted = words.to_vec();
        sorted.sort_by(|a, b| {
            (a.page, ordinal(a.y_min), ordinal(a.x_min)).cmp(&(
                b.page,
                ordinal(b.y_min),
                ordinal(b.x_min),
            ))
        });
        sorted
    };
    let (left, right) = (ordered(left), ordered(right));
    let mut moved = 0usize;
    let mut first_difference = None;
    for (index, (a, b)) in left.iter().zip(right.iter()).enumerate() {
        let gap = (a.x_min - b.x_min)
            .abs()
            .max((a.y_min - b.y_min).abs())
            .max((a.x_max - b.x_max).abs())
            .max((a.y_max - b.y_max).abs());
        let same_text = a.text == b.text;
        if gap > allowed_points || !same_text {
            moved += 1;
            if first_difference.is_none() {
                first_difference = Some(if same_text {
                    format!(
                        "word {index} `{}` is at ({:.2}, {:.2}) and was at ({:.2}, {:.2}) — \
                         {gap:.2} points, {allowed_points:.2} allowed",
                        a.text, a.x_min, a.y_min, b.x_min, b.y_min
                    )
                } else {
                    format!(
                        "word {index} reads `{}` and was `{}`, at ({:.2}, {:.2}) against \
                         ({:.2}, {:.2})",
                        a.text, b.text, a.x_min, a.y_min, b.x_min, b.y_min
                    )
                });
            }
        }
    }
    if left.len() != right.len() && first_difference.is_none() {
        first_difference = Some(format!(
            "one document has {} words and the other {}",
            left.len(),
            right.len()
        ));
    }
    WordComparison {
        counts: (left.len(), right.len()),
        allowed_points,
        first_difference,
        moved,
    }
}

/// A total order over a coordinate, so two word lists sort identically.
///
/// Coordinates are rounded to a tenth of a point before ordering, so that two words on the same line
/// whose `yMin` differs in the sixth decimal do not swap places between two runs — which would
/// present as a spurious divergence in the tier that exists to be exact.
fn ordinal(value: f64) -> i64 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a coordinate on a page of a few hundred points"
    )]
    {
        (value * 10.0).round() as i64
    }
}

/// The layout tier over two PDFs on disk.
///
/// # Errors
///
/// A sentence naming what `pdftotext` said.
pub fn layout_tier(
    left: &Path,
    right: &Path,
    allowed_points: f64,
) -> Result<WordComparison, String> {
    Ok(compare_words(
        &word_boxes(left)?,
        &word_boxes(right)?,
        allowed_points,
    ))
}

/// The pixel tier over two PDFs on disk: **one rasteriser, both sides, one DPI**.
///
/// # Errors
///
/// A sentence naming what `pdftoppm` said, or that the two pages are different sizes.
pub fn pixel_tier(
    left: &Path,
    right: &Path,
    page: usize,
    tolerance: Tolerance,
) -> Result<Difference, String> {
    let left = rasterise(left, page)?;
    let right = rasterise(right, page)?;
    compare(&as_image(&left), &as_image(&right), tolerance)
}

/// A `pdftoppm` raster as an opaque straight-alpha image, so one comparison serves both tiers.
#[must_use]
pub fn as_image(raster: &Raster) -> Image {
    let mut rgba = Vec::with_capacity(raster.rgb.len() / 3 * 4);
    for pixel in raster.rgb.as_chunks::<3>().0 {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 0xff]);
    }
    Image {
        width: raster.width,
        height: raster.height,
        rgba,
    }
}
