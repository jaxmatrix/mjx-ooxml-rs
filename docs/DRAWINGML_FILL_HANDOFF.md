# Handoff — DrawingML exhaustive fill model (`mjx-dml`), then PowerPoint fill integration

A self-contained brief to **resume the fill workstream from a cold start**. Read this after
`docs/PHASE2_HANDOFF.md` and `docs/DRAWINGML_PRESET_SHAPES.md`. It records what is done, the exact next
piece (make the fill model exhaustive), the verified schema, and the settled design decisions.

---

## 0. Project snapshot — what is on `main` (all green)

Pure-Rust, fidelity-first OOXML library (`mjx-ooxml-rs`). Crate layering (dependencies point **down
only**): `mjx-ooxml-core · mjx-xml · mjx-derive → mjx-opc · mjx-mce · mjx-ooxml-types → mjx-dml · … →
mjx-pptx · … → mjx-ooxml`. `References/` (XSD + geometries) is git-ignored; generated `mjx-ooxml-types`
output is **committed** (regenerate with `cargo run -p xtask -- codegen`). Guardrails in `CLAUDE.md`.

**Done — PowerPoint slice + DrawingML shape/color model:**
- OPC copy-on-write edit surface; `FromXml`/`ToXml` traits + the `mjx-derive` proc-macro; the DrawingML
  **text** model; `mjx-pptx` `Presentation` (open/save; read/edit shape text; `add_text_box`/`add_slide`;
  **read/set/add shape geometry** — `shape_geometry`/`set_shape_geometry`/`add_shape`).
- **Preset-shape geometry — COMPLETE.** `mjx-ooxml-types::drawingml`: generated `PresetShapeType` (187)
  + the `adjustments_of` mechanical table. `mjx-dml::geometry`: `PresetGeometry` / `GeometryGuideList` /
  `GeometryGuide` fidelity model + the typed `ShapeGeometry` tier (`shape()` / `set_shape()`, `Fraction`
  / `Angle` measures) covering **all 117 adjustable shapes named**. Only `teardrop` / `sun` stay
  `Unmodeled` (spec-ambiguous, need ECMA prose); parameterless shapes stay `fidelity`.
- **Color + solidFill — DONE.** Generated `SchemeColor` (`ST_SchemeColorVal`, 17). `mjx-dml`:
  - `color::{Color, ColorKind}` — a **fidelity view over an `EG_ColorChoice` element** (the element
    *name* is the discriminant; the 28 color transforms are preserved as opaque children; hand-written
    `FromXml`/`ToXml` like `GeometryGuide`). Accessors `kind()` / `hex()` / `scheme_color()` / `value()`;
    builders `Color::{srgb, scheme}`.
  - `fill::{SolidFill, SolidFillContent}` (`a:solidFill`) — derive container; the 6 color locals all map
    to the `Color` variant; `color()` accessor + `SolidFill::new` builder (color optional).
  - **Shared build helpers** `dml_name` / `dml_attr` / `attr_str` / `set_attr` live in crate-internal
    **`mjx-dml::build`** — reuse them.

**Key learned facts** (save re-deriving):
- The `mjx-derive` `#[xml(children, child(...))]` **accepts multiple `child(local=…)` entries mapping to
  the same content-enum variant** (that is how the 6 color locals all become `Color`).
- A color/fill **choice-by-element-name** type (`Color`, and the fill wrappers below) hand-writes
  `FromXml`/`ToXml` (or wraps the element as a fidelity struct) because the derive can't discriminate on
  the element's own name.
- The repo uses **squash merges** — branch every PR off `main`; never stack on an unmerged base.

---

## 1. The next piece — make the fill model EXHAUSTIVE (decisions locked)

Model **all 6 `EG_FillProperties` kinds** (`noFill` `solidFill` `gradFill` `blipFill` `pattFill`
`grpFill`), not just `solidFill`. Settled decisions:

- **Depth = type-level exhaustive + key accessors.** All 6 fills round-trip byte-for-byte; expose the
  KEY typed accessors; keep rare/deep internals **opaque** (the fidelity "unknown bucket"). **Do NOT**
  deep-type every internal (the 17 blip effects, path gradients, source/tile/fill rects) — preserve them
  verbatim.
- **PR structure = model now, pptx integration follow-up.** PR-1 = the exhaustive `Fill` model in
  `mjx-dml` (+ generated `PatternType`). PR-2 = `mjx-pptx` `shape_fill` / `set_shape_fill` returning the
  full `Fill` (+ office-open canary).
- **Deferred, ledger-tracked (§5): "PowerPoint default fill resolution."** This PR models only
  **explicit** fills; resolving a shape's *effective* fill when it has none is a separate future task.

---

## 2. Schema facts (verified against `dml-main.xsd`)

`EG_FillProperties` = `<choice>` of exactly one of: `noFill` `solidFill` `gradFill` `blipFill` `pattFill`
`grpFill`. All DrawingML-main (`a:` prefix) **except** the blip image-ref attributes (`r:embed`/`r:link`,
relationships namespace).

- **`a:noFill`** (`CT_NoFillProperties`) and **`a:grpFill`** (`CT_GroupFillProperties`): **empty** — no
  children, no attributes.
- **`a:gradFill`** (`CT_GradientFillProperties`): children (all optional, in order) `gsLst`
  (`CT_GradientStopList` = 2..∞ `gs`), one of `EG_ShadeProperties` (`a:lin`{`@ang`,`@scaled`} XOR
  `a:path`{`@path`, child `fillToRect`}), `tileRect` (`CT_RelativeRect`); attrs `@flip`
  (`ST_TileFlipMode`, default `none`), `@rotWithShape` (bool). **`gs`** (`CT_GradientStop`) = required
  `@pos` (`ST_PositiveFixedPercentage`) + one **direct** `EG_ColorChoice` (a color element, NOT wrapped
  in `CT_Color`).
- **`a:blipFill`** (`CT_BlipFillProperties`): children (optional, in order) `blip` (`CT_Blip`), `srcRect`
  (`CT_RelativeRect`), one of `EG_FillModeProperties` (`a:tile`{tx,ty,sx,sy,flip,algn} XOR `a:stretch`{
  child `fillRect`}); attrs `@dpi`, `@rotWithShape`. **`CT_Blip`** = attrs `@r:embed` / `@r:link`
  (`ST_RelationshipId` → the image part via the source part's `.rels`) + `@cstate`
  (`ST_BlipCompression`), plus a large **opaque** child choice of 17 blip-effect elements + `extLst`.
  Preserve the effects opaque.
- **`a:pattFill`** (`CT_PatternFillProperties`): attr `@prst` (`ST_PresetPatternVal`); children `fgClr`
  and `bgClr`, both `CT_Color` (each wraps one `EG_ColorChoice` — one color element).
- Supporting simple types: `ST_PresetPatternVal` (**54 tokens**, generate — below); `ST_PathShadeType`
  (`shape/circle/rect`); `ST_TileFlipMode` (`none/x/y/xy`); `ST_BlipCompression`
  (`email/screen/print/hqprint/none`); `ST_RectAlignment` (`tl…br`, 9). `CT_RelativeRect` = attrs
  `l/t/r/b` (`ST_Percentage`, default `0%`). Only **generate `ST_PresetPatternVal`**; hand-write the
  small mode enums if you type them.

**`ST_PresetPatternVal` (54, exact order):** `pct5 pct10 pct20 pct25 pct30 pct40 pct50 pct60 pct70 pct75
pct80 pct90 · horz vert ltHorz ltVert dkHorz dkVert narHorz narVert dashHorz dashVert · cross dnDiag
upDiag ltDnDiag ltUpDiag dkDnDiag dkUpDiag wdDnDiag wdUpDiag dashDnDiag dashUpDiag diagCross · smCheck
lgCheck smGrid lgGrid dotGrid smConfetti lgConfetti horzBrick diagBrick solidDmnd openDmnd dotDmnd plaid
sphere weave divot shingle wave trellis zigZag`.

---

## 3. Design to implement — PR-1 (the model)

**Generated enum.** Extend the DrawingML codegen allowlist to
`["ST_ShapeType","ST_SchemeColorVal","ST_PresetPatternVal"]` in `xtask/src/codegen/mod.rs`; add
`("ST_PresetPatternVal","PatternType")` to `TYPE_OVERRIDES` + ~40 `VARIANT_OVERRIDES` rows (spec-sourced,
self-explanatory) in `xtask/src/codegen/spec.rs`: `pct5→Percent5` … `pct90→Percent90`,
`ltHorz→LightHorizontal`, `dkVert→DarkVertical`, `narHorz→NarrowHorizontal`, `dashHorz→DashedHorizontal`,
`dnDiag→DownwardDiagonal`, `upDiag→UpwardDiagonal`, `ltDnDiag→LightDownwardDiagonal`,
`wdUpDiag→WideUpwardDiagonal`, `diagCross→DiagonalCross`, `smCheck→SmallCheckerboard`,
`lgCheck→LargeCheckerboard`, `smGrid→SmallGrid`, `dotGrid→DottedGrid`, `smConfetti→SmallConfetti`,
`horzBrick→HorizontalBrick`, `diagBrick→DiagonalBrick`, `solidDmnd→SolidDiamond`, `openDmnd→OpenDiamond`,
`dotDmnd→DottedDiamond`, `zigZag→ZigZag`, … (`plaid`/`sphere`/`weave`/`divot`/`shingle`/`wave`/`trellis`
auto-expand). Regenerate + **commit** `drawingml.rs`; `shared.rs`/`namespaces.rs` stay byte-identical;
codegen deterministic; add a `PatternType` wire round-trip test in `mjx-ooxml-types/tests/wire.rs`;
`pub use` `PatternType` from `mjx-ooxml-types::drawingml` (via the hand-written `drawingml` module).

**`mjx-dml::fill`.** Extend the module. Each fill type is a **fidelity wrapper** over its element
(framework fields `name`/`attributes`/`children`/`empty`; hand-written `FromXml`/`ToXml` like `Color`;
reuse `crate::build::{dml_name,dml_attr,attr_str}` + a new `crate::build` helper to find a child element
by `(DML_MAIN, local)` and to read a color child via `Color::from_xml`):

- `NoFill`, `GroupFill` — markers (fidelity wrappers; builder `::new(interner)`).
- `SolidFill` — exists (`color()`).
- `GradientFill` — accessors `stops(&Interner) -> Vec<GradientStop>` (walk `gsLst > gs`: `@pos` →
  `Fraction` (native/100000), color child → `Color`), `linear_angle(&Interner) -> Option<Angle>` (find
  `a:lin`, `@ang`), `flip()`, `rot_with_shape()`. Builder `GradientFill::linear(interner, stops:
  &[(Fraction, Color)], angle: Angle)`. `GradientStop { position: Fraction, color: Color }` is a parsed
  view (not a fidelity type).
- `BlipFill` — accessors `image_rel_id(&Interner) -> Option<&str>` (the `blip@r:embed`; the attribute is
  **prefixed** `r:embed`, so resolve the `r` prefix — the fidelity reader leaves attribute namespaces
  unresolved; see `mjx-pptx::nav::namespace_prefix` for the pattern), `mode() -> BlipFillMode`
  (`Tile`/`Stretch`/`None`). Builder `BlipFill::new(interner, rel_id, BlipFillMode)`. `BlipFillMode` enum.
- `PatternFill` — accessors `preset(&Interner) -> Option<PatternType>` (`@prst`), `foreground(&Interner)
  -> Option<Color>` (`fgClr`'s color child), `background(&Interner) -> Option<Color>` (`bgClr`). Builder
  `PatternFill::new(interner, PatternType, fg: Color, bg: Color)`.
- **`Fill` enum** over all 6: `None(NoFill) | Solid(SolidFill) | Gradient(GradientFill) | Blip(BlipFill)
  | Pattern(PatternFill) | Group(GroupFill)`. `Fill::from_xml` dispatches on the element **local name**;
  `Fill::to_xml` rebuilds. Re-export `Fill`, the 5 new types, `GradientStop`, `BlipFillMode`,
  `PatternType` from `lib.rs`.

**Tests** (`mjx-dml/tests/color_model.rs` or a new `fill_model.rs`): round-trip + structural for each
kind (gradient w/ 2 stops + `lin`; blip w/ `r:embed` + `stretch`, effects preserved opaque; pattern w/
`@prst` + `fgClr`/`bgClr`; empty `noFill`/`grpFill`); typed reads (stops pos+hex, linear angle, image
rel id, blip mode, pattern preset + fg/bg); builders → expected bytes; `Fill::from_xml` dispatch for all 6.

---

## 4. Follow-up — PR-2: `mjx-pptx` fill integration

`shape_fill(slide, shape) -> Option<Fill>` (navigate `p:spPr` → the fill child → `Fill::from_xml`);
`set_shape_fill(slide, shape, Fill)` / `set_shape_no_fill` — insert the fill element in the correct
`spPr` slot: **after** geometry (`prstGeom`/`custGeom`), **before** `a:ln`. Adding a `blipFill` image
needs an image part + a relationship (`insert_part` + `add_relationship`, `r:embed`) — its own step.
Extend the office-open canary with a gradient/pattern-filled shape. Mirrors `shape_geometry` /
`set_shape_geometry` (see `mjx-pptx/src/presentation.rs` + `slide::shape_prstgeom`).

---

## 5. Deferred (future round): "PowerPoint default fill resolution"

Modeling **explicit** fills is done here. Resolving a shape's **effective** fill when it has none —
inheritance from the placeholder → `p:style > a:fillRef` style-matrix index → the theme's
`a:fmtScheme`/`a:clrScheme` — is a separate, larger task, **deferred**. It needs the theme part + the
style matrix, neither modeled yet. (Tracked in `docs/DRAWINGML_PRESET_SHAPES.md`.)

---

## 6. Guardrails + verification

- **Fidelity first**: every fill/color preserves unknown attributes, transform/effect children,
  prefixes, and the self-closing flag verbatim; pair every round-trip test with a structural assertion.
  Names spec-sourced (`PatternType` from `ST_PresetPatternVal`). One interner per part; no
  `unwrap`/`panic`/`expect` on parse paths; `unsafe` denied; pure-Rust shipped.
- **Gate**: `cargo test --workspace` · `cargo clippy --workspace --all-targets -- -D warnings` ·
  `cargo fmt --all --check` · strict rustdoc · `cargo run -p xtask -- codegen` deterministic (re-run →
  no diff). Branch off `main`; atomic commit when green; **no AI-attribution trailer**; never stage
  `References/`.

## 7. First actions for the new session

1. `git switch main && git pull --ff-only`.
2. Read this + `docs/DRAWINGML_PRESET_SHAPES.md` + the existing `mjx-dml::{color, fill, build}` modules.
3. Discussion-first, then implement PR-1 (§3): generate `PatternType` → add the 5 fill wrappers +
   the `Fill` enum → tests → note the deferred default-fill task in the ledger (§5). PR-2 (§4) after.
