//! The processor's copy of `backend/shaders.wgsl`, function for function.
//!
//! # Why this is a copy and not a different idea
//!
//! A second painter exists to check the first. That only works if the two disagree about **one**
//! thing — how a shape becomes coverage — and agree about everything else. If the software painter
//! also resolved gradients differently, sampled hatches differently, or adjusted pictures
//! differently, a cross-painter difference would say nothing about either rasteriser and the whole
//! exercise would be a comparison of two shaders.
//!
//! So every function below is the WGSL one written in Rust, with the same clamping, the same
//! ordering and the same filtering:
//!
//! * `gradient_at` is [`crate::plan::GradientMapping::at`] — literally the same call, because R08
//!   already wrote the shader's arithmetic once in Rust as its executable specification;
//! * `pattern_at` is [`crate::pattern::cell_of`], the same fifty-four-entry table;
//! * a picture's `adjust` is the same additive brightness, the same multiplicative contrast about
//!   mid grey and the same Rec. 709 luma;
//! * sampling is **bilinear with clamp-to-edge**, because that is the sampler the shader binds. A
//!   nearest-neighbour software painter would be sharper than the GPU one at every glyph edge and
//!   the comparison would report a difference on every page with text on it.
//!
//! The one deliberate divergence is the hatch, and it is the shader's: `pattern_at` uses
//! `textureLoad` — nearest, no filtering — so this does too.
//!
//! # Everything here is premultiplied
//!
//! Same reason as the shader's: source-over is a blend factor and not a formula, and interpolating
//! non-premultiplied colour across a fade to transparency is the classic dark halo. A value leaving
//! this module is premultiplied and in `0.0..=1.0`.

use mjx_scene::{BitmapFormat, Color, ImageFillMode, SceneRect, SceneTransform};

use crate::gradient::premultiplied;
use crate::pattern::cell_of;
use crate::plan::{apply, PaintProgram};

/// A rectangle of bytes a shader samples: an atlas page, or a decoded picture.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Bitmap {
    pub(super) width: u32,
    pub(super) height: u32,
    /// One byte per pixel for coverage, four for colour.
    pub(super) channels: usize,
    pub(super) bytes: Vec<u8>,
}

impl Bitmap {
    /// One texel, as four channels in `0.0..=1.0`. Coverage answers in the red channel, as the
    /// `R8Unorm` texture the shader binds does.
    fn texel(&self, x: i64, y: i64) -> [f32; 4] {
        let x = x.clamp(0, i64::from(self.width.saturating_sub(1))) as usize;
        let y = y.clamp(0, i64::from(self.height.saturating_sub(1))) as usize;
        let at = (y * self.width as usize + x) * self.channels;
        match self.channels {
            1 => {
                let value = f32::from(self.bytes.get(at).copied().unwrap_or(0)) / 255.0;
                [value, 0.0, 0.0, 1.0]
            }
            _ => [
                f32::from(self.bytes.get(at).copied().unwrap_or(0)) / 255.0,
                f32::from(self.bytes.get(at + 1).copied().unwrap_or(0)) / 255.0,
                f32::from(self.bytes.get(at + 2).copied().unwrap_or(0)) / 255.0,
                f32::from(self.bytes.get(at + 3).copied().unwrap_or(0)) / 255.0,
            ],
        }
    }

    /// A bilinear sample at normalised coordinates, with the given wrapping.
    ///
    /// The arithmetic a GPU's linear filter does: the texel grid is offset by half a texel, the four
    /// neighbours are weighted by the fractional part, and addressing is either clamped to the edge
    /// or wrapped. Writing it out is what lets a glyph's antialiased edge come out of this painter
    /// the same width it comes out of the other one.
    pub(super) fn sample(&self, u: f32, v: f32, repeat: bool) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0; 4];
        }
        let (u, v) = if repeat {
            (u.rem_euclid(1.0), v.rem_euclid(1.0))
        } else {
            (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
        };
        let x = u * self.width as f32 - 0.5;
        let y = v * self.height as f32 - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let (x0, y0) = (x0 as i64, y0 as i64);
        let mut out = [0.0f32; 4];
        let corners = [
            (self.texel(x0, y0), (1.0 - fx) * (1.0 - fy)),
            (self.texel(x0 + 1, y0), fx * (1.0 - fy)),
            (self.texel(x0, y0 + 1), (1.0 - fx) * fy),
            (self.texel(x0 + 1, y0 + 1), fx * fy),
        ];
        for (texel, weight) in corners {
            for (slot, channel) in out.iter_mut().zip(texel.iter()) {
                *slot += channel * weight;
            }
        }
        out
    }
}

/// What a shader can reach besides the paint itself.
pub(super) struct Textures<'a> {
    /// Glyph atlas pages, by page index.
    pub(super) atlas: &'a std::collections::HashMap<u32, Bitmap>,
    /// Decoded pictures, by handle.
    pub(super) images: &'a std::collections::HashMap<u64, Bitmap>,
}

/// Where a draw's texture coordinates come from.
///
/// The shader is handed a `uv` per **vertex** and the hardware interpolates it; a processor has to
/// compute the same affine map per pixel, which needs both ends of it — the box the draw covers, and
/// the rectangle of the source it maps onto.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Placement {
    /// The box the draw covers, in the space the paint is resolved in.
    pub(super) bounds: SceneRect,
    /// The rectangle of the source it maps to, **in texels**, for a draw that samples a rectangle
    /// of a larger image.
    ///
    /// `None` for a picture, whose mapping is `a:srcRect` and `a:tile` and is computed by
    /// [`picture_uv`] instead. `Some` for a glyph, and getting it wrong is subtle rather than
    /// obvious: a display list records **an atlas rectangle and never the page's size**, so a
    /// painter that mapped a glyph's quad onto the whole page would sample the wrong glyph — and
    /// against a repeating test atlas it would sample something that looks almost right.
    pub(super) texels: Option<SceneRect>,
}

impl Placement {
    /// A draw that covers `bounds` and samples nothing, or samples a picture.
    pub(super) const fn covering(bounds: SceneRect) -> Self {
        Self {
            bounds,
            texels: None,
        }
    }

    /// A draw that covers `bounds` and samples `texels` of its source.
    pub(super) const fn sampling(bounds: SceneRect, texels: SceneRect) -> Self {
        Self {
            bounds,
            texels: Some(texels),
        }
    }
}

/// One draw's shading, resolved once so that the per-pixel loop is arithmetic.
pub(super) struct Shading<'a> {
    program: &'a PaintProgram,
    /// Device pixels back into the space the paint is resolved in — the shader's `local`.
    inverse: SceneTransform,
    /// Where the draw is laid and what it samples, for the `uv` the shader interpolates.
    placement: Option<Placement>,
    textures: &'a Textures<'a>,
}

impl<'a> Shading<'a> {
    /// Prepare `program` for a draw placed by `transform`.
    ///
    /// Answers `None` for a transform that cannot be inverted — a shape scaled to nothing — because
    /// there is then no `local` for any pixel and the draw covers no area to want one for.
    pub(super) fn new(
        program: &'a PaintProgram,
        transform: SceneTransform,
        placement: Option<Placement>,
        textures: &'a Textures<'a>,
    ) -> Option<Self> {
        Some(Self {
            program,
            inverse: invert(transform)?,
            placement,
            textures,
        })
    }

    /// The premultiplied colour at a device pixel's centre.
    pub(super) fn at(&self, device_x: f32, device_y: f32) -> [f32; 4] {
        let (x, y) = apply(self.inverse, device_x, device_y);
        match self.program {
            PaintProgram::Solid(color) => premultiplied(*color),
            PaintProgram::Gradient { ramp, mapping } => {
                // `textureSampleLevel(ramp, clamped, vec2(t, row))` over one 256-texel row of the
                // frame's ramp texture. `mapping.at` is the very function the shader's `gradient_at`
                // was written against, so the two cannot drift.
                sample_ramp(ramp.texels(), mapping.at(x, y))
            }
            PaintProgram::Pattern {
                preset,
                foreground,
                background,
            } => {
                // `pattern_at`: the cell the point falls in, with no filtering, mixed between the
                // two colours by a coverage that is exactly zero or exactly one.
                let cell_x = x.floor();
                let cell_y = y.floor();
                let inside = cell_of(
                    *preset,
                    cell_x.rem_euclid(8.0) as usize,
                    cell_y.rem_euclid(8.0) as usize,
                );
                if inside {
                    premultiplied(*foreground)
                } else {
                    premultiplied(*background)
                }
            }
            PaintProgram::Picture { handle, image, .. } => {
                let Some(bitmap) = self.textures.images.get(handle) else {
                    // A handle nobody supplied pixels for. The area is drawn as nothing, exactly as
                    // the GPU painter draws it: a missing picture is a document problem.
                    return [0.0; 4];
                };
                let Some(placement) = self.placement else {
                    return [0.0; 4];
                };
                let (u, v) = picture_uv(image, bitmap, placement.bounds, x, y);
                let repeat = image.fill_mode == ImageFillMode::Tile;
                let texel = bitmap.sample(u, v, repeat);
                let adjusted = adjust(
                    [texel[0], texel[1], texel[2]],
                    f32::from(image.adjustments.brightness_in_ten_thousandths) / 10_000.0,
                    1.0 + f32::from(image.adjustments.contrast_in_ten_thousandths) / 10_000.0,
                    image.adjustments.grayscale,
                );
                let own_alpha = f32::from(image.adjustments.alpha_in_ten_thousandths) / 10_000.0;
                [
                    adjusted[0] * texel[3] * own_alpha,
                    adjusted[1] * texel[3] * own_alpha,
                    adjusted[2] * texel[3] * own_alpha,
                    texel[3] * own_alpha,
                ]
            }
            PaintProgram::Glyphs {
                page,
                format,
                color,
            } => {
                let Some(bitmap) = self.textures.atlas.get(page) else {
                    return [0.0; 4];
                };
                let (Some(placement), Some(texels)) = (
                    self.placement,
                    self.placement.and_then(|placement| placement.texels),
                ) else {
                    return [0.0; 4];
                };
                // **In texels, then normalised against the page's own size** — which is what the
                // shader's `in.uv / textureDimensions(source)` does, and the whole reason a display
                // list can record an atlas rectangle without recording the page's dimensions.
                // Mapping the quad onto the *page* instead samples a different glyph, and against a
                // repeating test atlas it samples something that looks almost right.
                let across = lerp_inverse(placement.bounds.left, placement.bounds.right, x);
                let down = lerp_inverse(placement.bounds.top, placement.bounds.bottom, y);
                let u = (texels.left + (texels.right - texels.left) * across)
                    / bitmap.width.max(1) as f32;
                let v = (texels.top + (texels.bottom - texels.top) * down)
                    / bitmap.height.max(1) as f32;
                match format {
                    BitmapFormat::Coverage => {
                        let coverage = bitmap.sample(u, v, false)[0];
                        let ink = premultiplied(*color);
                        [
                            ink[0] * coverage,
                            ink[1] * coverage,
                            ink[2] * coverage,
                            ink[3] * coverage,
                        ]
                    }
                    // A colour glyph is already composited and already premultiplied.
                    BitmapFormat::Rgba => bitmap.sample(u, v, false),
                }
            }
        }
    }
}

/// A resolved ramp, sampled the way a 256-texel one-dimensional texture is.
fn sample_ramp(texels: &[u8], t: f32) -> [f32; 4] {
    let count = texels.len() / 4;
    if count == 0 {
        return [0.0; 4];
    }
    let position = t.clamp(0.0, 1.0) * count as f32 - 0.5;
    let low = position.floor();
    let fraction = position - low;
    let read = |index: f32| -> [f32; 4] {
        let index = (index.max(0.0) as usize).min(count - 1) * 4;
        [
            f32::from(texels.get(index).copied().unwrap_or(0)) / 255.0,
            f32::from(texels.get(index + 1).copied().unwrap_or(0)) / 255.0,
            f32::from(texels.get(index + 2).copied().unwrap_or(0)) / 255.0,
            f32::from(texels.get(index + 3).copied().unwrap_or(0)) / 255.0,
        ]
    };
    let a = read(low);
    let b = read(low + 1.0);
    [
        a[0] + (b[0] - a[0]) * fraction,
        a[1] + (b[1] - a[1]) * fraction,
        a[2] + (b[2] - a[2]) * fraction,
        a[3] + (b[3] - a[3]) * fraction,
    ]
}

/// Where in a picture a point of its destination rectangle falls.
///
/// The processor's copy of [`crate::backend::WgpuPainter::picture_uv`], which computes the same
/// numbers at the quad's four corners and lets the hardware interpolate between them. Doing it per
/// pixel is the same affine map read at a different rate.
fn picture_uv(
    image: &mjx_scene::Image,
    bitmap: &Bitmap,
    destination: SceneRect,
    x: f32,
    y: f32,
) -> (f32, f32) {
    let across = lerp_inverse(destination.left, destination.right, x);
    let down = lerp_inverse(destination.top, destination.bottom, y);
    match image.fill_mode {
        ImageFillMode::Stretch => (
            image.crop.left + (image.crop.right - image.crop.left) * across,
            image.crop.top + (image.crop.bottom - image.crop.top) * down,
        ),
        ImageFillMode::Tile => {
            let scale_x = finite_or(image.tile_scale_x, 1.0);
            let scale_y = finite_or(image.tile_scale_y, 1.0);
            let tile_width = (bitmap.width as f32 * scale_x).abs().max(0.001);
            let tile_height = (bitmap.height as f32 * scale_y).abs().max(0.001);
            let span_x = (destination.right - destination.left) / tile_width;
            let span_y = (destination.bottom - destination.top) / tile_height;
            (
                -image.tile_offset.x / tile_width + across * span_x,
                -image.tile_offset.y / tile_height + down * span_y,
            )
        }
    }
}

/// A picture's brightness, contrast and greyscale, in the order DrawingML's prose puts them.
pub(super) fn adjust(rgb: [f32; 3], brightness: f32, contrast: f32, grayscale: bool) -> [f32; 3] {
    let mut value = [
        rgb[0] + brightness,
        rgb[1] + brightness,
        rgb[2] + brightness,
    ];
    for channel in &mut value {
        *channel = ((*channel - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
    }
    if grayscale {
        // Rec. 709 luma, the same three constants the shader carries.
        let luma = value[0] * 0.2126 + value[1] * 0.7152 + value[2] * 0.0722;
        value = [luma, luma, luma];
    }
    value
}

/// Where `value` falls between `from` and `to`, as a fraction.
fn lerp_inverse(from: f32, to: f32, value: f32) -> f32 {
    let span = to - from;
    if span.abs() < f32::EPSILON {
        return 0.0;
    }
    (value - from) / span
}

/// `value` if it is a number, and `fallback` if a document said something else.
pub(super) fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

/// A colour, premultiplied, for a caller outside this module.
pub(super) fn premultiplied_color(color: Color) -> [f32; 4] {
    premultiplied(color)
}

/// The inverse of an affine map, or `None` when it has none.
pub(super) fn invert(transform: SceneTransform) -> Option<SceneTransform> {
    let determinant = transform.scale_x * transform.scale_y - transform.shear_x * transform.shear_y;
    if !determinant.is_finite() || determinant.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / determinant;
    let scale_x = transform.scale_y * inverse;
    let shear_y = -transform.shear_y * inverse;
    let shear_x = -transform.shear_x * inverse;
    let scale_y = transform.scale_x * inverse;
    Some(SceneTransform {
        scale_x,
        shear_y,
        shear_x,
        scale_y,
        translate_x: -(scale_x * transform.translate_x + shear_x * transform.translate_y),
        translate_y: -(shear_y * transform.translate_x + scale_y * transform.translate_y),
    })
}
