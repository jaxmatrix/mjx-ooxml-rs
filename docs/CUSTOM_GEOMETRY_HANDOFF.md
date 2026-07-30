# Handoff — custom geometry (`a:custGeom`) — COMPLETE

The freeform path a hand-drawn shape is traced from. Read after `docs/TRANSFORM_HANDOFF.md` (the
`a:xfrm` workstream, which left `a:custGeom` opaque and pointed here). Standard guardrails in
`CLAUDE.md` and `docs/PHASE2_HANDOFF.md` §3.

**Status: COMPLETE — CG1–CG4 shipped, `0.0.54`, 1052 tests green.** (MJX-44.)

**➡ NEXT: another MJX-38 follow-up, or v0.1 (MJX-37).** Custom geometry was the last genuinely-opaque
piece of the `p:spPr` visual family; `a:scene3d`/`a:sp3d` were already modeled (see the audit below).

## Why this workstream exists

Preset geometry (`a:prstGeom`, all ~187 built-in shapes) was modeled long ago. **Custom geometry was
not.** `a:custGeom` — the explicit path list a shape drawn by hand in PowerPoint uses — round-tripped
opaquely inside the raw `p:spPr` tree but had no API: `shape_geometry` returned
`ShapeHasNoGeometry` for it, and a renderer could not read the path list to draw the shape. MJX-44's
title also named `a:scene3d` / `a:sp3d`, but those had since been modeled by other workstreams (see
the audit), so the real remaining work was custom geometry.

## The 3-D audit (part of MJX-44's scope)

`Scene3D` / `Shape3D` (`crates/mjx-dml/src/shape3d.rs`) were checked against `CT_Scene3D` /
`CT_Shape3D` and found **complete** as fidelity wrappers:

- `CT_Scene3D` — `camera` / `lightRig` read typed; `backdrop` and `extLst` stay opaque but round-trip.
- `CT_Camera` / `CT_LightRig` / `CT_SphereCoords` — every attribute and child modeled.
- `CT_Shape3D` — `z` / `extrusionH` / `contourW` / `prstMaterial`, both bevels, both colors typed;
  `extLst` opaque.

No fidelity gap. The one thing without a typed accessor is `a:backdrop` (a rare, deliberately-opaque
child) — a possible future nicety (`Scene3D::backdrop()`), not a gap. MJX-44's 3-D portion is done.

## What shipped

```rust
// mjx-dml — the model (a fidelity wrapper + an interner-free spec, per piece).
let geom = CustomGeometry::from_xml(el, &interner)?;   // reads a:custGeom
geom.paths(&interner);                                  // Vec<Path2DSpec>
geom.adjust_handles(&interner);                         // Vec<AdjustHandle>  (ahXY / ahPolar)
geom.connection_sites(&interner);                       // Vec<ConnectionSite>
geom.text_rectangle(&interner);                         // Option<Rectangle>
let spec: CustomGeometrySpec = geom.spec(&interner);    // author from this
spec.to_custom_geometry(&mut interner);                 // build it back

// A path is a list of interner-free drawing commands a renderer follows.
Path2DSpec { commands: vec![
    DrawCommand::MoveTo(Point::from_emu(0, 0)),
    DrawCommand::CubicBezierTo(a, b, end),
    DrawCommand::Close,
], ..Default::default() };

// mjx-pptx — one accessor for both kinds of geometry.
match deck.shape_geometry(slide, shape)? {
    Geometry::Preset(shape_geometry) => { /* a:prstGeom */ }
    Geometry::Custom(spec)           => { /* a:custGeom  */ }
    Geometry::Inherited              => { /* neither — takes one from layout/master */ }
}
deck.set_shape_geometry(slide, shape, Geometry::Custom(spec))?;   // writes / converts / clears
```

Per PR:

- **CG1 (#99, `0.0.51`)** — foundation types. `PathFillMode` generated into `mjx-ooxml-types`
  (`ST_PathFillMode`, added to the DrawingML codegen allowlist + name map). `AdjustCoordinate`
  (`ST_AdjCoordinate`) and `AdjustAngle` (`ST_AdjAngle`) — each a literal-or-guide-reference union —
  and `AdjustPoint` (`CT_AdjPoint2D`) in `crates/mjx-dml/src/geometry/custom.rs`.
- **CG2 (#100, `0.0.52`)** — the path list. `Path2D` (`a:path`) and `Path2DList` (`a:pathLst`)
  fidelity wrappers, the interner-free `DrawCommand` (moveTo / lnTo / arcTo / quadBezTo / cubicBezTo /
  close), `Point`, and `Path2DSpec`.
- **CG3 (#101, `0.0.53`)** — the container and auxiliary lists. `CustomGeometry`
  (`CT_CustomGeometry2D`) reading `avLst` / `gdLst` guides, `ahLst` handles, `cxnLst` sites, `rect`,
  and `pathLst`; the value types `GuideSpec`, `AdjustHandle`, `ConnectionSite`, `Rectangle`; and
  `CustomGeometrySpec` / `to_custom_geometry`.
- **CG4 (#102, `0.0.54`)** — the PowerPoint surface. The `Geometry { Preset | Custom | Inherited }`
  enum; `shape_geometry` / `set_shape_geometry` unified onto it; `slide::set_geometry` (replace /
  insert / remove the geometry element). **Breaking** (pre-0.1): the accessors moved from
  `ShapeGeometry` to `Geometry`.

Tests: `crates/mjx-dml/tests/custom_geometry_model.rs` (18 — every value type, every command, byte-
exact round-trips with unknown children preserved, build-and-read-back for path / list / whole
geometry), `crates/mjx-pptx/tests/custom_geometry.rs` (4 — author, read-back, preset↔custom↔inherited
conversion, fidelity), and a LibreOffice canary in `office_open.rs` that draws a freeform triangle.

## Decisions settled — do not re-litigate

1. **Fidelity wrapper + interner-free spec, per piece** — the `Scene3D` pattern, not the
   `PresetGeometry` derive-container one. `CustomGeometry` / `Path2D` / `Path2DList` store their
   element verbatim and read facets through accessors, so any unmodeled child (an `extLst`) round-
   trips byte-for-byte; the `*Spec` types (`CustomGeometrySpec`, `Path2DSpec`) are what a caller
   reads and authors. One wire wrapper (`CustomGeometry`); its children are read into interner-free
   values, reusing `GeometryGuideList` (avLst/gdLst) and `Path2DList` from earlier atoms rather than
   minting more wrappers.
2. **Absent is not the default.** A `Path2D` flag (`w`/`h`/`fill`/`stroke`/`extrusionOk`) reads
   `None` when unstated, distinct from the schema default — the crate-wide rule.
3. **A coordinate / angle is a literal *or* a guide reference.** `AdjustCoordinate` /`AdjustAngle`
   are unions (`ST_AdjCoordinate` = `ST_Coordinate | ST_GeomGuideName`). An integer literal reads as
   `Emu` / `Angle`; anything else is a `Guide(name)`, resolved later by the (unbuilt) guide
   evaluator. Fidelity does not hinge on the split — the element preserves the attribute verbatim —
   so an exotic universal-measure literal (which no producer emits here) reading as a guide changes
   only the typed view, never the bytes.
4. **A malformed command still resolves.** A missing `a:pt` or radius reads as the origin / zero
   rather than failing the parse, so a broken geometry stays readable (mirrors `read_sphere_coordinates`).
5. **Unify preset and custom under one accessor.** `shape_geometry` returns `Geometry`, not a
   preset-only type; a shape with no geometry reads `Geometry::Inherited` rather than erroring.
   `set_shape_geometry` writes either kind and converts between them — the two are mutually exclusive
   (`prstGeom` XOR `custGeom`), so setting one drops the other; `Inherited` removes the shape's own
   element so an inherited one takes over. This was the deliberate, user-chosen unification.
6. **Geometry is the second `p:spPr` child**, right after `a:xfrm` and before the fill group.
   `slide::set_geometry` inserts a new element at that slot (`geometry_insert_index`) and replaces an
   existing `prstGeom`/`custGeom` in place.

## Verified schema — read from the XSDs

`CT_CustomGeometry2D` is a sequence: `avLst?`, `gdLst?` (both `CT_GeomGuideList`), `ahLst?`
(`CT_AdjustHandleList` → `ahXY` / `ahPolar`), `cxnLst?` (`CT_ConnectionSiteList` → `cxn`), `rect?`
(`CT_GeomRect`, four `ST_AdjCoordinate` edges), and the **required** `pathLst` (`CT_Path2DList` →
`path`). `CT_Path2D` is a choice of `close` / `moveTo` / `lnTo` / `arcTo` / `quadBezTo` / `cubicBezTo`
with attributes `w` / `h` (`ST_PositiveCoordinate`, default 0), `fill` (`ST_PathFillMode`, default
`norm`), `stroke` / `extrusionOk` (`xsd:boolean`, default `true`). `moveTo`/`lnTo` hold one `a:pt`,
`quadBezTo` two, `cubicBezTo` three; `arcTo` holds `wR`/`hR` (`ST_AdjCoordinate`) and `stAng`/`swAng`
(`ST_AdjAngle`); `close` is empty. Every `a:pt` / `a:pos` is a `CT_AdjPoint2D` (`x`/`y`, required).

## Known follow-ups (not blockers)

- **Guide-formula evaluator.** A `Guide(name)` coordinate/angle references a `gdLst` formula
  (`val`, `*/`, `+-`, `pin`, `sqrt`, `sin`, …). Resolving one to a number — needed to render a
  custom shape whose points are guide-driven, and to resolve preset adjustments too — is a rendering-
  phase concern deferred here. `GuideSpec` keeps the formula verbatim.
- **`Scene3D::backdrop()`** — a typed accessor for the one opaque-but-preserved `a:scene3d` child, if
  a consumer ever needs it (see the audit).
- **No custom-geometry fixture deck.** The tests construct freeform shapes programmatically (as the
  3-D tests do). A hand-authored fixture with a real freeform shape would add a preservation-of-a-
  producer's-bytes angle.

## Where to look

`crates/mjx-dml/src/geometry/custom.rs` (the whole model — value types, path list, container, and
the read/build helpers), `crates/mjx-dml/src/geometry/mod.rs` (re-exports and the fidelity-mechanism
notes), `crates/mjx-pptx/src/geometry.rs` (`Geometry`), `crates/mjx-pptx/src/slide.rs`
(`shape_prstgeom` / `shape_custgeom` / `set_geometry` and the geometry-slot index helpers), and
`crates/mjx-pptx/src/presentation.rs` (`shape_geometry` / `set_shape_geometry`).
