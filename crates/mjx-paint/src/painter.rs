//! The contract every renderer in this platform implements, and the reports that make it possible
//! to tell **which** renderer actually ran.
//!
//! # Three calls, and why it is three
//!
//! `begin` acquires a target, `draw` puts a display list on it, `end` presents it. The split is not
//! ceremony: several lists go onto one frame (a page, then the selection highlights, then the
//! in-canvas UI, each of which is rebuilt on a different schedule), and acquiring a swapchain
//! texture per list would tear. It is also what lets `begin` fail on a lost surface — the frame
//! never opens — separately from `draw` failing on a malformed list.
//!
//! # The trap this module exists to close
//!
//! MJXOFF-155 §8 names this project's signature defect: **a gate that is green precisely when the
//! work is skipped.** A painter is the sharpest instance of it in the whole programme, because
//! `wgpu` will quietly hand back a software adapter, a different backend, or a smaller sample count
//! than was asked for, and "the frame rendered without error" is true in every one of those cases.
//!
//! So no method here answers *whether* it worked. [`Painter::backend`] answers **what is actually
//! running** — read off the live adapter, never off the request — and [`FrameReport`] answers what
//! that run actually did. A test that asserts `painter.name() == "wgpu"` and
//! `painter.backend().api == GraphicsApi::Vulkan` and `report.draw_calls > 0` cannot pass on a
//! machine where the painter was skipped; a test that asserts `draw(..).is_ok()` can, and would.
//!
//! # Object-safe on purpose
//!
//! `Frame` is a token rather than an associated type, so `dyn Painter` exists. R09 adds three more
//! implementations and R10's golden-image suite runs the same display list through all four and
//! compares; that suite is a loop over `&mut [Box<dyn Painter>]` or it is four copies of itself.

use mjx_scene::DisplayList;

use crate::error::PaintError;
use crate::pool::PoolStatistics;
use crate::resources::Resources;
use crate::surface::{SurfaceHost, Viewport};

/// Which graphics API is doing the work.
///
/// Reported from the live adapter. There is deliberately no `Unknown`: a painter that could not say
/// what it was running on would satisfy every assertion a test could write about it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum GraphicsApi {
    /// Vulkan — Linux, Android, and Windows where it is preferred.
    Vulkan,
    /// Metal — macOS and iOS.
    Metal,
    /// Direct3D 12 — Windows.
    Direct3D12,
    /// OpenGL ES 3 / WebGL 2. The baseline: it exists everywhere and has no compute stage, which is
    /// the constraint that chose a tessellation pipeline over a compute-driven one.
    OpenGl,
    /// WebGPU in a browser.
    WebGpu,
    /// No graphics API at all — a painter that rasterises on the CPU or writes a document.
    /// R09's `tiny-skia`, PDF and SVG painters report this.
    None,
}

impl GraphicsApi {
    /// How it is written in a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vulkan => "Vulkan",
            Self::Metal => "Metal",
            Self::Direct3D12 => "Direct3D 12",
            Self::OpenGl => "OpenGL ES / WebGL 2",
            Self::WebGpu => "WebGPU",
            Self::None => "none (no graphics API)",
        }
    }
}

/// What kind of device the adapter is.
///
/// **This is the field that distinguishes a rendered frame from a skipped one that looked
/// rendered.** A continuous-integration machine with no GPU can still run every gate in this crate
/// through a software Vulkan implementation, and that is a legitimate way to run them — but a suite
/// that could not tell the difference would report a hardware result it never obtained.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum AdapterKind {
    /// A discrete graphics card.
    DiscreteGpu,
    /// A GPU sharing the processor's memory.
    IntegratedGpu,
    /// A GPU presented by a hypervisor.
    VirtualGpu,
    /// A software implementation — `lavapipe`, `llvmpipe`, WARP. Real rendering, no hardware.
    Cpu,
    /// Something the driver did not classify.
    Other,
}

impl AdapterKind {
    /// How it is written in a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DiscreteGpu => "discrete GPU",
            Self::IntegratedGpu => "integrated GPU",
            Self::VirtualGpu => "virtual GPU",
            Self::Cpu => "software (CPU)",
            Self::Other => "unclassified",
        }
    }

    /// Whether pixels are being produced by hardware.
    #[must_use]
    pub const fn is_hardware(self) -> bool {
        matches!(self, Self::DiscreteGpu | Self::IntegratedGpu)
    }
}

/// How edges are smoothed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum Antialiasing {
    /// Multisampling, at this many samples per pixel.
    ///
    /// Chosen over analytic coverage and over a coverage-buffer scheme because **it runs on every
    /// backend from WebGL 2 to Vulkan**, which is the constraint that decided it. An antialiasing
    /// strategy that needed a compute stage would work on three of the five targets.
    Multisample(u32),
    /// Coverage computed per pixel by the rasteriser, as a CPU painter does it.
    Analytic,
    /// None: hard edges. What a one-sample fallback reports, and what an SVG or PDF exporter reports
    /// because the decision belongs to whatever opens the file.
    None,
}

impl Antialiasing {
    /// How many samples per pixel, or `1` where the question does not apply.
    #[must_use]
    pub const fn sample_count(self) -> u32 {
        match self {
            Self::Multisample(samples) => samples,
            Self::Analytic | Self::None => 1,
        }
    }
}

/// What is actually running.
///
/// Every field is read off the live device after it was created, never copied from the request. A
/// painter that reported what it *asked for* would report Vulkan on a machine that gave it OpenGL.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BackendReport {
    /// Which API.
    pub api: GraphicsApi,
    /// What kind of device.
    pub adapter: AdapterKind,
    /// What the driver calls the device.
    pub adapter_name: String,
    /// What the driver calls itself.
    pub driver: String,
    /// How edges are being smoothed, at the sample count that was actually granted.
    pub antialiasing: Antialiasing,
}

impl core::fmt::Display for BackendReport {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "{} on {} ({}), {:?}",
            self.api.label(),
            self.adapter_name,
            self.adapter.label(),
            self.antialiasing
        )?;
        if !self.driver.is_empty() {
            write!(formatter, ", driver {}", self.driver)?;
        }
        Ok(())
    }
}

/// What a painter can do, so a caller can decide what to ask of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Capabilities {
    /// How edges are smoothed.
    pub antialiasing: Antialiasing,
    /// The largest render target the device will make, in device pixels on a side.
    pub max_texture_size: u32,
    /// Whether nested path clipping is exact. A GPU painter clips with a stencil buffer and answers
    /// `true`; an exporter that can only clip to rectangles would answer `false` and a caller could
    /// then choose the other painter for a page that needs it.
    pub exact_path_clipping: bool,
    /// The byte budget of the effect texture pool.
    pub effect_texture_budget: usize,
}

/// A frame in progress.
///
/// A token, not a handle: the painter keeps the swapchain texture, the encoder and the layer stack,
/// and this says only *which* frame the caller is talking about. Two consequences, both wanted —
/// a frame cannot be ended twice (it is consumed by [`Painter::end`] and is not `Copy`), and a
/// frame handed to the wrong painter is [`PaintError::WrongFrame`] rather than a mix-up nobody
/// notices.
#[derive(PartialEq, Debug)]
pub struct Frame {
    id: u64,
    viewport: Viewport,
}

impl Frame {
    /// Open a frame with this identity. Called by a painter's own `begin`.
    #[must_use]
    pub const fn new(id: u64, viewport: Viewport) -> Self {
        Self { id, viewport }
    }

    /// Which frame this is.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// The rectangle and scale it is being drawn at.
    #[must_use]
    pub const fn viewport(&self) -> Viewport {
        self.viewport
    }
}

/// What one [`Painter::draw`] did.
///
/// Counters rather than a boolean, because "it worked" is exactly the claim that stays true when
/// nothing happened. A display list with nine commands in it that produced no draw calls is a
/// painter that walked the list and drew none of it.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct DrawReport {
    /// How many commands of the list were walked.
    pub commands: usize,
    /// How many draw calls were issued.
    pub draw_calls: usize,
    /// How many triangles those calls covered.
    pub triangles: usize,
    /// How many glyph quads were built.
    pub glyphs: usize,
    /// How many pictures were drawn.
    pub images: usize,
    /// How many offscreen layers were opened — one per non-identity opacity group and one per
    /// effect.
    pub layers: usize,
    /// How many clip regions were pushed.
    pub clips: usize,
    /// How many bytes of glyph pixels were uploaded. **Zero on a frame that drew the same words as
    /// the last one**, which is the whole point of taking a delta.
    pub atlas_bytes_uploaded: usize,
    /// How many atlas pages were created, and how many released, on this call.
    pub atlas_pages_created: usize,
    /// How many were released.
    pub atlas_pages_released: usize,
    /// How many draws used **stand-in geometry** rather than the document's own shape.
    ///
    /// The whole reason `mjx-scene` grew [`mjx_scene::SceneMesh::provenance`] in MJXOFF-163. Every
    /// preset shape in this platform resolves to a placeholder today, and a golden-image gate that
    /// could not see that would compare a page of framed, crossed rounded rectangles against a
    /// golden image of the same rounded rectangles and record it as parity. **R10 must assert this
    /// is zero before calling a render a fidelity render.**
    ///
    /// It is here, on the value `draw` hands back, rather than only inside the painter, because a
    /// field the painter reads and the caller cannot act on is the same defect one layer up.
    pub placeholders: usize,
}

impl DrawReport {
    /// Fold `other` into this one.
    pub fn absorb(&mut self, other: Self) {
        self.commands += other.commands;
        self.draw_calls += other.draw_calls;
        self.triangles += other.triangles;
        self.glyphs += other.glyphs;
        self.images += other.images;
        self.layers += other.layers;
        self.clips += other.clips;
        self.atlas_bytes_uploaded += other.atlas_bytes_uploaded;
        self.atlas_pages_created += other.atlas_pages_created;
        self.atlas_pages_released += other.atlas_pages_released;
        self.placeholders += other.placeholders;
    }
}

/// What one frame did, once it is finished.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FrameReport {
    /// Which frame.
    pub frame: u64,
    /// Everything its [`Painter::draw`] calls did, added up.
    pub drawn: DrawReport,
    /// How many device pixels wide the target was.
    pub width: u32,
    /// How tall.
    pub height: u32,
    /// How many samples per pixel it was rendered at.
    pub sample_count: u32,
    /// Whether the frame reached a window. `false` for an offscreen target, which is not a failure.
    pub presented: bool,
    /// What the effect texture pool did.
    pub pool: PoolStatistics,
}

/// Pixels read back out of a painter.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pixels {
    /// How many pixels wide.
    pub width: u32,
    /// How many pixels tall.
    pub height: u32,
    /// Non-premultiplied `RGBA`, row by row from the top, four bytes to the pixel and no padding.
    pub rgba: Vec<u8>,
}

impl Pixels {
    /// The four bytes at `(x, y)`, or `None` outside the image.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = (y as usize)
            .checked_mul(self.width as usize)?
            .checked_add(x as usize)?
            .checked_mul(4)?;
        let bytes = self.rgba.get(offset..offset + 4)?;
        Some([
            *bytes.first()?,
            *bytes.get(1)?,
            *bytes.get(2)?,
            *bytes.get(3)?,
        ])
    }

    /// How many pixels are not fully transparent.
    ///
    /// The cheapest honest answer to *"did anything get drawn"*, and the one every rendering gate in
    /// this crate leans on: a frame that reported five hundred draw calls and covered no pixels
    /// drew five hundred nothings.
    #[must_use]
    pub fn covered(&self) -> usize {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] != 0)
            .count()
    }

    /// How many distinct colours the image holds.
    ///
    /// A page rendered as one flat rectangle and a page rendered properly both have pixels; they do
    /// not have the same number of colours.
    #[must_use]
    pub fn distinct_colors(&self) -> usize {
        let mut seen: Vec<[u8; 4]> = Vec::new();
        for pixel in self.rgba.as_chunks::<4>().0 {
            let value = *pixel;
            if !seen.contains(&value) {
                seen.push(value);
                // A page with more colours than this is certainly not a flat fill, which is all the
                // callers of this ask; counting the rest would walk a megapixel for no answer.
                if seen.len() >= 4096 {
                    break;
                }
            }
        }
        seen.len()
    }
}

/// Something that turns a display list into pixels, or into a document.
///
/// # Errors
///
/// Every method can fail, and none of the failures is a bug: a lost device, a resized surface, an
/// unavailable adapter and a malformed display list are all ordinary runtime events.
pub trait Painter {
    /// Which painter this is — `"wgpu"`, and later `"tiny-skia"`, `"pdf"`, `"svg"`.
    ///
    /// A `&'static str` rather than an enumeration so that a painter outside this workspace can
    /// name itself, and so that a gate asserting *which* painter ran does not have to be extended
    /// every time one is added.
    fn name(&self) -> &'static str;

    /// What is actually running underneath, read off the live device.
    fn backend(&self) -> BackendReport;

    /// What it can do.
    fn capabilities(&self) -> Capabilities;

    /// Open a frame on `target`.
    ///
    /// # Errors
    ///
    /// [`PaintError::FrameAlreadyOpen`] if one already is; [`PaintError::EmptyViewport`] for a
    /// viewport with no pixels; [`PaintError::SurfaceLost`] if the target's surface could not be
    /// acquired, in which case the painter has already asked the host to redraw.
    fn begin(
        &mut self,
        target: &mut dyn SurfaceHost,
        viewport: Viewport,
    ) -> Result<Frame, PaintError>;

    /// Draw `list` onto the open frame.
    ///
    /// # Errors
    ///
    /// [`PaintError::WrongFrame`] for a token this painter is not drawing, and whatever the list
    /// and the resources fail with.
    fn draw(
        &mut self,
        frame: &Frame,
        list: &DisplayList,
        resources: &mut Resources<'_>,
    ) -> Result<DrawReport, PaintError>;

    /// Finish the frame and present it, if it is on a window.
    ///
    /// # Errors
    ///
    /// [`PaintError::WrongFrame`], and whatever the device says about the submission.
    fn end(&mut self, frame: Frame) -> Result<FrameReport, PaintError>;

    /// The last finished frame's pixels, for a painter that drew into memory.
    ///
    /// `None` for a painter whose output went to a window, or to a file. Not an error: a golden-image
    /// suite asks every painter and skips the ones that cannot answer.
    ///
    /// # Errors
    ///
    /// [`PaintError::Readback`] if the pixels exist but could not be brought back.
    fn read_pixels(&mut self) -> Result<Option<Pixels>, PaintError>;
}
