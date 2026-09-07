//! Paths become triangles: the tessellator, its cache, and the scale the two are keyed on.
//!
//! # Why the triangles are made here and not in the painter
//!
//! Three reasons, and each of them is a gate somewhere above.
//!
//! **Determinism.** The same path at the same tolerance must become the same triangles on every
//! platform, or R10's golden images compare a Linux render against a Windows one and disagree about
//! nothing. A tessellator that lived in a painter would be a tessellator per backend.
//!
//! **Testability without a GPU.** Everything below is `f32` arithmetic and `Vec`s. A triangle count
//! and a bounding box are assertions a test runner can make; a frame buffer is not.
//!
//! **One answer, four consumers.** A GPU painter, a software painter, a PDF exporter and an SVG
//! exporter all want the same interior. Tessellating once, above the display list and below every
//! painter, is what stops the four disagreeing about what a self-intersecting path encloses.
//!
//! It is also why this platform's vector rendering is a *tessellation* pipeline rather than a
//! compute-shader one: `wgpu`'s WebGL2 backend has no compute stage and the browser is a target.
//!
//! # Why there is no new display-list section
//!
//! MJXOFF-161 left wire value 14 free and the rule that a new section must be pinned to a
//! hand-written byte string in `tests/the_encoding_is_pinned_to_literals.rs`. **No section is added
//! here, and 14 is still free** — deliberately, because triangles do not belong in a
//! [`DisplayList`]:
//!
//! * a display list is *diffed frame to frame with one `memcmp`* and *cached to disk*, and a page's
//!   triangles are far larger than the page's description of itself;
//! * triangles are **derived** data — recomputable from the list, at a tolerance the list does not
//!   fix — so storing them would put a cache inside a value that is itself cached;
//! * and the cache below is keyed on `(path, style, scale bucket)` and shared **across pages and
//!   across frames**. Meshes living inside one page's bytes could be shared with nothing, which
//!   makes the cache this child is asked for and meshes-in-the-list mutually exclusive designs.
//!
//! `crates/mjx-scene/src/lib.rs` says the same thing from the other side: tessellation lives *above*
//! the display list. This module is that place.
//!
//! # What is deliberately not tessellated
//!
//! **Line ends** — `a:headEnd` and `a:tailEnd`, the arrowheads. [`StrokeGeometry`] does not carry
//! them and no triangle below is one. That is the same decision as preset geometry and for the same
//! reason: ECMA-376 names the six shapes and the three sizes and states **no proportions at all**,
//! so a tessellator that drew them would be inventing fidelity data. The proportions are owned by
//! the geometry programme (MJXOFF-88) alongside `presetShapeDefinitions`; [`Stroke`]'s `head` and
//! `tail` fields carry the document's intent through the display list unchanged in the meantime.
//!
//! **Compound-line proportions are a choice, and are written down as one.** ECMA-376 §20.1.10.23
//! names `dbl`, `thickThin`, `thinThick` and `tri` and states no widths either. The bands in
//! `compound_bands` are this renderer's reading, declared as constants rather than buried in
//! arithmetic, so that the geometry programme can replace them with a measured table without
//! touching the stroker.

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use lyon_tessellation::path::Path as LyonPath;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule as LyonFillRule, FillTessellator, FillVertex,
    LineCap as LyonCap, LineJoin as LyonJoin, StrokeOptions, StrokeTessellator, StrokeVertex,
    VertexBuffers,
};
use mjx_text::{DeviceScale, ScaleBucket};

use crate::command::Command;
use crate::error::SceneError;
use crate::geometry::{finite, FillRule, Geometry, PathCommand, SceneRect};
use crate::glyphs::SceneGlyphRun;
use crate::list::DisplayList;
use crate::mesh_cache::{MeshCache, MeshKey};
use crate::paint::{
    CompoundStroke, DashPattern, LineCap, LineJoin, Stroke, StrokeAlignment, StrokeStyle,
};
use crate::provider::{GeometryProvider, OutlineProvenance};

/// How far a flattened curve may stray from the true curve, in device pixels.
///
/// A quarter of a pixel is below what a display resolves and well below what an eye does, and it is
/// the figure every one of this project's reference renderers uses. Halving it roughly doubles the
/// triangle count for a difference nobody can see.
pub const TOLERANCE_DEVICE_PIXELS: f32 = 0.25;

/// The largest coordinate a tessellated path may name, in device pixels.
///
/// Paths come out of documents and a document may say anything. The clamp is not tidiness: a round
/// join on a stroke whose coordinates are `1e30` is an arc of radius `1e30` subdivided to a quarter
/// of a pixel, which is not a slow render but an unbounded one. A page at ten thousand per cent zoom
/// is under a million pixels across, so nothing a reader can see is clamped.
pub const COORDINATE_LIMIT: f32 = 1.0e6;

/// The widest a stroke may be, in device pixels, for the same reason.
pub const STROKE_WIDTH_LIMIT: f32 = 1.0e4;

/// The most triangles one path may become before the tessellator refuses it.
pub const TRIANGLE_LIMIT: usize = 1 << 21;

/// The most dash segments one contour may be cut into.
///
/// Past this the contour is drawn **solid**: a hairline dash on a page-long path is tens of millions
/// of segments, and a picture nobody can distinguish from a solid line is better drawn as one than
/// spent an hour on.
pub const DASH_SEGMENT_LIMIT: usize = 65_536;

/// How many bytes of triangles a [`Tessellator`] keeps by default.
pub const DEFAULT_MESH_CACHE_BYTES: usize = 16 * 1024 * 1024;

/// The nominal em, in points, that a page's own scale is bucketed against.
///
/// [`ScaleBucket`] quantises a size in *pixels per em* and a display list's paths are in device
/// pixels rather than ems, so [`page_bucket`] applies the same quantiser to the em a twelve-point
/// body text would have. Twelve points because that is the size the rest of this workspace treats as
/// nominal; the number matters only in that a page and the glyphs on it are bucketed by one scheme
/// rather than two.
pub const REFERENCE_EM_POINTS: f32 = 12.0;

// -------------------------------------------------------------------------------------------
// The mesh
// -------------------------------------------------------------------------------------------

/// A tessellated path: interleaved `x, y` positions in device pixels, and the triangles over them.
///
/// Positions and indices rather than a vertex struct, because that is the shape a GPU buffer wants
/// and because it is the shape whose bytes can be compared: [`Mesh::vertex_bytes`] is little-endian
/// on every platform, so a determinism gate compares the same bytes wherever it runs.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Mesh {
    positions: Vec<f32>,
    indices: Vec<u32>,
}

impl Mesh {
    /// The vertex positions, `x` then `y`, two entries per vertex.
    #[must_use]
    pub fn positions(&self) -> &[f32] {
        &self.positions
    }

    /// The triangles, three indices each, into [`Mesh::positions`] by vertex.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// How many vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 2
    }

    /// How many triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Whether it draws nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// The smallest rectangle containing every vertex, or [`SceneRect::EMPTY`] for an empty mesh.
    #[must_use]
    pub fn bounds(&self) -> SceneRect {
        let mut edges: Option<(f32, f32, f32, f32)> = None;
        for pair in self.positions.as_chunks::<2>().0 {
            let (x, y) = (pair[0], pair[1]);
            edges = Some(match edges {
                None => (x, y, x, y),
                Some((left, top, right, bottom)) => {
                    (left.min(x), top.min(y), right.max(x), bottom.max(y))
                }
            });
        }
        match edges {
            None => SceneRect::EMPTY,
            Some((left, top, right, bottom)) => SceneRect {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    /// The positions as little-endian bytes — a vertex buffer, and the thing a determinism gate
    /// compares.
    #[must_use]
    pub fn vertex_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.positions.len() * size_of::<f32>());
        for value in &self.positions {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// The indices as little-endian bytes — an index buffer.
    #[must_use]
    pub fn index_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.indices.len() * size_of::<u32>());
        for value in &self.indices {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// How many bytes this mesh holds — the figure the cache's budget charges.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.positions.capacity() * size_of::<f32>() + self.indices.capacity() * size_of::<u32>()
    }

    /// Append `buffers` to this mesh, shifting its indices past the vertices already here.
    ///
    /// # Errors
    ///
    /// [`SceneError::Tessellation`] once the result would pass [`TRIANGLE_LIMIT`].
    fn append(&mut self, buffers: &VertexBuffers<[f32; 2], u32>) -> Result<(), SceneError> {
        if self.triangle_count() + buffers.indices.len() / 3 > TRIANGLE_LIMIT {
            return Err(SceneError::Tessellation {
                reason: "the path becomes more triangles than one mesh may hold",
            });
        }
        let base = u32::try_from(self.vertex_count()).map_err(|_| SceneError::Tessellation {
            reason: "the path becomes more vertices than an index can name",
        })?;
        for position in &buffers.vertices {
            // A `-0.0` and a `0.0` draw the same pixel and compare equal, but their *bytes* differ,
            // which would make a pinned vertex buffer depend on which side of an edge lyon happened
            // to approach a coordinate from. `x + 0.0` is `+0.0` for both and is the identity for
            // everything else.
            self.positions.push(finite(position[0]) + 0.0);
            self.positions.push(finite(position[1]) + 0.0);
        }
        for index in &buffers.indices {
            self.indices
                .push(index.checked_add(base).ok_or(SceneError::Tessellation {
                    reason: "the path becomes more vertices than an index can name",
                })?);
        }
        Ok(())
    }

    /// Give back whatever capacity the growth strategy over-allocated.
    ///
    /// Called once, when a mesh is finished. It is what makes [`Mesh::byte_len`] both *accurate* —
    /// capacity is what the allocator was actually asked for — and *stable*, which a cache asserting
    /// a byte bound needs it to be.
    fn shrink(&mut self) {
        self.positions.shrink_to_fit();
        self.indices.shrink_to_fit();
    }
}

// -------------------------------------------------------------------------------------------
// What is tessellated, and at what scale
// -------------------------------------------------------------------------------------------

/// The geometry of a stroke: how wide its outline is, with nothing about what colours it.
///
/// A [`Stroke`] record and a [`StrokeStyle`] both carry a paint, and the paint has no effect
/// whatever on the triangles. Dropping it here is what stops two strokes that differ only in colour
/// occupying two cache entries.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StrokeGeometry {
    /// How wide, in device pixels.
    pub width: f32,
    /// How the ends are finished.
    pub cap: LineCap,
    /// How the corners are finished.
    pub join: LineJoin,
    /// Which preset dash.
    pub dash: DashPattern,
    /// Where the stroke sits relative to the path.
    pub alignment: StrokeAlignment,
    /// How many parallel lines.
    pub compound: CompoundStroke,
}

impl StrokeGeometry {
    /// The geometry of a decoration's stroke.
    #[must_use]
    pub fn from_style(style: &StrokeStyle) -> Self {
        Self {
            width: style.width,
            cap: style.cap,
            join: style.join,
            dash: style.dash,
            alignment: style.alignment,
            compound: style.compound,
        }
    }

    /// The geometry of a stroke record read out of a display list.
    #[must_use]
    pub fn from_record(stroke: Stroke) -> Self {
        Self {
            width: stroke.width,
            cap: stroke.cap,
            join: stroke.join,
            dash: stroke.dash,
            alignment: stroke.alignment,
            compound: stroke.compound,
        }
    }
}

/// At what scale, and to what tolerance, a path is turned into triangles.
///
/// The bucket is [`mjx_text::ScaleBucket`] — the quantiser the glyph cache already uses, not a
/// second scheme — and it is in the cache key even though the tolerance is too: a painter that
/// magnifies a mesh magnifies its flattening error with it, so two callers at two scales must never
/// share one entry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TessellationOptions {
    bucket: ScaleBucket,
    tolerance: f32,
}

impl TessellationOptions {
    /// Tessellate for `bucket` at [`TOLERANCE_DEVICE_PIXELS`].
    ///
    /// The tolerance every path already expressed in device pixels wants — which is every path in a
    /// display list, because that is the unit a display list is in.
    #[must_use]
    pub fn for_bucket(bucket: ScaleBucket) -> Self {
        Self {
            bucket,
            tolerance: TOLERANCE_DEVICE_PIXELS,
        }
    }

    /// Tessellate for `bucket` at a tolerance the caller chooses.
    ///
    /// For a path in a space that is scaled before it is drawn, where a quarter of a device pixel at
    /// the destination is not a quarter of a unit at the source. The value is clamped to something a
    /// flattener can act on; zero and infinity are both numbers a document can produce.
    #[must_use]
    pub fn with_tolerance(bucket: ScaleBucket, tolerance: f32) -> Self {
        Self {
            bucket,
            tolerance: clamp_tolerance(tolerance),
        }
    }

    /// The options a glyph run's outlines are tessellated at, read out of the run's own record.
    ///
    /// A run carries the bucket its glyphs were scaled for and the residual factor a painter
    /// multiplies the whole run by — both put there by `build_scene`, and neither invented here. The
    /// outline is in the bucket's own pixels and will be drawn `residual_scale` times larger, so a
    /// quarter of a device pixel at the destination is that much less at the source.
    #[must_use]
    pub fn for_glyph_run(run: &SceneGlyphRun) -> Self {
        let residual = finite(run.residual_scale);
        let tolerance = if residual > 0.0 {
            TOLERANCE_DEVICE_PIXELS / residual
        } else {
            TOLERANCE_DEVICE_PIXELS
        };
        Self {
            bucket: ScaleBucket::from_steps(run.bucket_steps),
            tolerance: clamp_tolerance(tolerance),
        }
    }

    /// Which scale bucket.
    #[must_use]
    pub fn bucket(self) -> ScaleBucket {
        self.bucket
    }

    /// How far a flattened curve may stray from the true one, in the path's own units.
    #[must_use]
    pub fn tolerance(self) -> f32 {
        self.tolerance
    }
}

/// The scale bucket a page's own paths tessellate in.
///
/// See [`REFERENCE_EM_POINTS`] for why a pixels-per-em quantiser is the right one for a page.
#[must_use]
pub fn page_bucket(scale: DeviceScale) -> ScaleBucket {
    ScaleBucket::enclosing(scale.pixels_per_point() * REFERENCE_EM_POINTS)
}

// -------------------------------------------------------------------------------------------
// The tessellator
// -------------------------------------------------------------------------------------------

/// Turns paths into triangles, and remembers the answers.
///
/// One per painter. It owns lyon's two tessellators — both of which reuse their working buffers
/// between calls, which is most of why a page of shapes allocates as little as it does — and the
/// mesh cache.
pub struct Tessellator {
    fill: FillTessellator,
    stroke: StrokeTessellator,
    cache: MeshCache,
}

impl fmt::Debug for Tessellator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tessellator")
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

impl Default for Tessellator {
    fn default() -> Self {
        Self::new()
    }
}

impl Tessellator {
    /// A tessellator whose cache holds [`DEFAULT_MESH_CACHE_BYTES`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_budget(DEFAULT_MESH_CACHE_BYTES)
    }

    /// A tessellator whose cache holds `budget` bytes.
    #[must_use]
    pub fn with_budget(budget: usize) -> Self {
        Self {
            fill: FillTessellator::new(),
            stroke: StrokeTessellator::new(),
            cache: MeshCache::new(budget),
        }
    }

    /// What it is remembering.
    #[must_use]
    pub fn cache(&self) -> &MeshCache {
        &self.cache
    }

    /// Forget every cached tessellation, keeping the budget.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// The triangles that fill `geometry`, resolving it through `provider` if nobody has yet.
    ///
    /// # Errors
    ///
    /// Whatever `provider` fails with, and [`SceneError::Tessellation`] for a path that becomes more
    /// triangles than one mesh may hold. A degenerate path — zero length, self-intersecting, or
    /// carrying coordinates a document made up — is not an error: it produces an empty or a clamped
    /// mesh, because these paths come out of real files.
    pub fn fill(
        &mut self,
        geometry: &Geometry,
        provider: &dyn GeometryProvider,
        options: TessellationOptions,
    ) -> Result<Arc<Mesh>, SceneError> {
        self.fill_resolved(geometry, provider, options)
            .map(|(mesh, _)| mesh)
    }

    /// The same triangles, and **where the outline behind them came from**.
    ///
    /// The variant a painter uses. [`Tessellator::fill`] is the same call with the answer dropped,
    /// which is what almost every caller wants; this one exists because a painter that cannot tell
    /// a placeholder from the document's own shape cannot refuse to call a page of placeholders a
    /// fidelity render, and MJXOFF-163 established that it could not tell.
    ///
    /// # Errors
    ///
    /// As [`Tessellator::fill`].
    pub fn fill_resolved(
        &mut self,
        geometry: &Geometry,
        provider: &dyn GeometryProvider,
        options: TessellationOptions,
    ) -> Result<(Arc<Mesh>, Provenance), SceneError> {
        let (commands, fill_rule, provenance) = outline_of(geometry, provider)?;
        let key = MeshKey::new(fill_key_words(&commands, fill_rule, options));
        if let Some(held) = self.cache.get(&key) {
            return Ok((held, provenance));
        }
        let mut mesh = Mesh::default();
        let path = lyon_path_of(&commands);
        let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        let fill_options =
            FillOptions::tolerance(options.tolerance).with_fill_rule(match fill_rule {
                FillRule::NonZero => LyonFillRule::NonZero,
                FillRule::EvenOdd => LyonFillRule::EvenOdd,
            });
        self.fill
            .tessellate_path(
                &path,
                &fill_options,
                &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| {
                    vertex.position().to_array()
                }),
            )
            .map_err(|_| SceneError::Tessellation {
                reason: "the interior of the path could not be trapezoidated",
            })?;
        mesh.append(&buffers)?;
        mesh.shrink();
        let mesh = Arc::new(mesh);
        self.cache.insert(key, &mesh);
        Ok((mesh, provenance))
    }

    /// The triangles that stroke `geometry` with `stroke`, resolving it through `provider` if
    /// nobody has yet.
    ///
    /// # Errors
    ///
    /// As [`Tessellator::fill`].
    pub fn stroke(
        &mut self,
        geometry: &Geometry,
        stroke: StrokeGeometry,
        provider: &dyn GeometryProvider,
        options: TessellationOptions,
    ) -> Result<Arc<Mesh>, SceneError> {
        self.stroke_resolved(geometry, stroke, provider, options)
            .map(|(mesh, _)| mesh)
    }

    /// The same outline, and where the geometry behind it came from.
    ///
    /// See [`Tessellator::fill_resolved`] for why this variant exists.
    ///
    /// # Errors
    ///
    /// As [`Tessellator::fill`].
    pub fn stroke_resolved(
        &mut self,
        geometry: &Geometry,
        stroke: StrokeGeometry,
        provider: &dyn GeometryProvider,
        options: TessellationOptions,
    ) -> Result<(Arc<Mesh>, Provenance), SceneError> {
        let (commands, _, provenance) = outline_of(geometry, provider)?;
        let key = MeshKey::new(stroke_key_words(&commands, stroke, options));
        if let Some(held) = self.cache.get(&key) {
            return Ok((held, provenance));
        }
        let mut mesh = self.tessellate_stroke(&commands, stroke, options)?;
        mesh.shrink();
        let mesh = Arc::new(mesh);
        self.cache.insert(key, &mesh);
        Ok((mesh, provenance))
    }

    /// The stroke, before it reaches the cache.
    fn tessellate_stroke(
        &mut self,
        commands: &[PathCommand],
        stroke: StrokeGeometry,
        options: TessellationOptions,
    ) -> Result<Mesh, SceneError> {
        let width = clamp_width(stroke.width);
        let mut mesh = Mesh::default();
        if width <= 0.0 {
            return Ok(mesh);
        }
        let bands = compound_bands(stroke.compound, width);
        let dash = dash_lengths(stroke.dash, width);
        let path = lyon_path_of(commands);

        // The overwhelmingly common stroke — one band, on the centreline, undashed — is tessellated
        // from the *curved* path, so its round joins and its curves are lyon's own and not a
        // polyline's. Everything else needs the path flattened first, because an offset or a dash is
        // an operation on a polyline.
        if bands.len() == 1
            && bands[0].offset == 0.0
            && dash.is_empty()
            && stroke.alignment == StrokeAlignment::Centered
        {
            self.stroke_path_into(&mut mesh, &path, width, stroke, options)?;
            return Ok(mesh);
        }

        let inset = match stroke.alignment {
            StrokeAlignment::Centered => 0.0,
            StrokeAlignment::Inset => width / 2.0,
        };
        let miter_limit = match stroke.join {
            LineJoin::Miter { limit } => limit,
            LineJoin::Round | LineJoin::Bevel => DEFAULT_MITER_LIMIT,
        };
        for contour in flatten(&path, options.tolerance) {
            // Offsetting moves along the left normal, and for a closed contour the left normal
            // points *inward* exactly when the shoelace area is positive — which is what turns
            // `Inset` from a direction nobody agreed on into one the winding decides.
            let inward = signed_area(&contour).signum();
            for band in &bands {
                let distance = band.offset + inset * if contour.closed { inward } else { 0.0 };
                let placed = if distance == 0.0 {
                    Cow::Borrowed(&contour)
                } else {
                    Cow::Owned(offset_contour(&contour, distance, miter_limit))
                };
                for piece in dash_contour(&placed, &dash) {
                    let piece_path = lyon_path_of_contour(&piece);
                    self.stroke_path_into(&mut mesh, &piece_path, band.width, stroke, options)?;
                }
            }
        }
        Ok(mesh)
    }

    /// Stroke one path at one width and append the triangles.
    fn stroke_path_into(
        &mut self,
        mesh: &mut Mesh,
        path: &LyonPath,
        width: f32,
        stroke: StrokeGeometry,
        options: TessellationOptions,
    ) -> Result<(), SceneError> {
        if width <= 0.0 {
            return Ok(());
        }
        let cap = match stroke.cap {
            LineCap::Flat => LyonCap::Butt,
            LineCap::Round => LyonCap::Round,
            LineCap::Square => LyonCap::Square,
        };
        let mut stroke_options = StrokeOptions::tolerance(options.tolerance)
            .with_line_width(width)
            .with_start_cap(cap)
            .with_end_cap(cap)
            .with_line_join(match stroke.join {
                LineJoin::Round => LyonJoin::Round,
                LineJoin::Bevel => LyonJoin::Bevel,
                LineJoin::Miter { .. } => LyonJoin::Miter,
            });
        if let LineJoin::Miter { limit } = stroke.join {
            stroke_options = stroke_options.with_miter_limit(clamp_miter_limit(limit));
        }
        let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        self.stroke
            .tessellate_path(
                path,
                &stroke_options,
                &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| {
                    vertex.position().to_array()
                }),
            )
            .map_err(|_| SceneError::Tessellation {
                reason: "the outline of the path could not be stroked",
            })?;
        mesh.append(&buffers)
    }
}

// -------------------------------------------------------------------------------------------
// A whole display list
// -------------------------------------------------------------------------------------------

/// Whether a mesh fills a path or strokes it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshRole {
    /// The interior — a [`Command::FillPath`].
    Fill,
    /// The outline — a [`Command::StrokePath`].
    Stroke,
}

/// Where a mesh's outline came from, and what it is called.
///
/// # Why this travels with the triangles
///
/// A [`Mesh`] is a vertex buffer and cannot say whether it is a document's shape or a stand-in for
/// one. [`ResolvedOutline`](crate::ResolvedOutline) can say, and did, and until MJXOFF-163 nothing
/// read it: the answer was dropped inside this module's `outline_of`, at the single point where the
/// crate consumes a provider. So a painter could not paint a placeholder in a warning colour, and — the
/// reason it matters — **a golden-image gate could not refuse to call a page of placeholders a
/// fidelity render**, which is exactly the guard R10 was told to rely on.
///
/// The label is a `Box<str>` and is `None` for geometry that never went through a provider, which is
/// most of a page: an already-resolved path and a rectangle are the document's own by construction
/// and have nothing to be labelled. Where it is present it is *moved* out of the provider's answer
/// rather than copied, so carrying it costs one pointer per mesh and no allocation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Provenance {
    /// Whether this is the document's geometry or a stand-in for it.
    pub origin: OutlineProvenance,
    /// What the provider called it, for an outline that went through one.
    pub label: Option<Box<str>>,
}

impl Provenance {
    /// The document's own geometry, unlabelled.
    #[must_use]
    pub const fn document() -> Self {
        Self {
            origin: OutlineProvenance::Document,
            label: None,
        }
    }

    /// Whether this outline is a stand-in.
    #[must_use]
    pub const fn is_placeholder(&self) -> bool {
        matches!(self.origin, OutlineProvenance::Placeholder)
    }
}

/// One command's triangles.
#[derive(Clone, PartialEq, Debug)]
pub struct SceneMesh {
    /// Which command of the list produced it, counting from zero in paint order.
    pub command: usize,
    /// Interior or outline.
    pub role: MeshRole,
    /// The triangles.
    pub mesh: Arc<Mesh>,
    /// Where the outline behind them came from.
    ///
    /// **A painter must be able to tell a stand-in from the document's own shape**, and this is the
    /// only place it can learn it: the mesh is a vertex buffer and the display list's
    /// [`Geometry::Unresolved`] says only that *somebody* had to resolve it, not what they answered.
    pub provenance: Provenance,
}

/// Every mesh a display list needs, in paint order.
///
/// The whole of what a painter has to do before it can draw a page, and the whole of what changes
/// when the geometry provider changes: substituting a different [`GeometryProvider`] here produces a
/// different vertex buffer and nothing above has to know. That is the property
/// `tests/the_geometry_seam_is_swappable.rs` asserts, and it is what will let the real preset table
/// be dropped in later without touching a painter.
///
/// # Errors
///
/// Whatever `provider` fails with, and whatever [`Tessellator::fill`] does.
pub fn tessellate_scene(
    list: &DisplayList,
    provider: &dyn GeometryProvider,
    tessellator: &mut Tessellator,
) -> Result<Vec<SceneMesh>, SceneError> {
    let options = TessellationOptions::for_bucket(page_bucket(list.device_scale()));
    let mut meshes = Vec::new();
    for (command_index, command) in list.commands().enumerate() {
        match command {
            Command::FillPath { geometry, .. } => {
                let Some(geometry) = list.geometry(geometry) else {
                    continue;
                };
                let (mesh, provenance) = tessellator.fill_resolved(&geometry, provider, options)?;
                meshes.push(SceneMesh {
                    command: command_index,
                    role: MeshRole::Fill,
                    mesh,
                    provenance,
                });
            }
            Command::StrokePath { geometry, stroke } => {
                let (Some(geometry), Some(stroke)) = (list.geometry(geometry), list.stroke(stroke))
                else {
                    continue;
                };
                let (mesh, provenance) = tessellator.stroke_resolved(
                    &geometry,
                    StrokeGeometry::from_record(stroke),
                    provider,
                    options,
                )?;
                meshes.push(SceneMesh {
                    command: command_index,
                    role: MeshRole::Stroke,
                    mesh,
                    provenance,
                });
            }
            _ => {}
        }
    }
    Ok(meshes)
}

// -------------------------------------------------------------------------------------------
// Turning a geometry into a path
// -------------------------------------------------------------------------------------------

/// The path a geometry draws, which side of it is inside, and **where it came from**.
///
/// A [`Geometry::Path`] is borrowed rather than cloned: it is the case a page is full of, and a
/// cache hit that allocated a copy of the path in order to look itself up would be most of a cache
/// miss.
///
/// # Why the provenance leaves this function
///
/// Until MJXOFF-163 it did not. [`ResolvedOutline`](crate::ResolvedOutline) carried `provenance` and
/// `label`, this function destructured `commands` and `fill_rule` and dropped both, and nothing in
/// this crate read either — so a painter had no way at all to tell a placeholder rounded rectangle
/// from the document's own geometry. That mattered more than it looked: R10's fidelity rule is
/// *"golden images must not be taken against placeholder geometry and called fidelity"*, and every
/// preset shape resolves to a placeholder today, so the guard it names did not exist.
///
/// It leaves through [`SceneMesh::provenance`] rather than through [`Tessellator::fill`]'s ordinary
/// return, so that the one caller that needs it pays for it and the hundreds that do not are
/// unchanged. And it is read **before** the cache lookup, which is where `outline_of` already sat:
/// a cache hit answers with the same provenance as the miss that filled it, because the provider is
/// consulted either way.
fn outline_of<'a>(
    geometry: &'a Geometry,
    provider: &dyn GeometryProvider,
) -> Result<(Cow<'a, [PathCommand]>, FillRule, Provenance), SceneError> {
    let outline = resolve_outline(geometry, provider)?;
    Ok((outline.commands, outline.fill_rule, outline.provenance))
}

/// A geometry resolved into the steps that draw it — **the one interpretation of what a shape is**.
///
/// Answered by [`resolve_outline`], which is what [`Tessellator::fill_resolved`] and
/// [`Tessellator::stroke_resolved`] resolve their own input with.
#[derive(Clone, PartialEq, Debug)]
pub struct ResolvedGeometry<'a> {
    /// The steps that draw it, in device pixels. Borrowed for a [`Geometry::Path`] — the case a
    /// page is full of — and owned for the two that are computed.
    pub commands: Cow<'a, [PathCommand]>,
    /// Which side of the outline is inside it.
    pub fill_rule: FillRule,
    /// Whether this is the document's own shape or a stand-in for one, and what a provider called
    /// it.
    pub provenance: Provenance,
}

/// The outline a geometry draws, resolving it through `provider` if nobody has yet.
///
/// # Why this is public, and what it stops
///
/// [`Tessellator::fill_resolved`] answers with **triangles**, which is what a rasteriser wants and
/// what a *vector* exporter cannot use: a PDF or an SVG made of a shape's trapezoidation is a
/// hundred times the file, and every interior edge of it is a hairline seam in a viewer that
/// antialiases. MJXOFF-164's two exporters therefore need the outline itself.
///
/// The alternative to exposing it is each exporter deciding for itself what a
/// [`Geometry::Rectangle`] is, which side of a [`Geometry::Path`] is inside it and what to do with
/// a [`Geometry::Unresolved`] — **a second interpretation of the same shape**, which is precisely
/// what a display list exists to prevent. So there is one function, the tessellator calls it too,
/// and a change to what a shape means changes the triangles and the vector output together.
///
/// # Errors
///
/// Whatever `provider` fails with for a [`Geometry::Unresolved`]. The other two arms cannot fail.
pub fn resolve_outline<'a>(
    geometry: &'a Geometry,
    provider: &dyn GeometryProvider,
) -> Result<ResolvedGeometry<'a>, SceneError> {
    Ok(match geometry {
        Geometry::Rectangle(rect) => ResolvedGeometry {
            commands: Cow::Owned(rectangle_commands(*rect)),
            fill_rule: FillRule::NonZero,
            provenance: Provenance::document(),
        },
        Geometry::Path {
            commands,
            fill_rule,
            ..
        } => ResolvedGeometry {
            commands: Cow::Borrowed(commands.as_slice()),
            fill_rule: *fill_rule,
            provenance: Provenance::document(),
        },
        Geometry::Unresolved { outline, bounds } => {
            let resolved = provider.outline(*outline, *bounds)?;
            ResolvedGeometry {
                commands: Cow::Owned(resolved.commands),
                fill_rule: resolved.fill_rule,
                provenance: Provenance {
                    origin: resolved.provenance,
                    label: Some(resolved.label.into_boxed_str()),
                },
            }
        }
    })
}

/// A rectangle as a closed path.
fn rectangle_commands(rect: SceneRect) -> Vec<PathCommand> {
    use crate::geometry::ScenePoint;
    vec![
        PathCommand::MoveTo(ScenePoint::new(rect.left, rect.top)),
        PathCommand::LineTo(ScenePoint::new(rect.right, rect.top)),
        PathCommand::LineTo(ScenePoint::new(rect.right, rect.bottom)),
        PathCommand::LineTo(ScenePoint::new(rect.left, rect.bottom)),
        PathCommand::Close,
    ]
}

/// The path, in lyon's vocabulary, with every coordinate sanitised.
///
/// Steps before the first `MoveTo` are dropped rather than begun at the origin: a path that starts
/// with a `LineTo` is malformed, and inventing a start point for it would draw a line from the
/// page's corner that nothing in the document asks for.
fn lyon_path_of(commands: &[PathCommand]) -> LyonPath {
    use lyon_tessellation::math::point;
    let mut builder = LyonPath::builder();
    let mut open = false;
    for command in commands {
        match *command {
            PathCommand::MoveTo(at) => {
                if open {
                    builder.end(false);
                }
                builder.begin(point(clamp_coordinate(at.x), clamp_coordinate(at.y)));
                open = true;
            }
            PathCommand::LineTo(to) => {
                if open {
                    builder.line_to(point(clamp_coordinate(to.x), clamp_coordinate(to.y)));
                }
            }
            PathCommand::QuadraticTo { control, end } => {
                if open {
                    builder.quadratic_bezier_to(
                        point(clamp_coordinate(control.x), clamp_coordinate(control.y)),
                        point(clamp_coordinate(end.x), clamp_coordinate(end.y)),
                    );
                }
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                if open {
                    builder.cubic_bezier_to(
                        point(
                            clamp_coordinate(first_control.x),
                            clamp_coordinate(first_control.y),
                        ),
                        point(
                            clamp_coordinate(second_control.x),
                            clamp_coordinate(second_control.y),
                        ),
                        point(clamp_coordinate(end.x), clamp_coordinate(end.y)),
                    );
                }
            }
            PathCommand::Close => {
                if open {
                    builder.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        builder.end(false);
    }
    builder.build()
}

/// A finite coordinate inside [`COORDINATE_LIMIT`].
fn clamp_coordinate(value: f32) -> f32 {
    finite(value).clamp(-COORDINATE_LIMIT, COORDINATE_LIMIT)
}

/// A finite stroke width inside [`STROKE_WIDTH_LIMIT`].
fn clamp_width(value: f32) -> f32 {
    finite(value).clamp(0.0, STROKE_WIDTH_LIMIT)
}

/// A tolerance a flattener can act on.
fn clamp_tolerance(value: f32) -> f32 {
    let value = finite(value);
    if value > 0.0 {
        value.clamp(MINIMUM_TOLERANCE, MAXIMUM_TOLERANCE)
    } else {
        TOLERANCE_DEVICE_PIXELS
    }
}

/// A miter limit lyon can act on. Below one a miter is shorter than a bevel, which is not a shape.
fn clamp_miter_limit(value: f32) -> f32 {
    let value = finite(value);
    if value >= 1.0 {
        value.min(MAXIMUM_MITER_LIMIT)
    } else {
        1.0
    }
}

/// The finest flattening this tessellator will attempt, in path units.
///
/// Public because it is a **bound a caller has to know about**: a tolerance below this is clamped,
/// not honoured, and a caller magnifying a path by a large factor can ask for one without meaning
/// to. It is also the whole reason [`TessellationOptions::with_tolerance`] clamps at all — a
/// tolerance of `1e-30` on a curve is not a finer picture but an unbounded one.
pub const MINIMUM_TOLERANCE: f32 = 1.0e-3;

/// The coarsest, for the same reason from the other end: past this a curve is a straight line and
/// asking for more buys nothing.
pub const MAXIMUM_TOLERANCE: f32 = 1.0e3;
/// What a join that states no limit is held to — DrawingML's own default.
const DEFAULT_MITER_LIMIT: f32 = 4.0;
/// The longest miter this tessellator will draw, as a multiple of the width.
const MAXIMUM_MITER_LIMIT: f32 = 100.0;

// -------------------------------------------------------------------------------------------
// Polylines: flattening, offsetting, dashing
// -------------------------------------------------------------------------------------------

/// One contour of a flattened path.
#[derive(Clone, PartialEq, Debug)]
struct Contour {
    points: Vec<[f32; 2]>,
    closed: bool,
}

/// The path as polylines, at `tolerance`.
fn flatten(path: &LyonPath, tolerance: f32) -> Vec<Contour> {
    use lyon_tessellation::path::iterator::PathIterator;
    use lyon_tessellation::path::PathEvent;

    let mut contours: Vec<Contour> = Vec::new();
    let mut current: Option<Contour> = None;
    for event in path.iter().flattened(tolerance) {
        match event {
            PathEvent::Begin { at } => {
                current = Some(Contour {
                    points: vec![[at.x, at.y]],
                    closed: false,
                });
            }
            PathEvent::Line { to, .. } => {
                if let Some(contour) = current.as_mut() {
                    contour.points.push([to.x, to.y]);
                }
            }
            PathEvent::End { close, .. } => {
                if let Some(mut contour) = current.take() {
                    contour.closed = close;
                    // A closed contour's last point is its first; carrying both would put a
                    // zero-length segment in every offset and every dash.
                    if close && contour.points.len() > 1 {
                        let first = contour.points[0];
                        if contour.points.last() == Some(&first) {
                            contour.points.pop();
                        }
                    }
                    if contour.points.len() > 1 {
                        contours.push(contour);
                    }
                }
            }
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => {}
        }
    }
    contours
}

/// A polyline as a lyon path.
fn lyon_path_of_contour(contour: &Contour) -> LyonPath {
    use lyon_tessellation::math::point;
    let mut builder = LyonPath::builder();
    let Some(first) = contour.points.first() else {
        return builder.build();
    };
    builder.begin(point(first[0], first[1]));
    for step in contour.points.iter().skip(1) {
        builder.line_to(point(step[0], step[1]));
    }
    builder.end(contour.closed);
    builder.build()
}

/// Twice the area the contour encloses, signed by its winding. Zero for an open one.
fn signed_area(contour: &Contour) -> f32 {
    if !contour.closed {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    let count = contour.points.len();
    for index in 0..count {
        let a = contour.points[index];
        let b = contour.points[(index + 1) % count];
        sum += a[0] * b[1] - b[0] * a[1];
    }
    sum
}

/// The contour moved `distance` along its left normal, mitred at the corners.
///
/// An approximate offset — the honest kind, and the one every renderer without an exact offsetter
/// uses. It is exact for a straight segment and within the miter limit at a corner, which is all a
/// compound line or an inset stroke needs: those offsets are fractions of a line width, far below
/// the curvature radius of anything a document draws.
fn offset_contour(contour: &Contour, distance: f32, miter_limit: f32) -> Contour {
    let count = contour.points.len();
    if count < 2 {
        return contour.clone();
    }
    let segments = if contour.closed { count } else { count - 1 };
    let mut normals: Vec<[f32; 2]> = Vec::with_capacity(segments);
    for index in 0..segments {
        let a = contour.points[index];
        let b = contour.points[(index + 1) % count];
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy);
        normals.push(if length > 0.0 {
            [-dy / length, dx / length]
        } else {
            [0.0, 0.0]
        });
    }
    let limit = clamp_miter_limit(miter_limit);
    let mut points = Vec::with_capacity(count);
    for index in 0..count {
        let incoming = if contour.closed {
            normals[(index + segments - 1) % segments]
        } else {
            normals[index.saturating_sub(1)]
        };
        let outgoing = if contour.closed {
            normals[index % segments]
        } else {
            normals[index.min(segments - 1)]
        };
        let sum = [incoming[0] + outgoing[0], incoming[1] + outgoing[1]];
        let length = sum[0].hypot(sum[1]);
        let (unit, cosine) = if length > 1.0e-6 {
            let unit = [sum[0] / length, sum[1] / length];
            (unit, (unit[0] * outgoing[0] + unit[1] * outgoing[1]).abs())
        } else {
            // A reversal: the two normals cancel, and there is no bisector to follow. The outgoing
            // normal is the honest answer and it never produces a spike.
            (outgoing, 1.0)
        };
        let reach = distance / cosine.max(1.0 / limit);
        let at = contour.points[index];
        points.push([at[0] + unit[0] * reach, at[1] + unit[1] * reach]);
    }
    Contour {
        points,
        closed: contour.closed,
    }
}

/// The pattern's dash and gap lengths, in device pixels, or empty for a solid line.
///
/// The multipliers are ECMA-376's preset dash names read as the multiples of the line width every
/// renderer of this format uses; a `solid` line has no pattern at all rather than a pattern of one
/// infinite dash.
///
/// # Why this is public
///
/// A rasteriser gets its dashes for free — the tessellator cuts the path here and hands over
/// triangles. A **vector exporter** does not: an SVG writes `stroke-dasharray` and a PDF writes a
/// `d` array, and both need these same eleven number pairs. Each deciding for itself what `lgDashDot`
/// means would be a third and fourth interpretation of a preset the specification names and does not
/// measure, and the three would drift.
#[must_use]
pub fn dash_lengths(dash: DashPattern, width: f32) -> Vec<f32> {
    let multiples: &[f32] = match dash {
        DashPattern::Solid => return Vec::new(),
        DashPattern::Dot => &[1.0, 3.0],
        DashPattern::Dash => &[4.0, 3.0],
        DashPattern::LargeDash => &[8.0, 3.0],
        DashPattern::DashDot => &[4.0, 3.0, 1.0, 3.0],
        DashPattern::LargeDashDot => &[8.0, 3.0, 1.0, 3.0],
        DashPattern::LargeDashDotDot => &[8.0, 3.0, 1.0, 3.0, 1.0, 3.0],
        DashPattern::SystemDash => &[3.0, 1.0],
        DashPattern::SystemDot => &[1.0, 1.0],
        DashPattern::SystemDashDot => &[3.0, 1.0, 1.0, 1.0],
        DashPattern::SystemDashDotDot => &[3.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    };
    multiples.iter().map(|multiple| multiple * width).collect()
}

/// The contour cut into its dashes. One piece, unchanged, for a solid line.
fn dash_contour(contour: &Contour, pattern: &[f32]) -> Vec<Contour> {
    if pattern.is_empty() || contour.points.len() < 2 {
        return vec![contour.clone()];
    }
    let period: f32 = pattern.iter().sum();
    let count = contour.points.len();
    let segments = if contour.closed { count } else { count - 1 };
    let mut length = 0.0_f32;
    for index in 0..segments {
        let a = contour.points[index];
        let b = contour.points[(index + 1) % count];
        length += (b[0] - a[0]).hypot(b[1] - a[1]);
    }
    if period <= 0.0 || !period.is_finite() || length <= 0.0 {
        return vec![contour.clone()];
    }
    // See `DASH_SEGMENT_LIMIT`: a dash finer than the limit is drawn solid.
    let estimate = (length / period) * pattern.len() as f32;
    if estimate >= DASH_SEGMENT_LIMIT as f32 || !estimate.is_finite() {
        return vec![contour.clone()];
    }

    let mut pieces: Vec<Contour> = Vec::new();
    let mut index = 0_usize;
    let mut remaining = pattern[0];
    let mut drawing = true;
    let mut current: Vec<[f32; 2]> = vec![contour.points[0]];
    for step in 0..segments {
        let a = contour.points[step];
        let b = contour.points[(step + 1) % count];
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let span = dx.hypot(dy);
        if span <= 0.0 {
            continue;
        }
        let mut travelled = 0.0_f32;
        while span - travelled > remaining {
            travelled += remaining;
            let at = travelled / span;
            let cut = [a[0] + dx * at, a[1] + dy * at];
            if drawing {
                current.push(cut);
                if current.len() > 1 {
                    pieces.push(Contour {
                        points: std::mem::take(&mut current),
                        closed: false,
                    });
                } else {
                    current.clear();
                }
            } else {
                current = vec![cut];
            }
            drawing = !drawing;
            index = (index + 1) % pattern.len();
            remaining = pattern[index];
        }
        remaining -= span - travelled;
        if drawing {
            current.push(b);
        }
    }
    if drawing && current.len() > 1 {
        pieces.push(Contour {
            points: current,
            closed: false,
        });
    }
    pieces
}

// -------------------------------------------------------------------------------------------
// Compound lines
// -------------------------------------------------------------------------------------------

/// One parallel line of a compound stroke: how far off the centreline, and how wide.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Band {
    offset: f32,
    width: f32,
}

/// The bands `compound` draws, together spanning `width` about the centreline.
///
/// ECMA-376 §20.1.10.23 names the four compounds and states no proportions, so these are this
/// renderer's reading of the names: equal thirds for a double line, two-to-one for a thick-thin
/// pair, and a thin-thick-thin triple over sixths. They are constants rather than arithmetic so that
/// a measured table can replace them without touching the stroker.
fn compound_bands(compound: CompoundStroke, width: f32) -> Vec<Band> {
    /// A band as `(centre, width)`, both as fractions of the total width, with the centre measured
    /// from the centreline and positive towards the left normal.
    type Fractions = &'static [(f32, f32)];

    let fractions: Fractions = match compound {
        CompoundStroke::Single => &[(0.0, 1.0)],
        // Thirds: line, gap, line.
        CompoundStroke::Double => &[(-1.0 / 3.0, 1.0 / 3.0), (1.0 / 3.0, 1.0 / 3.0)],
        // Quarters, two of them the thick line: thick, gap, thin.
        CompoundStroke::ThickThin => &[(-0.25, 0.5), (0.375, 0.25)],
        // The mirror of it.
        CompoundStroke::ThinThick => &[(-0.375, 0.25), (0.25, 0.5)],
        // Sixths: thin, gap, thick, gap, thin.
        CompoundStroke::Triple => &[
            (-5.0 / 12.0, 1.0 / 6.0),
            (0.0, 1.0 / 3.0),
            (5.0 / 12.0, 1.0 / 6.0),
        ],
    };
    fractions
        .iter()
        .map(|(centre, share)| Band {
            offset: centre * width,
            width: share * width,
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// Cache keys
// -------------------------------------------------------------------------------------------

/// The tag that opens a fill's key.
const KEY_FILL: u32 = 1;
/// The tag that opens a stroke's key.
const KEY_STROKE: u32 = 2;

/// The key of a fill: the tag, the winding rule, the scale, and the path.
fn fill_key_words(
    commands: &[PathCommand],
    fill_rule: FillRule,
    options: TessellationOptions,
) -> Vec<u32> {
    let mut words = vec![
        KEY_FILL,
        match fill_rule {
            FillRule::NonZero => 0,
            FillRule::EvenOdd => 1,
        },
        options.bucket.steps(),
        options.tolerance.to_bits(),
    ];
    push_path_words(&mut words, commands);
    words
}

/// The key of a stroke: the tag, every geometric field of the stroke, the scale, and the path.
fn stroke_key_words(
    commands: &[PathCommand],
    stroke: StrokeGeometry,
    options: TessellationOptions,
) -> Vec<u32> {
    let (join_tag, miter_bits) = match stroke.join {
        LineJoin::Round => (0, 0),
        LineJoin::Bevel => (1, 0),
        LineJoin::Miter { limit } => (2, clamp_miter_limit(limit).to_bits()),
    };
    let mut words = vec![
        KEY_STROKE,
        clamp_width(stroke.width).to_bits(),
        match stroke.cap {
            LineCap::Flat => 0,
            LineCap::Round => 1,
            LineCap::Square => 2,
        },
        join_tag,
        miter_bits,
        dash_tag(stroke.dash),
        match stroke.alignment {
            StrokeAlignment::Centered => 0,
            StrokeAlignment::Inset => 1,
        },
        match stroke.compound {
            CompoundStroke::Single => 0,
            CompoundStroke::Double => 1,
            CompoundStroke::ThickThin => 2,
            CompoundStroke::ThinThick => 3,
            CompoundStroke::Triple => 4,
        },
        options.bucket.steps(),
        options.tolerance.to_bits(),
    ];
    push_path_words(&mut words, commands);
    words
}

/// Which dash, as a word.
fn dash_tag(dash: DashPattern) -> u32 {
    match dash {
        DashPattern::Solid => 0,
        DashPattern::Dot => 1,
        DashPattern::Dash => 2,
        DashPattern::LargeDash => 3,
        DashPattern::DashDot => 4,
        DashPattern::LargeDashDot => 5,
        DashPattern::LargeDashDotDot => 6,
        DashPattern::SystemDash => 7,
        DashPattern::SystemDot => 8,
        DashPattern::SystemDashDot => 9,
        DashPattern::SystemDashDotDot => 10,
    }
}

/// Append the path, as the **sanitised** coordinates that will actually be tessellated.
///
/// Sanitised rather than raw, so that two paths that differ only in a `NaN` this tessellator maps to
/// the same number key to the same entry rather than to two entries holding the same triangles.
fn push_path_words(words: &mut Vec<u32>, commands: &[PathCommand]) {
    words.reserve(commands.len() * 3);
    for command in commands {
        match *command {
            PathCommand::MoveTo(at) => {
                words.push(1);
                words.push(clamp_coordinate(at.x).to_bits());
                words.push(clamp_coordinate(at.y).to_bits());
            }
            PathCommand::LineTo(to) => {
                words.push(2);
                words.push(clamp_coordinate(to.x).to_bits());
                words.push(clamp_coordinate(to.y).to_bits());
            }
            PathCommand::QuadraticTo { control, end } => {
                words.push(3);
                words.push(clamp_coordinate(control.x).to_bits());
                words.push(clamp_coordinate(control.y).to_bits());
                words.push(clamp_coordinate(end.x).to_bits());
                words.push(clamp_coordinate(end.y).to_bits());
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                words.push(4);
                words.push(clamp_coordinate(first_control.x).to_bits());
                words.push(clamp_coordinate(first_control.y).to_bits());
                words.push(clamp_coordinate(second_control.x).to_bits());
                words.push(clamp_coordinate(second_control.y).to_bits());
                words.push(clamp_coordinate(end.x).to_bits());
                words.push(clamp_coordinate(end.y).to_bits());
            }
            PathCommand::Close => words.push(5),
        }
    }
}
