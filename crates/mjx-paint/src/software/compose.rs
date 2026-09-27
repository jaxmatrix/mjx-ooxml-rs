//! Blending, and the effect kernels — the processor's copy of the five fixed-function blend states
//! and the six effect branches of `backend/shaders.wgsl`.
//!
//! # Why the blend equations are written out rather than deferred to `tiny-skia`
//!
//! `tiny_skia::BlendMode` has `Multiply`, `Screen`, `Darken` and `Lighten`, and they are **not the
//! same functions**: they are the Porter-Duff separable modes from the PDF and CSS compositing
//! specification, which account for the backdrop's alpha. The GPU painter's are fixed-function
//! blend factors — `Dst * Src`, `Src + Dst*(1-Src)`, `min`, `max` — chosen because WebGL 2 has no
//! programmable blending and no advanced blend equations, and `backend/mod.rs` says so at the call
//! site.
//!
//! Using `tiny-skia`'s would make the two painters differ on every page with a blend mode on it,
//! and the difference would be *the software painter being more correct* — which is exactly the
//! shape of disagreement a cross-painter gate cannot act on, because neither answer is wrong and
//! nobody can tell which one the document meant. So this mirrors the GPU's, and if the five modes
//! are ever to become the specification's, both painters change together.
//!
//! # Everything is premultiplied and everything is clamped
//!
//! A render target holds premultiplied values in `0.0..=1.0`, the same as the GPU's `Rgba8Unorm`
//! attachments, and every write clamps — `Screen` and `Lighten` can both leave the unit interval,
//! and a value that escaped it would be truncated on write anyway but would first poison whatever
//! sampled it.

use mjx_scene::BlendMode;

use super::shade::Bitmap;

/// A premultiplied RGBA buffer: one layer, one effect intermediate, or the frame.
///
/// A thin owner around `tiny_skia::Pixmap` so that the byte order and the premultiplication are
/// stated in one place. `tiny-skia` stores exactly this — RGBA8, premultiplied — which is why the
/// two can share a buffer with no conversion at the boundary.
#[derive(Clone, Debug)]
pub(super) struct Canvas {
    pixmap: tiny_skia::Pixmap,
}

impl Canvas {
    /// A transparent canvas.
    pub(super) fn new(width: u32, height: u32) -> Option<Self> {
        Some(Self {
            pixmap: tiny_skia::Pixmap::new(width.max(1), height.max(1))?,
        })
    }

    /// How wide.
    pub(super) fn width(&self) -> u32 {
        self.pixmap.width()
    }

    /// How tall.
    pub(super) fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Forget everything drawn into it.
    pub(super) fn clear(&mut self) {
        self.pixmap.data_mut().fill(0);
    }

    /// The premultiplied bytes, RGBA, row by row from the top.
    pub(super) fn bytes(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// The premultiplied value at a pixel.
    pub(super) fn pixel(&self, x: u32, y: u32) -> [f32; 4] {
        if x >= self.width() || y >= self.height() {
            return [0.0; 4];
        }
        let at = ((y * self.width() + x) * 4) as usize;
        let bytes = self.pixmap.data();
        [
            f32::from(bytes.get(at).copied().unwrap_or(0)) / 255.0,
            f32::from(bytes.get(at + 1).copied().unwrap_or(0)) / 255.0,
            f32::from(bytes.get(at + 2).copied().unwrap_or(0)) / 255.0,
            f32::from(bytes.get(at + 3).copied().unwrap_or(0)) / 255.0,
        ]
    }

    /// Replace the value at a pixel.
    pub(super) fn set(&mut self, x: u32, y: u32, value: [f32; 4]) {
        if x >= self.width() || y >= self.height() {
            return;
        }
        let width = self.width();
        let at = ((y * width + x) * 4) as usize;
        let bytes = self.pixmap.data_mut();
        for (index, channel) in value.iter().enumerate() {
            if let Some(slot) = bytes.get_mut(at + index) {
                *slot = (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    /// Blend `source` — already premultiplied and already scaled by coverage — over the pixel.
    pub(super) fn blend(&mut self, x: u32, y: u32, source: [f32; 4], mode: BlendMode) {
        let destination = self.pixel(x, y);
        self.set(x, y, blend(source, destination, mode));
    }

    /// The canvas as a bitmap a shader can sample — for the effect passes, which read a whole
    /// target the way a fragment function reads a texture.
    pub(super) fn as_bitmap(&self) -> Bitmap {
        Bitmap {
            width: self.width(),
            height: self.height(),
            channels: 4,
            bytes: self.pixmap.data().to_vec(),
        }
    }
}

/// One pixel of `source` over one pixel of `destination`, both premultiplied.
///
/// The five fixed-function blend states of `backend/mod.rs::blend_state`, written as arithmetic. The
/// alpha channel follows source-over for every mode but `Darken` and `Lighten`, whose whole meaning
/// is a per-channel minimum or maximum — a `Multiply` that also multiplied alpha would make a shape
/// drawn over transparency vanish.
#[must_use]
pub(super) fn blend(source: [f32; 4], destination: [f32; 4], mode: BlendMode) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for channel in 0..3 {
        let s = source[channel];
        let d = destination[channel];
        out[channel] = match mode {
            // `src * One + dst * (1 - src.a)`
            BlendMode::Over => s + d * (1.0 - source[3]),
            // `src * Dst + dst * Zero`
            BlendMode::Multiply => s * d,
            // `src * One + dst * (1 - src)`
            BlendMode::Screen => s + d * (1.0 - s),
            BlendMode::Darken => s.min(d),
            BlendMode::Lighten => s.max(d),
        };
    }
    out[3] = match mode {
        BlendMode::Darken => source[3].min(destination[3]),
        BlendMode::Lighten => source[3].max(destination[3]),
        _ => source[3] + destination[3] * (1.0 - source[3]),
    };
    for channel in &mut out {
        *channel = channel.clamp(0.0, 1.0);
    }
    out
}

/// A separable Gaussian, seventeen taps wide, with the **spacing** carried rather than the count.
///
/// The shader's `blurred()`, in pixels rather than in texture coordinates — `step` there is a
/// fraction of the target's width and `offset * direction` is added to a `uv`, which is the same
/// arithmetic once both are multiplied by the target's size.
///
/// It is a true Gaussian to about eight device pixels of radius and a progressively coarser one past
/// that, which is R08's stated approximation and is deliberately matched rather than improved:
/// **a software painter that blurred better than the GPU one would make every shadow on every page
/// a cross-painter difference**, and the difference would say nothing about either rasteriser. If
/// the approximation is to become a downsampling chain, both painters change together.
pub(super) fn blur(source: &Bitmap, radius: f32, width: u32, height: u32) -> Bitmap {
    let step = radius / 8.0;
    if !step.is_finite() || step <= 0.0 || width == 0 || height == 0 {
        return source.clone();
    }
    let horizontal = blur_axis(source, step, 1.0, 0.0, width, height);
    blur_axis(&horizontal, step, 0.0, 1.0, width, height)
}

fn blur_axis(
    source: &Bitmap,
    step: f32,
    across: f32,
    down: f32,
    width: u32,
    height: u32,
) -> Bitmap {
    let mut bytes = vec![0u8; (width as usize) * (height as usize) * 4];
    // The seventeen weights, computed once: the shader recomputes `exp` per tap per pixel because a
    // GPU has nothing better to do with those cycles, and a processor does.
    let mut weights = [0.0f32; 17];
    let mut total_weight = 0.0f32;
    for (index, weight) in weights.iter_mut().enumerate() {
        let offset = index as f32 - 8.0;
        *weight = (-0.5 * offset * offset / 8.0).exp();
        total_weight += *weight;
    }
    let total_weight = total_weight.max(0.0001);

    for y in 0..height {
        for x in 0..width {
            let mut sum = [0.0f32; 4];
            for (index, weight) in weights.iter().enumerate() {
                let offset = index as f32 - 8.0;
                let u = (x as f32 + 0.5 + across * step * offset) / width as f32;
                let v = (y as f32 + 0.5 + down * step * offset) / height as f32;
                let texel = source.sample(u, v, false);
                for (slot, channel) in sum.iter_mut().zip(texel.iter()) {
                    *slot += channel * weight;
                }
            }
            let at = ((y as usize * width as usize) + x as usize) * 4;
            for (index, channel) in sum.iter().enumerate() {
                if let Some(slot) = bytes.get_mut(at + index) {
                    *slot = ((channel / total_weight).clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
    }
    Bitmap {
        width,
        height,
        channels: 4,
        bytes,
    }
}
