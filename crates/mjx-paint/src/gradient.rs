//! A gradient's stops, resolved into a ramp a shader can sample with one texture read.
//!
//! # Why a ramp texture and not the stops
//!
//! DrawingML puts no bound on how many stops a gradient has. A fragment shader that walked them
//! would need a loop with a dynamic bound and an array of uniforms sized for the worst case, and
//! **WebGL 2 has neither a storage buffer nor a dynamically indexed uniform array it is happy
//! about** — the same constraint that put tessellation above this crate rather than in a compute
//! shader. So the stops are resolved on the processor into [`RAMP_TEXELS`] premultiplied texels
//! once, and the shader does `textureSample(ramp, sampler, vec2(t, row))`.
//!
//! That is also the *cheaper* answer on a GPU: a page whose theme uses one gradient on forty shapes
//! resolves it once and forty draws share a row.
//!
//! # Premultiplied, and why the interpolation has to be
//!
//! Every colour in this painter is premultiplied by its own alpha before it is blended, because
//! source-over on a GPU is a blend factor and not a formula. Interpolating **non**-premultiplied
//! colours across a stop that fades to transparent is the classic dark-halo bug: a fade from opaque
//! red to transparent black passes through half-alpha *dark* red, which is a grey edge nobody asked
//! for. Interpolating premultiplied ones does not. So the conversion happens before the
//! interpolation, here, and `tests/the_gradient_ramp_is_a_ramp.rs` holds it with a fade to
//! transparency whose midpoint has to stay red.

use mjx_scene::{Color, Gradient, GradientStop};

/// How many texels a resolved ramp has.
///
/// 256 is one texel per eight-bit output level along the ramp, so a two-stop gradient across a
/// four-hundred-pixel shape bands by less than one level — and it is small enough that a page with
/// sixteen distinct gradients costs sixteen kilobytes of texture.
pub const RAMP_TEXELS: usize = 256;

/// How many bytes one resolved ramp takes.
pub const RAMP_BYTES: usize = RAMP_TEXELS * 4;

/// Premultiply a colour, as everything this painter blends is.
#[must_use]
pub fn premultiplied(color: Color) -> [f32; 4] {
    let alpha = f32::from(color.alpha) / 255.0;
    [
        (f32::from(color.red) / 255.0) * alpha,
        (f32::from(color.green) / 255.0) * alpha,
        (f32::from(color.blue) / 255.0) * alpha,
        alpha,
    ]
}

/// One resolved gradient: [`RAMP_TEXELS`] premultiplied `RGBA` texels, left to right along the ramp.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GradientRamp {
    texels: Vec<u8>,
}

impl GradientRamp {
    /// Resolve `gradient`'s stops into a ramp.
    ///
    /// A gradient with **no stops** resolves to a fully transparent ramp rather than to an error:
    /// `a:gradFill` with an empty `a:gsLst` is malformed markup a real document contains, and
    /// failing the frame for it would refuse to draw the other forty shapes on the page.
    #[must_use]
    pub fn resolve(gradient: &Gradient) -> Self {
        Self::from_stops(&gradient.stops)
    }

    /// Resolve a stop list on its own.
    #[must_use]
    pub fn from_stops(stops: &[GradientStop]) -> Self {
        let mut texels = vec![0u8; RAMP_BYTES];
        if stops.is_empty() {
            return Self { texels };
        }

        // Sorted by position, because DrawingML does not require `a:gs` elements to be in order and
        // a ramp built from an unsorted list runs backwards through part of itself. Sorting a copy
        // of the positions rather than the stops keeps this allocation-light for the usual two.
        let mut order: Vec<usize> = (0..stops.len()).collect();
        order.sort_by(|left, right| {
            let a = stops.get(*left).map(|s| s.position_in_ten_thousandths);
            let b = stops.get(*right).map(|s| s.position_in_ten_thousandths);
            a.cmp(&b)
        });

        for texel in 0..RAMP_TEXELS {
            // The centre of the texel, so that texel 0 is the very start of the ramp and texel 255
            // the very end; sampling at the left edge would make the last stop unreachable.
            let along = texel as f32 / (RAMP_TEXELS - 1) as f32;
            let color = sample(stops, &order, along);
            let offset = texel * 4;
            for (channel, value) in color.iter().enumerate() {
                if let Some(slot) = texels.get_mut(offset + channel) {
                    *slot = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
        Self { texels }
    }

    /// The texels, premultiplied `RGBA`, four bytes each.
    #[must_use]
    pub fn texels(&self) -> &[u8] {
        &self.texels
    }

    /// The texel at `index`, or `None` past the end.
    #[must_use]
    pub fn texel(&self, index: usize) -> Option<[u8; 4]> {
        let offset = index.checked_mul(4)?;
        let bytes = self.texels.get(offset..offset + 4)?;
        Some([
            *bytes.first()?,
            *bytes.get(1)?,
            *bytes.get(2)?,
            *bytes.get(3)?,
        ])
    }
}

/// The premultiplied colour `along` the ramp, `0.0` to `1.0`.
fn sample(stops: &[GradientStop], order: &[usize], along: f32) -> [f32; 4] {
    let first = order.first().and_then(|index| stops.get(*index));
    let last = order.last().and_then(|index| stops.get(*index));
    let (Some(first), Some(last)) = (first, last) else {
        return [0.0; 4];
    };
    // Before the first stop and after the last, DrawingML clamps rather than extrapolating.
    if along <= first.position() {
        return premultiplied(first.color);
    }
    if along >= last.position() {
        return premultiplied(last.color);
    }
    for pair in order.windows(2) {
        let (Some(left), Some(right)) = (
            pair.first().and_then(|index| stops.get(*index)),
            pair.get(1).and_then(|index| stops.get(*index)),
        ) else {
            continue;
        };
        let (start, end) = (left.position(), right.position());
        if along < start || along > end {
            continue;
        }
        let span = end - start;
        // Two stops at the same position are a hard edge, which is a thing designers do on purpose;
        // dividing by the zero span would make it a `NaN` instead.
        let mix = if span > 0.0 {
            (along - start) / span
        } else {
            1.0
        };
        let (from, to) = (premultiplied(left.color), premultiplied(right.color));
        return [
            from[0] + (to[0] - from[0]) * mix,
            from[1] + (to[1] - from[1]) * mix,
            from[2] + (to[2] - from[2]) * mix,
            from[3] + (to[3] - from[3]) * mix,
        ];
    }
    premultiplied(last.color)
}
