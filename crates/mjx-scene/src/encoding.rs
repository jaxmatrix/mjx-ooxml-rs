//! The flat binary encoding: what the bytes of a display list actually are.
//!
//! # The shape, in one picture
//!
//! ```text
//! ┌──────────────────────────────── 32 bytes ────────────────────────────────┐
//! │ "MJXS" │ version │ header │ device scale │ page w │ page h │ n │ f │ len │
//! ├──────────────────────── 12 bytes per section ────────────────────────────┤
//! │ kind │ stride │ offset │ length │   … one row per non-empty section …    │
//! ├──────────────────────────── the sections ────────────────────────────────┤
//! │ commands · transforms · clips · paints · gradients · stops · strokes ·   │
//! │ effects · geometries · path data · glyph runs · glyphs · images          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Everything is little-endian, every section starts on a four-byte boundary, and every table but
//! three has a **fixed stride**, so entry *n* of a table is at `offset + n * stride` and reading it
//! costs no allocation and no deserialisation. The three variable-length sections — the command
//! stream, the path data and nothing else — are walked, and each carries enough length in its own
//! records to be walked safely.
//!
//! # Why a section table and not a fixed set of offsets
//!
//! An empty table costs **no bytes at all**: only sections that hold something get a row. A page of
//! prose with no shapes, no images and no effects therefore carries four sections, not thirteen, and
//! the header of a small list is 68 bytes rather than 188. It also makes the format additive — a
//! later version may write a section this one has never heard of, and a reader that skips unknown
//! kinds still gets every record it does understand.
//!
//! # Why the version is in the first six bytes and checked before anything else
//!
//! Because it is the only field a reader of the *wrong* version can be trusted to understand. A
//! decoder that validated sections first would be interpreting a layout it does not know before it
//! discovered that it does not know it.
//!
//! # What is deliberately not in here
//!
//! Pixels. Not a glyph's, not an image's. A glyph record names an atlas page and a rectangle in it,
//! and an image record names the handle a box model issued; the pixels live in the atlas
//! (`mjx-text`) and in whatever holds decoded images, both of which outlive a frame and are shared
//! between pages. A display list that embedded them would be a copy of the page's memory rather than
//! a description of it.

use std::fmt;

/// Which table a section holds.
///
/// The discriminants are **wire values** and are stable for the life of a version: adding a kind
/// takes the next free number, and no existing number ever changes meaning.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u16)]
pub enum SectionKind {
    /// The command stream, in paint order. Variable-length records.
    Commands = 1,
    /// Affine maps, 24 bytes each.
    Transforms = 2,
    /// Clip regions, 24 bytes each.
    Clips = 3,
    /// Paints, 16 bytes each.
    Paints = 4,
    /// Gradients, 56 bytes each.
    Gradients = 5,
    /// Gradient stops, 8 bytes each; a gradient names a run of them.
    GradientStops = 6,
    /// Strokes, 24 bytes each.
    Strokes = 7,
    /// Effect nodes, 64 bytes each; the table is the effect DAG, in topological order.
    Effects = 8,
    /// Geometries, 32 bytes each.
    Geometries = 9,
    /// Path steps. Variable-length records; a geometry names a byte range.
    PathData = 10,
    /// Glyph runs, 32 bytes each; a run names a range of glyphs.
    GlyphRuns = 11,
    /// Glyphs, 36 bytes each.
    Glyphs = 12,
    /// Images, 72 bytes each.
    Images = 13,
}

impl SectionKind {
    /// Every kind, in wire order. The order sections are written in, and the order a diff walks
    /// them.
    pub const ALL: [Self; 13] = [
        Self::Commands,
        Self::Transforms,
        Self::Clips,
        Self::Paints,
        Self::Gradients,
        Self::GradientStops,
        Self::Strokes,
        Self::Effects,
        Self::Geometries,
        Self::PathData,
        Self::GlyphRuns,
        Self::Glyphs,
        Self::Images,
    ];

    /// The wire value.
    #[must_use]
    pub const fn wire_value(self) -> u16 {
        self as u16
    }

    /// The kind `value` names, or `None` for one this build has never heard of.
    #[must_use]
    pub const fn from_wire_value(value: u16) -> Option<Self> {
        Some(match value {
            1 => Self::Commands,
            2 => Self::Transforms,
            3 => Self::Clips,
            4 => Self::Paints,
            5 => Self::Gradients,
            6 => Self::GradientStops,
            7 => Self::Strokes,
            8 => Self::Effects,
            9 => Self::Geometries,
            10 => Self::PathData,
            11 => Self::GlyphRuns,
            12 => Self::Glyphs,
            13 => Self::Images,
            _ => return None,
        })
    }

    /// How many bytes one record takes, or `None` for a section whose records vary in length.
    #[must_use]
    pub const fn stride(self) -> Option<usize> {
        Some(match self {
            Self::Commands | Self::PathData => return None,
            Self::Transforms => TRANSFORM_STRIDE,
            Self::Clips => CLIP_STRIDE,
            Self::Paints => PAINT_STRIDE,
            Self::Gradients => GRADIENT_STRIDE,
            Self::GradientStops => GRADIENT_STOP_STRIDE,
            Self::Strokes => STROKE_STRIDE,
            Self::Effects => EFFECT_STRIDE,
            Self::Geometries => GEOMETRY_STRIDE,
            Self::GlyphRuns => GLYPH_RUN_STRIDE,
            Self::Glyphs => GLYPH_STRIDE,
            Self::Images => IMAGE_STRIDE,
        })
    }

    /// How the section is named in a message.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Commands => "commands",
            Self::Transforms => "transforms",
            Self::Clips => "clips",
            Self::Paints => "paints",
            Self::Gradients => "gradients",
            Self::GradientStops => "gradient stops",
            Self::Strokes => "strokes",
            Self::Effects => "effects",
            Self::Geometries => "geometries",
            Self::PathData => "path data",
            Self::GlyphRuns => "glyph runs",
            Self::Glyphs => "glyphs",
            Self::Images => "images",
        }
    }
}

impl fmt::Display for SectionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Where a record sits in its table.
///
/// **One index type for every table, not nine.** A typed index per table would catch a gradient
/// index handed to the geometry table at compile time, and it would cost nine newtypes with two
/// methods each on the public surface of a crate whose own reachability gate exists to keep that
/// surface small. The confusion it would catch is caught anyway, and later than compile time but
/// earlier than a wrong pixel: [`crate::DisplayList::from_bytes`] validates every index in every
/// record against the table its command addresses, so a list that named the wrong table does not
/// decode at all.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ResourceIndex(u32);

impl ResourceIndex {
    /// The value that names no entry, written where a record's reference is optional.
    ///
    /// `u32::MAX` rather than a separate presence bit, because a table that held four billion
    /// entries would already have failed [`crate::SceneError::TableFull`] one entry earlier.
    pub const NONE: u32 = u32::MAX;

    /// The `index`th entry.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Which entry.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

// -------------------------------------------------------------------------------------------
// The layout, as constants. Every one of these is asserted against a hand-written byte string in
// `tests/the_encoding_is_pinned_to_literals.rs`; changing one there and not here is what the gate
// is for.
// -------------------------------------------------------------------------------------------

/// The four bytes every display list begins with.
pub const MAGIC: [u8; 4] = *b"MJXS";

/// The encoding this build writes and the only one it reads.
pub const VERSION: u16 = 1;

/// How long the fixed header is, in bytes.
pub const HEADER_BYTES: usize = 32;

/// How long one row of the section table is, in bytes.
pub const SECTION_ROW_BYTES: usize = 12;

/// Every section starts on a multiple of this, so a record never straddles a word.
pub const SECTION_ALIGNMENT: usize = 4;

/// One affine map: six `f32`.
pub const TRANSFORM_STRIDE: usize = 24;
/// One clip: a kind, an optional geometry, and the box that bounds it.
pub const CLIP_STRIDE: usize = 24;
/// One paint: a kind and three payload words.
pub const PAINT_STRIDE: usize = 16;
/// One gradient.
pub const GRADIENT_STRIDE: usize = 56;
/// One gradient stop: a position and a colour.
pub const GRADIENT_STOP_STRIDE: usize = 8;
/// One stroke.
pub const STROKE_STRIDE: usize = 24;
/// One effect node.
pub const EFFECT_STRIDE: usize = 64;
/// One geometry.
pub const GEOMETRY_STRIDE: usize = 32;
/// One glyph run.
pub const GLYPH_RUN_STRIDE: usize = 32;
/// One glyph.
pub const GLYPH_STRIDE: usize = 36;
/// One image.
pub const IMAGE_STRIDE: usize = 72;

/// The opcode of each command, as a byte in the record header.
pub mod opcode {
    /// [`crate::Command::PushTransform`].
    pub const PUSH_TRANSFORM: u8 = 1;
    /// [`crate::Command::PushClip`].
    pub const PUSH_CLIP: u8 = 2;
    /// [`crate::Command::PushOpacity`].
    pub const PUSH_OPACITY: u8 = 3;
    /// [`crate::Command::PushEffect`].
    pub const PUSH_EFFECT: u8 = 4;
    /// [`crate::Command::Pop`].
    pub const POP: u8 = 5;
    /// [`crate::Command::FillPath`].
    pub const FILL_PATH: u8 = 6;
    /// [`crate::Command::StrokePath`].
    pub const STROKE_PATH: u8 = 7;
    /// [`crate::Command::DrawGlyphs`].
    pub const DRAW_GLYPHS: u8 = 8;
    /// [`crate::Command::DrawImage`].
    pub const DRAW_IMAGE: u8 = 9;
}

/// The tag of each path step, as the first word of its record.
pub mod path_step {
    /// [`crate::PathCommand::MoveTo`] — one point.
    pub const MOVE_TO: u32 = 1;
    /// [`crate::PathCommand::LineTo`] — one point.
    pub const LINE_TO: u32 = 2;
    /// [`crate::PathCommand::QuadraticTo`] — two points.
    pub const QUADRATIC_TO: u32 = 3;
    /// [`crate::PathCommand::CubicTo`] — three points.
    pub const CUBIC_TO: u32 = 4;
    /// [`crate::PathCommand::Close`] — no points.
    pub const CLOSE: u32 = 5;
}

// -------------------------------------------------------------------------------------------
// Reading. Every one of these takes a slice and an offset and returns an `Option`; there is no
// index and no `unwrap` anywhere below, which is what makes a malformed list an error.
// -------------------------------------------------------------------------------------------

/// `count` bytes of `bytes` from `offset`, or `None` if they are not all there.
#[must_use]
pub(crate) fn slice_at(bytes: &[u8], offset: usize, count: usize) -> Option<&[u8]> {
    let end = offset.checked_add(count)?;
    bytes.get(offset..end)
}

/// The `u8` at `offset`.
#[must_use]
pub(crate) fn read_u8(bytes: &[u8], offset: usize) -> Option<u8> {
    bytes.get(offset).copied()
}

/// The little-endian `u16` at `offset`.
#[must_use]
pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let field: [u8; 2] = slice_at(bytes, offset, 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(field))
}

/// The little-endian `i16` at `offset`.
#[must_use]
pub(crate) fn read_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    read_u16(bytes, offset).map(|value| value as i16)
}

/// The little-endian `u32` at `offset`.
#[must_use]
pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let field: [u8; 4] = slice_at(bytes, offset, 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(field))
}

/// The little-endian `i32` at `offset`.
#[must_use]
pub(crate) fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    read_u32(bytes, offset).map(|value| value as i32)
}

/// The little-endian `f32` at `offset`.
#[must_use]
pub(crate) fn read_f32(bytes: &[u8], offset: usize) -> Option<f32> {
    read_u32(bytes, offset).map(f32::from_bits)
}

// -------------------------------------------------------------------------------------------
// Writing.
// -------------------------------------------------------------------------------------------

/// Append a little-endian `u16`.
pub(crate) fn write_u16(into: &mut Vec<u8>, value: u16) {
    into.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `u32`.
pub(crate) fn write_u32(into: &mut Vec<u8>, value: u32) {
    into.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `i32`.
pub(crate) fn write_i32(into: &mut Vec<u8>, value: i32) {
    into.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `i16`.
pub(crate) fn write_i16(into: &mut Vec<u8>, value: i16) {
    into.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian `f32`, normalised so that no non-finite bit pattern ever reaches the
/// bytes — see [`crate::geometry::finite`] for why that matters to the frame diff.
pub(crate) fn write_f32(into: &mut Vec<u8>, value: f32) {
    write_u32(into, crate::geometry::finite(value).to_bits());
}

/// A colour as one word: red in the lowest byte, then green, blue and alpha.
#[must_use]
pub(crate) fn pack_color(color: mjx_tokens::Color) -> u32 {
    u32::from(color.red)
        | (u32::from(color.green) << 8)
        | (u32::from(color.blue) << 16)
        | (u32::from(color.alpha) << 24)
}

/// The colour a word packed by [`pack_color`] holds.
#[must_use]
pub(crate) fn unpack_color(word: u32) -> mjx_tokens::Color {
    mjx_tokens::Color {
        red: (word & 0xff) as u8,
        green: ((word >> 8) & 0xff) as u8,
        blue: ((word >> 16) & 0xff) as u8,
        alpha: ((word >> 24) & 0xff) as u8,
    }
}
