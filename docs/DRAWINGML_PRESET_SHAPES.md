# DrawingML preset-shape geometry — porting ledger & methodology

> Status of porting the DrawingML preset shapes into `mjx-dml` typed geometry, and the
> **methodology** for turning each shape’s raw `adj` control points into self-explanatory
> **named control parameters**. This document is the source of truth for that workstream. The
> **foundation** (the `PresetShapeType` enum + the `prstGeom` fidelity model) has **shipped** — every
> shape round-trips; the remaining batches add the named parameters. Foundation-first, names-in-batches.

Generated data (adjustment / handle counts) is read directly from the ECMA-376 reference
`presetShapeDefinitions.xml` so the ledger is truthful, not guessed. Regenerate with the script in
the PR that introduces this file.

## What a `.pptx` actually stores

A shape’s geometry is `spPr > (prstGeom | custGeom)`. A **preset** shape serialises only:

```xml
<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>
```

i.e. one `prst` token (an `ST_ShapeType`, **187** enumerated values) plus an optional `avLst` of
`gd` adjustment overrides (`name` + `fmla`). The guide formulas, adjust handles, connection sites,
text rectangle, and drawing paths are **not** in the file — they live in the spec’s
`presetShapeDefinitions.xml`. A reader resolves `prst` against that built-in table and applies the
`avLst` overrides on top.

## Two layers

1. **Fidelity layer** (small; round-trips *any* shape). Model
   `prstGeom = { preset: PresetShapeType, adjust_values: Vec<GeomGuide { name, formula }> }`,
   preserving `prst` + the `avLst` `gd` pairs **verbatim**. `PresetShapeType` = the 187 tokens as a
   generated enum with self-explanatory variant names. This is all round-trip fidelity requires.
2. **Semantic layer** (the goal of this ledger; ported in batches). Replace raw `adj`/`adj1`/`adj2`
   with **named control parameters** (e.g. `corner_radius_fraction`) whose meaning, unit, and
   value domain are **derived from `presetShapeDefinitions.xml`, never guessed**.

## Derivation methodology — how each named parameter is produced

- A **user-facing adjustment** is an `avLst` `gd` that has a matching **adjust handle** in the
  shape’s `ahLst`. An `avLst` `gd` with **no** handle (e.g. `star5.hf` / `star5.vf` fudge factors)
  is a constant — named but flagged non-interactive. So *"avLst entry" ≠ "user adjustment"*; the
  handle count below is the better guide to how many real control points a shape has.
- The handle’s **axis discloses the physical meaning**:
  - `ahXY gdRefX` → a **horizontal** offset / width fraction;
  - `ahXY gdRefY` → a **vertical** thickness / height fraction;
  - `ahPolar gdRefAng` → an **angle** (60000ths of a degree; a full turn = `21600000`);
  - `ahPolar gdRefR` → a **radius** fraction.
  Follow `gdRef*` to the `avLst` guide, then to the first `gdLst` formula that consumes it to fix
  the unit: `*/ ss adj 100000` ⇒ fraction of the shorter side; `*/ h adj 200000` ⇒ half-height
  fraction; `sin`/`cos … adj` ⇒ an angle.
- The **value domain** comes from the handle `min*` / `max*`. These are frequently **computed
  guides**, not literals — e.g. `chevron.maxAdj = 100000*w/ss`, `mathDivide.maxAdj2 = 73490 − 4·a3
  + a1`. So a parameter’s bounds can depend on the *other* parameters and on `w`/`h`. **Port bounds
  as formulas, never hard-coded literals.**
- Where the meaning is not mechanically inferable (the complex shapes with interdependent adjusts:
  `gear6/9`, all `star*`, `mathDivide/Equal/NotEqual`, `circularArrow`, `blockArc`, `ribbon*`, the
  callouts, `wave`/`doubleWave`), the name is **sourced from the ECMA-376 Part 1 prose** and curated
  in a table like `xtask/src/codegen/spec.rs` (“generate the mechanical, hand-write the meaningful”).

### Worked examples

| Shape | Raw | Named parameter | Unit / domain |
|---|---|---|---|
| `roundRect` | `adj` | `corner_radius_fraction` | fraction of shorter side, 0.0–0.5, default 1/6 |
| `round2SameRect` | `adj1`, `adj2` | `top_corner_radius_fraction`, `bottom_corner_radius_fraction` | each 0.0–0.5 |
| `chevron` | `adj` | `point_depth_fraction` | 0.0…`maxAdj` (= `w/ss`, data-dependent) |
| `mathDivide` | `adj1`, `adj2`, `adj3` | `bar_thickness_fraction`, `dot_gap_fraction`, `dot_radius_fraction` | coupled `maxAdj*` clamps |

## The two tiers

- **Mechanical tier — ✅ done.** The generated `adjustments_of(PresetShapeType)` table in
  `mjx-ooxml-types::drawingml` (`AdjustmentSpec { wire_name, axis, default, min, max }`, extracted
  from `presetShapeDefinitions.xml`, native spec units; computed bounds kept as `Guide(name)`) + a
  generic runtime API on `PresetGeometry` (`adjustment`/`adjustments`/`set_adjustment`) that reads/sets
  adjustments **by wire name**. Standard-faithful: regenerating ships new/changed shapes immediately.
- **Typed tier — ✅ COMPLETE (except 2 deferred).** Per-shape hand-written structs (a `ShapeGeometry`
  enum, read via `PresetGeometry::shape()` / written via `set_shape()`) with self-explanatory named
  fields in friendly units, built on the mechanical table. **All 117 shapes with user-facing adjustments
  are `named`** — single (43), two (34), and complex (40: callouts, arrows/ribbons/connectors,
  arrow-callouts, and the angle/math set `blockArc`/`mathDivide`/`mathNotEqual`/`circularArrow` family).
  `mjx-dml::geometry::measures` holds `Fraction` and `Angle` (radians). The parameterless fixed-geometry
  shapes stay `fidelity` (nothing to name); **only `teardrop`/`sun` remain `Unmodeled`** (spec-ambiguous,
  pending ECMA prose). A handful of formula-derived names are flagged in their PRs for reviewer
  confirmation (`swooshArrow`/`corner`/`halfFrame`, `ellipseRibbon`/`leftRightRibbon` fields,
  `uturnArrow.tip_height`).

  **Batches follow `adjustments_of` (real user-facing adjustments), not the ledger's raw `avLst`
  counts.** So handle-less `avLst` shapes (`decagon`, `heptagon`, `pentagon`) are *parameterless*, and
  `hexagon` + all stars are *single-adjustment* — even where the tables below place them in a different
  section by `avLst` count (e.g. `star5`/`star6`/`star7`/`star10` sit in the 2–3 adjust sections but
  are single-adjustment and are `named` here).

## Batch plan (each a reviewable, always-green PR that updates this ledger)

1. **Foundation — ✅ done.** `PresetShapeType` enum (187, generated in `mjx-ooxml-types::drawingml`)
   + the `prstGeom`/`avLst`/`gd` fidelity model in `mjx-dml::geometry` (`PresetGeometry` /
   `GeometryGuideList` / `GeometryGuide`, round-trips any shape, plus a minimal typed builder) +
   this ledger scaffold. Reuses the `FromXml`/`ToXml` traits from the text-model PR. **Every shape
   below is now at least `fidelity`.**
2. **Fixed geometry — ✅ done** (mechanical tier). 64 shapes with 0 adjustments → the generated table
   correctly reports them parameterless (`adjustments_of` returns an empty slice); no typed struct
   needed.
3. **Single adjustment — ✅ done** (typed tier). All 43 single-adjustment shapes (by `adjustments_of`)
   are `named`; `teardrop`/`sun` deferred (spec-ambiguous).
4. **Two adjustments — ✅ done** (typed tier). All 34 two-adjustment shapes (by `adjustments_of`) are
   `named`; introduced the `Angle` measure for `arc`/`chord`/`pie`.
5. **Complex — ✅ done** (typed tier). All 40 complex shapes (3–8 adjustments, by `adjustments_of`) are
   `named`, in sub-batches: 5a callouts (12), 5b-i arrows/ribbons/connectors (13), 5b-ii arrow-callouts +
   `bentArrow` + `uturnArrow` (9), 5c angle/math `blockArc`/`mathDivide`/`mathNotEqual`/`circularArrow`
   family (6). **Only `teardrop`/`sun` remain `Unmodeled`** (deferred, pending ECMA prose).

## Anomalies (in `presetShapeDefinitions.xml`)

- **`upArrow` is missing** from `presetShapeDefinitions.xml` although it *is* an `ST_ShapeType`
  token — author its geometry from the `upDownArrow` / `downArrow` pattern when porting.
- **`upDownArrow` is defined twice** (byte-for-byte identical) — dedupe on port. (Blocks in file:
  187; distinct shapes: 186.)

## Out of scope now (rendering phase)

The guide-formula **evaluator** (ops `+- */ +/ ?: pin val sin cos tan at2 cat2 sat2 mod min max
sqrt abs` + built-in vars `w h ss hc vc cd2 cd4 …`) and the **path model** (`moveTo/lnTo/arcTo/
cubicBezTo/quadBezTo/close`) compute actual geometry for *rendering*. Fidelity and named parameters
do **not** need them, so they are deferred.

## Status legend

`pending` = not started · `fidelity` = round-trips via the foundation model · `named` = control
parameters ported. Handle column: `n×ahXY` / `n×ahPolar` (— = fixed geometry, no draggable handle).

## Ledger — 186 distinct shapes

### Batch 2 — fixed geometry (0 adjustments) — 64 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `actionButtonBackPrevious` | Action button | 0 | — | fidelity | — |
| `actionButtonBeginning` | Action button | 0 | — | fidelity | — |
| `actionButtonBlank` | Action button | 0 | — | fidelity | — |
| `actionButtonDocument` | Action button | 0 | — | fidelity | — |
| `actionButtonEnd` | Action button | 0 | — | fidelity | — |
| `actionButtonForwardNext` | Action button | 0 | — | fidelity | — |
| `actionButtonHelp` | Action button | 0 | — | fidelity | — |
| `actionButtonHome` | Action button | 0 | — | fidelity | — |
| `actionButtonInformation` | Action button | 0 | — | fidelity | — |
| `actionButtonMovie` | Action button | 0 | — | fidelity | — |
| `actionButtonReturn` | Action button | 0 | — | fidelity | — |
| `actionButtonSound` | Action button | 0 | — | fidelity | — |
| `bentConnector2` | Connector | 0 | — | fidelity | — |
| `chartPlus` | Basic / geometric | 0 | — | fidelity | — |
| `chartStar` | Basic / geometric | 0 | — | fidelity | — |
| `chartX` | Basic / geometric | 0 | — | fidelity | — |
| `cloud` | Basic / geometric | 0 | — | fidelity | — |
| `cornerTabs` | Basic / geometric | 0 | — | fidelity | — |
| `curvedConnector2` | Connector | 0 | — | fidelity | — |
| `diamond` | Basic / geometric | 0 | — | fidelity | — |
| `dodecagon` | Basic / geometric | 0 | — | fidelity | — |
| `ellipse` | Basic / geometric | 0 | — | fidelity | — |
| `flowChartAlternateProcess` | Flowchart | 0 | — | fidelity | — |
| `flowChartCollate` | Flowchart | 0 | — | fidelity | — |
| `flowChartConnector` | Flowchart | 0 | — | fidelity | — |
| `flowChartDecision` | Flowchart | 0 | — | fidelity | — |
| `flowChartDelay` | Flowchart | 0 | — | fidelity | — |
| `flowChartDisplay` | Flowchart | 0 | — | fidelity | — |
| `flowChartDocument` | Flowchart | 0 | — | fidelity | — |
| `flowChartExtract` | Flowchart | 0 | — | fidelity | — |
| `flowChartInputOutput` | Flowchart | 0 | — | fidelity | — |
| `flowChartInternalStorage` | Flowchart | 0 | — | fidelity | — |
| `flowChartMagneticDisk` | Flowchart | 0 | — | fidelity | — |
| `flowChartMagneticDrum` | Flowchart | 0 | — | fidelity | — |
| `flowChartMagneticTape` | Flowchart | 0 | — | fidelity | — |
| `flowChartManualInput` | Flowchart | 0 | — | fidelity | — |
| `flowChartManualOperation` | Flowchart | 0 | — | fidelity | — |
| `flowChartMerge` | Flowchart | 0 | — | fidelity | — |
| `flowChartMultidocument` | Flowchart | 0 | — | fidelity | — |
| `flowChartOfflineStorage` | Flowchart | 0 | — | fidelity | — |
| `flowChartOffpageConnector` | Flowchart | 0 | — | fidelity | — |
| `flowChartOnlineStorage` | Flowchart | 0 | — | fidelity | — |
| `flowChartOr` | Flowchart | 0 | — | fidelity | — |
| `flowChartPredefinedProcess` | Flowchart | 0 | — | fidelity | — |
| `flowChartPreparation` | Flowchart | 0 | — | fidelity | — |
| `flowChartProcess` | Flowchart | 0 | — | fidelity | — |
| `flowChartPunchedCard` | Flowchart | 0 | — | fidelity | — |
| `flowChartPunchedTape` | Flowchart | 0 | — | fidelity | — |
| `flowChartSort` | Flowchart | 0 | — | fidelity | — |
| `flowChartSummingJunction` | Flowchart | 0 | — | fidelity | — |
| `flowChartTerminator` | Flowchart | 0 | — | fidelity | — |
| `funnel` | Basic / geometric | 0 | — | fidelity | — |
| `heart` | Basic / geometric | 0 | — | fidelity | — |
| `irregularSeal1` | Star / seal | 0 | — | fidelity | — |
| `irregularSeal2` | Star / seal | 0 | — | fidelity | — |
| `lightningBolt` | Basic / geometric | 0 | — | fidelity | — |
| `line` | Basic / geometric | 0 | — | fidelity | — |
| `lineInv` | Basic / geometric | 0 | — | fidelity | — |
| `pieWedge` | Basic / geometric | 0 | — | fidelity | — |
| `plaqueTabs` | Basic / geometric | 0 | — | fidelity | — |
| `rect` | Basic / geometric | 0 | — | fidelity | — |
| `rtTriangle` | Basic / geometric | 0 | — | fidelity | — |
| `squareTabs` | Basic / geometric | 0 | — | fidelity | — |
| `straightConnector1` | Connector | 0 | — | fidelity | — |

### Batch 3 — single adjustment — 41 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `bentConnector3` | Connector | 1 | 1×ahXY | named | `bend_position` |
| `bevel` | Basic / geometric | 1 | 1×ahXY | named | `bevel_width` |
| `bracePair` | Basic / geometric | 1 | 1×ahXY | named | `curl_radius` |
| `bracketPair` | Basic / geometric | 1 | 1×ahXY | named | `corner_radius` |
| `can` | Basic / geometric | 1 | 1×ahXY | named | `top_ellipse_height` |
| `chevron` | Arrow / ribbon | 1 | 1×ahXY | named | `point_depth` |
| `cube` | Basic / geometric | 1 | 1×ahXY | named | `depth` |
| `curvedConnector3` | Connector | 1 | 1×ahXY | named | `bend_position` |
| `decagon` | Basic / geometric | 1 | — | fidelity | — |
| `diagStripe` | Basic / geometric | 1 | 1×ahXY | named | `stripe_width` |
| `donut` | Basic / geometric | 1 | 1×ahPolar | named | `ring_thickness` |
| `foldedCorner` | Basic / geometric | 1 | 1×ahXY | named | `fold_size` |
| `frame` | Basic / geometric | 1 | 1×ahXY | named | `border_thickness` |
| `homePlate` | Arrow / ribbon | 1 | 1×ahXY | named | `point_depth` |
| `horizontalScroll` | Basic / geometric | 1 | 1×ahXY | named | `curl_size` |
| `leftBracket` | Basic / geometric | 1 | 1×ahXY | named | `corner_radius` |
| `mathMinus` | Math | 1 | 1×ahXY | named | `bar_thickness` |
| `mathMultiply` | Math | 1 | 1×ahXY | named | `stroke_thickness` |
| `mathPlus` | Math | 1 | 1×ahXY | named | `arm_thickness` |
| `moon` | Basic / geometric | 1 | 1×ahXY | named | `crescent_width` |
| `noSmoking` | Basic / geometric | 1 | 1×ahPolar | named | `band_thickness` |
| `octagon` | Basic / geometric | 1 | 1×ahXY | named | `corner_cut` |
| `parallelogram` | Basic / geometric | 1 | 1×ahXY | named | `skew_offset` |
| `plaque` | Basic / geometric | 1 | 1×ahXY | named | `corner_size` |
| `plus` | Basic / geometric | 1 | 1×ahXY | named | `arm_inset` |
| `rightBracket` | Basic / geometric | 1 | 1×ahXY | named | `corner_radius` |
| `round1Rect` | Basic / geometric | 1 | 1×ahXY | named | `corner_radius` |
| `roundRect` | Basic / geometric | 1 | 1×ahXY | named | `corner_radius` |
| `smileyFace` | Basic / geometric | 1 | 1×ahXY | named | `mouth_curve` |
| `snip1Rect` | Basic / geometric | 1 | 1×ahXY | named | `snip_size` |
| `star12` | Star / seal | 1 | 1×ahXY | named | `inner_radius` |
| `star16` | Star / seal | 1 | 1×ahXY | named | `inner_radius` |
| `star24` | Star / seal | 1 | 1×ahXY | named | `inner_radius` |
| `star32` | Star / seal | 1 | 1×ahXY | named | `inner_radius` |
| `star4` | Star / seal | 1 | 1×ahXY | named | `inner_radius` |
| `star8` | Star / seal | 1 | 1×ahXY | named | `inner_radius` |
| `sun` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `teardrop` | Basic / geometric | 1 | 1×ahXY | fidelity | — |
| `trapezoid` | Basic / geometric | 1 | 1×ahXY | named | `top_inset` |
| `triangle` | Basic / geometric | 1 | 1×ahXY | named | `apex_x` |
| `verticalScroll` | Basic / geometric | 1 | 1×ahXY | named | `curl_size` |

### Batch 4 — two adjustments — 38 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `arc` | Basic / geometric | 2 | 2×ahPolar | named | `start_angle`, `end_angle` |
| `bentConnector4` | Connector | 2 | 2×ahXY | named | `bend_x`, `bend_y` |
| `chord` | Basic / geometric | 2 | 2×ahPolar | named | `start_angle`, `end_angle` |
| `cloudCallout` | Callout | 2 | 1×ahXY | named | `tail_x`, `tail_y` |
| `corner` | Basic / geometric | 2 | 2×ahXY | named | `horizontal_arm_thickness`, `vertical_arm_thickness` |
| `curvedConnector4` | Connector | 2 | 2×ahXY | named | `bend_x`, `bend_y` |
| `doubleWave` | Basic / geometric | 2 | 2×ahXY | named | `amplitude`, `skew` |
| `downArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `gear6` | Basic / geometric | 2 | 2×ahXY | named | `tooth_depth`, `tooth_width` |
| `gear9` | Basic / geometric | 2 | 2×ahXY | named | `tooth_depth`, `tooth_width` |
| `halfFrame` | Basic / geometric | 2 | 2×ahXY | named | `top_arm_thickness`, `side_arm_thickness` |
| `heptagon` | Basic / geometric | 2 | — | fidelity | — |
| `hexagon` | Basic / geometric | 2 | 1×ahXY | named | `point_inset` |
| `leftArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `leftBrace` | Basic / geometric | 2 | 2×ahXY | named | `curl_radius`, `point_position` |
| `leftRightArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `mathEqual` | Math | 2 | 2×ahXY | named | `bar_thickness`, `bar_gap` |
| `nonIsoscelesTrapezoid` | Basic / geometric | 2 | 2×ahXY | named | `left_top_inset`, `right_top_inset` |
| `notchedRightArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `pentagon` | Basic / geometric | 2 | — | fidelity | — |
| `pie` | Basic / geometric | 2 | 2×ahPolar | named | `start_angle`, `end_angle` |
| `ribbon` | Arrow / ribbon | 2 | 2×ahXY | named | `band_height`, `panel_width` |
| `ribbon2` | Arrow / ribbon | 2 | 2×ahXY | named | `band_height`, `panel_width` |
| `rightArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `rightBrace` | Basic / geometric | 2 | 2×ahXY | named | `curl_radius`, `point_position` |
| `round2DiagRect` | Basic / geometric | 2 | 2×ahXY | named | `top_left_bottom_right_radius`, `top_right_bottom_left_radius` |
| `round2SameRect` | Basic / geometric | 2 | 2×ahXY | named | `top_corner_radius`, `bottom_corner_radius` |
| `snip2DiagRect` | Basic / geometric | 2 | 2×ahXY | named | `top_left_bottom_right_snip`, `top_right_bottom_left_snip` |
| `snip2SameRect` | Basic / geometric | 2 | 2×ahXY | named | `top_corner_snip`, `bottom_corner_snip` |
| `snipRoundRect` | Basic / geometric | 2 | 2×ahXY | named | `round_corner_radius`, `snip_corner_size` |
| `star10` | Star / seal | 2 | 1×ahXY | named | `inner_radius` |
| `star6` | Star / seal | 2 | 1×ahXY | named | `inner_radius` |
| `stripedRightArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `swooshArrow` | Arrow / ribbon | 2 | 2×ahXY | named | `head_thickness`, `head_length` |
| `upDownArrow` _(dup in spec)_ | Arrow / ribbon | 2 | 2×ahXY | named | `shaft_thickness`, `head_length` |
| `wave` | Basic / geometric | 2 | 2×ahXY | named | `amplitude`, `skew` |
| `wedgeEllipseCallout` | Callout | 2 | 1×ahXY | named | `tail_x`, `tail_y` |
| `wedgeRectCallout` | Callout | 2 | 1×ahXY | named | `tail_x`, `tail_y` |

### Batch 5 — complex (3–8 interdependent adjustments) — 43 shapes

| Shape (`prst`) | Category | Adjusts | Handles | Status | Named parameters |
|---|---|---|---|---|---|
| `accentBorderCallout1` | Callout | 4 | 2×ahXY | named | `vertex1_x/y`, `vertex2_x/y` |
| `accentBorderCallout2` | Callout | 6 | 3×ahXY | named | `vertex1_x/y` … `vertex3_x/y` |
| `accentBorderCallout3` | Callout | 8 | 4×ahXY | named | `vertex1_x/y` … `vertex4_x/y` |
| `accentCallout1` | Callout | 4 | 2×ahXY | named | `vertex1_x/y`, `vertex2_x/y` |
| `accentCallout2` | Callout | 6 | 3×ahXY | named | `vertex1_x/y` … `vertex3_x/y` |
| `accentCallout3` | Callout | 8 | 4×ahXY | named | `vertex1_x/y` … `vertex4_x/y` |
| `bentArrow` | Arrow / ribbon | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `bend_radius` |
| `bentConnector5` | Connector | 3 | 3×ahXY | named | `bend1_x`, `bend2_y`, `bend3_x` |
| `bentUpArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `shaft_thickness`, `head_width`, `head_length` |
| `blockArc` | Basic / geometric | 3 | 2×ahPolar | named | `start_angle`, `end_angle`, `ring_thickness` |
| `borderCallout1` | Callout | 4 | 2×ahXY | named | `vertex1_x/y`, `vertex2_x/y` |
| `borderCallout2` | Callout | 6 | 3×ahXY | named | `vertex1_x/y` … `vertex3_x/y` |
| `borderCallout3` | Callout | 8 | 4×ahXY | named | `vertex1_x/y` … `vertex4_x/y` |
| `callout1` | Basic / geometric | 4 | 2×ahXY | named | `vertex1_x/y`, `vertex2_x/y` |
| `callout2` | Basic / geometric | 6 | 3×ahXY | named | `vertex1_x/y` … `vertex3_x/y` |
| `callout3` | Basic / geometric | 8 | 4×ahXY | named | `vertex1_x/y` … `vertex4_x/y` |
| `circularArrow` | Arrow / ribbon | 5 | 4×ahPolar | named | `body_thickness`, `head_pointer_angle`, `end_angle`, `start_angle`, `head_width` |
| `curvedConnector5` | Connector | 3 | 3×ahXY | named | `bend1_x`, `bend2_y`, `bend3_x` |
| `curvedDownArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `body_thickness`, `head_width`, `head_length` |
| `curvedLeftArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `body_thickness`, `head_width`, `head_length` |
| `curvedRightArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `body_thickness`, `head_width`, `head_length` |
| `curvedUpArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `body_thickness`, `head_width`, `head_length` |
| `downArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `ellipseRibbon` | Arrow / ribbon | 3 | 3×ahXY | named | `arch_height`, `center_width`, `fold_thickness` |
| `ellipseRibbon2` | Arrow / ribbon | 3 | 3×ahXY | named | `arch_height`, `center_width`, `fold_thickness` |
| `leftArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `leftCircularArrow` | Arrow / ribbon | 5 | 4×ahPolar | named | `body_thickness`, `head_pointer_angle`, `end_angle`, `start_angle`, `head_width` |
| `leftRightArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `leftRightCircularArrow` | Arrow / ribbon | 5 | 4×ahPolar | named | `body_thickness`, `head_pointer_angle`, `end_angle`, `start_angle`, `head_width` |
| `leftRightRibbon` | Arrow / ribbon | 3 | 3×ahXY | named | `band_height`, `end_width`, `center_fold` |
| `leftRightUpArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `shaft_thickness`, `head_width`, `head_length` |
| `leftUpArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `shaft_thickness`, `head_width`, `head_length` |
| `mathDivide` | Math | 3 | 3×ahXY | named | `bar_thickness`, `dot_gap`, `dot_radius` |
| `mathNotEqual` | Math | 3 | 2×ahXY, 1×ahPolar | named | `bar_thickness`, `slash_angle`, `bar_gap` |
| `quadArrow` | Arrow / ribbon | 3 | 3×ahXY | named | `shaft_thickness`, `head_width`, `head_length` |
| `quadArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `rightArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `star5` | Star / seal | 3 | 1×ahXY | named | `inner_radius` |
| `star7` | Star / seal | 3 | 1×ahXY | named | `inner_radius` |
| `upArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `upDownArrowCallout` | Callout | 4 | 4×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `text_box_size` |
| `uturnArrow` | Arrow / ribbon | 5 | 5×ahXY | named | `shaft_thickness`, `arrowhead_width`, `arrowhead_length`, `bend_radius`, `tip_height` |
| `wedgeRoundRectCallout` | Callout | 3 | 1×ahXY | named | `tail_x`, `tail_y` |

---

_Missing from the spec file but standardised in `ST_ShapeType`: `upArrow` (author on port)._

## Related styling workstream — shape fill (`EG_FillProperties`)

Shape **fill** is a separate DrawingML workstream (see `docs/DRAWINGML_FILL_HANDOFF.md`).

- ✅ **Done:** the color model (`mjx-dml::color::{Color, ColorKind}` + generated `SchemeColor`) and
  the **exhaustive fill model** — all six `EG_FillProperties` kinds (`noFill`/`solidFill`/`gradFill`/
  `blipFill`/`pattFill`/`grpFill`) as a `mjx-dml::fill::Fill` enum of fidelity wrappers with key typed
  accessors + builders, backed by generated `PatternType` (`ST_PresetPatternVal`, 54 tokens).
- ✅ **Done (PR-2):** `mjx-pptx` `shape_fill` / `set_shape_fill` / `set_shape_no_fill` over an
  interner-free `mjx-dml::FillSpec` (+ `ColorSpec`, `GradientStopSpec`) — mirrors `ShapeGeometry`.
  Reads/sets all six kinds; inserts the fill after geometry, before `a:ln`; office-open canary covers
  a gradient- and pattern-filled deck.
- 🔄 **In progress: PowerPoint effective (default) fill resolution** (4-PR workstream, see
  `docs/DRAWINGML_EFFECTIVE_FILL_HANDOFF.md`). Resolves a shape's *effective* fill when it has none —
  inheritance from the placeholder → `p:style > a:fillRef` style-matrix index → the theme's
  `a:fmtScheme`/`a:clrScheme`. Public API interner-free; color resolution targets full concrete RGB.
  - ✅ **PR-1 done:** the theme model (`mjx-dml::theme` — `Theme`/`ColorScheme`/fill-style matrix +
    interner-free `ThemeInfo`) backed by generated `ColorSchemeSlot`, and `Presentation::slide_theme`
    walking slide→layout→master→theme.
  - ✅ **PR-2 done:** `mjx-dml::style` — `StyleMatrixReference` (`a:fillRef`) + `ColorMap`
    (`resolve(SchemeColor)`), and `Presentation::slide_color_map` (master `p:clrMap` + slide
    `p:clrMapOvr`).
  - ✅ **PR-3a/3b done:** `mjx-dml::resolve` — `resolve_color` / `SchemeColors` / `ResolvedColor`
    baking a color to concrete RGB: base kinds (`srgb`/`sys`/`scrgb`/`hsl`/`prst` incl. the 190-color
    table) + the full `EG_ColorTransform` set (`lumMod`/`shade`/`tint`/`alpha`/… applied per level).
  - ✅ **PR-4a done:** `resolve_fill` + public `Presentation::effective_shape_fill` — a shape's
    rendered fill resolved to concrete RGB from its explicit `p:spPr` fill or `p:style > a:fillRef`
    (theme fill-style + phClr).
  - ✅ **PR-4b done:** placeholder inheritance — `effective_shape_fill` walks slide→layout→master
    matching the same-slot `p:ph` (`slide::Placeholder`), completing the third fill source.
- ✅ **Workstream complete:** `effective_shape_fill` covers explicit fill, style `fillRef`, and
  placeholder inheritance, all baked to concrete RGB. Deferred beyond this: theme background fills
  (`p:bg`/`bgFillStyleLst`), non-`p:sp` shapes, exact PowerPoint placeholder-match edge cases.
