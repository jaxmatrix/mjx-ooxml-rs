//! The glyph atlas: where rasterised glyphs live, what they cost, and what changed since last frame.
//!
//! A page of dense text is tens of thousands of glyphs and a document is thousands of pages, drawn
//! from an alphabet of a few hundred shapes. The atlas is what turns the second number into the cost
//! instead of the first: each distinct [`GlyphRasterKey`] is rasterised once and packed into a
//! fixed-size page, and every later draw is a rectangle out of that page.
//!
//! # Three properties, and each is a design constraint rather than a nicety
//!
//! * **Bounded.** `docs/UI_PLATFORM_PLAN.md` §12 gives the whole GPU texture budget 256 MB on a
//!   desktop and 96 MB on a phone, and the glyph atlas is its largest single contributor. So the
//!   atlas has a declared byte ceiling, holds to it by evicting pages, and — because a cache that
//!   evicts everything satisfies every byte bound perfectly — **never evicts a page holding a glyph
//!   used in the current frame**. Both halves of that are asserted, in
//!   `tests/glyph_atlas_allocation.rs`.
//! * **Incremental.** A painter must not re-upload the alphabet every frame, so every insertion,
//!   page creation and page eviction is recorded and handed over as an [`AtlasDelta`] at the end of
//!   the frame. A frame that drew the same words as the last one uploads nothing at all.
//! * **Below the platform boundary.** The atlas produces **bytes**. Nothing here knows what a
//!   texture is, and uploading an [`AtlasUpload`] to one is R08's problem. That is what keeps a font
//!   engine out of the graphics stack and a graphics dependency out of the document graph.
//!
//! # Shelf packing, and why not something cleverer
//!
//! Glyphs from one run are all about the same height, and a shelf packer — rows of uniform height,
//! filled left to right — wastes almost nothing on input shaped like that while costing one
//! comparison per shelf to place. A skyline or MaxRects packer wins on input that is genuinely
//! arbitrary, which glyphs are not, and pays for it with a data structure that has to be maintained
//! as entries are evicted. Eviction here is by **whole page**, so nothing has to be maintained: a
//! page is either live or it is gone.
//!
//! There is a gutter of [`ATLAS_GUTTER_PIXELS`] between neighbours. Without one, a painter sampling
//! a glyph with bilinear filtering at a fractional destination reads a column of the letter next to
//! it, which shows up as a smear on the right-hand side of characters and nowhere else.

use std::collections::HashMap;

use crate::error::FontError;
use crate::placement::{PlacedGlyph, RunPlacement};
use crate::raster::{
    BitmapFormat, GlyphBitmap, GlyphOutline, GlyphRasterKey, GlyphRasteriser, GlyphRender,
    GlyphRoute, ScaleBucket,
};
use std::sync::Arc;

/// How many pixels wide and tall one atlas page is.
///
/// 512 rather than the 1024 or 2048 an atlas often uses, because the page is the unit eviction works
/// in and a page must therefore be small enough to evict without a visible stall. At 512 a coverage
/// page is a quarter of a megabyte and a colour page is one megabyte; at 2048 a colour page would be
/// sixteen megabytes, a sixth of the whole mobile texture budget in §12, and dropping one would take
/// most of a document's resident glyphs with it. It is still large enough that a 512-pixel page holds
/// well over a thousand body-sized glyphs.
pub const ATLAS_PAGE_SIZE_PIXELS: u16 = 512;

/// How many blank pixels separate one packed glyph from the next.
///
/// One is enough and one is necessary. A painter sampling a glyph with bilinear filtering at a
/// fractional destination position reads half a pixel outside the rectangle it asked for; with no
/// gutter that half pixel is the neighbouring letter.
pub const ATLAS_GUTTER_PIXELS: u16 = 1;

/// The glyph atlas's share of a desktop's GPU texture budget, in bytes.
///
/// `docs/UI_PLATFORM_PLAN.md` §12 budgets 256 MB for *all* GPU textures on a desktop. This is a
/// quarter of it: the atlas is the largest single contributor, and the remaining three quarters are
/// what decoded images, effect pools and offscreen render targets are drawn from. Sixty-four
/// megabytes is 256 coverage pages, which is far more glyph images than a document being read has
/// distinct glyphs at distinct scales.
pub const DESKTOP_GLYPH_ATLAS_BYTE_CEILING: usize = 64 * 1024 * 1024;

/// The same share of a phone's budget, from §12's 96 MB.
///
/// A quarter again, for the same division of the remainder. Twenty-four megabytes is ninety-six
/// coverage pages — fewer scales held at once than a desktop, which is right: a phone shows one
/// column of a document at one zoom rather than a spread at several.
pub const MOBILE_GLYPH_ATLAS_BYTE_CEILING: usize = 24 * 1024 * 1024;

/// Which page of the atlas something is on.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AtlasPageIndex(u32);

impl AtlasPageIndex {
    /// The index as a number, for a diagnostic message or a painter's own table.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Where one glyph's pixels sit in the atlas, and where they go relative to the pen.
///
/// The offsets carry through unchanged from [`GlyphBitmap`]: device pixels from the glyph's origin
/// to the top-left corner of these pixels, y downward.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtlasEntry {
    /// Which page.
    pub page: AtlasPageIndex,
    /// What the page's bytes mean.
    pub format: BitmapFormat,
    /// Pixels from the page's left edge.
    pub x: u16,
    /// Pixels from the page's top edge.
    pub y: u16,
    /// How many pixels wide.
    pub width: u16,
    /// How many pixels tall.
    pub height: u16,
    /// Pixels from the glyph's origin rightward to the left edge of these pixels.
    pub offset_from_origin_x: i16,
    /// Pixels from the glyph's origin downward to the top edge of these pixels; normally negative.
    pub offset_from_origin_y: i16,
}

impl AtlasEntry {
    /// How many bytes these pixels occupy.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        usize::from(self.width) * usize::from(self.height) * self.format.bytes_per_pixel()
    }
}

/// One rectangle of one page that changed and has to be sent to wherever the pixels really live.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AtlasUpload {
    /// Which page.
    pub page: AtlasPageIndex,
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
    pixels: Vec<u8>,
}

impl AtlasUpload {
    /// The pixels, row by row from the top, with no padding between rows.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// How many bytes this upload carries.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.pixels.len()
    }
}

/// A page the atlas started using.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtlasPageCreation {
    /// Which page.
    pub page: AtlasPageIndex,
    /// What its bytes mean, which decides how wide a texture holding it has to be.
    pub format: BitmapFormat,
    /// How many pixels wide and tall it is.
    pub size: u16,
}

/// What changed in the atlas since the last time a caller asked.
///
/// This is the whole point of the atlas being incremental. A frame that drew the same words as the
/// last one produces a delta with nothing in it, and a painter that honours it uploads nothing.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct AtlasDelta {
    created: Vec<AtlasPageCreation>,
    dropped: Vec<AtlasPageIndex>,
    uploads: Vec<AtlasUpload>,
}

impl AtlasDelta {
    /// Pages that did not exist before and now do.
    #[must_use]
    pub fn pages_created(&self) -> &[AtlasPageCreation] {
        &self.created
    }

    /// Pages that existed before and now do not. Their indices may be issued again later, so a
    /// painter must release the storage it held for one before honouring a creation that reuses it —
    /// which the ordering here guarantees, because a drop is recorded before the creation that
    /// reuses its slot.
    #[must_use]
    pub fn pages_dropped(&self) -> &[AtlasPageIndex] {
        &self.dropped
    }

    /// The rectangles that changed.
    #[must_use]
    pub fn uploads(&self) -> &[AtlasUpload] {
        &self.uploads
    }

    /// How many bytes of pixels this delta carries.
    #[must_use]
    pub fn uploaded_bytes(&self) -> usize {
        self.uploads.iter().map(AtlasUpload::byte_len).sum()
    }

    /// Whether nothing at all changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.created.is_empty() && self.dropped.is_empty() && self.uploads.is_empty()
    }
}

/// What the atlas has been doing, and what it currently costs.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct AtlasStatistics {
    /// How many glyph images are held.
    pub entries: usize,
    /// How many pages are live.
    pub pages: usize,
    /// What those pages cost, in bytes. The figure the ceiling is held against.
    pub resident_bytes: usize,
    /// The ceiling those bytes are held under.
    pub byte_ceiling: usize,
    /// Lookups answered from the atlas.
    pub hits: u64,
    /// Lookups that had to be rasterised.
    pub misses: u64,
    /// Glyph images lost to eviction.
    pub entries_evicted: u64,
    /// Pages dropped to stay under the ceiling.
    pub pages_evicted: u64,
    /// How many bytes have been handed out in deltas since the atlas was built.
    pub uploaded_bytes: u64,
}

/// One row of a page, filled left to right.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Shelf {
    top: u16,
    height: u16,
    next_x: u16,
}

/// One page: its pixels, its shelves, and when it was last drawn from.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Page {
    format: BitmapFormat,
    size: u16,
    shelves: Vec<Shelf>,
    open_top: u16,
    pixels: Vec<u8>,
    last_used_frame: u64,
    entries: usize,
}

impl Page {
    fn new(format: BitmapFormat, size: u16, frame: u64) -> Self {
        Self {
            format,
            size,
            shelves: Vec::new(),
            open_top: 0,
            pixels: vec![0; page_bytes(format, size)],
            last_used_frame: frame,
            entries: 0,
        }
    }

    /// Find room for a `width` by `height` rectangle, gutter included, and return its top-left
    /// corner.
    ///
    /// **Best fit, not first fit**: of the shelves the rectangle fits on, the shortest wins. First
    /// fit would put a body-sized glyph on the first shelf with room, which on a page that has
    /// already held a heading is a shelf several times too tall — every glyph after it then sits in
    /// a column of mostly empty page. Choosing the shortest shelf that fits costs one pass over a
    /// list that is a few dozen long at most, and it is the difference between a page holding its
    /// glyphs and a page holding a third of them.
    fn allocate(&mut self, width: u16, height: u16) -> Option<(u16, u16)> {
        let needed_width = width.checked_add(ATLAS_GUTTER_PIXELS)?;
        let needed_height = height.checked_add(ATLAS_GUTTER_PIXELS)?;
        if needed_width > self.size || needed_height > self.size {
            return None;
        }

        let mut best: Option<usize> = None;
        for (index, shelf) in self.shelves.iter().enumerate() {
            if needed_height > shelf.height || shelf.next_x + needed_width > self.size {
                continue;
            }
            let shorter = match best {
                Some(previous) => self
                    .shelves
                    .get(previous)
                    .is_some_and(|held| shelf.height < held.height),
                None => true,
            };
            if shorter {
                best = Some(index);
            }
        }

        if let Some(shelf) = best.and_then(|index| self.shelves.get_mut(index)) {
            let at = (shelf.next_x, shelf.top);
            shelf.next_x += needed_width;
            return Some(at);
        }

        if self.open_top.checked_add(needed_height)? > self.size {
            return None;
        }
        let top = self.open_top;
        self.open_top += needed_height;
        self.shelves.push(Shelf {
            top,
            height: needed_height,
            next_x: needed_width,
        });
        Some((0, top))
    }

    /// Copy `pixels` into the page at `(x, y)`.
    ///
    /// Bounds are re-checked here rather than trusted from the allocator: the pixels came out of a
    /// rasteriser reading an untrusted face, and a row that did not fit would otherwise be a panic
    /// on a slice index.
    fn write(&mut self, x: u16, y: u16, width: u16, height: u16, pixels: &[u8]) {
        let bytes_per_pixel = self.format.bytes_per_pixel();
        let stride = usize::from(self.size) * bytes_per_pixel;
        let row_bytes = usize::from(width) * bytes_per_pixel;
        for row in 0..usize::from(height) {
            let source_at = row * row_bytes;
            let target_at = (usize::from(y) + row) * stride + usize::from(x) * bytes_per_pixel;
            let (Some(source), Some(target)) = (
                pixels.get(source_at..source_at + row_bytes),
                self.pixels.get_mut(target_at..target_at + row_bytes),
            ) else {
                return;
            };
            target.copy_from_slice(source);
        }
    }
}

/// How many bytes a page of `size` in `format` occupies.
fn page_bytes(format: BitmapFormat, size: u16) -> usize {
    usize::from(size) * usize::from(size) * format.bytes_per_pixel()
}

/// One glyph image, and when it was last drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct HeldEntry {
    entry: AtlasEntry,
    last_used_frame: u64,
}

/// Rasterised glyphs, packed into pages, under a byte ceiling.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GlyphAtlas {
    page_size: u16,
    byte_ceiling: usize,
    /// Indexed by [`AtlasPageIndex`]; a `None` is a slot a dropped page left behind, which the next
    /// creation reuses so that the index space does not grow without bound.
    pages: Vec<Option<Page>>,
    entries: HashMap<GlyphRasterKey, HeldEntry>,
    frame: u64,
    delta: AtlasDelta,
    statistics: AtlasStatistics,
}

impl GlyphAtlas {
    /// An atlas with the default page size, under the desktop ceiling.
    #[must_use]
    pub fn new() -> Self {
        Self::with_byte_ceiling(DESKTOP_GLYPH_ATLAS_BYTE_CEILING)
    }

    /// An atlas with the default page size, under `byte_ceiling`.
    ///
    /// [`MOBILE_GLYPH_ATLAS_BYTE_CEILING`] is the other figure §12 names; a caller measuring a real
    /// device may pass anything.
    #[must_use]
    pub fn with_byte_ceiling(byte_ceiling: usize) -> Self {
        Self::with_configuration(ATLAS_PAGE_SIZE_PIXELS, byte_ceiling)
    }

    /// An atlas whose pages are `page_size` pixels square, under `byte_ceiling`.
    ///
    /// A page size of zero is raised to one, because a page has to be able to hold something for the
    /// atlas to make progress at all.
    #[must_use]
    pub fn with_configuration(page_size: u16, byte_ceiling: usize) -> Self {
        Self {
            page_size: page_size.max(1),
            byte_ceiling,
            pages: Vec::new(),
            entries: HashMap::new(),
            // Frames count from one so that a page's `last_used_frame` of zero means "never drawn
            // from", which is a state no live page is ever in.
            frame: 1,
            delta: AtlasDelta::default(),
            statistics: AtlasStatistics {
                byte_ceiling,
                ..AtlasStatistics::default()
            },
        }
    }

    /// Start a new frame.
    ///
    /// **This is what makes eviction safe**, and a caller that never calls it will find the atlas
    /// refusing to evict anything: a page is evictable only when nothing on it has been drawn from
    /// since the last frame began, so with no frame boundary every page is part of the working set
    /// forever.
    pub fn begin_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
    }

    /// Which frame the atlas is in.
    #[must_use]
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// What the atlas has been doing, and what it currently costs.
    #[must_use]
    pub fn statistics(&self) -> AtlasStatistics {
        AtlasStatistics {
            entries: self.entries.len(),
            pages: self.pages.iter().flatten().count(),
            resident_bytes: self.resident_bytes(),
            ..self.statistics
        }
    }

    /// What the live pages cost, in bytes. The figure the ceiling is held against.
    #[must_use]
    pub fn resident_bytes(&self) -> usize {
        self.pages
            .iter()
            .flatten()
            .map(|page| page.pixels.len())
            .sum()
    }

    /// The ceiling those bytes are held under.
    #[must_use]
    pub fn byte_ceiling(&self) -> usize {
        self.byte_ceiling
    }

    /// The pixels of one page, for a painter that has to re-upload a whole page after losing its
    /// storage. The ordinary path is [`GlyphAtlas::take_delta`], not this.
    #[must_use]
    pub fn page_pixels(&self, page: AtlasPageIndex) -> Option<&[u8]> {
        self.pages
            .get(page.0 as usize)
            .and_then(Option::as_ref)
            .map(|page| page.pixels.as_slice())
    }

    /// Where `key`'s pixels are, if the atlas holds them — and marks the page as part of this
    /// frame's working set, so it will not be evicted out from under the frame being drawn.
    pub fn get(&mut self, key: &GlyphRasterKey) -> Option<AtlasEntry> {
        let frame = self.frame;
        let held = self.entries.get_mut(key)?;
        held.last_used_frame = frame;
        let entry = held.entry;
        if let Some(Some(page)) = self.pages.get_mut(entry.page.0 as usize) {
            page.last_used_frame = frame;
        }
        self.statistics.hits += 1;
        Some(entry)
    }

    /// Take everything that has changed since the last time this was called.
    ///
    /// Called once per frame, after the frame's glyphs have been prepared and before they are drawn.
    pub fn take_delta(&mut self) -> AtlasDelta {
        let delta = std::mem::take(&mut self.delta);
        self.statistics.uploaded_bytes += delta.uploaded_bytes() as u64;
        delta
    }

    /// Pack `bitmap` into the atlas under `key`, evicting pages if that is what it takes to stay
    /// under the ceiling.
    ///
    /// Re-inserting a key the atlas already holds returns what it already has and copies nothing:
    /// the same key describes the same image by construction, so a second copy would be waste.
    ///
    /// # Errors
    ///
    /// [`FontError::GlyphTooLargeForAtlas`] if the bitmap cannot fit an empty page at all, which no
    /// size this crate rasterises at produces but a caller inserting its own bitmap could ask for.
    /// [`FontError::GlyphAtlasExhausted`] if a new page is needed and every live page holds a glyph
    /// this frame has already drawn — the atlas will not evict the frame it is in the middle of, so
    /// it says so instead.
    pub fn insert(
        &mut self,
        key: GlyphRasterKey,
        bitmap: &GlyphBitmap,
    ) -> Result<AtlasEntry, FontError> {
        if let Some(existing) = self.get(&key) {
            return Ok(existing);
        }

        let width = bitmap.width();
        let height = bitmap.height();
        let format = bitmap.format();
        if width.saturating_add(ATLAS_GUTTER_PIXELS) > self.page_size
            || height.saturating_add(ATLAS_GUTTER_PIXELS) > self.page_size
        {
            return Err(FontError::GlyphTooLargeForAtlas {
                width,
                height,
                page_size: self.page_size,
            });
        }

        let placed = match self.allocate_in_a_live_page(format, width, height) {
            Some(placed) => placed,
            None => {
                let index = self.open_a_page(format)?;
                let resident = self.resident_bytes();
                let ceiling = self.byte_ceiling;
                let page_size = self.page_size;
                let Some(Some(page)) = self.pages.get_mut(index.0 as usize) else {
                    // The slot was written immediately above; reporting rather than asserting is
                    // this crate's rule on every path a font's bytes reach.
                    return Err(FontError::GlyphAtlasExhausted {
                        resident_bytes: resident,
                        ceiling_bytes: ceiling,
                    });
                };
                let Some(at) = page.allocate(width, height) else {
                    return Err(FontError::GlyphTooLargeForAtlas {
                        width,
                        height,
                        page_size,
                    });
                };
                (index, at.0, at.1)
            }
        };

        let (index, x, y) = placed;
        let frame = self.frame;
        let resident = self.resident_bytes();
        let ceiling = self.byte_ceiling;
        let Some(Some(page)) = self.pages.get_mut(index.0 as usize) else {
            return Err(FontError::GlyphAtlasExhausted {
                resident_bytes: resident,
                ceiling_bytes: ceiling,
            });
        };
        page.write(x, y, width, height, bitmap.pixels());
        page.last_used_frame = frame;
        page.entries += 1;

        let entry = AtlasEntry {
            page: index,
            format,
            x,
            y,
            width,
            height,
            offset_from_origin_x: bitmap.offset_from_origin_x(),
            offset_from_origin_y: bitmap.offset_from_origin_y(),
        };
        self.entries.insert(
            key,
            HeldEntry {
                entry,
                last_used_frame: frame,
            },
        );
        self.delta.uploads.push(AtlasUpload {
            page: index,
            format,
            x,
            y,
            width,
            height,
            pixels: bitmap.pixels().to_vec(),
        });
        self.statistics.misses += 1;
        Ok(entry)
    }

    /// Make sure every glyph of `placement` has an image, rasterising the ones the atlas does not
    /// already hold.
    ///
    /// This is the loop a painter runs once per run per frame, and the one place the two routes out
    /// of [`GlyphRasteriser::render`] are joined back together: a small glyph becomes an
    /// [`AtlasEntry`], a large one stays a [`GlyphOutline`] for R07, and either way the caller reads
    /// [`PreparedGlyph::route`] rather than guessing from the size.
    ///
    /// # Errors
    ///
    /// Whatever [`GlyphRasteriser::render`] or [`GlyphAtlas::insert`] returned.
    pub fn prepare_run(
        &mut self,
        rasteriser: &mut GlyphRasteriser,
        placement: &RunPlacement,
    ) -> Result<PreparedRun, FontError> {
        let mut glyphs = Vec::with_capacity(placement.len());
        for placed in placement.glyphs() {
            let image = match self.get(&placed.key) {
                Some(entry) => PreparedImage::Atlas(entry),
                None => match rasteriser.render(placed.key)? {
                    GlyphRender::Bitmap(bitmap) => {
                        PreparedImage::Atlas(self.insert(placed.key, &bitmap)?)
                    }
                    GlyphRender::Outline(outline) => PreparedImage::Outline(outline),
                    GlyphRender::Blank => PreparedImage::Blank,
                },
            };
            glyphs.push(PreparedGlyph {
                placed: *placed,
                image,
            });
        }
        Ok(PreparedRun {
            bucket: placement.bucket(),
            residual_scale: placement.residual_scale(),
            glyphs,
        })
    }

    /// Try to fit a rectangle into a page that already exists.
    fn allocate_in_a_live_page(
        &mut self,
        format: BitmapFormat,
        width: u16,
        height: u16,
    ) -> Option<(AtlasPageIndex, u16, u16)> {
        for (index, slot) in self.pages.iter_mut().enumerate() {
            let Some(page) = slot else { continue };
            if page.format != format {
                continue;
            }
            if let Some((x, y)) = page.allocate(width, height) {
                let Ok(index) = u32::try_from(index) else {
                    continue;
                };
                return Some((AtlasPageIndex(index), x, y));
            }
        }
        None
    }

    /// Make room for, and then create, one more page.
    fn open_a_page(&mut self, format: BitmapFormat) -> Result<AtlasPageIndex, FontError> {
        let wanted = page_bytes(format, self.page_size);
        if wanted > self.byte_ceiling {
            return Err(FontError::GlyphAtlasExhausted {
                resident_bytes: self.resident_bytes(),
                ceiling_bytes: self.byte_ceiling,
            });
        }
        while self.resident_bytes() + wanted > self.byte_ceiling {
            if !self.evict_one_page() {
                return Err(FontError::GlyphAtlasExhausted {
                    resident_bytes: self.resident_bytes(),
                    ceiling_bytes: self.byte_ceiling,
                });
            }
        }

        let page = Page::new(format, self.page_size, self.frame);
        let index = match self.pages.iter().position(Option::is_none) {
            Some(hole) => {
                self.pages[hole] = Some(page);
                hole
            }
            None => {
                self.pages.push(Some(page));
                self.pages.len() - 1
            }
        };
        let Ok(index) = u32::try_from(index) else {
            return Err(FontError::GlyphAtlasExhausted {
                resident_bytes: self.resident_bytes(),
                ceiling_bytes: self.byte_ceiling,
            });
        };
        let index = AtlasPageIndex(index);
        self.delta.created.push(AtlasPageCreation {
            page: index,
            format,
            size: self.page_size,
        });
        Ok(index)
    }

    /// Drop the page that has gone longest without being drawn from, and every entry on it.
    ///
    /// Returns `false` when there is nothing that may be dropped, which happens exactly when every
    /// live page holds a glyph the current frame has already drawn. **That is the whole safety
    /// property of this cache**: a working set is never evicted to satisfy the ceiling, so a
    /// measurement that finds the ceiling held is not a measurement of an atlas that threw
    /// everything away.
    fn evict_one_page(&mut self) -> bool {
        let frame = self.frame;
        let mut oldest: Option<(usize, u64)> = None;
        for (index, slot) in self.pages.iter().enumerate() {
            let Some(page) = slot else { continue };
            if page.last_used_frame >= frame {
                continue;
            }
            match oldest {
                Some((_, age)) if age <= page.last_used_frame => {}
                _ => oldest = Some((index, page.last_used_frame)),
            }
        }

        let Some((index, _)) = oldest else {
            return false;
        };
        let Some(slot) = self.pages.get_mut(index) else {
            return false;
        };
        *slot = None;

        let Ok(dropped) = u32::try_from(index) else {
            return false;
        };
        let dropped = AtlasPageIndex(dropped);
        let before = self.entries.len();
        self.entries.retain(|_, held| held.entry.page != dropped);
        self.statistics.entries_evicted += (before - self.entries.len()) as u64;
        self.statistics.pages_evicted += 1;
        // Recorded before any creation that reuses the slot, which is what lets a painter replay a
        // delta in order and never hold two things under one index.
        self.delta.dropped.push(dropped);
        // Anything queued for a page that no longer exists would be an upload into nothing.
        self.delta.uploads.retain(|upload| upload.page != dropped);
        true
    }
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::new()
    }
}

/// Where one prepared glyph's image is.
#[derive(Clone, PartialEq, Debug)]
pub enum PreparedImage {
    /// In the atlas, at this rectangle.
    Atlas(AtlasEntry),
    /// Not in the atlas at all: too large to be worth a bitmap, so it is a path for R07.
    Outline(Arc<GlyphOutline>),
    /// Nothing to draw.
    Blank,
}

/// One glyph of a run, placed and given an image.
#[derive(Clone, PartialEq, Debug)]
pub struct PreparedGlyph {
    /// Where it goes.
    pub placed: PlacedGlyph,
    /// What to draw there.
    pub image: PreparedImage,
}

impl PreparedGlyph {
    /// Which route this glyph took out of the rasteriser.
    #[must_use]
    pub fn route(&self) -> GlyphRoute {
        match self.image {
            PreparedImage::Atlas(_) => GlyphRoute::Bitmap,
            PreparedImage::Outline(_) => GlyphRoute::Outline,
            PreparedImage::Blank => GlyphRoute::Blank,
        }
    }
}

/// A run, placed and with every glyph given an image — everything a painter needs and nothing it
/// does not.
#[derive(Clone, PartialEq, Debug)]
pub struct PreparedRun {
    bucket: ScaleBucket,
    residual_scale: f32,
    glyphs: Vec<PreparedGlyph>,
}

impl PreparedRun {
    /// The glyphs, in draw order.
    #[must_use]
    pub fn glyphs(&self) -> &[PreparedGlyph] {
        &self.glyphs
    }

    /// How many glyphs the run has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    /// Whether the run has no glyphs at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// The quantised scale every position and image here is expressed at.
    #[must_use]
    pub fn bucket(&self) -> ScaleBucket {
        self.bucket
    }

    /// What a painter multiplies the whole run by to draw it at the size that was asked for.
    #[must_use]
    pub fn residual_scale(&self) -> f32 {
        self.residual_scale
    }

    /// How many glyphs took each route, as `(bitmaps, outlines, blanks)`.
    #[must_use]
    pub fn route_counts(&self) -> (usize, usize, usize) {
        let mut counts = (0, 0, 0);
        for glyph in &self.glyphs {
            match glyph.route() {
                GlyphRoute::Bitmap => counts.0 += 1,
                GlyphRoute::Outline => counts.1 += 1,
                GlyphRoute::Blank => counts.2 += 1,
            }
        }
        counts
    }
}
