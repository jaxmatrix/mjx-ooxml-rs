//! A run of glyphs, placed — where the resolution-dependent half of text finally happens.
//!
//! # This is the step R05 deliberately did not take
//!
//! [`mjx_layout::GlyphRunFragment`] carries a [`ShapedRun`](mjx_text::ShapedRun) and a baseline
//! origin in EMU, **not positions**, and that is the design decision the box model contract is built
//! on: `mjx-text`'s [`place_run`](mjx_text::place_run) puts every glyph on a whole pixel at a
//! quantised scale, and which pixel that is depends on the zoom. A fragment tree that carried pixels
//! would be rebuilt on every zoom step, which would throw away the checkpoint machinery.
//!
//! So the call happens here, because **this is the first layer that knows the device scale**. A
//! `FragmentTree` is built once and survives a zoom; a `DisplayList` is built per scale and is the
//! thing that is rebuilt.
//!
//! # Every glyph carries its image, and none of them carry pixels
//!
//! A glyph is a rectangle of an atlas page, or a path, or nothing. `mjx-text`'s
//! [`GlyphAtlas::prepare_run`](mjx_text::GlyphAtlas::prepare_run) is what decides which — small
//! glyphs are rasterised into the atlas, glyphs above
//! [`OUTLINE_PIXELS_PER_EM_THRESHOLD`](mjx_text::OUTLINE_PIXELS_PER_EM_THRESHOLD) stay outlines for
//! R07 to tessellate — and a scene records the answer rather than guessing it back from the size.
//!
//! An outline glyph becomes an ordinary [`crate::Geometry::Path`] in the geometry table, which is
//! the whole of its handling: R07 tessellates it exactly as it tessellates a shape, and no painter
//! needs a second path pipeline for text.

use mjx_text::{BitmapFormat, Hinting, TextDirection};

use crate::encoding::ResourceIndex;
use crate::geometry::ScenePoint;

/// Where a glyph's pixels are in the atlas.
///
/// The same eight numbers [`mjx_text::AtlasEntry`] carries, and deliberately a copy rather than a
/// re-export: an `AtlasEntry` is a *live* answer from an atlas that evicts pages, and a display list
/// is a **record of a frame** that may be cached to disk and read back after that page is gone. A
/// painter checks the atlas's own delta to know what is still resident; what is stored here is what
/// the frame was built against.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtlasPlacement {
    /// Which atlas page.
    pub page: u32,
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
    /// Pixels from the glyph's origin downward to the top edge; normally negative.
    pub offset_from_origin_y: i16,
}

/// What is drawn for one glyph.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlyphImage {
    /// A rectangle of an atlas page.
    Atlas(AtlasPlacement),
    /// A path, at this index of the geometry table — a glyph too large for a bitmap to be worth it.
    Outline(ResourceIndex),
    /// Nothing at all: a space, or a glyph the face draws no ink for.
    Blank,
}

/// One glyph of a run, placed and given an image.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SceneGlyph {
    /// The whole pixel the glyph's origin sits at, horizontally, relative to the run's own origin
    /// and at the bucket's scale.
    pub x: i32,
    /// The pixel row it sits on, in the same terms.
    pub y: i32,
    /// The byte offset, within the run's text, of the first character this glyph belongs to.
    ///
    /// Carried through from [`mjx_text::ShapedGlyph::cluster`] unchanged, because hit-testing a
    /// pixel back to a character is what it is for, and a display list that dropped it would make
    /// the caret un-placeable from the only structure a painter holds.
    pub cluster: u32,
    /// Which glyph in the face.
    pub glyph: u16,
    /// Which of the four horizontal phases the glyph's image was rasterised for, so that `x` plus
    /// this phase is where the glyph really is.
    pub subpixel: u8,
    /// What to draw there.
    pub image: GlyphImage,
}

/// A run of shaped glyphs, placed at one origin in one face at one size.
#[derive(Clone, PartialEq, Debug)]
pub struct SceneGlyphRun {
    /// Which face, as the rasteriser that will draw these numbered it.
    ///
    /// **Per `Arc`, not per file.** Parsing one font twice yields two [`mjx_text::FaceId`]s, and a
    /// scene built from two trees that each parsed the same file would carry the same glyphs twice.
    /// Holding one `Arc` per face and passing it down is the caller's job and cannot be done here.
    pub face: u32,
    /// The quantised scale every position and image in this run is expressed at, as a count of
    /// [`mjx_text::SCALE_BUCKET_STEP_PIXELS_PER_EM`] steps.
    pub bucket_steps: u32,
    /// What a painter multiplies the whole run by — positions and images together — to draw it at
    /// the size that was actually asked for.
    pub residual_scale: f32,
    /// Where the pen starts: on the baseline, at the run's leading edge in visual order, in device
    /// pixels from the page's top-left corner.
    pub origin: ScenePoint,
    /// Which way the run is written.
    pub direction: TextDirection,
    /// Which hinting the glyphs were rasterised with.
    pub hinting: Hinting,
    /// The UAX #9 level the run was resolved at, which a selection needs in order to know that a
    /// visually contiguous highlight may be two logical ranges.
    pub level: u8,
    /// The glyphs, in draw order.
    pub glyphs: Vec<SceneGlyph>,
}

/// The wire value of a text direction.
#[must_use]
pub(crate) fn direction_wire_value(direction: TextDirection) -> u32 {
    match direction {
        TextDirection::LeftToRight => 0,
        TextDirection::RightToLeft => 1,
    }
}

/// The direction a wire value names; anything else reads left to right, which is the direction a run
/// with no information about itself is drawn in.
#[must_use]
pub(crate) fn direction_from_wire_value(value: u32) -> TextDirection {
    match value {
        1 => TextDirection::RightToLeft,
        _ => TextDirection::LeftToRight,
    }
}

/// The wire value of a hinting mode.
#[must_use]
pub(crate) fn hinting_wire_value(hinting: Hinting) -> u32 {
    match hinting {
        Hinting::GridFitted => 0,
        Hinting::Unhinted => 1,
    }
}

/// The hinting a wire value names.
#[must_use]
pub(crate) fn hinting_from_wire_value(value: u32) -> Hinting {
    match value {
        1 => Hinting::Unhinted,
        _ => Hinting::GridFitted,
    }
}

/// The wire value of a bitmap format.
#[must_use]
pub(crate) fn format_wire_value(format: BitmapFormat) -> u32 {
    match format {
        BitmapFormat::Coverage => 0,
        BitmapFormat::Rgba => 1,
    }
}

/// The bitmap format a wire value names.
#[must_use]
pub(crate) fn format_from_wire_value(value: u32) -> BitmapFormat {
    match value {
        1 => BitmapFormat::Rgba,
        _ => BitmapFormat::Coverage,
    }
}
