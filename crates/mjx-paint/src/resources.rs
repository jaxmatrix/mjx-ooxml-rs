//! What a display list does **not** carry, and a painter therefore has to be handed: glyph pixels,
//! picture pixels, and the outlines a geometry provider resolves.
//!
//! # Why the glyph atlas comes in through a trait this crate owns
//!
//! `mjx-scene`'s whole reason for existing is that *below a display list, nothing has heard of a
//! font, a layout algorithm or a document*, and this crate's dependency gate
//! (`tests/the_seam_holds.rs`) is what holds the other half of that: **`mjx-paint`'s source may not
//! name `mjx-layout`, `mjx-dml`, a format crate or the facade.** At rank 5.5 no layering rule can
//! do that job — every crate in the workspace is a legal dependency of a crate above the facade —
//! so the gate is the only thing standing.
//!
//! MJXOFF-163 also requires the painter to upload **only what changed** in the glyph atlas, from
//! R04's own delta report. Those two requirements meet head-on: a display list deliberately carries
//! **no pixels**, an [`mjx_scene::AtlasPlacement`] is a *record of the frame the list was built for*
//! rather than a live answer, and the delta lives on `mjx_text::GlyphAtlas`. There is no way to
//! reach it through `mjx-scene`, and dropping either clause would mean either re-uploading the
//! whole atlas every frame or a gate with a hole in it.
//!
//! **The decision, stated rather than omitted:** the atlas is taken through [`AtlasSource`], a
//! contract in *this crate's* vocabulary, and the adapter from `mjx_text::GlyphAtlas` to it lives in
//! exactly one file, [`crate::glyph_atlas`]. The gate forbids `mjx-text` in every other file of
//! `src/` **and asserts that it does appear in that one**, so the exemption cannot quietly become
//! dead code or quietly spread. Nothing in the painter itself — not the frame plan, not a pipeline,
//! not a shader — can name a font engine, and a shell that has its own atlas implements one trait
//! rather than forking a painter.
//!
//! # Why the visitor, rather than a returned delta
//!
//! [`AtlasSource::take_changes`] hands each changed rectangle to a visitor instead of returning a
//! collection of them. A returned collection would either borrow the source (and so could not be
//! `take`n, which is what makes a delta a delta) or own copies of every pixel — a copy of the whole
//! frame's new glyph coverage, made only to be copied again into a staging buffer. The visitor
//! borrows, and the source is free to drop its delta the moment the call returns.

use mjx_scene::{BitmapFormat, GeometryProvider, PathCommand};

use crate::error::PaintError;

/// A page of a glyph atlas, as a painter has to allocate storage for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtlasPage {
    /// Which page. Indices are reused after a page is released, which is why
    /// [`AtlasVisitor::page_released`] is always delivered before the creation that reuses a slot.
    pub page: u32,
    /// What its bytes mean, which decides the texture format storage has to be allocated in.
    pub format: BitmapFormat,
    /// How many pixels wide and tall it is. Atlas pages are square.
    pub size: u16,
}

impl AtlasPage {
    /// How many bytes a page of this format and size holds.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        let side = usize::from(self.size);
        side * side * bytes_per_pixel(self.format)
    }
}

/// One rectangle of one page whose pixels changed.
///
/// Borrowed, not owned: it is valid only for the duration of the [`AtlasVisitor::write`] call it is
/// handed to, which is what lets a source hand over a delta without copying it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtlasWrite<'a> {
    /// Which page.
    pub page: u32,
    /// What the bytes mean.
    pub format: BitmapFormat,
    /// Pixels from the page's left edge.
    pub x: u16,
    /// Pixels from the page's top edge.
    pub y: u16,
    /// How many pixels wide.
    pub width: u16,
    /// How many pixels tall.
    pub height: u16,
    /// The pixels, row by row from the top, with no padding between rows.
    pub pixels: &'a [u8],
}

/// How many bytes one pixel of `format` takes.
///
/// A `match` rather than a call into the font engine, because this file is on the painter's side of
/// the seam and the enumeration reaches it through `mjx-scene`'s re-export.
#[must_use]
pub fn bytes_per_pixel(format: BitmapFormat) -> usize {
    match format {
        BitmapFormat::Coverage => 1,
        BitmapFormat::Rgba => 4,
    }
}

/// What a painter does with each change a glyph atlas reports.
///
/// Implemented by the painter's own texture bookkeeping. The order the three methods arrive in is
/// part of the contract: **every release, then every creation, then every write** — a page index is
/// reused after the page it named is dropped, so honouring a creation before the release that freed
/// its slot would overwrite live storage with a new page's.
pub trait AtlasVisitor {
    /// A page that existed and no longer does. Its storage may be freed and its index reused.
    ///
    /// # Errors
    ///
    /// Whatever the painter's storage says.
    fn page_released(&mut self, page: u32) -> Result<(), PaintError>;

    /// A page that did not exist and now does. Storage of `page.byte_len()` bytes is needed for it.
    ///
    /// # Errors
    ///
    /// [`PaintError::TextureUnavailable`] if the storage cannot be allocated.
    fn page_created(&mut self, page: AtlasPage) -> Result<(), PaintError>;

    /// A rectangle of a page whose pixels changed.
    ///
    /// # Errors
    ///
    /// Whatever the painter's storage says.
    fn write(&mut self, write: AtlasWrite<'_>) -> Result<(), PaintError>;
}

/// A glyph atlas, as a painter sees one: a thing that can say what changed since it was last asked.
///
/// **This is deliberately not "an atlas".** A painter never looks a glyph up — the display list
/// already recorded where every glyph's pixels are, in an [`mjx_scene::AtlasPlacement`] — so the only
/// thing it needs from the live atlas is the delta, and asking for exactly that is what keeps the
/// contract to one method.
pub trait AtlasSource {
    /// Take everything that has changed since the last call, and hand it to `visitor`.
    ///
    /// Taking, not peeking: a change reported once is not reported again, which is the whole of
    /// "upload only what changed". Calling this twice in a frame therefore reports nothing the
    /// second time, and a painter that lost its textures must recreate them from the source rather
    /// than from a second delta.
    ///
    /// # Errors
    ///
    /// Whatever `visitor` fails with.
    fn take_changes(&mut self, visitor: &mut dyn AtlasVisitor) -> Result<(), PaintError>;
}

/// The decoded pixels of one picture, as the caller holds them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ImagePixels<'a> {
    /// How many pixels wide.
    pub width: u32,
    /// How many pixels tall.
    pub height: u32,
    /// Non-premultiplied `RGBA`, row by row from the top, four bytes to the pixel and no padding.
    pub rgba: &'a [u8],
}

/// Where a [`mjx_scene::Image`]'s pixels come from.
///
/// A display list addresses a picture by an opaque `u64` handle — `mjx-scene` never decodes a JPEG
/// — so the caller that built the list is the one that can answer this. A caller with no picture
/// for a handle answers `None`, and the painter draws the picture's area as nothing rather than
/// failing the frame: a missing image is a document problem, not a rendering one.
pub trait ImageSource {
    /// The pixels behind `handle`, or `None` if the caller does not have them.
    fn pixels(&self, handle: u64) -> Option<ImagePixels<'_>>;
}

/// An image source with nothing in it.
///
/// What a page with no pictures is drawn against, and what makes `Resources` constructible in a
/// test that is about something else.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct NoImages;

impl ImageSource for NoImages {
    fn pixels(&self, _handle: u64) -> Option<ImagePixels<'_>> {
        None
    }
}

/// A face, cut down to the glyphs a document used, with the numbers a document format needs.
///
/// # Why an exporter needs this and a rasteriser does not
///
/// A rasteriser is handed **pixels**: a display list records where each glyph's image is in the
/// atlas, and the atlas hands over its delta. A *vector exporter* has neither — a PDF embeds a font
/// file and addresses glyphs by id, and an SVG draws their outlines — so it needs the face itself,
/// and the metrics a `/FontDescriptor` is made of.
///
/// The bytes are **owned** because a subset is computed rather than borrowed. See
/// [`mjx_text::subset_truetype`] for what is cut and what is kept, and why a face that cannot be cut
/// comes back whole rather than empty.
#[derive(Clone, PartialEq, Debug)]
pub struct EmbeddableFace {
    /// The font file to embed.
    pub data: Vec<u8>,
    /// What to call it in the document.
    pub name: String,
    /// The em square every number below is measured in.
    pub units_per_em: u16,
    /// How far above the baseline the face reaches.
    pub ascender: i16,
    /// How far below, negative.
    pub descender: i16,
    /// The face's global bounding box: left, bottom, right, top.
    pub bounding_box: [i16; 4],
    /// How far the face leans, in degrees, negative for a forward lean.
    pub italic_angle: f32,
    /// The height of a capital letter.
    pub cap_height: i16,
    /// Whether every glyph has the same advance.
    pub fixed_pitch: bool,
    /// Whether the face leans.
    pub italic: bool,
    /// Whether the whole face was embedded because it could not be cut.
    ///
    /// Reported rather than hidden: an exporter may want to say so, and a suite asserting that a
    /// subset is smaller than the face has to know when the answer is legitimately "it is the face".
    pub whole_face: bool,
}

/// Where a *document* exporter's faces come from.
///
/// # Why this is a second contract beside [`AtlasSource`], rather than the same one
///
/// They answer different questions. An [`AtlasSource`] answers *"what glyph pixels changed since
/// you last asked"*, which is what a rasteriser needs and is a per-frame delta. This answers
/// *"which face is run 3 in, what does glyph 42 of it look like, and what would I have to embed to
/// draw it elsewhere"*, which is a property of the document rather than of the frame.
///
/// Both live in **this crate's** vocabulary for the same reason: `crates/mjx-paint/tests/the_seam_holds.rs`
/// confines the font engine to the files that adapt it, and a painter that named
/// `mjx_text::FontFace` in its exporter would have crossed the seam in a second place. The adapter
/// is [`crate::font_source::FaceLibrary`], and the gate names the file it lives in.
pub trait FontSource {
    /// The face behind a run's face number, cut down to `glyphs`.
    ///
    /// `None` for a face the caller does not have — an exporter then writes the run's metadata and
    /// no text, rather than failing the page or drawing a picture of it.
    fn face(&self, face: u32, glyphs: &[u16]) -> Option<EmbeddableFace>;

    /// One glyph's advance, in the face's own units.
    fn advance(&self, face: u32, glyph: u16) -> Option<u16>;

    /// A character the glyph stands for, for a `/ToUnicode` map.
    ///
    /// The face's `cmap`, read backwards. Several characters legitimately map to one glyph, so this
    /// answers **one** of them — the lowest, so that two runs of the same text produce the same
    /// map and an export can be compared against itself.
    fn character(&self, face: u32, glyph: u16) -> Option<char>;

    /// One glyph's outline at a size, in device pixels with **y increasing downward** from the
    /// glyph's own origin — the convention every other coordinate in this platform uses.
    fn outline(&self, face: u32, glyph: u16, pixels_per_em: f32) -> Option<Vec<PathCommand>>;
}

/// A font source with nothing in it.
///
/// What [`Resources`] carries unless a caller supplies one, and what a rasteriser uses: a painter
/// that draws atlas rectangles never asks any of the four questions above. An exporter handed this
/// writes a page with the shapes on it and the text's metadata recorded but no glyphs drawn, which
/// is visible in its output rather than silent.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct NoFonts;

impl FontSource for NoFonts {
    fn face(&self, _face: u32, _glyphs: &[u16]) -> Option<EmbeddableFace> {
        None
    }

    fn advance(&self, _face: u32, _glyph: u16) -> Option<u16> {
        None
    }

    fn character(&self, _face: u32, _glyph: u16) -> Option<char> {
        None
    }

    fn outline(&self, _face: u32, _glyph: u16, _pixels_per_em: f32) -> Option<Vec<PathCommand>> {
        None
    }
}

/// The empty font source, as something a borrow can point at.
static NO_FONTS: NoFonts = NoFonts;

/// An atlas source that reports nothing ever changed.
///
/// For a page with no text, and for a painter driven against a display list whose glyph pixels are
/// already resident.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct NoGlyphs;

impl AtlasSource for NoGlyphs {
    fn take_changes(&mut self, _visitor: &mut dyn AtlasVisitor) -> Result<(), PaintError> {
        Ok(())
    }
}

/// Everything a painter needs that is not in the display list.
///
/// The third argument of [`crate::Painter::draw`], and the reason that argument is not three
/// arguments: the set grows — a video frame source, a live chart — and every added member would be
/// a change to four painters' signatures.
pub struct Resources<'a> {
    glyphs: &'a mut dyn AtlasSource,
    geometry: &'a dyn GeometryProvider,
    images: &'a dyn ImageSource,
    fonts: &'a dyn FontSource,
}

impl<'a> Resources<'a> {
    /// The three sources a frame is **drawn** from.
    ///
    /// Faces are not among them, and that is not an omission: a rasteriser is handed glyph pixels
    /// and never opens a font. An exporter adds them with [`Resources::with_fonts`], and one that
    /// was not given any writes a page with its text's metadata recorded and no glyphs drawn.
    pub fn new(
        glyphs: &'a mut dyn AtlasSource,
        geometry: &'a dyn GeometryProvider,
        images: &'a dyn ImageSource,
    ) -> Self {
        Self {
            glyphs,
            geometry,
            images,
            fonts: &NO_FONTS,
        }
    }

    /// The same, with faces an exporter can embed.
    #[must_use]
    pub fn with_fonts(mut self, fonts: &'a dyn FontSource) -> Self {
        self.fonts = fonts;
        self
    }

    /// Where faces come from.
    #[must_use]
    pub fn fonts(&self) -> &dyn FontSource {
        self.fonts
    }

    /// The glyph atlas, mutably, because taking a delta changes it.
    pub fn glyphs(&mut self) -> &mut dyn AtlasSource {
        self.glyphs
    }

    /// The geometry provider that resolves a [`mjx_scene::Geometry::Unresolved`].
    #[must_use]
    pub fn geometry(&self) -> &dyn GeometryProvider {
        self.geometry
    }

    /// Where pictures come from.
    #[must_use]
    pub fn images(&self) -> &dyn ImageSource {
        self.images
    }
}

impl core::fmt::Debug for Resources<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // None of the three members is `Debug`, and requiring it of them would put a bound on every
        // shell's own atlas for the sake of a log line.
        formatter.write_str("Resources { glyphs, geometry, images, fonts }")
    }
}
