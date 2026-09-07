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

use mjx_scene::{BitmapFormat, GeometryProvider};

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
}

impl<'a> Resources<'a> {
    /// The three sources a frame is drawn from.
    pub fn new(
        glyphs: &'a mut dyn AtlasSource,
        geometry: &'a dyn GeometryProvider,
        images: &'a dyn ImageSource,
    ) -> Self {
        Self {
            glyphs,
            geometry,
            images,
        }
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
        formatter.write_str("Resources { glyphs, geometry, images }")
    }
}
