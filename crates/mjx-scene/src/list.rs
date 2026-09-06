//! [`DisplayList`] — the bytes, and everything that reads them.
//!
//! # One `Vec<u8>` and nothing else
//!
//! A display list owns exactly one allocation. There is no `Vec<Command>`, no `Vec<Paint>`, no
//! `Arc` anywhere in it — because the thing it has to be good at is being *made per frame, diffed
//! against the last frame, and cached to disk*, and a tree of allocated objects is bad at all three.
//! A 50 000-glyph page is one `malloc`, one `memcmp` against the previous frame, and one `write`.
//!
//! # Validated once, read for ever
//!
//! [`DisplayList::from_bytes`] does **all** the checking: the magic, the version, every section's
//! extent, every fixed-stride table's raggedness, every command's opcode and length, every resource
//! index against the table it names, every path's steps, every effect's input, and the balance of
//! the push/pop stack. A list that decodes is a list whose every reference is in range.
//!
//! That is why the accessors below return values rather than results, and why they are still written
//! with `get` and `Option` underneath: the validation is the guarantee, and the `Option` is the
//! belt — there is no index, no `unwrap` and no arithmetic that can wrap on any path here, so a
//! defect in the validator produces a wrong answer rather than a crash.

use mjx_text::DeviceScale;

use crate::command::{Clip, Command};
use crate::effect::{BlendMode, Effect, EffectKind};
use crate::encoding::{
    opcode, path_step, read_f32, read_i16, read_i32, read_u16, read_u32, read_u8, slice_at,
    unpack_color, ResourceIndex, SectionKind, CLIP_STRIDE, EFFECT_STRIDE, GEOMETRY_STRIDE,
    GLYPH_RUN_STRIDE, GLYPH_STRIDE, GRADIENT_STOP_STRIDE, GRADIENT_STRIDE, HEADER_BYTES,
    IMAGE_STRIDE, MAGIC, PAINT_STRIDE, SECTION_ALIGNMENT, SECTION_ROW_BYTES, STROKE_STRIDE,
    TRANSFORM_STRIDE, VERSION,
};
use crate::error::SceneError;
use crate::geometry::{FillRule, Geometry, PathCommand, ScenePoint, SceneRect, SceneTransform};
use crate::glyphs::{
    direction_from_wire_value, format_from_wire_value, hinting_from_wire_value, AtlasPlacement,
    GlyphImage, SceneGlyph, SceneGlyphRun,
};
use crate::paint::{
    CompoundStroke, DashPattern, Gradient, GradientKind, GradientStop, Image, ImageAdjustments,
    ImageFillMode, LineCap, LineEnd, LineEndShape, LineEndSize, LineJoin, Paint, PathShade,
    PatternPreset, RectangleAnchor, Stroke, StrokeAlignment, TileFlip,
};

/// How many section slots the wire values occupy, so that a lookup is an array index.
const SECTION_SLOTS: usize = 14;

/// Where one section lives in the blob.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Span {
    offset: usize,
    length: usize,
}

/// A page, drawn: a flat, versioned, typed-record arena that four painters and two exporters read.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DisplayList {
    bytes: Vec<u8>,
    /// By wire value, so `sections[SectionKind::Paints.wire_value() as usize]` is one array index.
    sections: [Option<Span>; SECTION_SLOTS],
    device_scale_bits: u32,
    page_width_bits: u32,
    page_height_bits: u32,
    command_count: u32,
}

impl DisplayList {
    /// The four bytes every display list begins with.
    pub const MAGIC: [u8; 4] = MAGIC;

    /// The encoding version this build writes and reads.
    pub const VERSION: u16 = VERSION;

    /// Read and fully validate a display list.
    ///
    /// # Errors
    ///
    /// Every way bytes can fail to be a display list this build understands: see [`SceneError`].
    /// Nothing here panics, indexes a slice or unwraps, because these bytes may have come off a disk
    /// or a wire.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SceneError> {
        let mut list = Self::read_header(bytes)?;
        list.command_count = list.validate()?;
        Ok(list)
    }

    /// The bytes, for a cache, a transport or a hash.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The bytes, taken.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// How many bytes the whole list occupies — the figure R13's byte-budgeted cache charges.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// The device scale every position and image in this list is expressed at.
    ///
    /// A display list is built for **one** scale. A zoom produces a new one; it does not transform
    /// this one, because the glyphs in it were rasterised for this scale's buckets.
    #[must_use]
    pub fn device_scale(&self) -> DeviceScale {
        DeviceScale::from_pixels_per_point(f32::from_bits(self.device_scale_bits))
    }

    /// How large the page is, in device pixels.
    #[must_use]
    pub fn page_size(&self) -> (f32, f32) {
        (
            f32::from_bits(self.page_width_bits),
            f32::from_bits(self.page_height_bits),
        )
    }

    /// The raw bytes of one section, for a frame diff that compares them without decoding them.
    #[must_use]
    pub fn section_bytes(&self, kind: SectionKind) -> &[u8] {
        let Some(span) = self.span(kind) else {
            return &[];
        };
        slice_at(&self.bytes, span.offset, span.length).unwrap_or(&[])
    }

    /// How many records a section holds.
    ///
    /// For [`SectionKind::PathData`] this is zero: path data is addressed by byte range from a
    /// geometry, not by index, so it has no record count to report.
    #[must_use]
    pub fn record_count(&self, kind: SectionKind) -> u32 {
        if kind == SectionKind::Commands {
            return self.command_count;
        }
        let Some(stride) = kind.stride() else {
            return 0;
        };
        let Some(span) = self.span(kind) else {
            return 0;
        };
        u32::try_from(span.length / stride).unwrap_or(u32::MAX)
    }

    /// The commands, in paint order.
    #[must_use]
    pub fn commands(&self) -> Commands<'_> {
        Commands {
            bytes: self.section_bytes(SectionKind::Commands),
            offset: 0,
        }
    }

    /// The `index`th affine map.
    #[must_use]
    pub fn transform(&self, index: ResourceIndex) -> Option<SceneTransform> {
        let at = self.record(SectionKind::Transforms, index, TRANSFORM_STRIDE)?;
        Some(SceneTransform {
            scale_x: read_f32(&self.bytes, at)?,
            shear_y: read_f32(&self.bytes, at + 4)?,
            shear_x: read_f32(&self.bytes, at + 8)?,
            scale_y: read_f32(&self.bytes, at + 12)?,
            translate_x: read_f32(&self.bytes, at + 16)?,
            translate_y: read_f32(&self.bytes, at + 20)?,
        })
    }

    /// The `index`th clip.
    #[must_use]
    pub fn clip(&self, index: ResourceIndex) -> Option<Clip> {
        let at = self.record(SectionKind::Clips, index, CLIP_STRIDE)?;
        let geometry = read_u32(&self.bytes, at + 4)?;
        Some(Clip {
            geometry: (geometry != ResourceIndex::NONE).then(|| ResourceIndex::new(geometry)),
            bounds: self.rect_at(at + 8)?,
        })
    }

    /// The `index`th paint.
    #[must_use]
    pub fn paint(&self, index: ResourceIndex) -> Option<Paint> {
        let at = self.record(SectionKind::Paints, index, PAINT_STRIDE)?;
        let first = read_u32(&self.bytes, at + 4)?;
        Some(match read_u32(&self.bytes, at)? {
            PAINT_SOLID => Paint::Solid(unpack_color(first)),
            PAINT_GRADIENT => Paint::Gradient(ResourceIndex::new(first)),
            PAINT_PATTERN => Paint::Pattern {
                preset: PatternPreset::from_wire_value(first)?,
                foreground: unpack_color(read_u32(&self.bytes, at + 8)?),
                background: unpack_color(read_u32(&self.bytes, at + 12)?),
            },
            PAINT_IMAGE => Paint::Image(ResourceIndex::new(first)),
            _ => return None,
        })
    }

    /// The `index`th gradient, with its stops read out of the stop table.
    #[must_use]
    pub fn gradient(&self, index: ResourceIndex) -> Option<Gradient> {
        let at = self.record(SectionKind::Gradients, index, GRADIENT_STRIDE)?;
        let kind = match read_u32(&self.bytes, at)? {
            GRADIENT_LINEAR => GradientKind::Linear,
            GRADIENT_RADIAL => GradientKind::Radial,
            GRADIENT_PATH => GradientKind::Path,
            _ => return None,
        };
        let first_stop = read_u32(&self.bytes, at + 4)?;
        let stop_count = read_u32(&self.bytes, at + 8)?;
        let flags = read_u32(&self.bytes, at + 12)?;
        let mut stops = Vec::with_capacity(stop_count as usize);
        for step in 0..stop_count {
            let stop = self.gradient_stop(ResourceIndex::new(first_stop.checked_add(step)?))?;
            stops.push(stop);
        }
        Some(Gradient {
            kind,
            stops,
            angle: read_f32(&self.bytes, at + 16)?,
            angle_is_scaled: flags & GRADIENT_FLAG_SCALED != 0,
            path_shade: match read_u32(&self.bytes, at + 20)? {
                PATH_SHADE_CIRCLE => PathShade::Circle,
                PATH_SHADE_RECTANGLE => PathShade::Rectangle,
                _ => PathShade::Shape,
            },
            focus: self.rect_at(at + 24)?,
            tile: self.rect_at(at + 40)?,
            flip: tile_flip_from_flags(flags),
            rotate_with_shape: flags & GRADIENT_FLAG_ROTATE_WITH_SHAPE != 0,
        })
    }

    /// The `index`th gradient stop.
    #[must_use]
    pub fn gradient_stop(&self, index: ResourceIndex) -> Option<GradientStop> {
        let at = self.record(SectionKind::GradientStops, index, GRADIENT_STOP_STRIDE)?;
        Some(GradientStop {
            position_in_ten_thousandths: read_u16(&self.bytes, at)?,
            color: unpack_color(read_u32(&self.bytes, at + 4)?),
        })
    }

    /// The `index`th stroke.
    #[must_use]
    pub fn stroke(&self, index: ResourceIndex) -> Option<Stroke> {
        let at = self.record(SectionKind::Strokes, index, STROKE_STRIDE)?;
        let join = match read_u8(&self.bytes, at + 13)? {
            JOIN_BEVEL => LineJoin::Bevel,
            JOIN_MITER => LineJoin::Miter {
                limit: read_f32(&self.bytes, at + 8)?,
            },
            _ => LineJoin::Round,
        };
        Some(Stroke {
            paint: ResourceIndex::new(read_u32(&self.bytes, at)?),
            width: read_f32(&self.bytes, at + 4)?,
            cap: match read_u8(&self.bytes, at + 12)? {
                CAP_ROUND => LineCap::Round,
                CAP_SQUARE => LineCap::Square,
                _ => LineCap::Flat,
            },
            join,
            dash: dash_from_wire_value(read_u8(&self.bytes, at + 14)?),
            alignment: match read_u8(&self.bytes, at + 15)? {
                ALIGNMENT_INSET => StrokeAlignment::Inset,
                _ => StrokeAlignment::Centered,
            },
            compound: compound_from_wire_value(read_u8(&self.bytes, at + 16)?),
            head: LineEnd {
                shape: line_end_shape_from_wire_value(read_u8(&self.bytes, at + 17)?),
                width: line_end_size_from_wire_value(read_u8(&self.bytes, at + 18)?),
                length: line_end_size_from_wire_value(read_u8(&self.bytes, at + 19)?),
            },
            tail: LineEnd {
                shape: line_end_shape_from_wire_value(read_u8(&self.bytes, at + 20)?),
                width: line_end_size_from_wire_value(read_u8(&self.bytes, at + 21)?),
                length: line_end_size_from_wire_value(read_u8(&self.bytes, at + 22)?),
            },
        })
    }

    /// The `index`th effect node.
    #[must_use]
    pub fn effect(&self, index: ResourceIndex) -> Option<Effect> {
        let at = self.record(SectionKind::Effects, index, EFFECT_STRIDE)?;
        let input = read_u32(&self.bytes, at + 4)?;
        let paint = read_u32(&self.bytes, at + 8)?;
        let flags = read_u32(&self.bytes, at + 60)?;
        Some(Effect {
            kind: EffectKind::from_wire_value(read_u32(&self.bytes, at)?)?,
            input: (input != ResourceIndex::NONE).then(|| ResourceIndex::new(input)),
            paint: (paint != ResourceIndex::NONE).then(|| ResourceIndex::new(paint)),
            radius: read_f32(&self.bytes, at + 12)?,
            distance: read_f32(&self.bytes, at + 16)?,
            direction: read_f32(&self.bytes, at + 20)?,
            scale_x: read_f32(&self.bytes, at + 24)?,
            scale_y: read_f32(&self.bytes, at + 28)?,
            skew_x: read_f32(&self.bytes, at + 32)?,
            skew_y: read_f32(&self.bytes, at + 36)?,
            start_alpha: read_f32(&self.bytes, at + 40)?,
            start_position: read_f32(&self.bytes, at + 44)?,
            end_alpha: read_f32(&self.bytes, at + 48)?,
            end_position: read_f32(&self.bytes, at + 52)?,
            fade_direction: read_f32(&self.bytes, at + 56)?,
            grow: flags & EFFECT_FLAG_GROW != 0,
            rotate_with_shape: flags & EFFECT_FLAG_ROTATE_WITH_SHAPE != 0,
            anchor: anchor_from_wire_value((flags >> EFFECT_ANCHOR_SHIFT) & 0xf),
            blend: blend_from_wire_value((flags >> EFFECT_BLEND_SHIFT) & 0xf),
        })
    }

    /// The `index`th geometry, with a path's steps read out of the path-data section.
    #[must_use]
    pub fn geometry(&self, index: ResourceIndex) -> Option<Geometry> {
        let at = self.record(SectionKind::Geometries, index, GEOMETRY_STRIDE)?;
        let first = read_u32(&self.bytes, at + 4)?;
        let second = read_u32(&self.bytes, at + 8)?;
        let flags = read_u32(&self.bytes, at + 12)?;
        let bounds = self.rect_at(at + 16)?;
        Some(match read_u32(&self.bytes, at)? {
            GEOMETRY_RECTANGLE => Geometry::Rectangle(bounds),
            GEOMETRY_PATH => Geometry::Path {
                commands: self.path_steps(first as usize, second as usize)?,
                fill_rule: if flags & GEOMETRY_FLAG_EVEN_ODD == 0 {
                    FillRule::NonZero
                } else {
                    FillRule::EvenOdd
                },
                bounds,
            },
            GEOMETRY_UNRESOLVED => Geometry::Unresolved {
                outline: u64::from(first) | (u64::from(second) << 32),
                bounds,
            },
            _ => return None,
        })
    }

    /// The `index`th glyph run, with its glyphs read out of the glyph table.
    #[must_use]
    pub fn glyph_run(&self, index: ResourceIndex) -> Option<SceneGlyphRun> {
        let at = self.record(SectionKind::GlyphRuns, index, GLYPH_RUN_STRIDE)?;
        let first_glyph = read_u32(&self.bytes, at + 4)?;
        let glyph_count = read_u32(&self.bytes, at + 8)?;
        let flags = read_u32(&self.bytes, at + 28)?;
        let mut glyphs = Vec::with_capacity(glyph_count as usize);
        for step in 0..glyph_count {
            glyphs.push(self.glyph(ResourceIndex::new(first_glyph.checked_add(step)?))?);
        }
        Some(SceneGlyphRun {
            face: read_u32(&self.bytes, at)?,
            bucket_steps: read_u32(&self.bytes, at + 12)?,
            residual_scale: read_f32(&self.bytes, at + 16)?,
            origin: ScenePoint::new(
                read_f32(&self.bytes, at + 20)?,
                read_f32(&self.bytes, at + 24)?,
            ),
            direction: direction_from_wire_value(flags & 0x1),
            hinting: hinting_from_wire_value((flags >> 1) & 0x1),
            level: ((flags >> 8) & 0xff) as u8,
            glyphs,
        })
    }

    /// The `index`th glyph.
    #[must_use]
    pub fn glyph(&self, index: ResourceIndex) -> Option<SceneGlyph> {
        let at = self.record(SectionKind::Glyphs, index, GLYPH_STRIDE)?;
        let kind = read_u8(&self.bytes, at + 15)?;
        let page_or_geometry = read_u32(&self.bytes, at + 16)?;
        Some(SceneGlyph {
            x: read_i32(&self.bytes, at)?,
            y: read_i32(&self.bytes, at + 4)?,
            cluster: read_u32(&self.bytes, at + 8)?,
            glyph: read_u16(&self.bytes, at + 12)?,
            subpixel: read_u8(&self.bytes, at + 14)?,
            image: match kind {
                GLYPH_IMAGE_ATLAS => GlyphImage::Atlas(AtlasPlacement {
                    page: page_or_geometry,
                    format: format_from_wire_value(read_u32(&self.bytes, at + 32)?),
                    x: read_u16(&self.bytes, at + 20)?,
                    y: read_u16(&self.bytes, at + 22)?,
                    width: read_u16(&self.bytes, at + 24)?,
                    height: read_u16(&self.bytes, at + 26)?,
                    offset_from_origin_x: read_i16(&self.bytes, at + 28)?,
                    offset_from_origin_y: read_i16(&self.bytes, at + 30)?,
                }),
                GLYPH_IMAGE_OUTLINE => GlyphImage::Outline(ResourceIndex::new(page_or_geometry)),
                _ => GlyphImage::Blank,
            },
        })
    }

    /// The `index`th image.
    #[must_use]
    pub fn image(&self, index: ResourceIndex) -> Option<Image> {
        let at = self.record(SectionKind::Images, index, IMAGE_STRIDE)?;
        let flags = read_u32(&self.bytes, at + 8)?;
        Some(Image {
            handle: u64::from(read_u32(&self.bytes, at)?)
                | (u64::from(read_u32(&self.bytes, at + 4)?) << 32),
            fill_mode: if flags & IMAGE_FLAG_TILE == 0 {
                ImageFillMode::Stretch
            } else {
                ImageFillMode::Tile
            },
            crop: self.rect_at(at + 12)?,
            tile_offset: ScenePoint::new(
                read_f32(&self.bytes, at + 28)?,
                read_f32(&self.bytes, at + 32)?,
            ),
            tile_scale_x: read_f32(&self.bytes, at + 36)?,
            tile_scale_y: read_f32(&self.bytes, at + 40)?,
            flip: tile_flip_from_flags(flags),
            anchor: anchor_from_wire_value((flags >> IMAGE_ANCHOR_SHIFT) & 0xf),
            rotate_with_shape: flags & IMAGE_FLAG_ROTATE_WITH_SHAPE != 0,
            adjustments: ImageAdjustments {
                alpha_in_ten_thousandths: read_u16(&self.bytes, at + 44)?,
                brightness_in_ten_thousandths: read_i16(&self.bytes, at + 48)?,
                contrast_in_ten_thousandths: read_i16(&self.bytes, at + 50)?,
                grayscale: flags & IMAGE_FLAG_GRAYSCALE != 0,
                duotone: (flags & IMAGE_FLAG_DUOTONE != 0).then(|| {
                    (
                        unpack_color(read_u32(&self.bytes, at + 56).unwrap_or(0)),
                        unpack_color(read_u32(&self.bytes, at + 60).unwrap_or(0)),
                    )
                }),
                color_change: (flags & IMAGE_FLAG_COLOR_CHANGE != 0).then(|| {
                    (
                        unpack_color(read_u32(&self.bytes, at + 64).unwrap_or(0)),
                        unpack_color(read_u32(&self.bytes, at + 68).unwrap_or(0)),
                    )
                }),
            },
        })
    }

    // -------------------------------------------------------------------------------------
    // Reading the header, and validating everything
    // -------------------------------------------------------------------------------------

    /// The header and the section table, with nothing inside the sections looked at yet.
    fn read_header(bytes: Vec<u8>) -> Result<Self, SceneError> {
        let available = bytes.len();
        let Some(magic) = slice_at(&bytes, 0, 4) else {
            return Err(SceneError::Truncated {
                offset: 0,
                needed: HEADER_BYTES,
                available,
            });
        };
        if magic != MAGIC {
            let mut found = [0_u8; 4];
            found.copy_from_slice(magic);
            return Err(SceneError::NotADisplayList { found });
        }
        let Some(version) = read_u16(&bytes, 4) else {
            return Err(SceneError::Truncated {
                offset: 4,
                needed: HEADER_BYTES,
                available,
            });
        };
        if version != VERSION {
            return Err(SceneError::UnsupportedVersion {
                found: version,
                supported: VERSION,
            });
        }
        let header_bytes = read_u16(&bytes, 6).ok_or(SceneError::Truncated {
            offset: 6,
            needed: HEADER_BYTES,
            available,
        })?;
        if usize::from(header_bytes) != HEADER_BYTES {
            return Err(SceneError::MalformedHeader {
                reason: "the header length is not the one this version defines",
            });
        }
        let read = |offset: usize| -> Result<u32, SceneError> {
            read_u32(&bytes, offset).ok_or(SceneError::Truncated {
                offset,
                needed: HEADER_BYTES,
                available,
            })
        };
        let device_scale_bits = read(8)?;
        let page_width_bits = read(12)?;
        let page_height_bits = read(16)?;
        let section_count = read_u16(&bytes, 20).ok_or(SceneError::Truncated {
            offset: 20,
            needed: HEADER_BYTES,
            available,
        })?;
        let flags = read_u16(&bytes, 22).ok_or(SceneError::Truncated {
            offset: 22,
            needed: HEADER_BYTES,
            available,
        })?;
        if flags != 0 {
            return Err(SceneError::MalformedHeader {
                reason: "a flag this version does not define is set",
            });
        }
        let total = read(24)? as usize;
        if total != available {
            return Err(SceneError::MalformedHeader {
                reason: "the declared total length is not the number of bytes there are",
            });
        }
        if read(28)? != 0 {
            return Err(SceneError::MalformedHeader {
                reason: "the reserved word is not zero",
            });
        }
        if usize::from(section_count) > SECTION_SLOTS - 1 {
            return Err(SceneError::MalformedHeader {
                reason: "more sections are declared than this version has kinds",
            });
        }

        let mut sections: [Option<Span>; SECTION_SLOTS] = [None; SECTION_SLOTS];
        let mut previous_kind = 0_u16;
        let mut watermark = HEADER_BYTES + usize::from(section_count) * SECTION_ROW_BYTES;
        for row in 0..usize::from(section_count) {
            let at = HEADER_BYTES + row * SECTION_ROW_BYTES;
            let kind_value = read_u16(&bytes, at).ok_or(SceneError::Truncated {
                offset: at,
                needed: SECTION_ROW_BYTES,
                available,
            })?;
            let stride = read_u16(&bytes, at + 2).ok_or(SceneError::Truncated {
                offset: at + 2,
                needed: SECTION_ROW_BYTES,
                available,
            })?;
            let offset = read(at + 4)? as usize;
            let length = read(at + 8)? as usize;
            let Some(kind) = SectionKind::from_wire_value(kind_value) else {
                return Err(SceneError::MalformedHeader {
                    reason: "a section declares a kind this version does not define",
                });
            };
            if kind_value <= previous_kind {
                return Err(SceneError::MalformedHeader {
                    reason: "the sections are not in ascending order of kind, so one is repeated \
                             or misplaced",
                });
            }
            previous_kind = kind_value;
            if usize::from(stride) != kind.stride().unwrap_or(0) {
                return Err(SceneError::MalformedSection {
                    section: kind,
                    offset,
                    length,
                    reason: "declares a record size this version does not use for it",
                });
            }
            if !offset.is_multiple_of(SECTION_ALIGNMENT) {
                return Err(SceneError::MalformedSection {
                    section: kind,
                    offset,
                    length,
                    reason: "does not begin on a four-byte boundary",
                });
            }
            if offset < watermark {
                return Err(SceneError::MalformedSection {
                    section: kind,
                    offset,
                    length,
                    reason: "begins before the end of what precedes it",
                });
            }
            let end = offset
                .checked_add(length)
                .ok_or(SceneError::MalformedSection {
                    section: kind,
                    offset,
                    length,
                    reason: "ends past the end of anything addressable",
                })?;
            if end > available {
                return Err(SceneError::MalformedSection {
                    section: kind,
                    offset,
                    length,
                    reason: "ends past the end of the blob",
                });
            }
            if let Some(stride) = kind.stride() {
                if stride == 0 || !length.is_multiple_of(stride) {
                    return Err(SceneError::RaggedSection {
                        section: kind,
                        length,
                        stride,
                    });
                }
            }
            watermark = end;
            sections[usize::from(kind_value)] = Some(Span { offset, length });
        }

        Ok(Self {
            bytes,
            sections,
            device_scale_bits,
            page_width_bits,
            page_height_bits,
            command_count: 0,
        })
    }

    /// Check every record of every section, and return how many commands there are.
    fn validate(&self) -> Result<u32, SceneError> {
        self.validate_paints()?;
        self.validate_gradients()?;
        self.validate_strokes()?;
        self.validate_effects()?;
        self.validate_geometries()?;
        self.validate_clips()?;
        self.validate_glyphs()?;
        self.validate_glyph_runs()?;
        self.validate_commands()
    }

    fn require(&self, section: SectionKind, index: u32) -> Result<(), SceneError> {
        let count = self.record_count(section);
        if index < count {
            return Ok(());
        }
        Err(SceneError::ResourceOutOfRange {
            section,
            index,
            count,
        })
    }

    fn validate_paints(&self) -> Result<(), SceneError> {
        for index in 0..self.record_count(SectionKind::Paints) {
            let paint =
                self.paint(ResourceIndex::new(index))
                    .ok_or(SceneError::UnknownRecordKind {
                        section: SectionKind::Paints,
                        index,
                        reason: "names a paint kind or a pattern preset this version does not \
                                 define",
                    })?;
            match paint {
                Paint::Gradient(gradient) => {
                    self.require(SectionKind::Gradients, gradient.index())?;
                }
                Paint::Image(image) => self.require(SectionKind::Images, image.index())?,
                Paint::Solid(_) | Paint::Pattern { .. } => {}
            }
        }
        Ok(())
    }

    fn validate_gradients(&self) -> Result<(), SceneError> {
        let stops = self.record_count(SectionKind::GradientStops);
        for index in 0..self.record_count(SectionKind::Gradients) {
            let at = self
                .record(
                    SectionKind::Gradients,
                    ResourceIndex::new(index),
                    GRADIENT_STRIDE,
                )
                .ok_or(SceneError::ResourceOutOfRange {
                    section: SectionKind::Gradients,
                    index,
                    count: self.record_count(SectionKind::Gradients),
                })?;
            let kind = read_u32(&self.bytes, at).unwrap_or(u32::MAX);
            if kind > GRADIENT_PATH {
                return Err(SceneError::UnknownRecordKind {
                    section: SectionKind::Gradients,
                    index,
                    reason: "names a gradient kind this version does not define",
                });
            }
            let first = read_u32(&self.bytes, at + 4).unwrap_or(u32::MAX);
            let count = read_u32(&self.bytes, at + 8).unwrap_or(u32::MAX);
            let end = first
                .checked_add(count)
                .ok_or(SceneError::ResourceOutOfRange {
                    section: SectionKind::GradientStops,
                    index: first,
                    count: stops,
                })?;
            if end > stops {
                return Err(SceneError::ResourceOutOfRange {
                    section: SectionKind::GradientStops,
                    index: end.saturating_sub(1),
                    count: stops,
                });
            }
        }
        Ok(())
    }

    fn validate_strokes(&self) -> Result<(), SceneError> {
        for index in 0..self.record_count(SectionKind::Strokes) {
            let stroke =
                self.stroke(ResourceIndex::new(index))
                    .ok_or(SceneError::ResourceOutOfRange {
                        section: SectionKind::Strokes,
                        index,
                        count: self.record_count(SectionKind::Strokes),
                    })?;
            self.require(SectionKind::Paints, stroke.paint.index())?;
        }
        Ok(())
    }

    fn validate_effects(&self) -> Result<(), SceneError> {
        for index in 0..self.record_count(SectionKind::Effects) {
            let effect =
                self.effect(ResourceIndex::new(index))
                    .ok_or(SceneError::UnknownRecordKind {
                        section: SectionKind::Effects,
                        index,
                        reason: "names an effect kind this version does not define",
                    })?;
            if let Some(input) = effect.input {
                if input.index() >= index {
                    return Err(SceneError::EffectCycle {
                        index,
                        input: input.index(),
                    });
                }
            }
            if let Some(paint) = effect.paint {
                self.require(SectionKind::Paints, paint.index())?;
            }
        }
        Ok(())
    }

    fn validate_geometries(&self) -> Result<(), SceneError> {
        for index in 0..self.record_count(SectionKind::Geometries) {
            let at = self
                .record(
                    SectionKind::Geometries,
                    ResourceIndex::new(index),
                    GEOMETRY_STRIDE,
                )
                .unwrap_or(0);
            self.geometry(ResourceIndex::new(index))
                .ok_or(SceneError::MalformedPath {
                    offset: read_u32(&self.bytes, at + 4).unwrap_or(0) as usize,
                    reason: "the geometry names a kind this version does not define, or its path \
                             data is truncated or carries an unknown step",
                })?;
        }
        Ok(())
    }

    fn validate_clips(&self) -> Result<(), SceneError> {
        for index in 0..self.record_count(SectionKind::Clips) {
            let clip =
                self.clip(ResourceIndex::new(index))
                    .ok_or(SceneError::ResourceOutOfRange {
                        section: SectionKind::Clips,
                        index,
                        count: self.record_count(SectionKind::Clips),
                    })?;
            if let Some(geometry) = clip.geometry {
                self.require(SectionKind::Geometries, geometry.index())?;
            }
        }
        Ok(())
    }

    fn validate_glyphs(&self) -> Result<(), SceneError> {
        for index in 0..self.record_count(SectionKind::Glyphs) {
            let glyph =
                self.glyph(ResourceIndex::new(index))
                    .ok_or(SceneError::ResourceOutOfRange {
                        section: SectionKind::Glyphs,
                        index,
                        count: self.record_count(SectionKind::Glyphs),
                    })?;
            if let GlyphImage::Outline(geometry) = glyph.image {
                self.require(SectionKind::Geometries, geometry.index())?;
            }
        }
        Ok(())
    }

    fn validate_glyph_runs(&self) -> Result<(), SceneError> {
        let glyphs = self.record_count(SectionKind::Glyphs);
        for index in 0..self.record_count(SectionKind::GlyphRuns) {
            let at = self
                .record(
                    SectionKind::GlyphRuns,
                    ResourceIndex::new(index),
                    GLYPH_RUN_STRIDE,
                )
                .ok_or(SceneError::ResourceOutOfRange {
                    section: SectionKind::GlyphRuns,
                    index,
                    count: self.record_count(SectionKind::GlyphRuns),
                })?;
            let first = read_u32(&self.bytes, at + 4).unwrap_or(u32::MAX);
            let count = read_u32(&self.bytes, at + 8).unwrap_or(u32::MAX);
            let end = first
                .checked_add(count)
                .ok_or(SceneError::ResourceOutOfRange {
                    section: SectionKind::Glyphs,
                    index: first,
                    count: glyphs,
                })?;
            if end > glyphs {
                return Err(SceneError::ResourceOutOfRange {
                    section: SectionKind::Glyphs,
                    index: end.saturating_sub(1),
                    count: glyphs,
                });
            }
        }
        Ok(())
    }

    fn validate_commands(&self) -> Result<u32, SceneError> {
        let bytes = self.section_bytes(SectionKind::Commands);
        let mut offset = 0_usize;
        let mut index = 0_usize;
        let mut depth = 0_usize;
        let mut count = 0_u32;
        while offset < bytes.len() {
            let code = read_u8(bytes, offset).ok_or(SceneError::MalformedCommand {
                index,
                opcode: 0,
                length: 0,
                reason: "is truncated before its own header",
            })?;
            let length = usize::from(read_u16(bytes, offset + 2).ok_or(
                SceneError::MalformedCommand {
                    index,
                    opcode: code,
                    length: 0,
                    reason: "is truncated before its own header",
                },
            )?);
            let command = decode_command(bytes, offset, index)?;
            if command.pushes_state() {
                depth += 1;
            } else if command == Command::Pop {
                depth = depth.checked_sub(1).ok_or(SceneError::UnbalancedStack {
                    reason: "a `Pop` arrived with nothing pushed",
                })?;
            }
            match command {
                Command::PushTransform(id) => self.require(SectionKind::Transforms, id.index())?,
                Command::PushClip(id) => self.require(SectionKind::Clips, id.index())?,
                Command::PushEffect(id) => self.require(SectionKind::Effects, id.index())?,
                Command::PushOpacity(_) | Command::Pop => {}
                Command::FillPath { geometry, paint } => {
                    self.require(SectionKind::Geometries, geometry.index())?;
                    self.require(SectionKind::Paints, paint.index())?;
                }
                Command::StrokePath { geometry, stroke } => {
                    self.require(SectionKind::Geometries, geometry.index())?;
                    self.require(SectionKind::Strokes, stroke.index())?;
                }
                Command::DrawGlyphs { run, paint } => {
                    self.require(SectionKind::GlyphRuns, run.index())?;
                    self.require(SectionKind::Paints, paint.index())?;
                }
                Command::DrawImage { image, .. } => {
                    self.require(SectionKind::Images, image.index())?;
                }
            }
            offset += length;
            index += 1;
            count = count.saturating_add(1);
        }
        if depth != 0 {
            return Err(SceneError::UnbalancedStack {
                reason: "the stream ended with state still pushed",
            });
        }
        Ok(count)
    }

    /// Where the `index`th record of a fixed-stride section begins, or `None` when there is no such
    /// record.
    fn record(&self, kind: SectionKind, index: ResourceIndex, stride: usize) -> Option<usize> {
        let span = self.span(kind)?;
        let at = (index.index() as usize).checked_mul(stride)?;
        let end = at.checked_add(stride)?;
        if end > span.length {
            return None;
        }
        span.offset.checked_add(at)
    }

    fn span(&self, kind: SectionKind) -> Option<Span> {
        self.sections
            .get(usize::from(kind.wire_value()))
            .copied()
            .flatten()
    }

    fn rect_at(&self, offset: usize) -> Option<SceneRect> {
        Some(SceneRect {
            left: read_f32(&self.bytes, offset)?,
            top: read_f32(&self.bytes, offset + 4)?,
            right: read_f32(&self.bytes, offset + 8)?,
            bottom: read_f32(&self.bytes, offset + 12)?,
        })
    }

    /// The steps of the path occupying `length` bytes from `offset` of the path-data section.
    fn path_steps(&self, offset: usize, length: usize) -> Option<Vec<PathCommand>> {
        let data = self.section_bytes(SectionKind::PathData);
        let end = offset.checked_add(length)?;
        let path = data.get(offset..end)?;
        let mut commands = Vec::new();
        let mut at = 0_usize;
        while at < path.len() {
            let base = at;
            let tag = read_u32(path, base)?;
            let point = |step: usize| -> Option<ScenePoint> {
                Some(ScenePoint {
                    x: read_f32(path, base + 4 + step * 8)?,
                    y: read_f32(path, base + 8 + step * 8)?,
                })
            };
            let (command, size) = match tag {
                path_step::MOVE_TO => (PathCommand::MoveTo(point(0)?), 12),
                path_step::LINE_TO => (PathCommand::LineTo(point(0)?), 12),
                path_step::QUADRATIC_TO => (
                    PathCommand::QuadraticTo {
                        control: point(0)?,
                        end: point(1)?,
                    },
                    20,
                ),
                path_step::CUBIC_TO => (
                    PathCommand::CubicTo {
                        first_control: point(0)?,
                        second_control: point(1)?,
                        end: point(2)?,
                    },
                    28,
                ),
                path_step::CLOSE => (PathCommand::Close, 4),
                _ => return None,
            };
            commands.push(command);
            at = at.checked_add(size)?;
        }
        if at == path.len() {
            Some(commands)
        } else {
            None
        }
    }
}

/// The commands of a display list, in paint order.
///
/// Borrowing rather than owning, and yielding a value rather than a reference: a command is at most
/// 24 bytes and is decoded from the arena on the spot, so walking a page's commands allocates
/// nothing at all.
#[derive(Clone, Debug)]
pub struct Commands<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Iterator for Commands<'_> {
    type Item = Command;

    fn next(&mut self) -> Option<Command> {
        if self.offset >= self.bytes.len() {
            return None;
        }
        let length = usize::from(read_u16(self.bytes, self.offset + 2)?);
        let command = decode_command(self.bytes, self.offset, 0).ok()?;
        self.offset = self.offset.checked_add(length)?;
        Some(command)
    }
}

/// One command out of the stream, with the checks a malformed record has to fail.
fn decode_command(bytes: &[u8], offset: usize, index: usize) -> Result<Command, SceneError> {
    let short = |code: u8, length: usize, reason: &'static str| SceneError::MalformedCommand {
        index,
        opcode: code,
        length,
        reason,
    };
    let code = read_u8(bytes, offset).ok_or_else(|| short(0, 0, "is truncated"))?;
    let length =
        usize::from(read_u16(bytes, offset + 2).ok_or_else(|| short(code, 0, "is truncated"))?);
    if length < 4 || length % 4 != 0 {
        return Err(short(
            code,
            length,
            "is shorter than a record header or is not a whole number of words",
        ));
    }
    if slice_at(bytes, offset, length).is_none() {
        return Err(short(
            code,
            length,
            "ends past the end of the command stream",
        ));
    }
    let word = |step: usize| -> Result<u32, SceneError> {
        read_u32(bytes, offset + 4 + step * 4).ok_or_else(|| short(code, length, "is truncated"))
    };
    let expect = |wanted: usize| -> Result<(), SceneError> {
        if length == wanted {
            Ok(())
        } else {
            Err(short(code, length, "is not the length its opcode defines"))
        }
    };
    Ok(match code {
        opcode::PUSH_TRANSFORM => {
            expect(8)?;
            Command::PushTransform(ResourceIndex::new(word(0)?))
        }
        opcode::PUSH_CLIP => {
            expect(8)?;
            Command::PushClip(ResourceIndex::new(word(0)?))
        }
        opcode::PUSH_OPACITY => {
            expect(8)?;
            Command::PushOpacity(f32::from_bits(word(0)?))
        }
        opcode::PUSH_EFFECT => {
            expect(8)?;
            Command::PushEffect(ResourceIndex::new(word(0)?))
        }
        opcode::POP => {
            expect(4)?;
            Command::Pop
        }
        opcode::FILL_PATH => {
            expect(12)?;
            Command::FillPath {
                geometry: ResourceIndex::new(word(0)?),
                paint: ResourceIndex::new(word(1)?),
            }
        }
        opcode::STROKE_PATH => {
            expect(12)?;
            Command::StrokePath {
                geometry: ResourceIndex::new(word(0)?),
                stroke: ResourceIndex::new(word(1)?),
            }
        }
        opcode::DRAW_GLYPHS => {
            expect(12)?;
            Command::DrawGlyphs {
                run: ResourceIndex::new(word(0)?),
                paint: ResourceIndex::new(word(1)?),
            }
        }
        opcode::DRAW_IMAGE => {
            expect(24)?;
            Command::DrawImage {
                image: ResourceIndex::new(word(0)?),
                destination: SceneRect {
                    left: f32::from_bits(word(1)?),
                    top: f32::from_bits(word(2)?),
                    right: f32::from_bits(word(3)?),
                    bottom: f32::from_bits(word(4)?),
                },
            }
        }
        other => {
            return Err(SceneError::UnknownOpcode {
                index,
                opcode: other,
            })
        }
    })
}

// -------------------------------------------------------------------------------------------
// The small wire vocabularies, shared by the encoder and this decoder. Each one is asserted
// against a hand-written literal in `tests/the_encoding_is_pinned_to_literals.rs`, which is what
// stops the two agreeing with each other and with nothing else.
// -------------------------------------------------------------------------------------------

pub(crate) const PAINT_SOLID: u32 = 0;
pub(crate) const PAINT_GRADIENT: u32 = 1;
pub(crate) const PAINT_PATTERN: u32 = 2;
pub(crate) const PAINT_IMAGE: u32 = 3;

pub(crate) const GRADIENT_LINEAR: u32 = 0;
pub(crate) const GRADIENT_RADIAL: u32 = 1;
pub(crate) const GRADIENT_PATH: u32 = 2;

pub(crate) const PATH_SHADE_SHAPE: u32 = 0;
pub(crate) const PATH_SHADE_CIRCLE: u32 = 1;
pub(crate) const PATH_SHADE_RECTANGLE: u32 = 2;

pub(crate) const GEOMETRY_RECTANGLE: u32 = 0;
pub(crate) const GEOMETRY_PATH: u32 = 1;
pub(crate) const GEOMETRY_UNRESOLVED: u32 = 2;
pub(crate) const GEOMETRY_FLAG_EVEN_ODD: u32 = 1 << 0;

pub(crate) const GLYPH_IMAGE_ATLAS: u8 = 0;
pub(crate) const GLYPH_IMAGE_OUTLINE: u8 = 1;
pub(crate) const GLYPH_IMAGE_BLANK: u8 = 2;

pub(crate) const CAP_FLAT: u8 = 0;
pub(crate) const CAP_ROUND: u8 = 1;
pub(crate) const CAP_SQUARE: u8 = 2;

pub(crate) const JOIN_ROUND: u8 = 0;
pub(crate) const JOIN_BEVEL: u8 = 1;
pub(crate) const JOIN_MITER: u8 = 2;

pub(crate) const ALIGNMENT_CENTERED: u8 = 0;
pub(crate) const ALIGNMENT_INSET: u8 = 1;

/// `flip` occupies bits 1 and 2 of a gradient's or an image's flag word.
pub(crate) const TILE_FLIP_SHIFT: u32 = 1;

pub(crate) const GRADIENT_FLAG_ROTATE_WITH_SHAPE: u32 = 1 << 0;
pub(crate) const GRADIENT_FLAG_SCALED: u32 = 1 << 3;

pub(crate) const EFFECT_FLAG_GROW: u32 = 1 << 0;
pub(crate) const EFFECT_FLAG_ROTATE_WITH_SHAPE: u32 = 1 << 1;
pub(crate) const EFFECT_ANCHOR_SHIFT: u32 = 4;
pub(crate) const EFFECT_BLEND_SHIFT: u32 = 8;

pub(crate) const IMAGE_FLAG_TILE: u32 = 1 << 0;
pub(crate) const IMAGE_FLAG_ROTATE_WITH_SHAPE: u32 = 1 << 3;
pub(crate) const IMAGE_FLAG_GRAYSCALE: u32 = 1 << 4;
pub(crate) const IMAGE_FLAG_DUOTONE: u32 = 1 << 5;
pub(crate) const IMAGE_FLAG_COLOR_CHANGE: u32 = 1 << 6;
pub(crate) const IMAGE_ANCHOR_SHIFT: u32 = 8;

pub(crate) fn tile_flip_wire_value(flip: TileFlip) -> u32 {
    match flip {
        TileFlip::None => 0,
        TileFlip::Horizontal => 1,
        TileFlip::Vertical => 2,
        TileFlip::Both => 3,
    }
}

fn tile_flip_from_flags(flags: u32) -> TileFlip {
    match (flags >> TILE_FLIP_SHIFT) & 0x3 {
        1 => TileFlip::Horizontal,
        2 => TileFlip::Vertical,
        3 => TileFlip::Both,
        _ => TileFlip::None,
    }
}

pub(crate) fn path_shade_wire_value(shade: PathShade) -> u32 {
    match shade {
        PathShade::Shape => PATH_SHADE_SHAPE,
        PathShade::Circle => PATH_SHADE_CIRCLE,
        PathShade::Rectangle => PATH_SHADE_RECTANGLE,
    }
}

pub(crate) fn dash_wire_value(dash: DashPattern) -> u8 {
    match dash {
        DashPattern::Solid => 0,
        DashPattern::Dot => 1,
        DashPattern::Dash => 2,
        DashPattern::LargeDash => 3,
        DashPattern::DashDot => 4,
        DashPattern::LargeDashDot => 5,
        DashPattern::LargeDashDotDot => 6,
        DashPattern::SystemDash => 7,
        DashPattern::SystemDot => 8,
        DashPattern::SystemDashDot => 9,
        DashPattern::SystemDashDotDot => 10,
    }
}

fn dash_from_wire_value(value: u8) -> DashPattern {
    match value {
        1 => DashPattern::Dot,
        2 => DashPattern::Dash,
        3 => DashPattern::LargeDash,
        4 => DashPattern::DashDot,
        5 => DashPattern::LargeDashDot,
        6 => DashPattern::LargeDashDotDot,
        7 => DashPattern::SystemDash,
        8 => DashPattern::SystemDot,
        9 => DashPattern::SystemDashDot,
        10 => DashPattern::SystemDashDotDot,
        _ => DashPattern::Solid,
    }
}

pub(crate) fn compound_wire_value(compound: CompoundStroke) -> u8 {
    match compound {
        CompoundStroke::Single => 0,
        CompoundStroke::Double => 1,
        CompoundStroke::ThickThin => 2,
        CompoundStroke::ThinThick => 3,
        CompoundStroke::Triple => 4,
    }
}

fn compound_from_wire_value(value: u8) -> CompoundStroke {
    match value {
        1 => CompoundStroke::Double,
        2 => CompoundStroke::ThickThin,
        3 => CompoundStroke::ThinThick,
        4 => CompoundStroke::Triple,
        _ => CompoundStroke::Single,
    }
}

pub(crate) fn line_end_shape_wire_value(shape: LineEndShape) -> u8 {
    match shape {
        LineEndShape::None => 0,
        LineEndShape::Triangle => 1,
        LineEndShape::Stealth => 2,
        LineEndShape::Diamond => 3,
        LineEndShape::Oval => 4,
        LineEndShape::Arrow => 5,
    }
}

fn line_end_shape_from_wire_value(value: u8) -> LineEndShape {
    match value {
        1 => LineEndShape::Triangle,
        2 => LineEndShape::Stealth,
        3 => LineEndShape::Diamond,
        4 => LineEndShape::Oval,
        5 => LineEndShape::Arrow,
        _ => LineEndShape::None,
    }
}

pub(crate) fn line_end_size_wire_value(size: LineEndSize) -> u8 {
    match size {
        LineEndSize::Small => 0,
        LineEndSize::Medium => 1,
        LineEndSize::Large => 2,
    }
}

fn line_end_size_from_wire_value(value: u8) -> LineEndSize {
    match value {
        0 => LineEndSize::Small,
        2 => LineEndSize::Large,
        _ => LineEndSize::Medium,
    }
}

pub(crate) fn anchor_wire_value(anchor: RectangleAnchor) -> u32 {
    match anchor {
        RectangleAnchor::TopLeft => 0,
        RectangleAnchor::Top => 1,
        RectangleAnchor::TopRight => 2,
        RectangleAnchor::Left => 3,
        RectangleAnchor::Center => 4,
        RectangleAnchor::Right => 5,
        RectangleAnchor::BottomLeft => 6,
        RectangleAnchor::Bottom => 7,
        RectangleAnchor::BottomRight => 8,
    }
}

fn anchor_from_wire_value(value: u32) -> RectangleAnchor {
    match value {
        1 => RectangleAnchor::Top,
        2 => RectangleAnchor::TopRight,
        3 => RectangleAnchor::Left,
        4 => RectangleAnchor::Center,
        5 => RectangleAnchor::Right,
        6 => RectangleAnchor::BottomLeft,
        7 => RectangleAnchor::Bottom,
        8 => RectangleAnchor::BottomRight,
        _ => RectangleAnchor::TopLeft,
    }
}

pub(crate) fn blend_wire_value(blend: BlendMode) -> u32 {
    match blend {
        BlendMode::Over => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Darken => 3,
        BlendMode::Lighten => 4,
    }
}

fn blend_from_wire_value(value: u32) -> BlendMode {
    match value {
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Darken,
        4 => BlendMode::Lighten,
        _ => BlendMode::Over,
    }
}
