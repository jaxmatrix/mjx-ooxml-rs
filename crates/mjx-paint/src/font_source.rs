//! **The second file in this crate that names the font engine**, and the reason the seam gate's
//! exemption grew from one file to two.
//!
//! # Why the exemption had to grow, and why it was widened rather than deleted
//!
//! `tests/the_seam_holds.rs` confines `mjx-text` to `src/glyph_atlas.rs` and asserts that the file
//! still uses it, so that the permission can neither spread nor rot. MJXOFF-164 needs a second
//! thing from the font engine and it is not the atlas: **a PDF embeds a font file**, and an SVG
//! draws glyph outlines, and neither the file nor the outlines can be reached through a display
//! list. A display list records where a glyph's *pixels* are; it does not record the face.
//!
//! The three ways that could have gone, and why this is the one taken:
//!
//! * **Delete the assertion** and let any file name `mjx-text`. That trades a gate for a
//!   convenience, and the gate is the only thing holding the seam at rank 5.5 — see
//!   `tests/the_seam_holds.rs` for why no layering rule can do it.
//! * **Put the exporter's font handling in `glyph_atlas.rs`.** It would pass the gate unchanged and
//!   would be a file called "glyph atlas" that embeds font files, which is worse than a widened
//!   rule because the next reader has no way to know.
//! * **Name a second file, in the gate, with the reason.** Which is this: the gate lists two
//!   permitted files, asserts each of them *does* name the font engine, and refuses every other.
//!   The exemption is still enumerated, still checked in both directions, and still visible in the
//!   one place a reader would look.
//!
//! # What is here and what deliberately is not
//!
//! Here: the adapter from [`mjx_text::FontFace`] to [`crate::FontSource`] — a subset for embedding,
//! an advance, a glyph's character for a `/ToUnicode` map, and a glyph's outline.
//!
//! **Not here: any font parsing.** The subsetter is [`mjx_text::subset_truetype`] and the outline
//! walk is `mjx_text::FaceReader::outline`, both in the font engine where table surgery on
//! untrusted bytes belongs. MJXOFF-164 is explicit that a painter must not grow a second font
//! reader, and this file reads no font tables: it holds parsed faces and asks them questions.
//!
//! # The reverse `cmap` is built once per face, on the way in
//!
//! A `/ToUnicode` map needs glyph-to-character, and a `cmap` is character-to-glyph. The inversion is
//! done in [`FaceLibrary::insert`] rather than per glyph, because [`crate::FontSource`]'s methods
//! take `&self` and a lazily-filled cache would need interior mutability to answer them — an
//! `Rc<RefCell<_>>` in a type a render thread may hold, for an answer that costs one table walk.
//!
//! Where several characters map to one glyph the **lowest** is kept, so that two exports of the
//! same page produce the same map and an export can be compared against itself.

use std::collections::HashMap;
use std::sync::Arc;

use mjx_scene::{FillRule, PathCommand, ScenePoint};
use mjx_text::{subset_truetype, FontFace, GlyphIndex, OutlineCommand};

use crate::resources::{EmbeddableFace, FontSource};

/// The faces a document's runs are in, by the number the display list gives them.
///
/// A [`mjx_scene::SceneGlyphRun`]'s `face` is *"which face, as the rasteriser that will draw these
/// numbered it"* — per `Arc`, not per file — so the caller that built the scene is the one that can
/// say which face that is. This is where it says so.
#[derive(Clone, Default)]
pub struct FaceLibrary {
    faces: HashMap<u32, Entry>,
}

#[derive(Clone)]
struct Entry {
    face: Arc<FontFace>,
    /// Glyph to character, the `cmap` read backwards, lowest character per glyph.
    characters: HashMap<u16, char>,
}

impl core::fmt::Debug for FaceLibrary {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("FaceLibrary")
            .field("faces", &self.faces.len())
            .finish()
    }
}

impl FaceLibrary {
    /// A library with no faces in it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            faces: HashMap::new(),
        }
    }

    /// Register `parsed` as the face a display list calls `number`.
    ///
    /// Walks the face's `cmap` once, backwards, so that a `/ToUnicode` map costs no table work per
    /// glyph afterwards. See the module documentation for why that is done here.
    pub fn insert(&mut self, number: u32, parsed: Arc<FontFace>) {
        let mut characters: HashMap<u16, char> = HashMap::new();
        if let Ok(reader) = parsed.reader() {
            reader.for_each_mapped_character(&mut |character, glyph| {
                characters
                    .entry(glyph.0)
                    .and_modify(|held| {
                        if character < *held {
                            *held = character;
                        }
                    })
                    .or_insert(character);
            });
        }
        self.faces.insert(
            number,
            Entry {
                face: parsed,
                characters,
            },
        );
    }

    /// How many faces it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

impl FontSource for FaceLibrary {
    fn face(&self, face: u32, glyphs: &[u16]) -> Option<EmbeddableFace> {
        let entry = self.faces.get(&face)?;
        let wanted: Vec<GlyphIndex> = glyphs.iter().copied().map(GlyphIndex).collect();
        let subset = subset_truetype(&entry.face, &wanted).ok()?;
        let metrics = entry.face.metrics();
        let identity = entry.face.identity();
        Some(EmbeddableFace {
            data: subset.data().to_vec(),
            name: identity
                .postscript_name
                .clone()
                .unwrap_or_else(|| identity.family.replace(' ', "")),
            units_per_em: metrics.units_per_em,
            ascender: metrics.ascender,
            descender: metrics.descender,
            bounding_box: [
                metrics.global_bounds.left,
                metrics.global_bounds.bottom,
                metrics.global_bounds.right,
                metrics.global_bounds.top,
            ],
            italic_angle: metrics.italic_angle,
            cap_height: metrics.cap_height.unwrap_or(metrics.ascender),
            fixed_pitch: metrics.is_monospaced,
            italic: identity.slant != mjx_text::FontSlant::Upright,
            whole_face: subset.whole_face(),
        })
    }

    fn advance(&self, face: u32, glyph: u16) -> Option<u16> {
        let entry = self.faces.get(&face)?;
        let advance = entry.face.reader().ok()?.advance(GlyphIndex(glyph))?;
        u16::try_from(advance.font_units).ok()
    }

    fn character(&self, face: u32, glyph: u16) -> Option<char> {
        self.faces.get(&face)?.characters.get(&glyph).copied()
    }

    fn outline(&self, face: u32, glyph: u16, pixels_per_em: f32) -> Option<Vec<PathCommand>> {
        let entry = self.faces.get(&face)?;
        let reader = entry.face.reader().ok()?;
        let outline = reader.outline(GlyphIndex(glyph), pixels_per_em)?;
        Some(path_of(outline.commands()))
    }
}

/// The font engine's outline vocabulary in the display list's.
///
/// Two names for the same four curves. The conversion is here rather than in either crate because
/// neither should have to know about the other: `mjx-text` describes a glyph and `mjx-scene`
/// describes a page, and this file is the seam they are allowed to meet at.
fn path_of(commands: &[OutlineCommand]) -> Vec<PathCommand> {
    let point = |at: mjx_text::OutlinePoint| ScenePoint::new(at.x, at.y);
    commands
        .iter()
        .map(|command| match *command {
            OutlineCommand::MoveTo(to) => PathCommand::MoveTo(point(to)),
            OutlineCommand::LineTo(to) => PathCommand::LineTo(point(to)),
            OutlineCommand::QuadraticTo(control, end) => PathCommand::QuadraticTo {
                control: point(control),
                end: point(end),
            },
            OutlineCommand::CubicTo(first, second, end) => PathCommand::CubicTo {
                first_control: point(first),
                second_control: point(second),
                end: point(end),
            },
            OutlineCommand::Close => PathCommand::Close,
        })
        .collect()
}

/// The rule a glyph's contours are filled with.
///
/// Non-zero, always: a TrueType or `CFF` outline winds its holes the other way, and filling one
/// even-odd would leave the counter of an `o` solid wherever two contours overlap. Stated as a
/// constant so that both exporters read it from one place rather than each writing `NonZero`.
pub const GLYPH_FILL_RULE: FillRule = FillRule::NonZero;
