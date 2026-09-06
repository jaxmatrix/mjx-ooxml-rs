//! [`SceneBuilder`] — the encoder, and the only thing that writes a display list's bytes.
//!
//! # Interning is not an optimisation here, it is the point
//!
//! A slide with four hundred shapes in the theme's accent colour has one paint. A page of prose has
//! one transform, one clip and one paint. Every `add_*` below looks the record up by its own bytes
//! before appending it, so a repeated fill costs an integer in the command stream and nothing else —
//! which is what `docs/UI_PLATFORM_PLAN.md` §4 L4 means by *"addressed by index so a repeated fill
//! costs an integer"*.
//!
//! Two tables intern on more than their own record. A gradient's bytes name a *range* of the stop
//! table, so two identical gradients would look different if they were compared after the range was
//! assigned; they are compared before it, on the record with the range zeroed followed by the stops
//! themselves. A path geometry is the same, with its path data.
//!
//! # The builder validates its own output
//!
//! [`SceneBuilder::finish`] hands its bytes to [`DisplayList::from_bytes`] rather than constructing
//! a `DisplayList` directly, so the list a builder produces is a list that passed every check a list
//! off a disk passes. A builder that could emit something the decoder refuses would be a builder
//! whose output nobody could read back, and the failure would surface a frame later in a painter
//! rather than here.

use std::collections::HashMap;

use mjx_text::DeviceScale;

use crate::command::{Clip, Command};
use crate::effect::{Effect, EffectStyle};
use crate::encoding::{
    opcode, pack_color, path_step, write_f32, write_i16, write_i32, write_u16, write_u32,
    ResourceIndex, SectionKind, CLIP_STRIDE, EFFECT_STRIDE, GEOMETRY_STRIDE, GRADIENT_STOP_STRIDE,
    GRADIENT_STRIDE, HEADER_BYTES, IMAGE_STRIDE, MAGIC, PAINT_STRIDE, SECTION_ROW_BYTES,
    STROKE_STRIDE, TRANSFORM_STRIDE, VERSION,
};
use crate::error::SceneError;
use crate::geometry::{finite, FillRule, Geometry, PathCommand, SceneRect, SceneTransform};
use crate::glyphs::{
    direction_wire_value, format_wire_value, hinting_wire_value, GlyphImage, SceneGlyphRun,
};
use mjx_tokens::Color;

use crate::list::{
    anchor_wire_value, blend_wire_value, compound_wire_value, dash_wire_value,
    line_end_shape_wire_value, line_end_size_wire_value, path_shade_wire_value,
    tile_flip_wire_value, ALIGNMENT_CENTERED, ALIGNMENT_INSET, CAP_FLAT, CAP_ROUND, CAP_SQUARE,
    EFFECT_ANCHOR_SHIFT, EFFECT_BLEND_SHIFT, EFFECT_FLAG_GROW, EFFECT_FLAG_ROTATE_WITH_SHAPE,
    GEOMETRY_FLAG_EVEN_ODD, GEOMETRY_PATH, GEOMETRY_RECTANGLE, GEOMETRY_UNRESOLVED,
    GLYPH_IMAGE_ATLAS, GLYPH_IMAGE_BLANK, GLYPH_IMAGE_OUTLINE, GRADIENT_FLAG_ROTATE_WITH_SHAPE,
    GRADIENT_FLAG_SCALED, GRADIENT_LINEAR, GRADIENT_PATH, GRADIENT_RADIAL, IMAGE_ANCHOR_SHIFT,
    IMAGE_FLAG_COLOR_CHANGE, IMAGE_FLAG_DUOTONE, IMAGE_FLAG_GRAYSCALE,
    IMAGE_FLAG_ROTATE_WITH_SHAPE, IMAGE_FLAG_TILE, JOIN_BEVEL, JOIN_MITER, JOIN_ROUND,
    PAINT_GRADIENT, PAINT_IMAGE, PAINT_PATTERN, PAINT_SOLID, TILE_FLIP_SHIFT,
};
use crate::paint::{
    FillStyle, Gradient, GradientKind, Image, LineCap, LineJoin, Paint, Stroke, StrokeAlignment,
    StrokeStyle,
};
use crate::DisplayList;

/// The colour written where a record has a colour field an absent adjustment does not fill.
const TRANSPARENT: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 0,
};

/// One fixed-stride table, with the interner that makes a repeat cost an integer.
#[derive(Debug)]
struct Table {
    kind: SectionKind,
    stride: usize,
    bytes: Vec<u8>,
    seen: HashMap<Box<[u8]>, u32>,
}

impl Table {
    fn new(kind: SectionKind, stride: usize) -> Self {
        Self {
            kind,
            stride,
            bytes: Vec::new(),
            seen: HashMap::new(),
        }
    }

    fn count(&self) -> u32 {
        // Every record is exactly `stride` bytes and the count is bounded by `insert`.
        u32::try_from(self.bytes.len() / self.stride.max(1)).unwrap_or(u32::MAX)
    }

    fn lookup(&self, key: &[u8]) -> Option<ResourceIndex> {
        self.seen.get(key).copied().map(ResourceIndex::new)
    }

    fn insert(&mut self, key: Vec<u8>, record: &[u8]) -> Result<ResourceIndex, SceneError> {
        let count = self.bytes.len() / self.stride.max(1);
        if count >= u32::MAX as usize - 1 {
            return Err(SceneError::TableFull {
                section: self.kind,
                count,
            });
        }
        self.bytes.extend_from_slice(record);
        // `count` was just bounded below `u32::MAX`, so the cast is exact.
        let index = count as u32;
        self.seen.insert(key.into_boxed_slice(), index);
        Ok(ResourceIndex::new(index))
    }

    /// Append `record`, or return the index of an identical one.
    fn intern(&mut self, record: &[u8]) -> Result<ResourceIndex, SceneError> {
        if let Some(existing) = self.lookup(record) {
            return Ok(existing);
        }
        self.insert(record.to_vec(), record)
    }
}

/// Builds one display list, one record at a time.
///
/// A builder rather than a constructor for the same reason [`mjx_layout::FragmentTreeBuilder`] is
/// one: a scene is produced as a tree is walked, nobody knows how many records there will be, and
/// appending to a flat vector is the one shape that stays a single allocation while it grows.
#[derive(Debug)]
pub struct SceneBuilder {
    device_scale: DeviceScale,
    page_width: f32,
    page_height: f32,
    commands: Vec<u8>,
    command_count: usize,
    depth: usize,
    transforms: Table,
    clips: Table,
    paints: Table,
    gradients: Table,
    gradient_stops: Vec<u8>,
    strokes: Table,
    effects: Table,
    geometries: Table,
    path_data: Vec<u8>,
    glyph_runs: Vec<u8>,
    glyph_run_count: u32,
    glyphs: Vec<u8>,
    glyph_count: u32,
    images: Table,
}

impl SceneBuilder {
    /// An empty builder for a page `width` by `height` device pixels at `device_scale`.
    #[must_use]
    pub fn new(device_scale: DeviceScale, width: f32, height: f32) -> Self {
        Self {
            device_scale,
            page_width: finite(width),
            page_height: finite(height),
            commands: Vec::new(),
            command_count: 0,
            depth: 0,
            transforms: Table::new(SectionKind::Transforms, TRANSFORM_STRIDE),
            clips: Table::new(SectionKind::Clips, CLIP_STRIDE),
            paints: Table::new(SectionKind::Paints, PAINT_STRIDE),
            gradients: Table::new(SectionKind::Gradients, GRADIENT_STRIDE),
            gradient_stops: Vec::new(),
            strokes: Table::new(SectionKind::Strokes, STROKE_STRIDE),
            effects: Table::new(SectionKind::Effects, EFFECT_STRIDE),
            geometries: Table::new(SectionKind::Geometries, GEOMETRY_STRIDE),
            path_data: Vec::new(),
            glyph_runs: Vec::new(),
            glyph_run_count: 0,
            glyphs: Vec::new(),
            glyph_count: 0,
            images: Table::new(SectionKind::Images, IMAGE_STRIDE),
        }
    }

    /// How many commands have been pushed.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.command_count
    }

    /// How deep the push/pop stack currently is.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Append a command.
    ///
    /// # Errors
    ///
    /// [`SceneError::UnbalancedStack`] for a [`Command::Pop`] with nothing pushed. Everything else a
    /// command can be wrong about — an index naming no record — is caught by [`SceneBuilder::finish`],
    /// because a command may legitimately name a resource that has not been added yet.
    pub fn push(&mut self, command: Command) -> Result<(), SceneError> {
        if command == Command::Pop {
            self.depth = self
                .depth
                .checked_sub(1)
                .ok_or(SceneError::UnbalancedStack {
                    reason: "a `Pop` arrived with nothing pushed",
                })?;
        } else if command.pushes_state() {
            self.depth += 1;
        }
        let length = command.record_bytes();
        self.commands.push(command.opcode());
        self.commands.push(0);
        // Every record is at most 24 bytes, far below what a `u16` holds.
        write_u16(&mut self.commands, length as u16);
        match command {
            Command::Pop => {}
            Command::PushTransform(id) | Command::PushClip(id) | Command::PushEffect(id) => {
                write_u32(&mut self.commands, id.index());
            }
            Command::PushOpacity(opacity) => write_f32(&mut self.commands, opacity),
            Command::FillPath { geometry, paint } => {
                write_u32(&mut self.commands, geometry.index());
                write_u32(&mut self.commands, paint.index());
            }
            Command::StrokePath { geometry, stroke } => {
                write_u32(&mut self.commands, geometry.index());
                write_u32(&mut self.commands, stroke.index());
            }
            Command::DrawGlyphs { run, paint } => {
                write_u32(&mut self.commands, run.index());
                write_u32(&mut self.commands, paint.index());
            }
            Command::DrawImage { image, destination } => {
                write_u32(&mut self.commands, image.index());
                write_rect(&mut self.commands, destination);
            }
        }
        self.command_count += 1;
        Ok(())
    }

    /// Add an affine map, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when the table cannot grow.
    pub fn add_transform(
        &mut self,
        transform: SceneTransform,
    ) -> Result<ResourceIndex, SceneError> {
        let mut record = Vec::with_capacity(TRANSFORM_STRIDE);
        write_f32(&mut record, transform.scale_x);
        write_f32(&mut record, transform.shear_y);
        write_f32(&mut record, transform.shear_x);
        write_f32(&mut record, transform.scale_y);
        write_f32(&mut record, transform.translate_x);
        write_f32(&mut record, transform.translate_y);
        self.transforms.intern(&record)
    }

    /// Add a clip region, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when the table cannot grow.
    pub fn add_clip(&mut self, clip: Clip) -> Result<ResourceIndex, SceneError> {
        let mut record = Vec::with_capacity(CLIP_STRIDE);
        write_u32(&mut record, u32::from(clip.geometry.is_some()));
        write_u32(
            &mut record,
            clip.geometry
                .map_or(ResourceIndex::NONE, ResourceIndex::index),
        );
        write_rect(&mut record, clip.bounds);
        self.clips.intern(&record)
    }

    /// Add a paint record, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when the table cannot grow.
    pub fn add_paint(&mut self, paint: Paint) -> Result<ResourceIndex, SceneError> {
        let mut record = Vec::with_capacity(PAINT_STRIDE);
        match paint {
            Paint::Solid(color) => {
                write_u32(&mut record, PAINT_SOLID);
                write_u32(&mut record, pack_color(color));
                write_u32(&mut record, 0);
                write_u32(&mut record, 0);
            }
            Paint::Gradient(gradient) => {
                write_u32(&mut record, PAINT_GRADIENT);
                write_u32(&mut record, gradient.index());
                write_u32(&mut record, 0);
                write_u32(&mut record, 0);
            }
            Paint::Pattern {
                preset,
                foreground,
                background,
            } => {
                write_u32(&mut record, PAINT_PATTERN);
                write_u32(&mut record, preset.wire_value());
                write_u32(&mut record, pack_color(foreground));
                write_u32(&mut record, pack_color(background));
            }
            Paint::Image(image) => {
                write_u32(&mut record, PAINT_IMAGE);
                write_u32(&mut record, image.index());
                write_u32(&mut record, 0);
                write_u32(&mut record, 0);
            }
        }
        self.paints.intern(&record)
    }

    /// Add a gradient and its stops, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when either table cannot grow.
    pub fn add_gradient(&mut self, gradient: &Gradient) -> Result<ResourceIndex, SceneError> {
        let mut flags = tile_flip_wire_value(gradient.flip) << TILE_FLIP_SHIFT;
        if gradient.rotate_with_shape {
            flags |= GRADIENT_FLAG_ROTATE_WITH_SHAPE;
        }
        if gradient.angle_is_scaled {
            flags |= GRADIENT_FLAG_SCALED;
        }
        let mut stops = Vec::with_capacity(gradient.stops.len() * GRADIENT_STOP_STRIDE);
        for stop in &gradient.stops {
            write_u16(&mut stops, stop.position_in_ten_thousandths);
            write_u16(&mut stops, 0);
            write_u32(&mut stops, pack_color(stop.color));
        }

        let mut record = Vec::with_capacity(GRADIENT_STRIDE);
        write_u32(
            &mut record,
            match gradient.kind {
                GradientKind::Linear => GRADIENT_LINEAR,
                GradientKind::Radial => GRADIENT_RADIAL,
                GradientKind::Path => GRADIENT_PATH,
            },
        );
        // Zero where the stop range goes, so that the interning key is the gradient's *meaning*
        // rather than where its stops happened to land.
        write_u32(&mut record, 0);
        write_u32(&mut record, 0);
        write_u32(&mut record, flags);
        write_f32(&mut record, gradient.angle);
        write_u32(&mut record, path_shade_wire_value(gradient.path_shade));
        write_rect(&mut record, gradient.focus);
        write_rect(&mut record, gradient.tile);

        let mut key = record.clone();
        key.extend_from_slice(&stops);
        if let Some(existing) = self.gradients.lookup(&key) {
            return Ok(existing);
        }

        let first_stop =
            u32::try_from(self.gradient_stops.len() / GRADIENT_STOP_STRIDE).map_err(|_| {
                SceneError::TableFull {
                    section: SectionKind::GradientStops,
                    count: self.gradient_stops.len() / GRADIENT_STOP_STRIDE,
                }
            })?;
        let count = u32::try_from(gradient.stops.len()).map_err(|_| SceneError::TableFull {
            section: SectionKind::GradientStops,
            count: gradient.stops.len(),
        })?;
        self.gradient_stops.extend_from_slice(&stops);
        patch_u32(&mut record, 4, first_stop);
        patch_u32(&mut record, 8, count);
        self.gradients.insert(key, &record)
    }

    /// Add a stroke record, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when the table cannot grow.
    pub fn add_stroke(&mut self, stroke: Stroke) -> Result<ResourceIndex, SceneError> {
        let (join, miter_limit) = match stroke.join {
            LineJoin::Round => (JOIN_ROUND, 0.0),
            LineJoin::Bevel => (JOIN_BEVEL, 0.0),
            LineJoin::Miter { limit } => (JOIN_MITER, limit),
        };
        let mut record = Vec::with_capacity(STROKE_STRIDE);
        write_u32(&mut record, stroke.paint.index());
        write_f32(&mut record, stroke.width);
        write_f32(&mut record, miter_limit);
        record.push(match stroke.cap {
            LineCap::Flat => CAP_FLAT,
            LineCap::Round => CAP_ROUND,
            LineCap::Square => CAP_SQUARE,
        });
        record.push(join);
        record.push(dash_wire_value(stroke.dash));
        record.push(match stroke.alignment {
            StrokeAlignment::Centered => ALIGNMENT_CENTERED,
            StrokeAlignment::Inset => ALIGNMENT_INSET,
        });
        record.push(compound_wire_value(stroke.compound));
        record.push(line_end_shape_wire_value(stroke.head.shape));
        record.push(line_end_size_wire_value(stroke.head.width));
        record.push(line_end_size_wire_value(stroke.head.length));
        record.push(line_end_shape_wire_value(stroke.tail.shape));
        record.push(line_end_size_wire_value(stroke.tail.width));
        record.push(line_end_size_wire_value(stroke.tail.length));
        record.push(0);
        self.strokes.intern(&record)
    }

    /// Add one node of an effect DAG, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::EffectCycle`] when the node names an input that is not already in the table —
    /// which is what makes the table topological — and [`SceneError::TableFull`] when it cannot
    /// grow.
    pub fn add_effect(&mut self, effect: Effect) -> Result<ResourceIndex, SceneError> {
        let next = self.effects.count();
        if let Some(input) = effect.input {
            if input.index() >= next {
                return Err(SceneError::EffectCycle {
                    index: next,
                    input: input.index(),
                });
            }
        }
        let mut flags = anchor_wire_value(effect.anchor) << EFFECT_ANCHOR_SHIFT
            | blend_wire_value(effect.blend) << EFFECT_BLEND_SHIFT;
        if effect.grow {
            flags |= EFFECT_FLAG_GROW;
        }
        if effect.rotate_with_shape {
            flags |= EFFECT_FLAG_ROTATE_WITH_SHAPE;
        }
        let mut record = Vec::with_capacity(EFFECT_STRIDE);
        write_u32(&mut record, effect.kind.wire_value());
        write_u32(
            &mut record,
            effect
                .input
                .map_or(ResourceIndex::NONE, ResourceIndex::index),
        );
        write_u32(
            &mut record,
            effect
                .paint
                .map_or(ResourceIndex::NONE, ResourceIndex::index),
        );
        for value in [
            effect.radius,
            effect.distance,
            effect.direction,
            effect.scale_x,
            effect.scale_y,
            effect.skew_x,
            effect.skew_y,
            effect.start_alpha,
            effect.start_position,
            effect.end_alpha,
            effect.end_position,
            effect.fade_direction,
        ] {
            write_f32(&mut record, value);
        }
        write_u32(&mut record, flags);
        self.effects.intern(&record)
    }

    /// Add a geometry and, for a path, its steps — or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when either table cannot grow.
    pub fn add_geometry(&mut self, geometry: &Geometry) -> Result<ResourceIndex, SceneError> {
        let mut path = Vec::new();
        let (kind, first, second, flags, bounds) = match geometry {
            Geometry::Rectangle(rect) => (GEOMETRY_RECTANGLE, 0, 0, 0, *rect),
            Geometry::Path {
                commands,
                fill_rule,
                bounds,
            } => {
                for command in commands {
                    write_path_step(&mut path, *command);
                }
                let flags = match fill_rule {
                    FillRule::NonZero => 0,
                    FillRule::EvenOdd => GEOMETRY_FLAG_EVEN_ODD,
                };
                (GEOMETRY_PATH, 0, 0, flags, *bounds)
            }
            Geometry::Unresolved { outline, bounds } => (
                GEOMETRY_UNRESOLVED,
                (*outline & 0xffff_ffff) as u32,
                (*outline >> 32) as u32,
                0,
                *bounds,
            ),
        };

        let mut record = Vec::with_capacity(GEOMETRY_STRIDE);
        write_u32(&mut record, kind);
        write_u32(&mut record, first);
        write_u32(&mut record, second);
        write_u32(&mut record, flags);
        write_rect(&mut record, bounds);

        let mut key = record.clone();
        key.extend_from_slice(&path);
        if let Some(existing) = self.geometries.lookup(&key) {
            return Ok(existing);
        }
        if kind == GEOMETRY_PATH {
            let offset =
                u32::try_from(self.path_data.len()).map_err(|_| SceneError::TableFull {
                    section: SectionKind::PathData,
                    count: self.path_data.len(),
                })?;
            let length = u32::try_from(path.len()).map_err(|_| SceneError::TableFull {
                section: SectionKind::PathData,
                count: path.len(),
            })?;
            self.path_data.extend_from_slice(&path);
            patch_u32(&mut record, 4, offset);
            patch_u32(&mut record, 8, length);
        }
        self.geometries.insert(key, &record)
    }

    /// Add a glyph run and its glyphs.
    ///
    /// Not interned: two runs of the same word at different origins differ in their record and two
    /// at the *same* origin are the same draw twice, which no fragment tree produces. A hash of a
    /// hundred glyphs to discover that would cost more than the record it saved.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when either table cannot grow.
    pub fn add_glyph_run(&mut self, run: &SceneGlyphRun) -> Result<ResourceIndex, SceneError> {
        let first_glyph = self.glyph_count;
        let count = u32::try_from(run.glyphs.len()).map_err(|_| SceneError::TableFull {
            section: SectionKind::Glyphs,
            count: run.glyphs.len(),
        })?;
        if first_glyph.checked_add(count).is_none() {
            return Err(SceneError::TableFull {
                section: SectionKind::Glyphs,
                count: first_glyph as usize,
            });
        }
        for glyph in &run.glyphs {
            write_i32(&mut self.glyphs, glyph.x);
            write_i32(&mut self.glyphs, glyph.y);
            write_u32(&mut self.glyphs, glyph.cluster);
            write_u16(&mut self.glyphs, glyph.glyph);
            self.glyphs.push(glyph.subpixel);
            match glyph.image {
                GlyphImage::Atlas(entry) => {
                    self.glyphs.push(GLYPH_IMAGE_ATLAS);
                    write_u32(&mut self.glyphs, entry.page);
                    write_u16(&mut self.glyphs, entry.x);
                    write_u16(&mut self.glyphs, entry.y);
                    write_u16(&mut self.glyphs, entry.width);
                    write_u16(&mut self.glyphs, entry.height);
                    write_i16(&mut self.glyphs, entry.offset_from_origin_x);
                    write_i16(&mut self.glyphs, entry.offset_from_origin_y);
                    write_u32(&mut self.glyphs, format_wire_value(entry.format));
                }
                GlyphImage::Outline(geometry) => {
                    self.glyphs.push(GLYPH_IMAGE_OUTLINE);
                    write_u32(&mut self.glyphs, geometry.index());
                    self.glyphs.extend_from_slice(&[0; 16]);
                }
                GlyphImage::Blank => {
                    self.glyphs.push(GLYPH_IMAGE_BLANK);
                    self.glyphs.extend_from_slice(&[0; 20]);
                }
            }
        }
        self.glyph_count += count;

        let mut flags = (direction_wire_value(run.direction) & 0x1)
            | (hinting_wire_value(run.hinting) & 0x1) << 1
            | u32::from(run.level) << 8;
        flags &= 0xffff;
        write_u32(&mut self.glyph_runs, run.face);
        write_u32(&mut self.glyph_runs, first_glyph);
        write_u32(&mut self.glyph_runs, count);
        write_u32(&mut self.glyph_runs, run.bucket_steps);
        write_f32(&mut self.glyph_runs, run.residual_scale);
        write_f32(&mut self.glyph_runs, run.origin.x);
        write_f32(&mut self.glyph_runs, run.origin.y);
        write_u32(&mut self.glyph_runs, flags);
        let index = self.glyph_run_count;
        self.glyph_run_count = index.checked_add(1).ok_or(SceneError::TableFull {
            section: SectionKind::GlyphRuns,
            count: index as usize,
        })?;
        Ok(ResourceIndex::new(index))
    }

    /// Add a picture, or find the identical one already there.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when the table cannot grow.
    pub fn add_image(&mut self, image: Image) -> Result<ResourceIndex, SceneError> {
        let mut flags = tile_flip_wire_value(image.flip) << TILE_FLIP_SHIFT
            | anchor_wire_value(image.anchor) << IMAGE_ANCHOR_SHIFT;
        if matches!(image.fill_mode, crate::paint::ImageFillMode::Tile) {
            flags |= IMAGE_FLAG_TILE;
        }
        if image.rotate_with_shape {
            flags |= IMAGE_FLAG_ROTATE_WITH_SHAPE;
        }
        if image.adjustments.grayscale {
            flags |= IMAGE_FLAG_GRAYSCALE;
        }
        if image.adjustments.duotone.is_some() {
            flags |= IMAGE_FLAG_DUOTONE;
        }
        if image.adjustments.color_change.is_some() {
            flags |= IMAGE_FLAG_COLOR_CHANGE;
        }
        // A pair the flag word says is absent is written as two transparent blacks rather than
        // left uninitialised, so that two images that differ only in an absent adjustment still
        // intern to one record.
        const ABSENT: (Color, Color) = (TRANSPARENT, TRANSPARENT);
        let duotone = image.adjustments.duotone.unwrap_or(ABSENT);
        let color_change = image.adjustments.color_change.unwrap_or(ABSENT);

        let mut record = Vec::with_capacity(IMAGE_STRIDE);
        write_u32(&mut record, (image.handle & 0xffff_ffff) as u32);
        write_u32(&mut record, (image.handle >> 32) as u32);
        write_u32(&mut record, flags);
        write_rect(&mut record, image.crop);
        write_f32(&mut record, image.tile_offset.x);
        write_f32(&mut record, image.tile_offset.y);
        write_f32(&mut record, image.tile_scale_x);
        write_f32(&mut record, image.tile_scale_y);
        write_u16(&mut record, image.adjustments.alpha_in_ten_thousandths);
        write_u16(&mut record, 0);
        write_i16(&mut record, image.adjustments.brightness_in_ten_thousandths);
        write_i16(&mut record, image.adjustments.contrast_in_ten_thousandths);
        write_u32(&mut record, 0);
        write_u32(&mut record, pack_color(duotone.0));
        write_u32(&mut record, pack_color(duotone.1));
        write_u32(&mut record, pack_color(color_change.0));
        write_u32(&mut record, pack_color(color_change.1));
        self.images.intern(&record)
    }

    /// Add whatever a [`FillStyle`] needs — a gradient and its stops, or a picture — and return the
    /// paint that names it. `None` for [`FillStyle::None`], which paints nothing.
    ///
    /// This is the bridge between the vocabulary a caller writes and the tables a list stores, and
    /// it is why a [`crate::ResourceResolver`] can be written without ever seeing the builder.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when a table cannot grow.
    pub fn add_fill_style(
        &mut self,
        fill: &FillStyle,
    ) -> Result<Option<ResourceIndex>, SceneError> {
        Ok(Some(match fill {
            FillStyle::None => return Ok(None),
            FillStyle::Solid(color) => self.add_paint(Paint::Solid(*color))?,
            FillStyle::Gradient(gradient) => {
                let index = self.add_gradient(gradient)?;
                self.add_paint(Paint::Gradient(index))?
            }
            FillStyle::Pattern {
                preset,
                foreground,
                background,
            } => self.add_paint(Paint::Pattern {
                preset: *preset,
                foreground: *foreground,
                background: *background,
            })?,
            FillStyle::Image(image) => {
                let index = self.add_image(*image)?;
                self.add_paint(Paint::Image(index))?
            }
        }))
    }

    /// Add a stroke described by value, interning its fill first. `None` when the stroke has no
    /// fill and would therefore draw nothing.
    ///
    /// # Errors
    ///
    /// [`SceneError::TableFull`] when a table cannot grow.
    pub fn add_stroke_style(
        &mut self,
        stroke: &StrokeStyle,
    ) -> Result<Option<ResourceIndex>, SceneError> {
        let Some(paint) = self.add_fill_style(&stroke.fill)? else {
            return Ok(None);
        };
        self.add_stroke(Stroke {
            paint,
            width: stroke.width,
            cap: stroke.cap,
            join: stroke.join,
            dash: stroke.dash,
            alignment: stroke.alignment,
            compound: stroke.compound,
            head: stroke.head,
            tail: stroke.tail,
        })
        .map(Some)
    }

    /// Add a whole effect DAG and return the index of its **root**, which is the last entry of
    /// `effects`. `None` for an empty list.
    ///
    /// Each entry's `input` is a position within `effects` and must be below that entry's own,
    /// which is what makes the result a DAG in topological order.
    ///
    /// # Errors
    ///
    /// [`SceneError::MalformedEffectChain`] for an input that is not below its own entry, and
    /// [`SceneError::TableFull`] when a table cannot grow.
    pub fn add_effect_styles(
        &mut self,
        effects: &[EffectStyle],
    ) -> Result<Option<ResourceIndex>, SceneError> {
        let mut placed: Vec<ResourceIndex> = Vec::with_capacity(effects.len());
        for (position, style) in effects.iter().enumerate() {
            let input = match style.input {
                None => None,
                Some(named) => {
                    if named >= position {
                        return Err(SceneError::MalformedEffectChain {
                            index: position,
                            input: named,
                        });
                    }
                    placed.get(named).copied()
                }
            };
            let paint = self.add_fill_style(&style.fill)?;
            let index = self.add_effect(Effect {
                kind: style.kind,
                input,
                paint,
                radius: style.radius,
                distance: style.distance,
                direction: style.direction,
                scale_x: style.scale_x,
                scale_y: style.scale_y,
                skew_x: style.skew_x,
                skew_y: style.skew_y,
                start_alpha: style.start_alpha,
                start_position: style.start_position,
                end_alpha: style.end_alpha,
                end_position: style.end_position,
                fade_direction: style.fade_direction,
                grow: style.grow,
                rotate_with_shape: style.rotate_with_shape,
                anchor: style.anchor,
                blend: style.blend,
            })?;
            placed.push(index);
        }
        Ok(placed.last().copied())
    }

    /// Assemble the bytes and validate them.
    ///
    /// # Errors
    ///
    /// [`SceneError::UnbalancedStack`] when something is still pushed, and whatever
    /// [`DisplayList::from_bytes`] rejects — which, for a builder that has been used correctly, is
    /// only an index naming a record that was never added.
    pub fn finish(self) -> Result<DisplayList, SceneError> {
        if self.depth != 0 {
            return Err(SceneError::UnbalancedStack {
                reason: "the scene ended with state still pushed",
            });
        }
        let sections: Vec<(SectionKind, &[u8])> = vec![
            (SectionKind::Commands, &self.commands),
            (SectionKind::Transforms, &self.transforms.bytes),
            (SectionKind::Clips, &self.clips.bytes),
            (SectionKind::Paints, &self.paints.bytes),
            (SectionKind::Gradients, &self.gradients.bytes),
            (SectionKind::GradientStops, &self.gradient_stops),
            (SectionKind::Strokes, &self.strokes.bytes),
            (SectionKind::Effects, &self.effects.bytes),
            (SectionKind::Geometries, &self.geometries.bytes),
            (SectionKind::PathData, &self.path_data),
            (SectionKind::GlyphRuns, &self.glyph_runs),
            (SectionKind::Glyphs, &self.glyphs),
            (SectionKind::Images, &self.images.bytes),
        ];
        let present: Vec<(SectionKind, &[u8])> = sections
            .into_iter()
            .filter(|(_, bytes)| !bytes.is_empty())
            .collect();

        let table_bytes = HEADER_BYTES + present.len() * SECTION_ROW_BYTES;
        let total = present
            .iter()
            .try_fold(table_bytes, |sum, (_, bytes)| sum.checked_add(bytes.len()))
            .and_then(|total| u32::try_from(total).ok())
            .ok_or(SceneError::TableFull {
                section: SectionKind::Commands,
                count: table_bytes,
            })?;

        let mut blob = Vec::with_capacity(total as usize);
        blob.extend_from_slice(&MAGIC);
        write_u16(&mut blob, VERSION);
        // The header length and the section count are both far below what a `u16` holds.
        write_u16(&mut blob, HEADER_BYTES as u16);
        write_f32(&mut blob, self.device_scale.pixels_per_point());
        write_f32(&mut blob, self.page_width);
        write_f32(&mut blob, self.page_height);
        write_u16(&mut blob, present.len() as u16);
        write_u16(&mut blob, 0);
        write_u32(&mut blob, total);
        write_u32(&mut blob, 0);

        let mut offset = table_bytes;
        for (kind, bytes) in &present {
            write_u16(&mut blob, kind.wire_value());
            // Every stride is far below what a `u16` holds, and a variable section writes zero.
            write_u16(&mut blob, kind.stride().unwrap_or(0) as u16);
            write_u32(&mut blob, offset as u32);
            write_u32(&mut blob, bytes.len() as u32);
            offset += bytes.len();
        }
        for (_, bytes) in &present {
            blob.extend_from_slice(bytes);
        }

        DisplayList::from_bytes(blob)
    }
}

/// Append a rectangle: four `f32`, left, top, right, bottom.
fn write_rect(into: &mut Vec<u8>, rect: SceneRect) {
    write_f32(into, rect.left);
    write_f32(into, rect.top);
    write_f32(into, rect.right);
    write_f32(into, rect.bottom);
}

/// Append one path step: a tag word and its points.
fn write_path_step(into: &mut Vec<u8>, command: PathCommand) {
    match command {
        PathCommand::MoveTo(point) => {
            write_u32(into, path_step::MOVE_TO);
            write_f32(into, point.x);
            write_f32(into, point.y);
        }
        PathCommand::LineTo(point) => {
            write_u32(into, path_step::LINE_TO);
            write_f32(into, point.x);
            write_f32(into, point.y);
        }
        PathCommand::QuadraticTo { control, end } => {
            write_u32(into, path_step::QUADRATIC_TO);
            write_f32(into, control.x);
            write_f32(into, control.y);
            write_f32(into, end.x);
            write_f32(into, end.y);
        }
        PathCommand::CubicTo {
            first_control,
            second_control,
            end,
        } => {
            write_u32(into, path_step::CUBIC_TO);
            write_f32(into, first_control.x);
            write_f32(into, first_control.y);
            write_f32(into, second_control.x);
            write_f32(into, second_control.y);
            write_f32(into, end.x);
            write_f32(into, end.y);
        }
        PathCommand::Close => write_u32(into, path_step::CLOSE),
    }
}

/// Overwrite the little-endian `u32` at `offset` of a record being built.
///
/// Written as a `get_mut` and a `copy_from_slice` rather than an index, because there is no
/// `unwrap` and no slice index anywhere in this crate — not even where the offset is a constant this
/// file wrote itself.
fn patch_u32(record: &mut [u8], offset: usize, value: u32) {
    if let Some(field) = record.get_mut(offset..offset + 4) {
        field.copy_from_slice(&value.to_le_bytes());
    }
}

/// Append opcodes into a command stream; the `opcode` module is used by [`Command::opcode`] and is
/// named here so the module's constants have a reader in this file too.
const _: u8 = opcode::POP;
