//! The `tiny-skia` painter: a [`crate::FramePlan`] executed on the processor, with no graphics
//! stack, no window and no `unsafe`.
//!
//! # What this is for, which is not "a fallback"
//!
//! **It is what makes the rest of the programme testable.** R10's fidelity oracle, R11's plate
//! regression and every golden image after them render headlessly through this painter, on machines
//! that have no GPU and no display server. And it is this crate's guarantee that a **fully pure-Rust
//! path to pixels always exists**, which is half of what made MJXOFF-163's amendment to
//! `CLAUDE.md`'s pure-Rust rule a boundary rather than a concession.
//!
//! It is also the only honest way to test the `wgpu` painter. Two independent implementations
//! agreeing is evidence; one implementation agreeing with itself is not. That is what
//! [`crate::compare`] is for, and it is why this painter is *deliberately not independent* about
//! anything except rasterisation.
//!
//! # What is shared with the GPU painter, and what is not
//!
//! **Shared, on purpose:** the lowering ([`crate::plan_frame`], not a second walk of the display
//! list), the tessellation (R07's triangles, the same ones), the fifty-four hatch masks, the
//! gradient ramp resolver, the gradient mapping, the picture adjustments, the blend equations, the
//! blur kernel, the effect ordering ([`crate::plan::draws_behind`]) and the texture pool.
//!
//! **Different, which is the whole point:** how a triangle becomes coverage. `wgpu` rasterises with
//! four-sample multisampling on whatever hardware is present; this walks the same triangles through
//! `tiny-skia`'s analytic scan converter. A cross-painter difference is therefore a statement about
//! *rasterisation*, which is a thing a reviewer can act on.
//!
//! # How a frame is drawn
//!
//! Exactly as the GPU painter draws one, because a different structure would be a different
//! interpretation. Layers are rendered in **descending index order** — a child always has a higher
//! index than its parent, so counting down is a topological order with no sort — each into a
//! viewport-sized premultiplied canvas out of the same [`crate::TexturePool`], and each is
//! composited into its parent at the position the group occupied.
//!
//! # Where the triangles go
//!
//! Every draw is scan-converted into a coverage [`tiny_skia::Mask`] **sized to its own device
//! bounding box** rather than to the viewport. That is not tidiness: a run of a thousand glyphs is a
//! thousand quads with a thousand different atlas rectangles, so each has to be rasterised
//! separately, and a full-viewport mask cleared a thousand times is a hundred megabytes of memset
//! for one line of text.
//!
//! Triangles are written into one path per draw with **every triangle wound the same way** and
//! filled non-zero. Adjacent triangles then union with no seam along their shared edges — the
//! winding number is at least one on both sides of an interior edge — where filling them one at a
//! time would leave a visible antialiased hairline everywhere `lyon` happened to cut.
//!
//! # Nothing here panics
//!
//! Same reason as everywhere else in this crate, and one more: this painter is what continuous
//! integration runs, so a panic here is a build that fails rather than a frame that looks wrong.

mod compose;
mod shade;

use std::collections::HashMap;

use mjx_scene::{
    BlendMode, Color, DisplayList, EffectKind, Mesh, SceneRect, SceneTransform, Tessellator,
};

use crate::error::PaintError;
use crate::painter::{
    AdapterKind, Antialiasing, BackendReport, Capabilities, DrawReport, Frame, FrameReport,
    GraphicsApi, Painter, Pixels,
};
use crate::plan::{
    apply, draws_behind, plan_frame, replaces_subtree, DrawOp, EffectNode, FramePlan, GlyphQuad,
    LayerKind, PaintProgram,
};
use crate::pool::{
    PoolHandle, PoolStatistics, TexturePool, TextureSize, DEFAULT_TEXTURE_POOL_BYTES,
};
use crate::resources::{
    bytes_per_pixel, AtlasPage, AtlasVisitor, AtlasWrite, ImageSource, Resources,
};
use crate::surface::{SurfaceHost, Viewport};

use compose::{blend, blur, Canvas};
use shade::{finite_or, invert, premultiplied_color, Bitmap, Placement, Shading, Textures};

/// The name this painter reports.
///
/// A constant rather than a literal inside one method, because *which painter ran* is the question
/// every gate in this crate is built to answer, and a test that spelled the name itself could spell
/// it wrong and still pass.
pub const SOFTWARE_PAINTER: &str = "tiny-skia";

/// The largest software target this painter will allocate, on a side.
///
/// Bounded by memory rather than by a driver: sixteen thousand a side is a gigabyte of premultiplied
/// pixels, which is the point past which allocating is a worse answer than refusing.
pub const MAXIMUM_SOFTWARE_TARGET: u32 = 16_384;

/// What one composite of a finished layer into its parent does.
///
/// The processor's copy of `backend/frame.rs`'s `CompositeStep`, kept as its own type rather than
/// shared: the GPU's carries `TexRef` handles into `wgpu` textures, and a shared type would put a
/// graphics handle into a software painter's vocabulary.
#[derive(Clone, Debug)]
struct Step {
    source: Bitmap,
    second: Option<Bitmap>,
    kind: StepKind,
    alpha: f32,
    transform: SceneTransform,
    color: Color,
    /// How far the **source** is shifted before it is read, in texture coordinates.
    ///
    /// Only an inner shadow uses it, and it is the whole of what distinguishes an inner shadow from
    /// an outer one: the step's two inputs are the blurred copy and the mask, and moving the quad
    /// would move both. See `backend/shaders.wgsl`'s `KIND_INNER_SHADOW`.
    source_offset: [f32; 2],
    /// Start opacity, end opacity, start position, end position — a reflection's fade.
    fade: [f32; 4],
    /// `0.0` for a fade along `x`, `1.0` along `y`.
    fade_axis: f32,
    blend: BlendMode,
}

impl Step {
    fn blit(source: Bitmap) -> Self {
        Self {
            source,
            second: None,
            kind: StepKind::Blit,
            alpha: 1.0,
            transform: SceneTransform::IDENTITY,
            color: TRANSPARENT,
            source_offset: [0.0, 0.0],
            fade: [1.0, 1.0, 0.0, 1.0],
            fade_axis: 1.0,
            blend: BlendMode::Over,
        }
    }
}

/// What a composite step computes, mirroring the shader's effect branches one for one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepKind {
    /// The source, unchanged. `KIND_BLIT`.
    Blit,
    /// The source's silhouette in one colour. `KIND_TINT`.
    Tint,
    /// The second input, masked by the first's alpha. `KIND_MASK_BY_SOURCE_ALPHA`.
    MaskBySourceAlpha,
    /// A colour inside the second's alpha and outside the first's. `KIND_INNER_SHADOW`.
    InnerShadow,
    /// The source, faded along an axis. `KIND_FADE`.
    Fade,
}

/// A colour with nothing in it.
const TRANSPARENT: Color = Color {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 0,
};

/// What is clipping a draw.
///
/// Three states rather than an `Option<Mask>`, because *"nothing gets through"* is a real state a
/// document produces — a clip whose shape has no area — and it is not the same as *"nothing is
/// clipping"*. Collapsing the two would draw a clipped-away shape at full size.
#[derive(Clone, Debug, Default)]
enum ClipState {
    /// Nothing is clipping.
    #[default]
    Open,
    /// This coverage is.
    Mask(tiny_skia::Mask),
    /// Nothing gets through.
    Closed,
}

impl ClipState {
    /// How much of a pixel this lets through.
    fn at(&self, x: u32, y: u32) -> f32 {
        match self {
            Self::Open => 1.0,
            Self::Closed => 0.0,
            Self::Mask(mask) => {
                if x >= mask.width() || y >= mask.height() {
                    return 0.0;
                }
                let at = (y * mask.width() + x) as usize;
                f32::from(mask.data().get(at).copied().unwrap_or(0)) / 255.0
            }
        }
    }

    /// This clip, narrowed to a shape.
    fn intersect(&self, mesh: &Mesh, transform: SceneTransform, width: u32, height: u32) -> Self {
        let Some(mut mask) = tiny_skia::Mask::new(width.max(1), height.max(1)) else {
            // No storage for a clip. Clipping nothing away would draw *more* than the document
            // asks for, so the safe answer is the closed one.
            return Self::Closed;
        };
        // A clip whose shape has no triangles clips everything away — the mask stays zero, which is
        // what an empty clip means and what the GPU painter's stencil arithmetic produces for it.
        if let Some(path) = mesh_path(mesh, transform) {
            mask.fill_path(
                &path,
                tiny_skia::FillRule::Winding,
                true,
                tiny_skia::Transform::identity(),
            );
        }
        match self {
            Self::Open => Self::Mask(mask),
            Self::Closed => Self::Closed,
            Self::Mask(outer) => {
                for (slot, keep) in mask.data_mut().iter_mut().zip(outer.data().iter()) {
                    *slot = ((u16::from(*slot) * u16::from(*keep)) / 255) as u8;
                }
                Self::Mask(mask)
            }
        }
    }
}

/// A frame in progress.
struct OpenFrame {
    id: u64,
    canvas: Canvas,
    report: DrawReport,
}

/// The software painter.
pub struct SoftwarePainter {
    atlas: HashMap<u32, Bitmap>,
    images: HashMap<u64, Bitmap>,
    pool: TexturePool<Canvas>,
    tessellator: Tessellator,
    open: Option<OpenFrame>,
    next_frame: u64,
    highlight_placeholders: bool,
    last_pixels: Option<Pixels>,
    last_report: Option<FrameReport>,
}

impl core::fmt::Debug for SoftwarePainter {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SoftwarePainter")
            .field("atlas_pages", &self.atlas.len())
            .field("images", &self.images.len())
            .field("pool", &self.pool)
            .field("frame", &self.open.as_ref().map(|frame| frame.id))
            .finish()
    }
}

impl Default for SoftwarePainter {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwarePainter {
    /// A painter with nothing open.
    ///
    /// **This constructor cannot fail**, and that is the point of it: a `wgpu` painter answers
    /// [`PaintError::NoAdapter`] on a machine with no graphics stack, and this one has no adapter to
    /// look for. It is what a headless test, a `wasm32` build with no WebGPU and a continuous-
    /// integration machine with no display all reach for.
    #[must_use]
    pub fn new() -> Self {
        Self {
            atlas: HashMap::new(),
            images: HashMap::new(),
            pool: TexturePool::with_budget(DEFAULT_TEXTURE_POOL_BYTES),
            tessellator: Tessellator::new(),
            open: None,
            next_frame: 1,
            highlight_placeholders: true,
            last_pixels: None,
            last_report: None,
        }
    }

    /// Whether stand-in geometry is drawn in [`crate::PLACEHOLDER_WARNING`] rather than in its own
    /// paint.
    ///
    /// On by default, and **the same default the GPU painter has**: a cross-painter comparison
    /// between one painter that highlights placeholders and one that does not would report a
    /// difference on every page that contains a shape neither could draw.
    #[must_use]
    pub fn highlighting_placeholders(mut self, highlight: bool) -> Self {
        self.highlight_placeholders = highlight;
        self
    }

    /// Give the layer pool a different byte budget.
    #[must_use]
    pub fn with_texture_budget(mut self, bytes: usize) -> Self {
        self.pool = TexturePool::with_budget(bytes);
        self
    }

    /// What the layer pool has been doing.
    #[must_use]
    pub fn pool_statistics(&self) -> PoolStatistics {
        self.pool.statistics()
    }

    /// How many bytes of layer target the pool is holding for reuse.
    #[must_use]
    pub fn retained_texture_bytes(&self) -> usize {
        self.pool.retained_bytes()
    }

    /// The last frame's report, for a caller that dropped it.
    #[must_use]
    pub fn last_frame(&self) -> Option<FrameReport> {
        self.last_report
    }

    /// Render one layer of a plan, **with its own effect applied**, into pixels of its own.
    ///
    /// # Why an exporter needs this
    ///
    /// **PDF has no blur.** There is no filter model in its imaging model at all, so a shadow, a
    /// glow, a soft edge and a blur cannot be written as operators however patiently one tries. The
    /// exporter's documented fallback is to rasterise, and the thing it rasterises with must be the
    /// pure-Rust one — a PDF export that needed a GPU would fail on exactly the machines that export
    /// PDFs.
    ///
    /// So this is the one public entry into this painter that is not the [`Painter`] contract, and
    /// it exists for that one caller. It renders `index` and every layer below it in the plan — a
    /// child always has a higher index than its parent, so those are all of its descendants — and
    /// then applies the layer's own composite steps onto a transparent canvas.
    ///
    /// Answers `None` for the root layer, which has no effect of its own to apply and is what
    /// [`Painter::draw`] is for.
    ///
    /// # Errors
    ///
    /// Whatever the atlas, the images and the pool fail with.
    pub fn rasterise_layer(
        &mut self,
        plan: &FramePlan,
        index: usize,
        width: u32,
        height: u32,
        resources: &mut Resources<'_>,
    ) -> Result<Option<Pixels>, PaintError> {
        if index == 0 || index >= plan.layers().len() {
            return Ok(None);
        }
        {
            let mut store = AtlasStore {
                pages: &mut self.atlas,
                created: 0,
                released: 0,
                bytes: 0,
            };
            resources.glyphs().take_changes(&mut store)?;
        }
        take_images(&mut self.images, plan, resources.images());

        let mut finished: HashMap<usize, Vec<Step>> = HashMap::new();
        let mut targets: HashMap<usize, PoolHandle> = HashMap::new();
        for layer_index in (index..plan.layers().len()).rev() {
            let handle = acquire(&mut self.pool, width, height)?;
            targets.insert(layer_index, handle);
            let bounds = render_layer(
                &mut self.pool,
                handle,
                plan,
                layer_index,
                &self.atlas,
                &self.images,
                self.highlight_placeholders,
                &finished,
            )?;
            let content = bounds.unwrap_or(SceneRect::new(0.0, 0.0, width as f32, height as f32));
            let Some(layer) = plan.layer(layer_index) else {
                continue;
            };
            let rendered = self.pool.texture(handle)?.as_bitmap();
            let steps = match &layer.kind {
                LayerKind::Root => continue,
                LayerKind::Opacity(alpha) => {
                    let mut step = Step::blit(rendered);
                    step.alpha = *alpha;
                    vec![step]
                }
                LayerKind::Effect(nodes) => {
                    evaluate_effects(nodes, &rendered, content, width, height)
                }
                LayerKind::Mask {
                    fill,
                    bounds,
                    transform,
                } => {
                    let painted = paint_bitmap(
                        &self.atlas,
                        &self.images,
                        fill,
                        *bounds,
                        *transform,
                        width,
                        height,
                    );
                    let mut step = Step::blit(rendered);
                    step.kind = StepKind::MaskBySourceAlpha;
                    step.second = Some(painted);
                    vec![step]
                }
            };
            finished.insert(layer_index, steps);
        }

        let pixels = match (Canvas::new(width, height), finished.get(&index)) {
            (Some(mut canvas), Some(steps)) => {
                for step in steps {
                    composite(&mut canvas, step, &ClipState::Open);
                }
                Some(Pixels {
                    width,
                    height,
                    rgba: canvas.bytes().to_vec(),
                })
            }
            _ => None,
        };
        for handle in targets.into_values() {
            self.pool.release(handle)?;
        }
        Ok(pixels)
    }
}

impl Painter for SoftwarePainter {
    fn name(&self) -> &'static str {
        SOFTWARE_PAINTER
    }

    fn backend(&self) -> BackendReport {
        BackendReport {
            api: GraphicsApi::None,
            // Honest rather than flattering: this *is* the processor, and a report saying anything
            // else would let a suite believe a GPU had been exercised.
            adapter: AdapterKind::Cpu,
            adapter_name: "tiny-skia software rasteriser".to_owned(),
            driver: String::new(),
            antialiasing: Antialiasing::Analytic,
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            antialiasing: Antialiasing::Analytic,
            max_texture_size: MAXIMUM_SOFTWARE_TARGET,
            exact_path_clipping: true,
            effect_texture_budget: self.pool.budget(),
        }
    }

    fn begin(
        &mut self,
        target: &mut dyn SurfaceHost,
        viewport: Viewport,
    ) -> Result<Frame, PaintError> {
        if let Some(open) = &self.open {
            return Err(PaintError::FrameAlreadyOpen { open: open.id });
        }
        let (width, height) = viewport.require_pixels()?;
        if width > MAXIMUM_SOFTWARE_TARGET || height > MAXIMUM_SOFTWARE_TARGET {
            return Err(PaintError::TextureUnavailable {
                width,
                height,
                detail: format!(
                    "a software target larger than {MAXIMUM_SOFTWARE_TARGET} on a side is refused"
                ),
            });
        }
        let canvas = Canvas::new(width, height).ok_or_else(|| PaintError::TextureUnavailable {
            width,
            height,
            detail: "the frame's pixels could not be allocated".to_owned(),
        })?;
        // Nothing is being recovered from, so the host is asked for nothing; its target is read so
        // that a shell handing this painter a window rather than an offscreen surface is a
        // supported call rather than an ignored one.
        let _ = target.raw_handle();
        let id = self.next_frame;
        self.next_frame = self.next_frame.saturating_add(1);
        self.open = Some(OpenFrame {
            id,
            canvas,
            report: DrawReport::default(),
        });
        Ok(Frame::new(id, viewport))
    }

    fn draw(
        &mut self,
        frame: &Frame,
        list: &DisplayList,
        resources: &mut Resources<'_>,
    ) -> Result<DrawReport, PaintError> {
        let (width, height) = match &self.open {
            Some(open) if open.id == frame.id() => (open.canvas.width(), open.canvas.height()),
            open => {
                return Err(PaintError::WrongFrame {
                    given: frame.id(),
                    open: open.as_ref().map(|open| open.id),
                })
            }
        };

        let plan = plan_frame(list, resources.geometry(), &mut self.tessellator)?;
        let mut report = plan.report();

        {
            let mut store = AtlasStore {
                pages: &mut self.atlas,
                created: 0,
                released: 0,
                bytes: 0,
            };
            resources.glyphs().take_changes(&mut store)?;
            report.atlas_pages_created = store.created;
            report.atlas_pages_released = store.released;
            report.atlas_bytes_uploaded = store.bytes;
        }
        take_images(&mut self.images, &plan, resources.images());

        let mut finished: HashMap<usize, Vec<Step>> = HashMap::new();
        let mut targets: HashMap<usize, PoolHandle> = HashMap::new();

        // Descending, which is a topological order with no sort — the same order the GPU painter
        // renders in, and for the same reason: a layer is opened while its parent is being walked.
        for index in (0..plan.layers().len()).rev() {
            let handle = acquire(&mut self.pool, width, height)?;
            targets.insert(index, handle);
            let bounds = render_layer(
                &mut self.pool,
                handle,
                &plan,
                index,
                &self.atlas,
                &self.images,
                self.highlight_placeholders,
                &finished,
            )?;
            let content = bounds.unwrap_or(SceneRect::new(0.0, 0.0, width as f32, height as f32));
            let Some(layer) = plan.layer(index) else {
                continue;
            };
            let rendered = self.pool.texture(handle)?.as_bitmap();
            let steps = match &layer.kind {
                LayerKind::Root => continue,
                LayerKind::Opacity(alpha) => {
                    let mut step = Step::blit(rendered);
                    step.alpha = *alpha;
                    vec![step]
                }
                LayerKind::Effect(nodes) => {
                    evaluate_effects(nodes, &rendered, content, width, height)
                }
                LayerKind::Mask {
                    fill,
                    bounds,
                    transform,
                } => {
                    // One composite, which is what a software painter can do here that a GPU
                    // cannot: the fill is shaded straight into a buffer and the stencil multiplies
                    // it. `a:textFill` with a gradient in it stops being one representative colour.
                    let painted = paint_bitmap(
                        &self.atlas,
                        &self.images,
                        fill,
                        *bounds,
                        *transform,
                        width,
                        height,
                    );
                    let mut step = Step::blit(rendered);
                    step.kind = StepKind::MaskBySourceAlpha;
                    step.second = Some(painted);
                    vec![step]
                }
            };
            finished.insert(index, steps);
        }

        // The root layer, composited onto the frame. One pass, exactly as the GPU painter finishes
        // a `draw` with one blit, which is what lets several display lists accumulate onto one
        // frame rather than needing a frame each.
        if let Some(root) = targets.get(&0).copied() {
            let bitmap = self.pool.texture(root)?.as_bitmap();
            if let Some(open) = self.open.as_mut() {
                composite(&mut open.canvas, &Step::blit(bitmap), &ClipState::Open);
            }
        }

        for handle in targets.into_values() {
            self.pool.release(handle)?;
        }

        if let Some(open) = &mut self.open {
            open.report.absorb(report);
        }
        Ok(report)
    }

    fn end(&mut self, frame: Frame) -> Result<FrameReport, PaintError> {
        let Some(open) = self.open.take() else {
            return Err(PaintError::WrongFrame {
                given: frame.id(),
                open: None,
            });
        };
        if open.id != frame.id() {
            let id = open.id;
            self.open = Some(open);
            return Err(PaintError::WrongFrame {
                given: frame.id(),
                open: Some(id),
            });
        }
        let report = FrameReport {
            frame: open.id,
            drawn: open.report,
            width: open.canvas.width(),
            height: open.canvas.height(),
            // Analytic coverage is one sample per pixel with a fractional answer, so this is `1`
            // and not `4`: a report of four would claim a multisampled render that never happened.
            sample_count: 1,
            // Nothing was presented. A software painter draws into memory, and a shell that wants
            // the pixels on a window blits them itself — a platform call, and not this crate's.
            presented: false,
            pool: self.pool.statistics(),
        };
        self.last_pixels = Some(Pixels {
            width: open.canvas.width(),
            height: open.canvas.height(),
            rgba: open.canvas.bytes().to_vec(),
        });
        self.last_report = Some(report);
        Ok(report)
    }

    fn read_pixels(&mut self) -> Result<Option<Pixels>, PaintError> {
        Ok(self.last_pixels.clone())
    }
}

/// Take a viewport-sized canvas out of the pool, cleared.
fn acquire(
    pool: &mut TexturePool<Canvas>,
    width: u32,
    height: u32,
) -> Result<PoolHandle, PaintError> {
    let handle = pool.acquire(TextureSize::new(width, height), |size| {
        Canvas::new(size.width, size.height).ok_or_else(|| PaintError::TextureUnavailable {
            width: size.width,
            height: size.height,
            detail: "a layer's pixels could not be allocated".to_owned(),
        })
    })?;
    // A pooled canvas holds whatever the last frame left in it, and every layer starts empty.
    pool.texture_mut(handle)?.clear();
    Ok(handle)
}

/// Draw one layer's operations into its canvas, and answer the box its content covered.
///
/// A free function rather than a method because it needs the pool mutably and the texture stores
/// immutably at the same moment, which one `&mut self` cannot give.
#[allow(
    clippy::too_many_arguments,
    reason = "one layer's whole state, threaded once"
)]
fn render_layer(
    pool: &mut TexturePool<Canvas>,
    handle: PoolHandle,
    plan: &FramePlan,
    index: usize,
    atlas: &HashMap<u32, Bitmap>,
    images: &HashMap<u64, Bitmap>,
    highlight_placeholders: bool,
    finished: &HashMap<usize, Vec<Step>>,
) -> Result<Option<SceneRect>, PaintError> {
    let Some(layer) = plan.layer(index) else {
        return Ok(None);
    };
    let size = pool.size_of(handle)?;
    let (width, height) = (size.width, size.height);
    let textures = Textures { atlas, images };

    // The clip, as coverage. A stack rather than a stencil's depth counter, which is the same idea:
    // `PushClip` intersects, `PopClip` restores what was in force before it.
    let mut clip = ClipState::Open;
    let mut clips: Vec<ClipState> = Vec::new();
    let mut bounds: Option<SceneRect> = None;

    for op in &layer.ops {
        match op {
            DrawOp::Mesh {
                mesh,
                transform,
                paint,
                provenance,
                ..
            } => {
                if mesh.is_empty() {
                    continue;
                }
                absorb(&mut bounds, mesh.bounds(), *transform);
                // A stand-in shape, painted so a reviewer can see it is a stand-in — the same
                // default and the same colour the GPU painter uses, so the two agree.
                let warning = PaintProgram::Solid(crate::PLACEHOLDER_WARNING);
                let paint = if highlight_placeholders && provenance.is_placeholder() {
                    &warning
                } else {
                    paint
                };
                let placement = match paint {
                    PaintProgram::Picture { destination, .. } => {
                        Some(Placement::covering(*destination))
                    }
                    _ => None,
                };
                let Some(path) = mesh_path(mesh, *transform) else {
                    continue;
                };
                fill_path(
                    pool, handle, &path, paint, *transform, placement, &clip, &textures, width,
                    height,
                )?;
            }
            DrawOp::Glyphs {
                quads,
                transform,
                paint,
                ..
            } => {
                for quad in quads {
                    let rect =
                        SceneRect::new(quad.x, quad.y, quad.x + quad.width, quad.y + quad.height);
                    absorb(&mut bounds, rect, *transform);
                    let Some(path) = quad_path(quad, *transform) else {
                        continue;
                    };
                    fill_path(
                        pool,
                        handle,
                        &path,
                        paint,
                        *transform,
                        Some(Placement::sampling(
                            rect,
                            SceneRect::new(
                                quad.u,
                                quad.v,
                                quad.u + quad.texel_width,
                                quad.v + quad.texel_height,
                            ),
                        )),
                        &clip,
                        &textures,
                        width,
                        height,
                    )?;
                }
            }
            DrawOp::Picture {
                destination,
                transform,
                paint,
                ..
            } => {
                absorb(&mut bounds, *destination, *transform);
                let Some(path) = rect_path(*destination, *transform) else {
                    continue;
                };
                fill_path(
                    pool,
                    handle,
                    &path,
                    paint,
                    *transform,
                    Some(Placement::covering(*destination)),
                    &clip,
                    &textures,
                    width,
                    height,
                )?;
            }
            DrawOp::PushClip {
                mesh, transform, ..
            } => {
                clips.push(clip.clone());
                clip = clip.intersect(mesh, *transform, width, height);
            }
            DrawOp::PopClip { .. } => {
                clip = clips.pop().unwrap_or_default();
            }
            DrawOp::Composite { layer: child, .. } => {
                let Some(steps) = finished.get(child) else {
                    continue;
                };
                for step in steps {
                    composite(pool.texture_mut(handle)?, step, &clip);
                }
            }
        }
    }
    Ok(bounds)
}

/// Rasterise `path` and shade it with `paint` into a pooled canvas.
#[allow(
    clippy::too_many_arguments,
    reason = "one draw's whole description, threaded once"
)]
fn fill_path(
    pool: &mut TexturePool<Canvas>,
    handle: PoolHandle,
    path: &tiny_skia::Path,
    paint: &PaintProgram,
    transform: SceneTransform,
    placement: Option<Placement>,
    clip: &ClipState,
    textures: &Textures<'_>,
    width: u32,
    height: u32,
) -> Result<(), PaintError> {
    // The mask is the size of the draw's own bounding box rather than of the viewport. See the
    // module documentation: a paragraph is a thousand quads, and a thousand full-viewport masks is
    // a hundred megabytes of memset for one line of text.
    let device = path.bounds();
    let left = (device.left().floor().max(0.0) as u32).min(width);
    let top = (device.top().floor().max(0.0) as u32).min(height);
    let right = (device.right().ceil().max(0.0) as u32).min(width);
    let bottom = (device.bottom().ceil().max(0.0) as u32).min(height);
    if right <= left || bottom <= top {
        return Ok(());
    }
    let Some(mut mask) = tiny_skia::Mask::new(right - left, bottom - top) else {
        return Ok(());
    };
    mask.fill_path(
        path,
        tiny_skia::FillRule::Winding,
        true,
        tiny_skia::Transform::from_translate(-(left as f32), -(top as f32)),
    );
    let Some(shading) = Shading::new(paint, transform, placement, textures) else {
        return Ok(());
    };

    let mask_width = mask.width();
    let coverage = mask.data();
    let canvas = pool.texture_mut(handle)?;
    for y in top..bottom {
        for x in left..right {
            let at = ((y - top) * mask_width + (x - left)) as usize;
            let mut alpha = f32::from(coverage.get(at).copied().unwrap_or(0)) / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            alpha *= clip.at(x, y);
            if alpha <= 0.0 {
                continue;
            }
            let value = shading.at(x as f32 + 0.5, y as f32 + 0.5);
            canvas.blend(
                x,
                y,
                [
                    value[0] * alpha,
                    value[1] * alpha,
                    value[2] * alpha,
                    value[3] * alpha,
                ],
                BlendMode::Over,
            );
        }
    }
    Ok(())
}

/// Shade `paint` over `bounds` into a bitmap of its own — a mask layer's fill.
fn paint_bitmap(
    atlas: &HashMap<u32, Bitmap>,
    images: &HashMap<u64, Bitmap>,
    paint: &PaintProgram,
    bounds: SceneRect,
    transform: SceneTransform,
    width: u32,
    height: u32,
) -> Bitmap {
    let mut bytes = vec![0u8; (width as usize) * (height as usize) * 4];
    let textures = Textures { atlas, images };
    let (Some(shading), Some(inverse)) = (
        Shading::new(
            paint,
            transform,
            Some(Placement::covering(bounds)),
            &textures,
        ),
        invert(transform),
    ) else {
        return Bitmap {
            width,
            height,
            channels: 4,
            bytes,
        };
    };
    for y in 0..height {
        for x in 0..width {
            // Only inside the box the fill covers, which is what makes a gradient across a line of
            // text run across *the text* rather than across the page.
            let (local_x, local_y) = apply(inverse, x as f32 + 0.5, y as f32 + 0.5);
            if local_x < bounds.left
                || local_x > bounds.right
                || local_y < bounds.top
                || local_y > bounds.bottom
            {
                continue;
            }
            let value = shading.at(x as f32 + 0.5, y as f32 + 0.5);
            let at = ((y as usize * width as usize) + x as usize) * 4;
            for (index, channel) in value.iter().enumerate() {
                if let Some(slot) = bytes.get_mut(at + index) {
                    *slot = (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
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

/// Run an effect DAG over a subtree's pixels and answer the parent's composite steps.
///
/// The same shape as the GPU painter's: every node but the root is materialised so that a chain — a
/// blur of a glow — needs no special case, and only the root's steps reach the parent.
fn evaluate_effects(
    nodes: &[EffectNode],
    subtree: &Bitmap,
    content: SceneRect,
    width: u32,
    height: u32,
) -> Vec<Step> {
    if nodes.is_empty() {
        return vec![Step::blit(subtree.clone())];
    }
    let mut outputs: Vec<Bitmap> = Vec::with_capacity(nodes.len());
    let mut root_steps = Vec::new();
    for (position, node) in nodes.iter().enumerate() {
        let input = match node.input.and_then(|index| outputs.get(index)) {
            Some(bitmap) => bitmap.clone(),
            None => subtree.clone(),
        };
        let steps = effect_steps(node, &input, content, width, height);
        if position + 1 == nodes.len() {
            root_steps = steps;
            break;
        }
        // A node that is not the root becomes a bitmap, because the node above it takes one.
        let Some(mut canvas) = Canvas::new(width, height) else {
            break;
        };
        for step in &steps {
            composite(&mut canvas, step, &ClipState::Open);
        }
        outputs.push(canvas.as_bitmap());
    }
    root_steps
}

/// One effect node's own output, and the subtree placed relative to it.
fn effect_steps(
    node: &EffectNode,
    input: &Bitmap,
    content: SceneRect,
    width: u32,
    height: u32,
) -> Vec<Step> {
    let radius = finite_or(node.effect.radius, 0.0).max(0.0);
    let direction = finite_or(node.effect.direction, 0.0);
    let distance = finite_or(node.effect.distance, 0.0).max(0.0);
    let offset = SceneTransform {
        scale_x: 1.0,
        shear_y: 0.0,
        shear_x: 0.0,
        scale_y: 1.0,
        translate_x: distance * direction.cos(),
        translate_y: distance * direction.sin(),
    };
    let mut steps = Vec::new();
    match node.effect.kind {
        EffectKind::Blur => {
            steps.push(Step::blit(blur(input, radius, width, height)));
        }
        EffectKind::Glow | EffectKind::OuterShadow => {
            // Blur then tint, which is the GPU painter's order and one full-viewport pass fewer than
            // tinting first: a tint reads only alpha, and blurring alpha then colouring it is the
            // same picture as colouring then blurring.
            let blurred = blur(input, radius.max(1.0), width, height);
            let mut tint = Step::blit(blurred);
            tint.kind = StepKind::Tint;
            tint.color = node.color;
            if node.effect.kind == EffectKind::OuterShadow {
                tint.transform = offset;
            }
            steps.push(tint);
        }
        EffectKind::InnerShadow => {
            let blurred = blur(input, radius.max(1.0), width, height);
            let mut inner = Step::blit(blurred);
            inner.kind = StepKind::InnerShadow;
            inner.second = Some(input.clone());
            inner.color = node.color;
            // The offset moves the **blurred copy** and not the mask, exactly as the shader does.
            // Putting it on the quad — which is what R08's painter did — moves both inputs and
            // pushes the shadow outside the shape it is inside.
            inner.source_offset = [
                -offset.translate_x / width.max(1) as f32,
                -offset.translate_y / height.max(1) as f32,
            ];
            steps.push(inner);
        }
        EffectKind::SoftEdge => {
            let blurred = blur(input, radius.max(1.0), width, height);
            let mut soft = Step::blit(blurred);
            soft.kind = StepKind::MaskBySourceAlpha;
            soft.second = Some(input.clone());
            steps.push(soft);
        }
        EffectKind::Reflection => {
            // Flipped about the bottom of what the subtree actually covers, not about the viewport:
            // a reflection about the page's own edge is a reflection of nothing in particular.
            let axis = content.bottom;
            let flip = SceneTransform {
                scale_x: finite_or(node.effect.scale_x, 1.0),
                shear_y: 0.0,
                shear_x: 0.0,
                scale_y: -finite_or(node.effect.scale_y, 1.0),
                translate_x: distance * direction.cos(),
                translate_y: 2.0 * axis + distance * direction.sin(),
            };
            let start_position = finite_or(node.effect.start_position, 0.0);
            let end_position = finite_or(node.effect.end_position, 1.0);
            let mut reflected = Step::blit(input.clone());
            reflected.kind = StepKind::Fade;
            reflected.transform = flip;
            reflected.fade = [
                finite_or(node.effect.start_alpha, 1.0),
                finite_or(node.effect.end_alpha, 0.0),
                start_position,
                end_position.max(start_position + 0.001),
            ];
            reflected.fade_axis = 1.0;
            steps.push(reflected);
        }
        EffectKind::FillOverlay => {
            let mut overlay = Step::blit(input.clone());
            overlay.kind = StepKind::Tint;
            overlay.color = node.color;
            overlay.blend = node.effect.blend;
            steps.push(overlay);
        }
    }
    // **The same two predicates the GPU painter reads**, in the same order, so that a change to
    // either moves both painters' pixels together. See `crate::plan::draws_behind` for what those
    // two functions decided before MJXOFF-164, which was nothing.
    if !replaces_subtree(node.effect.kind) {
        if draws_behind(node.effect.kind) {
            steps.push(Step::blit(input.clone()));
        } else {
            steps.insert(0, Step::blit(input.clone()));
        }
    }
    steps
}

/// Draw one composite step onto a canvas.
///
/// The full-viewport quad the GPU painter draws, read the other way round: for each destination
/// pixel, where in the source it comes from. A transformed quad and an inverse-mapped read are the
/// same picture, and the inverse map is what a processor can do without a rasteriser.
fn composite(canvas: &mut Canvas, step: &Step, clip: &ClipState) {
    let (width, height) = (canvas.width(), canvas.height());
    let Some(inverse) = invert(step.transform) else {
        return;
    };
    for y in 0..height {
        for x in 0..width {
            let (source_x, source_y) = apply(inverse, x as f32 + 0.5, y as f32 + 0.5);
            let u = source_x / width.max(1) as f32;
            let v = source_y / height.max(1) as f32;
            // Outside the quad there is nothing to draw, exactly as a GPU rasterises nothing
            // outside the triangles it was given.
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let alpha = step.alpha * clip.at(x, y);
            if alpha <= 0.0 {
                continue;
            }
            let value = step_value(step, u, v);
            let source = [
                value[0] * alpha,
                value[1] * alpha,
                value[2] * alpha,
                value[3] * alpha,
            ];
            let destination = canvas.pixel(x, y);
            canvas.set(x, y, blend(source, destination, step.blend));
        }
    }
}

/// What one composite step reads at a point, mirroring the shader's effect branches.
fn step_value(step: &Step, u: f32, v: f32) -> [f32; 4] {
    let source = step
        .source
        .sample(u + step.source_offset[0], v + step.source_offset[1], false);
    match step.kind {
        StepKind::Blit => source,
        StepKind::Tint => {
            // A shadow and a glow are the subtree's *silhouette* in one colour: its alpha decides
            // where and the colour decides what. Reading the RGB too would give a shadow of the
            // picture rather than of its outline.
            let ink = premultiplied_color(step.color);
            scale(ink, source[3])
        }
        StepKind::MaskBySourceAlpha => match &step.second {
            Some(second) => scale(second.sample(u, v, false), source[3]),
            None => [0.0; 4],
        },
        StepKind::InnerShadow => match &step.second {
            Some(second) => {
                // Inside the shape's own alpha, wherever the blurred *inverse* of that alpha
                // reaches.
                let outside = 1.0 - source[3];
                let inside = second.sample(u, v, false)[3];
                scale(premultiplied_color(step.color), outside * inside)
            }
            None => [0.0; 4],
        },
        StepKind::Fade => {
            let along = if step.fade_axis > 0.5 { v } else { u }.clamp(0.0, 1.0);
            let span = (step.fade[3] - step.fade[2]).max(0.0001);
            let position = ((along - step.fade[2]) / span).clamp(0.0, 1.0);
            scale(
                source,
                step.fade[0] + (step.fade[1] - step.fade[0]) * position,
            )
        }
    }
}

/// A premultiplied colour, scaled — which for premultiplied values is one multiply on all four
/// channels rather than an un-premultiply, a scale and a re-premultiply.
fn scale(value: [f32; 4], by: f32) -> [f32; 4] {
    [value[0] * by, value[1] * by, value[2] * by, value[3] * by]
}

/// Make sure every picture the plan names has been decoded into a bitmap.
fn take_images(images: &mut HashMap<u64, Bitmap>, plan: &FramePlan, source: &dyn ImageSource) {
    for layer in plan.layers() {
        for op in &layer.ops {
            let handle = match op {
                DrawOp::Picture { paint, .. } | DrawOp::Mesh { paint, .. } => match paint {
                    PaintProgram::Picture { handle, .. } => *handle,
                    _ => continue,
                },
                _ => continue,
            };
            if images.contains_key(&handle) {
                continue;
            }
            let Some(pixels) = source.pixels(handle) else {
                // A handle the caller has no picture for. The area is drawn as nothing rather than
                // the frame failing, which is what the GPU painter does with the same case: a
                // missing picture is a document problem, not a rendering one.
                continue;
            };
            if pixels.width == 0 || pixels.height == 0 {
                continue;
            }
            images.insert(
                handle,
                Bitmap {
                    width: pixels.width,
                    height: pixels.height,
                    channels: 4,
                    bytes: pixels.rgba.to_vec(),
                },
            );
        }
    }
}

/// The triangles of a mesh as one path, every triangle wound the same way.
///
/// Non-zero filling then unions them with no seam: the winding number is at least one on both sides
/// of an edge two triangles share, so `tiny-skia` computes coverage for the *union* rather than for
/// each triangle in turn. Filling them one at a time would leave an antialiased hairline everywhere
/// `lyon` happened to cut, which on a large shape is a visible mesh of pale lines.
fn mesh_path(mesh: &Mesh, transform: SceneTransform) -> Option<tiny_skia::Path> {
    let positions = mesh.positions();
    let indices = mesh.indices();
    let mut builder = tiny_skia::PathBuilder::new();
    let point = |vertex: u32| -> (f32, f32) {
        let at = vertex as usize * 2;
        let x = positions.get(at).copied().unwrap_or_default();
        let y = positions.get(at + 1).copied().unwrap_or_default();
        apply(transform, x, y)
    };
    for triangle in indices.chunks(3) {
        let (Some(a), Some(b), Some(c)) = (
            triangle.first().copied(),
            triangle.get(1).copied(),
            triangle.get(2).copied(),
        ) else {
            continue;
        };
        let (a, b, c) = (point(a), point(b), point(c));
        let area = (b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1);
        if !area.is_finite() || area == 0.0 {
            continue;
        }
        let (first, second, third) = if area > 0.0 { (a, b, c) } else { (a, c, b) };
        builder.move_to(first.0, first.1);
        builder.line_to(second.0, second.1);
        builder.line_to(third.0, third.1);
        builder.close();
    }
    builder.finish()
}

/// A glyph quad as a path, under the run's transform.
fn quad_path(quad: &GlyphQuad, transform: SceneTransform) -> Option<tiny_skia::Path> {
    rect_path(
        SceneRect::new(quad.x, quad.y, quad.x + quad.width, quad.y + quad.height),
        transform,
    )
}

/// A rectangle as a path under a transform — a quadrilateral once the transform shears it.
fn rect_path(rect: SceneRect, transform: SceneTransform) -> Option<tiny_skia::Path> {
    let corners = [
        apply(transform, rect.left, rect.top),
        apply(transform, rect.right, rect.top),
        apply(transform, rect.right, rect.bottom),
        apply(transform, rect.left, rect.bottom),
    ];
    let mut builder = tiny_skia::PathBuilder::new();
    let first = corners.first()?;
    builder.move_to(first.0, first.1);
    for corner in corners.iter().skip(1) {
        builder.line_to(corner.0, corner.1);
    }
    builder.close();
    builder.finish()
}

/// Grow `bounds` to contain a transformed rectangle.
fn absorb(bounds: &mut Option<SceneRect>, rect: SceneRect, transform: SceneTransform) {
    for (x, y) in [
        (rect.left, rect.top),
        (rect.right, rect.top),
        (rect.right, rect.bottom),
        (rect.left, rect.bottom),
    ] {
        let (x, y) = apply(transform, x, y);
        *bounds = Some(match *bounds {
            Some(existing) => SceneRect::new(
                existing.left.min(x),
                existing.top.min(y),
                existing.right.max(x),
                existing.bottom.max(y),
            ),
            None => SceneRect::new(x, y, x, y),
        });
    }
}

/// The atlas store: what turns a glyph atlas's delta into bytes this painter can sample.
struct AtlasStore<'a> {
    pages: &'a mut HashMap<u32, Bitmap>,
    created: usize,
    released: usize,
    bytes: usize,
}

impl AtlasVisitor for AtlasStore<'_> {
    fn page_released(&mut self, page: u32) -> Result<(), PaintError> {
        self.pages.remove(&page);
        self.released += 1;
        Ok(())
    }

    fn page_created(&mut self, page: AtlasPage) -> Result<(), PaintError> {
        let side = u32::from(page.size);
        if side == 0 {
            return Err(PaintError::TextureUnavailable {
                width: side,
                height: side,
                detail: "an atlas page with no area".to_owned(),
            });
        }
        let channels = bytes_per_pixel(page.format);
        self.pages.insert(
            page.page,
            Bitmap {
                width: side,
                height: side,
                channels,
                bytes: vec![0u8; (side as usize) * (side as usize) * channels],
            },
        );
        self.created += 1;
        Ok(())
    }

    fn write(&mut self, write: AtlasWrite<'_>) -> Result<(), PaintError> {
        let Some(page) = self.pages.get_mut(&write.page) else {
            // A write for a page nobody created. The atlas reports creations before writes, so this
            // is a painter being handed a partial delta; saying so beats writing into whichever page
            // happens to hold that index.
            return Err(PaintError::TextureUnavailable {
                width: u32::from(write.width),
                height: u32::from(write.height),
                detail: format!("no storage for atlas page {}", write.page),
            });
        };
        let channels = bytes_per_pixel(write.format);
        if channels != page.channels {
            return Err(PaintError::TextureUnavailable {
                width: u32::from(write.width),
                height: u32::from(write.height),
                detail: format!(
                    "atlas page {} was created with {} bytes per pixel and written with {channels}",
                    write.page, page.channels
                ),
            });
        }
        let stride = usize::from(write.width) * channels;
        for row in 0..usize::from(write.height) {
            let from = row * stride;
            let Some(source) = write.pixels.get(from..from + stride) else {
                break;
            };
            let y = usize::from(write.y) + row;
            let into = (y * page.width as usize + usize::from(write.x)) * channels;
            let Some(slot) = page.bytes.get_mut(into..into + stride) else {
                continue;
            };
            slot.copy_from_slice(source);
        }
        self.bytes += write.pixels.len();
        Ok(())
    }
}
