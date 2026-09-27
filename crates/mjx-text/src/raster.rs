//! Turning a glyph into pixels — or, above a size, into a path.
//!
//! Shaping ([`mod@crate::shaping`]) answers *which glyphs, in what order, at what positions*,
//! entirely in the face's own units. This module answers the next question: *what does one of those
//! glyphs look like at this zoom level*. It is the first module in the crate that has heard of a
//! pixel.
//!
//! # Why the size a glyph is rasterised at is not the size it is drawn at
//!
//! A document is read at a continuously varying scale — a pinch, a zoom slider, a window that
//! changes size — and rasterising a glyph is expensive: an outline is scaled, hinted, scan-converted
//! and antialiased. Doing that for every glyph on every frame of a zoom is unaffordable, and holding
//! a bitmap for every glyph at every scale is unbounded.
//!
//! So the scale is **quantised** into [`ScaleBucket`]s, and a run is rasterised *and positioned* at
//! its bucket's size rather than at the size that was asked for. The difference between the two is
//! one number, [`ScaleBucket::residual_scale`], which the painter applies to the whole run as a
//! uniform scale. That is what makes the technique exact rather than approximate: because positions
//! and images are scaled by the same factor, no letter moves relative to another and no line changes
//! width. The only cost is that a glyph's image is resampled by at most one bucket step, which is
//! [`SCALE_BUCKET_STEP_PIXELS_PER_EM`] out of the requested size.
//!
//! # Why a glyph is rasterised more than once at one size
//!
//! Text does not sit on whole pixels. A pen advances by a fractional number of them, and snapping
//! every glyph to the pixel grid is what makes text visibly jitter as it scrolls — letters bunch and
//! separate by a pixel at a time. The fix is to rasterise each glyph at a small fixed set of
//! horizontal phases, [`SUBPIXEL_POSITION_COUNT`] of them, and pick the one nearest the position the
//! glyph actually wants. Vertical positions are snapped, deliberately: hinting aligns stems and
//! baselines to the pixel grid, and a fractional vertical offset would blur every baseline in the
//! document to undo the work the hinter just did.
//!
//! # Why there are two routes out of here
//!
//! Below [`OUTLINE_PIXELS_PER_EM_THRESHOLD`] a glyph is worth a bitmap. Above it, it is not: the
//! bitmap grows with the square of the size, the atlas fills with a handful of letters, and the
//! hinting that justifies a bitmap at body sizes stops mattering because the stems are many pixels
//! wide. So a large glyph comes out as a [`GlyphOutline`] instead, for R07 to tessellate — one path
//! that is correct at every zoom, rather than one bitmap per bucket.
//!
//! # Colour glyphs are not an edge case
//!
//! An emoji in a document is ordinary, and a colour glyph is not an outline: `COLR`/`CPAL` is a
//! stack of layers with palette indices, and `sbix` and `CBDT` are pictures with no outline at all.
//! Those rasterise to [`BitmapFormat::Rgba`] rather than to coverage. **Which formats a face carries
//! is read from [`crate::FontFace::colour_formats`]** — the tables are probed once, when the face is
//! parsed, and this module asks that answer rather than re-opening `COLR`, `sbix` and `CBDT` for
//! itself.
//!
//! # Why `swash`, and why it is given bytes rather than a parsed face
//!
//! Three bodies of arithmetic over untrusted tables are needed here and none of them is worth
//! writing twice: the TrueType hinting interpreter, the scan converter, and the compositor that
//! flattens a `COLR` layer stack onto one image. `swash` has all three, is pure Rust, and reads
//! fonts through `skrifa` — a *different* parser from the `ttf-parser` the rest of this crate uses.
//! That is exactly why [`crate::FontFace`] hands out [`crate::FontFace::data`] and
//! [`crate::FontFace::index`] and never a parsed type: each parser reads the file for itself, and
//! neither is ever given the other's view of it.
//!
//! # Nothing here panics on a font
//!
//! A glyph table inside a `.pptx` is exactly as untrusted as the `.pptx`. Every size is clamped
//! before it reaches an allocation, every cast is bounded, and a face `swash` will not read, a glyph
//! the face draws nothing for, and a size that would ask for a gigabyte all come back as a value or
//! a [`FontError`] rather than as a panic.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::error::FontError;
use crate::face::{FontFace, GlyphIndex};

/// The width of one scale bucket, in pixels per em.
///
/// This is the trade between how often a zoom re-rasterises and how far a drawn glyph may be scaled
/// away from the size its bitmap was made at. A quarter of a pixel per em is 1.6 % of a 16 ppem body
/// size and 0.26 % of a 96 ppem heading — under the threshold at which a resampled stem visibly
/// changes weight — while a pinch that doubles the scale over a second moves it by roughly 1.2 % per
/// frame at 60 Hz, so consecutive frames land in the same bucket about half the time and a slower,
/// more usual pinch almost always does. Every bucket a zoom passes through stays in the atlas, so
/// zooming back through one costs nothing at all.
///
/// It is deliberately **not** a geometric ladder. A constant ratio is the obvious design and it is
/// wrong at both ends: a ratio wide enough to be worth having at 96 ppem is a step of several points
/// at 12, which is a visible change of size, and a ratio fine enough at 12 ppem saves nothing at 96.
pub const SCALE_BUCKET_STEP_PIXELS_PER_EM: f32 = 0.25;

/// The largest scale a [`ScaleBucket`] can name, in pixels per em.
///
/// Positions are computed at the bucket's scale, so this bounds a *zoom* rather than a bitmap: 4096
/// pixels to the em is one letter filling a 4K display, past which nothing legible is being read. It
/// exists so that a document declaring `sz="2147483647"`, or a caller multiplying a size by an
/// enormous zoom factor, produces a clamped number rather than an infinity that would poison every
/// position derived from it.
pub const MAXIMUM_PIXELS_PER_EM: f32 = 4096.0;

/// Above this many pixels per em a glyph is emitted as an outline instead of a bitmap.
///
/// Three reasons converge on roughly this figure, and it is one constant so that moving it moves all
/// three together.
///
/// * **Cost.** A bitmap's area grows with the square of the size. At 96 ppem a dense glyph is about
///   nine thousand coverage bytes, so one line of a heading already occupies a third of a
///   [`crate::atlas::ATLAS_PAGE_SIZE_PIXELS`]-square page; at 200 ppem a single *word* would not fit
///   one page.
/// * **Benefit.** Hinting and antialiasing are what a bitmap buys, and both stop mattering as the
///   stems get wide. At body sizes a stem is one or two pixels and grid-fitting decides whether the
///   letter is legible; at 96 ppem it is six or seven and the unhinted outline is indistinguishable.
/// * **Reuse.** A path is correct at every scale, so a heading being zoomed is tessellated once
///   rather than re-rasterised at every bucket the zoom passes through.
///
/// A face carrying colour glyphs is exempt: see [`GlyphRasteriser::render`].
pub const OUTLINE_PIXELS_PER_EM_THRESHOLD: f32 = 96.0;

/// The largest size this module will rasterise a bitmap at, in pixels per em.
///
/// Nothing on the ordinary route reaches it, because [`OUTLINE_PIXELS_PER_EM_THRESHOLD`] is far
/// below it. It is the guard on the one route that has no such threshold — a colour glyph, which
/// stays a bitmap at any size because it has no single outline. It is set to the atlas page size, so
/// a bitmap that passes this check is one a page could in principle hold, and a request past it is
/// refused as [`FontError::GlyphTooLargeToRasterise`] **before** anything allocates for it rather
/// than after: at 4096 pixels to the em one colour glyph would be sixty-four megabytes.
pub const MAXIMUM_RASTERISED_PIXELS_PER_EM: f32 = 512.0;

/// How many horizontal phases a glyph is rasterised at within one pixel.
///
/// Four is the number every mainstream text stack has settled on, and the reasoning is the same
/// here: the error left after quantising to a quarter of a pixel is an eighth of a pixel, below what
/// the eye resolves as jitter in a scrolling line, while the cost is a factor of four on the number
/// of cached images rather than the unbounded factor an exact position would need. Two phases leave
/// a visible half-pixel wobble; eight quadruple the atlas for a difference nobody can see.
pub const SUBPIXEL_POSITION_COUNT: u8 = 4;

/// How many outlines a [`GlyphRasteriser`] remembers before it starts forgetting the oldest.
///
/// Small on purpose. Outlines are produced only above [`OUTLINE_PIXELS_PER_EM_THRESHOLD`], and a
/// page of a document holds very few glyphs that large — a title, a slide's headline, a drop cap.
/// The cache exists so that a heading held still on screen is not re-scaled on every frame, not to
/// hold a working set.
pub const DEFAULT_OUTLINE_CACHE_CAPACITY: usize = 256;

/// A quantised rasterisation scale.
///
/// Held as a count of [`SCALE_BUCKET_STEP_PIXELS_PER_EM`] steps, so it is `Eq` and `Hash`. A `f32`
/// could be neither, and two scales that print the same could differ in the last bit and split one
/// cache entry into two.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ScaleBucket(u32);

impl ScaleBucket {
    /// The smallest bucket at or above `pixels_per_em`.
    ///
    /// At or *above*, never below: a glyph drawn from a bitmap made at a smaller size is visibly
    /// soft, and one drawn from a bitmap made at a larger size is not.
    ///
    /// A negative, zero, `NaN` or infinite request all come back as the empty bucket, which
    /// rasterises to [`GlyphRender::Blank`]. A size is a number out of a document and may be any of
    /// those.
    #[must_use]
    pub fn enclosing(pixels_per_em: f32) -> Self {
        if pixels_per_em.is_nan() || pixels_per_em <= 0.0 {
            return Self(0);
        }
        let steps = (pixels_per_em / SCALE_BUCKET_STEP_PIXELS_PER_EM).ceil();
        let highest = MAXIMUM_PIXELS_PER_EM / SCALE_BUCKET_STEP_PIXELS_PER_EM;
        // `steps` is positive here and `min` bounds it below `u32::MAX`, so the cast is exact; an
        // infinity is bounded by the same `min`.
        Self(steps.min(highest) as u32)
    }

    /// The bucket `steps` steps above zero. Chiefly of use to a caller that wants two buckets known
    /// to be adjacent.
    #[must_use]
    pub const fn from_steps(steps: u32) -> Self {
        Self(steps)
    }

    /// How many steps above zero this bucket is.
    #[must_use]
    pub const fn steps(self) -> u32 {
        self.0
    }

    /// The size this bucket rasterises at, in pixels per em.
    #[must_use]
    pub fn pixels_per_em(self) -> f32 {
        // `u32` to `f32` loses precision above 2^24; buckets stop at 16384, far below it.
        self.0 as f32 * SCALE_BUCKET_STEP_PIXELS_PER_EM
    }

    /// Whether this bucket names no size at all, so nothing drawn at it would be visible.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// What a painter multiplies the whole run by to reach the size that was asked for.
    ///
    /// Never far from `1.0` — at most one bucket step below it — and applied uniformly to positions
    /// and images together, which is why bucketing costs no geometric accuracy at all. It is `1.0`
    /// for the empty bucket, because there is nothing to scale.
    #[must_use]
    pub fn residual_scale(self, requested_pixels_per_em: f32) -> f32 {
        let bucket = self.pixels_per_em();
        if bucket <= 0.0 || !requested_pixels_per_em.is_finite() {
            return 1.0;
        }
        requested_pixels_per_em / bucket
    }

    /// Whether a glyph at this bucket is emitted as an outline rather than rasterised.
    #[must_use]
    pub fn is_above_the_outline_threshold(self) -> bool {
        self.pixels_per_em() > OUTLINE_PIXELS_PER_EM_THRESHOLD
    }
}

impl fmt::Debug for ScaleBucket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ScaleBucket({}ppem)", self.pixels_per_em())
    }
}

/// Which of the [`SUBPIXEL_POSITION_COUNT`] horizontal phases within a pixel a glyph is drawn at.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct SubpixelPosition(u8);

impl SubpixelPosition {
    /// The phase exactly on the pixel boundary.
    pub const ALIGNED: Self = Self(0);

    /// Split a horizontal position into the whole pixel a glyph is drawn at and the phase it is
    /// rasterised for.
    ///
    /// The two come back together because they are one number: rounding to the nearest phase can
    /// carry into the next pixel, and a caller that quantised the phase separately from the pixel
    /// would place a glyph a whole pixel from where it asked for, once in every eight.
    ///
    /// A position that is not finite comes back as the origin, aligned — positions are derived from
    /// advances that came out of an untrusted face.
    #[must_use]
    pub fn split(position_in_pixels: f32) -> (i32, Self) {
        if !position_in_pixels.is_finite() {
            return (0, Self::ALIGNED);
        }
        // Bounded well inside `i32` so the cast below can neither saturate nor wrap. A position this
        // far out is off every conceivable surface; clamping keeps the arithmetic total.
        const LIMIT: f32 = 1_073_741_824.0;
        let phases = f32::from(SUBPIXEL_POSITION_COUNT);
        let steps = (position_in_pixels * phases).round().clamp(-LIMIT, LIMIT) as i32;
        let count = i32::from(SUBPIXEL_POSITION_COUNT);
        let whole = steps.div_euclid(count);
        // `rem_euclid` with a positive divisor lands in `0..count`, and `count` came from a `u8`.
        let phase = steps.rem_euclid(count) as u8;
        (whole, Self(phase))
    }

    /// The phase `index` steps into the pixel, wrapped into range.
    #[must_use]
    pub fn from_index(index: u8) -> Self {
        Self(index % SUBPIXEL_POSITION_COUNT)
    }

    /// Which phase this is, counting from zero.
    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }

    /// How far into the pixel this phase sits, in pixels.
    #[must_use]
    pub fn offset_in_pixels(self) -> f32 {
        f32::from(self.0) / f32::from(SUBPIXEL_POSITION_COUNT)
    }
}

/// Whether a glyph's outline is fitted to the pixel grid before it is scan-converted.
///
/// Grid-fitting is what keeps stems the same width and baselines on the same row at body sizes. It
/// is part of the raster key rather than a rasteriser-wide setting because it changes the image: a
/// caller that flipped it would otherwise be served whichever version happened to be cached.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum Hinting {
    /// Fit the outline to the pixel grid. What body text wants.
    #[default]
    GridFitted,
    /// Scan-convert the outline as the face draws it. What a rotated or animated run wants, because
    /// grid-fitting a glyph that is about to be transformed fits it to the wrong grid.
    Unhinted,
}

/// A face's identity within one [`GlyphRasteriser`], as [`GlyphRasteriser::register`] assigned it.
///
/// A small integer rather than a pointer or a handle, so that a [`GlyphRasterKey`] stays `Copy`,
/// small and cheap to hash — a glyph key is looked up once per glyph per frame, which is tens of
/// thousands of times a second. `docs/UI_PLATFORM_PLAN.md` §4 calls the same thing a *font id* where
/// it describes what a `GlyphRunFragment` carries.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FaceId(u32);

impl FaceId {
    /// The identity as a number, for a diagnostic message.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Everything that decides what one glyph's image looks like.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GlyphRasterKey {
    /// Which face, within the rasteriser that issued the identity.
    pub face: FaceId,
    /// Which glyph within that face.
    pub glyph: GlyphIndex,
    /// The quantised size it is rasterised at.
    pub bucket: ScaleBucket,
    /// Which phase within a pixel it is rasterised for.
    pub subpixel: SubpixelPosition,
    /// Whether it is fitted to the pixel grid.
    pub hinting: Hinting,
}

/// How many bytes a pixel takes, and what they mean.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BitmapFormat {
    /// One byte per pixel: how much of the pixel the glyph covers. The colour comes from the
    /// document, and the painter multiplies it by this.
    Coverage,
    /// Four bytes per pixel — red, green, blue, alpha, in that order, **not** premultiplied by the
    /// alpha. A colour glyph carries its own colours, so the document's text colour does not apply.
    Rgba,
}

impl BitmapFormat {
    /// How many bytes one pixel occupies.
    #[must_use]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Coverage => 1,
            Self::Rgba => 4,
        }
    }
}

/// One glyph, rasterised.
///
/// The offsets are in device pixels from the glyph's origin — the point on the baseline the pen is
/// at — to the bitmap's **top-left** corner, with **y increasing downward**, which is the convention
/// every surface this eventually reaches uses. So `offset_from_origin_y` is normally negative,
/// because a letter is drawn above its baseline.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GlyphBitmap {
    format: BitmapFormat,
    width: u16,
    height: u16,
    offset_from_origin_x: i16,
    offset_from_origin_y: i16,
    pixels: Vec<u8>,
}

impl GlyphBitmap {
    /// What the bytes mean.
    #[must_use]
    pub const fn format(&self) -> BitmapFormat {
        self.format
    }

    /// How many pixels wide.
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// How many pixels tall.
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// Pixels from the glyph's origin rightward to the bitmap's left edge.
    #[must_use]
    pub const fn offset_from_origin_x(&self) -> i16 {
        self.offset_from_origin_x
    }

    /// Pixels from the glyph's origin downward to the bitmap's top edge; normally negative.
    #[must_use]
    pub const fn offset_from_origin_y(&self) -> i16 {
        self.offset_from_origin_y
    }

    /// The pixels, row by row from the top, with no padding between rows.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// How many bytes the pixels occupy.
    ///
    /// Never zero: a glyph with no pixels comes back from [`GlyphRasteriser::render`] as
    /// [`GlyphRender::Blank`] rather than as an empty bitmap, so a caller never has to test for one.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.pixels.len()
    }
}

/// A point on a glyph's outline, in device pixels at the bucket's scale, with **y increasing
/// downward** — the same convention as [`GlyphBitmap`]'s offsets, so a caller never has to know
/// which route a glyph took in order to know which way is up.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct OutlinePoint {
    /// Pixels right of the glyph's origin.
    pub x: f32,
    /// Pixels below the glyph's origin.
    pub y: f32,
}

/// One step of a glyph's outline.
///
/// Quadratic and cubic curves are both here and neither is converted into the other: a TrueType
/// outline is quadratic and a `CFF` outline is cubic, and elevating one to the other would add
/// control points a tessellator then has to flatten more of.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OutlineCommand {
    /// Begin a new contour at this point.
    MoveTo(OutlinePoint),
    /// A straight line to this point.
    LineTo(OutlinePoint),
    /// A quadratic curve through one control point to the second point.
    QuadraticTo(OutlinePoint, OutlinePoint),
    /// A cubic curve through two control points to the third point.
    CubicTo(OutlinePoint, OutlinePoint, OutlinePoint),
    /// Close the current contour back to where it began.
    Close,
}

/// A glyph as a path rather than as pixels — the route a glyph takes above
/// [`OUTLINE_PIXELS_PER_EM_THRESHOLD`], for R07 to tessellate.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct GlyphOutline {
    commands: Vec<OutlineCommand>,
}

impl GlyphOutline {
    /// A path somebody else walked out of the face.
    ///
    /// [`crate::FaceReader::outline`] is the other producer of one of these, and it is a *vector
    /// exporter's* route into the same type: an SVG or a PDF wants a path at every size, where this
    /// module's rasteriser produces one only above [`OUTLINE_PIXELS_PER_EM_THRESHOLD`]. One type
    /// for both is what stops an exporter growing a second outline vocabulary.
    #[must_use]
    pub fn from_commands(commands: Vec<OutlineCommand>) -> Self {
        Self { commands }
    }

    /// The path, in order.
    #[must_use]
    pub fn commands(&self) -> &[OutlineCommand] {
        &self.commands
    }

    /// How many commands the path has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the glyph drew nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Which of the two routes a glyph took, as a value a test or a diagnostic can compare.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GlyphRoute {
    /// Rasterised to pixels, destined for the atlas.
    Bitmap,
    /// Emitted as a path, destined for a tessellator.
    Outline,
    /// Nothing to draw: a space, a glyph with an empty outline, or a size of zero.
    Blank,
}

/// What one glyph rasterised to.
#[derive(Clone, PartialEq, Debug)]
pub enum GlyphRender {
    /// Pixels.
    Bitmap(GlyphBitmap),
    /// A path. Behind an [`Arc`] because the rasteriser keeps one and hands out clones.
    Outline(Arc<GlyphOutline>),
    /// Nothing drawable.
    Blank,
}

impl GlyphRender {
    /// Which route this took.
    #[must_use]
    pub fn route(&self) -> GlyphRoute {
        match self {
            Self::Bitmap(_) => GlyphRoute::Bitmap,
            Self::Outline(_) => GlyphRoute::Outline,
            Self::Blank => GlyphRoute::Blank,
        }
    }
}

/// What a [`GlyphRasteriser`] has been doing.
///
/// [`RasterStatistics::glyphs_rendered`] is the figure a scale-bucketing or caching claim is made
/// against: it counts work the rasteriser actually did, so a claim that a zoom reused what it
/// already had is falsified by this number moving.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct RasterStatistics {
    /// Glyphs scan-converted to pixels.
    pub bitmaps: u64,
    /// Glyphs scaled into a path.
    pub outlines: u64,
    /// Glyphs that had nothing to draw.
    pub blanks: u64,
    /// Outlines answered from the outline cache rather than scaled again.
    pub outline_cache_hits: u64,
    /// Glyphs whose tables made the underlying parser panic, and which were refused rather than
    /// drawn. Non-zero only for a malformed face; the `read_a_glyph_table` boundary at the foot of
    /// this module says exactly which malformation.
    pub unreadable_glyphs: u64,
    /// How many faces are registered.
    pub faces: usize,
}

impl RasterStatistics {
    /// How many glyphs the rasteriser has actually done work for — bitmaps plus outlines, counting
    /// neither the blanks it decided cheaply nor the outlines it already held.
    #[must_use]
    pub const fn glyphs_rendered(&self) -> u64 {
        self.bitmaps + self.outlines
    }
}

/// One registered face: the bytes, and what `swash` needs to find its tables again.
#[derive(Debug)]
struct RegisteredFace {
    /// Held so that the address this face is filed under cannot be reused by another face while the
    /// registration stands — the same reason [`crate::ShapedRunCache`] holds one.
    face: Arc<FontFace>,
    /// Where in the file this face's table directory begins. Not the index: a collection's second
    /// face is at an offset the collection header names.
    table_directory_offset: u32,
    /// The identity `swash` caches its own parsed view of this face under. Taken from the `FontRef`
    /// `swash` itself built, because the counter behind it is what makes it unique; synthesising one
    /// would risk handing `swash` a parsed view of a different face.
    cache_key: swash::CacheKey,
}

/// An outline remembered so that a heading held still is not re-scaled on every frame.
#[derive(Debug)]
struct CachedOutline {
    outline: Arc<GlyphOutline>,
    last_used: u64,
}

/// Rasterises glyphs, and remembers which faces it has been shown.
///
/// One per thread. `swash`'s scaling context holds scratch buffers and a small parsed-face cache,
/// and sharing those across threads would put a lock on the hottest path in the renderer.
pub struct GlyphRasteriser {
    context: swash::scale::ScaleContext,
    faces: Vec<RegisteredFace>,
    by_address: HashMap<usize, FaceId>,
    outlines: HashMap<GlyphRasterKey, CachedOutline>,
    outline_capacity: usize,
    clock: u64,
    statistics: RasterStatistics,
}

impl fmt::Debug for GlyphRasteriser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `swash::scale::ScaleContext` is opaque and not `Debug`, and there is nothing in it a
        // reader of a log would act on. The counts below are what such a reader wants.
        formatter
            .debug_struct("GlyphRasteriser")
            .field("faces", &self.faces.len())
            .field("cached_outlines", &self.outlines.len())
            .field("statistics", &self.statistics)
            .finish_non_exhaustive()
    }
}

impl GlyphRasteriser {
    /// A rasteriser with no faces registered and the default outline-cache capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_outline_capacity(DEFAULT_OUTLINE_CACHE_CAPACITY)
    }

    /// A rasteriser whose outline cache holds at most `capacity` paths. Zero disables it.
    #[must_use]
    pub fn with_outline_capacity(capacity: usize) -> Self {
        Self {
            context: swash::scale::ScaleContext::new(),
            faces: Vec::new(),
            by_address: HashMap::new(),
            outlines: HashMap::new(),
            outline_capacity: capacity,
            clock: 0,
            statistics: RasterStatistics::default(),
        }
    }

    /// Give this rasteriser a face, and get back the identity every key for it will carry.
    ///
    /// Idempotent by **identity**, not by content: registering the same [`Arc`] twice returns the
    /// same [`FaceId`], and two faces built from equal bytes are two faces. That is the rule
    /// [`crate::ShapedRunCache`] already keys on, and for the same reasons — it is exact, it is a
    /// pointer comparison, and the strong reference held here is what stops the address it is filed
    /// under being reused underneath the registration.
    ///
    /// # Errors
    ///
    /// [`FontError::MalformedFace`] if `swash` will not read the bytes. They already parsed once, as
    /// a [`FontFace`] — but with a *different* parser, so this is a real possibility rather than a
    /// formality, and it is reported instead of asserted away.
    /// [`FontError::TooManyRegisteredFaces`] past four billion faces in one rasteriser.
    pub fn register(&mut self, face: &Arc<FontFace>) -> Result<FaceId, FontError> {
        let address = Arc::as_ptr(face) as usize;
        if let Some(existing) = self.by_address.get(&address) {
            return Ok(*existing);
        }

        let index = usize::try_from(face.index()).unwrap_or(usize::MAX);
        let Some(font) = swash::FontRef::from_index(face.data(), index) else {
            return Err(FontError::MalformedFace {
                source: ttf_parser::FaceParsingError::MalformedFont,
            });
        };

        let Ok(next) = u32::try_from(self.faces.len()) else {
            return Err(FontError::TooManyRegisteredFaces {
                registered: self.faces.len(),
            });
        };
        self.faces.push(RegisteredFace {
            face: Arc::clone(face),
            table_directory_offset: font.offset,
            cache_key: font.key,
        });
        let identity = FaceId(next);
        self.by_address.insert(address, identity);
        self.statistics.faces = self.faces.len();
        Ok(identity)
    }

    /// The face `identity` names, if it is registered here.
    #[must_use]
    pub fn face(&self, identity: FaceId) -> Option<&Arc<FontFace>> {
        self.faces
            .get(identity.0 as usize)
            .map(|registered| &registered.face)
    }

    /// What this rasteriser has been doing.
    #[must_use]
    pub fn statistics(&self) -> RasterStatistics {
        self.statistics
    }

    /// Forget every cached outline. The faces stay registered, because their identities are already
    /// inside keys the caller holds.
    pub fn clear_outline_cache(&mut self) {
        self.outlines.clear();
    }

    /// Throw away `swash`'s scratch buffers and its parsed-face cache.
    ///
    /// Called after the `read_a_glyph_table` boundary catches a panic. Nothing here is *memory*-unsafe — the
    /// panic came out of safe code in a safe crate — but the scratch buffers and the half-filled
    /// outline the panic left behind are in a state nothing has reasoned about, and reusing them
    /// would make the next glyph's image depend on the malformed one before it. Rebuilding costs an
    /// allocation, on a path that is only reached by a font that is already broken.
    fn discard_scaling_state(&mut self) {
        self.context = swash::scale::ScaleContext::new();
        self.statistics.unreadable_glyphs += 1;
    }

    /// Rasterise one glyph.
    ///
    /// Which route it takes is decided in this order, and the order is the point:
    ///
    /// 1. An empty bucket is [`GlyphRender::Blank`]; so is a glyph the face draws nothing for.
    /// 2. **A face that carries colour glyphs always rasterises**, at any size, because a colour
    ///    glyph has no single outline to hand a tessellator — `COLR` is a layer stack, and `sbix`
    ///    and `CBDT` are pictures. Which formats the face carries is read from
    ///    [`crate::FontFace::colour_formats`], which probed the tables once when the face was
    ///    parsed; this method does not re-open them. Its ordinary letters still come out as coverage,
    ///    because `swash` walks the source list until one can answer for the glyph in hand.
    /// 3. Above [`OUTLINE_PIXELS_PER_EM_THRESHOLD`], [`GlyphRender::Outline`].
    /// 4. Otherwise, a coverage bitmap.
    ///
    /// # Errors
    ///
    /// [`FontError::UnregisteredFace`] if the key names a face this rasteriser has never been shown.
    /// [`FontError::GlyphTooLargeToRasterise`] if a colour glyph is asked for above
    /// [`MAXIMUM_RASTERISED_PIXELS_PER_EM`], which is refused before anything allocates for it.
    /// [`FontError::UnreadableGlyphOutline`] if the face's glyph tables are malformed in a way the
    /// underlying parser does not survive; the `read_a_glyph_table` boundary at the foot of this
    /// module records exactly which way that is.
    pub fn render(&mut self, key: GlyphRasterKey) -> Result<GlyphRender, FontError> {
        let Some(registered) = self.faces.get(key.face.0 as usize) else {
            return Err(FontError::UnregisteredFace {
                face: key.face.as_u32(),
            });
        };

        if key.bucket.is_empty() {
            self.statistics.blanks += 1;
            return Ok(GlyphRender::Blank);
        }

        let pixels_per_em = key.bucket.pixels_per_em();
        let wants_colour = registered.face.colour_formats().any();

        if wants_colour && pixels_per_em > MAXIMUM_RASTERISED_PIXELS_PER_EM {
            return Err(FontError::GlyphTooLargeToRasterise {
                pixels_per_em,
                maximum: MAXIMUM_RASTERISED_PIXELS_PER_EM,
            });
        }

        if !wants_colour && key.bucket.is_above_the_outline_threshold() {
            return self.render_outline(key, pixels_per_em);
        }

        self.render_bitmap(key, pixels_per_em, wants_colour)
    }

    /// The outline route, through the outline cache.
    fn render_outline(
        &mut self,
        key: GlyphRasterKey,
        pixels_per_em: f32,
    ) -> Result<GlyphRender, FontError> {
        if self.outline_capacity > 0 {
            self.clock += 1;
            let clock = self.clock;
            if let Some(held) = self.outlines.get_mut(&key) {
                held.last_used = clock;
                self.statistics.outline_cache_hits += 1;
                return Ok(GlyphRender::Outline(Arc::clone(&held.outline)));
            }
        }

        let Some(registered) = self.faces.get(key.face.0 as usize) else {
            return Err(FontError::UnregisteredFace {
                face: key.face.as_u32(),
            });
        };
        let data = Arc::clone(registered.face.data());
        let offset = registered.table_directory_offset;
        let cache_key = registered.cache_key;

        let context = &mut self.context;
        // The outer `Option` is the guard's: `None` means the glyph table made the parser panic.
        // The inner one is the face's: `None` means the glyph simply has no outline.
        let scaled = read_a_glyph_table(|| {
            let font = swash::FontRef {
                data: &data,
                offset,
                key: cache_key,
            };
            let mut scaler = context
                .builder(font)
                .size(pixels_per_em)
                .hint(key.hinting == Hinting::GridFitted)
                .build();
            let scaled = scaler.scale_outline(swash::GlyphId::from(key.glyph.0))?;
            let mut commands = Vec::with_capacity(scaled.verbs().len());
            for command in swash::zeno::PathData::commands(&scaled.path()) {
                commands.push(convert_command(command));
            }
            Some(commands)
        });

        let commands = match scaled {
            Some(Some(commands)) => commands,
            Some(None) => {
                self.statistics.blanks += 1;
                return Ok(GlyphRender::Blank);
            }
            None => {
                self.discard_scaling_state();
                return Err(FontError::UnreadableGlyphOutline {
                    glyph: key.glyph.0,
                    pixels_per_em,
                });
            }
        };
        if commands.is_empty() {
            self.statistics.blanks += 1;
            return Ok(GlyphRender::Blank);
        }

        let outline = Arc::new(GlyphOutline { commands });
        self.statistics.outlines += 1;
        if self.outline_capacity > 0 {
            if self.outlines.len() >= self.outline_capacity {
                self.evict_oldest_outlines();
            }
            self.clock += 1;
            self.outlines.insert(
                key,
                CachedOutline {
                    outline: Arc::clone(&outline),
                    last_used: self.clock,
                },
            );
        }
        Ok(GlyphRender::Outline(outline))
    }

    /// Drop the least recently used half of the outline cache.
    ///
    /// The batched policy [`crate::ShapedRunCache`] already uses, for the same reason: one
    /// `O(n log n)` pass per `n/2` insertions rather than an `O(n)` scan per insertion, with no
    /// intrusive list and no `unsafe`.
    fn evict_oldest_outlines(&mut self) {
        let mut ages: Vec<u64> = self.outlines.values().map(|held| held.last_used).collect();
        if ages.is_empty() {
            return;
        }
        ages.sort_unstable();
        let cut = ages[ages.len() / 2];
        self.outlines.retain(|_, held| held.last_used > cut);
    }

    /// The bitmap route, coverage or colour.
    fn render_bitmap(
        &mut self,
        key: GlyphRasterKey,
        pixels_per_em: f32,
        wants_colour: bool,
    ) -> Result<GlyphRender, FontError> {
        let Some(registered) = self.faces.get(key.face.0 as usize) else {
            return Err(FontError::UnregisteredFace {
                face: key.face.as_u32(),
            });
        };
        let data = Arc::clone(registered.face.data());
        let table_directory_offset = registered.table_directory_offset;
        let cache_key = registered.cache_key;

        // The colour sources come first and the plain outline last, so a face carrying colour glyphs
        // still draws its ordinary letters as coverage. `PALETTE` is the face's first palette, which
        // is the one a face declares as its default; choosing between several is a document-level
        // decision nothing above this has yet had to make.
        const PALETTE: u16 = 0;
        let sources: &[swash::scale::Source] = if wants_colour {
            &[
                swash::scale::Source::ColorOutline(PALETTE),
                swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
                swash::scale::Source::Outline,
            ]
        } else {
            &[swash::scale::Source::Outline]
        };

        let context = &mut self.context;
        let rendered = read_a_glyph_table(|| {
            let font = swash::FontRef {
                data: &data,
                offset: table_directory_offset,
                key: cache_key,
            };
            let mut scaler = context
                .builder(font)
                .size(pixels_per_em)
                .hint(key.hinting == Hinting::GridFitted)
                .build();
            let phase = swash::zeno::Vector::new(key.subpixel.offset_in_pixels(), 0.0);
            swash::scale::Render::new(sources)
                .format(swash::zeno::Format::Alpha)
                .offset(phase)
                .render(&mut scaler, swash::GlyphId::from(key.glyph.0))
        });

        let image = match rendered {
            Some(Some(image)) => image,
            Some(None) => {
                self.statistics.blanks += 1;
                return Ok(GlyphRender::Blank);
            }
            None => {
                self.discard_scaling_state();
                return Err(FontError::UnreadableGlyphOutline {
                    glyph: key.glyph.0,
                    pixels_per_em,
                });
            }
        };

        let format = match image.content {
            swash::scale::image::Content::Mask => BitmapFormat::Coverage,
            swash::scale::image::Content::Color | swash::scale::image::Content::SubpixelMask => {
                BitmapFormat::Rgba
            }
        };

        let (Ok(width), Ok(height)) = (
            u16::try_from(image.placement.width),
            u16::try_from(image.placement.height),
        ) else {
            // A placement wider or taller than 65535 pixels cannot have come from a size this method
            // allows; refusing it is cheaper than trusting the arithmetic that produced it.
            return Err(FontError::GlyphTooLargeToRasterise {
                pixels_per_em,
                maximum: MAXIMUM_RASTERISED_PIXELS_PER_EM,
            });
        };

        let expected = usize::from(width) * usize::from(height) * format.bytes_per_pixel();
        if width == 0 || height == 0 || image.data.len() < expected {
            self.statistics.blanks += 1;
            return Ok(GlyphRender::Blank);
        }

        let mut pixels = image.data;
        pixels.truncate(expected);

        self.statistics.bitmaps += 1;
        Ok(GlyphRender::Bitmap(GlyphBitmap {
            format,
            width,
            height,
            offset_from_origin_x: clamp_to_i16(image.placement.left),
            // `swash` measures its placement from the origin *upward*; every surface below this
            // measures downward, so the sign is flipped exactly here and nowhere else.
            offset_from_origin_y: clamp_to_i16(-image.placement.top),
            pixels,
        }))
    }
}

impl Default for GlyphRasteriser {
    fn default() -> Self {
        Self::new()
    }
}

/// Read a glyph table through `swash`, and come back with `None` rather than unwinding if it
/// panics.
///
/// # Why this exists, exactly
///
/// `swash 0.2.10` reads fonts through `skrifa`, whose version range it pins as
/// `>= 0.31.1, <= 0.44`; `skrifa 0.44.0` pins `read-fonts 0.41.0`; and **`read-fonts 0.41.0` panics
/// on a malformed `glyf`**. `SimpleGlyph::num_points` computes the point count as the last entry of
/// `endPtsOfContours` plus one, in `u16`, so an entry of `0xFFFF` wraps it to zero — and
/// `read_points_fast` then writes `flags[0]` into a zero-length slice before it checks whether it is
/// done (`read-fonts-0.41.0/src/tables/glyf.rs:244`). One flipped byte in an embedded font reaches
/// it. `crates/mjx-text/tests/glyph_rasterisation.rs`'s corruption sweep found it, and it is what
/// that sweep exists for: the source grep in `untrusted_faces.rs` proves the absence of a *token*,
/// and this panic is inside a dependency, several crates below anything a grep of this crate reads.
///
/// **`read-fonts 0.43.3` fixes it** — `if n_points == 0 { return Ok(()); }`, at the top of the same
/// function — and the fix is out of reach: `0.43` is not semver-compatible with the `0.41` `skrifa`
/// asks for, and no `skrifa` inside `swash`'s range depends on it. Taking the fix means `swash`
/// relaxing its pin upstream, or vendoring, and that is a dependency decision this crate does not
/// get to make on its own. It is recorded on MJXOFF-159 for the repository's owner.
///
/// # What this does and does not guarantee
///
/// It converts an upstream panic into a [`FontError::UnreadableGlyphOutline`], which is what this
/// crate promises for every other kind of hostile font, and it is free when nothing panics. It is
/// **not** a proof: a consumer that builds with `panic = "abort"` gets an abort instead, because
/// there is no unwind to catch. This workspace deliberately keeps `panic = "unwind"` — the release
/// profile says so, for PyO3's sake — so the guard holds for everything built here.
///
/// `AssertUnwindSafe` is what makes the call compile, and the assertion is discharged by the caller:
/// `GlyphRasteriser::discard_scaling_state` throws away every piece of `swash` state the panic could
/// have left half-written, so nothing observes it.
fn read_a_glyph_table<T>(work: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).ok()
}

/// `swash` reports a placement in `i32`; a bitmap's offset is an `i16`, because a bitmap that far
/// from its origin is not a glyph. Saturating keeps the arithmetic total on a malformed face.
fn clamp_to_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// One `zeno` path command, converted into device pixels with y downward.
fn convert_command(command: swash::zeno::Command) -> OutlineCommand {
    let point = |p: swash::zeno::Point| OutlinePoint { x: p.x, y: -p.y };
    match command {
        swash::zeno::Command::MoveTo(to) => OutlineCommand::MoveTo(point(to)),
        swash::zeno::Command::LineTo(to) => OutlineCommand::LineTo(point(to)),
        swash::zeno::Command::QuadTo(control, to) => {
            OutlineCommand::QuadraticTo(point(control), point(to))
        }
        swash::zeno::Command::CurveTo(first, second, to) => {
            OutlineCommand::CubicTo(point(first), point(second), point(to))
        }
        swash::zeno::Command::Close => OutlineCommand::Close,
    }
}
