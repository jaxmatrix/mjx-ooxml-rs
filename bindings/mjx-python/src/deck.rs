//! [`Deck`] — the whole PowerPoint surface, 257 methods, from Python.
//!
//! ```python
//! import mjx_ooxml
//!
//! deck = mjx_ooxml.Deck.blank(mjx_ooxml.SlideSize.widescreen())
//! slide = deck.add_slide_from_layout(0)
//! title = deck.add_text_box(slide, "Quarterly results",
//!                           mjx_ooxml.ShapeBounds.from_inches(0.5, 0.4, 9.0, 1.2))
//! deck.set_shape_run_properties(
//!     slide, title,
//!     mjx_ooxml.CharacterPropertiesSpec().with_size_points(40).with_bold(True))
//! open("out.pptx", "wb").write(deck.save())
//! ```
//!
//! # One deck, one thread
//!
//! Almost every method takes `&mut self`, because reading a part materialises it. Each call
//! therefore takes a mutable borrow of the deck and **releases it before returning**. That is safe
//! precisely because of what this surface does *not* do:
//!
//! * **No method takes a callback.** Python can never run while a borrow is held, so a second
//!   borrow can never be live. The closure-taking methods one layer down —
//!   `Presentation::with_table_style` and the VML readers — are the ones left out, and this is why.
//! * **No method returns a view into the deck.** Iteration returns lists; byte windows return
//!   `bytes`; addresses return values. Nothing a caller holds can outlive the call that produced it.
//! * **No method reaches the package.** `Deck.remove_unused_parts`, `Deck.external_links` and
//!   `Deck.retarget_external_link` are three delegates, not an escape hatch.
//!
//! What remains is the ordinary rule for a `#[pyclass]` with interior mutability: **use one `Deck`
//! from one thread.** Two threads calling into the same deck will make one of them raise
//! `RuntimeError: Already borrowed`, not corrupt it — but the fix is to move the deck, not to guard
//! it. `Deck.open` and `Deck.save` release the interpreter lock for their duration, so opening one
//! deck per thread genuinely parallelises; nothing else does enough work to be worth it.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use mjx_ooxml as ooxml;

use crate::address::{ShapePath, ShapePathArg, SurfaceArg};
use crate::charts::{
    ChartAxisData, ChartData, ChartErrorBarData, ChartLabelScope, ChartLegendData,
    ChartPointFormatData, ChartSeriesData, ChartTrendlineData, ChartWorkbook,
    DanglingPointReference, DataLabelSettings, DataLabelSpec, ErrorBarSpec, TrendlineSpec,
};
use crate::content::{
    ActiveXControlSpec, DiagramContent, DiagramParts, DiagramRelationshipIds, ExternalLink,
    Hyperlink, InkReference, LayoutInfo, LinkedImage, MediaReference, OleObject, OleObjectSpec,
    PlaceholderInfo, ShapeInfo,
};
use crate::enums::{
    ActiveXPersistence, AxisOrientation, CellBorder, ChartKind, DiagramPartKind, GraphicFrameKind,
    LegendPosition, PlaceholderType, PresetShapeType, ShapeKind, SlideLayoutKind, TablePart,
    TableStylePart, TargetMode, TextAnchoring, TextDirection,
};
use crate::errors::to_py_err;
use crate::format::Format;
use crate::geometry::{
    BoundedAdjustment, CellMargins, Geometry, GuideContext, ShapeBounds, SlideSize, Transform2D,
};
use crate::measures::{Emu, IndentLevel};
use crate::paint::{ColorMap, EffectListSpec, FillSpec, LineSpec};
use crate::support::{as_str_slice, RangeArg};
use crate::tables::{CellFormat, Cells, TableStyleDefinition, TableStyleFormat};
use crate::text::{CharacterPropertiesSpec, ParagraphPropertiesSpec, ThemeInfo};
use crate::three_d::{Scene3DSpec, Shape3DSpec};

/// An open PowerPoint deck.
///
/// A deck comes from exactly two places — `Deck.blank` authors one from nothing, and `Deck.open`
/// reads one from bytes — so `mjx_ooxml.Deck()` raises rather than handing back
/// something half-built.
#[pyclass(module = "mjx_ooxml")]
#[derive(Debug)]
pub struct Deck {
    inner: ooxml::Deck,
}

#[pymethods]
impl Deck {
    /// A new deck with nothing in it: one slide master, one blank layout, a theme, and no slides.
    ///
    /// Nothing is read from disk and no template is embedded — every part is authored from this
    /// library's own element builders, which is what makes a deck buildable from a `pip install`
    /// with no input file.
    ///
    /// Raises `InvalidArgumentError` if the size is outside the 914 400–51 206 400 EMU range
    /// `p:sldSz` can express (1 to 56 inches on each axis).
    #[staticmethod]
    fn blank(size: SlideSize) -> PyResult<Self> {
        ooxml::Deck::blank(size.0)
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Opens a deck from the bytes of a `.pptx`, `.pptm`, `.potx`, `.potm`, `.ppsx` or `.ppsm`.
    ///
    /// The interpreter lock is released for the parse, so several threads can open several decks at
    /// once. Raises `IoError` for bytes that are not a readable container,
    /// `MalformedDocumentError` for a package whose markup is not PresentationML, and
    /// `UnsupportedFormatError` — naming the format — for a Word or Excel document.
    #[staticmethod]
    fn open(python: Python<'_>, data: &[u8]) -> PyResult<Self> {
        python
            .detach(|| ooxml::Deck::open(data))
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// What this deck's main part says it is — `Format.Presentation`, `Format.PresentationTemplate`
    /// and so on. A deck authored by `blank` reports `Format.Presentation`.
    fn format(&self) -> PyResult<Format> {
        Format::from_model(self.inner.format())
    }

    /// The deck as the bytes of a `.pptx`, **validated first**.
    ///
    /// Every part that was never touched is re-emitted verbatim; only the parts an edit
    /// materialised are serialised from the model. The interpreter lock is released for the write.
    ///
    /// Raises `InvalidDocumentError` rather than emitting a file PowerPoint would offer to repair.
    /// [`save_unchecked`](Deck::save_unchecked) is the deliberate override.
    fn save<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        python
            .detach(|| self.inner.save())
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    /// The deck as bytes, **without** the validation pass.
    ///
    /// For the one case that needs it: writing a deck whose defect you already know about and
    /// intend to inspect. Anything this writes and `save` refuses is a file PowerPoint may decline
    /// to open.
    fn save_unchecked<'py>(&self, python: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        python
            .detach(|| self.inner.save_unchecked())
            .map(|bytes| PyBytes::new(python, &bytes))
            .map_err(to_py_err)
    }

    /// Runs the packaging and PresentationML checks `save` runs, without writing anything.
    ///
    /// Raises `InvalidDocumentError` describing the first defect found.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(to_py_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "<mjx_ooxml.Deck {} slide(s), {} layout(s), {} master(s)>",
            self.inner.slide_count(),
            self.inner.layout_count(),
            self.inner.master_count()
        )
    }

    // --- the delegated surface ------------------------------------------------------------------
    //
    // 251 methods, one per `mjx_ooxml::Deck` method that can cross a foreign function boundary.
    // Each takes its arguments in the classes this module defines, calls exactly one method on the
    // deck, and drops its borrow before returning. The docstrings are `mjx-ooxml`'s own summaries,
    // so the Python help and the Rust documentation cannot drift apart.
    //
    // Three of the facade's methods are absent, and all three return a `Presentation`:
    // `presentation`, `presentation_mut` and `into_presentation`. They are the Rust-only escape
    // hatch to the `ShapeCursor` and the closure-taking readers — the things a binding cannot carry
    // — so exposing them here would hand Python an object with no methods on it.

    /// The explicit fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`, or
    /// `None` if the shape declares no fill in its `p:spPr` (its fill is then inherited from the
    /// placeholder / style / theme — resolving that is a separate, future task). Reading does not
    /// dirty the part.
    fn shape_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<FillSpec>> {
        self.inner
            .shape_fill(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(FillSpec))
    }

    /// Sets the fill of shape `shape_idx` on `surface` from an interner-free `FillSpec`, rebuilding
    /// the `p:spPr` fill element (replacing an existing one in place, or inserting a new one after
    /// any geometry and before `a:ln`). Marks only that part dirty.
    fn set_shape_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        fill: &FillSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_fill(surface.0, shape_idx.0, &fill.0)
            .map_err(to_py_err)
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no fill" (`a:noFill`). A shorthand for
    /// `set_shape_fill` with `FillSpec::None`.
    fn set_shape_no_fill(&mut self, surface: SurfaceArg, shape_idx: ShapePathArg) -> PyResult<()> {
        self.inner
            .set_shape_no_fill(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The **explicit** outline of shape `shape_idx` on `surface` — its `p:spPr > a:ln` as an
    /// interner- free `LineSpec` — or `None` when the shape declares no `a:ln` (its outline is then
    /// inherited; effective outline resolution is a later step). Reading does not dirty the part.
    fn shape_outline(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<LineSpec>> {
        self.inner
            .shape_outline(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(LineSpec))
    }

    /// Sets the outline of shape `shape_idx` on `surface` from an interner-free `LineSpec`,
    /// rebuilding the `p:spPr` `a:ln` element (replacing an existing one in place, or inserting a
    /// new one after any geometry and fill, before effects). Marks only that part dirty.
    fn set_shape_outline(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        line: &LineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_outline(surface.0, shape_idx.0, &line.0)
            .map_err(to_py_err)
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no outline"
    /// (`<a:ln><a:noFill/></a:ln>`). A shorthand for `set_shape_outline` with a `LineSpec` whose
    /// fill is `FillSpec::None` — PowerPoint's "no line", distinct from an absent `a:ln`.
    fn set_shape_no_outline(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<()> {
        self.inner
            .set_shape_no_outline(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The **explicit** effects of shape `shape_idx` on `surface` — its `p:spPr > a:effectLst` as
    /// an interner-free `EffectListSpec` — or `None` when the shape declares no `a:effectLst` (its
    /// effects are then inherited; effective effect resolution is a later step). A shape whose
    /// effects use the rarer `a:effectDag` alternative also reads as `None` (that opaque graph is
    /// not modeled). Reading does not dirty the part.
    fn shape_effects(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<EffectListSpec>> {
        self.inner
            .shape_effects(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(EffectListSpec))
    }

    /// Sets the effects of shape `shape_idx` on `surface` from an interner-free `EffectListSpec`,
    /// rebuilding the `p:spPr` `a:effectLst` element (replacing an existing effect container in
    /// place — either an `a:effectLst` or the mutually-exclusive `a:effectDag`, which is
    /// overwritten — or inserting a new one after any geometry, fill, and outline, before the 3-D
    /// and extension children). Marks only that part dirty.
    fn set_shape_effects(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        effects: &EffectListSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_effects(surface.0, shape_idx.0, &effects.0)
            .map_err(to_py_err)
    }

    /// Sets shape `shape_idx` on `surface` to explicit "no effects" (an empty `<a:effectLst/>`). A
    /// shorthand for `set_shape_effects` with an empty `EffectListSpec` — the explicitly-cleared
    /// effect state that overrides inheritance, distinct from an absent `a:effectLst`. Reads back
    /// as `Some(EffectListSpec::default())`.
    fn set_shape_no_effects(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<()> {
        self.inner
            .set_shape_no_effects(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The **explicit** 3-D scene of shape `shape_idx` on `surface` — its `p:spPr > a:scene3d`
    /// (`CT_Scene3D`) as an interner-free `Scene3DSpec` — or `None` when the shape declares no
    /// `a:scene3d`. 3-D has no inheritance chain, so an absent scene means the shape is flat, not
    /// that it inherits one. A scene present but missing a schema-required part (its `a:camera` or
    /// `a:lightRig`) also reads as `None`. Reading does not dirty the part.
    fn shape_scene_3d(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Scene3DSpec>> {
        self.inner
            .shape_scene_3d(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(Scene3DSpec))
    }

    /// Sets the 3-D scene of shape `shape_idx` on `surface` from an interner-free `Scene3DSpec`,
    /// rebuilding the `p:spPr` `a:scene3d` (replacing an existing one in place, or inserting a new
    /// one after any geometry, fill, outline, and effects, before `a:sp3d`). Rebuilding from a spec
    /// drops any opaque scene internals (`a:backdrop`, `extLst`). Marks only that part dirty.
    fn set_shape_scene_3d(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        scene: &Scene3DSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_scene_3d(surface.0, shape_idx.0, &scene.0)
            .map_err(to_py_err)
    }

    /// Clears the 3-D scene of shape `shape_idx` on `surface` by **removing** its `a:scene3d`
    /// entirely — a shape without a scene is flat. Unlike effects, there is no "explicitly empty"
    /// scene: `CT_Scene3D` requires a camera and light rig, and 3-D does not inherit, so clearing
    /// removes rather than empties. A no-op (still `Ok`) when the shape has no scene. Marks the
    /// part dirty only if it removed something.
    fn clear_shape_scene_3d(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<()> {
        self.inner
            .clear_shape_scene_3d(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The **explicit** 3-D properties of shape `shape_idx` on `surface` — its `p:spPr > a:sp3d`
    /// (`CT_Shape3D`: extrusion, contour, bevels, material) as an interner-free `Shape3DSpec` — or
    /// `None` when the shape declares no `a:sp3d`. Reading does not dirty the part.
    fn shape_3d_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Shape3DSpec>> {
        self.inner
            .shape_3d_properties(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(Shape3DSpec))
    }

    /// Sets the 3-D properties of shape `shape_idx` on `surface` from an interner-free
    /// `Shape3DSpec`, rebuilding the `p:spPr` `a:sp3d` (replacing an existing one in place, or
    /// inserting a new one after every other visual property, before any `a:extLst`). Rebuilding
    /// from a spec drops any opaque `extLst`. Marks only that part dirty.
    fn set_shape_3d_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        properties: &Shape3DSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_3d_properties(surface.0, shape_idx.0, &properties.0)
            .map_err(to_py_err)
    }

    /// Clears the 3-D properties of shape `shape_idx` on `surface` by **removing** its `a:sp3d`
    /// entirely. A no-op (still `Ok`) when the shape has none. Marks the part dirty only if it
    /// removed something.
    fn clear_shape_3d_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<()> {
        self.inner
            .clear_shape_3d_properties(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The position and size of shape `shape_idx` on `surface` **on the slide** — absolute within
    /// `slide_size`, whether the shape is top-level or nested inside groups.
    fn shape_bounds(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<ShapeBounds>> {
        self.inner
            .shape_bounds(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(ShapeBounds))
    }

    /// Moves and resizes shape `shape_idx` on `surface` to `bounds`, given **on the slide** — the
    /// same absolute space `shape_bounds` answers in. Creates the shape's transform element if it
    /// had none, and marks only that part dirty.
    fn set_shape_bounds(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        bounds: ShapeBounds,
    ) -> PyResult<()> {
        self.inner
            .set_shape_bounds(surface.0, shape_idx.0, bounds.0)
            .map_err(to_py_err)
    }

    /// The **explicit** transform of shape `shape_idx` on `surface` — its position, size, rotation
    /// and mirror flags, plus the child coordinate space if it is a group — or `None` when the
    /// shape declares no transform at all.
    fn shape_transform(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Transform2D>> {
        self.inner
            .shape_transform(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(Transform2D))
    }

    /// Applies `transform` to shape `shape_idx` on `surface`, creating its transform element if it
    /// had none. Marks only that part dirty; everything else re-emits verbatim.
    fn set_shape_transform(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        transform: &Transform2D,
    ) -> PyResult<()> {
        self.inner
            .set_shape_transform(surface.0, shape_idx.0, &transform.0)
            .map_err(to_py_err)
    }

    /// The geometry of shape `shape_idx` on `surface`, as a `Geometry` — a preset shape
    /// (`Geometry::Preset`), a custom path list (`Geometry::Custom`), or `Geometry::Inherited` when
    /// the shape states no geometry of its own (it takes one from its placeholder / layout).
    /// Reading does not dirty the part.
    fn shape_geometry(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Geometry> {
        self.inner
            .shape_geometry(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(Geometry)
    }

    /// Every adjustment of shape `shape_idx`'s **preset** geometry, resolved against a concrete
    /// shape size: each value *and* the numeric domain it may move in.
    fn shape_adjustments(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        size: GuideContext,
    ) -> PyResult<Vec<BoundedAdjustment>> {
        self.inner
            .shape_adjustments(surface.0, shape_idx.0, size.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(BoundedAdjustment).collect())
    }

    /// Sets the geometry of shape `shape_idx` on `surface` from a `Geometry`: a preset shape
    /// (`Geometry::Preset`) rewrites the `a:prstGeom`, a custom path list (`Geometry::Custom`)
    /// writes an `a:custGeom`, and `Geometry::Inherited` removes the shape's own geometry so an
    /// inherited one takes over. The two kinds are mutually exclusive, so setting one drops the
    /// other. Marks only that slide part dirty; everything else re-emits verbatim.
    fn set_shape_geometry(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        geometry: Geometry,
    ) -> PyResult<()> {
        self.inner
            .set_shape_geometry(surface.0, shape_idx.0, geometry.0)
            .map_err(to_py_err)
    }

    /// The text of the cell at `(row, column)` — its paragraphs joined by newlines.
    fn cell_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<String> {
        self.inner
            .cell_text(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// The text that actually **renders** at `(row, column)` — the text of the cell if it stands
    /// alone, or of the merge **anchor** covering it if it is merged away.
    fn visible_cell_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<String> {
        self.inner
            .visible_cell_text(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the cell's paragraphs) of the cell
    /// at `(row, column)`. Marks only that part dirty.
    fn set_cell_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        run_idx: u32,
        text: &str,
    ) -> PyResult<()> {
        self.inner
            .set_cell_text(surface.0, shape_idx.0, row, column, run_idx, text)
            .map_err(to_py_err)
    }

    /// The number of paragraphs in the cell at `(row, column)`.
    fn cell_paragraph_count(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<u32> {
        self.inner
            .cell_paragraph_count(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// The number of runs in one paragraph of the cell at `(row, column)`.
    fn cell_run_count(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .cell_run_count(surface.0, shape_idx.0, row, column, para_idx)
            .map_err(to_py_err)
    }

    /// The text of one paragraph of the cell at `(row, column)`.
    fn cell_paragraph_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> PyResult<String> {
        self.inner
            .cell_paragraph_text(surface.0, shape_idx.0, row, column, para_idx)
            .map_err(to_py_err)
    }

    /// The text of one run of the cell at `(row, column)`.
    fn cell_run_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<String> {
        self.inner
            .cell_run_text(surface.0, shape_idx.0, row, column, para_idx, run_idx)
            .map_err(to_py_err)
    }

    /// The layout properties a paragraph of the cell at `(row, column)` declares of its own.
    fn cell_paragraph_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> PyResult<Option<ParagraphPropertiesSpec>> {
        self.inner
            .cell_paragraph_properties(surface.0, shape_idx.0, row, column, para_idx)
            .map_err(to_py_err)
            .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// The character properties a run of the cell at `(row, column)` declares of its own.
    fn cell_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<Option<CharacterPropertiesSpec>> {
        self.inner
            .cell_run_properties(surface.0, shape_idx.0, row, column, para_idx, run_idx)
            .map_err(to_py_err)
            .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// The paragraph-mark properties (`a:endParaRPr`) of a paragraph of the cell at `(row, column)`
    /// — the format an empty cell holds, and what text typed into it would take on.
    fn cell_end_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> PyResult<Option<CharacterPropertiesSpec>> {
        self.inner
            .cell_end_run_properties(surface.0, shape_idx.0, row, column, para_idx)
            .map_err(to_py_err)
            .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// Applies `spec` to one run of one paragraph of the cell at `(row, column)`.
    // The coordinates the delegated method takes, restated one for one — the same
    // `expect` `mjx-ooxml` carries on the method this calls.
    #[expect(
        clippy::too_many_arguments,
        reason = "the coordinates the delegated method takes, restated one for one"
    )]
    fn set_cell_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_run_properties(
                surface.0,
                shape_idx.0,
                row,
                column,
                para_idx,
                run_idx,
                &spec.0,
            )
            .map_err(to_py_err)
    }

    /// Applies `spec` to **every run** of one paragraph of the cell at `(row, column)`, and to its
    /// paragraph mark.
    fn set_cell_paragraph_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_paragraph_run_properties(
                surface.0,
                shape_idx.0,
                row,
                column,
                para_idx,
                &spec.0,
            )
            .map_err(to_py_err)
    }

    /// Applies `spec` to **every run of every paragraph** of the cell at `(row, column)` — what
    /// selecting a whole cell and restyling it means, and the usual way to make a header bold.
    fn set_cell_run_properties_all(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_run_properties_all(surface.0, shape_idx.0, row, column, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to a paragraph mark (`a:endParaRPr`) of the cell at `(row, column)`, creating
    /// the element if the paragraph has none — how an **empty** cell is formatted.
    fn set_cell_end_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_end_run_properties(surface.0, shape_idx.0, row, column, para_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`) in the cell at `(row, column)`,
    /// creating the element if it has none. The properties **merge**, as run properties do.
    fn set_cell_paragraph_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &ParagraphPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_paragraph_properties(surface.0, shape_idx.0, row, column, para_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to part of a paragraph of the cell at `(row, column)` — the characters in
    /// `range`, counted in Unicode scalars. Splits runs at the range's edges, exactly as the shape-
    /// addressed form does.
    // The coordinates the delegated method takes, restated one for one — the same
    // `expect` `mjx-ooxml` carries on the method this calls.
    #[expect(
        clippy::too_many_arguments,
        reason = "the coordinates the delegated method takes, restated one for one"
    )]
    fn set_cell_text_range_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        range: RangeArg,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_text_range_properties(
                surface.0,
                shape_idx.0,
                row,
                column,
                para_idx,
                range.0,
                &spec.0,
            )
            .map_err(to_py_err)
    }

    /// The fill the cell at `(row, column)` declares, or `None` when it declares none — in which
    /// case the table style decides. Reading does not dirty the part.
    fn cell_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<Option<FillSpec>> {
        self.inner
            .cell_fill(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
            .map(|value| value.map(FillSpec))
    }

    /// Fills the cell at `(row, column)`. Marks only that part dirty.
    fn set_cell_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        fill: &FillSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_fill(surface.0, shape_idx.0, row, column, &fill.0)
            .map_err(to_py_err)
    }

    /// Removes the cell's own fill, so the table style decides how it is filled again.
    fn clear_cell_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<()> {
        self.inner
            .clear_cell_fill(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// The border the cell at `(row, column)` declares on `edge`, or `None` if it declares none
    /// there. Reading does not dirty the part.
    fn cell_border(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> PyResult<Option<LineSpec>> {
        self.inner
            .cell_border(surface.0, shape_idx.0, row, column, edge.into())
            .map_err(to_py_err)
            .map(|value| value.map(LineSpec))
    }

    /// Draws a border on one edge of the cell at `(row, column)`. Marks only that part dirty.
    fn set_cell_border(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
        line: &LineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_cell_border(surface.0, shape_idx.0, row, column, edge.into(), &line.0)
            .map_err(to_py_err)
    }

    /// The ids of the header cells that describe the cell at `(row, column)` (`a:tcPr >
    /// a:headers`), in order — the accessibility association a screen reader announces. Empty when
    /// the cell names none. Reading does not dirty the part.
    fn cell_headers(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<Vec<String>> {
        self.inner
            .cell_headers(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// Sets the header-cell ids that describe the cell at `(row, column)`, replacing whatever it
    /// had; an empty slice removes the association. Marks only that part dirty.
    fn set_cell_headers(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        header_ids: Vec<String>,
    ) -> PyResult<()> {
        self.inner
            .set_cell_headers(
                surface.0,
                shape_idx.0,
                row,
                column,
                &as_str_slice(&header_ids),
            )
            .map_err(to_py_err)
    }

    /// Removes the border on one edge of the cell at `(row, column)`.
    fn clear_cell_border(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> PyResult<()> {
        self.inner
            .clear_cell_border(surface.0, shape_idx.0, row, column, edge.into())
            .map_err(to_py_err)
    }

    /// The four insets between the cell's edges and its text, each `None` when the cell does not
    /// state it. Reading does not dirty the part.
    fn cell_margins(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<CellMargins> {
        self.inner
            .cell_margins(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
            .map(CellMargins)
    }

    /// Sets the cell's insets. Each field left `None` is **not written**, so a caller can set one
    /// margin without stating the other three.
    fn set_cell_margins(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        margins: CellMargins,
    ) -> PyResult<()> {
        self.inner
            .set_cell_margins(surface.0, shape_idx.0, row, column, margins.0)
            .map_err(to_py_err)
    }

    /// Where the text sits vertically in the cell at `(row, column)`, or `None` if unstated (the
    /// wire default is `TextAnchoring::Top`). Reading does not dirty the part.
    fn cell_anchor(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<Option<TextAnchoring>> {
        self.inner
            .cell_anchor(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)?
            .map(TextAnchoring::from_model)
            .transpose()
    }

    /// Sets where the text sits vertically in the cell at `(row, column)`.
    fn set_cell_anchor(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        anchor: TextAnchoring,
    ) -> PyResult<()> {
        self.inner
            .set_cell_anchor(surface.0, shape_idx.0, row, column, anchor.into())
            .map_err(to_py_err)
    }

    /// Which way the text flows in the cell at `(row, column)`, or `None` if unstated (the wire
    /// default is `TextDirection::Horizontal`). Reading does not dirty the part.
    fn cell_text_direction(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<Option<TextDirection>> {
        self.inner
            .cell_text_direction(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)?
            .map(TextDirection::from_model)
            .transpose()
    }

    /// Sets which way the text flows in the cell at `(row, column)` — how a rotated header row is
    /// made.
    fn set_cell_text_direction(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        direction: TextDirection,
    ) -> PyResult<()> {
        self.inner
            .set_cell_text_direction(surface.0, shape_idx.0, row, column, direction.into())
            .map_err(to_py_err)
    }

    /// Applies `format` to every cell in `cells`. Marks only that part dirty.
    fn format_cells(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        cells: Cells,
        format: &CellFormat,
    ) -> PyResult<()> {
        self.inner
            .format_cells(surface.0, shape_idx.0, cells.0, &format.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to **every run of every paragraph** in each cell of `cells`, and to each
    /// paragraph's mark — bolding a header row in one call.
    fn format_cell_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        cells: Cells,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .format_cell_text(surface.0, shape_idx.0, cells.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to the layout properties of **every paragraph** in each cell of `cells` —
    /// right- aligning a column of numbers in one call.
    fn format_cell_paragraphs(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        cells: Cells,
        spec: &ParagraphPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .format_cell_paragraphs(surface.0, shape_idx.0, cells.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Merges `cells` into one region. Marks only that part dirty.
    fn merge_cells(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        cells: Cells,
    ) -> PyResult<()> {
        self.inner
            .merge_cells(surface.0, shape_idx.0, cells.0)
            .map_err(to_py_err)
    }

    /// Undoes the merge covering the cell at `(row, column)`, whichever cell of the region is
    /// named. Marks only that part dirty.
    fn unmerge_cells(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<()> {
        self.inner
            .unmerge_cells(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// The fill of series `series_idx` of the chart the frame `shape_idx` on `surface` references —
    /// what colour it is drawn in — or `None` when the series declares none and takes its colour
    /// from the chart style. Reading does not dirty the part.
    fn chart_series_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<Option<FillSpec>> {
        self.inner
            .chart_series_fill(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
            .map(|value| value.map(FillSpec))
    }

    /// Sets the fill of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references, creating its `c:spPr` if it had none. Marks only the chart part dirty.
    fn set_chart_series_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        fill: &FillSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_fill(surface.0, shape_idx.0, series_idx, &fill.0)
            .map_err(to_py_err)
    }

    /// Sets the outline of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references — the line a line or radar plot draws, or the border of a bar or area. Marks only
    /// the chart part dirty.
    fn set_chart_series_line(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        line: &LineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_line(surface.0, shape_idx.0, series_idx, &line.0)
            .map_err(to_py_err)
    }

    /// The data-label settings **in force** for one point of series `series_idx` of the chart the
    /// frame `shape_idx` on `surface` references — the point's `c:dLbl` merged over the series'
    /// `c:dLbls` merged over the owning plot's.
    #[pyo3(signature = (surface, shape_idx, series_idx, point_idx = None))]
    fn chart_data_labels(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        point_idx: Option<u32>,
    ) -> PyResult<DataLabelSettings> {
        self.inner
            .chart_data_labels(surface.0, shape_idx.0, series_idx, point_idx)
            .map_err(to_py_err)
            .map(DataLabelSettings)
    }

    /// The data-label settings one **tier** states in its own right — what that tier contributes to
    /// the merge, with everything it leaves unset reported as `None`.
    fn chart_data_label_tier(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        scope: ChartLabelScope,
    ) -> PyResult<Option<DataLabelSettings>> {
        self.inner
            .chart_data_label_tier(surface.0, shape_idx.0, scope.0)
            .map_err(to_py_err)
            .map(|value| value.map(DataLabelSettings))
    }

    /// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None` when it
    /// states none and shows what the settings say. Reading does not dirty the part.
    fn chart_point_label_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        point_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .chart_point_label_text(surface.0, shape_idx.0, series_idx, point_idx)
            .map_err(to_py_err)
    }

    /// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had
    /// none and leaving every setting `spec` does not state alone. Marks only the chart part dirty.
    fn set_chart_data_labels(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_data_labels(surface.0, shape_idx.0, scope.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which is
    /// how one series of a labelled plot, or one point of a labelled series, is silenced without
    /// disturbing the rest. Marks only the chart part dirty.
    fn delete_chart_data_labels(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        scope: ChartLabelScope,
    ) -> PyResult<()> {
        self.inner
            .delete_chart_data_labels(surface.0, shape_idx.0, scope.0)
            .map_err(to_py_err)
    }

    /// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
    /// again. Answers whether an element was there. Marks only the chart part dirty.
    fn remove_chart_data_labels(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        scope: ChartLabelScope,
    ) -> PyResult<bool> {
        self.inner
            .remove_chart_data_labels(surface.0, shape_idx.0, scope.0)
            .map_err(to_py_err)
    }

    /// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document
    /// order. Reading does not dirty the part.
    fn chart_point_formats(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<Vec<ChartPointFormatData>> {
        self.inner
            .chart_point_formats(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ChartPointFormatData).collect())
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series,
    /// creating its `c:dPt` at the schema rank if it had none. Marks only the chart part dirty.
    fn set_chart_point_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        point_idx: u32,
        fill: &FillSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_point_fill(surface.0, shape_idx.0, series_idx, point_idx, &fill.0)
            .map_err(to_py_err)
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
    /// Marks only the chart part dirty.
    fn set_chart_point_line(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        point_idx: u32,
        line: &LineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_point_line(surface.0, shape_idx.0, series_idx, point_idx, &line.0)
            .map_err(to_py_err)
    }

    /// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
    /// `percent` of the radius (`c:explosion`), or (for `None`) puts it back. Marks only the chart
    /// part dirty.
    #[pyo3(signature = (surface, shape_idx, series_idx, point_idx, percent = None))]
    fn set_chart_point_explosion(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        point_idx: u32,
        percent: Option<u32>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_point_explosion(surface.0, shape_idx.0, series_idx, point_idx, percent)
            .map_err(to_py_err)
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like the
    /// rest of its series. Answers whether any was there. Marks only the chart part dirty.
    fn remove_chart_point_format(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        point_idx: u32,
    ) -> PyResult<bool> {
        self.inner
            .remove_chart_point_format(surface.0, shape_idx.0, series_idx, point_idx)
            .map_err(to_py_err)
    }

    /// Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
    /// Reading does not dirty the part.
    fn chart_trendlines(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<Vec<ChartTrendlineData>> {
        self.inner
            .chart_trendlines(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ChartTrendlineData).collect())
    }

    /// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends** — a
    /// series may carry a linear fit and a moving average at once. Marks only the chart part dirty.
    fn add_chart_trendline(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        spec: &TrendlineSpec,
    ) -> PyResult<()> {
        self.inner
            .add_chart_trendline(surface.0, shape_idx.0, series_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** — the
    /// curve keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional setting
    /// `spec` leaves unset is cleared. Marks only the chart part dirty.
    fn set_chart_trendline(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        trendline_idx: u32,
        spec: &TrendlineSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_trendline(surface.0, shape_idx.0, series_idx, trendline_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Removes every trendline from series `series_idx`, answering how many went. Marks only the
    /// chart part dirty.
    fn remove_chart_trendlines(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .remove_chart_trendlines(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
    }

    /// Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or line
    /// series, up to two (x and y) for scatter, area and bubble. Reading does not dirty the part.
    fn chart_error_bars(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<Vec<ChartErrorBarData>> {
        self.inner
            .chart_error_bars(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ChartErrorBarData).collect())
    }

    /// Gives series `series_idx` error bars, replacing an existing set that runs along the same
    /// axis. Marks only the chart part dirty.
    fn set_chart_error_bars(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        spec: &ErrorBarSpec,
    ) -> PyResult<()> {
        self.inner
            .set_chart_error_bars(surface.0, shape_idx.0, series_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went. Marks
    /// only the chart part dirty.
    fn remove_chart_error_bars(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .remove_chart_error_bars(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no
    /// longer has. Reading does not dirty the part.
    fn chart_dangling_decoration(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<Vec<DanglingPointReference>> {
        self.inner
            .chart_dangling_decoration(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(DanglingPointReference).collect())
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the end of
    /// its data, answering how many went. Marks only the chart part dirty.
    fn drop_chart_dangling_decoration(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .drop_chart_dangling_decoration(surface.0, shape_idx.0, series_idx)
            .map_err(to_py_err)
    }

    /// Adds `chart` to `surface` as a new chart, laid out inside `bounds`, and returns its index in
    /// the shape tree.
    fn add_chart(
        &mut self,
        surface: SurfaceArg,
        chart: &ChartData,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_chart(surface.0, &chart.0, bounds.0)
            .map_err(to_py_err)
    }

    /// The raw XML bytes of the chart part the chart frame `shape_idx` on `surface` references
    /// (`/ppt/charts/chartN.xml`), exactly as the package holds them, or `None` when the shape
    /// frames no chart. Borrowed from the package, so the part is not copied.
    fn chart_part_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .chart_part_bytes(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// Every chart on `surface` that references a backing workbook (`c:externalData`), with where
    /// each is referenced from and whether that reference is external.
    fn chart_workbooks(&mut self, surface: SurfaceArg) -> PyResult<Vec<ChartWorkbook>> {
        self.inner
            .chart_workbooks(surface.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ChartWorkbook).collect())
    }

    /// Detaches the backing workbook from the chart `shape_idx` on `surface`: removes its
    /// `c:externalData` reference — the element and its relationship — leaving the chart to render
    /// from its cached values. This neutralizes a chart that links an unreachable external workbook
    /// (the caller decides accessibility; use `chart_workbooks` to find the candidates), and yields
    /// exactly the cache-only shape a freshly authored chart has.
    fn detach_chart_workbook(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<()> {
        self.inner
            .detach_chart_workbook(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The series of the chart the frame `shape_idx` on `surface` references — for each, its name,
    /// category labels and values (for a scatter series, its X labels and Y values), flattened
    /// across the chart's plots. Reading does not dirty the part.
    fn chart_series(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Vec<ChartSeriesData>> {
        self.inner
            .chart_series(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ChartSeriesData).collect())
    }

    /// Rewrites the values of series `series_idx` (0-based across the chart's plots) of the chart
    /// the frame `shape_idx` on `surface` references — whichever source the series names: a
    /// `c:numRef`'s cache or a `c:numLit`.
    fn set_chart_series_values(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        values: Vec<f64>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_values(surface.0, shape_idx.0, series_idx, &values)
            .map_err(to_py_err)
    }

    /// Rewrites the category labels of series `series_idx` (0-based across the chart's plots) of
    /// the chart the frame `shape_idx` on `surface` references, and refreshes the chart's embedded
    /// workbook alongside it.
    fn set_chart_series_categories(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        series_idx: u32,
        labels: Vec<String>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_series_categories(surface.0, shape_idx.0, series_idx, &as_str_slice(&labels))
            .map_err(to_py_err)
    }

    /// Rewrites the embedded workbook of the chart the frame `shape_idx` on `surface` references so
    /// its cells hold exactly what the chart now draws, and answers whether it rewrote one.
    fn refresh_chart_workbook(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<bool> {
        self.inner
            .refresh_chart_workbook(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The kind of every plot the chart the frame `shape_idx` on `surface` references draws, in
    /// document order — one entry per plot element, so a combo chart yields several. Reading does
    /// not dirty the part.
    fn chart_kinds(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Vec<ChartKind>> {
        self.inner
            .chart_kinds(surface.0, shape_idx.0)
            .map_err(to_py_err)?
            .into_iter()
            .map(ChartKind::from_model)
            .collect()
    }

    /// The axes of the chart the frame `shape_idx` on `surface` references, in document order.
    /// Reading does not dirty the part.
    fn chart_axes(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Vec<ChartAxisData>> {
        self.inner
            .chart_axes(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ChartAxisData).collect())
    }

    /// Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order) of the chart
    /// the frame `shape_idx` on `surface` references. `None` returns that end of the axis to
    /// automatic scaling. Marks only the chart part dirty.
    #[pyo3(signature = (surface, shape_idx, axis_idx, minimum = None, maximum = None))]
    fn set_chart_axis_scale(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        axis_idx: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_scale(surface.0, shape_idx.0, axis_idx, minimum, maximum)
            .map_err(to_py_err)
    }

    /// Sets the direction of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references — smallest value first, or reversed. Marks only the chart part dirty.
    fn set_chart_axis_orientation(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        axis_idx: u32,
        orientation: AxisOrientation,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_orientation(surface.0, shape_idx.0, axis_idx, orientation.into())
            .map_err(to_py_err)
    }

    /// Sets or removes the title of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references. `None` removes the title. Marks only the chart part dirty.
    #[pyo3(signature = (surface, shape_idx, axis_idx, text = None))]
    fn set_chart_axis_title(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        axis_idx: u32,
        text: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_title(surface.0, shape_idx.0, axis_idx, text)
            .map_err(to_py_err)
    }

    /// Turns the gridlines of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references on or off. Marks only the chart part dirty.
    fn set_chart_axis_gridlines(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        axis_idx: u32,
        major: bool,
        minor: bool,
    ) -> PyResult<()> {
        self.inner
            .set_chart_axis_gridlines(surface.0, shape_idx.0, axis_idx, major, minor)
            .map_err(to_py_err)
    }

    /// The heading of the chart the frame `shape_idx` on `surface` references (`c:title`), or
    /// `None` when it has none. Reading does not dirty the part.
    fn chart_title(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<String>> {
        self.inner
            .chart_title(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Sets or removes the heading of the chart the frame `shape_idx` on `surface` references.
    /// `None` removes it. Marks only the chart part dirty.
    #[pyo3(signature = (surface, shape_idx, text = None))]
    fn set_chart_title(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        text: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_title(surface.0, shape_idx.0, text)
            .map_err(to_py_err)
    }

    /// The legend of the chart the frame `shape_idx` on `surface` references, or `None` when it has
    /// none. Reading does not dirty the part.
    fn chart_legend(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<ChartLegendData>> {
        self.inner
            .chart_legend(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(ChartLegendData))
    }

    /// Places the legend of the chart the frame `shape_idx` on `surface` references at `position`,
    /// adding one if the chart had none. `None` removes the legend. Marks only the chart part
    /// dirty.
    #[pyo3(signature = (surface, shape_idx, position = None))]
    fn set_chart_legend(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        position: Option<LegendPosition>,
    ) -> PyResult<()> {
        self.inner
            .set_chart_legend(surface.0, shape_idx.0, position.map(Into::into))
            .map_err(to_py_err)
    }

    /// The built-in style id the chart the frame `shape_idx` on `surface` references names
    /// (`c:style@val`, 1 to 48) — the palette and effect set Office draws an unstyled series with —
    /// or `None` when it names none. Reading does not dirty the part.
    fn chart_style_id(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<u32>> {
        self.inner
            .chart_style_id(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The number of slides, in presentation order.
    fn slide_count(&self) -> u32 {
        self.inner.slide_count()
    }

    /// The number of slide masters, in `p:sldMasterIdLst` order.
    fn master_count(&self) -> u32 {
        self.inner.master_count()
    }

    /// The name of master `idx` (`p:cSld@name`, e.g. `Office Theme`), or `None` if it is unnamed.
    fn master_name(&mut self, idx: u32) -> PyResult<Option<String>> {
        self.inner.master_name(idx).map_err(to_py_err)
    }

    /// Every slide layout the deck offers, in layout-index order — the inventory a caller reads
    /// before choosing one to build a slide on.
    fn layouts(&mut self) -> PyResult<Vec<LayoutInfo>> {
        self.inner
            .layouts()
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(LayoutInfo).collect())
    }

    /// The number of slide layouts across the whole deck, in (master order, `p:sldLayoutIdLst`
    /// order) — so layout indices run master by master. `layout_master` says which master an index
    /// belongs to.
    fn layout_count(&self) -> u32 {
        self.inner.layout_count()
    }

    /// The index of the master that lists layout `idx`.
    fn layout_master(&self, idx: u32) -> Option<u32> {
        self.inner.layout_master(idx)
    }

    /// The name of layout `idx` (`p:cSld@name`, e.g. `Title and Content` — the name PowerPoint
    /// shows in its layout gallery), or `None` if it is unnamed.
    fn layout_name(&mut self, idx: u32) -> PyResult<Option<String>> {
        self.inner.layout_name(idx).map_err(to_py_err)
    }

    /// How layout `idx` arranges its content (`p:sldLayout@type`) — a coarse description of which
    /// placeholders it offers, which an application can use to map between layouts.
    fn layout_kind(&mut self, idx: u32) -> PyResult<SlideLayoutKind> {
        SlideLayoutKind::from_model(self.inner.layout_kind(idx).map_err(to_py_err)?)
    }

    /// The index of the layout slide `slide_idx` is built on, or `None` if the slide relates to no
    /// layout (or to one no master lists).
    fn slide_layout(&self, slide_idx: u32) -> PyResult<Option<u32>> {
        self.inner.slide_layout(slide_idx).map_err(to_py_err)
    }

    /// The size of every slide in the deck (`p:sldSz`) — the extent shape bounds are laid out in.
    fn slide_size(&mut self) -> PyResult<SlideSize> {
        self.inner.slide_size().map_err(to_py_err).map(SlideSize)
    }

    /// The theme that governs `surface`, as an interner-free `ThemeInfo` (its color scheme + fill-
    /// style matrix) — the theme related to the last part of the surface's inheritance chain (slide
    /// → slideLayout → slideMaster → theme, and the shorter walks from a layout or master). Returns
    /// `Ok(None)` if any hop is absent (a deck without a theme). Reading does not dirty any part.
    fn theme(&mut self, surface: SurfaceArg) -> PyResult<Option<ThemeInfo>> {
        self.inner
            .theme(surface.0)
            .map_err(to_py_err)
            .map(|value| value.map(ThemeInfo))
    }

    /// The effective theme `ColorMap` for `surface`: the master's `p:clrMap` (reached along the
    /// surface's inheritance chain), replaced by the surface's own `p:clrMapOvr >
    /// a:overrideClrMapping` when it supplies a full mapping (a `masterClrMapping`, an absent
    /// override, or a schema-loose attribute-less override all inherit the master's map). It maps
    /// the logical color names a shape may reference (`bg1`/`tx1`/…) to the theme's concrete scheme
    /// slots. `Ok(None)` when there is no reachable master or no `p:clrMap`. Reading does not dirty
    /// a part.
    fn color_map(&mut self, surface: SurfaceArg) -> PyResult<Option<ColorMap>> {
        self.inner
            .color_map(surface.0)
            .map_err(to_py_err)
            .map(|value| value.map(ColorMap))
    }

    /// The **effective** fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`
    /// whose colors are resolved to concrete `RRGGBB` values — the fill the shape actually renders.
    /// Three sources are tried, in order: an explicit `p:spPr` fill; a `p:style > a:fillRef` (the
    /// theme fill- style at that index, `phClr` substituted by the reference's color); and, for a
    /// placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the layout
    /// then the master. Scheme colors and color transforms are baked against the surface's theme +
    /// map.
    fn effective_shape_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<FillSpec>> {
        self.inner
            .effective_shape_fill(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(FillSpec))
    }

    /// The **effective** outline of shape `shape_idx` on `surface`, as an interner-free `LineSpec`
    /// whose stroke color is resolved to a concrete `RRGGBB` value — the outline the shape actually
    /// renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`; a `p:style >
    /// a:lnRef` (the theme line-style at that index, `phClr` substituted by the reference's color);
    /// and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the
    /// slide layout then the master. Scheme colors and color transforms are baked against the
    /// slide's theme + map.
    fn effective_shape_outline(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<LineSpec>> {
        self.inner
            .effective_shape_outline(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(LineSpec))
    }

    /// The **effective** effects of shape `shape_idx` on `surface`, as an interner-free
    /// `EffectListSpec` whose colors are resolved to concrete `RRGGBB` values — the effects the
    /// shape actually renders. Three sources are tried, in order: an explicit `p:spPr >
    /// a:effectLst`; a `p:style > a:effectRef` (the theme effect-style at that index, `phClr`
    /// substituted by the reference's color); and, for a placeholder shape (`p:ph`),
    /// **inheritance** from the same-slot placeholder on the slide layout then the master. Scheme
    /// colors and color transforms are baked against the slide's theme + map.
    fn effective_shape_effects(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<EffectListSpec>> {
        self.inner
            .effective_shape_effects(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(EffectListSpec))
    }

    /// The **effective** transform of shape `shape_idx` on `surface` — where the shape actually
    /// renders, not what it declares. For a placeholder that places itself nowhere, this is the
    /// same- slot placeholder's transform on the slide layout, and failing that on the master.
    fn effective_shape_transform(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Transform2D>> {
        self.inner
            .effective_shape_transform(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(Transform2D))
    }

    /// The **effective** position and size of shape `shape_idx` on `surface` — where the shape
    /// actually renders, with the layout and master consulted for a placeholder that declares no
    /// bounds of its own.
    fn effective_shape_bounds(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<ShapeBounds>> {
        self.inner
            .effective_shape_bounds(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(ShapeBounds))
    }

    /// The **effective** character properties of run `run_idx` — what the run actually renders as,
    /// with every tier of inheritance resolved and its colors baked to concrete `RRGGBB`.
    fn effective_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<CharacterPropertiesSpec> {
        self.inner
            .effective_run_properties(surface.0, shape_idx.0, para_idx, run_idx)
            .map_err(to_py_err)
            .map(CharacterPropertiesSpec)
    }

    /// The **effective** paragraph properties of paragraph `para_idx` — the layout it actually
    /// renders with, every tier of inheritance resolved.
    fn effective_paragraph_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<ParagraphPropertiesSpec> {
        self.inner
            .effective_paragraph_properties(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
            .map(ParagraphPropertiesSpec)
    }

    /// The **effective** fill of the cell at `(row, column)` of the table shape `shape_idx` frames
    /// — an interner-free `FillSpec` with its colour baked to concrete `RRGGBB`, or `None` if
    /// nothing fills the cell. The cell's own `a:tcPr` fill wins; else the first applicable style
    /// part with a fill (explicit or a theme `fillRef`).
    fn effective_cell_fill(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<Option<FillSpec>> {
        self.inner
            .effective_cell_fill(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
            .map(|value| value.map(FillSpec))
    }

    /// The **effective** border on one `edge` of the cell at `(row, column)` — an interner-free
    /// `LineSpec` with its stroke colour baked, or `None`. The cell's own `a:tcPr` edge wins; else
    /// the applicable style parts' `a:tcBdr`, taking the outer edge (`top`/`left`/…) for a cell on
    /// the table's rim and the interior edge (`insideH`/`insideV`) for one within it.
    fn effective_cell_border(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> PyResult<Option<LineSpec>> {
        self.inner
            .effective_cell_border(surface.0, shape_idx.0, row, column, edge.into())
            .map_err(to_py_err)
            .map(|value| value.map(LineSpec))
    }

    /// The **effective** run properties of a cell's text run — the `CharacterPropertiesSpec` it
    /// actually renders with, colours baked. A shorter ladder than a shape's (a cell inherits from
    /// its table style, not a placeholder chain), highest first: the run's own `a:rPr`, the
    /// paragraph's `a:defRPr`, the table style's `a:tcTxStyle` for each applicable part (bold /
    /// italic / colour), then the presentation's `p:defaultTextStyle`.
    fn effective_cell_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<CharacterPropertiesSpec> {
        self.inner
            .effective_cell_run_properties(surface.0, shape_idx.0, row, column, para_idx, run_idx)
            .map_err(to_py_err)
            .map(CharacterPropertiesSpec)
    }

    /// Removes every part the package no longer reaches from its root, and reports what was swept.
    fn remove_unused_parts(&mut self) -> PyResult<Vec<String>> {
        self.inner.remove_unused_parts().map_err(to_py_err)
    }

    /// Every relationship in the package whose target lies **outside** it — a linked image, a
    /// chart's external workbook, a linked OLE object or media file — with the part that owns each.
    fn external_links(&self) -> Vec<ExternalLink> {
        self.inner
            .external_links()
            .into_iter()
            .map(ExternalLink)
            .collect()
    }

    /// Repoints the relationship `id` of `source` (`None` = the package root) at `new_target`,
    /// keeping its id and its place in the `.rels`. Returns whether one was found.
    #[pyo3(signature = (source, id, new_target, mode))]
    fn retarget_external_link(
        &mut self,
        source: Option<&str>,
        id: &str,
        new_target: &str,
        mode: TargetMode,
    ) -> PyResult<bool> {
        self.inner
            .retarget_external_link(source, id, new_target, mode.into())
            .map_err(to_py_err)
    }

    /// The click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` on
    /// `surface`, resolved to a `Hyperlink` (a URL or a slide index), or `None` if the run has no
    /// hyperlink — or one this build does not model (a mouse-over action, a show jump). Reading
    /// does not dirty the part.
    fn run_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<Option<Hyperlink>> {
        self.inner
            .run_hyperlink(surface.0, shape_idx.0, para_idx, run_idx)
            .map_err(to_py_err)
            .map(|value| value.map(Hyperlink))
    }

    /// Sets the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` to
    /// `link`, adding its relationship. If the run already linked somewhere, that relationship is
    /// removed once nothing else in the part still names it.
    fn set_run_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
        link: &Hyperlink,
    ) -> PyResult<()> {
        self.inner
            .set_run_hyperlink(surface.0, shape_idx.0, para_idx, run_idx, &link.0)
            .map_err(to_py_err)
    }

    /// Removes the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx`,
    /// and the relationship it named once nothing else in the part still references it. A no-op if
    /// the run has no hyperlink.
    fn clear_run_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<()> {
        self.inner
            .clear_run_hyperlink(surface.0, shape_idx.0, para_idx, run_idx)
            .map_err(to_py_err)
    }

    /// Sets the click hyperlink over a **scalar range** of paragraph `para_idx` in shape
    /// `shape_idx`, splitting runs at the boundaries so exactly the selected text is linked (as
    /// `set_text_range_properties` does). One relationship is added and shared by every run in the
    /// range. An empty range links nothing.
    fn set_text_range_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        range: RangeArg,
        link: &Hyperlink,
    ) -> PyResult<()> {
        self.inner
            .set_text_range_hyperlink(surface.0, shape_idx.0, para_idx, range.0, &link.0)
            .map_err(to_py_err)
    }

    /// The click hyperlink on shape `shape_idx` itself (`p:cNvPr > a:hlinkClick`), resolved to a
    /// `Hyperlink`, or `None` if the shape has no hyperlink (or one this build does not model).
    /// Reading does not dirty the part.
    fn shape_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Hyperlink>> {
        self.inner
            .shape_hyperlink(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(Hyperlink))
    }

    /// Sets the click hyperlink on shape `shape_idx` itself to `link`, adding its relationship and
    /// removing the one any previous link named once unreferenced.
    fn set_shape_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        link: &Hyperlink,
    ) -> PyResult<()> {
        self.inner
            .set_shape_hyperlink(surface.0, shape_idx.0, &link.0)
            .map_err(to_py_err)
    }

    /// Removes the click hyperlink on shape `shape_idx` itself, and the relationship it named once
    /// unreferenced. A no-op if the shape has no hyperlink.
    fn clear_shape_hyperlink(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<()> {
        self.inner
            .clear_shape_hyperlink(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The raw bytes of the embedded object the OLE frame `shape_idx` on `surface` references
    /// (`/ppt/embeddings/oleObjectN.bin` or an embedded package), exactly as the package holds
    /// them, or `None` when the shape frames no OLE object. Borrowed from the package, so the part
    /// is not copied.
    fn ole_object_part_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .ole_object_part_bytes(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// The stored bytes of the OLE fallback snapshot image the frame `shape_idx` on `surface`
    /// embeds, exactly as the package holds them (never decoded or re-encoded), or `None` when the
    /// frame is not an OLE object or carries no snapshot. Borrowed from the package.
    fn ole_snapshot_image_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .ole_snapshot_image_bytes(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// The `progId` the OLE frame `shape_idx` on `surface` declares (e.g. `"Excel.Sheet.12"`) — the
    /// application that owns the embedded object — or `None` when the shape frames no OLE object or
    /// the attribute is absent. Reading does not dirty the part.
    fn ole_prog_id(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<String>> {
        self.inner
            .ole_prog_id(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Every OLE object frame on `surface`, with where its object data is referenced from and
    /// whether that reference is external.
    fn ole_objects(&mut self, surface: SurfaceArg) -> PyResult<Vec<OleObject>> {
        self.inner
            .ole_objects(surface.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(OleObject).collect())
    }

    /// Replaces the object data of the OLE frame `shape_idx` on `surface` with an in-package
    /// placeholder, so an object that points at unreachable external data resolves inside the
    /// package instead. The placeholder is `placeholder` if given, else `default_placeholder_ole`
    /// (a minimal valid compound file). The `p:oleObj` markup is unchanged — its relationship is
    /// simply retargeted at the placeholder — and the object keeps displaying via its snapshot
    /// image.
    #[pyo3(signature = (surface, shape_idx, placeholder = None))]
    fn replace_ole_object_with_placeholder(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        placeholder: Option<&[u8]>,
    ) -> PyResult<()> {
        self.inner
            .replace_ole_object_with_placeholder(surface.0, shape_idx.0, placeholder)
            .map_err(to_py_err)
    }

    /// The number of legacy **ActiveX** form controls on `surface` (`p:cSld > p:controls >
    /// p:control`).
    fn activex_control_count(&mut self, surface: SurfaceArg) -> PyResult<u32> {
        self.inner
            .activex_control_count(surface.0)
            .map_err(to_py_err)
    }

    /// The `name` the ActiveX control `control_idx` on `surface` declares (e.g.
    /// `"CommandButton1"`), or `None` when there is no such control or it is unnamed. Reading does
    /// not dirty the part.
    fn activex_control_name(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .activex_control_name(surface.0, control_idx)
            .map_err(to_py_err)
    }

    /// The raw bytes of the ActiveX control part (`ax:ocx` markup) the control `control_idx` on
    /// `surface` references, exactly as the package holds them, or `None` when there is no such
    /// control. Borrowed from the package; reading does not dirty anything.
    fn activex_part_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .activex_part_bytes(surface.0, control_idx)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// The ActiveX control's **persisted state** — the bytes of `/ppt/activeX/activeXN.bin` — for
    /// the control `control_idx` on `surface`, or `None` when there is no such control or it
    /// persists no state. Borrowed from the package; reading does not dirty anything.
    fn activex_state_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .activex_state_bytes(surface.0, control_idx)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// The stored bytes of the ActiveX control's fallback snapshot image for the control
    /// `control_idx` on `surface`, exactly as the package holds them (never decoded or re-encoded),
    /// or `None` when there is no such control or snapshot. Borrowed from the package.
    fn activex_snapshot_image_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .activex_snapshot_image_bytes(surface.0, control_idx)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// The names of every legacy **VML** drawing part in the package
    /// (`ppt/drawings/vmlDrawingN.vml` and the like), in package order.
    // Behind the `vml` feature, exactly as it is one layer down: the typed VML
    // model is opt-in.
    #[cfg(feature = "vml")]
    fn vml_part_names(&self) -> Vec<String> {
        self.inner.vml_part_names()
    }

    /// The raw bytes of the VML drawing `part`, exactly as the package holds them, or `None` when
    /// the package has no such part (or it has been edited elsewhere). Borrowed from the package,
    /// so the part is not copied and nothing is dirtied.
    // Behind the `vml` feature, exactly as it is one layer down: the typed VML
    // model is opt-in.
    #[cfg(feature = "vml")]
    fn vml_part_bytes<'py>(&self, python: Python<'py>, part: &str) -> Option<Bound<'py, PyBytes>> {
        self.inner
            .vml_part_bytes(part)
            .map(|bytes| PyBytes::new(python, &bytes))
    }

    /// The names of every **ink** (InkML) part in the package (`ppt/ink/inkN.xml`), in package
    /// order.
    fn ink_part_names(&self) -> Vec<String> {
        self.inner.ink_part_names()
    }

    /// The raw bytes of the ink (InkML) `part`, exactly as the package holds them, or `None` when
    /// the package has no such part (or it has been edited elsewhere). Borrowed from the package,
    /// so the part is not copied and nothing is dirtied.
    fn ink_part_bytes<'py>(&self, python: Python<'py>, part: &str) -> Option<Bound<'py, PyBytes>> {
        self.inner
            .ink_part_bytes(part)
            .map(|bytes| PyBytes::new(python, &bytes))
    }

    /// Every ink (InkML) part `surface` references, with where it is referenced from.
    fn ink_references(&mut self, surface: SurfaceArg) -> PyResult<Vec<InkReference>> {
        self.inner
            .ink_references(surface.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(InkReference).collect())
    }

    /// The ink part the shape `shape_idx` on `surface` references, or `None` when that shape is not
    /// a content part or does not reference ink.
    fn ink_part_for_shape(
        &mut self,
        surface: SurfaceArg,
        shape_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .ink_part_for_shape(surface.0, shape_idx)
            .map_err(to_py_err)
    }

    /// The shape index of the content part on `surface` that references the ink `part`, or `None`
    /// when no shape on that surface does (or the reference lives inside an `mc:AlternateContent`,
    /// which is out of the shape index space).
    fn shape_for_ink_part(&mut self, surface: SurfaceArg, part: &str) -> PyResult<Option<u32>> {
        self.inner
            .shape_for_ink_part(surface.0, part)
            .map_err(to_py_err)
    }

    /// Adds an ink (InkML) part holding `inkml` to the package and a `p:contentPart` referencing it
    /// to `surface`, and returns the new shape's index in the one shape index space.
    fn add_ink(&mut self, surface: SurfaceArg, inkml: &[u8]) -> PyResult<u32> {
        self.inner.add_ink(surface.0, inkml).map_err(to_py_err)
    }

    /// Replaces the strokes of the ink the shape `shape_idx` on `surface` references, in place.
    fn set_ink_content(
        &mut self,
        surface: SurfaceArg,
        shape_idx: u32,
        inkml: &[u8],
    ) -> PyResult<()> {
        self.inner
            .set_ink_content(surface.0, shape_idx, inkml)
            .map_err(to_py_err)
    }

    /// The four relationship ids the SmartArt frame `shape_idx` on `surface` names in its
    /// `dgm:relIds`, or `None` when the shape frames no diagram. Reading does not dirty the part.
    fn diagram_relationship_ids(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<DiagramRelationshipIds>> {
        self.inner
            .diagram_relationship_ids(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(DiagramRelationshipIds))
    }

    /// The parts of the SmartArt diagram the frame `shape_idx` on `surface` references, resolved to
    /// part names — the relationship graph behind the diagram, `None` when the shape frames none.
    fn diagram_parts(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<DiagramParts>> {
        self.inner
            .diagram_parts(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(DiagramParts))
    }

    /// The raw bytes of a diagram `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed; nothing is dirtied.
    fn diagram_part_bytes<'py>(
        &self,
        python: Python<'py>,
        part: &str,
    ) -> Option<Bound<'py, PyBytes>> {
        self.inner
            .diagram_part_bytes(part)
            .map(|bytes| PyBytes::new(python, &bytes))
    }

    /// Adds a SmartArt diagram to `surface`, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    fn add_diagram(
        &mut self,
        surface: SurfaceArg,
        content: &DiagramContent,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_diagram(surface.0, &content.0, bounds.0)
            .map_err(to_py_err)
    }

    /// Replaces one part of the SmartArt diagram the frame `shape_idx` on `surface` references, in
    /// place.
    fn set_diagram_part(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        kind: DiagramPartKind,
        bytes: Vec<u8>,
    ) -> PyResult<()> {
        self.inner
            .set_diagram_part(surface.0, shape_idx.0, kind.into(), bytes)
            .map_err(to_py_err)
    }

    /// Adds an OLE object to `surface`, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    fn add_ole_object(
        &mut self,
        surface: SurfaceArg,
        spec: &OleObjectSpec,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_ole_object(surface.0, &spec.borrowed(), bounds.0)
            .map_err(to_py_err)
    }

    /// Sets the `progId` of the OLE frame `shape_idx` on `surface` — which application owns the
    /// embedded object. Only the surface's part is dirtied.
    fn set_ole_prog_id(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        prog_id: &str,
    ) -> PyResult<()> {
        self.inner
            .set_ole_prog_id(surface.0, shape_idx.0, prog_id)
            .map_err(to_py_err)
    }

    /// Replaces the data of the OLE object the frame `shape_idx` on `surface` embeds, in place.
    fn set_ole_object_data(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        bytes: &[u8],
    ) -> PyResult<()> {
        self.inner
            .set_ole_object_data(surface.0, shape_idx.0, bytes)
            .map_err(to_py_err)
    }

    /// Replaces the fallback snapshot image of the OLE frame `shape_idx` on `surface` — the picture
    /// a consumer draws in place of the object it will never run.
    fn set_ole_snapshot_image(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        bytes: &[u8],
    ) -> PyResult<()> {
        self.inner
            .set_ole_snapshot_image(surface.0, shape_idx.0, bytes)
            .map_err(to_py_err)
    }

    /// Adds an ActiveX form control to `surface`, laid out inside `bounds`, and returns its index
    /// in the surface's **control** index space (not the shape index space — a `p:control` is a
    /// sibling of the shape tree, not a member of it).
    fn add_activex_control(
        &mut self,
        surface: SurfaceArg,
        spec: &ActiveXControlSpec,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_activex_control(surface.0, &spec.borrowed(), bounds.0)
            .map_err(to_py_err)
    }

    /// Points the OLE frame `shape_idx` on `surface` at the VML shape with `identifier`
    /// (`p:oleObj@spid`) — how an authored object is bound to the legacy fallback that draws it.
    fn set_ole_legacy_shape_id(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        identifier: &str,
    ) -> PyResult<()> {
        self.inner
            .set_ole_legacy_shape_id(surface.0, shape_idx.0, identifier)
            .map_err(to_py_err)
    }

    /// Points the ActiveX control `control_idx` on `surface` at the VML shape with `identifier`
    /// (`p:control@spid`). As `set_ole_legacy_shape_id`.
    fn set_activex_control_shape_id(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
        identifier: &str,
    ) -> PyResult<()> {
        self.inner
            .set_activex_control_shape_id(surface.0, control_idx, identifier)
            .map_err(to_py_err)
    }

    /// The `spid` the ActiveX control `control_idx` on `surface` names — the `id` of the VML shape
    /// that draws it in a legacy consumer — or `None` when there is no such control or it names
    /// none.
    fn activex_control_shape_id(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .activex_control_shape_id(surface.0, control_idx)
            .map_err(to_py_err)
    }

    /// The COM class id the ActiveX control `control_idx` on `surface` names (`ax:ocx@ax:classid`),
    /// or `None` when there is no such control or its part states none.
    fn activex_class_id(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .activex_class_id(surface.0, control_idx)
            .map_err(to_py_err)
    }

    /// How the ActiveX control `control_idx` on `surface` persists its state
    /// (`ax:ocx@ax:persistence`), or `None` when there is no such control, its part states none, or
    /// it names a value the ActiveX part does not define.
    fn activex_persistence(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
    ) -> PyResult<Option<ActiveXPersistence>> {
        self.inner
            .activex_persistence(surface.0, control_idx)
            .map_err(to_py_err)?
            .map(ActiveXPersistence::from_model)
            .transpose()
    }

    /// Renames the ActiveX control `control_idx` on `surface` (`p:control@name`). Only the
    /// surface's part is dirtied.
    fn set_activex_control_name(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
        name: &str,
    ) -> PyResult<()> {
        self.inner
            .set_activex_control_name(surface.0, control_idx, name)
            .map_err(to_py_err)
    }

    /// Replaces the persisted state of the ActiveX control `control_idx` on `surface`, in place.
    fn set_activex_state(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
        state: &[u8],
    ) -> PyResult<()> {
        self.inner
            .set_activex_state(surface.0, control_idx, state)
            .map_err(to_py_err)
    }

    /// Replaces the fallback snapshot image of the ActiveX control `control_idx` on `surface` — the
    /// picture a consumer draws in place of the control it will never run.
    fn set_activex_snapshot_image(
        &mut self,
        surface: SurfaceArg,
        control_idx: u32,
        bytes: &[u8],
    ) -> PyResult<()> {
        self.inner
            .set_activex_snapshot_image(surface.0, control_idx, bytes)
            .map_err(to_py_err)
    }

    /// Removes the ActiveX control `control_idx` from `surface`, closing the gap in the control
    /// index space. Only the surface's part is dirtied.
    fn remove_activex_control(&mut self, surface: SurfaceArg, control_idx: u32) -> PyResult<()> {
        self.inner
            .remove_activex_control(surface.0, control_idx)
            .map_err(to_py_err)
    }

    /// The `spid` the OLE frame `shape_idx` on `surface` names — the `id` of the VML shape that
    /// draws it in a legacy consumer — or `None` when the shape frames no OLE object or names no
    /// `spid`.
    fn ole_legacy_shape_id(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<String>> {
        self.inner
            .ole_legacy_shape_id(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The legacy VML drawing part `surface` relates to, or `None` when it has none.
    // Behind the `vml` feature, exactly as it is one layer down: the typed VML
    // model is opt-in.
    #[cfg(feature = "vml")]
    fn vml_drawing_part(&self, surface: SurfaceArg) -> PyResult<Option<String>> {
        self.inner.vml_drawing_part(surface.0).map_err(to_py_err)
    }

    /// Stores `drawing` as a new legacy VML drawing part and relates it to `surface`, returning the
    /// part's name.
    // Behind the `vml` feature, exactly as it is one layer down: the typed VML
    // model is opt-in.
    #[cfg(feature = "vml")]
    fn add_vml_drawing(&mut self, surface: SurfaceArg, drawing: &[u8]) -> PyResult<String> {
        self.inner
            .add_vml_drawing(surface.0, drawing)
            .map_err(to_py_err)
    }

    /// The speaker notes of slide `slide_idx` — the text of its notes slide's `body` placeholder —
    /// or `None` if the slide has no notes slide (or its notes slide has no body placeholder).
    fn notes_text(&mut self, slide_idx: u32) -> PyResult<Option<String>> {
        self.inner.notes_text(slide_idx).map_err(to_py_err)
    }

    /// Sets the speaker notes of slide `slide_idx` to `text`, creating the notes slide (and, if the
    /// deck has none, the notes master it follows) on demand.
    fn set_notes_text(&mut self, slide_idx: u32, text: &str) -> PyResult<()> {
        self.inner
            .set_notes_text(slide_idx, text)
            .map_err(to_py_err)
    }

    /// Removes the speaker notes of slide `slide_idx`: unwires the slide → notes-slide relationship
    /// and removes the notes slide part (with its `.rels` and content-type override). A no-op if
    /// the slide has no notes.
    fn clear_notes(&mut self, slide_idx: u32) -> PyResult<()> {
        self.inner.clear_notes(slide_idx).map_err(to_py_err)
    }

    /// Appends a picture (`p:pic`) showing `bytes` to `surface`, laid out at `bounds`. Returns the
    /// index of the new shape in the slide's one shape index space (see `shape_count`);
    /// `shape_kind` reports it as `ShapeKind::Picture`, and the whole `p:spPr` surface — outline,
    /// effects, geometry — applies to it like any other shape.
    fn add_picture(
        &mut self,
        surface: SurfaceArg,
        bytes: &[u8],
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_picture(surface.0, bytes, bounds.0)
            .map_err(to_py_err)
    }

    /// Every audio/video/media relationship on `surface`, with where each is referenced from and
    /// whether it is external.
    fn media_references(&mut self, surface: SurfaceArg) -> PyResult<Vec<MediaReference>> {
        self.inner
            .media_references(surface.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(MediaReference).collect())
    }

    /// Replaces the media that relationship `rel_id` on `surface` binds with an in-package
    /// placeholder, so a reference to unreachable external audio/video resolves inside the package
    /// instead. The placeholder is `placeholder` if given, else a built-in one matching the media
    /// kind — a valid silent WAV for audio (`default_placeholder_audio`) or a minimal MP4 for video
    /// (`default_placeholder_video`). The relationship is retargeted at the placeholder, so every
    /// carrier that named it — the `p:pic`, its `a14:media` fallback, timing/transition sounds —
    /// now resolves locally; the poster image is untouched.
    #[pyo3(signature = (surface, rel_id, placeholder = None))]
    fn replace_media_with_placeholder(
        &mut self,
        surface: SurfaceArg,
        rel_id: &str,
        placeholder: Option<&[u8]>,
    ) -> PyResult<()> {
        self.inner
            .replace_media_with_placeholder(surface.0, rel_id, placeholder)
            .map_err(to_py_err)
    }

    /// The target of the image that picture `shape_idx` on `surface` *links* (`p:blipFill >
    /// a:blip@r:link`), exactly as the relationship records it — an external path/URL for the
    /// common case, or an in-package part target for an internal link. `None` when the picture
    /// embeds its image (or binds none): an embedded image has no separate target, its bytes are
    /// the image.
    fn picture_image_link_target(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<String>> {
        self.inner
            .picture_image_link_target(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The stored bytes of the image that picture `shape_idx` on `surface` binds, exactly as the
    /// package holds them (never decoded or re-encoded), or `None` when the picture binds no image.
    /// Borrowed from the package, so a large image is not copied.
    fn picture_image_bytes<'py>(
        &mut self,
        python: Python<'py>,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        self.inner
            .picture_image_bytes(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|bytes| bytes.map(|bytes| PyBytes::new(python, &bytes)))
    }

    /// Points picture `shape_idx` on `surface` at `bytes`, adding the image to the package if it is
    /// not already there (`add_image`, so identical bytes are stored once) and rewriting the blip's
    /// `@r:embed`. Any `@r:link` is dropped — the picture now embeds its image — and the rest of
    /// the `p:blipFill` (source rect, tile/stretch) is preserved.
    fn set_picture_image(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        bytes: &[u8],
    ) -> PyResult<()> {
        self.inner
            .set_picture_image(surface.0, shape_idx.0, bytes)
            .map_err(to_py_err)
    }

    /// Every picture on `surface` that *links* its image (`a:blip@r:link`) rather than embedding
    /// it, with where each links from — the candidates for `replace_linked_image_with_placeholder`.
    /// A linked image is the common source that can be unreachable on another platform; this saves
    /// the caller from walking the shapes themselves. Reading does not dirty the part.
    fn linked_images(&mut self, surface: SurfaceArg) -> PyResult<Vec<LinkedImage>> {
        self.inner
            .linked_images(surface.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(LinkedImage).collect())
    }

    /// Replaces the *linked* image of picture `shape_idx` on `surface` with an embedded
    /// placeholder, so a picture that points at an unreachable external file resolves inside the
    /// package instead. The placeholder is `placeholder` if given, else
    /// `DEFAULT_PLACEHOLDER_IMAGE`. The picture becomes an ordinary embedded picture (`@r:link` →
    /// `@r:embed`), keeping its bounds and the rest of its `p:blipFill`, and the now-unused link
    /// relationship is dropped.
    #[pyo3(signature = (surface, shape_idx, placeholder = None))]
    fn replace_linked_image_with_placeholder(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        placeholder: Option<&[u8]>,
    ) -> PyResult<()> {
        self.inner
            .replace_linked_image_with_placeholder(surface.0, shape_idx.0, placeholder)
            .map_err(to_py_err)
    }

    /// Stores `bytes` as an image part of the package and relates it to `surface`, returning the
    /// **slide-scoped relationship id** that names the image — the `rel_id` to hand to
    /// `FillSpec::Picture` via `set_shape_fill`.
    fn add_image(&mut self, surface: SurfaceArg, bytes: &[u8]) -> PyResult<String> {
        self.inner.add_image(surface.0, bytes).map_err(to_py_err)
    }

    /// The number of **top-level** shapes on `surface` — of **every** `ShapeKind` (autoshapes,
    /// pictures, groups, graphic frames, connectors), in document order. A group counts as one
    /// shape here; its own members are addressed by descending into it with a `ShapePath` and are
    /// not included in this count.
    fn shape_count(&mut self, surface: SurfaceArg) -> PyResult<u32> {
        self.inner.shape_count(surface.0).map_err(to_py_err)
    }

    /// What kind of shape `shape_idx` on `surface` is — which of the index-addressed APIs apply to
    /// it (a `Picture` takes the `p:spPr` surface but has no text body; a `GroupShape` has no
    /// `p:spPr` at all).
    fn shape_kind(&mut self, surface: SurfaceArg, shape_idx: ShapePathArg) -> PyResult<ShapeKind> {
        ShapeKind::from_model(
            self.inner
                .shape_kind(surface.0, shape_idx.0)
                .map_err(to_py_err)?,
        )
    }

    /// How many member shapes the group at `shape_idx` holds — `0` for anything that is not a
    /// group, since only a `p:grpSp` has members. This is the range a `ShapePath` may descend into.
    fn shape_member_count(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<u32> {
        self.inner
            .shape_member_count(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Every shape of `surface`, in document order — what it is and the placeholder slot it fills.
    fn shapes(&mut self, surface: SurfaceArg) -> PyResult<Vec<ShapeInfo>> {
        self.inner
            .shapes(surface.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ShapeInfo).collect())
    }

    /// The address of the first shape on `surface` that fills the `kind` placeholder slot, or
    /// `None` if the surface offers none.
    fn shape_for_placeholder(
        &mut self,
        surface: SurfaceArg,
        kind: PlaceholderType,
    ) -> PyResult<Option<u32>> {
        self.inner
            .shape_for_placeholder(surface.0, kind.into())
            .map_err(to_py_err)
    }

    /// The placeholder shape `shape_idx` on `surface` occupies (`p:nvPr > p:ph`), or `None` if it
    /// is not a placeholder.
    fn shape_placeholder(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<PlaceholderInfo>> {
        self.inner
            .shape_placeholder(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(PlaceholderInfo))
    }

    /// Appends a new rectangular text-box shape (`p:sp`) to `surface`, laid out at `bounds` and
    /// containing `text` (one paragraph per line, split on `\n`). Returns the index of the new
    /// shape in the slide's one shape index space (see `shape_count`). Only that part is marked
    /// dirty.
    fn add_text_box(
        &mut self,
        surface: SurfaceArg,
        text: &str,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_text_box(surface.0, text, bounds.0)
            .map_err(to_py_err)
    }

    /// Appends a new autoshape (`p:sp`) with the given `preset` geometry to `surface`, laid out at
    /// `bounds`, with an empty text body. Returns the index of the new shape in the slide's one
    /// shape index space (see `shape_count`). Only that part is marked dirty.
    fn add_shape(
        &mut self,
        surface: SurfaceArg,
        preset: PresetShapeType,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_shape(surface.0, preset.into(), bounds.0)
            .map_err(to_py_err)
    }

    /// Removes shape `shape_idx` from `surface`, closing the gap in the shape index space: every
    /// later shape on that surface moves down one index. Only that part is marked dirty.
    fn remove_shape(&mut self, surface: SurfaceArg, shape_idx: ShapePathArg) -> PyResult<()> {
        self.inner
            .remove_shape(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Wraps `members` — which must be siblings — in a new group, returning the group's address.
    fn group_shapes(
        &mut self,
        surface: SurfaceArg,
        members: Vec<ShapePathArg>,
    ) -> PyResult<ShapePath> {
        self.inner
            .group_shapes(
                surface.0,
                &members.into_iter().map(|path| path.0).collect::<Vec<_>>(),
            )
            .map_err(to_py_err)
            .map(ShapePath)
    }

    /// Dissolves the group at `shape_idx`, returning where its members now are.
    fn ungroup(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Vec<ShapePath>> {
        self.inner
            .ungroup(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|values| values.into_iter().map(ShapePath).collect())
    }

    /// Moves shape `shape_idx` into the group at `group_idx`, as its last member, and returns its
    /// new address.
    fn move_shape_into_group(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        group_idx: ShapePathArg,
    ) -> PyResult<ShapePath> {
        self.inner
            .move_shape_into_group(surface.0, shape_idx.0, group_idx.0)
            .map_err(to_py_err)
            .map(ShapePath)
    }

    /// Moves shape `shape_idx` out of the group holding it, into that group's own container and
    /// directly after it in z-order. Returns its new address.
    fn move_shape_out_of_group(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<ShapePath> {
        self.inner
            .move_shape_out_of_group(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(ShapePath)
    }

    /// What the graphic frame `shape_idx` on `surface` frames — a `Table`, a `Chart`, a `Diagram`
    /// or something else — or `None` when the shape is not a `p:graphicFrame` at all. Reading does
    /// not dirty the part.
    fn graphic_frame_kind(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<GraphicFrameKind>> {
        self.inner
            .graphic_frame_kind(surface.0, shape_idx.0)
            .map_err(to_py_err)?
            .map(GraphicFrameKind::from_model)
            .transpose()
    }

    /// Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0 —
    /// or, on a deck with no slides yet, to the deck's first layout — and returns its index. The
    /// new slide is a blank shape tree; add content with `add_text_box` or use
    /// `add_slide_with_text`.
    fn add_slide(&mut self) -> PyResult<u32> {
        self.inner.add_slide().map_err(to_py_err)
    }

    /// Adds a new slide at the end of the deck built on layout `layout_idx`, carrying a copy of
    /// every placeholder that layout declares, and returns the slide's index.
    fn add_slide_from_layout(&mut self, layout_idx: u32) -> PyResult<u32> {
        self.inner
            .add_slide_from_layout(layout_idx)
            .map_err(to_py_err)
    }

    /// Removes slide `slide_idx` from the deck, unwiring it completely: the `p:sldId` naming it,
    /// the presentation's relationship to it, the slide part, its own `.rels`, and its content-type
    /// `Override`.
    fn remove_slide(&mut self, slide_idx: u32) -> PyResult<()> {
        self.inner.remove_slide(slide_idx).map_err(to_py_err)
    }

    /// Adds a new slide (via `add_slide`) carrying a single text box with `text` laid out at
    /// `bounds`, and returns the new slide's index.
    fn add_slide_with_text(&mut self, text: &str, bounds: ShapeBounds) -> PyResult<u32> {
        self.inner
            .add_slide_with_text(text, bounds.0)
            .map_err(to_py_err)
    }

    /// Adds a `rows` x `columns` table to `surface`, laid out inside `bounds`, and returns its
    /// index in the shape tree.
    fn add_table(
        &mut self,
        surface: SurfaceArg,
        rows: u32,
        columns: u32,
        bounds: ShapeBounds,
    ) -> PyResult<u32> {
        self.inner
            .add_table(surface.0, rows, columns, bounds.0)
            .map_err(to_py_err)
    }

    /// The shape of the table shape `shape_idx` on `surface` frames, as `(rows, columns)`.
    fn table_dimensions(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<(u32, u32)> {
        self.inner
            .table_dimensions(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The width of column `column` of the table shape `shape_idx` frames, or `None` if the column
    /// states none. Reading does not dirty the part.
    fn column_width(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        column: u32,
    ) -> PyResult<Option<Emu>> {
        self.inner
            .column_width(surface.0, shape_idx.0, column)
            .map_err(to_py_err)
            .map(|value| value.map(Emu))
    }

    /// Sets the width of column `column`. Marks only that part dirty.
    fn set_column_width(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        column: u32,
        width: Emu,
    ) -> PyResult<()> {
        self.inner
            .set_column_width(surface.0, shape_idx.0, column, width.0)
            .map_err(to_py_err)
    }

    /// The height row `row` asks for, or `None` if it states none. PowerPoint grows a row whose
    /// content does not fit, so a rendered row is never shorter than this but may be taller.
    fn row_height(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
    ) -> PyResult<Option<Emu>> {
        self.inner
            .row_height(surface.0, shape_idx.0, row)
            .map_err(to_py_err)
            .map(|value| value.map(Emu))
    }

    /// Sets the height row `row` asks for. Marks only that part dirty.
    fn set_row_height(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        height: Emu,
    ) -> PyResult<()> {
        self.inner
            .set_row_height(surface.0, shape_idx.0, row, height.0)
            .map_err(to_py_err)
    }

    /// Inserts a row into the table shape `shape_idx` frames so it becomes row `row`; `row` equal
    /// to the current row count appends at the end. The new row copies the height of the row beside
    /// it and its cells are empty and ready for `set_cell_text`. A merge the new row falls inside
    /// grows to include it. Marks only that part dirty; the frame's own bounds are **not** enlarged
    /// (as PowerPoint does not either — resize with `set_shape_bounds`).
    fn insert_row(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
    ) -> PyResult<()> {
        self.inner
            .insert_row(surface.0, shape_idx.0, row)
            .map_err(to_py_err)
    }

    /// Removes row `row` from the table shape `shape_idx` frames. A merge the row lies inside
    /// shrinks; a merge anchored in the row promotes the cell below it, which takes over the
    /// anchor's text and formatting so the table looks unchanged. Marks only that part dirty.
    fn remove_row(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
    ) -> PyResult<()> {
        self.inner
            .remove_row(surface.0, shape_idx.0, row)
            .map_err(to_py_err)
    }

    /// Inserts a column into the table shape `shape_idx` frames so it becomes column `column`;
    /// `column` equal to the current column count appends. The grid gains one `a:gridCol` (width
    /// copied from the column beside it) and every row gains one empty cell, so the grid and rows
    /// stay in step. A merge the new column falls inside grows to include it. Marks only that part
    /// dirty; the frame's own bounds are **not** enlarged.
    fn insert_column(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        column: u32,
    ) -> PyResult<()> {
        self.inner
            .insert_column(surface.0, shape_idx.0, column)
            .map_err(to_py_err)
    }

    /// Removes column `column` from the table shape `shape_idx` frames: its `a:gridCol` and one
    /// cell from every row, together. A merge the column lies inside shrinks; a merge anchored in
    /// the column promotes the cell to its right, which takes over the anchor's text and
    /// formatting. Marks only that part dirty.
    fn remove_column(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        column: u32,
    ) -> PyResult<()> {
        self.inner
            .remove_column(surface.0, shape_idx.0, column)
            .map_err(to_py_err)
    }

    /// How many rows and columns the cell at `(row, column)` spans, as `(rows, columns)` — the same
    /// order `table_dimensions` answers in, and the order every address on this surface is written
    /// in.
    fn cell_span(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<(u32, u32)> {
        self.inner
            .cell_span(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// Which cell actually renders at `(row, column)` — itself when it is not merged away, or the
    /// anchor of the merged region covering it.
    fn merged_cell_anchor(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        row: u32,
        column: u32,
    ) -> PyResult<(u32, u32)> {
        self.inner
            .merged_cell_anchor(surface.0, shape_idx.0, row, column)
            .map_err(to_py_err)
    }

    /// Whether the table shape `shape_idx` frames declares banding/emphasis `part` (a `a:tblPr`
    /// flag), or `None` if it does not state the flag. Reading does not dirty the part.
    fn table_part(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        part: TablePart,
    ) -> PyResult<Option<bool>> {
        self.inner
            .table_part(surface.0, shape_idx.0, part.into())
            .map_err(to_py_err)
    }

    /// Turns a table's banding/emphasis flag `part` on or off, creating its `a:tblPr` if it had
    /// none. `false` removes the flag rather than writing a `"0"`. Marks only that part dirty.
    fn set_table_part(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        part: TablePart,
        on: bool,
    ) -> PyResult<()> {
        self.inner
            .set_table_part(surface.0, shape_idx.0, part.into(), on)
            .map_err(to_py_err)
    }

    /// The GUID of the table style the table shape `shape_idx` frames names (`a:tableStyleId`), or
    /// `None` if it names none. Reading does not dirty the part.
    fn table_style_id(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<String>> {
        self.inner
            .table_style_id(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Points the table shape `shape_idx` frames at the table style `style_id`, creating its
    /// `a:tblPr` if it had none. Does not check that the style exists — pair it with
    /// `create_table_style`. Marks only that part dirty.
    fn set_table_style(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        style_id: &str,
    ) -> PyResult<()> {
        self.inner
            .set_table_style(surface.0, shape_idx.0, style_id)
            .map_err(to_py_err)
    }

    /// Creates the presentation's `tableStyles.xml` part if it has none, and adds a style with GUID
    /// `style_id` and gallery name `style_name` — replacing one already carrying that GUID. The
    /// style is born empty; give its parts formatting with `format_table_style_part`, and point a
    /// table at it with `set_table_style`.
    fn create_table_style(&mut self, style_id: &str, style_name: &str) -> PyResult<()> {
        self.inner
            .create_table_style(style_id, style_name)
            .map_err(to_py_err)
    }

    /// Sets the formatting the style `style_id` gives table `part` (`wholeTbl`, `firstRow`, a
    /// banded row, a corner cell). Only the facets `format` sets are written; the part keeps
    /// whatever else it held. Marks only the `tableStyles.xml` part dirty.
    fn format_table_style_part(
        &mut self,
        style_id: &str,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> PyResult<()> {
        self.inner
            .format_table_style_part(style_id, part.into(), &format.0)
            .map_err(to_py_err)
    }

    /// Gives the table shape `shape_idx` frames its own **inline** style (`a:tableStyle`),
    /// replacing any inline or referenced style it had — the lean alternative to a shared
    /// `tableStyles.xml` style: the whole look is spelled out in `definition` and travels with the
    /// table, so no shared part, relationship or referenced GUID is involved. Marks only that part
    /// dirty.
    fn set_inline_table_style(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        definition: &TableStyleDefinition,
    ) -> PyResult<()> {
        self.inner
            .set_inline_table_style(surface.0, shape_idx.0, &definition.0)
            .map_err(to_py_err)
    }

    /// Sets the formatting the table's **inline** style gives one `part`, creating the inline style
    /// if the table had none — the incremental sibling of `set_inline_table_style`, mirroring
    /// `format_table_style_part` for a self-contained style. Only the facets `format` sets are
    /// written. Marks only that part dirty.
    fn format_inline_table_style_part(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> PyResult<()> {
        self.inner
            .format_inline_table_style_part(surface.0, shape_idx.0, part.into(), &format.0)
            .map_err(to_py_err)
    }

    /// The full text of shape `shape_idx` on `surface` (paragraphs joined by `\n`).
    fn shape_text(&mut self, surface: SurfaceArg, shape_idx: ShapePathArg) -> PyResult<String> {
        self.inner
            .shape_text(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in
    /// document order) of shape `shape_idx` on `surface`. Marks only that part dirty.
    fn set_shape_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        run_idx: u32,
        text: &str,
    ) -> PyResult<()> {
        self.inner
            .set_shape_text(surface.0, shape_idx.0, run_idx, text)
            .map_err(to_py_err)
    }

    /// Replaces the **whole text** of shape `shape_idx` on `surface` with `text` — one paragraph
    /// per line, each holding exactly one run, so `shape_text` reads back exactly what was written.
    /// Marks only that part dirty.
    fn set_shape_text_content(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        text: &str,
    ) -> PyResult<()> {
        self.inner
            .set_shape_text_content(surface.0, shape_idx.0, text)
            .map_err(to_py_err)
    }

    /// The number of paragraphs in shape `shape_idx`'s text body. Reading does not dirty the part.
    fn paragraph_count(&mut self, surface: SurfaceArg, shape_idx: ShapePathArg) -> PyResult<u32> {
        self.inner
            .paragraph_count(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// The number of runs in paragraph `para_idx` of shape `shape_idx`. Reading does not dirty the
    /// part.
    fn run_count(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .run_count(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
    }

    /// The text of paragraph `para_idx` — its runs concatenated. Reading does not dirty the part.
    fn paragraph_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<String> {
        self.inner
            .paragraph_text(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
    }

    /// The text of one run. Reading does not dirty the part.
    fn run_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<String> {
        self.inner
            .run_text(surface.0, shape_idx.0, para_idx, run_idx)
            .map_err(to_py_err)
    }

    /// The number of text fields (`a:fld`) in paragraph `para_idx` — generated values such as a
    /// slide number or a date. Fields are a **separate index space** from the runs, so a field
    /// never shifts a run index. Reading does not dirty the part.
    fn paragraph_field_count(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .paragraph_field_count(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
    }

    /// The cached text of field `field_idx` in paragraph `para_idx` — the value the producer last
    /// computed for it (a slide number, a formatted date), not a live value. Reading does not dirty
    /// the part.
    fn paragraph_field_text(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        field_idx: u32,
    ) -> PyResult<String> {
        self.inner
            .paragraph_field_text(surface.0, shape_idx.0, para_idx, field_idx)
            .map_err(to_py_err)
    }

    /// What field `field_idx` in paragraph `para_idx` generates (`a:fld@type`, e.g. `slidenum` or
    /// `datetime`), or `None` if it names no type. Reading does not dirty the part.
    fn paragraph_field_type(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        field_idx: u32,
    ) -> PyResult<Option<String>> {
        self.inner
            .paragraph_field_type(surface.0, shape_idx.0, para_idx, field_idx)
            .map_err(to_py_err)
    }

    /// The layout properties a paragraph declares of its own (`a:pPr`), or `None` if it declares
    /// none — in which case every property is inherited. Reading does not dirty the part.
    fn paragraph_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<Option<ParagraphPropertiesSpec>> {
        self.inner
            .paragraph_properties(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
            .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// The character properties a run declares of its own (`a:rPr`), or `None` if it declares none.
    /// Reading does not dirty the part.
    fn run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> PyResult<Option<CharacterPropertiesSpec>> {
        self.inner
            .run_properties(surface.0, shape_idx.0, para_idx, run_idx)
            .map_err(to_py_err)
            .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares none.
    fn end_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<Option<CharacterPropertiesSpec>> {
        self.inner
            .end_run_properties(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
            .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// Applies `spec` to one run's character properties, creating its `a:rPr` if it has none.
    fn set_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        run_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_run_properties(surface.0, shape_idx.0, para_idx, run_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to **every run** in paragraph `para_idx`, and to its `a:endParaRPr` if it has
    /// one — so text typed at the end of the paragraph takes the same formatting, which is what
    /// selecting a paragraph and restyling it means.
    fn set_paragraph_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_paragraph_run_properties(surface.0, shape_idx.0, para_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to **every run of every paragraph** in the shape, and to each paragraph's
    /// `a:endParaRPr` where present — selecting a whole text box and restyling it.
    fn set_shape_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_run_properties(surface.0, shape_idx.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Merges adjacent runs in paragraph `para_idx` that would render identically, returning the
    /// number of runs merged away. This undoes the run splitting that `set_text_range_properties`
    /// does: formatting a sub-range splits a run, and repeatedly formatting overlapping ranges
    /// leaves a paragraph with more runs than it needs.
    fn coalesce_paragraph_runs(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
    ) -> PyResult<u32> {
        self.inner
            .coalesce_paragraph_runs(surface.0, shape_idx.0, para_idx)
            .map_err(to_py_err)
    }

    /// Merges adjacent identical runs across **every** paragraph of a shape's text body, returning
    /// the total number of runs merged away. The per-paragraph rule is `coalesce_paragraph_runs`.
    fn coalesce_shape_runs(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<u32> {
        self.inner
            .coalesce_shape_runs(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to the paragraph-mark properties (`a:endParaRPr`), creating the element if
    /// the paragraph has none.
    fn set_end_run_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_end_run_properties(surface.0, shape_idx.0, para_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if it has
    /// none. The properties **merge**, as run properties do.
    fn set_paragraph_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        spec: &ParagraphPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_paragraph_properties(surface.0, shape_idx.0, para_idx, &spec.0)
            .map_err(to_py_err)
    }

    /// The layout properties the shape's own list style offers at `level` (`a:lstStyle >
    /// a:lvlNpPr`), or `None` if it offers none there — or declares no list style at all. Reading
    /// does not dirty the part.
    fn shape_list_style_level(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        level: IndentLevel,
    ) -> PyResult<Option<ParagraphPropertiesSpec>> {
        self.inner
            .shape_list_style_level(surface.0, shape_idx.0, level.0)
            .map_err(to_py_err)
            .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// The properties the shape's own list style offers where no level applies (`a:lstStyle >
    /// a:defPPr`), or `None` if it declares none. Reading does not dirty the part.
    fn shape_list_style_default(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<Option<ParagraphPropertiesSpec>> {
        self.inner
            .shape_list_style_default(surface.0, shape_idx.0)
            .map_err(to_py_err)
            .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// Applies `spec` to what the shape's own list style offers at `level`, creating the
    /// `a:lstStyle` — and the `a:lvlNpPr` within it — if the shape has none. Marks only that part
    /// dirty.
    fn set_shape_list_style_level(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        level: IndentLevel,
        spec: &ParagraphPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_list_style_level(surface.0, shape_idx.0, level.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to what the shape's own list style offers where no level applies (`a:lstStyle
    /// > a:defPPr`), creating the elements if the shape has none. Marks only that part dirty.
    /// Merges as `set_shape_list_style_level` does.
    fn set_shape_list_style_default(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        spec: &ParagraphPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_shape_list_style_default(surface.0, shape_idx.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Removes what the shape's own list style offers at `level`, so the level falls through to the
    /// tier below again. Returns whether it offered anything there; a `false` changes nothing and
    /// does **not** dirty the part.
    fn clear_shape_list_style_level(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        level: IndentLevel,
    ) -> PyResult<bool> {
        self.inner
            .clear_shape_list_style_level(surface.0, shape_idx.0, level.0)
            .map_err(to_py_err)
    }

    /// Removes the default properties of the shape's own list style (`a:lstStyle > a:defPPr`).
    /// Returns whether it had any; a `false` changes nothing and does **not** dirty the part.
    fn clear_shape_list_style_default(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<bool> {
        self.inner
            .clear_shape_list_style_default(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Removes the shape's own list style entirely (`a:lstStyle`), so every level falls through to
    /// the tier below. Returns whether the shape had one; a `false` changes nothing and does
    /// **not** dirty the part.
    fn clear_shape_list_style(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
    ) -> PyResult<bool> {
        self.inner
            .clear_shape_list_style(surface.0, shape_idx.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **Unicode
    /// scalars** across the paragraph's whole text.
    fn set_text_range_properties(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        range: RangeArg,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_text_range_properties(surface.0, shape_idx.0, para_idx, range.0, &spec.0)
            .map_err(to_py_err)
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **grapheme
    /// clusters**: what a reader would call characters, and what a text selection actually spans.
    fn set_text_range_properties_by_grapheme(
        &mut self,
        surface: SurfaceArg,
        shape_idx: ShapePathArg,
        para_idx: u32,
        range: RangeArg,
        spec: &CharacterPropertiesSpec,
    ) -> PyResult<()> {
        self.inner
            .set_text_range_properties_by_grapheme(
                surface.0,
                shape_idx.0,
                para_idx,
                range.0,
                &spec.0,
            )
            .map_err(to_py_err)
    }
}

/// Adds the deck class to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Deck>()
}
