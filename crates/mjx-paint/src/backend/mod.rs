//! The `wgpu` painter: a [`crate::FramePlan`] executed on the platform's graphics stack.
//!
//! # How a frame is drawn
//!
//! Every layer of the plan gets a render pass, and **layers are rendered in descending index
//! order**. That is not an ordering choice, it is a consequence: a layer is opened while its parent
//! is being walked, so a child always has a higher index than its parent, and counting down is
//! therefore a topological order with no sort. Each layer resolves into a single-sampled texture out
//! of the effect pool; its parent brings it back with a full-viewport quad at the position the group
//! occupied.
//!
//! Layer zero renders into an offscreen `Rgba8Unorm` texture even when the target is a window, and
//! the window is painted with one final blit. That costs a full-screen quad a frame and buys three
//! things: one colour format for every pipeline instead of one per surface, a readback path that
//! works on a window as well as offscreen (which R10's golden images want), and a present step that
//! is one pass rather than a second code path.
//!
//! # Multisampling, and why the effect passes are not multisampled
//!
//! Geometry passes render into a shared four-sample colour attachment and resolve into the layer's
//! texture. Effect passes — blur, tint, fade, composite — are full-viewport quads with no edges in
//! them at all, so they are single-sampled: multisampling a quad that covers every pixel entirely
//! costs four times the bandwidth to produce the same picture.
//!
//! # What the effect passes actually do
//!
//! A blur is two separable seventeen-tap passes with the *spacing* scaled by the radius. That is a
//! true Gaussian up to about eight device pixels and a progressively coarser one past it, which is
//! this painter's stated approximation: the exact answer for a large radius is a downsampling
//! chain, which is more passes and more pool pressure than this child's scope, and the visible
//! difference on a shadow is a slightly flatter falloff. It is written here rather than hidden so
//! that R09's software painter can decide whether to match it or to be better.
//!
//! # Nothing here panics
//!
//! Not on a malformed list, not on a lost device, not on a driver complaint. `wgpu`'s own
//! uncaptured-error handler would abort the process; [`GraphicsDevice`] replaces it with one that
//! collects, and every failure below is a typed [`PaintError`].

mod device;
mod execute;
mod frame;
mod render;

use std::collections::HashMap;

pub use device::{
    drawing_backends, GraphicsDevice, OFFSCREEN_FORMAT, REQUESTED_SAMPLE_COUNT, STENCIL_FORMAT,
};

use mjx_scene::{BlendMode, Color, Tessellator};

use crate::error::PaintError;
use crate::painter::{DrawReport, FrameReport, Pixels};
use crate::pattern::{coverage_atlas, PATTERN_SIDE};
use crate::pool::{PoolHandle, PoolStatistics, TexturePool, DEFAULT_TEXTURE_POOL_BYTES};
use crate::surface::{SurfaceHost, SurfaceTarget, Viewport};

/// Which branch of the fragment function a draw takes.
///
/// **Mirrored by the `KIND_*` constants in `shaders.wgsl`**, and the two are held in step by
/// `tests/the_shader_and_the_painter_agree.rs`: a shader that disagreed with its own uniform about
/// what `3` means would draw a picture where a hatch belongs, and nothing would fail to compile.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PaintKind {
    /// One colour.
    Solid = 0,
    /// A ramp, sampled along the shape.
    Gradient = 1,
    /// One of the fifty-four hatches.
    Pattern = 2,
    /// A picture.
    Image = 3,
    /// A glyph's coverage in one ink colour.
    GlyphCoverage = 4,
    /// A colour glyph, already composited.
    GlyphColor = 5,
    /// Another target, brought back unchanged.
    Blit = 6,
    /// Another target, blurred along one axis.
    Blur = 7,
    /// Another target's silhouette, in one colour.
    Tint = 8,
    /// The second texture, masked by the first's alpha.
    MaskBySourceAlpha = 9,
    /// A colour inside the second texture's alpha and outside the first's.
    InnerShadow = 10,
    /// Another target, faded along an axis.
    Fade = 11,
}

impl PaintKind {
    /// The number the shader compares against.
    #[must_use]
    pub const fn wire(self) -> f32 {
        self as u32 as f32
    }

    /// Every kind, so a gate can sweep them all.
    pub const ALL: [Self; 12] = [
        Self::Solid,
        Self::Gradient,
        Self::Pattern,
        Self::Image,
        Self::GlyphCoverage,
        Self::GlyphColor,
        Self::Blit,
        Self::Blur,
        Self::Tint,
        Self::MaskBySourceAlpha,
        Self::InnerShadow,
        Self::Fade,
    ];

    /// The name of the shader constant that must carry the same number.
    #[must_use]
    pub const fn shader_constant(self) -> &'static str {
        match self {
            Self::Solid => "KIND_SOLID",
            Self::Gradient => "KIND_GRADIENT",
            Self::Pattern => "KIND_PATTERN",
            Self::Image => "KIND_IMAGE",
            Self::GlyphCoverage => "KIND_GLYPH_COVERAGE",
            Self::GlyphColor => "KIND_GLYPH_COLOR",
            Self::Blit => "KIND_BLIT",
            Self::Blur => "KIND_BLUR",
            Self::Tint => "KIND_TINT",
            Self::MaskBySourceAlpha => "KIND_MASK_BY_SOURCE_ALPHA",
            Self::InnerShadow => "KIND_INNER_SHADOW",
            Self::Fade => "KIND_FADE",
        }
    }
}

/// The shader source, so a gate can read the constants out of the very file that is compiled.
pub const SHADER_SOURCE: &str = include_str!("shaders.wgsl");

/// How many `f32`s one draw's uniform block holds before padding.
const UNIFORM_FLOATS: usize = 40;

/// How the clip stencil is used by a draw.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum StencilMode {
    /// No depth-stencil attachment at all — an effect pass, where there is nothing to clip.
    Absent,
    /// Draw where the stencil equals the current clip depth.
    Test,
    /// Write no colour; deepen the stencil inside the shape.
    Push,
    /// Write no colour; shallow it again.
    Pop,
}

/// Everything that decides which pipeline a draw needs.
///
/// `Hash` is written out rather than derived because `mjx_scene::BlendMode` is not `Hash` — a
/// display list's vocabulary has no reason to be — and hashing its position in the vocabulary is
/// both stable and one line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct PipelineKey {
    blend: BlendMode,
    stencil: StencilMode,
    samples: u32,
    /// The colour format of the attachment. Every target in this painter is
    /// [`OFFSCREEN_FORMAT`]; the exception is the one blit that puts a finished frame on a window,
    /// whose format is whatever the swapchain chose.
    format: wgpu::TextureFormat,
}

impl core::hash::Hash for PipelineKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        blend_index(self.blend).hash(state);
        self.stencil.hash(state);
        self.samples.hash(state);
        self.format.hash(state);
    }
}

/// A blend mode's position in the vocabulary, for hashing.
const fn blend_index(mode: BlendMode) -> u8 {
    match mode {
        BlendMode::Over => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Darken => 3,
        BlendMode::Lighten => 4,
    }
}

/// Which texture a draw samples.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum TexRef {
    /// One opaque white texel, for a draw that samples nothing.
    White,
    /// The fifty-four hatches.
    Pattern,
    /// This frame's gradient ramps.
    Ramps,
    /// A glyph atlas page.
    Atlas(u32),
    /// A picture.
    Image(u64),
    /// A layer or an effect intermediate.
    Pooled(PoolHandle),
}

/// One draw, resolved.
struct Record {
    uniform_slot: usize,
    indices: std::ops::Range<u32>,
    base_vertex: i32,
    textures: (TexRef, TexRef),
    key: PipelineKey,
    stencil_reference: u32,
}

/// One render pass.
struct Pass {
    /// Where it draws. `None` is the frame's own colour texture.
    target: Option<PoolHandle>,
    /// Whether it is multisampled and carries a stencil.
    geometry: bool,
    /// Whether the target starts empty. Every layer's own pass does; the one that composites a
    /// finished layer onto the frame does not, because that is what accumulates several display
    /// lists onto one frame.
    clear: bool,
    records: Vec<Record>,
}

/// The `wgpu` painter.
pub struct WgpuPainter {
    device: GraphicsDevice,
    shader: wgpu::ShaderModule,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    clamped: wgpu::Sampler,
    repeating: wgpu::Sampler,
    white: wgpu::TextureView,
    pattern: wgpu::TextureView,
    atlas: HashMap<u32, (wgpu::Texture, wgpu::TextureView)>,
    images: HashMap<u64, (wgpu::Texture, wgpu::TextureView)>,
    pool: TexturePool<wgpu::Texture>,
    tessellator: Tessellator,
    uniform_stride: u32,
    surface: Option<wgpu::Surface<'static>>,
    surface_format: wgpu::TextureFormat,
    open: Option<OpenFrame>,
    next_frame: u64,
    highlight_placeholders: bool,
    last_colour: Option<wgpu::Texture>,
    last_pixels: Option<Pixels>,
    last_report: Option<FrameReport>,
}

impl core::fmt::Debug for WgpuPainter {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("WgpuPainter")
            .field("device", &self.device)
            .field("pipelines", &self.pipelines.len())
            .field("atlas_pages", &self.atlas.len())
            .field("images", &self.images.len())
            .field("pool", &self.pool)
            .field("frame", &self.open.as_ref().map(|frame| frame.id))
            .finish()
    }
}

/// A frame in progress.
struct OpenFrame {
    id: u64,
    width: u32,
    height: u32,
    colour: wgpu::Texture,
    colour_view: wgpu::TextureView,
    multisample: Option<wgpu::TextureView>,
    stencil: wgpu::TextureView,
    presenting: bool,
    report: DrawReport,
}

/// The warning colour a stand-in shape is drawn in.
///
/// Not a token from `mjx-tokens`' theme: this is not part of any design, it is a developer signal
/// that the shape on the screen is not the document's shape. Magenta at three-quarters alpha, which
/// is the one colour a document theme reliably does not contain.
pub const PLACEHOLDER_WARNING: Color = Color {
    red: 0xff,
    green: 0x00,
    blue: 0xff,
    alpha: 0xc0,
};

impl WgpuPainter {
    /// Open a painter with no window: everything is drawn into textures it owns.
    ///
    /// # Errors
    ///
    /// [`PaintError::NoAdapter`] on a machine with no graphics stack — which is a normal answer and
    /// the one a loud, named skip is written against — and [`PaintError::Device`] if one is found
    /// and will not open.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn offscreen() -> Result<Self, PaintError> {
        let device = GraphicsDevice::open(None)?;
        Self::on(device, None, OFFSCREEN_FORMAT)
    }

    /// The same, awaited rather than blocked on.
    ///
    /// **The only form available in a browser.** See [`GraphicsDevice::open_on_async`]: blocking a
    /// `wasm32` thread on adapter acquisition never returns, so the blocking constructors are
    /// compiled out there rather than left as a trap for the shell that meets them.
    ///
    /// # Errors
    ///
    /// As [`WgpuPainter::offscreen`].
    pub async fn offscreen_async() -> Result<Self, PaintError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let device = GraphicsDevice::open_on_async(instance, None, drawing_backends()).await?;
        Self::on(device, None, OFFSCREEN_FORMAT)
    }

    /// Open a painter on a window.
    ///
    /// # Errors
    ///
    /// [`PaintError::Surface`] if no surface can be made for the handles, plus everything
    /// [`WgpuPainter::offscreen`] can fail with.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn for_window(host: &dyn SurfaceHost) -> Result<Self, PaintError> {
        pollster::block_on(Self::for_window_async(host))
    }

    /// The same, awaited rather than blocked on. The only form available in a browser.
    ///
    /// # Errors
    ///
    /// As [`WgpuPainter::for_window`].
    pub async fn for_window_async(host: &dyn SurfaceHost) -> Result<Self, PaintError> {
        let SurfaceTarget::Window(handles) = host.raw_handle() else {
            return Self::offscreen_async().await;
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        // SAFETY(MJX-PAINT-SURFACE-UNSAFE): `handles` describes a live window, and the window
        // outlives every surface made from it. That is the window system's invariant and the
        // shell's to keep — the shell is what destroys the window — and it is stated as the caller's
        // obligation on `DesktopWindow::new`, which is the only way to obtain a `WindowHandles`.
        // Nothing else in this crate is `unsafe`, and the CI grep in `.github/workflows/ci.yml`
        // refuses any `unsafe` in `crates/mjx-paint/src` that does not carry this marker.
        let surface = unsafe /* MJX-PAINT-SURFACE-UNSAFE */ {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: handles.display,
                raw_window_handle: handles.window,
            })
        }
        .map_err(|error| PaintError::Surface(error.to_string()))?;

        let device =
            GraphicsDevice::open_on_async(instance, Some(&surface), drawing_backends()).await?;
        let capabilities = surface.get_capabilities(device.adapter());
        let format = capabilities
            .formats
            .first()
            .copied()
            .unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
        let (width, height) = host.size();
        let scale = host.scale_factor();
        let viewport = Viewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            scale_factor: scale,
        };
        let (physical_width, physical_height) = viewport.require_pixels()?;
        surface.configure(
            device.device(),
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: physical_width,
                height: physical_height,
                present_mode: capabilities
                    .present_modes
                    .first()
                    .copied()
                    .unwrap_or(wgpu::PresentMode::Fifo),
                alpha_mode: capabilities
                    .alpha_modes
                    .first()
                    .copied()
                    .unwrap_or(wgpu::CompositeAlphaMode::Auto),
                view_formats: Vec::new(),
                desired_maximum_frame_latency: 2,
                color_space: wgpu::SurfaceColorSpace::Auto,
            },
        );
        Self::on(device, Some(surface), format)
    }

    /// Build a painter on an already-open device.
    fn on(
        device: GraphicsDevice,
        surface: Option<wgpu::Surface<'static>>,
        surface_format: wgpu::TextureFormat,
    ) -> Result<Self, PaintError> {
        let shader = device
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mjx-paint"),
                source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
            });

        let uniform_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("mjx-paint draw"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: wgpu::BufferSize::new((UNIFORM_FLOATS * 4) as u64),
                        },
                        count: None,
                    }],
                });
        let texture_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("mjx-paint textures"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            device
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("mjx-paint"),
                    bind_group_layouts: &[Some(&uniform_layout), Some(&texture_layout)],
                    // No immediate (push-constant) data. WebGL 2 has none, and a uniform buffer
                    // with a dynamic offset is what works on every backend in the matrix.
                    immediate_size: 0,
                });

        let clamped = device.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mjx-paint clamped"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let repeating = device.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mjx-paint repeating"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let white = upload_texture(
            &device,
            "mjx-paint white",
            1,
            1,
            wgpu::TextureFormat::Rgba8Unorm,
            &[0xff, 0xff, 0xff, 0xff],
        )?;
        let pattern = upload_texture(
            &device,
            "mjx-paint patterns",
            PATTERN_SIDE as u32,
            (coverage_atlas().len() / PATTERN_SIDE) as u32,
            wgpu::TextureFormat::R8Unorm,
            &coverage_atlas(),
        )?;

        let uniform_stride = align_to(
            (UNIFORM_FLOATS * 4) as u32,
            device
                .device()
                .limits()
                .min_uniform_buffer_offset_alignment
                .max(1),
        );

        Ok(Self {
            shader,
            uniform_layout,
            texture_layout,
            pipeline_layout,
            pipelines: HashMap::new(),
            clamped,
            repeating,
            white: white.1,
            pattern: pattern.1,
            atlas: HashMap::new(),
            images: HashMap::new(),
            pool: TexturePool::with_budget(DEFAULT_TEXTURE_POOL_BYTES),
            tessellator: Tessellator::new(),
            uniform_stride,
            surface,
            surface_format,
            open: None,
            next_frame: 1,
            highlight_placeholders: true,
            last_colour: None,
            last_pixels: None,
            last_report: None,
            device,
        })
    }

    /// Whether stand-in geometry is drawn in [`PLACEHOLDER_WARNING`] rather than in its own paint.
    ///
    /// On by default. **Every preset shape in this platform resolves to a stand-in today**, so a
    /// developer looking at a render should be able to see that at a glance rather than be told
    /// later by a report. R10's golden-image suite turns it off and asserts
    /// [`DrawReport::placeholders`] is zero instead, which is the stronger of the two checks.
    #[must_use]
    pub fn highlighting_placeholders(mut self, highlight: bool) -> Self {
        self.highlight_placeholders = highlight;
        self
    }

    /// Give the effect texture pool a different byte budget.
    #[must_use]
    pub fn with_texture_budget(mut self, bytes: usize) -> Self {
        self.pool = TexturePool::with_budget(bytes);
        self
    }

    /// What the effect texture pool has been doing.
    #[must_use]
    pub fn pool_statistics(&self) -> PoolStatistics {
        self.pool.statistics()
    }

    /// How many bytes of render target the pool is holding for reuse.
    #[must_use]
    pub fn retained_texture_bytes(&self) -> usize {
        self.pool.retained_bytes()
    }

    /// The device, for a caller that wants to ask it something this surface does not expose.
    #[must_use]
    pub fn graphics_device(&self) -> &GraphicsDevice {
        &self.device
    }

    /// The last frame's report, for a caller that dropped it.
    #[must_use]
    pub fn last_frame(&self) -> Option<FrameReport> {
        self.last_report
    }
}

/// Round `value` up to a multiple of `alignment`.
fn align_to(value: u32, alignment: u32) -> u32 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + (alignment - remainder)
    }
}

/// Create a texture and fill it.
fn upload_texture(
    device: &GraphicsDevice,
    label: &str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    pixels: &[u8],
) -> Result<(wgpu::Texture, wgpu::TextureView), PaintError> {
    if width == 0 || height == 0 {
        return Err(PaintError::TextureUnavailable {
            width,
            height,
            detail: "a texture with no area".to_owned(),
        });
    }
    let texture = device.device().create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bytes = format.block_copy_size(None).unwrap_or(4);
    device.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok((texture, view))
}

/// The blend state a blend mode is.
///
/// Every one of these is expressible with fixed-function blending, which is what makes them
/// available on the baseline backend: WebGL 2 has no programmable blending and no advanced blend
/// equations, so a mode that needed either would work on three of the five targets.
fn blend_state(mode: BlendMode) -> wgpu::BlendState {
    let component = match mode {
        // Premultiplied source-over.
        BlendMode::Over => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        BlendMode::Multiply => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Dst,
            dst_factor: wgpu::BlendFactor::Zero,
            operation: wgpu::BlendOperation::Add,
        },
        BlendMode::Screen => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrc,
            operation: wgpu::BlendOperation::Add,
        },
        BlendMode::Darken => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Min,
        },
        BlendMode::Lighten => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Max,
        },
    };
    // Alpha follows source-over for every mode. A `Multiply` that also multiplied alpha would make
    // a shape drawn over transparency vanish, which is not what any of the five modes means.
    let alpha = match mode {
        BlendMode::Darken | BlendMode::Lighten => component,
        _ => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    };
    wgpu::BlendState {
        color: component,
        alpha,
    }
}

/// The stencil face a mode uses.
fn stencil_face(mode: StencilMode) -> wgpu::StencilFaceState {
    match mode {
        StencilMode::Absent | StencilMode::Test => wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Equal,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::Keep,
        },
        StencilMode::Push => wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Equal,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::IncrementClamp,
        },
        StencilMode::Pop => wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Equal,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::DecrementClamp,
        },
    }
}

/// Build one pipeline.
fn build_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    key: PipelineKey,
) -> wgpu::RenderPipeline {
    let writes_colour = !matches!(key.stencil, StencilMode::Push | StencilMode::Pop);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mjx-paint"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 8,
                        shader_location: 1,
                    },
                ],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: key.format,
                blend: Some(blend_state(key.blend)),
                write_mask: if writes_colour {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::empty()
                },
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            // No culling. A tessellated path's triangles have no consistent winding — `lyon` emits
            // whatever the trapezoidation produced — so culling by face would drop half of every
            // shape.
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: match key.stencil {
            StencilMode::Absent => None,
            mode => Some(wgpu::DepthStencilState {
                format: STENCIL_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: stencil_face(mode),
                    back: stencil_face(mode),
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
        },
        multisample: wgpu::MultisampleState {
            count: key.samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
