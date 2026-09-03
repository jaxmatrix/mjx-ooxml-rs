//! [`Deck`] — the whole PowerPoint surface, 257 methods, from JavaScript and TypeScript.
//!
//! ```js
//! import init, { Deck, SlideSize, ShapeBounds, CharacterPropertiesSpec } from "@mjx/ooxml";
//!
//! await init();
//! const deck = Deck.blank(SlideSize.widescreen());
//! try {
//!   const slide = deck.addSlideFromLayout(0);
//!   const title = deck.addTextBox(slide, "Quarterly results",
//!                                 ShapeBounds.fromInches(0.5, 0.4, 9.0, 1.2));
//!   deck.setShapeRunProperties(
//!     slide, title,
//!     new CharacterPropertiesSpec().withSizePoints(40).withBold(true));
//!   const blob = new Blob([deck.save()], {
//!     type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
//!   });
//! } finally {
//!   deck.free();   // ← not optional. See below.
//! }
//! ```
//!
//! # `free()` IS MANDATORY
//!
//! A `Deck` is memory on the wasm heap. JavaScript's garbage collector does not know it exists and
//! will never reclaim it: **a deck you do not free is leaked for the lifetime of the module.** A
//! multi-megabyte deck leaked in a loop will exhaust the wasm heap and every later call will throw.
//!
//! So: `try { … } finally { deck.free() }`, every time. The same is true of every class in this
//! package — a `FillSpec`, a `ShapeBounds`, a `Surface` — though those are small enough that
//! leaking one matters far less than leaking a deck. In a runtime with explicit resource management
//! (`using deck = Deck.open(bytes)`) the disposal is automatic; `wasm-bindgen` emits
//! `[Symbol.dispose]` for exactly that.
//!
//! Calling a method on a freed deck throws `Error: null pointer passed to rust`. That is not a
//! failure of this library; it is the double-free this design makes visible instead of silent.
//!
//! # Method names are camelCase
//!
//! Every method carries an explicit `js_name`, because a `snake_case` API is an immediate smell to
//! a TypeScript consumer: `deck.setShapeRunProperties(…)`. The rule is mechanical and total —
//! `set_shape_run_properties` → `setShapeRunProperties`, with each underscore removed and the next
//! letter capitalised, and no other change. Class names are already PascalCase and are unchanged;
//! enumeration members are already PascalCase and are unchanged.
//!
//! This is a deliberate divergence from the Python binding, whose mapping is the identity, because
//! Python's own standard library is `snake_case` and JavaScript's is not.
//!
//! # Addressing
//!
//! A surface is a `Surface` or a number (the slide index); a shape is a `ShapePath`, a number (a
//! top-level shape), or an array of numbers (a descent through groups). The numeric spellings
//! allocate nothing and need no `free()`, which is why the examples use them.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::address::{
    path_of, paths_of, surface_of, ShapePath, ShapePathArg, ShapePathListArg, SurfaceArg,
};
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
use crate::errors::map_error;
use crate::format::Format;
use crate::geometry::{
    BoundedAdjustment, CellMargins, Geometry, GuideContext, ShapeBounds, SlideSize, Transform2D,
};
use crate::measures::{Emu, IndentLevel};
use crate::paint::{ColorMap, EffectListSpec, FillSpec, LineSpec};
use crate::support::str_list;
use crate::tables::{CellFormat, Cells, TableStyleDefinition, TableStyleFormat};
use crate::text::{CharacterPropertiesSpec, ParagraphPropertiesSpec, ThemeInfo};
use crate::three_d::{Scene3DSpec, Shape3DSpec};

/// How many rows and columns something spans.
///
/// Rust returns a `(rows, columns)` tuple, which `wasm-bindgen` cannot project; a two-element array
/// would type as `number[]` and lose which half is which. Two named getters do not.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellExtent {
    rows: u32,
    columns: u32,
}

#[wasm_bindgen]
impl CellExtent {
    /// How many rows.
    #[wasm_bindgen(getter, js_name = "rows")]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// How many columns.
    #[wasm_bindgen(getter, js_name = "columns")]
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// `3×2`.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        format!("{}×{}", self.rows, self.columns)
    }
}

impl CellExtent {
    /// The extent, from the model's pair.
    pub(crate) fn new(rows: u32, columns: u32) -> Self {
        Self { rows, columns }
    }
}

/// Which cell of a table something is at.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    row: u32,
    column: u32,
}

#[wasm_bindgen]
impl CellAddress {
    /// The row, counting from zero.
    #[wasm_bindgen(getter, js_name = "row")]
    pub fn row(&self) -> u32 {
        self.row
    }

    /// The column, counting from zero.
    #[wasm_bindgen(getter, js_name = "column")]
    pub fn column(&self) -> u32 {
        self.column
    }

    /// `(1, 2)`.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_display_string(&self) -> String {
        format!("({}, {})", self.row, self.column)
    }
}

impl CellAddress {
    /// The address, from the model's pair.
    pub(crate) fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}

/// An open PowerPoint deck.
///
/// A deck comes from exactly two places — `Deck.blank` authors one from nothing, and `Deck.open`
/// reads one from bytes — so `new Deck()` throws rather than handing back something half-built.
///
/// **Call `free()` when you are done with it.** See the module documentation.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Deck {
    inner: ooxml::Deck,
}

#[wasm_bindgen]
impl Deck {
    /// A new deck with nothing in it: one slide master, one blank layout, a theme, and no slides.
    ///
    /// Nothing is read from disk and no template is embedded — every part is authored from this
    /// library's own element builders, which is what makes a deck buildable in a browser with no
    /// input file.
    ///
    /// Throws an `OoxmlError` with code `InvalidArgument` if the size is outside the
    /// 914 400–51 206 400 EMU range `p:sldSz` can express (1 to 56 inches on each axis).
    #[wasm_bindgen(js_name = "blank")]
    pub fn blank(size: &SlideSize) -> Result<Deck, JsValue> {
        map_error(ooxml::Deck::blank(size.0)).map(|inner| Self { inner })
    }

    /// Opens a deck from the bytes of a `.pptx`, `.pptm`, `.potx`, `.potm`, `.ppsx` or `.ppsm`.
    ///
    /// In a browser: `Deck.open(new Uint8Array(await file.arrayBuffer()))`.
    ///
    /// Throws an `OoxmlError` whose `code` is `Io` for bytes that are not a readable container,
    /// `MalformedDocument` for a package whose markup is not PresentationML, and
    /// `UnsupportedFormat` — naming the format — for a Word or Excel document.
    #[wasm_bindgen(js_name = "open")]
    pub fn open(data: &[u8]) -> Result<Deck, JsValue> {
        map_error(ooxml::Deck::open(data)).map(|inner| Self { inner })
    }

    /// What this deck's main part says it is. A deck authored by `blank` reports
    /// `Format.Presentation`.
    #[wasm_bindgen(js_name = "format")]
    pub fn format(&self) -> Result<Format, JsValue> {
        Format::from_model(self.inner.format())
    }

    /// The deck as the bytes of a `.pptx`, **validated first**.
    ///
    /// Every part that was never touched is re-emitted verbatim; only the parts an edit
    /// materialised are serialised from the model. In a browser:
    /// `new Blob([deck.save()], { type: … })`.
    ///
    /// Throws an `OoxmlError` with code `InvalidDocument` rather than emitting a file PowerPoint
    /// would offer to repair. `saveUnchecked` is the deliberate override.
    #[wasm_bindgen(js_name = "save")]
    pub fn save(&self) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.save())
    }

    /// The deck as bytes, **without** the validation pass.
    ///
    /// For the one case that needs it: writing a deck whose defect you already know about and
    /// intend to inspect.
    #[wasm_bindgen(js_name = "saveUnchecked")]
    pub fn save_unchecked(&self) -> Result<Vec<u8>, JsValue> {
        map_error(self.inner.save_unchecked())
    }

    /// Runs the packaging and PresentationML checks `save` runs, without writing anything.
    ///
    /// Throws an `OoxmlError` with code `InvalidDocument` describing the first defect found.
    #[wasm_bindgen(js_name = "validate")]
    pub fn validate(&self) -> Result<(), JsValue> {
        map_error(self.inner.validate())
    }

    // --- the delegated surface ------------------------------------------------------------------
    //
    // 251 methods, one per `mjx_ooxml::Deck` method that can cross a foreign function boundary.
    // Each takes its arguments in the classes this crate defines, calls exactly one method on the
    // deck, and returns an owned value. The documentation comments are `mjx-ooxml`'s own summaries,
    // so the `.d.ts` and the Rust documentation cannot drift apart.
    //
    // Three of the facade's methods are absent, and all three return a `Presentation`:
    // `presentation`, `presentation_mut` and `into_presentation`. They are the Rust-only escape
    // hatch to the `ShapeCursor` and the closure-taking readers — the things a binding cannot carry.
    //
    // One shape difference from the Rust: a `Range<u32>` argument becomes two numbers, `…Start` and
    // `…End`, because JavaScript has no half-open range and an object literal would type as `any`.

    /// The explicit fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`, or
    /// `None` if the shape declares no fill in its `p:spPr` (its fill is then inherited from the
    /// placeholder / style / theme — resolving that is a separate, future task). Reading does not
    /// dirty the part.
    #[wasm_bindgen(js_name = "shapeFill")]
    pub fn shape_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<FillSpec>, JsValue> {
        map_error(
            self.inner
                .shape_fill(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(FillSpec))
    }

    /// Sets the fill of shape `shape_idx` on `surface` from an interner-free `FillSpec`, rebuilding
    /// the `p:spPr` fill element (replacing an existing one in place, or inserting a new one after
    /// any geometry and before `a:ln`). Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeFill")]
    pub fn set_shape_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        fill: &FillSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_shape_fill(surface_of(surface)?, path_of(shape_idx)?, &fill.0),
        )
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no fill" (`a:noFill`). A shorthand for
    /// `set_shape_fill` with `FillSpec::None`.
    #[wasm_bindgen(js_name = "setShapeNoFill")]
    pub fn set_shape_no_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_shape_no_fill(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The **explicit** outline of shape `shape_idx` on `surface` — its `p:spPr > a:ln` as an
    /// interner- free `LineSpec` — or `None` when the shape declares no `a:ln` (its outline is then
    /// inherited; effective outline resolution is a later step). Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shapeOutline")]
    pub fn shape_outline(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<LineSpec>, JsValue> {
        map_error(
            self.inner
                .shape_outline(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(LineSpec))
    }

    /// Sets the outline of shape `shape_idx` on `surface` from an interner-free `LineSpec`,
    /// rebuilding the `p:spPr` `a:ln` element (replacing an existing one in place, or inserting a
    /// new one after any geometry and fill, before effects). Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeOutline")]
    pub fn set_shape_outline(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        line: &LineSpec,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_shape_outline(surface_of(surface)?, path_of(shape_idx)?, &line.0),
        )
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no outline"
    /// (`<a:ln><a:noFill/></a:ln>`). A shorthand for `set_shape_outline` with a `LineSpec` whose
    /// fill is `FillSpec::None` — PowerPoint's "no line", distinct from an absent `a:ln`.
    #[wasm_bindgen(js_name = "setShapeNoOutline")]
    pub fn set_shape_no_outline(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_shape_no_outline(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The **explicit** effects of shape `shape_idx` on `surface` — its `p:spPr > a:effectLst` as
    /// an interner-free `EffectListSpec` — or `None` when the shape declares no `a:effectLst` (its
    /// effects are then inherited; effective effect resolution is a later step). A shape whose
    /// effects use the rarer `a:effectDag` alternative also reads as `None` (that opaque graph is
    /// not modeled). Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shapeEffects")]
    pub fn shape_effects(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<EffectListSpec>, JsValue> {
        map_error(
            self.inner
                .shape_effects(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(EffectListSpec))
    }

    /// Sets the effects of shape `shape_idx` on `surface` from an interner-free `EffectListSpec`,
    /// rebuilding the `p:spPr` `a:effectLst` element (replacing an existing effect container in
    /// place — either an `a:effectLst` or the mutually-exclusive `a:effectDag`, which is
    /// overwritten — or inserting a new one after any geometry, fill, and outline, before the 3-D
    /// and extension children). Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeEffects")]
    pub fn set_shape_effects(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        effects: &EffectListSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_effects(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &effects.0,
        ))
    }

    /// Sets shape `shape_idx` on `surface` to explicit "no effects" (an empty `<a:effectLst/>`). A
    /// shorthand for `set_shape_effects` with an empty `EffectListSpec` — the explicitly-cleared
    /// effect state that overrides inheritance, distinct from an absent `a:effectLst`. Reads back
    /// as `Some(EffectListSpec::default())`.
    #[wasm_bindgen(js_name = "setShapeNoEffects")]
    pub fn set_shape_no_effects(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_shape_no_effects(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The **explicit** 3-D scene of shape `shape_idx` on `surface` — its `p:spPr > a:scene3d`
    /// (`CT_Scene3D`) as an interner-free `Scene3DSpec` — or `None` when the shape declares no
    /// `a:scene3d`. 3-D has no inheritance chain, so an absent scene means the shape is flat, not
    /// that it inherits one. A scene present but missing a schema-required part (its `a:camera` or
    /// `a:lightRig`) also reads as `None`. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shapeScene3d")]
    pub fn shape_scene_3d(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Scene3DSpec>, JsValue> {
        map_error(
            self.inner
                .shape_scene_3d(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(Scene3DSpec))
    }

    /// Sets the 3-D scene of shape `shape_idx` on `surface` from an interner-free `Scene3DSpec`,
    /// rebuilding the `p:spPr` `a:scene3d` (replacing an existing one in place, or inserting a new
    /// one after any geometry, fill, outline, and effects, before `a:sp3d`). Rebuilding from a spec
    /// drops any opaque scene internals (`a:backdrop`, `extLst`). Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeScene3d")]
    pub fn set_shape_scene_3d(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        scene: &Scene3DSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_scene_3d(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &scene.0,
        ))
    }

    /// Clears the 3-D scene of shape `shape_idx` on `surface` by **removing** its `a:scene3d`
    /// entirely — a shape without a scene is flat. Unlike effects, there is no "explicitly empty"
    /// scene: `CT_Scene3D` requires a camera and light rig, and 3-D does not inherit, so clearing
    /// removes rather than empties. A no-op (still `Ok`) when the shape has no scene. Marks the
    /// part dirty only if it removed something.
    #[wasm_bindgen(js_name = "clearShapeScene3d")]
    pub fn clear_shape_scene_3d(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .clear_shape_scene_3d(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The **explicit** 3-D properties of shape `shape_idx` on `surface` — its `p:spPr > a:sp3d`
    /// (`CT_Shape3D`: extrusion, contour, bevels, material) as an interner-free `Shape3DSpec` — or
    /// `None` when the shape declares no `a:sp3d`. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shape3dProperties")]
    pub fn shape_3d_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Shape3DSpec>, JsValue> {
        map_error(
            self.inner
                .shape_3d_properties(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(Shape3DSpec))
    }

    /// Sets the 3-D properties of shape `shape_idx` on `surface` from an interner-free
    /// `Shape3DSpec`, rebuilding the `p:spPr` `a:sp3d` (replacing an existing one in place, or
    /// inserting a new one after every other visual property, before any `a:extLst`). Rebuilding
    /// from a spec drops any opaque `extLst`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShape3dProperties")]
    pub fn set_shape_3d_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        properties: &Shape3DSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_3d_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &properties.0,
        ))
    }

    /// Clears the 3-D properties of shape `shape_idx` on `surface` by **removing** its `a:sp3d`
    /// entirely. A no-op (still `Ok`) when the shape has none. Marks the part dirty only if it
    /// removed something.
    #[wasm_bindgen(js_name = "clearShape3dProperties")]
    pub fn clear_shape_3d_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .clear_shape_3d_properties(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The position and size of shape `shape_idx` on `surface` **on the slide** — absolute within
    /// `slide_size`, whether the shape is top-level or nested inside groups.
    #[wasm_bindgen(js_name = "shapeBounds")]
    pub fn shape_bounds(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<ShapeBounds>, JsValue> {
        map_error(
            self.inner
                .shape_bounds(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(ShapeBounds))
    }

    /// Moves and resizes shape `shape_idx` on `surface` to `bounds`, given **on the slide** — the
    /// same absolute space `shape_bounds` answers in. Creates the shape's transform element if it
    /// had none, and marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeBounds")]
    pub fn set_shape_bounds(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        bounds: &ShapeBounds,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_shape_bounds(surface_of(surface)?, path_of(shape_idx)?, bounds.0),
        )
    }

    /// The **explicit** transform of shape `shape_idx` on `surface` — its position, size, rotation
    /// and mirror flags, plus the child coordinate space if it is a group — or `None` when the
    /// shape declares no transform at all.
    #[wasm_bindgen(js_name = "shapeTransform")]
    pub fn shape_transform(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Transform2D>, JsValue> {
        map_error(
            self.inner
                .shape_transform(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(Transform2D))
    }

    /// Applies `transform` to shape `shape_idx` on `surface`, creating its transform element if it
    /// had none. Marks only that part dirty; everything else re-emits verbatim.
    #[wasm_bindgen(js_name = "setShapeTransform")]
    pub fn set_shape_transform(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        transform: &Transform2D,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_transform(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &transform.0,
        ))
    }

    /// The geometry of shape `shape_idx` on `surface`, as a `Geometry` — a preset shape
    /// (`Geometry::Preset`), a custom path list (`Geometry::Custom`), or `Geometry::Inherited` when
    /// the shape states no geometry of its own (it takes one from its placeholder / layout).
    /// Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shapeGeometry")]
    pub fn shape_geometry(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Geometry, JsValue> {
        map_error(
            self.inner
                .shape_geometry(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(Geometry)
    }

    /// Every adjustment of shape `shape_idx`'s **preset** geometry, resolved against a concrete
    /// shape size: each value *and* the numeric domain it may move in.
    #[wasm_bindgen(js_name = "shapeAdjustments")]
    pub fn shape_adjustments(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        size: &GuideContext,
    ) -> Result<Vec<BoundedAdjustment>, JsValue> {
        map_error(
            self.inner
                .shape_adjustments(surface_of(surface)?, path_of(shape_idx)?, size.0),
        )
        .map(|values| values.into_iter().map(BoundedAdjustment).collect())
    }

    /// Sets the geometry of shape `shape_idx` on `surface` from a `Geometry`: a preset shape
    /// (`Geometry::Preset`) rewrites the `a:prstGeom`, a custom path list (`Geometry::Custom`)
    /// writes an `a:custGeom`, and `Geometry::Inherited` removes the shape's own geometry so an
    /// inherited one takes over. The two kinds are mutually exclusive, so setting one drops the
    /// other. Marks only that slide part dirty; everything else re-emits verbatim.
    #[wasm_bindgen(js_name = "setShapeGeometry")]
    pub fn set_shape_geometry(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        geometry: &Geometry,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_geometry(
            surface_of(surface)?,
            path_of(shape_idx)?,
            geometry.0.clone(),
        ))
    }

    /// The text of the cell at `(row, column)` — its paragraphs joined by newlines.
    #[wasm_bindgen(js_name = "cellText")]
    pub fn cell_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<String, JsValue> {
        map_error(
            self.inner
                .cell_text(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )
    }

    /// The text that actually **renders** at `(row, column)` — the text of the cell if it stands
    /// alone, or of the merge **anchor** covering it if it is merged away.
    #[wasm_bindgen(js_name = "visibleCellText")]
    pub fn visible_cell_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<String, JsValue> {
        map_error(self.inner.visible_cell_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
        ))
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the cell's paragraphs) of the cell
    /// at `(row, column)`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setCellText")]
    pub fn set_cell_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        run_idx: u32,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            run_idx,
            text,
        ))
    }

    /// The number of paragraphs in the cell at `(row, column)`.
    #[wasm_bindgen(js_name = "cellParagraphCount")]
    pub fn cell_paragraph_count(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.cell_paragraph_count(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
        ))
    }

    /// The number of runs in one paragraph of the cell at `(row, column)`.
    #[wasm_bindgen(js_name = "cellRunCount")]
    pub fn cell_run_count(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.cell_run_count(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
        ))
    }

    /// The text of one paragraph of the cell at `(row, column)`.
    #[wasm_bindgen(js_name = "cellParagraphText")]
    pub fn cell_paragraph_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<String, JsValue> {
        map_error(self.inner.cell_paragraph_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
        ))
    }

    /// The text of one run of the cell at `(row, column)`.
    #[wasm_bindgen(js_name = "cellRunText")]
    pub fn cell_run_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<String, JsValue> {
        map_error(self.inner.cell_run_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            run_idx,
        ))
    }

    /// The layout properties a paragraph of the cell at `(row, column)` declares of its own.
    #[wasm_bindgen(js_name = "cellParagraphProperties")]
    pub fn cell_paragraph_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<Option<ParagraphPropertiesSpec>, JsValue> {
        map_error(self.inner.cell_paragraph_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
        ))
        .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// The character properties a run of the cell at `(row, column)` declares of its own.
    #[wasm_bindgen(js_name = "cellRunProperties")]
    pub fn cell_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, JsValue> {
        map_error(self.inner.cell_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            run_idx,
        ))
        .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// The paragraph-mark properties (`a:endParaRPr`) of a paragraph of the cell at `(row, column)`
    /// — the format an empty cell holds, and what text typed into it would take on.
    #[wasm_bindgen(js_name = "cellEndRunProperties")]
    pub fn cell_end_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, JsValue> {
        map_error(self.inner.cell_end_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
        ))
        .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// Applies `spec` to one run of one paragraph of the cell at `(row, column)`.
    #[expect(
        clippy::too_many_arguments,
        reason = "the coordinates the delegated method takes, restated one for one"
    )]
    #[wasm_bindgen(js_name = "setCellRunProperties")]
    pub fn set_cell_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            run_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to **every run** of one paragraph of the cell at `(row, column)`, and to its
    /// paragraph mark.
    #[wasm_bindgen(js_name = "setCellParagraphRunProperties")]
    pub fn set_cell_paragraph_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_paragraph_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to **every run of every paragraph** of the cell at `(row, column)` — what
    /// selecting a whole cell and restyling it means, and the usual way to make a header bold.
    #[wasm_bindgen(js_name = "setCellRunPropertiesAll")]
    pub fn set_cell_run_properties_all(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_run_properties_all(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            &spec.0,
        ))
    }

    /// Applies `spec` to a paragraph mark (`a:endParaRPr`) of the cell at `(row, column)`, creating
    /// the element if the paragraph has none — how an **empty** cell is formatted.
    #[wasm_bindgen(js_name = "setCellEndRunProperties")]
    pub fn set_cell_end_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_end_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`) in the cell at `(row, column)`,
    /// creating the element if it has none. The properties **merge**, as run properties do.
    #[wasm_bindgen(js_name = "setCellParagraphProperties")]
    pub fn set_cell_paragraph_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_paragraph_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to part of a paragraph of the cell at `(row, column)` — the characters in
    /// `range`, counted in Unicode scalars. Splits runs at the range's edges, exactly as the shape-
    /// addressed form does.
    #[expect(
        clippy::too_many_arguments,
        reason = "the coordinates the delegated method takes, restated one for one"
    )]
    #[wasm_bindgen(js_name = "setCellTextRangeProperties")]
    pub fn set_cell_text_range_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        range_start: u32,
        range_end: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_text_range_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            range_start..range_end,
            &spec.0,
        ))
    }

    /// The fill the cell at `(row, column)` declares, or `None` when it declares none — in which
    /// case the table style decides. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "cellFill")]
    pub fn cell_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<Option<FillSpec>, JsValue> {
        map_error(
            self.inner
                .cell_fill(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )
        .map(|value| value.map(FillSpec))
    }

    /// Fills the cell at `(row, column)`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setCellFill")]
    pub fn set_cell_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        fill: &FillSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_fill(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            &fill.0,
        ))
    }

    /// Removes the cell's own fill, so the table style decides how it is filled again.
    #[wasm_bindgen(js_name = "clearCellFill")]
    pub fn clear_cell_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<(), JsValue> {
        map_error(self.inner.clear_cell_fill(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
        ))
    }

    /// The border the cell at `(row, column)` declares on `edge`, or `None` if it declares none
    /// there. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "cellBorder")]
    pub fn cell_border(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, JsValue> {
        map_error(self.inner.cell_border(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            edge.into(),
        ))
        .map(|value| value.map(LineSpec))
    }

    /// Draws a border on one edge of the cell at `(row, column)`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setCellBorder")]
    pub fn set_cell_border(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
        line: &LineSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_border(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            edge.into(),
            &line.0,
        ))
    }

    /// The ids of the header cells that describe the cell at `(row, column)` (`a:tcPr >
    /// a:headers`), in order — the accessibility association a screen reader announces. Empty when
    /// the cell names none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "cellHeaders")]
    pub fn cell_headers(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<Vec<String>, JsValue> {
        map_error(
            self.inner
                .cell_headers(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )
    }

    /// Sets the header-cell ids that describe the cell at `(row, column)`, replacing whatever it
    /// had; an empty slice removes the association. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setCellHeaders")]
    pub fn set_cell_headers(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        header_ids: Vec<String>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_headers(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            &str_list(&header_ids),
        ))
    }

    /// Removes the border on one edge of the cell at `(row, column)`.
    #[wasm_bindgen(js_name = "clearCellBorder")]
    pub fn clear_cell_border(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> Result<(), JsValue> {
        map_error(self.inner.clear_cell_border(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            edge.into(),
        ))
    }

    /// The four insets between the cell's edges and its text, each `None` when the cell does not
    /// state it. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "cellMargins")]
    pub fn cell_margins(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<CellMargins, JsValue> {
        map_error(
            self.inner
                .cell_margins(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )
        .map(CellMargins)
    }

    /// Sets the cell's insets. Each field left `None` is **not written**, so a caller can set one
    /// margin without stating the other three.
    #[wasm_bindgen(js_name = "setCellMargins")]
    pub fn set_cell_margins(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        margins: &CellMargins,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_margins(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            margins.0,
        ))
    }

    /// Where the text sits vertically in the cell at `(row, column)`, or `None` if unstated (the
    /// wire default is `TextAnchoring::Top`). Reading does not dirty the part.
    #[wasm_bindgen(js_name = "cellAnchor")]
    pub fn cell_anchor(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<Option<TextAnchoring>, JsValue> {
        map_error(
            self.inner
                .cell_anchor(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )?
        .map(TextAnchoring::from_model)
        .transpose()
    }

    /// Sets where the text sits vertically in the cell at `(row, column)`.
    #[wasm_bindgen(js_name = "setCellAnchor")]
    pub fn set_cell_anchor(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        anchor: TextAnchoring,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_anchor(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            anchor.into(),
        ))
    }

    /// Which way the text flows in the cell at `(row, column)`, or `None` if unstated (the wire
    /// default is `TextDirection::Horizontal`). Reading does not dirty the part.
    #[wasm_bindgen(js_name = "cellTextDirection")]
    pub fn cell_text_direction(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<Option<TextDirection>, JsValue> {
        map_error(self.inner.cell_text_direction(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
        ))?
        .map(TextDirection::from_model)
        .transpose()
    }

    /// Sets which way the text flows in the cell at `(row, column)` — how a rotated header row is
    /// made.
    #[wasm_bindgen(js_name = "setCellTextDirection")]
    pub fn set_cell_text_direction(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        direction: TextDirection,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_cell_text_direction(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            direction.into(),
        ))
    }

    /// Applies `format` to every cell in `cells`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "formatCells")]
    pub fn format_cells(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        cells: &Cells,
        format: &CellFormat,
    ) -> Result<(), JsValue> {
        map_error(self.inner.format_cells(
            surface_of(surface)?,
            path_of(shape_idx)?,
            cells.0.clone(),
            &format.0,
        ))
    }

    /// Applies `spec` to **every run of every paragraph** in each cell of `cells`, and to each
    /// paragraph's mark — bolding a header row in one call.
    #[wasm_bindgen(js_name = "formatCellText")]
    pub fn format_cell_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        cells: &Cells,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.format_cell_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            cells.0.clone(),
            &spec.0,
        ))
    }

    /// Applies `spec` to the layout properties of **every paragraph** in each cell of `cells` —
    /// right- aligning a column of numbers in one call.
    #[wasm_bindgen(js_name = "formatCellParagraphs")]
    pub fn format_cell_paragraphs(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        cells: &Cells,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.format_cell_paragraphs(
            surface_of(surface)?,
            path_of(shape_idx)?,
            cells.0.clone(),
            &spec.0,
        ))
    }

    /// Merges `cells` into one region. Marks only that part dirty.
    #[wasm_bindgen(js_name = "mergeCells")]
    pub fn merge_cells(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        cells: &Cells,
    ) -> Result<(), JsValue> {
        map_error(self.inner.merge_cells(
            surface_of(surface)?,
            path_of(shape_idx)?,
            cells.0.clone(),
        ))
    }

    /// Undoes the merge covering the cell at `(row, column)`, whichever cell of the region is
    /// named. Marks only that part dirty.
    #[wasm_bindgen(js_name = "unmergeCells")]
    pub fn unmerge_cells(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .unmerge_cells(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )
    }

    /// The fill of series `series_idx` of the chart the frame `shape_idx` on `surface` references —
    /// what colour it is drawn in — or `None` when the series declares none and takes its colour
    /// from the chart style. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartSeriesFill")]
    pub fn chart_series_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<Option<FillSpec>, JsValue> {
        map_error(self.inner.chart_series_fill(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
        .map(|value| value.map(FillSpec))
    }

    /// Sets the fill of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references, creating its `c:spPr` if it had none. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartSeriesFill")]
    pub fn set_chart_series_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_series_fill(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            &fill.0,
        ))
    }

    /// Sets the outline of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references — the line a line or radar plot draws, or the border of a bar or area. Marks only
    /// the chart part dirty.
    #[wasm_bindgen(js_name = "setChartSeriesLine")]
    pub fn set_chart_series_line(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        line: &LineSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_series_line(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            &line.0,
        ))
    }

    /// The data-label settings **in force** for one point of series `series_idx` of the chart the
    /// frame `shape_idx` on `surface` references — the point's `c:dLbl` merged over the series'
    /// `c:dLbls` merged over the owning plot's.
    #[wasm_bindgen(js_name = "chartDataLabels")]
    pub fn chart_data_labels(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, JsValue> {
        map_error(self.inner.chart_data_labels(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            point_idx,
        ))
        .map(DataLabelSettings)
    }

    /// The data-label settings one **tier** states in its own right — what that tier contributes to
    /// the merge, with everything it leaves unset reported as `None`.
    #[wasm_bindgen(js_name = "chartDataLabelTier")]
    pub fn chart_data_label_tier(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        scope: &ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, JsValue> {
        map_error(self.inner.chart_data_label_tier(
            surface_of(surface)?,
            path_of(shape_idx)?,
            scope.0,
        ))
        .map(|value| value.map(DataLabelSettings))
    }

    /// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None` when it
    /// states none and shows what the settings say. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartPointLabelText")]
    pub fn chart_point_label_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.chart_point_label_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            point_idx,
        ))
    }

    /// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had
    /// none and leaving every setting `spec` does not state alone. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartDataLabels")]
    pub fn set_chart_data_labels(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        scope: &ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_data_labels(
            surface_of(surface)?,
            path_of(shape_idx)?,
            scope.0,
            &spec.0,
        ))
    }

    /// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which is
    /// how one series of a labelled plot, or one point of a labelled series, is silenced without
    /// disturbing the rest. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "suppressChartDataLabels")]
    pub fn suppress_chart_data_labels(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        scope: &ChartLabelScope,
    ) -> Result<(), JsValue> {
        map_error(self.inner.suppress_chart_data_labels(
            surface_of(surface)?,
            path_of(shape_idx)?,
            scope.0,
        ))
    }

    /// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
    /// again. Answers whether an element was there. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "removeChartDataLabels")]
    pub fn remove_chart_data_labels(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        scope: &ChartLabelScope,
    ) -> Result<bool, JsValue> {
        map_error(self.inner.remove_chart_data_labels(
            surface_of(surface)?,
            path_of(shape_idx)?,
            scope.0,
        ))
    }

    /// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document
    /// order. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartPointFormats")]
    pub fn chart_point_formats(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<Vec<ChartPointFormatData>, JsValue> {
        map_error(self.inner.chart_point_formats(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
        .map(|values| values.into_iter().map(ChartPointFormatData).collect())
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series,
    /// creating its `c:dPt` at the schema rank if it had none. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartPointFill")]
    pub fn set_chart_point_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_point_fill(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            point_idx,
            &fill.0,
        ))
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
    /// Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartPointLine")]
    pub fn set_chart_point_line(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_point_line(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            point_idx,
            &line.0,
        ))
    }

    /// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
    /// `percent` of the radius (`c:explosion`), or (for `None`) puts it back. Marks only the chart
    /// part dirty.
    #[wasm_bindgen(js_name = "setChartPointExplosion")]
    pub fn set_chart_point_explosion(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_point_explosion(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            point_idx,
            percent,
        ))
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like the
    /// rest of its series. Answers whether any was there. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "removeChartPointFormat")]
    pub fn remove_chart_point_format(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<bool, JsValue> {
        map_error(self.inner.remove_chart_point_format(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            point_idx,
        ))
    }

    /// Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
    /// Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartTrendlines")]
    pub fn chart_trendlines(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<Vec<ChartTrendlineData>, JsValue> {
        map_error(self.inner.chart_trendlines(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
        .map(|values| values.into_iter().map(ChartTrendlineData).collect())
    }

    /// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends** — a
    /// series may carry a linear fit and a moving average at once. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "addChartTrendline")]
    pub fn add_chart_trendline(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.add_chart_trendline(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            &spec.0,
        ))
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** — the
    /// curve keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional setting
    /// `spec` leaves unset is cleared. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartTrendline")]
    pub fn set_chart_trendline(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        trendline_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_trendline(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            trendline_idx,
            &spec.0,
        ))
    }

    /// Removes every trendline from series `series_idx`, answering how many went. Marks only the
    /// chart part dirty.
    #[wasm_bindgen(js_name = "removeChartTrendlines")]
    pub fn remove_chart_trendlines(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.remove_chart_trendlines(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
    }

    /// Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or line
    /// series, up to two (x and y) for scatter, area and bubble. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartErrorBars")]
    pub fn chart_error_bars(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<Vec<ChartErrorBarData>, JsValue> {
        map_error(self.inner.chart_error_bars(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
        .map(|values| values.into_iter().map(ChartErrorBarData).collect())
    }

    /// Gives series `series_idx` error bars, replacing an existing set that runs along the same
    /// axis. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartErrorBars")]
    pub fn set_chart_error_bars(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        spec: &ErrorBarSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_error_bars(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            &spec.0,
        ))
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went. Marks
    /// only the chart part dirty.
    #[wasm_bindgen(js_name = "removeChartErrorBars")]
    pub fn remove_chart_error_bars(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.remove_chart_error_bars(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no
    /// longer has. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartDanglingDecoration")]
    pub fn chart_dangling_decoration(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<Vec<DanglingPointReference>, JsValue> {
        map_error(self.inner.chart_dangling_decoration(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
        .map(|values| values.into_iter().map(DanglingPointReference).collect())
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the end of
    /// its data, answering how many went. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "dropChartDanglingDecoration")]
    pub fn drop_chart_dangling_decoration(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.drop_chart_dangling_decoration(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
        ))
    }

    /// Adds `chart` to `surface` as a new chart, laid out inside `bounds`, and returns its index in
    /// the shape tree.
    #[wasm_bindgen(js_name = "addChart")]
    pub fn add_chart(
        &mut self,
        surface: &SurfaceArg,
        chart: &ChartData,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_chart(surface_of(surface)?, &chart.0, bounds.0),
        )
    }

    /// The raw XML bytes of the chart part the chart frame `shape_idx` on `surface` references
    /// (`/ppt/charts/chartN.xml`), exactly as the package holds them, or `None` when the shape
    /// frames no chart. Borrowed from the package, so the part is not copied.
    #[wasm_bindgen(js_name = "chartPartBytes")]
    pub fn chart_part_bytes(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .chart_part_bytes(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Every chart on `surface` that references a backing workbook (`c:externalData`), with where
    /// each is referenced from and whether that reference is external.
    #[wasm_bindgen(js_name = "chartWorkbooks")]
    pub fn chart_workbooks(&mut self, surface: &SurfaceArg) -> Result<Vec<ChartWorkbook>, JsValue> {
        map_error(self.inner.chart_workbooks(surface_of(surface)?))
            .map(|values| values.into_iter().map(ChartWorkbook).collect())
    }

    /// Detaches the backing workbook from the chart `shape_idx` on `surface`: removes its
    /// `c:externalData` reference — the element and its relationship — leaving the chart to render
    /// from its cached values. This neutralizes a chart that links an unreachable external workbook
    /// (the caller decides accessibility; use `chart_workbooks` to find the candidates), and yields
    /// exactly the cache-only shape a freshly authored chart has.
    #[wasm_bindgen(js_name = "detachChartWorkbook")]
    pub fn detach_chart_workbook(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .detach_chart_workbook(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The series of the chart the frame `shape_idx` on `surface` references — for each, its name,
    /// category labels and values (for a scatter series, its X labels and Y values), flattened
    /// across the chart's plots. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartSeries")]
    pub fn chart_series(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Vec<ChartSeriesData>, JsValue> {
        map_error(
            self.inner
                .chart_series(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|values| values.into_iter().map(ChartSeriesData).collect())
    }

    /// Rewrites the values of series `series_idx` (0-based across the chart's plots) of the chart
    /// the frame `shape_idx` on `surface` references — whichever source the series names: a
    /// `c:numRef`'s cache or a `c:numLit`.
    #[wasm_bindgen(js_name = "setChartSeriesValues")]
    pub fn set_chart_series_values(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        values: &[f64],
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_series_values(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            values,
        ))
    }

    /// Rewrites the category labels of series `series_idx` (0-based across the chart's plots) of
    /// the chart the frame `shape_idx` on `surface` references, and refreshes the chart's embedded
    /// workbook alongside it.
    #[wasm_bindgen(js_name = "setChartSeriesCategories")]
    pub fn set_chart_series_categories(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        series_idx: u32,
        labels: Vec<String>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_series_categories(
            surface_of(surface)?,
            path_of(shape_idx)?,
            series_idx,
            &str_list(&labels),
        ))
    }

    /// Rewrites the embedded workbook of the chart the frame `shape_idx` on `surface` references so
    /// its cells hold exactly what the chart now draws, and answers whether it rewrote one.
    #[wasm_bindgen(js_name = "refreshChartWorkbook")]
    pub fn refresh_chart_workbook(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<bool, JsValue> {
        map_error(
            self.inner
                .refresh_chart_workbook(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The kind of every plot the chart the frame `shape_idx` on `surface` references draws, in
    /// document order — one entry per plot element, so a combo chart yields several. Reading does
    /// not dirty the part.
    #[wasm_bindgen(js_name = "chartKinds")]
    pub fn chart_kinds(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Vec<ChartKind>, JsValue> {
        map_error(
            self.inner
                .chart_kinds(surface_of(surface)?, path_of(shape_idx)?),
        )?
        .into_iter()
        .map(ChartKind::from_model)
        .collect()
    }

    /// The axes of the chart the frame `shape_idx` on `surface` references, in document order.
    /// Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartAxes")]
    pub fn chart_axes(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Vec<ChartAxisData>, JsValue> {
        map_error(
            self.inner
                .chart_axes(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|values| values.into_iter().map(ChartAxisData).collect())
    }

    /// Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order) of the chart
    /// the frame `shape_idx` on `surface` references. `None` returns that end of the axis to
    /// automatic scaling. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartAxisScale")]
    pub fn set_chart_axis_scale(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        axis_idx: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_axis_scale(
            surface_of(surface)?,
            path_of(shape_idx)?,
            axis_idx,
            minimum,
            maximum,
        ))
    }

    /// Sets the direction of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references — smallest value first, or reversed. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartAxisOrientation")]
    pub fn set_chart_axis_orientation(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        axis_idx: u32,
        orientation: AxisOrientation,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_axis_orientation(
            surface_of(surface)?,
            path_of(shape_idx)?,
            axis_idx,
            orientation.into(),
        ))
    }

    /// Sets or removes the title of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references. `None` removes the title. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartAxisTitle")]
    pub fn set_chart_axis_title(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        axis_idx: u32,
        text: Option<String>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_axis_title(
            surface_of(surface)?,
            path_of(shape_idx)?,
            axis_idx,
            text.as_deref(),
        ))
    }

    /// Turns the gridlines of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references on or off. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartAxisGridlines")]
    pub fn set_chart_axis_gridlines(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        axis_idx: u32,
        major: bool,
        minor: bool,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_axis_gridlines(
            surface_of(surface)?,
            path_of(shape_idx)?,
            axis_idx,
            major,
            minor,
        ))
    }

    /// The heading of the chart the frame `shape_idx` on `surface` references (`c:title`), or
    /// `None` when it has none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartTitle")]
    pub fn chart_title(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .chart_title(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Sets or removes the heading of the chart the frame `shape_idx` on `surface` references.
    /// `None` removes it. Marks only the chart part dirty.
    #[wasm_bindgen(js_name = "setChartTitle")]
    pub fn set_chart_title(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        text: Option<String>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_title(
            surface_of(surface)?,
            path_of(shape_idx)?,
            text.as_deref(),
        ))
    }

    /// The legend of the chart the frame `shape_idx` on `surface` references, or `None` when it has
    /// none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartLegend")]
    pub fn chart_legend(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<ChartLegendData>, JsValue> {
        map_error(
            self.inner
                .chart_legend(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(ChartLegendData))
    }

    /// Places the legend of the chart the frame `shape_idx` on `surface` references at `position`,
    /// adding one if the chart had none. `None` removes the legend. Marks only the chart part
    /// dirty.
    #[wasm_bindgen(js_name = "setChartLegend")]
    pub fn set_chart_legend(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        position: Option<LegendPosition>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_chart_legend(
            surface_of(surface)?,
            path_of(shape_idx)?,
            position.map(Into::into),
        ))
    }

    /// The built-in style id the chart the frame `shape_idx` on `surface` references names
    /// (`c:style@val`, 1 to 48) — the palette and effect set Office draws an unstyled series with —
    /// or `None` when it names none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "chartStyleId")]
    pub fn chart_style_id(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<u32>, JsValue> {
        map_error(
            self.inner
                .chart_style_id(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The number of slides, in presentation order.
    #[wasm_bindgen(js_name = "slideCount")]
    pub fn slide_count(&self) -> u32 {
        self.inner.slide_count()
    }

    /// The number of slide masters, in `p:sldMasterIdLst` order.
    #[wasm_bindgen(js_name = "masterCount")]
    pub fn master_count(&self) -> u32 {
        self.inner.master_count()
    }

    /// The name of master `idx` (`p:cSld@name`, e.g. `Office Theme`), or `None` if it is unnamed.
    #[wasm_bindgen(js_name = "masterName")]
    pub fn master_name(&mut self, idx: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.master_name(idx))
    }

    /// Every slide layout the deck offers, in layout-index order — the inventory a caller reads
    /// before choosing one to build a slide on.
    #[wasm_bindgen(js_name = "layouts")]
    pub fn layouts(&mut self) -> Result<Vec<LayoutInfo>, JsValue> {
        map_error(self.inner.layouts()).map(|values| values.into_iter().map(LayoutInfo).collect())
    }

    /// The number of slide layouts across the whole deck, in (master order, `p:sldLayoutIdLst`
    /// order) — so layout indices run master by master. `layout_master` says which master an index
    /// belongs to.
    #[wasm_bindgen(js_name = "layoutCount")]
    pub fn layout_count(&self) -> u32 {
        self.inner.layout_count()
    }

    /// The index of the master that lists layout `idx`.
    #[wasm_bindgen(js_name = "layoutMaster")]
    pub fn layout_master(&self, idx: u32) -> Option<u32> {
        self.inner.layout_master(idx)
    }

    /// The name of layout `idx` (`p:cSld@name`, e.g. `Title and Content` — the name PowerPoint
    /// shows in its layout gallery), or `None` if it is unnamed.
    #[wasm_bindgen(js_name = "layoutName")]
    pub fn layout_name(&mut self, idx: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.layout_name(idx))
    }

    /// How layout `idx` arranges its content (`p:sldLayout@type`) — a coarse description of which
    /// placeholders it offers, which an application can use to map between layouts.
    #[wasm_bindgen(js_name = "layoutKind")]
    pub fn layout_kind(&mut self, idx: u32) -> Result<SlideLayoutKind, JsValue> {
        SlideLayoutKind::from_model(map_error(self.inner.layout_kind(idx))?)
    }

    /// The index of the layout slide `slide_idx` is built on, or `None` if the slide relates to no
    /// layout (or to one no master lists).
    #[wasm_bindgen(js_name = "slideLayout")]
    pub fn slide_layout(&self, slide_idx: u32) -> Result<Option<u32>, JsValue> {
        map_error(self.inner.slide_layout(slide_idx))
    }

    /// The size of every slide in the deck (`p:sldSz`) — the extent shape bounds are laid out in.
    #[wasm_bindgen(js_name = "slideSize")]
    pub fn slide_size(&mut self) -> Result<SlideSize, JsValue> {
        map_error(self.inner.slide_size()).map(SlideSize)
    }

    /// The theme that governs `surface`, as an interner-free `ThemeInfo` (its color scheme + fill-
    /// style matrix) — the theme related to the last part of the surface's inheritance chain (slide
    /// → slideLayout → slideMaster → theme, and the shorter walks from a layout or master). Returns
    /// `Ok(None)` if any hop is absent (a deck without a theme). Reading does not dirty any part.
    #[wasm_bindgen(js_name = "theme")]
    pub fn theme(&mut self, surface: &SurfaceArg) -> Result<Option<ThemeInfo>, JsValue> {
        map_error(self.inner.theme(surface_of(surface)?)).map(|value| value.map(ThemeInfo))
    }

    /// The effective theme `ColorMap` for `surface`: the master's `p:clrMap` (reached along the
    /// surface's inheritance chain), replaced by the surface's own `p:clrMapOvr >
    /// a:overrideClrMapping` when it supplies a full mapping (a `masterClrMapping`, an absent
    /// override, or a schema-loose attribute-less override all inherit the master's map). It maps
    /// the logical color names a shape may reference (`bg1`/`tx1`/…) to the theme's concrete scheme
    /// slots. `Ok(None)` when there is no reachable master or no `p:clrMap`. Reading does not dirty
    /// a part.
    #[wasm_bindgen(js_name = "colorMap")]
    pub fn color_map(&mut self, surface: &SurfaceArg) -> Result<Option<ColorMap>, JsValue> {
        map_error(self.inner.color_map(surface_of(surface)?)).map(|value| value.map(ColorMap))
    }

    /// The **effective** fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`
    /// whose colors are resolved to concrete `RRGGBB` values — the fill the shape actually renders.
    /// Three sources are tried, in order: an explicit `p:spPr` fill; a `p:style > a:fillRef` (the
    /// theme fill- style at that index, `phClr` substituted by the reference's color); and, for a
    /// placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the layout
    /// then the master. Scheme colors and color transforms are baked against the surface's theme +
    /// map.
    #[wasm_bindgen(js_name = "effectiveShapeFill")]
    pub fn effective_shape_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<FillSpec>, JsValue> {
        map_error(
            self.inner
                .effective_shape_fill(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(FillSpec))
    }

    /// The **effective** outline of shape `shape_idx` on `surface`, as an interner-free `LineSpec`
    /// whose stroke color is resolved to a concrete `RRGGBB` value — the outline the shape actually
    /// renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`; a `p:style >
    /// a:lnRef` (the theme line-style at that index, `phClr` substituted by the reference's color);
    /// and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the
    /// slide layout then the master. Scheme colors and color transforms are baked against the
    /// slide's theme + map.
    #[wasm_bindgen(js_name = "effectiveShapeOutline")]
    pub fn effective_shape_outline(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<LineSpec>, JsValue> {
        map_error(
            self.inner
                .effective_shape_outline(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(LineSpec))
    }

    /// The **effective** effects of shape `shape_idx` on `surface`, as an interner-free
    /// `EffectListSpec` whose colors are resolved to concrete `RRGGBB` values — the effects the
    /// shape actually renders. Three sources are tried, in order: an explicit `p:spPr >
    /// a:effectLst`; a `p:style > a:effectRef` (the theme effect-style at that index, `phClr`
    /// substituted by the reference's color); and, for a placeholder shape (`p:ph`),
    /// **inheritance** from the same-slot placeholder on the slide layout then the master. Scheme
    /// colors and color transforms are baked against the slide's theme + map.
    #[wasm_bindgen(js_name = "effectiveShapeEffects")]
    pub fn effective_shape_effects(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<EffectListSpec>, JsValue> {
        map_error(
            self.inner
                .effective_shape_effects(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(EffectListSpec))
    }

    /// The **effective** transform of shape `shape_idx` on `surface` — where the shape actually
    /// renders, not what it declares. For a placeholder that places itself nowhere, this is the
    /// same- slot placeholder's transform on the slide layout, and failing that on the master.
    #[wasm_bindgen(js_name = "effectiveShapeTransform")]
    pub fn effective_shape_transform(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Transform2D>, JsValue> {
        map_error(
            self.inner
                .effective_shape_transform(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(Transform2D))
    }

    /// The **effective** position and size of shape `shape_idx` on `surface` — where the shape
    /// actually renders, with the layout and master consulted for a placeholder that declares no
    /// bounds of its own.
    #[wasm_bindgen(js_name = "effectiveShapeBounds")]
    pub fn effective_shape_bounds(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<ShapeBounds>, JsValue> {
        map_error(
            self.inner
                .effective_shape_bounds(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(ShapeBounds))
    }

    /// The **effective** character properties of run `run_idx` — what the run actually renders as,
    /// with every tier of inheritance resolved and its colors baked to concrete `RRGGBB`.
    #[wasm_bindgen(js_name = "effectiveRunProperties")]
    pub fn effective_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<CharacterPropertiesSpec, JsValue> {
        map_error(self.inner.effective_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            run_idx,
        ))
        .map(CharacterPropertiesSpec)
    }

    /// The **effective** paragraph properties of paragraph `para_idx` — the layout it actually
    /// renders with, every tier of inheritance resolved.
    #[wasm_bindgen(js_name = "effectiveParagraphProperties")]
    pub fn effective_paragraph_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<ParagraphPropertiesSpec, JsValue> {
        map_error(self.inner.effective_paragraph_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
        ))
        .map(ParagraphPropertiesSpec)
    }

    /// The **effective** fill of the cell at `(row, column)` of the table shape `shape_idx` frames
    /// — an interner-free `FillSpec` with its colour baked to concrete `RRGGBB`, or `None` if
    /// nothing fills the cell. The cell's own `a:tcPr` fill wins; else the first applicable style
    /// part with a fill (explicit or a theme `fillRef`).
    #[wasm_bindgen(js_name = "effectiveCellFill")]
    pub fn effective_cell_fill(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<Option<FillSpec>, JsValue> {
        map_error(self.inner.effective_cell_fill(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
        ))
        .map(|value| value.map(FillSpec))
    }

    /// The **effective** border on one `edge` of the cell at `(row, column)` — an interner-free
    /// `LineSpec` with its stroke colour baked, or `None`. The cell's own `a:tcPr` edge wins; else
    /// the applicable style parts' `a:tcBdr`, taking the outer edge (`top`/`left`/…) for a cell on
    /// the table's rim and the interior edge (`insideH`/`insideV`) for one within it.
    #[wasm_bindgen(js_name = "effectiveCellBorder")]
    pub fn effective_cell_border(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, JsValue> {
        map_error(self.inner.effective_cell_border(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            edge.into(),
        ))
        .map(|value| value.map(LineSpec))
    }

    /// The **effective** run properties of a cell's text run — the `CharacterPropertiesSpec` it
    /// actually renders with, colours baked. A shorter ladder than a shape's (a cell inherits from
    /// its table style, not a placeholder chain), highest first: the run's own `a:rPr`, the
    /// paragraph's `a:defRPr`, the table style's `a:tcTxStyle` for each applicable part (bold /
    /// italic / colour), then the presentation's `p:defaultTextStyle`.
    #[wasm_bindgen(js_name = "effectiveCellRunProperties")]
    pub fn effective_cell_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<CharacterPropertiesSpec, JsValue> {
        map_error(self.inner.effective_cell_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
            para_idx,
            run_idx,
        ))
        .map(CharacterPropertiesSpec)
    }

    /// Removes every part the package no longer reaches from its root, and reports what was swept.
    #[wasm_bindgen(js_name = "removeUnusedParts")]
    pub fn remove_unused_parts(&mut self) -> Result<Vec<String>, JsValue> {
        map_error(self.inner.remove_unused_parts())
    }

    /// Every relationship in the package whose target lies **outside** it — a linked image, a
    /// chart's external workbook, a linked OLE object or media file — with the part that owns each.
    #[wasm_bindgen(js_name = "externalLinks")]
    pub fn external_links(&self) -> Vec<ExternalLink> {
        self.inner
            .external_links()
            .into_iter()
            .map(ExternalLink)
            .collect()
    }

    /// Repoints the relationship `id` of `source` (`None` = the package root) at `new_target`,
    /// keeping its id and its place in the `.rels`. Returns whether one was found.
    #[wasm_bindgen(js_name = "retargetExternalLink")]
    pub fn retarget_external_link(
        &mut self,
        source: Option<String>,
        id: &str,
        new_target: &str,
        mode: TargetMode,
    ) -> Result<bool, JsValue> {
        map_error(
            self.inner
                .retarget_external_link(source.as_deref(), id, new_target, mode.into()),
        )
    }

    /// The click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` on
    /// `surface`, resolved to a `Hyperlink` (a URL or a slide index), or `None` if the run has no
    /// hyperlink — or one this build does not model (a mouse-over action, a show jump). Reading
    /// does not dirty the part.
    #[wasm_bindgen(js_name = "runHyperlink")]
    pub fn run_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<Option<Hyperlink>, JsValue> {
        map_error(self.inner.run_hyperlink(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            run_idx,
        ))
        .map(|value| value.map(Hyperlink))
    }

    /// Sets the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` to
    /// `link`, adding its relationship. If the run already linked somewhere, that relationship is
    /// removed once nothing else in the part still names it.
    #[wasm_bindgen(js_name = "setRunHyperlink")]
    pub fn set_run_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
        link: &Hyperlink,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_run_hyperlink(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            run_idx,
            &link.0,
        ))
    }

    /// Removes the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx`,
    /// and the relationship it named once nothing else in the part still references it. A no-op if
    /// the run has no hyperlink.
    #[wasm_bindgen(js_name = "clearRunHyperlink")]
    pub fn clear_run_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<(), JsValue> {
        map_error(self.inner.clear_run_hyperlink(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            run_idx,
        ))
    }

    /// Sets the click hyperlink over a **scalar range** of paragraph `para_idx` in shape
    /// `shape_idx`, splitting runs at the boundaries so exactly the selected text is linked (as
    /// `set_text_range_properties` does). One relationship is added and shared by every run in the
    /// range. An empty range links nothing.
    #[wasm_bindgen(js_name = "setTextRangeHyperlink")]
    pub fn set_text_range_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        range_start: u32,
        range_end: u32,
        link: &Hyperlink,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_text_range_hyperlink(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            range_start..range_end,
            &link.0,
        ))
    }

    /// The click hyperlink on shape `shape_idx` itself (`p:cNvPr > a:hlinkClick`), resolved to a
    /// `Hyperlink`, or `None` if the shape has no hyperlink (or one this build does not model).
    /// Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shapeHyperlink")]
    pub fn shape_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Hyperlink>, JsValue> {
        map_error(
            self.inner
                .shape_hyperlink(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(Hyperlink))
    }

    /// Sets the click hyperlink on shape `shape_idx` itself to `link`, adding its relationship and
    /// removing the one any previous link named once unreferenced.
    #[wasm_bindgen(js_name = "setShapeHyperlink")]
    pub fn set_shape_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        link: &Hyperlink,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_hyperlink(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &link.0,
        ))
    }

    /// Removes the click hyperlink on shape `shape_idx` itself, and the relationship it named once
    /// unreferenced. A no-op if the shape has no hyperlink.
    #[wasm_bindgen(js_name = "clearShapeHyperlink")]
    pub fn clear_shape_hyperlink(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .clear_shape_hyperlink(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The raw bytes of the embedded object the OLE frame `shape_idx` on `surface` references
    /// (`/ppt/embeddings/oleObjectN.bin` or an embedded package), exactly as the package holds
    /// them, or `None` when the shape frames no OLE object. Borrowed from the package, so the part
    /// is not copied.
    #[wasm_bindgen(js_name = "oleObjectPartBytes")]
    pub fn ole_object_part_bytes(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .ole_object_part_bytes(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The stored bytes of the OLE fallback snapshot image the frame `shape_idx` on `surface`
    /// embeds, exactly as the package holds them (never decoded or re-encoded), or `None` when the
    /// frame is not an OLE object or carries no snapshot. Borrowed from the package.
    #[wasm_bindgen(js_name = "oleSnapshotImageBytes")]
    pub fn ole_snapshot_image_bytes(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .ole_snapshot_image_bytes(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The `progId` the OLE frame `shape_idx` on `surface` declares (e.g. `"Excel.Sheet.12"`) — the
    /// application that owns the embedded object — or `None` when the shape frames no OLE object or
    /// the attribute is absent. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "oleProgId")]
    pub fn ole_prog_id(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .ole_prog_id(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Every OLE object frame on `surface`, with where its object data is referenced from and
    /// whether that reference is external.
    #[wasm_bindgen(js_name = "oleObjects")]
    pub fn ole_objects(&mut self, surface: &SurfaceArg) -> Result<Vec<OleObject>, JsValue> {
        map_error(self.inner.ole_objects(surface_of(surface)?))
            .map(|values| values.into_iter().map(OleObject).collect())
    }

    /// Replaces the object data of the OLE frame `shape_idx` on `surface` with an in-package
    /// placeholder, so an object that points at unreachable external data resolves inside the
    /// package instead. The placeholder is `placeholder` if given, else `default_placeholder_ole`
    /// (a minimal valid compound file). The `p:oleObj` markup is unchanged — its relationship is
    /// simply retargeted at the placeholder — and the object keeps displaying via its snapshot
    /// image.
    #[wasm_bindgen(js_name = "replaceOleObjectWithPlaceholder")]
    pub fn replace_ole_object_with_placeholder(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        placeholder: Option<Vec<u8>>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.replace_ole_object_with_placeholder(
            surface_of(surface)?,
            path_of(shape_idx)?,
            placeholder.as_deref(),
        ))
    }

    /// The number of legacy **ActiveX** form controls on `surface` (`p:cSld > p:controls >
    /// p:control`).
    #[wasm_bindgen(js_name = "activexControlCount")]
    pub fn activex_control_count(&mut self, surface: &SurfaceArg) -> Result<u32, JsValue> {
        map_error(self.inner.activex_control_count(surface_of(surface)?))
    }

    /// The `name` the ActiveX control `control_idx` on `surface` declares (e.g.
    /// `"CommandButton1"`), or `None` when there is no such control or it is unnamed. Reading does
    /// not dirty the part.
    #[wasm_bindgen(js_name = "activexControlName")]
    pub fn activex_control_name(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .activex_control_name(surface_of(surface)?, control_idx),
        )
    }

    /// The raw bytes of the ActiveX control part (`ax:ocx` markup) the control `control_idx` on
    /// `surface` references, exactly as the package holds them, or `None` when there is no such
    /// control. Borrowed from the package; reading does not dirty anything.
    #[wasm_bindgen(js_name = "activexPartBytes")]
    pub fn activex_part_bytes(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .activex_part_bytes(surface_of(surface)?, control_idx),
        )
    }

    /// The ActiveX control's **persisted state** — the bytes of `/ppt/activeX/activeXN.bin` — for
    /// the control `control_idx` on `surface`, or `None` when there is no such control or it
    /// persists no state. Borrowed from the package; reading does not dirty anything.
    #[wasm_bindgen(js_name = "activexStateBytes")]
    pub fn activex_state_bytes(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .activex_state_bytes(surface_of(surface)?, control_idx),
        )
    }

    /// The stored bytes of the ActiveX control's fallback snapshot image for the control
    /// `control_idx` on `surface`, exactly as the package holds them (never decoded or re-encoded),
    /// or `None` when there is no such control or snapshot. Borrowed from the package.
    #[wasm_bindgen(js_name = "activexSnapshotImageBytes")]
    pub fn activex_snapshot_image_bytes(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .activex_snapshot_image_bytes(surface_of(surface)?, control_idx),
        )
    }

    /// The names of every legacy **VML** drawing part in the package
    /// (`ppt/drawings/vmlDrawingN.vml` and the like), in package order.
    // Behind the `vml` feature, exactly as it is one layer down.
    #[cfg(feature = "vml")]
    #[wasm_bindgen(js_name = "vmlPartNames")]
    pub fn vml_part_names(&self) -> Vec<String> {
        self.inner.vml_part_names()
    }

    /// The raw bytes of the VML drawing `part`, exactly as the package holds them, or `None` when
    /// the package has no such part (or it has been edited elsewhere). Borrowed from the package,
    /// so the part is not copied and nothing is dirtied.
    // Behind the `vml` feature, exactly as it is one layer down.
    #[cfg(feature = "vml")]
    #[wasm_bindgen(js_name = "vmlPartBytes")]
    pub fn vml_part_bytes(&self, part: &str) -> Option<Vec<u8>> {
        self.inner.vml_part_bytes(part)
    }

    /// The names of every **ink** (InkML) part in the package (`ppt/ink/inkN.xml`), in package
    /// order.
    #[wasm_bindgen(js_name = "inkPartNames")]
    pub fn ink_part_names(&self) -> Vec<String> {
        self.inner.ink_part_names()
    }

    /// The raw bytes of the ink (InkML) `part`, exactly as the package holds them, or `None` when
    /// the package has no such part (or it has been edited elsewhere). Borrowed from the package,
    /// so the part is not copied and nothing is dirtied.
    #[wasm_bindgen(js_name = "inkPartBytes")]
    pub fn ink_part_bytes(&self, part: &str) -> Option<Vec<u8>> {
        self.inner.ink_part_bytes(part)
    }

    /// Every ink (InkML) part `surface` references, with where it is referenced from.
    #[wasm_bindgen(js_name = "inkReferences")]
    pub fn ink_references(&mut self, surface: &SurfaceArg) -> Result<Vec<InkReference>, JsValue> {
        map_error(self.inner.ink_references(surface_of(surface)?))
            .map(|values| values.into_iter().map(InkReference).collect())
    }

    /// The ink part the shape `shape_idx` on `surface` references, or `None` when that shape is not
    /// a content part or does not reference ink.
    #[wasm_bindgen(js_name = "inkPartForShape")]
    pub fn ink_part_for_shape(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .ink_part_for_shape(surface_of(surface)?, shape_idx),
        )
    }

    /// The shape index of the content part on `surface` that references the ink `part`, or `None`
    /// when no shape on that surface does (or the reference lives inside an `mc:AlternateContent`,
    /// which is out of the shape index space).
    #[wasm_bindgen(js_name = "shapeForInkPart")]
    pub fn shape_for_ink_part(
        &mut self,
        surface: &SurfaceArg,
        part: &str,
    ) -> Result<Option<u32>, JsValue> {
        map_error(self.inner.shape_for_ink_part(surface_of(surface)?, part))
    }

    /// Adds an ink (InkML) part holding `inkml` to the package and a `p:contentPart` referencing it
    /// to `surface`, and returns the new shape's index in the one shape index space.
    #[wasm_bindgen(js_name = "addInk")]
    pub fn add_ink(&mut self, surface: &SurfaceArg, inkml: &[u8]) -> Result<u32, JsValue> {
        map_error(self.inner.add_ink(surface_of(surface)?, inkml))
    }

    /// Replaces the strokes of the ink the shape `shape_idx` on `surface` references, in place.
    #[wasm_bindgen(js_name = "setInkContent")]
    pub fn set_ink_content(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: u32,
        inkml: &[u8],
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_ink_content(surface_of(surface)?, shape_idx, inkml),
        )
    }

    /// The four relationship ids the SmartArt frame `shape_idx` on `surface` names in its
    /// `dgm:relIds`, or `None` when the shape frames no diagram. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "diagramRelationshipIds")]
    pub fn diagram_relationship_ids(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<DiagramRelationshipIds>, JsValue> {
        map_error(
            self.inner
                .diagram_relationship_ids(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(DiagramRelationshipIds))
    }

    /// The parts of the SmartArt diagram the frame `shape_idx` on `surface` references, resolved to
    /// part names — the relationship graph behind the diagram, `None` when the shape frames none.
    #[wasm_bindgen(js_name = "diagramParts")]
    pub fn diagram_parts(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<DiagramParts>, JsValue> {
        map_error(
            self.inner
                .diagram_parts(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(DiagramParts))
    }

    /// The raw bytes of a diagram `part`, exactly as the package holds them, or `None` when the
    /// package has no such part (or it has been edited elsewhere). Borrowed; nothing is dirtied.
    #[wasm_bindgen(js_name = "diagramPartBytes")]
    pub fn diagram_part_bytes(&self, part: &str) -> Option<Vec<u8>> {
        self.inner.diagram_part_bytes(part)
    }

    /// Adds a SmartArt diagram to `surface`, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    #[wasm_bindgen(js_name = "addDiagram")]
    pub fn add_diagram(
        &mut self,
        surface: &SurfaceArg,
        content: &DiagramContent,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_diagram(surface_of(surface)?, &content.0, bounds.0),
        )
    }

    /// Replaces one part of the SmartArt diagram the frame `shape_idx` on `surface` references, in
    /// place.
    #[wasm_bindgen(js_name = "setDiagramPart")]
    pub fn set_diagram_part(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        kind: DiagramPartKind,
        bytes: Vec<u8>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_diagram_part(
            surface_of(surface)?,
            path_of(shape_idx)?,
            kind.into(),
            bytes,
        ))
    }

    /// Adds an OLE object to `surface`, laid out inside `bounds`, and returns its index in the
    /// shape tree.
    #[wasm_bindgen(js_name = "addOleObject")]
    pub fn add_ole_object(
        &mut self,
        surface: &SurfaceArg,
        spec: &OleObjectSpec,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_ole_object(surface_of(surface)?, &spec.borrowed(), bounds.0),
        )
    }

    /// Sets the `progId` of the OLE frame `shape_idx` on `surface` — which application owns the
    /// embedded object. Only the surface's part is dirtied.
    #[wasm_bindgen(js_name = "setOleProgId")]
    pub fn set_ole_prog_id(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        prog_id: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_ole_prog_id(surface_of(surface)?, path_of(shape_idx)?, prog_id),
        )
    }

    /// Replaces the data of the OLE object the frame `shape_idx` on `surface` embeds, in place.
    #[wasm_bindgen(js_name = "setOleObjectData")]
    pub fn set_ole_object_data(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_ole_object_data(surface_of(surface)?, path_of(shape_idx)?, bytes),
        )
    }

    /// Replaces the fallback snapshot image of the OLE frame `shape_idx` on `surface` — the picture
    /// a consumer draws in place of the object it will never run.
    #[wasm_bindgen(js_name = "setOleSnapshotImage")]
    pub fn set_ole_snapshot_image(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_ole_snapshot_image(
            surface_of(surface)?,
            path_of(shape_idx)?,
            bytes,
        ))
    }

    /// Adds an ActiveX form control to `surface`, laid out inside `bounds`, and returns its index
    /// in the surface's **control** index space (not the shape index space — a `p:control` is a
    /// sibling of the shape tree, not a member of it).
    #[wasm_bindgen(js_name = "addActivexControl")]
    pub fn add_activex_control(
        &mut self,
        surface: &SurfaceArg,
        spec: &ActiveXControlSpec,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_activex_control(surface_of(surface)?, &spec.borrowed(), bounds.0),
        )
    }

    /// Points the OLE frame `shape_idx` on `surface` at the VML shape with `identifier`
    /// (`p:oleObj@spid`) — how an authored object is bound to the legacy fallback that draws it.
    #[wasm_bindgen(js_name = "setOleLegacyShapeId")]
    pub fn set_ole_legacy_shape_id(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        identifier: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_ole_legacy_shape_id(
            surface_of(surface)?,
            path_of(shape_idx)?,
            identifier,
        ))
    }

    /// Points the ActiveX control `control_idx` on `surface` at the VML shape with `identifier`
    /// (`p:control@spid`). As `set_ole_legacy_shape_id`.
    #[wasm_bindgen(js_name = "setActivexControlShapeId")]
    pub fn set_activex_control_shape_id(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
        identifier: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_activex_control_shape_id(
            surface_of(surface)?,
            control_idx,
            identifier,
        ))
    }

    /// The `spid` the ActiveX control `control_idx` on `surface` names — the `id` of the VML shape
    /// that draws it in a legacy consumer — or `None` when there is no such control or it names
    /// none.
    #[wasm_bindgen(js_name = "activexControlShapeId")]
    pub fn activex_control_shape_id(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .activex_control_shape_id(surface_of(surface)?, control_idx),
        )
    }

    /// The COM class id the ActiveX control `control_idx` on `surface` names (`ax:ocx@ax:classid`),
    /// or `None` when there is no such control or its part states none.
    #[wasm_bindgen(js_name = "activexClassId")]
    pub fn activex_class_id(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .activex_class_id(surface_of(surface)?, control_idx),
        )
    }

    /// How the ActiveX control `control_idx` on `surface` persists its state
    /// (`ax:ocx@ax:persistence`), or `None` when there is no such control, its part states none, or
    /// it names a value the ActiveX part does not define.
    #[wasm_bindgen(js_name = "activexPersistence")]
    pub fn activex_persistence(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<Option<ActiveXPersistence>, JsValue> {
        map_error(
            self.inner
                .activex_persistence(surface_of(surface)?, control_idx),
        )?
        .map(ActiveXPersistence::from_model)
        .transpose()
    }

    /// Renames the ActiveX control `control_idx` on `surface` (`p:control@name`). Only the
    /// surface's part is dirtied.
    #[wasm_bindgen(js_name = "setActivexControlName")]
    pub fn set_activex_control_name(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
        name: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_activex_control_name(surface_of(surface)?, control_idx, name),
        )
    }

    /// Replaces the persisted state of the ActiveX control `control_idx` on `surface`, in place.
    #[wasm_bindgen(js_name = "setActivexState")]
    pub fn set_activex_state(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
        state: &[u8],
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_activex_state(surface_of(surface)?, control_idx, state),
        )
    }

    /// Replaces the fallback snapshot image of the ActiveX control `control_idx` on `surface` — the
    /// picture a consumer draws in place of the control it will never run.
    #[wasm_bindgen(js_name = "setActivexSnapshotImage")]
    pub fn set_activex_snapshot_image(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_activex_snapshot_image(surface_of(surface)?, control_idx, bytes),
        )
    }

    /// Removes the ActiveX control `control_idx` from `surface`, closing the gap in the control
    /// index space. Only the surface's part is dirtied.
    #[wasm_bindgen(js_name = "removeActivexControl")]
    pub fn remove_activex_control(
        &mut self,
        surface: &SurfaceArg,
        control_idx: u32,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .remove_activex_control(surface_of(surface)?, control_idx),
        )
    }

    /// The `spid` the OLE frame `shape_idx` on `surface` names — the `id` of the VML shape that
    /// draws it in a legacy consumer — or `None` when the shape frames no OLE object or names no
    /// `spid`.
    #[wasm_bindgen(js_name = "oleLegacyShapeId")]
    pub fn ole_legacy_shape_id(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .ole_legacy_shape_id(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The legacy VML drawing part `surface` relates to, or `None` when it has none.
    // Behind the `vml` feature, exactly as it is one layer down.
    #[cfg(feature = "vml")]
    #[wasm_bindgen(js_name = "vmlDrawingPart")]
    pub fn vml_drawing_part(&self, surface: &SurfaceArg) -> Result<Option<String>, JsValue> {
        map_error(self.inner.vml_drawing_part(surface_of(surface)?))
    }

    /// Stores `drawing` as a new legacy VML drawing part and relates it to `surface`, returning the
    /// part's name.
    // Behind the `vml` feature, exactly as it is one layer down.
    #[cfg(feature = "vml")]
    #[wasm_bindgen(js_name = "addVmlDrawing")]
    pub fn add_vml_drawing(
        &mut self,
        surface: &SurfaceArg,
        drawing: &[u8],
    ) -> Result<String, JsValue> {
        map_error(self.inner.add_vml_drawing(surface_of(surface)?, drawing))
    }

    /// The speaker notes of slide `slide_idx` — the text of its notes slide's `body` placeholder —
    /// or `None` if the slide has no notes slide (or its notes slide has no body placeholder).
    #[wasm_bindgen(js_name = "notesText")]
    pub fn notes_text(&mut self, slide_idx: u32) -> Result<Option<String>, JsValue> {
        map_error(self.inner.notes_text(slide_idx))
    }

    /// Sets the speaker notes of slide `slide_idx` to `text`, creating the notes slide (and, if the
    /// deck has none, the notes master it follows) on demand.
    #[wasm_bindgen(js_name = "setNotesText")]
    pub fn set_notes_text(&mut self, slide_idx: u32, text: &str) -> Result<(), JsValue> {
        map_error(self.inner.set_notes_text(slide_idx, text))
    }

    /// Removes the speaker notes of slide `slide_idx`: unwires the slide → notes-slide relationship
    /// and removes the notes slide part (with its `.rels` and content-type override). A no-op if
    /// the slide has no notes.
    #[wasm_bindgen(js_name = "clearNotes")]
    pub fn clear_notes(&mut self, slide_idx: u32) -> Result<(), JsValue> {
        map_error(self.inner.clear_notes(slide_idx))
    }

    /// Appends a picture (`p:pic`) showing `bytes` to `surface`, laid out at `bounds`. Returns the
    /// index of the new shape in the slide's one shape index space (see `shape_count`);
    /// `shape_kind` reports it as `ShapeKind::Picture`, and the whole `p:spPr` surface — outline,
    /// effects, geometry — applies to it like any other shape.
    #[wasm_bindgen(js_name = "addPicture")]
    pub fn add_picture(
        &mut self,
        surface: &SurfaceArg,
        bytes: &[u8],
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_picture(surface_of(surface)?, bytes, bounds.0),
        )
    }

    /// Every audio/video/media relationship on `surface`, with where each is referenced from and
    /// whether it is external.
    #[wasm_bindgen(js_name = "mediaReferences")]
    pub fn media_references(
        &mut self,
        surface: &SurfaceArg,
    ) -> Result<Vec<MediaReference>, JsValue> {
        map_error(self.inner.media_references(surface_of(surface)?))
            .map(|values| values.into_iter().map(MediaReference).collect())
    }

    /// Replaces the media that relationship `rel_id` on `surface` binds with an in-package
    /// placeholder, so a reference to unreachable external audio/video resolves inside the package
    /// instead. The placeholder is `placeholder` if given, else a built-in one matching the media
    /// kind — a valid silent WAV for audio (`default_placeholder_audio`) or a minimal MP4 for video
    /// (`default_placeholder_video`). The relationship is retargeted at the placeholder, so every
    /// carrier that named it — the `p:pic`, its `a14:media` fallback, timing/transition sounds —
    /// now resolves locally; the poster image is untouched.
    #[wasm_bindgen(js_name = "replaceMediaWithPlaceholder")]
    pub fn replace_media_with_placeholder(
        &mut self,
        surface: &SurfaceArg,
        rel_id: &str,
        placeholder: Option<Vec<u8>>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.replace_media_with_placeholder(
            surface_of(surface)?,
            rel_id,
            placeholder.as_deref(),
        ))
    }

    /// The target of the image that picture `shape_idx` on `surface` *links* (`p:blipFill >
    /// a:blip@r:link`), exactly as the relationship records it — an external path/URL for the
    /// common case, or an in-package part target for an internal link. `None` when the picture
    /// embeds its image (or binds none): an embedded image has no separate target, its bytes are
    /// the image.
    #[wasm_bindgen(js_name = "pictureImageLinkTarget")]
    pub fn picture_image_link_target(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .picture_image_link_target(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The stored bytes of the image that picture `shape_idx` on `surface` binds, exactly as the
    /// package holds them (never decoded or re-encoded), or `None` when the picture binds no image.
    /// Borrowed from the package, so a large image is not copied.
    #[wasm_bindgen(js_name = "pictureImageBytes")]
    pub fn picture_image_bytes(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        map_error(
            self.inner
                .picture_image_bytes(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Points picture `shape_idx` on `surface` at `bytes`, adding the image to the package if it is
    /// not already there (`add_image`, so identical bytes are stored once) and rewriting the blip's
    /// `@r:embed`. Any `@r:link` is dropped — the picture now embeds its image — and the rest of
    /// the `p:blipFill` (source rect, tile/stretch) is preserved.
    #[wasm_bindgen(js_name = "setPictureImage")]
    pub fn set_picture_image(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_picture_image(surface_of(surface)?, path_of(shape_idx)?, bytes),
        )
    }

    /// Every picture on `surface` that *links* its image (`a:blip@r:link`) rather than embedding
    /// it, with where each links from — the candidates for `replace_linked_image_with_placeholder`.
    /// A linked image is the common source that can be unreachable on another platform; this saves
    /// the caller from walking the shapes themselves. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "linkedImages")]
    pub fn linked_images(&mut self, surface: &SurfaceArg) -> Result<Vec<LinkedImage>, JsValue> {
        map_error(self.inner.linked_images(surface_of(surface)?))
            .map(|values| values.into_iter().map(LinkedImage).collect())
    }

    /// Replaces the *linked* image of picture `shape_idx` on `surface` with an embedded
    /// placeholder, so a picture that points at an unreachable external file resolves inside the
    /// package instead. The placeholder is `placeholder` if given, else
    /// `DEFAULT_PLACEHOLDER_IMAGE`. The picture becomes an ordinary embedded picture (`@r:link` →
    /// `@r:embed`), keeping its bounds and the rest of its `p:blipFill`, and the now-unused link
    /// relationship is dropped.
    #[wasm_bindgen(js_name = "replaceLinkedImageWithPlaceholder")]
    pub fn replace_linked_image_with_placeholder(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        placeholder: Option<Vec<u8>>,
    ) -> Result<(), JsValue> {
        map_error(self.inner.replace_linked_image_with_placeholder(
            surface_of(surface)?,
            path_of(shape_idx)?,
            placeholder.as_deref(),
        ))
    }

    /// Stores `bytes` as an image part of the package and relates it to `surface`, returning the
    /// **slide-scoped relationship id** that names the image — the `rel_id` to hand to
    /// `FillSpec::Picture` via `set_shape_fill`.
    #[wasm_bindgen(js_name = "addImage")]
    pub fn add_image(&mut self, surface: &SurfaceArg, bytes: &[u8]) -> Result<String, JsValue> {
        map_error(self.inner.add_image(surface_of(surface)?, bytes))
    }

    /// The number of **top-level** shapes on `surface` — of **every** `ShapeKind` (autoshapes,
    /// pictures, groups, graphic frames, connectors), in document order. A group counts as one
    /// shape here; its own members are addressed by descending into it with a `ShapePath` and are
    /// not included in this count.
    #[wasm_bindgen(js_name = "shapeCount")]
    pub fn shape_count(&mut self, surface: &SurfaceArg) -> Result<u32, JsValue> {
        map_error(self.inner.shape_count(surface_of(surface)?))
    }

    /// What kind of shape `shape_idx` on `surface` is — which of the index-addressed APIs apply to
    /// it (a `Picture` takes the `p:spPr` surface but has no text body; a `GroupShape` has no
    /// `p:spPr` at all).
    #[wasm_bindgen(js_name = "shapeKind")]
    pub fn shape_kind(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<ShapeKind, JsValue> {
        ShapeKind::from_model(map_error(
            self.inner
                .shape_kind(surface_of(surface)?, path_of(shape_idx)?),
        )?)
    }

    /// How many member shapes the group at `shape_idx` holds — `0` for anything that is not a
    /// group, since only a `p:grpSp` has members. This is the range a `ShapePath` may descend into.
    #[wasm_bindgen(js_name = "shapeMemberCount")]
    pub fn shape_member_count(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .shape_member_count(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Every shape of `surface`, in document order — what it is and the placeholder slot it fills.
    #[wasm_bindgen(js_name = "shapes")]
    pub fn shapes(&mut self, surface: &SurfaceArg) -> Result<Vec<ShapeInfo>, JsValue> {
        map_error(self.inner.shapes(surface_of(surface)?))
            .map(|values| values.into_iter().map(ShapeInfo).collect())
    }

    /// The address of the first shape on `surface` that fills the `kind` placeholder slot, or
    /// `None` if the surface offers none.
    #[wasm_bindgen(js_name = "shapeForPlaceholder")]
    pub fn shape_for_placeholder(
        &mut self,
        surface: &SurfaceArg,
        kind: PlaceholderType,
    ) -> Result<Option<u32>, JsValue> {
        map_error(
            self.inner
                .shape_for_placeholder(surface_of(surface)?, kind.into()),
        )
    }

    /// The placeholder shape `shape_idx` on `surface` occupies (`p:nvPr > p:ph`), or `None` if it
    /// is not a placeholder.
    #[wasm_bindgen(js_name = "shapePlaceholder")]
    pub fn shape_placeholder(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<PlaceholderInfo>, JsValue> {
        map_error(
            self.inner
                .shape_placeholder(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(PlaceholderInfo))
    }

    /// Appends a new rectangular text-box shape (`p:sp`) to `surface`, laid out at `bounds` and
    /// containing `text` (one paragraph per line, split on `\n`). Returns the index of the new
    /// shape in the slide's one shape index space (see `shape_count`). Only that part is marked
    /// dirty.
    #[wasm_bindgen(js_name = "addTextBox")]
    pub fn add_text_box(
        &mut self,
        surface: &SurfaceArg,
        text: &str,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_text_box(surface_of(surface)?, text, bounds.0),
        )
    }

    /// Appends a new autoshape (`p:sp`) with the given `preset` geometry to `surface`, laid out at
    /// `bounds`, with an empty text body. Returns the index of the new shape in the slide's one
    /// shape index space (see `shape_count`). Only that part is marked dirty.
    #[wasm_bindgen(js_name = "addShape")]
    pub fn add_shape(
        &mut self,
        surface: &SurfaceArg,
        preset: PresetShapeType,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_shape(surface_of(surface)?, preset.into(), bounds.0),
        )
    }

    /// Removes shape `shape_idx` from `surface`, closing the gap in the shape index space: every
    /// later shape on that surface moves down one index. Only that part is marked dirty.
    #[wasm_bindgen(js_name = "removeShape")]
    pub fn remove_shape(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .remove_shape(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Wraps `members` — which must be siblings — in a new group, returning the group's address.
    #[wasm_bindgen(js_name = "groupShapes")]
    pub fn group_shapes(
        &mut self,
        surface: &SurfaceArg,
        members: &ShapePathListArg,
    ) -> Result<ShapePath, JsValue> {
        map_error(
            self.inner
                .group_shapes(surface_of(surface)?, &paths_of(members)?),
        )
        .map(ShapePath)
    }

    /// Dissolves the group at `shape_idx`, returning where its members now are.
    #[wasm_bindgen(js_name = "ungroup")]
    pub fn ungroup(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Vec<ShapePath>, JsValue> {
        map_error(
            self.inner
                .ungroup(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|values| values.into_iter().map(ShapePath).collect())
    }

    /// Moves shape `shape_idx` into the group at `group_idx`, as its last member, and returns its
    /// new address.
    #[wasm_bindgen(js_name = "moveShapeIntoGroup")]
    pub fn move_shape_into_group(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        group_idx: &ShapePathArg,
    ) -> Result<ShapePath, JsValue> {
        map_error(self.inner.move_shape_into_group(
            surface_of(surface)?,
            path_of(shape_idx)?,
            path_of(group_idx)?,
        ))
        .map(ShapePath)
    }

    /// Moves shape `shape_idx` out of the group holding it, into that group's own container and
    /// directly after it in z-order. Returns its new address.
    #[wasm_bindgen(js_name = "moveShapeOutOfGroup")]
    pub fn move_shape_out_of_group(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<ShapePath, JsValue> {
        map_error(
            self.inner
                .move_shape_out_of_group(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(ShapePath)
    }

    /// What the graphic frame `shape_idx` on `surface` frames — a `Table`, a `Chart`, a `Diagram`
    /// or something else — or `None` when the shape is not a `p:graphicFrame` at all. Reading does
    /// not dirty the part.
    #[wasm_bindgen(js_name = "graphicFrameKind")]
    pub fn graphic_frame_kind(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<GraphicFrameKind>, JsValue> {
        map_error(
            self.inner
                .graphic_frame_kind(surface_of(surface)?, path_of(shape_idx)?),
        )?
        .map(GraphicFrameKind::from_model)
        .transpose()
    }

    /// Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0 —
    /// or, on a deck with no slides yet, to the deck's first layout — and returns its index. The
    /// new slide is a blank shape tree; add content with `add_text_box` or use
    /// `add_slide_with_text`.
    #[wasm_bindgen(js_name = "addSlide")]
    pub fn add_slide(&mut self) -> Result<u32, JsValue> {
        map_error(self.inner.add_slide())
    }

    /// Adds a new slide at the end of the deck built on layout `layout_idx`, carrying a copy of
    /// every placeholder that layout declares, and returns the slide's index.
    #[wasm_bindgen(js_name = "addSlideFromLayout")]
    pub fn add_slide_from_layout(&mut self, layout_idx: u32) -> Result<u32, JsValue> {
        map_error(self.inner.add_slide_from_layout(layout_idx))
    }

    /// Removes slide `slide_idx` from the deck, unwiring it completely: the `p:sldId` naming it,
    /// the presentation's relationship to it, the slide part, its own `.rels`, and its content-type
    /// `Override`.
    #[wasm_bindgen(js_name = "removeSlide")]
    pub fn remove_slide(&mut self, slide_idx: u32) -> Result<(), JsValue> {
        map_error(self.inner.remove_slide(slide_idx))
    }

    /// Adds a new slide (via `add_slide`) carrying a single text box with `text` laid out at
    /// `bounds`, and returns the new slide's index.
    #[wasm_bindgen(js_name = "addSlideWithText")]
    pub fn add_slide_with_text(
        &mut self,
        text: &str,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.add_slide_with_text(text, bounds.0))
    }

    /// Adds a `rows` x `columns` table to `surface`, laid out inside `bounds`, and returns its
    /// index in the shape tree.
    #[wasm_bindgen(js_name = "addTable")]
    pub fn add_table(
        &mut self,
        surface: &SurfaceArg,
        rows: u32,
        columns: u32,
        bounds: &ShapeBounds,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .add_table(surface_of(surface)?, rows, columns, bounds.0),
        )
    }

    /// The shape of the table shape `shape_idx` on `surface` frames, as `(rows, columns)`.
    #[wasm_bindgen(js_name = "tableDimensions")]
    pub fn table_dimensions(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<CellExtent, JsValue> {
        map_error(
            self.inner
                .table_dimensions(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|(first, second)| CellExtent::new(first, second))
    }

    /// The width of column `column` of the table shape `shape_idx` frames, or `None` if the column
    /// states none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "columnWidth")]
    pub fn column_width(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        column: u32,
    ) -> Result<Option<Emu>, JsValue> {
        map_error(
            self.inner
                .column_width(surface_of(surface)?, path_of(shape_idx)?, column),
        )
        .map(|value| value.map(Emu))
    }

    /// Sets the width of column `column`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setColumnWidth")]
    pub fn set_column_width(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        column: u32,
        width: &Emu,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_column_width(
            surface_of(surface)?,
            path_of(shape_idx)?,
            column,
            width.0,
        ))
    }

    /// The height row `row` asks for, or `None` if it states none. PowerPoint grows a row whose
    /// content does not fit, so a rendered row is never shorter than this but may be taller.
    #[wasm_bindgen(js_name = "rowHeight")]
    pub fn row_height(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
    ) -> Result<Option<Emu>, JsValue> {
        map_error(
            self.inner
                .row_height(surface_of(surface)?, path_of(shape_idx)?, row),
        )
        .map(|value| value.map(Emu))
    }

    /// Sets the height row `row` asks for. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setRowHeight")]
    pub fn set_row_height(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        height: &Emu,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_row_height(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            height.0,
        ))
    }

    /// Inserts a row into the table shape `shape_idx` frames so it becomes row `row`; `row` equal
    /// to the current row count appends at the end. The new row copies the height of the row beside
    /// it and its cells are empty and ready for `set_cell_text`. A merge the new row falls inside
    /// grows to include it. Marks only that part dirty; the frame's own bounds are **not** enlarged
    /// (as PowerPoint does not either — resize with `set_shape_bounds`).
    #[wasm_bindgen(js_name = "insertRow")]
    pub fn insert_row(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .insert_row(surface_of(surface)?, path_of(shape_idx)?, row),
        )
    }

    /// Removes row `row` from the table shape `shape_idx` frames. A merge the row lies inside
    /// shrinks; a merge anchored in the row promotes the cell below it, which takes over the
    /// anchor's text and formatting so the table looks unchanged. Marks only that part dirty.
    #[wasm_bindgen(js_name = "removeRow")]
    pub fn remove_row(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .remove_row(surface_of(surface)?, path_of(shape_idx)?, row),
        )
    }

    /// Inserts a column into the table shape `shape_idx` frames so it becomes column `column`;
    /// `column` equal to the current column count appends. The grid gains one `a:gridCol` (width
    /// copied from the column beside it) and every row gains one empty cell, so the grid and rows
    /// stay in step. A merge the new column falls inside grows to include it. Marks only that part
    /// dirty; the frame's own bounds are **not** enlarged.
    #[wasm_bindgen(js_name = "insertColumn")]
    pub fn insert_column(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        column: u32,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .insert_column(surface_of(surface)?, path_of(shape_idx)?, column),
        )
    }

    /// Removes column `column` from the table shape `shape_idx` frames: its `a:gridCol` and one
    /// cell from every row, together. A merge the column lies inside shrinks; a merge anchored in
    /// the column promotes the cell to its right, which takes over the anchor's text and
    /// formatting. Marks only that part dirty.
    #[wasm_bindgen(js_name = "removeColumn")]
    pub fn remove_column(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        column: u32,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .remove_column(surface_of(surface)?, path_of(shape_idx)?, column),
        )
    }

    /// How many rows and columns the cell at `(row, column)` spans, as `(rows, columns)` — the same
    /// order `table_dimensions` answers in, and the order every address on this surface is written
    /// in.
    #[wasm_bindgen(js_name = "cellSpan")]
    pub fn cell_span(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<CellExtent, JsValue> {
        map_error(
            self.inner
                .cell_span(surface_of(surface)?, path_of(shape_idx)?, row, column),
        )
        .map(|(first, second)| CellExtent::new(first, second))
    }

    /// Which cell actually renders at `(row, column)` — itself when it is not merged away, or the
    /// anchor of the merged region covering it.
    #[wasm_bindgen(js_name = "mergedCellAnchor")]
    pub fn merged_cell_anchor(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        row: u32,
        column: u32,
    ) -> Result<CellAddress, JsValue> {
        map_error(self.inner.merged_cell_anchor(
            surface_of(surface)?,
            path_of(shape_idx)?,
            row,
            column,
        ))
        .map(|(first, second)| CellAddress::new(first, second))
    }

    /// Whether the table shape `shape_idx` frames declares banding/emphasis `part` (a `a:tblPr`
    /// flag), or `None` if it does not state the flag. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "tablePart")]
    pub fn table_part(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        part: TablePart,
    ) -> Result<Option<bool>, JsValue> {
        map_error(
            self.inner
                .table_part(surface_of(surface)?, path_of(shape_idx)?, part.into()),
        )
    }

    /// Turns a table's banding/emphasis flag `part` on or off, creating its `a:tblPr` if it had
    /// none. `false` removes the flag rather than writing a `"0"`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setTablePart")]
    pub fn set_table_part(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        part: TablePart,
        on: bool,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_table_part(
            surface_of(surface)?,
            path_of(shape_idx)?,
            part.into(),
            on,
        ))
    }

    /// The GUID of the table style the table shape `shape_idx` frames names (`a:tableStyleId`), or
    /// `None` if it names none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "tableStyleId")]
    pub fn table_style_id(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<String>, JsValue> {
        map_error(
            self.inner
                .table_style_id(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Points the table shape `shape_idx` frames at the table style `style_id`, creating its
    /// `a:tblPr` if it had none. Does not check that the style exists — pair it with
    /// `create_table_style`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setTableStyle")]
    pub fn set_table_style(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        style_id: &str,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .set_table_style(surface_of(surface)?, path_of(shape_idx)?, style_id),
        )
    }

    /// Creates the presentation's `tableStyles.xml` part if it has none, and adds a style with GUID
    /// `style_id` and gallery name `style_name` — replacing one already carrying that GUID. The
    /// style is born empty; give its parts formatting with `format_table_style_part`, and point a
    /// table at it with `set_table_style`.
    #[wasm_bindgen(js_name = "createTableStyle")]
    pub fn create_table_style(&mut self, style_id: &str, style_name: &str) -> Result<(), JsValue> {
        map_error(self.inner.create_table_style(style_id, style_name))
    }

    /// Sets the formatting the style `style_id` gives table `part` (`wholeTbl`, `firstRow`, a
    /// banded row, a corner cell). Only the facets `format` sets are written; the part keeps
    /// whatever else it held. Marks only the `tableStyles.xml` part dirty.
    #[wasm_bindgen(js_name = "formatTableStylePart")]
    pub fn format_table_style_part(
        &mut self,
        style_id: &str,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), JsValue> {
        map_error(
            self.inner
                .format_table_style_part(style_id, part.into(), &format.0),
        )
    }

    /// Gives the table shape `shape_idx` frames its own **inline** style (`a:tableStyle`),
    /// replacing any inline or referenced style it had — the lean alternative to a shared
    /// `tableStyles.xml` style: the whole look is spelled out in `definition` and travels with the
    /// table, so no shared part, relationship or referenced GUID is involved. Marks only that part
    /// dirty.
    #[wasm_bindgen(js_name = "setInlineTableStyle")]
    pub fn set_inline_table_style(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        definition: &TableStyleDefinition,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_inline_table_style(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &definition.0,
        ))
    }

    /// Sets the formatting the table's **inline** style gives one `part`, creating the inline style
    /// if the table had none — the incremental sibling of `set_inline_table_style`, mirroring
    /// `format_table_style_part` for a self-contained style. Only the facets `format` sets are
    /// written. Marks only that part dirty.
    #[wasm_bindgen(js_name = "formatInlineTableStylePart")]
    pub fn format_inline_table_style_part(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), JsValue> {
        map_error(self.inner.format_inline_table_style_part(
            surface_of(surface)?,
            path_of(shape_idx)?,
            part.into(),
            &format.0,
        ))
    }

    /// The full text of shape `shape_idx` on `surface` (paragraphs joined by `\n`).
    #[wasm_bindgen(js_name = "shapeText")]
    pub fn shape_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<String, JsValue> {
        map_error(
            self.inner
                .shape_text(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in
    /// document order) of shape `shape_idx` on `surface`. Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeText")]
    pub fn set_shape_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        run_idx: u32,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            run_idx,
            text,
        ))
    }

    /// Replaces the **whole text** of shape `shape_idx` on `surface` with `text` — one paragraph
    /// per line, each holding exactly one run, so `shape_text` reads back exactly what was written.
    /// Marks only that part dirty.
    #[wasm_bindgen(js_name = "setShapeTextContent")]
    pub fn set_shape_text_content(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        text: &str,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_text_content(
            surface_of(surface)?,
            path_of(shape_idx)?,
            text,
        ))
    }

    /// The number of paragraphs in shape `shape_idx`'s text body. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "paragraphCount")]
    pub fn paragraph_count(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .paragraph_count(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// The number of runs in paragraph `para_idx` of shape `shape_idx`. Reading does not dirty the
    /// part.
    #[wasm_bindgen(js_name = "runCount")]
    pub fn run_count(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .run_count(surface_of(surface)?, path_of(shape_idx)?, para_idx),
        )
    }

    /// The text of paragraph `para_idx` — its runs concatenated. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "paragraphText")]
    pub fn paragraph_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<String, JsValue> {
        map_error(
            self.inner
                .paragraph_text(surface_of(surface)?, path_of(shape_idx)?, para_idx),
        )
    }

    /// The text of one run. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "runText")]
    pub fn run_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<String, JsValue> {
        map_error(
            self.inner
                .run_text(surface_of(surface)?, path_of(shape_idx)?, para_idx, run_idx),
        )
    }

    /// The number of text fields (`a:fld`) in paragraph `para_idx` — generated values such as a
    /// slide number or a date. Fields are a **separate index space** from the runs, so a field
    /// never shifts a run index. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "paragraphFieldCount")]
    pub fn paragraph_field_count(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.paragraph_field_count(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
        ))
    }

    /// The cached text of field `field_idx` in paragraph `para_idx` — the value the producer last
    /// computed for it (a slide number, a formatted date), not a live value. Reading does not dirty
    /// the part.
    #[wasm_bindgen(js_name = "paragraphFieldText")]
    pub fn paragraph_field_text(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        field_idx: u32,
    ) -> Result<String, JsValue> {
        map_error(self.inner.paragraph_field_text(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            field_idx,
        ))
    }

    /// What field `field_idx` in paragraph `para_idx` generates (`a:fld@type`, e.g. `slidenum` or
    /// `datetime`), or `None` if it names no type. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "paragraphFieldType")]
    pub fn paragraph_field_type(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        field_idx: u32,
    ) -> Result<Option<String>, JsValue> {
        map_error(self.inner.paragraph_field_type(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            field_idx,
        ))
    }

    /// The layout properties a paragraph declares of its own (`a:pPr`), or `None` if it declares
    /// none — in which case every property is inherited. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "paragraphProperties")]
    pub fn paragraph_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<Option<ParagraphPropertiesSpec>, JsValue> {
        map_error(self.inner.paragraph_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
        ))
        .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// The character properties a run declares of its own (`a:rPr`), or `None` if it declares none.
    /// Reading does not dirty the part.
    #[wasm_bindgen(js_name = "runProperties")]
    pub fn run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, JsValue> {
        map_error(self.inner.run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            run_idx,
        ))
        .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares none.
    #[wasm_bindgen(js_name = "endRunProperties")]
    pub fn end_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<Option<CharacterPropertiesSpec>, JsValue> {
        map_error(self.inner.end_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
        ))
        .map(|value| value.map(CharacterPropertiesSpec))
    }

    /// Applies `spec` to one run's character properties, creating its `a:rPr` if it has none.
    #[wasm_bindgen(js_name = "setRunProperties")]
    pub fn set_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        run_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            run_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to **every run** in paragraph `para_idx`, and to its `a:endParaRPr` if it has
    /// one — so text typed at the end of the paragraph takes the same formatting, which is what
    /// selecting a paragraph and restyling it means.
    #[wasm_bindgen(js_name = "setParagraphRunProperties")]
    pub fn set_paragraph_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_paragraph_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to **every run of every paragraph** in the shape, and to each paragraph's
    /// `a:endParaRPr` where present — selecting a whole text box and restyling it.
    #[wasm_bindgen(js_name = "setShapeRunProperties")]
    pub fn set_shape_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &spec.0,
        ))
    }

    /// Merges adjacent runs in paragraph `para_idx` that would render identically, returning the
    /// number of runs merged away. This undoes the run splitting that `set_text_range_properties`
    /// does: formatting a sub-range splits a run, and repeatedly formatting overlapping ranges
    /// leaves a paragraph with more runs than it needs.
    #[wasm_bindgen(js_name = "coalesceParagraphRuns")]
    pub fn coalesce_paragraph_runs(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
    ) -> Result<u32, JsValue> {
        map_error(self.inner.coalesce_paragraph_runs(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
        ))
    }

    /// Merges adjacent identical runs across **every** paragraph of a shape's text body, returning
    /// the total number of runs merged away. The per-paragraph rule is `coalesce_paragraph_runs`.
    #[wasm_bindgen(js_name = "coalesceShapeRuns")]
    pub fn coalesce_shape_runs(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<u32, JsValue> {
        map_error(
            self.inner
                .coalesce_shape_runs(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Applies `spec` to the paragraph-mark properties (`a:endParaRPr`), creating the element if
    /// the paragraph has none.
    #[wasm_bindgen(js_name = "setEndRunProperties")]
    pub fn set_end_run_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_end_run_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            &spec.0,
        ))
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if it has
    /// none. The properties **merge**, as run properties do.
    #[wasm_bindgen(js_name = "setParagraphProperties")]
    pub fn set_paragraph_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_paragraph_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            &spec.0,
        ))
    }

    /// The layout properties the shape's own list style offers at `level` (`a:lstStyle >
    /// a:lvlNpPr`), or `None` if it offers none there — or declares no list style at all. Reading
    /// does not dirty the part.
    #[wasm_bindgen(js_name = "shapeListStyleLevel")]
    pub fn shape_list_style_level(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        level: &IndentLevel,
    ) -> Result<Option<ParagraphPropertiesSpec>, JsValue> {
        map_error(self.inner.shape_list_style_level(
            surface_of(surface)?,
            path_of(shape_idx)?,
            level.0,
        ))
        .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// The properties the shape's own list style offers where no level applies (`a:lstStyle >
    /// a:defPPr`), or `None` if it declares none. Reading does not dirty the part.
    #[wasm_bindgen(js_name = "shapeListStyleDefault")]
    pub fn shape_list_style_default(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<Option<ParagraphPropertiesSpec>, JsValue> {
        map_error(
            self.inner
                .shape_list_style_default(surface_of(surface)?, path_of(shape_idx)?),
        )
        .map(|value| value.map(ParagraphPropertiesSpec))
    }

    /// Applies `spec` to what the shape's own list style offers at `level`, creating the
    /// `a:lstStyle` — and the `a:lvlNpPr` within it — if the shape has none. Marks only that part
    /// dirty.
    #[wasm_bindgen(js_name = "setShapeListStyleLevel")]
    pub fn set_shape_list_style_level(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        level: &IndentLevel,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_list_style_level(
            surface_of(surface)?,
            path_of(shape_idx)?,
            level.0,
            &spec.0,
        ))
    }

    /// Applies `spec` to what the shape's own list style offers where no level applies (`a:lstStyle
    /// > a:defPPr`), creating the elements if the shape has none. Marks only that part dirty.
    /// Merges as `set_shape_list_style_level` does.
    #[wasm_bindgen(js_name = "setShapeListStyleDefault")]
    pub fn set_shape_list_style_default(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_shape_list_style_default(
            surface_of(surface)?,
            path_of(shape_idx)?,
            &spec.0,
        ))
    }

    /// Removes what the shape's own list style offers at `level`, so the level falls through to the
    /// tier below again. Returns whether it offered anything there; a `false` changes nothing and
    /// does **not** dirty the part.
    #[wasm_bindgen(js_name = "clearShapeListStyleLevel")]
    pub fn clear_shape_list_style_level(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        level: &IndentLevel,
    ) -> Result<bool, JsValue> {
        map_error(self.inner.clear_shape_list_style_level(
            surface_of(surface)?,
            path_of(shape_idx)?,
            level.0,
        ))
    }

    /// Removes the default properties of the shape's own list style (`a:lstStyle > a:defPPr`).
    /// Returns whether it had any; a `false` changes nothing and does **not** dirty the part.
    #[wasm_bindgen(js_name = "clearShapeListStyleDefault")]
    pub fn clear_shape_list_style_default(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<bool, JsValue> {
        map_error(
            self.inner
                .clear_shape_list_style_default(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Removes the shape's own list style entirely (`a:lstStyle`), so every level falls through to
    /// the tier below. Returns whether the shape had one; a `false` changes nothing and does
    /// **not** dirty the part.
    #[wasm_bindgen(js_name = "clearShapeListStyle")]
    pub fn clear_shape_list_style(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
    ) -> Result<bool, JsValue> {
        map_error(
            self.inner
                .clear_shape_list_style(surface_of(surface)?, path_of(shape_idx)?),
        )
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **Unicode
    /// scalars** across the paragraph's whole text.
    #[wasm_bindgen(js_name = "setTextRangeProperties")]
    pub fn set_text_range_properties(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        range_start: u32,
        range_end: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_text_range_properties(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            range_start..range_end,
            &spec.0,
        ))
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **grapheme
    /// clusters**: what a reader would call characters, and what a text selection actually spans.
    #[wasm_bindgen(js_name = "setTextRangePropertiesByGrapheme")]
    pub fn set_text_range_properties_by_grapheme(
        &mut self,
        surface: &SurfaceArg,
        shape_idx: &ShapePathArg,
        para_idx: u32,
        range_start: u32,
        range_end: u32,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), JsValue> {
        map_error(self.inner.set_text_range_properties_by_grapheme(
            surface_of(surface)?,
            path_of(shape_idx)?,
            para_idx,
            range_start..range_end,
            &spec.0,
        ))
    }
}
