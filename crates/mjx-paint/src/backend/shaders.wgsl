// The whole painter's shading, in one module.
//
// # Why one shader and not eight
//
// Every draw this painter issues has the same shape: transform a position in device pixels into
// clip space, and produce one premultiplied RGBA value for it. What differs is where that value
// comes from — a colour, a ramp, a hatch, a picture, a glyph's coverage, or another render target
// being brought back. That is a `switch` in a fragment function, and it costs one uniform read.
//
// The alternative is a pipeline per paint kind, each with its own bind-group layout, and a bind
// group rebuilt whenever the kind changes as a page alternates fills and text. On the baseline
// backend — WebGL 2 — a pipeline switch is a program switch, which is the most expensive state
// change there is. One program, one layout, and a dynamic uniform offset per draw is both simpler
// and faster.
//
// # No compute, anywhere
//
// WebGL 2 has no compute stage, and the browser is a target. Everything below is a vertex or a
// fragment function over triangles that arrived already tessellated, which is the constraint that
// put `lyon` above this crate rather than a compute-driven vector renderer inside it.
//
// # Everything is premultiplied
//
// Colours reach here premultiplied by their own alpha, every render target holds premultiplied
// values, and source-over is `(One, OneMinusSrcAlpha)`. That is what makes a fade to transparency
// interpolate without a dark halo, and it is what makes multiplying a layer by a group opacity a
// single multiply rather than an un-premultiply, a scale and a re-premultiply.

struct Draw {
    // a, b, c, d of the 2x3 affine map, in device pixels.
    linear: vec4<f32>,
    // e, f — the translation — then the viewport's width and height in device pixels.
    translate: vec4<f32>,
    // The paint's colour, premultiplied. A glyph's ink, a shadow's colour, a hatch's foreground.
    color: vec4<f32>,
    // A hatch's background, premultiplied.
    background: vec4<f32>,
    // Gradient: origin.xy, then the axis a linear ramp's `t` is a dot product with.
    gradient_origin: vec4<f32>,
    // Gradient: the radii a radial ramp is measured against, whether it is radial, and which row of
    // the frame's ramp texture it is.
    gradient_shape: vec4<f32>,
    // kind, opacity, and two scalars whose meaning is the kind's. The mapping, in full, because a
    // shared slot with an undocumented meaning is a defect waiting for the next paint kind:
    //
    //   kind          | params.z                  | params.w
    //   --------------+---------------------------+---------------------------
    //   pattern       | which of the 54 presets   | unused
    //   gradient      | unused                    | how many ramp rows exist
    //   image         | 1 when the picture tiles  | unused
    //   everything else: both unused.
    params: vec4<f32>,
    // Effect: blur step in texels, blur direction x, blur direction y, and which axis a fade runs
    // along (0 for horizontal, 1 for vertical).
    effect: vec4<f32>,
    // A picture's adjustments: brightness, contrast, whether to desaturate, and its own alpha.
    // Its own four numbers rather than borrowed room in `background`, because a slot that means one
    // thing for a hatch and another for a picture is a defect waiting for a paint kind to be added.
    image: vec4<f32>,
    // A reflection's fade: the opacity where it starts, where it ends, and the two positions along
    // the direction those apply at.
    fade: vec4<f32>,
}

@group(0) @binding(0) var<uniform> draw: Draw;
@group(1) @binding(0) var source: texture_2d<f32>;
@group(1) @binding(1) var second: texture_2d<f32>;
// Two samplers rather than one, and not for convenience: a tiled picture must wrap and a gradient
// ramp must not. One sampler in `Repeat` would make the ramp's last texel wrap round to its first,
// which is a visible wrong colour at exactly `t == 1`; one in `ClampToEdge` would smear a tiled
// picture's last column across the whole page instead of repeating it.
@group(1) @binding(2) var clamped: sampler;
@group(1) @binding(3) var repeating: sampler;

struct Varying {
    @builtin(position) clip: vec4<f32>,
    // The vertex's position *before* the transform, which is the space every paint is resolved in:
    // a gradient's axis and a hatch's tile are properties of the shape, not of where it was moved
    // to, so a rotated shape's hatch rotates with it for free.
    @location(0) local: vec2<f32>,
    // Texture coordinates. Texels for a glyph — normalised in the fragment function against the
    // atlas page's own size, because a display list records an atlas rectangle and never the page's
    // dimensions — and already normalised for everything else.
    @location(1) uv: vec2<f32>,
}

@vertex
fn vertex_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
) -> Varying {
    var out: Varying;
    let x = draw.linear.x * position.x + draw.linear.z * position.y + draw.translate.x;
    let y = draw.linear.y * position.x + draw.linear.w * position.y + draw.translate.y;
    // Device pixels, y downward from the top-left, into clip space, y upward from the centre.
    out.clip = vec4<f32>(
        x / max(draw.translate.z, 1.0) * 2.0 - 1.0,
        1.0 - y / max(draw.translate.w, 1.0) * 2.0,
        0.0,
        1.0,
    );
    out.local = position;
    out.uv = uv;
    return out;
}

// Paint kinds. Mirrored by `PaintKind` in `backend/mod.rs`, and the two are held in step by
// `tests/the_shader_kinds_match_the_painter.rs`: a shader that disagreed with its own uniform about
// what `3` means would draw a picture where a hatch belongs and nothing would fail to compile.
const KIND_SOLID: f32 = 0.0;
const KIND_GRADIENT: f32 = 1.0;
const KIND_PATTERN: f32 = 2.0;
const KIND_IMAGE: f32 = 3.0;
const KIND_GLYPH_COVERAGE: f32 = 4.0;
const KIND_GLYPH_COLOR: f32 = 5.0;
const KIND_BLIT: f32 = 6.0;
const KIND_BLUR: f32 = 7.0;
const KIND_TINT: f32 = 8.0;
const KIND_MASK_BY_SOURCE_ALPHA: f32 = 9.0;
const KIND_INNER_SHADOW: f32 = 10.0;
const KIND_FADE: f32 = 11.0;

/// How far along a gradient's ramp a point is. The executable specification of this arithmetic is
/// `GradientMapping::at`, which is the same three lines in Rust and is what the ramp's own test
/// checks — a shader cannot be unit-tested and that function can.
fn gradient_at(local: vec2<f32>) -> f32 {
    let delta = local - draw.gradient_origin.xy;
    if (draw.gradient_shape.z > 0.5) {
        let rx = select(0.0, delta.x / draw.gradient_shape.x, draw.gradient_shape.x != 0.0);
        let ry = select(0.0, delta.y / draw.gradient_shape.y, draw.gradient_shape.y != 0.0);
        return clamp(sqrt(rx * rx + ry * ry), 0.0, 1.0);
    }
    return clamp(dot(delta, draw.gradient_origin.zw), 0.0, 1.0);
}

/// The hatch cell a point falls in. `textureLoad` rather than a sampler: the fifty-four masks are
/// stacked in one texture eight texels wide, and filtering across the boundary between two presets
/// would blend one hatch into the next.
fn pattern_at(local: vec2<f32>) -> vec4<f32> {
    let cell = vec2<i32>(
        i32(floor(local.x)) % 8,
        i32(floor(local.y)) % 8,
    );
    let wrapped = vec2<i32>((cell.x + 8) % 8, (cell.y + 8) % 8);
    let row = i32(draw.params.z) * 8 + wrapped.y;
    let coverage = textureLoad(source, vec2<i32>(wrapped.x, row), 0).r;
    return mix(draw.background, draw.color, coverage);
}

/// A separable Gaussian, seventeen taps wide, with the spacing carried in the uniform.
///
/// Seventeen taps and a variable step rather than a kernel sized to the radius: a loop with a
/// constant bound is what the baseline backend can compile, and widening the *spacing* rather than
/// the tap count turns a large radius into a coarser Gaussian instead of an unbounded one. Past
/// about eight pixels of radius that is visibly a box blur, which is the trade this makes on
/// purpose — see `backend/mod.rs` for why it is not a downsampling chain yet.
fn blurred(uv: vec2<f32>) -> vec4<f32> {
    var total = vec4<f32>(0.0);
    var weight = 0.0;
    let direction = draw.effect.yz * draw.effect.x;
    for (var tap = -8; tap <= 8; tap = tap + 1) {
        let offset = f32(tap);
        let w = exp(-0.5 * offset * offset / 8.0);
        total = total + textureSampleLevel(source, clamped, uv + direction * offset, 0.0) * w;
        weight = weight + w;
    }
    return total / max(weight, 0.0001);
}

@fragment
fn fragment_main(in: Varying) -> @location(0) vec4<f32> {
    let kind = draw.params.x;
    let opacity = draw.params.y;

    if (kind == KIND_SOLID) {
        return draw.color * opacity;
    }
    if (kind == KIND_GRADIENT) {
        let t = gradient_at(in.local);
        let row = (draw.gradient_shape.w + 0.5) / max(draw.params.w, 1.0);
        return textureSampleLevel(source, clamped, vec2<f32>(t, row), 0.0) * opacity;
    }
    if (kind == KIND_PATTERN) {
        return pattern_at(in.local) * opacity;
    }
    if (kind == KIND_IMAGE) {
        var texel = textureSampleLevel(source, clamped, in.uv, 0.0);
        if (draw.params.z > 0.5) {
            texel = textureSampleLevel(source, repeating, in.uv, 0.0);
        }
        // A picture arrives non-premultiplied — that is what a decoded PNG is — and every target
        // here holds premultiplied values, so the multiply happens exactly once, here.
        let adjusted = adjust(texel.rgb);
        return vec4<f32>(adjusted * texel.a, texel.a) * opacity * draw.image.w;
    }
    if (kind == KIND_GLYPH_COVERAGE) {
        let size = vec2<f32>(textureDimensions(source, 0));
        let coverage = textureSampleLevel(source, clamped, in.uv / max(size, vec2<f32>(1.0)), 0.0).r;
        return draw.color * coverage * opacity;
    }
    if (kind == KIND_GLYPH_COLOR) {
        let size = vec2<f32>(textureDimensions(source, 0));
        // A colour glyph is already composited and already premultiplied — `mjx-text` flattens a
        // `COLR`/`CPAL` layer stack into one image — so it is scaled, never re-premultiplied.
        return textureSampleLevel(source, clamped, in.uv / max(size, vec2<f32>(1.0)), 0.0) * opacity;
    }
    if (kind == KIND_BLIT) {
        return textureSampleLevel(source, clamped, in.uv, 0.0) * opacity;
    }
    if (kind == KIND_BLUR) {
        return blurred(in.uv) * opacity;
    }
    if (kind == KIND_TINT) {
        // A shadow and a glow are the subtree's *silhouette* in one colour: its alpha decides where,
        // and the colour decides what. Reading the RGB as well would tint a black shadow by the
        // shape's own colour and produce a shadow of the picture rather than of its outline.
        let alpha = textureSampleLevel(source, clamped, in.uv, 0.0).a;
        return draw.color * alpha * opacity;
    }
    if (kind == KIND_MASK_BY_SOURCE_ALPHA) {
        // A soft edge: the subtree, with its alpha multiplied by a blurred copy of its own alpha, so
        // the edge feathers inward from wherever the edge actually is.
        let mask = textureSampleLevel(source, clamped, in.uv, 0.0).a;
        return textureSampleLevel(second, clamped, in.uv, 0.0) * mask * opacity;
    }
    if (kind == KIND_INNER_SHADOW) {
        // Inside the shape's own alpha, wherever the blurred *inverse* of that alpha reaches.
        let inverse = 1.0 - textureSampleLevel(source, clamped, in.uv, 0.0).a;
        let inside = textureSampleLevel(second, clamped, in.uv, 0.0).a;
        return draw.color * inverse * inside * opacity;
    }
    if (kind == KIND_FADE) {
        // A reflection: the subtree, faded from one opacity to another along the effect's direction.
        let along = clamp(mix(in.uv.x, in.uv.y, draw.effect.w), 0.0, 1.0);
        let position = clamp(
            (along - draw.fade.z) / max(draw.fade.w - draw.fade.z, 0.0001),
            0.0,
            1.0,
        );
        let alpha = mix(draw.fade.x, draw.fade.y, position);
        return textureSampleLevel(source, clamped, in.uv, 0.0) * alpha * opacity;
    }
    return draw.color * opacity;
}

/// A picture's brightness, contrast and greyscale adjustments, applied in that order.
///
/// `a:lum`'s brightness is additive and its contrast is multiplicative about the mid grey, which is
/// what DrawingML's own prose describes and is why they are not one multiply.
fn adjust(rgb: vec3<f32>) -> vec3<f32> {
    var value = rgb + vec3<f32>(draw.image.x);
    value = clamp(
        (value - vec3<f32>(0.5)) * draw.image.y + vec3<f32>(0.5),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    if (draw.image.z > 0.5) {
        // Rec. 709 luma, which is what every surface this platform reaches uses for a greyscale.
        let luma = dot(value, vec3<f32>(0.2126, 0.7152, 0.0722));
        value = vec3<f32>(luma);
    }
    return value;
}
