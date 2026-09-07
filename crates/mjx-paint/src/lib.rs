//! **The platform boundary.** Where a display list becomes pixels, and where — for the first time
//! in this workspace — code that is not pure Rust is allowed in.
//!
//! # What this crate is
//!
//! A [`Painter`] takes a [`DisplayList`](mjx_scene::DisplayList) and puts it on a
//! [`SurfaceHost`]. This crate defines that contract and ships the first implementation of it, the
//! `wgpu` painter; R09 adds three more — `tiny-skia`, PDF and SVG — against the same fifteen-odd
//! methods.
//!
//! # Rank 5.5, and the two things it does and does not buy
//!
//! `mjx-paint` sits **above the facade**, and that is the whole of what its rank buys: nothing in
//! the document graph — no format crate, no `mjx-ooxml`, and above all no binding — can then declare
//! an edge to a crate that links Vulkan, Metal or Direct3D. `bindings/mjx-python` must never grow a
//! GPU dependency, and at 5.5 it structurally cannot.
//!
//! **What the rank does not buy is the other direction.** `xtask/tests/layering.rs` refuses only an
//! edge that points up or sideways, so at 5.5 *every crate in the workspace is a legal dependency of
//! this one*. The architecture's second seam — *below a display list, nothing has heard of a font, a
//! layout algorithm or a document* — is therefore held here by an explicit manifest gate,
//! `tests/the_seam_holds.rs`, and by nothing else. `mjx-scene` could be given rank 1.7 so that the
//! layering test would refuse its illegal edge by name; **no rank can do that job for a painter, in
//! either direction**, and a reader who reaches for one should read that file first.
//!
//! # The two `CLAUDE.md` amendments this crate is the reason for
//!
//! Both are written into `CLAUDE.md` itself, with their reasoning, and both are checked rather than
//! asserted. They are restated here because this is the crate they are about.
//!
//! **The pure-Rust rule now governs the *document graph*, not the workspace.** A pixel cannot reach
//! a screen without the operating system's graphics stack, and `wgpu` links `ash` (Vulkan),
//! `metal`/`objc2` and `windows-rs`. Ranks 0 through the facade stay pure Rust; `mjx-paint` at 5.5
//! is the declared boundary. `mjx-scene` and everything below it stay pure Rust so the headless,
//! `wasm32`, export and test paths never require a GPU — and `tiny-skia` is a **required** second
//! painter in R09 precisely so that a fully pure-Rust path to pixels always exists.
//!
//! **This is the fourth crate with a local `#[allow(unsafe_code)]`.** There is exactly one
//! hand-written `unsafe` block in it: the one that hands a window's platform handles to
//! `wgpu::Instance::create_surface_unsafe`, because no safe API can promise that a raw window
//! handle outlives the surface made from it. That promise is the window system's invariant and the
//! shell's to keep, it is documented at the call site and stated as the caller's obligation on
//! [`DesktopWindow::new`], and **CI greps for it**: any `unsafe` in `crates/mjx-paint/src` that does
//! not carry the marker on its own line fails the build, in the same job that already guards
//! `bindings/*/src`. That job's own comment states the principle — *"a claim CI does not check is a
//! claim that quietly stops being true"*.
//!
//! # Nothing here panics
//!
//! A painter's inputs are untrusted twice over: the display list may have come off a disk or a wire,
//! and the graphics stack is not under this program's control either. A lost device, a surface that
//! outlived its window, an adapter that vanished when a laptop switched GPUs and a driver that
//! refuses a texture size are **normal runtime events**, and every one of them is a typed
//! [`PaintError`]. `wgpu`'s own uncaptured-error handler aborts the process; this crate replaces it
//! with one that collects.
//!
//! # A frame, end to end
//!
//! ```no_run
//! use mjx_paint::{OffscreenSurface, Painter, Resources, Viewport, WgpuPainter};
//! use mjx_scene::{DisplayList, PlaceholderGeometry};
//!
//! # fn run(list: &DisplayList) -> Result<(), mjx_paint::PaintError> {
//! let mut painter = WgpuPainter::offscreen()?;
//! // Which painter, and which backend. Never "did it work": a frame that rendered on a backend
//! // nobody asked for, or was skipped entirely, satisfies "did it work" perfectly.
//! println!("{} on {}", painter.name(), painter.backend());
//!
//! let mut host = OffscreenSurface::new(400, 300, 1.0);
//! let mut glyphs = mjx_paint::NoGlyphs;
//! let images = mjx_paint::NoImages;
//! let geometry = PlaceholderGeometry::new();
//!
//! let viewport = Viewport::covering(&host);
//! let frame = painter.begin(&mut host, viewport)?;
//! let mut resources = Resources::new(&mut glyphs, &geometry, &images);
//! let drawn = painter.draw(&frame, list, &mut resources)?;
//! let report = painter.end(frame)?;
//! assert_eq!(drawn.placeholders, 0, "this page is not a fidelity render");
//! let _ = report;
//! # Ok(())
//! # }
//! ```

// The fourth crate in the workspace to open this door, and the third justification of the same
// shape. There is **one** hand-written `unsafe` block below — `create_surface_unsafe`, in
// `backend/mod.rs` — and it is the only way to give a graphics API a window it did not create: the
// obligation is that the handles describe a live window that outlives the surface, which is the
// window system's invariant and the shell's to keep, and it is documented at the call site and on
// `DesktopWindow::new`. Every other `unsafe` in this crate's dependency tree belongs to `wgpu`.
//
// `.github/workflows/ci.yml` greps `crates/mjx-paint/src` for `unsafe` outside a comment and
// outside this attribute, and fails on any line that does not carry the marker
// `MJX-PAINT-SURFACE-UNSAFE`. A claim CI does not check is a claim that quietly stops being true.
#![allow(unsafe_code)]

pub mod backend;
pub mod compare;
pub mod error;
pub mod export;
pub mod font_source;
pub mod glyph_atlas;
pub mod gradient;
pub mod painter;
pub mod pattern;
pub mod plan;
pub mod pool;
pub mod resources;
pub mod software;
pub mod surface;

pub use backend::{PaintKind, WgpuPainter, PLACEHOLDER_WARNING};
pub use compare::{
    compare_painters, compare_renders, render_offscreen, render_once, Agreement, Disagreement,
    Render, ResourceFactory, DEFAULT_CHANNEL_TOLERANCE,
};
pub use error::PaintError;
pub use export::pdf::{PdfPainter, PDF_EXPORTER};
pub use export::svg::{SvgPainter, SVG_EXPORTER};
pub use font_source::{FaceLibrary, GLYPH_FILL_RULE};
pub use gradient::{GradientRamp, RAMP_TEXELS};
pub use painter::{
    AdapterKind, Antialiasing, BackendReport, Capabilities, DrawReport, Frame, FrameReport,
    GraphicsApi, Painter, Pixels,
};
pub use pattern::{cell_of, coverage_atlas, mask_of, PATTERN_MASKS, PATTERN_SIDE};
pub use plan::{
    plan_frame, plan_frame_with, DrawOp, FramePlan, Layer, LayerKind, OpOrigin, PaintProgram,
    PlanOptions, RunIdentity, VectorPath,
};
pub use pool::{PoolHandle, PoolStatistics, TexturePool, TextureSize, DEFAULT_TEXTURE_POOL_BYTES};
pub use resources::{
    AtlasPage, AtlasSource, AtlasVisitor, AtlasWrite, EmbeddableFace, FontSource, ImagePixels,
    ImageSource, NoFonts, NoGlyphs, NoImages, Resources,
};
pub use software::{SoftwarePainter, MAXIMUM_SOFTWARE_TARGET, SOFTWARE_PAINTER};
pub use surface::{
    DesktopWindow, OffscreenSurface, SurfaceHost, SurfaceTarget, Viewport, WindowHandles,
};
