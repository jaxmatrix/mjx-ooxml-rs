//! The display list: what a page becomes once it is decided *what* to draw and before anything
//! decides *how*.
//!
//! # This is the upper of the architecture's two seams
//!
//! `docs/UI_PLATFORM_PLAN.md` §2 draws two horizontal cuts through the stack. `mjx-layout`'s
//! [`FragmentTree`](mjx_layout::FragmentTree) is the lower one: above it, nothing has heard of
//! OOXML. [`DisplayList`] is the upper one, and it is stronger — **below it, nothing has heard of a
//! font, a layout algorithm or a document either.**
//!
//! That is what lets four painters consume one output. A GPU painter, a software painter, a PDF
//! exporter and an SVG exporter all read the same nine commands and the same eight resource tables,
//! and none of them can tell a `.pptx` from a Markdown file from the plain-text box model
//! `mjx-layout`'s own tests carry. It is also what will later let the same bytes cross a transport
//! boundary without a redesign, because they are already bytes.
//!
//! # What is in here
//!
//! * [`Command`] — nine of them: `PushTransform`, `PushClip`, `PushOpacity`, `PushEffect`, `Pop`,
//!   `FillPath`, `StrokePath`, `DrawGlyphs`, `DrawImage`.
//! * [`FillStyle`] and [`Paint`] — solid, gradient, the 54 preset patterns, and pictures with the
//!   crop, tiling and adjustments DrawingML defines, in a vocabulary with no DrawingML in it.
//! * [`EffectStyle`] and [`Effect`] — shadows, glow, soft edges, reflection, blur, and the DAG that
//!   composes them. **Declared here as data and executed in R08/R09**: a scene says *what* effect
//!   applies, never how a shader achieves it.
//! * [`DisplayList`] — the flat, versioned, typed-record arena, readable without deserialisation,
//!   cacheable to disk, and diffable frame to frame with [`diff_frames`].
//! * [`build_scene`] — a [`FragmentTree`](mjx_layout::FragmentTree) in, a [`DisplayList`] out.
//!
//! # Why the bytes are the type
//!
//! A display list of a 50 000-glyph page cannot be a tree of allocated objects if it is to be built
//! per frame, diffed between frames and cached to disk. So a [`DisplayList`] owns exactly one
//! `Vec<u8>`: one allocation to build, one `memcmp` to diff, one `write` to cache. Every accessor
//! decodes a record out of that arena on the spot, and walking a page's commands allocates nothing.
//!
//! # Nothing here panics, and nothing here needs a compute shader
//!
//! Two constraints, both load-bearing.
//!
//! **A display list is untrusted input.** It may have come off a disk or a wire, so decoding one is
//! parsing: there is no `unwrap`, no `expect`, no `panic!` and no slice index on any path a caller
//! can reach with bytes it did not write, and every failure is a typed [`SceneError`].
//! `tests/a_malformed_list_errors.rs` holds that true against truncation at every length, a wrong
//! version, overlapping sections, unknown opcodes and out-of-range indices.
//!
//! **No construct in the vocabulary needs a compute shader.** `wgpu`'s WebGL2 backend has none, and
//! the browser is a target, so a compute-driven vector renderer cannot be the design — which is why
//! tessellation lives above this crate (R07) rather than in the painter, and why every paint here is
//! a fragment-shader operation over triangles.
//!
//! # Rank 1.7, and why not higher
//!
//! This crate sits **below `mjx-dml` at 2.0**, one step above the box model contract at 1.6. That is
//! not where a display list "naturally" falls in a reading of the stack — its consumers are painters
//! and they are far above — and it is deliberate. `xtask/tests/layering.rs` only refuses an edge
//! that points up or sideways, so a `mjx-scene` ranked above shared markup would make
//! `mjx-scene → mjx-dml` a legal downward edge and the guarantee this crate exists to hold would be
//! enforced by nothing. At 1.7 that edge is structurally impossible, exactly as `mjx-layout`'s 1.6
//! makes an edge to a format crate impossible. Everything this crate actually depends on —
//! `mjx-layout` (1.6), `mjx-text` (1.5), `mjx-tokens` (0.2), `mjx-ooxml-core` (0.0) — is below it,
//! so the rank costs nothing and buys the rule.

pub mod build;
pub mod command;
pub mod diff;
pub mod effect;
pub mod encoding;
pub mod error;
pub mod geometry;
pub mod glyphs;
pub mod list;
pub mod mesh_cache;
pub mod paint;
pub mod provider;
pub mod scene;
pub mod tessellate;

pub use build::SceneBuilder;
pub use command::{Clip, Command};
pub use diff::{diff_frames, FrameDiff, RecordChange, RecordChangeKind};
pub use effect::{BlendMode, Effect, EffectKind, EffectStyle};
pub use encoding::{ResourceIndex, SectionKind, HEADER_BYTES, MAGIC, SECTION_ROW_BYTES, VERSION};
pub use error::SceneError;
pub use geometry::{
    finite, pixels_from_emu, FillRule, Geometry, PathCommand, ScenePoint, SceneRect, SceneTransform,
};
pub use glyphs::{AtlasPlacement, GlyphImage, SceneGlyph, SceneGlyphRun};
pub use list::{Commands, DisplayList};
pub use mesh_cache::MeshCache;
pub use paint::{
    CompoundStroke, DashPattern, FillStyle, Gradient, GradientKind, GradientStop, Image,
    ImageAdjustments, ImageFillMode, LineCap, LineEnd, LineEndShape, LineEndSize, LineJoin, Paint,
    PathShade, PatternPreset, RectangleAnchor, Stroke, StrokeAlignment, StrokeStyle, TileFlip,
    PATTERN_PRESET_COUNT,
};
pub use provider::{
    GeometryProvider, OutlineProvenance, PlaceholderGeometry, ResolvedOutline,
    PLACEHOLDER_CORNER_FRACTION, PLACEHOLDER_FRAME_FRACTION,
};
pub use scene::{build_scene, Decoration, ResourceResolver, SceneOptions, DEFAULT_TEXT_COLOR};
pub use tessellate::{
    page_bucket, tessellate_scene, Mesh, MeshRole, SceneMesh, StrokeGeometry, TessellationOptions,
    Tessellator, COORDINATE_LIMIT, DASH_SEGMENT_LIMIT, DEFAULT_MESH_CACHE_BYTES, MAXIMUM_TOLERANCE,
    MINIMUM_TOLERANCE, REFERENCE_EM_POINTS, STROKE_WIDTH_LIMIT, TOLERANCE_DEVICE_PIXELS,
    TRIANGLE_LIMIT,
};

/// The one sRGB colour in this workspace, re-exported so that a painter written against this crate
/// does not have to name the design-token crate to say what colour something is.
pub use mjx_tokens::Color;

/// The five `mjx-text` types this crate's own surface is written in, re-exported for the same
/// reason [`Color`] is: **a painter must be able to read every record of a list, and ask for its
/// triangles, without naming a crate below this one.**
///
/// Not a convenience. [`AtlasPlacement::format`] *is* a [`BitmapFormat`]; [`SceneGlyphRun`]'s
/// direction and hinting are `mjx-text`'s enumerations; [`DisplayList::device_scale`] answers with a
/// [`DeviceScale`]; and [`page_bucket`] answers with a [`ScaleBucket`], which is what
/// [`TessellationOptions`] is keyed on. R08's own seam gate (MJXOFF-163) asserts that
/// `mjx-paint`'s source names neither `mjx-text` nor `mjx-layout`; without these five lines that
/// gate is **unachievable by inspection of this crate's surface**, and the painter would have to
/// reach past the display list to do the one thing the display list exists to let it do.
pub use mjx_text::{BitmapFormat, DeviceScale, Hinting, ScaleBucket, TextDirection};
