//! Where each glyph of a shaped run lands on a device surface.
//!
//! This is the join between [`mod@crate::shaping`], which works entirely in a face's own units, and
//! [`mod@crate::raster`], which works entirely in pixels. It walks a [`ShapedRun`] **forwards** —
//! the shaper has already put the glyphs in draw order, left to right whatever the run's direction,
//! so nothing here ever reverses anything — accumulates the pen, and turns each glyph into a
//! [`GlyphRasterKey`] and a whole-pixel position.
//!
//! # This is not layout
//!
//! Nothing here decides where a *line* goes, how tall it is, or what flows around it. It is handed
//! an origin and it places one run from it. The box model owns everything above that, from R05
//! onward.
//!
//! # Everything is placed at the bucket's scale, not at the requested size
//!
//! A run is positioned in the same quantised scale its glyphs are rasterised at, and
//! [`RunPlacement::residual_scale`] is the single number a painter multiplies the whole run by to
//! reach the size that was actually asked for. Doing it this way is what makes scale bucketing
//! exact: positions and images scale together, so no letter moves relative to another and the run's
//! width is right to the last fraction of a pixel. It is also what makes bucketing *work* — two
//! nearby sizes in one bucket produce byte-identical placements, so the second costs no
//! rasterisation at all, which the alternative (positions at the requested size, images at the
//! bucket's) would not: every glyph would land on a different subpixel phase and miss the cache.
//!
//! # Glyph offsets are not decoration
//!
//! [`crate::ShapedGlyph::x_offset`] and [`crate::ShapedGlyph::y_offset`] are how the shaper says
//! *this mark goes here relative to its base*. A placer that dropped them would draw every Arabic
//! and Devanagari mark, every combining accent and every stacked diacritic on the baseline, on top
//! of the letter it belongs over. They are applied here, scaled like everything else, and
//! `crates/mjx-text/tests/glyph_rasterisation.rs` asserts on a non-zero one — which nothing in this
//! crate did until MJXOFF-159, so the offsets were produced and never checked.

use crate::face::GlyphIndex;
use crate::raster::{FaceId, GlyphRasterKey, Hinting, ScaleBucket, SubpixelPosition};
use crate::shaping::ShapedRun;

/// How many device pixels one typographic point covers.
///
/// One number carrying the whole chain a document's size passes through on its way to a surface:
/// the display's own density, the operating system's scaling factor, and the zoom the reader chose.
/// Held as a `f32` because it is genuinely continuous — a pinch produces every value between two
/// zoom levels — and quantised only at the moment it becomes a [`ScaleBucket`].
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct DeviceScale(f32);

impl DeviceScale {
    /// The scale at which one typographic point is one CSS pixel: 96 pixels to the inch against
    /// 72 points to the inch. What an unzoomed document on a non-Retina display is drawn at, and the
    /// figure every other scale here is a multiple of.
    pub const UNZOOMED: Self = Self(96.0 / 72.0);

    /// A scale of `pixels_per_point`.
    ///
    /// Saturating rather than fallible, and deliberately: a zoom factor arrives from a gesture, and
    /// a gesture can produce a `NaN` on a divide by zero. A non-finite or negative scale becomes
    /// zero, which places every glyph at the origin in the empty bucket and draws nothing, rather
    /// than an error a gesture handler would have to invent a recovery for.
    #[must_use]
    pub fn from_pixels_per_point(pixels_per_point: f32) -> Self {
        if pixels_per_point.is_nan() || pixels_per_point <= 0.0 {
            return Self(0.0);
        }
        Self(pixels_per_point.min(f32::MAX))
    }

    /// The same scale multiplied by `zoom`.
    #[must_use]
    pub fn zoomed_by(self, zoom: f32) -> Self {
        Self::from_pixels_per_point(self.0 * zoom)
    }

    /// How many pixels one point covers.
    #[must_use]
    pub const fn pixels_per_point(self) -> f32 {
        self.0
    }
}

impl Default for DeviceScale {
    fn default() -> Self {
        Self::UNZOOMED
    }
}

/// One glyph of a run, placed.
///
/// `x` and `y` are whole device pixels **at the bucket's scale**, with y increasing downward and the
/// origin at the run's own origin, which sits on the baseline. The fraction of a pixel that `x`
/// dropped is not lost: it is in [`GlyphRasterKey::subpixel`], which is the phase the glyph's image
/// was rasterised for, so `x` plus that phase is where the glyph really is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlacedGlyph {
    /// Everything that decides the glyph's image.
    pub key: GlyphRasterKey,
    /// The byte offset, within the run's text, of the first character this glyph belongs to. Carried
    /// through unchanged from [`crate::ShapedGlyph::cluster`], because hit-testing a pixel back to a
    /// character is what it is for.
    pub cluster: u32,
    /// The whole pixel the glyph's origin sits at, horizontally.
    pub x: i32,
    /// The pixel row the glyph's origin sits on. Snapped, not phased: see [`mod@crate::raster`].
    pub y: i32,
}

impl PlacedGlyph {
    /// Which glyph in the face.
    #[must_use]
    pub const fn glyph(&self) -> GlyphIndex {
        self.key.glyph
    }

    /// Where the glyph's origin really is horizontally, whole pixel plus rasterised phase.
    #[must_use]
    pub fn exact_x_in_pixels(&self) -> f32 {
        // `i32` to `f32` is exact below 2^24 and rounds above it; a position past sixteen million
        // pixels is off every surface, and rounding it is the right answer there.
        self.x as f32 + self.key.subpixel.offset_in_pixels()
    }
}

/// A shaped run, placed on a device surface.
#[derive(Clone, PartialEq, Debug)]
pub struct RunPlacement {
    bucket: ScaleBucket,
    residual_scale: f32,
    requested_pixels_per_em: f32,
    glyphs: Vec<PlacedGlyph>,
    advance_in_pixels: f32,
}

impl RunPlacement {
    /// The glyphs, in draw order — the order [`ShapedRun::glyphs`] was already in.
    #[must_use]
    pub fn glyphs(&self) -> &[PlacedGlyph] {
        &self.glyphs
    }

    /// How many glyphs the run placed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    /// Whether the run placed no glyphs at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// The quantised scale every position and image in this placement is expressed at.
    #[must_use]
    pub fn bucket(&self) -> ScaleBucket {
        self.bucket
    }

    /// What a painter multiplies this whole placement by — positions, images and advance together —
    /// to draw it at the size that was asked for.
    #[must_use]
    pub fn residual_scale(&self) -> f32 {
        self.residual_scale
    }

    /// The size that was asked for, in pixels per em, before it was quantised into a bucket.
    #[must_use]
    pub fn requested_pixels_per_em(&self) -> f32 {
        self.requested_pixels_per_em
    }

    /// How far the pen moved, in pixels at the bucket's scale.
    #[must_use]
    pub fn advance_in_pixels(&self) -> f32 {
        self.advance_in_pixels
    }

    /// How far the pen moved at the size that was asked for.
    #[must_use]
    pub fn advance_at_requested_size(&self) -> f32 {
        self.advance_in_pixels * self.residual_scale
    }
}

/// Place `run` on a surface, starting from `origin`.
///
/// `origin` is in device pixels at the requested scale's *bucket*, with y downward and the y
/// coordinate on the **baseline**, which is where a pen sits. The face must already be registered
/// with the rasteriser that will draw these glyphs; `face` is the identity
/// [`crate::GlyphRasteriser::register`] handed back.
///
/// The run's own [`crate::FontSize`] and `device_scale` decide the bucket between them: a size is in
/// points, a scale is pixels per point, and their product is pixels per em, which is what a
/// rasteriser wants.
#[must_use]
pub fn place_run(
    run: &ShapedRun,
    face: FaceId,
    device_scale: DeviceScale,
    hinting: Hinting,
    origin: (f32, f32),
) -> RunPlacement {
    // `in_points` is a `f64` because a size is stored in thousandths of a point; the product is
    // narrowed once, here, rather than carrying a `f64` through arithmetic a surface will do in
    // `f32` anyway.
    let requested = (run.size().in_points() * f64::from(device_scale.pixels_per_point())) as f32;
    let bucket = ScaleBucket::enclosing(requested);
    let residual_scale = bucket.residual_scale(requested);

    let units_per_em = f32::from(run.units_per_em().max(1));
    let scale = bucket.pixels_per_em() / units_per_em;

    let mut glyphs = Vec::with_capacity(run.len());
    let mut pen_x = origin.0;
    let mut pen_y = origin.1;

    for glyph in run.glyphs() {
        // The offsets are what put a mark over its base rather than on the baseline. `y_offset` is
        // upward in font units and this surface's y is downward, so it is subtracted.
        let drawn_x = pen_x + glyph.x_offset as f32 * scale;
        let drawn_y = pen_y - glyph.y_offset as f32 * scale;

        let (x, subpixel) = SubpixelPosition::split(drawn_x);
        glyphs.push(PlacedGlyph {
            key: GlyphRasterKey {
                face,
                glyph: glyph.glyph,
                bucket,
                subpixel,
                hinting,
            },
            cluster: glyph.cluster,
            x,
            y: round_to_pixel(drawn_y),
        });

        pen_x += glyph.x_advance as f32 * scale;
        pen_y -= glyph.y_advance as f32 * scale;
    }

    RunPlacement {
        bucket,
        residual_scale,
        requested_pixels_per_em: requested,
        glyphs,
        advance_in_pixels: pen_x - origin.0,
    }
}

/// Round a vertical position to the pixel grid, totally.
///
/// A baseline is snapped rather than phased — see [`mod@crate::raster`] — and the clamp is the same
/// guard [`SubpixelPosition::split`] applies horizontally: the number came from advances in an
/// untrusted face, so it may be enormous or not a number at all.
fn round_to_pixel(position: f32) -> i32 {
    if !position.is_finite() {
        return 0;
    }
    const LIMIT: f32 = 1_073_741_824.0;
    position.round().clamp(-LIMIT, LIMIT) as i32
}
